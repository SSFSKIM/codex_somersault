# Entrypoint, Bootstrap, CLI, and Setup Specification

> Spec **01** in the reverse-spec project. Anchors are 00-overview (architecture, glossary, canonical 12-section template §6.1, feature-flag matrix), 08-tool-base-registry (tool exposure), and 20-command-system (command surface). This file owns the boot path from process start through `setup()` completion and the Commander.js root + subcommand registration. Anything past `await launchRepl(...)` or past `runHeadless(...)` belongs to other specs.

> **Phase 9.6 revisions (B-full pass):** clarified env-sentinel `'sdk-py'` vs stored `STATE.clientType` value `'sdk-python'` (§1, §3.4); replaced sequential-await wording for the `preAction` hook with explicit `Promise.all([...])` (§1, §5.4 frame); corrected systematic off-by-one in §2.1 line counts (`setup.ts` 478→477, `init.ts` 341→340, `cli.tsx` 303→302, `agentSdkTypes.ts` 444→443, `mcp.ts` 197→196, `sandboxTypes.ts` 157→156, `state.ts` 1759→1758); narrowed `init.ts:57-238` body claim and added EOF-aware framing (§1, §5.4); annotated DSP-strip behavior on the `cc://` print-mode `open` rewrite (§5.1); added spec 04 cross-reference (§2.7). Full provenance in `docs/specs/PHASE9-FIXES-01.md`.

---

## 1. Purpose & Scope

This subsystem is the **boot lifecycle** for the Claude Code CLI. Its responsibility is to take a fresh `node`/`bun` process and bring it to a state where one of three follow-on subsystems can take over:

1. The **Ink REPL** (interactive sessions, owned by spec 37).
2. The **headless `--print` runner** in `src/cli/print.ts` (owned by this spec; algorithmic body delegates to spec 03/04 once `runHeadless` is reached).
3. A **subcommand handler** (`mcp serve`, `auth`, `plugin`, `update`, `doctor`, `agents`, `setup-token`, `install`, etc. — see spec 20 for the slash-command counterparts; classic CLI subcommands are owned here).

Concretely, this subsystem:

- Fires three pre-module-eval side effects (`profileCheckpoint`, `startMdmRawRead`, `startKeychainPrefetch`) before any heavy import resolves, so subprocesses run in parallel with the ~135ms of module loading (`src/main.tsx:1-20` header).
- Memoizes the `init()` function that runs once per process and configures global mTLS, proxy agents, telemetry, OAuth account population, JetBrains detection, GitHub repo detection, remote-managed-settings prefetch, policy-limits prefetch, scratchpad creation, graceful-shutdown setup, CCR upstream proxy startup, and lazy-loaded 1P event logging (`src/entrypoints/init.ts:57-340`; the memoized `init` body's outer `try`/`catch` ends at `:238` where the catch clause begins, but the function's full extent including the `ConfigParseError` branch and trailing `initializeTelemetryAfterTrust` block runs to EOF at `:340`).
- Decides the **entrypoint identity** (`mcp` / `claude-code-github-action` / `sdk-cli` / `cli`) before any event is logged (`src/main.tsx:517-540`) and the **client type** — stored in `STATE.clientType` as one of `'github-action'`, `'sdk-typescript'`, `'sdk-python'`, `'sdk-cli'`, `'claude-vscode'`, `'local-agent'`, `'claude-desktop'`, `'remote'`, `'cli'` — derived from the `CLAUDE_CODE_ENTRYPOINT` env sentinels `'sdk-ts'`, `'sdk-py'`, `'sdk-cli'`, `'claude-vscode'`, `'local-agent'`, `'claude-desktop'`, `'remote'` (note: env sentinel `'sdk-py'` maps to stored value `'sdk-python'`) (`src/main.tsx:817-834`).
- Performs early argv-rewriting fast paths for `cc://` URLs (DIRECT_CONNECT), `--handle-uri` deep links (LODESTONE), `claude assistant` (KAIROS), and `claude ssh` (SSH_REMOTE) — each strips its own argument shape from `process.argv` and stashes a module-scoped `_pendingX` record consumed later (`src/main.tsx:611-795`).
- Runs an eager settings flag pass (`--settings`, `--setting-sources`) BEFORE `init()` so settings filtering is in place when configs are first read (`src/main.tsx:498-516`).
- Builds the Commander.js `program`, registers a single `preAction` hook that awaits `Promise.all([ensureMdmSettingsLoaded(), ensureKeychainPrefetchCompleted()])` (parallel — see `main.tsx:914`) and then runs `init()`, `runMigrations()`, `loadRemoteManagedSettings()`, `loadPolicyLimits()`, and (gated) `uploadUserSettingsInBackground()` (`src/main.tsx:907-967`).
- Registers all flat root options (~70+) and subcommands (`mcp`, `server`, `ssh`, `open`, `auth`, `plugin`, `setup-token`, `agents`, `auto-mode`, `remote-control`, `assistant`, `doctor`, `update`, `up`, `rollback`, `install`, `log`, `error`, `export`, `task`, `completion`) with feature-flag and `USER_TYPE === 'ant'` gates (`src/main.tsx:968-4496`).
- Calls `setup(cwd, permissionMode, ...)` from `src/setup.ts` once per session inside the default action handler. `setup()` does Node version gating, optional `customSessionId` switching, `--bare` opt-outs, UDS messaging server start, teammate snapshot capture, terminal-backup restoration (iTerm2/Terminal.app), the `--worktree` branch (createWorktreeForSession + tmux session creation + chdir + clear claudemd cache + re-snapshot hooks), bundled-skills/plugins init, session-memory hook registration, context-collapse init, version lock, plugin prefetch, attribution-hook + session-file-access + team-memory-watcher install, sinks attach, `tengu_started` beacon, apiKeyHelper prefetch, release-notes/recent-activity prefetch, dangerously-skip-permissions sandbox enforcement, and `tengu_exit` log of the previous session.
- Owns `bootstrap/state.ts` — a 1758-line module that is the **session-global mutable state** holding `sessionId`, `originalCwd`, `projectRoot`, OTel counters, telemetry providers, model usage map, SDK init state, registered hooks, agent color map, scheduled tasks, beta header latches, and ~70 other bound fields. The single `STATE` object is created via `getInitialState()` and mutated through narrow getter/setter pairs.
- Owns the **non-interactive print path** (`src/cli/print.ts`, 5594 lines). `print.ts` builds the `StructuredIO` (or `RemoteIO`) writer, registers process-output error handlers, drives the streaming JSON / NDJSON / text output formats, owns the SDK control-plane request handling (initialize, set-model, set-permission-mode, set-mcp-servers, reload-plugins, channel-enable, rewind-files), services orphaned permission responses, and ultimately calls `ask()` from `src/QueryEngine.ts`.

**Out of scope** (cited, not duplicated):

- Settings/Zod schemas/migrations — spec 02. (Sandbox Zod schemas in `entrypoints/sandboxTypes.ts` are in scope here only because the file is in our owned set; their semantic ownership is spec 02.)
- Query loop, retries, streaming, thinking, token counting — spec 03.
- Turn pipeline (system-reminder injection, hook fan-out, message normalization) — spec 04.
- Context (CLAUDE.md chain, git status, system context, user context) — spec 05.
- Cost/token tracking — spec 06.
- Compaction — spec 07.
- Tool registry — spec 08.
- Permission system — spec 09.
- API service, MCP service, LSP service, OAuth, analytics, plugins, memory — specs 22..29.
- Modes (PROACTIVE/KAIROS/DAEMON/BRIDGE/REMOTE/VOICE/COORDINATOR) — specs 30..36.
- Ink UI shell — spec 37.
- Output styles — spec 38.
- Persistent memory (`memdir/`) — spec 40.
- State / history (`src/state/`, `src/history.ts`, `assistant/sessionHistory`) — spec 41.

---

## 2. Source Map

### 2.1 Owned files (Source Coverage Inventory)

| Path | Lines | Coverage | Notes |
|---|---|---|---|
| `src/main.tsx` | 4683 | sampled top 50 + grep-inspected (tool: bundled-minified — cited regions read in full where load-bearing) | Bundled artifact. Header `:1-20` read fully. Action handler / Commander setup grep-inspected by checkpoint name. Bit-exact does not apply to the bundled body except for explicit citations of literal strings. |
| `src/setup.ts` | 477 | read fully | Includes ANT-only branches at `:337-348` (auto-undercover commitAttribution prime) and `:417-441` (sudo + sandbox hard-gate for `--dangerously-skip-permissions`). |
| `src/entrypoints/init.ts` | 340 | read fully | `init` is `memoize(...)` so it runs at most once per process. |
| `src/entrypoints/cli.tsx` | 302 | read fully | The Agent SDK public `query()` re-export shim. Despite its size, this file does not own the runtime `query()` body — that lives in `print.ts` / `QueryEngine.ts`. |
| `src/entrypoints/agentSdkTypes.ts` | 443 | read fully | SDK type re-exports + unimplemented function shims (`tool`, `createSdkMcpServer`, `query`, `unstable_v2_*`, `getSessionMessages`, `listSessions`, `getSessionInfo`, `renameSession`, `tagSession`, `forkSession`, `watchScheduledTasks`, `buildMissedTaskNotification`, `connectRemoteControl`). Each throws `'... not implemented'` — they are signature anchors only. |
| `src/entrypoints/mcp.ts` | 196 | read fully | Implements `claude mcp serve` — a stdio MCP server that re-exposes the local tool set. |
| `src/entrypoints/sandboxTypes.ts` | 156 | read fully | Sandbox Zod schemas (`SandboxNetworkConfigSchema`, `SandboxFilesystemConfigSchema`, `SandboxSettingsSchema`). Imported by both the SDK type tree and settings validation. Co-owned with spec 02. |
| `src/entrypoints/sdk/coreSchemas.ts` | — | grep-inspected | SDK serializable type Zod schemas. |
| `src/entrypoints/sdk/coreTypes.ts` | — | grep-inspected | SDK serializable types (messages, configs). |
| `src/entrypoints/sdk/controlSchemas.ts` | — | grep-inspected | SDK control-protocol schemas. |
| `src/bootstrap/state.ts` | 1758 | read fully | Session-global mutable state. Header comment block: "DO NOT ADD MORE STATE HERE - BE JUDICIOUS WITH GLOBAL STATE" (`:31`); "ALSO HERE - THINK THRICE BEFORE MODIFYING" (`:259`); "AND ESPECIALLY HERE" (`:428`). |
| `src/cli/print.ts` | 5594 | grep-inspected (function table only) | Headless-mode harness. Top-level identifiers: `runHeadless`, `runHeadlessStreaming`, `createCanUseToolWithPermissionPrompt`, `getCanUseToolFn`, `handleInitializeRequest`, `handleRewindFiles`, `handleSetPermissionMode`, `handleChannelEnable`, `reregisterChannelHandlerAfterReconnect`, `emitLoadError`, `removeInterruptedMessage`, `loadInitialMessages`, `getStructuredIO`, `handleOrphanedPermissionResponse`, `handleMcpSetServers`, `reconcileMcpServers`, plus `SHUTDOWN_TEAM_PROMPT` constant and a `MAX_RECEIVED_UUIDS = 10_000` deduplication ring buffer. |
| `src/cli/structuredIO.ts` | 859 | sampled top 50 | Owns the `StructuredIO` class (line 135). Constants: `SANDBOX_NETWORK_ACCESS_TOOL_NAME = 'SandboxNetworkAccess'` (`:62`); `MAX_RESOLVED_TOOL_USE_IDS = 1000` (`:133`). Internal helpers: `serializeDecisionReason`, `buildRequiresActionDetails`, `exitWithMessage`, `executePermissionRequestHooksForSDK`. |
| `src/cli/remoteIO.ts` | 255 | grep-inspected | The `RemoteIO` writer used in lieu of `StructuredIO` when running under remote control. |
| `src/cli/exit.ts` | 31 | read fully | `cliError(msg?)` and `cliOk(msg?)` — centralized `process.exit`/console-output helpers used by `mcp`/`plugin` handlers. Both return `: never`. |
| `src/cli/ndjsonSafeStringify.ts` | 32 | read fully | `JSON.stringify` wrapper that escapes U+2028 / U+2029 to `\uXXXX` so NDJSON receivers cannot split a line mid-string. |
| `src/cli/update.ts` | 422 | grep-inspected | Implements `claude update` / `claude upgrade`. |
| `src/cli/handlers/agents.ts` | — | grep-inspected | `agentsHandler()` for `claude agents`. |
| `src/cli/handlers/auth.ts` | — | grep-inspected | Auth subcommand handlers + `installOAuthTokens`. |
| `src/cli/handlers/autoMode.ts` | — | grep-inspected | Auto-mode classifier inspection (`auto-mode defaults` / `config` / `critique`). |
| `src/cli/handlers/mcp.tsx` | — | grep-inspected | `claude mcp` family (`serve`, `add-json`, `remove`, `list`, `get`, `add-from-claude-desktop`, `reset-project-choices`). |
| `src/cli/handlers/plugins.ts` | — | grep-inspected | `claude plugin` family (validate, list, install, uninstall, enable, disable, update, marketplace add/list/remove/update). |
| `src/cli/handlers/util.tsx` | — | grep-inspected | `doctorHandler`, `setupTokenHandler`, `installHandler`. |
| `src/cli/transports/{ccrClient,HybridTransport,SerialBatchEventUploader,SSETransport,WebSocketTransport,WorkerStateUploader,transportUtils}.ts` | — | not in §1 scope | Listed for completeness; analytics/transport plumbing is spec 22/26. |

### 2.2 Pre-module-eval ordering anchor

`src/main.tsx:1-20` is the **only authoritative ordering point** for boot. Anything moved above `import { feature } from 'bun:bundle'` (line 21) runs as a side effect during module evaluation; everything below is gated by `feature(...)` DCE and Commander.js dispatch.

```
src/main.tsx:1   // These side-effects must run before all other imports:
src/main.tsx:2   // 1. profileCheckpoint marks entry before heavy module evaluation begins
src/main.tsx:3   // 2. startMdmRawRead fires MDM subprocesses (plutil/reg query) so they run in
src/main.tsx:4   //    parallel with the remaining ~135ms of imports below
src/main.tsx:5   // 3. startKeychainPrefetch fires both macOS keychain reads (OAuth + legacy API
src/main.tsx:6   //    key) in parallel — isRemoteManagedSettingsEligible() otherwise reads them
src/main.tsx:7   //    sequentially via sync spawn inside applySafeConfigEnvironmentVariables()
src/main.tsx:8   //    (~65ms on every macOS startup)
src/main.tsx:9   import { profileCheckpoint, profileReport } from './utils/startupProfiler.js';
src/main.tsx:11  // eslint-disable-next-line custom-rules/no-top-level-side-effects
src/main.tsx:12  profileCheckpoint('main_tsx_entry');
src/main.tsx:13  import { startMdmRawRead } from './utils/settings/mdm/rawRead.js';
src/main.tsx:15  // eslint-disable-next-line custom-rules/no-top-level-side-effects
src/main.tsx:16  startMdmRawRead();
src/main.tsx:17  import { ensureKeychainPrefetchCompleted, startKeychainPrefetch } from './utils/secureStorage/keychainPrefetch.js';
src/main.tsx:19  // eslint-disable-next-line custom-rules/no-top-level-side-effects
src/main.tsx:20  startKeychainPrefetch();
```

The `eslint-disable-next-line custom-rules/no-top-level-side-effects` markers above each side-effect call enforce that this is the **only** place such side effects are permitted (`src/main.tsx:11, :15, :19`).

### 2.3 Feature-flag and ANT guard locations

| Gate | Locations | Owner spec |
|---|---|---|
| `feature('COORDINATOR_MODE')` | `main.tsx:74-77` (require gate), `:1872`, `:2199` (handler), `:3770`, `:4593` | 30 |
| `feature('KAIROS')` | `main.tsx:78-81` (require gate `assistantModule` + `kairosGate`), `:559-562` (`_pendingAssistantChat`), `:685`, `:1058`, `:1642`, `:1728`, `:2184`, `:2197`, `:2206`, `:2647`, `:3035`, `:3259`, `:3838-3844`, `:4334`, `:4623`, `:4647`; `setup.ts` indirectly via `feature('TEAMMEM')`/`isAgentSwarmsEnabled()` | 32 |
| `feature('TRANSCRIPT_CLASSIFIER')` | `main.tsx:171` (`autoModeStateModule`), `:337`, `:1399`, `:1769`, `:2663`, `:3829`, `:4285` | 09 / 04 |
| `feature('DIRECT_CONNECT')` | `main.tsx:548-552`, `:612-642`, `:3156`, `:3961`, `:4058-4099` | 35 |
| `feature('SSH_REMOTE')` | `main.tsx:577-584`, `:706-795`, `:3193`, `:4045-4057` | 35 |
| `feature('LODESTONE')` | `main.tsx:647-677`, `:3781` | 42 / 35 |
| `feature('BRIDGE_MODE')` | `main.tsx:2246`, `:3866`, `:4322-4333` (registers `remote-control` / alias `rc`, always hidden) | 34 |
| `feature('UDS_INBOX')` | `main.tsx:1910`, `:1945`, `:3835`; `setup.ts:95-101` | 19 / 33 |
| `feature('CONTEXT_COLLAPSE')` | `setup.ts:295-301` | 19 / 07 |
| `feature('COMMIT_ATTRIBUTION')` | `setup.ts:350-361` | 10 |
| `feature('TEAMMEM')` | `setup.ts:365-369` | 29 |
| `feature('PROACTIVE')` | `main.tsx:2197`, `:3832`, `:4612-4615` | 31 |
| `feature('KAIROS_BRIEF')` / `feature('KAIROS_CHANNELS')` / `feature('KAIROS_PUSH_NOTIFICATION')` / `feature('KAIROS_GITHUB_WEBHOOKS')` | `main.tsx:1642`, `:1728`, `:2184`, `:2197`, `:2201`, `:3838`, `:3844`, `:4623`, `:4627` | 32 |
| `feature('UPLOAD_USER_SETTINGS')` | `main.tsx:963-965` | 26 / 27 |
| `feature('AGENT_TRIGGERS')` | (cron-related at print.ts:365-371 grep-inspected) | 19 / 32 |
| `feature('EXTRACT_MEMORIES')` | print.ts:374 grep-inspected | 29 |
| `feature('CCR_MIRROR')` | `main.tsx:2918` | 35 |
| `feature('BG_SESSIONS')` | `main.tsx:1116-1119` | 19 |
| `feature('AGENT_MEMORY_SNAPSHOT')` | `main.tsx:2258` | 14 |
| `feature('CHICAGO_MCP')` | `main.tsx:1477`, `:1608` | 23 |
| `feature('WEB_BROWSER_TOOL')` | `main.tsx:1571` | 19 |
| `feature('HARD_FAIL')` | `main.tsx:3870-3872` (registers hidden `--hard-fail` flag) | 42 |
| `"external" === 'ant'` (literal-substituted at build) | `main.tsx:266` (debugger-exit guard for non-ANT), `:340-342` (`migrateFennecToOpus`), `:428-430` (event-loop stall detector — ANT-only), `:4287-4496` (`log`/`error`/`export`/`task`/`completion` ANT-only commands), `:4371` (`up`), `:4382` (`rollback`), `:4595-4604` (relativeProjectPath for tengu_init) | 26 / 41 / 42 |
| `process.env.USER_TYPE === 'ant'` | `state.ts:391-395` (adds `replBridgeActive` to initial state), `state.ts:1570` (`addSlowOperation` early-return); `setup.ts:337-348` (auto-undercover commitAttribution prime) and `setup.ts:417-441` (sandbox + sudo enforcement for ANT users on `--dangerously-skip-permissions` outside Docker/Bubblewrap, exempt for `local-agent` / `claude-desktop`) | this spec / 10 / 27 |

### 2.4 Imports from (top-level only)

The `src/main.tsx` import header (`:1-209`) imports from ~140 internal modules. The most architecturally meaningful (omitting tools/commands which are owned by 08/20):

- `./utils/startupProfiler.js` — `profileCheckpoint`, `profileReport` (boot tracing).
- `./utils/settings/mdm/rawRead.js` — `startMdmRawRead`.
- `./utils/secureStorage/keychainPrefetch.js` — `ensureKeychainPrefetchCompleted`, `startKeychainPrefetch`.
- `bun:bundle` — `feature` (DCE primitive).
- `@commander-js/extra-typings` — `Command as CommanderCommand`, `InvalidArgumentError`, `Option`.
- `./entrypoints/init.js` — `init`, `initializeTelemetryAfterTrust`.
- `./bootstrap/state.js` — ~30 named exports (state setters consumed in the action handler).
- `./commands.js` — `filterCommandsForRemoteMode`, `getCommands` (spec 20).
- `./tools.js` — `getTools` (spec 08).
- `./services/api/bootstrap.js` — `fetchBootstrapData`.
- `./services/policyLimits/index.js` — `loadPolicyLimits`, `refreshPolicyLimits`, `waitForPolicyLimitsToLoad`, `isPolicyAllowed`.
- `./services/remoteManagedSettings/index.js` — `loadRemoteManagedSettings`, `refreshRemoteManagedSettings`.
- `./services/mcp/officialRegistry.js` — `prefetchOfficialMcpUrls`.
- `./services/mcp/client.js` — `getMcpToolsCommandsAndResources`, `prefetchAllMcpResources`, `clearServerCache`.
- `./plugins/bundled/index.js` — `initBuiltinPlugins`.
- `./skills/bundled/index.js` — `initBundledSkills`.
- `./services/analytics/growthbook.js` — `initializeGrowthBook`, `refreshGrowthBookAfterAuthChange`, `getFeatureValue_CACHED_MAY_BE_STALE`, `hasGrowthBookEnvOverride`.
- `./services/analytics/{config,sink,index}.js` — `isAnalyticsDisabled`, `initializeAnalyticsGates`, `logEvent`.
- `./utils/auth.js` — `getSubscriptionType`, `isClaudeAISubscriber`, `prefetchAwsCredentialsAndBedRockInfoIfSafe`, `prefetchGcpCredentialsIfSafe`, `validateForceLoginOrg`.
- `./utils/config.js` — `checkHasTrustDialogAccepted`, `getGlobalConfig`, `getRemoteControlAtStartup`, `isAutoUpdaterDisabled`, `saveGlobalConfig`.
- `./migrations/migrate*.js` — eleven migrations, listed explicitly in §5.5.
- `./remote/RemoteSessionManager.js`, `./server/createDirectConnectSession.js`, `./services/lsp/manager.js`.
- `./dialogLaunchers.js` — `launchAssistantInstallWizard`, `launchAssistantSessionChooser`, `launchInvalidSettingsDialog`, `launchResumeChooser`, `launchSnapshotUpdateDialog`, `launchTeleportRepoMismatchDialog`, `launchTeleportResumeWrapper`.
- `./interactiveHelpers.js` — `exitWithError`, `exitWithMessage`, `getRenderContext`, `renderAndRun`, `showSetupScreens`.
- `./replLauncher.js` — `launchRepl`.
- `./services/teamMemorySync/watcher.js` (lazy import in setup.ts).
- `./utils/swarm/backends/teammateModeSnapshot.js` (lazy import in setup.ts).
- `./utils/udsMessaging.js` (lazy import in setup.ts).
- `./services/contextCollapse/index.js` (lazy require in setup.ts).
- `./services/SessionMemory/sessionMemory.js` — `initSessionMemory`.
- `./utils/nativeInstaller/index.js` — `lockCurrentVersion`.

### 2.5 Imported by (downstream consumers)

`src/bootstrap/state.ts` is the second-most-imported module in the codebase (after `Tool.ts` / `tools.ts`); a non-exhaustive list of consumers visible from imports:

- `src/cli/print.ts`, `src/cli/structuredIO.ts`, `src/cli/remoteIO.ts`.
- `src/QueryEngine.ts`, `src/query.ts`, `src/setup.ts`, `src/entrypoints/init.ts`.
- `src/state/AppStateStore.js` (consumes `getDefaultAppState`).
- All tool implementations under `src/tools/*` that read `getSessionId`, `getOriginalCwd`, `getProjectRoot`, `getMainLoopModelOverride`, etc.

`src/entrypoints/cli.tsx` is **not** the module the published `@anthropic-ai/claude-agent-sdk` SDK consumes at runtime — the same name is shared between the SDK shim (this file, in our owned set) and the CLI bin entrypoint (`bin/claude` shell wrapper, not in the leaked tree). The SDK at runtime delegates to `print.ts:runHeadless` via subprocess; the function bodies in `agentSdkTypes.ts` therefore all throw `not implemented`.

### 2.6 Missing-source ledger

| Symbol | Citation | Reason |
|---|---|---|
| `bin/claude` (the published shell-wrapper bin) | inferred from `package.json` (not in leak); `cli.tsx` itself does not call `process.argv[0]` so cannot serve as the bin | The leaked tree is `src/` only; the published binary is not present. Boot reaches `main.tsx:main()` via the runtime npm bin shim, which is undocumented in the leak. |
| `src/utils/startupProfiler.js` | imported `main.tsx:9`, `setup.ts:48`, `init.ts:1` | Owned by spec 26 (analytics/observability) or spec 42 (long tail). Not in this spec's owned set. |
| `src/utils/settings/mdm/rawRead.js` | imported `main.tsx:13` | Owned by spec 02 (settings). |
| `src/utils/secureStorage/keychainPrefetch.js` | imported `main.tsx:17` | Owned by spec 25 (oauth) or spec 22. |
| `package.json`, `tsconfig.json`, `bin/claude`, build scripts | absent from leak (per CLAUDE.md) | Cannot reverse the actual `node`/`bun` startup invocation. |
| `MACRO.VERSION` global referenced in `entrypoints/mcp.ts:51` | Bundler-replaced macro; literal value not present in source | Build-time substitution; verbatim version string not recoverable from source alone. |

### 2.7 Adjacent-spec cross-references

- 00 — overview, glossary, canonical 12-section template, feature-flag matrix.
- 02 — `enableConfigs()` / `applySafeConfigEnvironmentVariables()` / Sandbox Zod schemas / migration files listed in §5.5.
- 03 — `runHeadless()` body, retry/streaming/thinking; this spec's `print.ts` only frames the call.
- 04 — turn pipeline (system-reminder injection, hook fan-out, message normalization) reached after `runHeadless` or REPL launch.
- 22 — `preconnectAnthropicApi()`, `loadRemoteManagedSettings()`, `loadPolicyLimits()`, OAuth account population (`populateOAuthAccountInfoIfNeeded`).
- 25 — `installOAuthTokens` re-exported from `cli/handlers/auth.ts`.
- 26 — `initializeAnalyticsGates`, `logEvent`, `firstPartyEventLogger`, telemetry / OTel attribution.
- 34 — `bridge/replBridge.js`, `bridge/bridgeMain.js` (subcommand fast-path elision).
- 35 — `remote/RemoteSessionManager.js`, `server/createDirectConnectSession.js`, `server/server.js`, the `claude server` / `claude open` / `claude ssh` subcommands.

---

## 3. Public Interface (Contract)

### 3.1 Process entry

The CLI is launched via the published `claude` binary (not in leak; see §2.6). It calls `main()` exported from `src/main.tsx`:

```ts
// src/main.tsx:585
export async function main(): Promise<void>
```

There is no other documented entry. `await main()` performs all subsequent work; the process exits when `setup()` rejects, when `program.parseAsync(process.argv)` throws/exits, when `runHeadless` returns (with `gracefulShutdown(exitCode)`), or when `launchRepl(...)` returns from the Ink render loop.

### 3.2 `init()` (memoized)

```ts
// src/entrypoints/init.ts:57
export const init: () => Promise<void> = memoize(async (): Promise<void> => { ... })
```

Memoized via `lodash-es/memoize.js`. Runs the global mTLS / proxy / telemetry / OAuth / repo-detection / scratchpad / cleanup-registry init once. `init()` is awaited inside `program.hook('preAction', ...)` (`main.tsx:916`).

```ts
// src/entrypoints/init.ts:247
export function initializeTelemetryAfterTrust(): void
```

Called by interactive paths after the trust dialog accepts. For SDK / headless mode with beta tracing, eagerly fires before remote settings load (`init.ts:252-258`).

### 3.3 `setup()`

```ts
// src/setup.ts:56
export async function setup(
  cwd: string,
  permissionMode: PermissionMode,
  allowDangerouslySkipPermissions: boolean,
  worktreeEnabled: boolean,
  worktreeName: string | undefined,
  tmuxEnabled: boolean,
  customSessionId?: string | null,
  worktreePRNumber?: number,
  messagingSocketPath?: string,
): Promise<void>
```

Called once per session by the default action handler (`main.tsx:~1936`).

### 3.4 `bootstrap/state.ts` — public surface

The module is written as a flat collection of getter/setter pairs over a private `STATE` object. The full surface (98 exports) is too large to inline; the **load-bearing** pairs that the rest of the harness depends on, with citations:

| Function | Citation | Notes |
|---|---|---|
| `getSessionId()` / `regenerateSessionId({setCurrentAsParent?})` / `getParentSessionId()` | `state.ts:431-454` | `randomUUID()` from `src/utils/crypto.js` (browser-aware indirection per `state.ts:13-18`). |
| `switchSession(sessionId, projectDir = null)` | `state.ts:468-479` | Atomic — drops outgoing slug from `planSlugCache`, sets both fields together. Emits `sessionSwitched` signal. |
| `onSessionSwitch(cb)` | `state.ts:489` | Subscribe to atomic switches (used by `concurrentSessions.ts`). |
| `getSessionProjectDir()` | `state.ts:496-498` | `null` means derive from `originalCwd` at read time. |
| `getOriginalCwd()` / `setOriginalCwd(cwd)` | `state.ts:500-517` | NFC-normalized. |
| `getProjectRoot()` / `setProjectRoot(cwd)` | `state.ts:511-525` | **NFC-normalized; only `--worktree` startup flag should call `setProjectRoot`. Mid-session `EnterWorktreeTool` MUST NOT.** |
| `setCwdState(cwd)` / `getCwdState()` | `state.ts:527-533` | NFC-normalized. |
| `setKairosActive(value)` / `getKairosActive()` | `state.ts:1085-1091` | KAIROS gate bit. |
| `setIsInteractive(value)` / `getIsInteractive()` / `getIsNonInteractiveSession()` | `state.ts:1057-1067` | `getIsNonInteractiveSession()` returns `!isInteractive`. |
| `setClientType(type)` / `getClientType()` | `state.ts:1069-1075` | Values: `'cli'`, `'sdk-typescript'`, `'sdk-python'`, `'sdk-cli'`, `'claude-vscode'`, `'local-agent'`, `'claude-desktop'`, `'remote'`, `'github-action'` (see `main.tsx:818-833`). |
| `preferThirdPartyAuthentication()` | `state.ts:1234-1237` | Returns `getIsNonInteractiveSession() && clientType !== 'claude-vscode'`. |
| `getAllowedSettingSources()` / `setAllowedSettingSources(sources)` | `state.ts:1226-1232` | Default is `['userSettings','projectSettings','localSettings','flagSettings','policySettings']` (`state.ts:313-319`). |
| `setMeter(meter, createCounter)` | `state.ts:948-987` | Initializes 8 OTel counters by name. See §10. |
| `setLastAPIRequest(...)` / `setLastAPIRequestMessages(...)` / `setLastClassifierRequests(...)` | `state.ts:1174-1205` | `lastAPIRequestMessages` is ant-only; reference, not clone (`state.ts:115-118` comment). |
| `addToTotalCostState(cost, modelUsage, model)` / `addToTotalDurationState(duration, durationWithoutRetries)` / `addToToolDuration(duration)` / `addToTotalLinesChanged(added, removed)` | `state.ts:543-604` | Cost-tracker peers; details in spec 06. |
| `snapshotOutputTokensForTurn(budget)` / `getTurnOutputTokens()` / `getCurrentTurnTokenBudget()` / `getBudgetContinuationCount()` / `incrementBudgetContinuationCount()` | `state.ts:724-743` | Module-scope (not in `STATE`) ephemerals. |
| `markScrollActivity()` / `getIsScrollDraining()` / `waitForScrollIdle()` | `state.ts:792-824` | Module-scope; debounce at `SCROLL_DRAIN_IDLE_MS = 150` (`:794`). Background intervals must early-return when scrolling. |
| `addInvokedSkill(...)` / `getInvokedSkills()` / `getInvokedSkillsForAgent(agentId)` / `clearInvokedSkills(preserved?)` / `clearInvokedSkillsForAgent(agentId)` | `state.ts:1510-1563` | Composite key `${agentId ?? ''}:${skillName}`. Preserves cross-agent isolation. |
| `addSlowOperation(operation, durationMs)` / `getSlowOperations()` | `state.ts:1569-1621` | ANT-only. `MAX_SLOW_OPERATIONS = 10`, `SLOW_OPERATION_TTL_MS = 10000` (`state.ts:1566-1567`). Editor-prompt operations skipped (`includes('exec')` && `includes('claude-prompt-')`). |
| `getSessionCronTasks()` / `addSessionCronTask(task)` / `removeSessionCronTasks(ids)` | `state.ts:1294-1315` | Session-only cron tasks (durable: false); never written to disk. |
| `setTeleportedSessionInfo(info)` / `getTeleportedSessionInfo()` / `markFirstTeleportMessageLogged()` | `state.ts:1477-1499` | Reliability logging for `--from-pr` / teleport flow. |
| `clearBetaHeaderLatches()` | `state.ts:1744-1749` | Called on `/clear` and `/compact`. |
| `markPostCompaction()` / `consumePostCompaction()` | `state.ts:771-781` | Tags the next API success event with `isPostCompaction=true`. |
| `setLastApiCompletionTimestamp(ts)` / `getLastApiCompletionTimestamp()` | `state.ts:761-767` | Used to compute `timeSinceLastApiCallMs` in `tengu_api_success`. |
| `getPromptId()` / `setPromptId(id)` | `state.ts:1751-1757` | UUID correlating user prompt with subsequent OTel events. |
| `resetCostState()` | `state.ts:864-875` | Resets totals, duration, lines, modelUsage, promptId; reset on `/clear`. |
| `setCostStateForRestore({...})` | `state.ts:881-916` | Adjusts `startTime = Date.now() - lastDuration` so wall-clock duration accumulates correctly across `--resume`. |
| `resetStateForTests()` | `state.ts:919-930` | Throws unless `process.env.NODE_ENV === 'test'`. |

### 3.5 Sandbox Zod schemas (`entrypoints/sandboxTypes.ts`)

The lazy-schema factory pattern is used (see spec 02 for the pattern). Schemas exposed verbatim in §6.4.

```ts
export const SandboxNetworkConfigSchema   // sandboxTypes.ts:14
export const SandboxFilesystemConfigSchema // sandboxTypes.ts:47
export const SandboxSettingsSchema         // sandboxTypes.ts:91
export type SandboxSettings = z.infer<ReturnType<typeof SandboxSettingsSchema>>
export type SandboxNetworkConfig = NonNullable<z.infer<ReturnType<typeof SandboxNetworkConfigSchema>>>
export type SandboxFilesystemConfig = NonNullable<z.infer<ReturnType<typeof SandboxFilesystemConfigSchema>>>
export type SandboxIgnoreViolations = NonNullable<SandboxSettings['ignoreViolations']>
```

### 3.6 SDK type re-exports (`agentSdkTypes.ts`)

This module is an **alpha public-API anchor**. All function bodies throw — they exist for type/signature stability. See `agentSdkTypes.ts:73-443` for full signatures of `tool()`, `createSdkMcpServer()`, `query()`, `unstable_v2_createSession()`, `unstable_v2_resumeSession()`, `unstable_v2_prompt()`, `getSessionMessages()`, `listSessions()`, `getSessionInfo()`, `renameSession()`, `tagSession()`, `forkSession()`, `watchScheduledTasks()`, `buildMissedTaskNotification()`, `connectRemoteControl()`. Every body is `throw new Error('... not implemented')`.

### 3.7 `entrypoints/mcp.ts` — `claude mcp serve`

```ts
// entrypoints/mcp.ts:35
export async function startMCPServer(
  cwd: string,
  debug: boolean,
  verbose: boolean,
): Promise<void>
```

Implements an MCP server (over stdio) that exposes the local tool set to upstream MCP clients. Constants: `READ_FILE_STATE_CACHE_SIZE = 100`, "100 files and 25MB limit" (`mcp.ts:42-43` comment). `MCP_COMMANDS` is currently `[review]` only (`mcp.ts:33`). The handler list-tools converts each tool's Zod input schema via `zodToJsonSchema`; output schemas with non-`'object'` root are **silently skipped** to comply with the MCP SDK contract (`mcp.ts:75-82`). The server identifies itself as `name: 'claude/tengu', version: MACRO.VERSION` (`mcp.ts:48-51`).

---

## 4. Data Model & State

### 4.1 The `State` type in `bootstrap/state.ts:45-256`

The full type is too large (90+ fields) to inline. Categorical breakdown:

- **Identity / cwd**: `originalCwd`, `projectRoot`, `cwd`, `sessionId`, `parentSessionId`, `sessionProjectDir`.
- **Cost**: `totalCostUSD`, `totalAPIDuration`, `totalAPIDurationWithoutRetries`, `totalToolDuration`, `totalLinesAdded`, `totalLinesRemoved`, `hasUnknownModelCost`, `modelUsage`, `lastApiCompletionTimestamp`, `pendingPostCompaction`.
- **Per-turn timers**: `turnHookDurationMs`, `turnToolDurationMs`, `turnClassifierDurationMs`, `turnToolCount`, `turnHookCount`, `turnClassifierCount`.
- **Session timing**: `startTime`, `lastInteractionTime`.
- **Telemetry providers**: `meter`, `loggerProvider`, `eventLogger`, `meterProvider`, `tracerProvider`.
- **OTel counters** (8): `sessionCounter`, `locCounter`, `prCounter`, `commitCounter`, `costCounter`, `tokenCounter`, `codeEditToolDecisionCounter`, `activeTimeCounter`.
- **Stats observation** sink: `statsStore`.
- **Model**: `mainLoopModelOverride`, `initialMainLoopModel`, `modelStrings`, `sdkBetas`, `mainThreadAgentType`.
- **Mode bits**: `isInteractive`, `kairosActive`, `strictToolResultPairing`, `sdkAgentProgressSummariesEnabled`, `userMsgOptIn`, `clientType`, `sessionSource`, `questionPreviewFormat`, `isRemoteMode`, `directConnectServerUrl`.
- **Settings/auth**: `flagSettingsPath`, `flagSettingsInline`, `allowedSettingSources`, `sessionIngressToken`, `oauthTokenFromFd`, `apiKeyFromFd`.
- **Color**: `agentColorMap` (`Map<string, AgentColorName>`), `agentColorIndex`.
- **API request audit (ANT-only)**: `lastAPIRequest` (without messages), `lastAPIRequestMessages` (with messages, reference not clone), `lastClassifierRequests`.
- **Caches**: `cachedClaudeMdContent`, `systemPromptSectionCache`, `planSlugCache`.
- **Error log**: `inMemoryErrorLog` (capped at `MAX_IN_MEMORY_ERRORS = 100` per `state.ts:1219`).
- **Plugins / channels**: `inlinePlugins`, `chromeFlagOverride`, `useCoworkPlugins`, `allowedChannels`, `hasDevChannels`.
- **Permission session bits**: `sessionBypassPermissionsMode`, `scheduledTasksEnabled`, `sessionCronTasks`, `sessionCreatedTeams`, `sessionTrustAccepted`, `sessionPersistenceDisabled`.
- **One-time UI flags**: `hasExitedPlanMode`, `needsPlanModeExitAttachment`, `needsAutoModeExitAttachment`, `lspRecommendationShownThisSession`.
- **SDK init**: `initJsonSchema`, `registeredHooks`.
- **Teleport / channels / skills**: `teleportedSessionInfo`, `invokedSkills`, `slowOperations`, `additionalDirectoriesForClaudeMd`.
- **Beta header sticky-on latches** (4): `afkModeHeaderLatched`, `fastModeHeaderLatched`, `cacheEditingHeaderLatched`, `thinkingClearLatched`. All sticky-on; reset only on `/clear` and `/compact` via `clearBetaHeaderLatches()`.
- **Eligibility caches**: `promptCache1hAllowlist`, `promptCache1hEligible`.
- **Date-rollover tracking**: `lastEmittedDate`.
- **Prompt correlation**: `promptId`.
- **Last main-conversation request id** (for shutdown cache eviction hints): `lastMainRequestId`.
- **ANT-only conditional bit** (added at `state.ts:391-395`): `replBridgeActive`.

### 4.2 Initial state derivation

`getInitialState()` (`state.ts:260-426`) constructs `STATE` once. `cwd` is derived by:

```ts
let resolvedCwd = ''
if (
  typeof process !== 'undefined' &&
  typeof process.cwd === 'function' &&
  typeof realpathSync === 'function'
) {
  const rawCwd = cwd()  // from 'process'
  try {
    resolvedCwd = realpathSync(rawCwd).normalize('NFC')
  } catch {
    // File Provider EPERM on CloudStorage mounts (lstat per path component).
    resolvedCwd = rawCwd.normalize('NFC')
  }
}
```
(`state.ts:262-275`). Sets `originalCwd`, `projectRoot`, and `cwd` to `resolvedCwd`.

`sessionId` is `randomUUID() as SessionId` (`state.ts:331`).

The initial `allowedSettingSources` value is `['userSettings', 'projectSettings', 'localSettings', 'flagSettings', 'policySettings']` (`state.ts:313-319`).

### 4.3 Module-scope mutable globals (NOT in `STATE`)

- `interactionTimeDirty: boolean` (`state.ts:665`) — debounce flag for `updateLastInteractionTime`.
- `outputTokensAtTurnStart: number = 0`, `currentTurnTokenBudget: number | null = null`, `budgetContinuationCount: number = 0` (`state.ts:724-732`).
- `scrollDraining: boolean = false`, `scrollDrainTimer: ReturnType<typeof setTimeout> | undefined`, `SCROLL_DRAIN_IDLE_MS = 150` (`state.ts:792-794`).
- `EMPTY_SLOW_OPERATIONS: ReadonlyArray<...> = []` (`state.ts:1589-1593`) — stable empty reference.
- `MAX_SLOW_OPERATIONS = 10`, `SLOW_OPERATION_TTL_MS = 10000` (`state.ts:1566-1567`).
- `telemetryInitialized: boolean = false` in `init.ts:55` — guards `doInitializeTelemetry` against re-entry.
- `MAX_RECEIVED_UUIDS = 10_000` and `receivedMessageUuids: Set<UUID>` / `receivedMessageUuidsOrder: UUID[]` in `print.ts:394-396` — bounded UUID dedup ring buffer for inbound message replay.
- `MAX_RESOLVED_TOOL_USE_IDS = 1000` in `cli/structuredIO.ts:133`.
- `_pendingConnect`, `_pendingAssistantChat`, `_pendingSSH` in `main.tsx:548-584` — module-scoped slots populated by argv pre-rewrite, consumed in the action handler.

### 4.4 Persistent state schema

This subsystem owns no persistent on-disk schema. Files written by code in this spec:

- `<tmpdir>/claude-settings-<contentHash>.json` — `loadSettingsFromFlag` may write a temp file when `--settings <json>` is a JSON string (`main.tsx:444-457`); content-hash basename ensures cache stability.
- `STATE.flagSettingsPath` is set to that path via `setFlagSettingsPath()`.
- `setSessionPersistenceDisabled(true)` (controlled by `--no-session-persistence` only with `--print`) prevents downstream session file writes (spec 41).

The `--worktree` branch in `setup.ts:175-285` performs filesystem mutations (worktree creation, tmux session, chdir) but the on-disk shape is owned by spec 18 (`createWorktreeForSession`).

`switchSession(asSessionId(customSessionId))` is called from `setup.ts:82-84` when `customSessionId` is provided.

### 4.5 Lifecycle / state machine

Boot has a strict linear lifecycle:

```
[process start]
  → main.tsx pre-imports run (profileCheckpoint, MDM read, keychain prefetch)
  → heavy imports complete (~135ms)
  → main.tsx:209  profileCheckpoint('main_tsx_imports_loaded')
  → main()  [main.tsx:585]
     ├ NoDefaultCurrentDirectoryInExePath = '1'             [main.tsx:591]
     ├ initializeWarningHandler()                            [:594]
     ├ process.on('exit', resetCursor); process.on('SIGINT', ...)  [:595-606]
     ├ DIRECT_CONNECT cc:// argv rewrite                     [:611-642]
     ├ LODESTONE --handle-uri & macOS bundle URL handling    [:647-677]
     ├ KAIROS `claude assistant` argv rewrite                [:685-700]
     ├ SSH_REMOTE `claude ssh` argv rewrite                  [:706-795]
     ├ early --print/--init-only/--sdk-url detection         [:797-808]
     ├ setIsInteractive(...)                                 [:812]
     ├ initializeEntrypoint(isNonInteractive)                [:815]
     ├ setClientType(...)                                    [:834]
     ├ setQuestionPreviewFormat(...)                         [:836-843]
     ├ setSessionSource('remote-control') if env==bridge     [:846-848]
     ├ eagerLoadSettings()                                   [:852]
     └ run()                                                 [:854]
        ├ build Commander program (sorted help, positional opts)
        ├ program.hook('preAction', async () => { ... })     [:907-967]
        │   ├ await ensureMdmSettingsLoaded()
        │   ├ await ensureKeychainPrefetchCompleted()
        │   ├ await init()                                   [memoized]
        │   ├ if (!CLAUDE_CODE_DISABLE_TERMINAL_TITLE) process.title = 'claude'
        │   ├ await import('./utils/sinks.js').initSinks()
        │   ├ if (--plugin-dir) setInlinePlugins + clearPluginCache
        │   ├ runMigrations()
        │   ├ void loadRemoteManagedSettings()
        │   ├ void loadPolicyLimits()
        │   └ if (UPLOAD_USER_SETTINGS) void uploadUserSettingsInBackground()
        ├ register all root options (~70+)
        ├ if (--print) skip subcommand registration; parseAsync; return
        ├ register subcommands (mcp/server/ssh/open/auth/plugin/setup-token/agents/auto-mode/remote-control/assistant/doctor/update/up/rollback/install/log/error/export/task/completion)
        ├ await program.parseAsync(process.argv)
        ├ profileReport()
        └ return program
        ↓ default action handler              [main.tsx:1007-3870, ~2860 lines]
           ├ logTenguInit({...})
           ├ await setup(cwd, permissionMode, ...)
           ├ getCommands() / getTools()
           ├ MCP config load + connect
           ├ launchRepl(...) | runHeadless(...)
```

### 4.6 Cancellation & cleanup

- `setupGracefulShutdown()` is called eagerly in `init()` (`init.ts:87`).
- `registerCleanup(shutdownLspServerManager)` (`init.ts:189`) — LSP shutdown.
- `registerCleanup(async () => { (await import('../utils/swarm/teamHelpers.js')).cleanupSessionTeams() })` (`init.ts:195-200`) — clean up `sessionCreatedTeams` on `gracefulShutdown` to avoid orphaned team disk records (referenced as gh-32730 in source comment).
- `lockCurrentVersion()` (`setup.ts:303`) — fire-and-forget version lock to prevent the auto-updater from deleting the running binary mid-session.
- `gracefulShutdownSync(1)` is the synchronous-exit entry; `gracefulShutdown(code)` is the async path.

---

## 5. Algorithm / Control Flow

### 5.1 `main()` — the top-level orchestrator

Pseudocode (`main.tsx:585-856`, with literal flag handling and ordering preserved):

```
main():
  profileCheckpoint('main_function_start')
  // Windows PATH-hijack defense
  process.env.NoDefaultCurrentDirectoryInExePath = '1'
  initializeWarningHandler()
  process.on('exit', resetCursor)
  process.on('SIGINT', () => {
    if (process.argv.includes('-p') || process.argv.includes('--print')) return  // print.ts has its own handler
    process.exit(0)
  })
  profileCheckpoint('main_warning_handler_initialized')

  // ---- Argv pre-rewrite fast paths (each gated by feature(...)) ----
  if feature('DIRECT_CONNECT'):
    find a 'cc://' or 'cc+unix://' arg
    parse it via parseConnectUrl()
    set _pendingConnect.dangerouslySkipPermissions
    if -p / --print:
      rewrite argv to use the internal `open` subcommand: ['claude','open',ccUrl, ...stripped]
      // where `stripped` = rawCliArgs with the cc:// URL AND `--dangerously-skip-permissions` removed.
      // The DSP flag is stashed on `_pendingConnect.dangerouslySkipPermissions` for later application
      // by the `open` handler — it is intentionally NOT forwarded as an argv flag (`main.tsx:621-628`).
    else:
      strip the cc:// arg + dangerously-skip-permissions, populate _pendingConnect.{url,authToken}
  if feature('LODESTONE'):
    if --handle-uri <uri> in argv:
      enableConfigs()
      handleDeepLinkUri(uri); process.exit(handler exit code)
    if process.platform === 'darwin' && process.env.__CFBundleIdentifier === 'com.anthropic.claude-code-url-handler':
      enableConfigs()
      handleUrlSchemeLaunch(); process.exit(result ?? 1)
  if feature('KAIROS') && _pendingAssistantChat:
    if rawArgs[0] === 'assistant':
      if rawArgs[1] is a non-flag → _pendingAssistantChat.sessionId = rawArgs[1]
      elif !rawArgs[1] → _pendingAssistantChat.discover = true
      else fall through (e.g. `assistant --help`)
      strip 'assistant' (and sessionId) from argv
  if feature('SSH_REMOTE') && _pendingSSH:
    if rawArgs[0] === 'ssh':
      pull --local, --dangerously-skip-permissions, --permission-mode {value|=value}
      forward -c/--continue, --resume <uuid>, --model <model> via _pendingSSH.extraCliArgs
    if rawArgs[0]==='ssh' && rawArgs[1] is non-flag:
      _pendingSSH.host = rawArgs[1]
      if rawArgs[2] non-flag: _pendingSSH.cwd = rawArgs[2]
      if rest contains -p/--print: stderr 'Error: headless not supported with claude ssh'; gracefulShutdownSync(1); return
      rewrite argv to [...rest]   // no `ssh` token left

  // ---- Mode classification ----
  const cliArgs = process.argv.slice(2)
  hasPrintFlag    = cliArgs.includes('-p') || cliArgs.includes('--print')
  hasInitOnlyFlag = cliArgs.includes('--init-only')
  hasSdkUrl       = cliArgs.some(a => a.startsWith('--sdk-url'))
  isNonInteractive = hasPrintFlag || hasInitOnlyFlag || hasSdkUrl || !process.stdout.isTTY
  if isNonInteractive: stopCapturingEarlyInput()
  setIsInteractive(!isNonInteractive)

  initializeEntrypoint(isNonInteractive)
  setClientType( ... see §5.2 ... )

  previewFormat = process.env.CLAUDE_CODE_QUESTION_PREVIEW_FORMAT
  if previewFormat in {'markdown','html'}: setQuestionPreviewFormat(previewFormat)
  elif clientType not in {'sdk-*','claude-desktop','local-agent','remote'}: setQuestionPreviewFormat('markdown')

  if process.env.CLAUDE_CODE_ENVIRONMENT_KIND === 'bridge': setSessionSource('remote-control')

  eagerLoadSettings()        // --settings, --setting-sources
  await run()                // builds Commander, hooks preAction, parses argv
```

### 5.2 `setClientType` decision tree (`main.tsx:818-833`)

```
clientType =
  GITHUB_ACTIONS                                         → 'github-action'
  CLAUDE_CODE_ENTRYPOINT === 'sdk-ts'                    → 'sdk-typescript'
  CLAUDE_CODE_ENTRYPOINT === 'sdk-py'                    → 'sdk-python'
  CLAUDE_CODE_ENTRYPOINT === 'sdk-cli'                   → 'sdk-cli'
  CLAUDE_CODE_ENTRYPOINT === 'claude-vscode'             → 'claude-vscode'
  CLAUDE_CODE_ENTRYPOINT === 'local-agent'               → 'local-agent'
  CLAUDE_CODE_ENTRYPOINT === 'claude-desktop'            → 'claude-desktop'
  CLAUDE_CODE_ENTRYPOINT === 'remote'
   OR CLAUDE_CODE_SESSION_ACCESS_TOKEN
   OR CLAUDE_CODE_WEBSOCKET_AUTH_FILE_DESCRIPTOR         → 'remote'
  else                                                   → 'cli'
```

### 5.3 `initializeEntrypoint(isNonInteractive)` (`main.tsx:517-540`)

```
if process.env.CLAUDE_CODE_ENTRYPOINT already set: return
cliArgs = process.argv.slice(2)
mcpIndex = cliArgs.indexOf('mcp')
if mcpIndex !== -1 && cliArgs[mcpIndex+1] === 'serve':
  process.env.CLAUDE_CODE_ENTRYPOINT = 'mcp'; return
if isEnvTruthy(process.env.CLAUDE_CODE_ACTION):
  process.env.CLAUDE_CODE_ENTRYPOINT = 'claude-code-github-action'; return
process.env.CLAUDE_CODE_ENTRYPOINT = isNonInteractive ? 'sdk-cli' : 'cli'
```

`local-agent` is set externally before invoking the binary; it falls through the early-return guard.

### 5.4 `init()` body (`init.ts:57-238` happy-path try block; full module extends to `:340` including the `ConfigParseError` catch and the trailing `initializeTelemetryAfterTrust` export at `:247`) — memoized

```
init = memoize(async () => {
  const start = Date.now()
  profileCheckpoint('init_function_start')
  try:
    const t0 = Date.now()
    enableConfigs()                                    // validate configs, enable system
    profileCheckpoint('init_configs_enabled')

    applySafeConfigEnvironmentVariables()              // pre-trust env subset
    applyExtraCACertsFromConfig()                      // NODE_EXTRA_CA_CERTS — must be before any TLS handshake (Bun caches at boot)
    profileCheckpoint('init_safe_env_vars_applied')

    setupGracefulShutdown()
    profileCheckpoint('init_after_graceful_shutdown')

    // 1P event logging deferred (avoids loading OTel sdk-logs at startup)
    void Promise.all([
      import('../services/analytics/firstPartyEventLogger.js'),
      import('../services/analytics/growthbook.js'),
    ]).then(([fp, gb]) => {
      fp.initialize1PEventLogging()
      gb.onGrowthBookRefresh(() => void fp.reinitialize1PEventLoggingIfConfigChanged())
    })
    profileCheckpoint('init_after_1p_event_logging')

    void populateOAuthAccountInfoIfNeeded()
    profileCheckpoint('init_after_oauth_populate')

    void initJetBrainsDetection()
    profileCheckpoint('init_after_jetbrains_detection')
    void detectCurrentRepository()                     // fire-and-forget GitHub repo detection

    if isEligibleForRemoteManagedSettings(): initializeRemoteManagedSettingsLoadingPromise()
    if isPolicyLimitsEligible():               initializePolicyLimitsLoadingPromise()
    profileCheckpoint('init_after_remote_settings_check')

    recordFirstStartTime()

    configureGlobalMTLS()                              // NEW: must run before configureGlobalAgents (proxy)
    configureGlobalAgents()                            // proxy + mTLS http(s).Agent
    profileCheckpoint('init_network_configured')

    preconnectAnthropicApi()                           // overlap TCP+TLS handshake with handler work; skipped for proxy/mTLS/unix/cloud
    if isEnvTruthy(process.env.CLAUDE_CODE_REMOTE):
      try {
        const { initUpstreamProxy, getUpstreamProxyEnv } = await import('../upstreamproxy/upstreamproxy.js')
        const { registerUpstreamProxyEnvFn }           = await import('../utils/subprocessEnv.js')
        registerUpstreamProxyEnvFn(getUpstreamProxyEnv)
        await initUpstreamProxy()                       // CCR upstream proxy: local CONNECT relay
      } catch (err) { logForDebugging('[init] upstreamproxy init failed: ...; continuing without proxy', { level: 'warn' }) }

    setShellIfWindows()                                // git-bash detection
    registerCleanup(shutdownLspServerManager)
    registerCleanup(async () => {                       // gh-32730 — clean orphaned subagent teams
      const { cleanupSessionTeams } = await import('../utils/swarm/teamHelpers.js')
      await cleanupSessionTeams()
    })

    if isScratchpadEnabled(): await ensureScratchpadDir()
    profileCheckpoint('init_function_end')
  catch error:
    if error instanceof ConfigParseError:
      if getIsNonInteractiveSession():
        process.stderr.write(`Configuration error in ${error.filePath}: ${error.message}\n`)
        gracefulShutdownSync(1); return
      else:
        return import('../components/InvalidConfigDialog.js').then(m => m.showInvalidConfigDialog({ error }))
    else: throw
})
```

### 5.5 `runMigrations()` (`main.tsx:325-352`) — ordered list

`CURRENT_MIGRATION_VERSION = 11` (`main.tsx:325`). When `getGlobalConfig().migrationVersion !== 11`, run in order:

1. `migrateAutoUpdatesToSettings()`
2. `migrateBypassPermissionsAcceptedToSettings()`
3. `migrateEnableAllProjectMcpServersToSettings()`
4. `resetProToOpusDefault()`
5. `migrateSonnet1mToSonnet45()`
6. `migrateLegacyOpusToCurrent()`
7. `migrateSonnet45ToSonnet46()`
8. `migrateOpusToOpus1m()`
9. `migrateReplBridgeEnabledToRemoteControlAtStartup()`
10. If `feature('TRANSCRIPT_CLASSIFIER')`: `resetAutoModeOptInForDefaultOffer()`
11. If `"external" === 'ant'`: `migrateFennecToOpus()`

Then `saveGlobalConfig(prev => prev.migrationVersion === 11 ? prev : { ...prev, migrationVersion: 11 })` — idempotent CAS.

Async migration (no version bar): `migrateChangelogFromConfig().catch(() => {})` — silently swallowed; will retry on next startup.

### 5.6 `setup()` body (`src/setup.ts:56-477`) — the per-session setup function

```
setup(cwd, permissionMode, allowDangerouslySkipPermissions, worktreeEnabled, worktreeName, tmuxEnabled, customSessionId?, worktreePRNumber?, messagingSocketPath?):
  logForDiagnosticsNoPII('info', 'setup_started')

  // Node version gate
  let m = process.version.match(/^v(\d+)\./)
  if !m || parseInt(m[1]) < 18:
    console.error(chalk.bold.red('Error: Claude Code requires Node.js version 18 or higher.'))
    process.exit(1)

  if customSessionId: switchSession(asSessionId(customSessionId))

  // ---- UDS messaging ----
  if !isBareMode() || messagingSocketPath !== undefined:
    if feature('UDS_INBOX'):
      const m = await import('./utils/udsMessaging.js')
      await m.startUdsMessaging(
        messagingSocketPath ?? m.getDefaultUdsSocketPath(),
        { isExplicit: messagingSocketPath !== undefined }
      )

  // ---- Teammate snapshot ----
  if !isBareMode() && isAgentSwarmsEnabled():
    const { captureTeammateModeSnapshot } = await import('./utils/swarm/backends/teammateModeSnapshot.js')
    captureTeammateModeSnapshot()

  // ---- Terminal-backup restoration (interactive only) ----
  if !getIsNonInteractiveSession():
    if isAgentSwarmsEnabled():
      const r = await checkAndRestoreITerm2Backup()
      if r.status === 'restored': console.log(chalk.yellow('Detected an interrupted iTerm2 setup. Your original settings have been restored. You may need to restart iTerm2 for the changes to take effect.'))
      elif r.status === 'failed':  console.error(chalk.red(`Failed to restore iTerm2 settings. Please manually restore your original settings with: defaults import com.googlecode.iterm2 ${r.backupPath}.`))
    try:
      const r = await checkAndRestoreTerminalBackup()
      if r.status === 'restored': console.log(chalk.yellow('Detected an interrupted Terminal.app setup. Your original settings have been restored. You may need to restart Terminal.app for the changes to take effect.'))
      elif r.status === 'failed':  console.error(chalk.red(`Failed to restore Terminal.app settings. Please manually restore your original settings with: defaults import com.apple.Terminal ${r.backupPath}.`))
    catch err: logError(err)

  setCwd(cwd)                                          // MUST be before anything depending on cwd

  const hooksStart = Date.now()
  captureHooksConfigSnapshot()                          // freezes hook config at this point
  initializeFileChangedWatcher(cwd)

  // ---- Worktree branch ----
  if worktreeEnabled:
    hasHook = hasWorktreeCreateHook()
    inGit   = await getIsGit()
    if !hasHook && !inGit:
      stderr 'Error: Can only use --worktree in a git repository, but {cwd} is not a git repository. Configure a WorktreeCreate hook in settings.json to use --worktree with other VCS systems.'
      process.exit(1)
    slug = worktreePRNumber ? `pr-${worktreePRNumber}` : (worktreeName ?? getPlanSlug())
    let tmuxSessionName: string | undefined
    if inGit:
      mainRepoRoot = findCanonicalGitRoot(getCwd())
      if !mainRepoRoot:
        stderr 'Error: Could not determine the main git repository root.'
        process.exit(1)
      if mainRepoRoot !== (findGitRoot(getCwd()) ?? getCwd()):
        process.chdir(mainRepoRoot); setCwd(mainRepoRoot)
      tmuxSessionName = tmuxEnabled ? generateTmuxSessionName(mainRepoRoot, worktreeBranchName(slug)) : undefined
    else:
      tmuxSessionName = tmuxEnabled ? generateTmuxSessionName(getCwd(), worktreeBranchName(slug)) : undefined
    try:
      worktreeSession = await createWorktreeForSession(getSessionId(), slug, tmuxSessionName, worktreePRNumber ? { prNumber: worktreePRNumber } : undefined)
    catch error:
      stderr `Error creating worktree: ${errorMessage(error)}`; process.exit(1)
    logEvent('tengu_worktree_created', { tmux_enabled: tmuxEnabled })
    if tmuxEnabled && tmuxSessionName:
      tmuxResult = await createTmuxSessionForWorktree(tmuxSessionName, worktreeSession.worktreePath)
      if tmuxResult.created: console.log(chalk.green(`Created tmux session: ${tmuxSessionName}\nTo attach: tmux attach -t ${tmuxSessionName}`))
      else: console.error(chalk.yellow(`Warning: Failed to create tmux session: ${tmuxResult.error}`))
    process.chdir(worktreeSession.worktreePath); setCwd(worktreeSession.worktreePath)
    setOriginalCwd(getCwd()); setProjectRoot(getCwd())   // --worktree IS the session's project
    saveWorktreeState(worktreeSession)
    clearMemoryFileCaches()                              // originalCwd changed → drop CLAUDE.md cache
    updateHooksConfigSnapshot()                          // re-snapshot hooks from the new dir

  logForDiagnosticsNoPII('info', 'setup_background_jobs_starting')

  if !isBareMode():
    initSessionMemory()                                  // sync hook registration; gate check is lazy
    if feature('CONTEXT_COLLAPSE'):
      require('./services/contextCollapse/index.js').initContextCollapse()
  void lockCurrentVersion()
  logForDiagnosticsNoPII('info', 'setup_background_jobs_launched')

  profileCheckpoint('setup_before_prefetch')
  logForDiagnosticsNoPII('info', 'setup_prefetch_starting')

  skipPluginPrefetch = (getIsNonInteractiveSession() && isEnvTruthy(process.env.CLAUDE_CODE_SYNC_PLUGIN_INSTALL))
                       || isBareMode()
  if !skipPluginPrefetch: void getCommands(getProjectRoot())
  void import('./utils/plugins/loadPluginHooks.js').then(m => {
    if !skipPluginPrefetch:
      void m.loadPluginHooks()
      m.setupPluginHookHotReload()
  })

  if !isBareMode():
    if process.env.USER_TYPE === 'ant':
      // Auto-undercover: prime repo classification cache so default of undercover-ON
      // can be overridden when repo turns out to be internal.
      void import('./utils/commitAttribution.js').then(async m => {
        if await m.isInternalModelRepo():
          (await import('./constants/systemPromptSections.js')).clearSystemPromptSections()
      })
    if feature('COMMIT_ATTRIBUTION'):
      // Defer to next tick so the git subprocess spawn runs after first render
      setImmediate(() => {
        void import('./utils/attributionHooks.js').then(({ registerAttributionHooks }) => registerAttributionHooks())
      })
    void import('./utils/sessionFileAccessHooks.js').then(m => m.registerSessionFileAccessHooks())
    if feature('TEAMMEM'):
      void import('./services/teamMemorySync/watcher.js').then(m => m.startTeamMemoryWatcher())

  initSinks()                                           // attach error log + analytics sinks; drains queued events

  logEvent('tengu_started', {})                          // session-success-rate denominator

  void prefetchApiKeyFromApiKeyHelperIfSafe(getIsNonInteractiveSession())
  profileCheckpoint('setup_after_prefetch')

  if !isBareMode():
    const { hasReleaseNotes } = await checkForReleaseNotes(getGlobalConfig().lastReleaseNotesSeen)
    if hasReleaseNotes: await getRecentActivity()      // up to 10 session JSONL files

  // ---- Dangerous-permissions sandbox enforcement ----
  if permissionMode === 'bypassPermissions' || allowDangerouslySkipPermissions:
    // Block sudo/root unless inside Docker/Bubblewrap/IS_SANDBOX
    if process.platform !== 'win32' && typeof process.getuid === 'function' && process.getuid() === 0
       && process.env.IS_SANDBOX !== '1' && !isEnvTruthy(process.env.CLAUDE_CODE_BUBBLEWRAP):
      console.error('--dangerously-skip-permissions cannot be used with root/sudo privileges for security reasons')
      process.exit(1)
    // ANT: hard-gate to sandbox AND no-internet
    if process.env.USER_TYPE === 'ant'
       && process.env.CLAUDE_CODE_ENTRYPOINT !== 'local-agent'
       && process.env.CLAUDE_CODE_ENTRYPOINT !== 'claude-desktop':
      [isDocker, hasInternet] = await Promise.all([envDynamic.getIsDocker(), env.hasInternetAccess()])
      isBubblewrap = envDynamic.getIsBubblewrapSandbox()
      isSandbox    = process.env.IS_SANDBOX === '1'
      isSandboxed  = isDocker || isBubblewrap || isSandbox
      if !isSandboxed || hasInternet:
        console.error(`--dangerously-skip-permissions can only be used in Docker/sandbox containers with no internet access but got Docker: ${isDocker}, Bubblewrap: ${isBubblewrap}, IS_SANDBOX: ${isSandbox}, hasInternet: ${hasInternet}`)
        process.exit(1)

  if process.env.NODE_ENV === 'test': return

  // ---- tengu_exit beacon for the PREVIOUS session ----
  projectConfig = getCurrentProjectConfig()
  if projectConfig.lastCost !== undefined && projectConfig.lastDuration !== undefined:
    logEvent('tengu_exit', {
      last_session_cost,
      last_session_api_duration,
      last_session_tool_duration,
      last_session_duration,
      last_session_lines_added,
      last_session_lines_removed,
      last_session_total_input_tokens,
      last_session_total_output_tokens,
      last_session_total_cache_creation_input_tokens,
      last_session_total_cache_read_input_tokens,
      last_session_fps_average,
      last_session_fps_low_1_pct,
      last_session_id,
      ...projectConfig.lastSessionMetrics,
    })
    // NOT cleared — needed for cost restoration when resuming sessions
```

### 5.7 Commander program build (`run()` in `main.tsx:884-4506`)

Top-level structure:

1. Build `program` with `createSortedHelpConfig` (sortSubcommands, sortOptions, custom compareOptions by long-or-short name) and `enablePositionalOptions()`.
2. Register `preAction` hook (see §5.4 outline).
3. Register the **default command** (no subcommand): `.name('claude').description(...).argument('[prompt]')` plus ~70 root flags. Full flag table inline in §6.5.
4. Register `--hard-fail` if `feature('HARD_FAIL')`, hidden.
5. **Print mode short-circuit** (`main.tsx:3875-3892`): if `process.argv.includes('-p') || '--print'` and there is no `cc://` URL in argv, **skip all subcommand registration** (saves ~65ms — the `isBridgeEnabled()` settings Zod parse + sync `security` keychain subprocess that the registration would trigger), call `program.parseAsync(process.argv)`, and `return program`.
6. Otherwise, register subcommands in source order:
   - `mcp` (parent) with subcommands: `serve`, `add`, `xaa-idp` (if `isXaaEnabled()`), `remove`, `list`, `get`, `add-json`, `add-from-claude-desktop`, `reset-project-choices`.
   - `server` (gated on `feature('DIRECT_CONNECT')`).
   - `ssh <host> [dir]` (gated on `feature('SSH_REMOTE')`) — stub for help only; argv rewrite handles real flow.
   - `open <cc-url>` (gated on `feature('DIRECT_CONNECT')`) — internal headless cc:// handler.
   - `auth` family.
   - `plugin` family (alias `plugins`): `validate`, `list`, `marketplace add/list/remove/update`, `install`, `uninstall`, `enable`, `disable`, `update`. All take `--cowork` (hidden).
   - `setup-token`.
   - `agents`.
   - `auto-mode {defaults|config|critique}` (gated on `feature('TRANSCRIPT_CLASSIFIER')` AND `getAutoModeEnabledStateIfCached() !== 'disabled'`).
   - `remote-control` (alias `rc`, hidden, gated on `feature('BRIDGE_MODE')`) — stub; argv intercept handles real flow.
   - `assistant [sessionId]` (gated on `feature('KAIROS')`) — stub.
   - `doctor`.
   - `update` (alias `upgrade`).
   - `up` (ANT-only).
   - `rollback [target]` (ANT-only).
   - `install [target]`.
   - ANT-only: `log`, `error`, `export`, `task {create|list|get|update|dir}`, `completion <shell>` (hidden).
7. `await program.parseAsync(process.argv)`; `profileCheckpoint('run_after_parse')`; `profileReport()`.

### 5.8 Default action handler outline (`main.tsx:1007-3870`)

Because of bundling, the action handler is a single ~2860-line function. The control structure (per `profileCheckpoint` markers) is:

```
action(prompt, options) {
  profileCheckpoint('action_handler_start')
  // --bare → CLAUDE_CODE_SIMPLE = '1'                       [main.tsx:1009-1015]
  ...sets options for KAIROS, PROACTIVE, BG_SESSIONS (CLAUDE_CODE_AGENT), task list (CLAUDE_CODE_TASK_LIST_ID), structured output JSON schema
  ...computes effectiveIncludePartialMessages = includePartialMessages || isEnvTruthy(CLAUDE_CODE_INCLUDE_PARTIAL_MESSAGES)
  ...if includeHookEvents || isEnvTruthy(CLAUDE_CODE_REMOTE) → enable hook events
  ...if --no-session-persistence with -p → setSessionPersistenceDisabled(true)
  await getInputPrompt(prompt, inputFormat)                  // §5.9
  profileCheckpoint('action_after_input_prompt')
  ...load tools, agents
  profileCheckpoint('action_tools_loaded')
  ...resolve permission mode (initialPermissionModeFromCLI)
  profileCheckpoint('action_before_setup')
  await setup(cwd, permissionMode, ..., messagingSocketPath: feature('UDS_INBOX') ? options['messaging-socket-path'] : undefined)
  profileCheckpoint('action_after_setup')
  ...if feature('UDS_INBOX'): export $CLAUDE_CODE_MESSAGING_SOCKET
  ...load commands, MCP configs, claude.ai MCP fetch
  profileCheckpoint('action_commands_loaded')
  ...claudeInChrome setup
  ...buildDeepLinkBanner
  ...handleAutoMode/Plan setup
  ...validate auth/login
  ...async MCP connect
  profileCheckpoint('before_validateForceLoginOrg')
  profileCheckpoint('before_connectMcp')
  profileCheckpoint('after_connectMcp')
  profileCheckpoint('after_connectMcp_claudeai')
  // ---- MODE BRANCH ----
  if -p mode:
    profileCheckpoint('before_print_import')
    const { runHeadless, runHeadlessStreaming } = await import('./cli/print.js')
    profileCheckpoint('after_print_import')
    runHeadless(...)            // never returns — exits via gracefulShutdown
  else:
    if feature('DIRECT_CONNECT') && _pendingConnect?.url:
      ...createDirectConnectSession(...) → launchRepl(...)
    elif feature('SSH_REMOTE') && _pendingSSH?.host:
      ...remote SSH session → launchRepl(...)
    elif feature('KAIROS') && _pendingAssistantChat:
      ...launchAssistantSessionChooser → launchRepl(...)
    else:
      ...possibly --resume / --continue / --from-pr / teleport → launchRepl(...)
  profileCheckpoint('action_after_hooks')
  if feature('COORDINATOR_MODE'): coordinatorModeModule.activate?
  if feature('LODESTONE'): logEvent('tengu_deep_link_opened', {...})
}
```

### 5.9 Stdin handling (`getInputPrompt`)

```ts
// main.tsx:857-883
async function getInputPrompt(prompt: string, inputFormat: 'text' | 'stream-json'): Promise<string | AsyncIterable<string>>
```

```
if !process.stdin.isTTY && !process.argv.includes('mcp'):    // mcp uses stdin for protocol
  if inputFormat === 'stream-json': return process.stdin
  process.stdin.setEncoding('utf8')
  data = ''
  process.stdin.on('data', chunk => data += chunk)
  timedOut = await peekForStdinData(process.stdin, 3000)
  process.stdin.off('data', onData)
  if timedOut:
    stderr 'Warning: no stdin data received in 3s, proceeding without it. ' +
           'If piping from a slow command, redirect stdin explicitly: < /dev/null to skip, or wait longer.'
  return [prompt, data].filter(Boolean).join('\n')
return prompt
```

### 5.10 Argv-rewrite ordering invariants

The pre-`run()` argv rewriting in `main()` MUST execute in the documented order: DIRECT_CONNECT → LODESTONE → KAIROS → SSH_REMOTE → mode classification. The downstream `_pendingX` slots in `main.tsx:548-584` are module-scope `const` records initialized once at module load (gated by `feature(...)` for tree-shaking); the action handler reads them at `:3156` (cc:// → DirectConnect), `:3193` (SSH), and `:3259` (assistant). If a feature flag is off, the corresponding `_pendingX` is `undefined` and the handler branch is dead code (DCE).

### 5.11 `startMCPServer` (entrypoints/mcp.ts:35-196)

```
startMCPServer(cwd, debug, verbose):
  readFileStateCache = createFileStateCacheWithSizeLimit(100)   // 25MB cap
  setCwd(cwd)
  server = new Server({ name: 'claude/tengu', version: MACRO.VERSION }, { capabilities: { tools: {} } })
  server.setRequestHandler(ListToolsRequestSchema, async () => {
    permCtx = getEmptyToolPermissionContext()
    tools  = getTools(permCtx)
    return { tools: await Promise.all(tools.map(async tool => {
      outputSchema = undefined
      if tool.outputSchema:
        converted = zodToJsonSchema(tool.outputSchema)
        if converted is object && converted.type === 'object': outputSchema = converted
        // else: silently skip (MCP SDK requires type: 'object' at root; gh-issue 8014)
      return {
        ...tool,
        description: await tool.prompt({ getToolPermissionContext: async () => permCtx, tools, agents: [] }),
        inputSchema: zodToJsonSchema(tool.inputSchema),
        outputSchema,
      }
    }))}
  })
  server.setRequestHandler(CallToolRequestSchema, async ({ params: { name, arguments: args }}) => {
    permCtx = getEmptyToolPermissionContext()
    tools = getTools(permCtx)
    tool  = findToolByName(tools, name)
    if !tool: throw new Error(`Tool ${name} not found`)
    toolUseContext = { abortController: createAbortController(), options: { commands: MCP_COMMANDS, tools, mainLoopModel, thinkingConfig: { type: 'disabled' }, mcpClients: [], mcpResources: {}, isNonInteractiveSession: true, debug, verbose, agentDefinitions: { activeAgents: [], allAgents: [] } }, getAppState: () => getDefaultAppState(), setAppState: () => {}, messages: [], readFileState: readFileStateCache, setInProgressToolUseIDs: () => {}, setResponseLength: () => {}, updateFileHistoryState: () => {}, updateAttributionState: () => {} }
    try:
      if !tool.isEnabled(): throw new Error(`Tool ${name} is not enabled`)
      vr = await tool.validateInput?.(args ?? {}, toolUseContext)
      if vr && !vr.result: throw new Error(`Tool ${name} input is invalid: ${vr.message}`)
      result = await tool.call(args ?? {}, toolUseContext, hasPermissionsToUseTool, createAssistantMessage({ content: [] }))
      return { content: [{ type: 'text', text: typeof result === 'string' ? result : jsonStringify(result.data) }] }
    catch error:
      logError(error)
      parts = error instanceof Error ? getErrorParts(error) : [String(error)]
      errorText = parts.filter(Boolean).join('\n').trim() || 'Error'
      return { isError: true, content: [{ type: 'text', text: errorText }] }
  })
  await new StdioServerTransport(); await server.connect(transport)
```

### 5.12 `loadSettingsFromFlag` (`main.tsx:432-483`)

```
loadSettingsFromFlag(settingsFile):
  trim = settingsFile.trim()
  looksLikeJson = trim.startsWith('{') && trim.endsWith('}')
  if looksLikeJson:
    parsed = safeParseJSON(trim)
    if !parsed: stderr 'Error: Invalid JSON provided to --settings'; process.exit(1)
    // Use content-hash (NOT random UUID) so the same JSON produces the same path
    // across subprocess boundaries — random UUID would invalidate Anthropic API
    // prompt cache (settings path leaks into Bash sandbox denyWithinAllow which
    // is part of the tool description sent to the API; ~12x token cost penalty).
    settingsPath = generateTempFilePath('claude-settings', '.json', { contentHash: trim })
    writeFileSync_DEPRECATED(settingsPath, trim, 'utf8')
  else:
    { resolvedPath } = safeResolvePath(getFsImplementation(), settingsFile)
    try: readFileSync(resolvedPath, 'utf8')
    catch e:
      if isENOENT(e): stderr `Error: Settings file not found: ${resolvedPath}`; process.exit(1)
      throw e
    settingsPath = resolvedPath
  setFlagSettingsPath(settingsPath)
  resetSettingsCache()
```

### 5.13 `eagerLoadSettings` (`main.tsx:498-516`)

```
eagerLoadSettings():
  profileCheckpoint('eagerLoadSettings_start')
  settingsFile = eagerParseCliFlag('--settings')           // bypasses Commander
  if settingsFile: loadSettingsFromFlag(settingsFile)
  ssArg = eagerParseCliFlag('--setting-sources')
  if ssArg !== undefined:
    sources = parseSettingSourcesFlag(ssArg)               // user/project/local
    setAllowedSettingSources(sources)
    resetSettingsCache()
  profileCheckpoint('eagerLoadSettings_end')
```

### 5.14 `isBeingDebugged` (`main.tsx:232-263`) — startup guard

```
isBeingDebugged():
  isBun = isRunningWithBun()
  hasInspectArg = process.execArgv.some(arg => isBun ? /--inspect(-brk)?/.test(arg) : /--inspect(-brk)?|--debug(-brk)?/.test(arg))
  hasInspectEnv = process.env.NODE_OPTIONS && /--inspect(-brk)?|--debug(-brk)?/.test(process.env.NODE_OPTIONS)
  try:
    inspector = (global as any).require('inspector')
    return !!inspector.url() || hasInspectArg || hasInspectEnv
  catch: return hasInspectArg || hasInspectEnv

// At main.tsx:266 — top-level side-effect, before main() call
if "external" !== 'ant' && isBeingDebugged():
  process.exit(1)            // public binary refuses to run under inspector/debugger
```

### 5.15 `startDeferredPrefetches` (`main.tsx:388-431`)

Called by the action handler **after first render** (interactive mode).

```
startDeferredPrefetches():
  if isEnvTruthy(CLAUDE_CODE_EXIT_AFTER_FIRST_RENDER) || isBareMode(): return
  void initUser()
  void getUserContext()
  prefetchSystemContextIfSafe()                            // git status only if trust accepted or non-interactive
  void getRelevantTips()
  if CLAUDE_CODE_USE_BEDROCK && !CLAUDE_CODE_SKIP_BEDROCK_AUTH: void prefetchAwsCredentialsAndBedRockInfoIfSafe()
  if CLAUDE_CODE_USE_VERTEX && !CLAUDE_CODE_SKIP_VERTEX_AUTH: void prefetchGcpCredentialsIfSafe()
  void countFilesRoundedRg(getCwd(), AbortSignal.timeout(3000), [])
  void initializeAnalyticsGates()
  void prefetchOfficialMcpUrls()
  void refreshModelCapabilities()
  void settingsChangeDetector.initialize()
  if !isBareMode(): void skillChangeDetector.initialize()
  if "external" === 'ant':
    void import('./utils/eventLoopStallDetector.js').then(m => m.startEventLoopStallDetector())
```

### 5.16 `prefetchSystemContextIfSafe` (`main.tsx:360-380`)

```
if getIsNonInteractiveSession(): void getSystemContext(); return  // -p implicitly trusts cwd
elif checkHasTrustDialogAccepted(): void getSystemContext()
else: log 'prefetch_system_context_skipped_no_trust' and don't prefetch (git can run hooks → arbitrary code)
```

---

## 6. Verbatim Assets

### 6.1 Pre-module-eval ordering header (verbatim, `src/main.tsx:1-20`)

```ts
// These side-effects must run before all other imports:
// 1. profileCheckpoint marks entry before heavy module evaluation begins
// 2. startMdmRawRead fires MDM subprocesses (plutil/reg query) so they run in
//    parallel with the remaining ~135ms of imports below
// 3. startKeychainPrefetch fires both macOS keychain reads (OAuth + legacy API
//    key) in parallel — isRemoteManagedSettingsEligible() otherwise reads them
//    sequentially via sync spawn inside applySafeConfigEnvironmentVariables()
//    (~65ms on every macOS startup)
import { profileCheckpoint, profileReport } from './utils/startupProfiler.js';

// eslint-disable-next-line custom-rules/no-top-level-side-effects
profileCheckpoint('main_tsx_entry');
import { startMdmRawRead } from './utils/settings/mdm/rawRead.js';

// eslint-disable-next-line custom-rules/no-top-level-side-effects
startMdmRawRead();
import { ensureKeychainPrefetchCompleted, startKeychainPrefetch } from './utils/secureStorage/keychainPrefetch.js';

// eslint-disable-next-line custom-rules/no-top-level-side-effects
startKeychainPrefetch();
```

### 6.2 User-facing error and warning strings (verbatim)

| ID | Location | Text |
|---|---|---|
| E-NODE-VER | `setup.ts:74-77` | `Error: Claude Code requires Node.js version 18 or higher.` (chalk red bold; printed to `console.error`) |
| E-WT-NOT-GIT | `setup.ts:182-187` | `Error: Can only use --worktree in a git repository, but {cwd} is not a git repository. Configure a WorktreeCreate hook in settings.json to use --worktree with other VCS systems.\n` (chalk red, with `cwd` bolded) |
| E-WT-NO-ROOT | `setup.ts:206-209` | `Error: Could not determine the main git repository root.\n` (chalk red) |
| E-WT-CREATE | `setup.ts:240-241` | `Error creating worktree: ${errorMessage(error)}\n` (chalk red) |
| E-DSP-ROOT | `setup.ts:411-412` | `--dangerously-skip-permissions cannot be used with root/sudo privileges for security reasons` |
| E-DSP-SANDBOX | `setup.ts:436-438` | `--dangerously-skip-permissions can only be used in Docker/sandbox containers with no internet access but got Docker: ${isDocker}, Bubblewrap: ${isBubblewrap}, IS_SANDBOX: ${isSandbox}, hasInternet: ${hasInternet}` |
| W-ITERM2-RESTORED | `setup.ts:122-124` | `Detected an interrupted iTerm2 setup. Your original settings have been restored. You may need to restart iTerm2 for the changes to take effect.` (chalk yellow) |
| E-ITERM2-FAILED | `setup.ts:128-131` | `Failed to restore iTerm2 settings. Please manually restore your original settings with: defaults import com.googlecode.iterm2 ${restoredIterm2Backup.backupPath}.` (chalk red) |
| W-TERM-RESTORED | `setup.ts:140-144` | `Detected an interrupted Terminal.app setup. Your original settings have been restored. You may need to restart Terminal.app for the changes to take effect.` (chalk yellow) |
| E-TERM-FAILED | `setup.ts:148-151` | `Failed to restore Terminal.app settings. Please manually restore your original settings with: defaults import com.apple.Terminal ${restoredTerminalBackup.backupPath}.` (chalk red) |
| I-TMUX-CREATED | `setup.ts:256-259` | `Created tmux session: ${chalk.bold(tmuxSessionName)}\nTo attach: ${chalk.bold('tmux attach -t ${tmuxSessionName}')}` (chalk green) |
| W-TMUX-FAILED | `setup.ts:263-266` | `Warning: Failed to create tmux session: ${tmuxResult.error}` (chalk yellow) |
| E-CONFIG-PARSE-NONI | `init.ts:222` | `Configuration error in ${error.filePath}: ${error.message}\n` (process.stderr; non-interactive only, then `gracefulShutdownSync(1)`) |
| W-PROXY-INIT-FAIL | `init.ts:178-181` | `[init] upstreamproxy init failed: ${err}; continuing without proxy` (debug log only) |
| E-INVALID-SETTINGS-JSON | `main.tsx:441` | `Error: Invalid JSON provided to --settings\n` (chalk red) |
| E-SETTINGS-NOT-FOUND | `main.tsx:467` | `Error: Settings file not found: ${resolvedSettingsPath}\n` (chalk red) |
| E-SETTINGS-PROCESS | `main.tsx:480` | `Error processing settings: ${errorMessage(error)}\n` (chalk red) |
| E-SETTING-SOURCES-PROCESS | `main.tsx:493` | `Error processing --setting-sources: ${errorMessage(error)}\n` (chalk red) |
| E-SSH-NO-PRINT | `main.tsx:787` | `Error: headless (-p/--print) mode is not supported with claude ssh\n` (process.stderr; then `gracefulShutdownSync(1)`) |
| W-STDIN-TIMEOUT | `main.tsx:878` | `Warning: no stdin data received in 3s, proceeding without it. If piping from a slow command, redirect stdin explicitly: < /dev/null to skip, or wait longer.\n` |
| U-ASSISTANT-USAGE | `main.tsx:~4341` | `Usage: claude assistant [sessionId]\n\nAttach the REPL as a viewer client to a running bridge session.\nOmit sessionId to discover and pick from available sessions.\n` |
| U-SSH-USAGE | `main.tsx:~4053` | `Usage: claude ssh <user@host \| ssh-config-alias> [dir]\n\nRuns Claude Code on a remote Linux host. You don't need to install\nanything on the remote or run \`claude auth login\` there — the binary is\ndeployed over SSH and API auth tunnels back through your local machine.\n` |
| E-SERVER-EXISTS | `main.tsx:~4002` | `A claude server is already running (pid ${existing.pid}) at ${existing.httpUrl}\n` |

### 6.3 Pre-rewrite/argv guard regexes (verbatim)

```
// main.tsx:242 (Bun arg detection)
/--inspect(-brk)?/

// main.tsx:245 (Node arg detection — also catches legacy --debug)
/--inspect(-brk)?|--debug(-brk)?/

// main.tsx:250 (NODE_OPTIONS env detection — same shape)
/--inspect(-brk)?|--debug(-brk)?/

// setup.ts:70 (Node version match)
/^v(\d+)\./

// cli/ndjsonSafeStringify.ts:16 (NDJSON-safe escape)
/ | /g
```

### 6.4 Sandbox Zod schemas — verbatim (`entrypoints/sandboxTypes.ts`)

```ts
export const SandboxNetworkConfigSchema = lazySchema(() =>
  z
    .object({
      allowedDomains: z.array(z.string()).optional(),
      allowManagedDomainsOnly: z
        .boolean()
        .optional()
        .describe(
          'When true (and set in managed settings), only allowedDomains and WebFetch(domain:...) allow rules from managed settings are respected. ' +
            'User, project, local, and flag settings domains are ignored. Denied domains are still respected from all sources.',
        ),
      allowUnixSockets: z
        .array(z.string())
        .optional()
        .describe(
          'macOS only: Unix socket paths to allow. Ignored on Linux (seccomp cannot filter by path).',
        ),
      allowAllUnixSockets: z
        .boolean()
        .optional()
        .describe(
          'If true, allow all Unix sockets (disables blocking on both platforms).',
        ),
      allowLocalBinding: z.boolean().optional(),
      httpProxyPort: z.number().optional(),
      socksProxyPort: z.number().optional(),
    })
    .optional(),
)

export const SandboxFilesystemConfigSchema = lazySchema(() =>
  z
    .object({
      allowWrite: z.array(z.string()).optional().describe(
        'Additional paths to allow writing within the sandbox. ' +
        'Merged with paths from Edit(...) allow permission rules.',
      ),
      denyWrite: z.array(z.string()).optional().describe(
        'Additional paths to deny writing within the sandbox. ' +
        'Merged with paths from Edit(...) deny permission rules.',
      ),
      denyRead: z.array(z.string()).optional().describe(
        'Additional paths to deny reading within the sandbox. ' +
        'Merged with paths from Read(...) deny permission rules.',
      ),
      allowRead: z.array(z.string()).optional().describe(
        'Paths to re-allow reading within denyRead regions. ' +
        'Takes precedence over denyRead for matching paths.',
      ),
      allowManagedReadPathsOnly: z.boolean().optional().describe(
        'When true (set in managed settings), only allowRead paths from policySettings are used.',
      ),
    })
    .optional(),
)

export const SandboxSettingsSchema = lazySchema(() =>
  z
    .object({
      enabled: z.boolean().optional(),
      failIfUnavailable: z.boolean().optional().describe(
        'Exit with an error at startup if sandbox.enabled is true but the sandbox cannot start ' +
        '(missing dependencies, unsupported platform, or platform not in enabledPlatforms). ' +
        'When false (default), a warning is shown and commands run unsandboxed. ' +
        'Intended for managed-settings deployments that require sandboxing as a hard gate.',
      ),
      // Note: enabledPlatforms is an undocumented setting read via .passthrough()
      // It restricts sandboxing to specific platforms (e.g., ["macos"]).
      autoAllowBashIfSandboxed: z.boolean().optional(),
      allowUnsandboxedCommands: z.boolean().optional().describe(
        'Allow commands to run outside the sandbox via the dangerouslyDisableSandbox parameter. ' +
        'When false, the dangerouslyDisableSandbox parameter is completely ignored and all commands must run sandboxed. ' +
        'Default: true.',
      ),
      network: SandboxNetworkConfigSchema(),
      filesystem: SandboxFilesystemConfigSchema(),
      ignoreViolations: z.record(z.string(), z.array(z.string())).optional(),
      enableWeakerNestedSandbox: z.boolean().optional(),
      enableWeakerNetworkIsolation: z.boolean().optional().describe(
        'macOS only: Allow access to com.apple.trustd.agent in the sandbox. ' +
        'Needed for Go-based CLI tools (gh, gcloud, terraform, etc.) to verify TLS certificates ' +
        'when using httpProxyPort with a MITM proxy and custom CA. ' +
        '**Reduces security** — opens a potential data exfiltration vector through the trustd service. Default: false',
      ),
      excludedCommands: z.array(z.string()).optional(),
      ripgrep: z.object({
          command: z.string(),
          args: z.array(z.string()).optional(),
        })
        .optional()
        .describe('Custom ripgrep configuration for bundled ripgrep support'),
    })
    .passthrough(),
)
```

### 6.5 Default-command flag table (verbatim help text from `main.tsx:968-1006`, paraphrased only where help text spans multiple paragraphs)

| Flag | Type | Description (verbatim help) |
|---|---|---|
| `[prompt]` | positional `string` | `Your prompt` |
| `-h, --help` | help | `Display help for command` |
| `-d, --debug [filter]` | bool/string | `Enable debug mode with optional category filtering (e.g., "api,hooks" or "!1p,!file")` |
| `-d2e, --debug-to-stderr` | bool (hidden) | `Enable debug mode (to stderr)` |
| `--debug-file <path>` | string | `Write debug logs to a specific file path (implicitly enables debug mode)` |
| `--verbose` | bool | `Override verbose mode setting from config` |
| `-p, --print` | bool | `Print response and exit (useful for pipes). Note: The workspace trust dialog is skipped when Claude is run with the -p mode. Only use this flag in directories you trust.` |
| `--bare` | bool | `Minimal mode: skip hooks, LSP, plugin sync, attribution, auto-memory, background prefetches, keychain reads, and CLAUDE.md auto-discovery. Sets CLAUDE_CODE_SIMPLE=1. Anthropic auth is strictly ANTHROPIC_API_KEY or apiKeyHelper via --settings (OAuth and keychain are never read). 3P providers (Bedrock/Vertex/Foundry) use their own credentials. Skills still resolve via /skill-name. Explicitly provide context via: --system-prompt[-file], --append-system-prompt[-file], --add-dir (CLAUDE.md dirs), --mcp-config, --settings, --agents, --plugin-dir.` |
| `--init` | bool (hidden) | `Run Setup hooks with init trigger, then continue` |
| `--init-only` | bool (hidden) | `Run Setup and SessionStart:startup hooks, then exit` |
| `--maintenance` | bool (hidden) | `Run Setup hooks with maintenance trigger, then continue` |
| `--output-format <format>` | choices `text`/`json`/`stream-json` | `Output format (only works with --print): "text" (default), "json" (single result), or "stream-json" (realtime streaming)` |
| `--json-schema <schema>` | string | `JSON Schema for structured output validation. Example: {"type":"object","properties":{"name":{"type":"string"}},"required":["name"]}` |
| `--include-hook-events` | bool | `Include all hook lifecycle events in the output stream (only works with --output-format=stream-json)` |
| `--include-partial-messages` | bool | `Include partial message chunks as they arrive (only works with --print and --output-format=stream-json)` |
| `--input-format <format>` | choices `text`/`stream-json` | `Input format (only works with --print): "text" (default), or "stream-json" (realtime streaming input)` |
| `--mcp-debug` | bool | `[DEPRECATED. Use --debug instead] Enable MCP debug mode (shows MCP server errors)` |
| `--dangerously-skip-permissions` | bool | `Bypass all permission checks. Recommended only for sandboxes with no internet access.` |
| `--allow-dangerously-skip-permissions` | bool | `Enable bypassing all permission checks as an option, without it being enabled by default. Recommended only for sandboxes with no internet access.` |
| `--thinking <mode>` | choices `enabled`/`adaptive`/`disabled` (hidden) | `Thinking mode: enabled (equivalent to adaptive), disabled` |
| `--max-thinking-tokens <tokens>` | number (hidden) | `[DEPRECATED. Use --thinking instead for newer models] Maximum number of thinking tokens (only works with --print)` |
| `--max-turns <turns>` | number (hidden) | `Maximum number of agentic turns in non-interactive mode. ...` |
| `--max-budget-usd <amount>` | number (hidden, validated >0) | `Maximum dollar amount to spend on API calls (only works with --print)` |
| `--task-budget <tokens>` | positive integer (hidden) | `API-side task budget in tokens (output_config.task_budget)` |
| `--replay-user-messages` | bool | `Re-emit user messages from stdin back on stdout for acknowledgment (only works with --input-format=stream-json and --output-format=stream-json)` |
| `--enable-auth-status` | bool (hidden, default false) | `Enable auth status messages in SDK mode` |
| `--allowedTools, --allowed-tools <tools...>` | string[] | `Comma or space-separated list of tool names to allow (e.g. "Bash(git:*) Edit")` |
| `--tools <tools...>` | string[] | `Specify the list of available tools from the built-in set. Use "" to disable all tools, "default" to use all tools, or specify tool names (e.g. "Bash,Edit,Read").` |
| `--disallowedTools, --disallowed-tools <tools...>` | string[] | `Comma or space-separated list of tool names to deny (e.g. "Bash(git:*) Edit")` |
| `--mcp-config <configs...>` | string[] | `Load MCP servers from JSON files or strings (space-separated)` |
| `--permission-prompt-tool <tool>` | string (hidden) | `MCP tool to use for permission prompts (only works with --print)` |
| `--system-prompt <prompt>` | string | `System prompt to use for the session` |
| `--system-prompt-file <file>` | string (hidden) | `Read system prompt from a file` |
| `--append-system-prompt <prompt>` | string | `Append a system prompt to the default system prompt` |
| `--append-system-prompt-file <file>` | string (hidden) | `Read system prompt from a file and append to the default system prompt` |
| `--permission-mode <mode>` | choices `PERMISSION_MODES` | `Permission mode to use for the session` |
| `-c, --continue` | bool | `Continue the most recent conversation in the current directory` |
| `-r, --resume [value]` | string\|true | `Resume a conversation by session ID, or open interactive picker with optional search term` |
| `--fork-session` | bool | `When resuming, create a new session ID instead of reusing the original (use with --resume or --continue)` |
| `--prefill <text>` | string (hidden) | `Pre-fill the prompt input with text without submitting it` |
| `--deep-link-origin` | bool (hidden) | `Signal that this session was launched from a deep link` |
| `--deep-link-repo <slug>` | string (hidden) | `Repo slug the deep link ?repo= parameter resolved to the current cwd` |
| `--deep-link-last-fetch <ms>` | number (hidden) | `FETCH_HEAD mtime in epoch ms, precomputed by the deep link trampoline` |
| `--from-pr [value]` | string\|true | `Resume a session linked to a PR by PR number/URL, or open interactive picker with optional search term` |
| `--no-session-persistence` | bool | `Disable session persistence - sessions will not be saved to disk and cannot be resumed (only works with --print)` |
| `--resume-session-at <message id>` | string (hidden) | `When resuming, only messages up to and including the assistant message with <message.id> (use with --resume in print mode)` |
| `--rewind-files <user-message-id>` | string (hidden) | `Restore files to state at the specified user message and exit (requires --resume)` |
| `--model <model>` | string | `Model for the current session. Provide an alias for the latest model (e.g. 'sonnet' or 'opus') or a model's full name (e.g. 'claude-sonnet-4-6').` |
| `--effort <level>` | choices `low`/`medium`/`high`/`max` | `Effort level for the current session (low, medium, high, max)` |
| `--agent <agent>` | string | `Agent for the current session. Overrides the 'agent' setting.` |
| `--betas <betas...>` | string[] | `Beta headers to include in API requests (API key users only)` |
| `--fallback-model <model>` | string | `Enable automatic fallback to specified model when default model is overloaded (only works with --print)` |
| `--workload <tag>` | string (hidden) | `Workload tag for billing-header attribution (cc_workload). Process-scoped; set by SDK daemon callers that spawn subprocesses for cron work. (only works with --print)` |
| `--settings <file-or-json>` | string | `Path to a settings JSON file or a JSON string to load additional settings from` |
| `--add-dir <directories...>` | string[] | `Additional directories to allow tool access to` |
| `--ide` | bool | `Automatically connect to IDE on startup if exactly one valid IDE is available` |
| `--strict-mcp-config` | bool | `Only use MCP servers from --mcp-config, ignoring all other MCP configurations` |
| `--session-id <uuid>` | string | `Use a specific session ID for the conversation (must be a valid UUID)` |
| `-n, --name <name>` | string | `Set a display name for this session (shown in /resume and terminal title)` |
| `--agents <json>` | string | `JSON object defining custom agents (e.g. '{"reviewer": {"description": "Reviews code", "prompt": "You are a code reviewer"}}')` |
| `--setting-sources <sources>` | string | `Comma-separated list of setting sources to load (user, project, local).` |
| `--hard-fail` | bool (hidden, gated `feature('HARD_FAIL')`) | `Crash on logError calls instead of silently logging` |

### 6.6 Sub-command roster (verbatim descriptions)

| Subcommand | Gate | Description (verbatim) |
|---|---|---|
| `mcp serve` | none | `Start the Claude Code MCP server` |
| `mcp add` | none | (see spec 23) |
| `mcp xaa-idp` | `isXaaEnabled()` | (XAA IDP login) |
| `mcp remove <name>` | none | `Remove an MCP server` |
| `mcp list` | none | `List configured MCP servers. Note: The workspace trust dialog is skipped and stdio servers from .mcp.json are spawned for health checks. Only use this command in directories you trust.` |
| `mcp get <name>` | none | `Get details about an MCP server. Note: ...` |
| `mcp add-json <name> <json>` | none | `Add an MCP server (stdio or SSE) with a JSON string` |
| `mcp add-from-claude-desktop` | none | `Import MCP servers from Claude Desktop (Mac and WSL only)` |
| `mcp reset-project-choices` | none | `Reset all approved and rejected project-scoped (.mcp.json) servers within this project` |
| `server` | `feature('DIRECT_CONNECT')` | `Start a Claude Code session server` |
| `ssh <host> [dir]` | `feature('SSH_REMOTE')` | `Run Claude Code on a remote host over SSH. Deploys the binary and tunnels API auth back through your local machine — no remote setup needed.` |
| `open <cc-url>` | `feature('DIRECT_CONNECT')` | `Connect to a Claude Code server (internal — use cc:// URLs)` |
| `auth` | none | `Manage authentication` |
| `plugin` (alias `plugins`) | none | `Manage Claude Code plugins` (children: `validate`, `list`, `install`, `uninstall`, `enable`, `disable`, `update`, plus `marketplace add/list/remove/update`) |
| `setup-token` | none | `Set up a long-lived authentication token (requires Claude subscription)` |
| `agents` | none | `List configured agents` |
| `auto-mode` (with `defaults`/`config`/`critique`) | `feature('TRANSCRIPT_CLASSIFIER') && getAutoModeEnabledStateIfCached() !== 'disabled'` | `Inspect auto mode classifier configuration` |
| `remote-control` (alias `rc`, hidden) | `feature('BRIDGE_MODE')` | `Connect your local environment for remote-control sessions via claude.ai/code` |
| `assistant [sessionId]` | `feature('KAIROS')` | `Attach the REPL as a client to a running bridge session. Discovers sessions via API if no sessionId given.` |
| `doctor` | none | `Check the health of your Claude Code auto-updater. Note: The workspace trust dialog is skipped and stdio servers from .mcp.json are spawned for health checks. Only use this command in directories you trust.` |
| `update` (alias `upgrade`) | none | `Check for updates and install if available` |
| `up` | `"external" === 'ant'` | `[ANT-ONLY] Initialize or upgrade the local dev environment using the "# claude up" section of the nearest CLAUDE.md` |
| `rollback [target]` | `"external" === 'ant'` | `[ANT-ONLY] Roll back to a previous release\n\nExamples:\n  claude rollback                                    Go 1 version back from current\n  claude rollback 3                                  Go 3 versions back from current\n  claude rollback 2.0.73-dev.20251217.t190658        Roll back to a specific version` |
| `install [target]` | none | `Install Claude Code native build. Use [target] to specify version (stable, latest, or specific version)` |
| `log [number\|sessionId]` | `"external" === 'ant'` | `[ANT-ONLY] Manage conversation logs.` |
| `error [number]` | `"external" === 'ant'` | `[ANT-ONLY] View error logs. Optionally provide a number (0, -1, -2, etc.) to display a specific log.` |
| `export <source> <outputFile>` | `"external" === 'ant'` | `[ANT-ONLY] Export a conversation to a text file.` (with help text examples) |
| `task {create\|list\|get\|update\|dir}` | `"external" === 'ant'` | `[ANT-ONLY] Manage task list tasks` |
| `completion <shell>` | `"external" === 'ant'` (hidden) | `Generate shell completion script (bash, zsh, or fish)` |

### 6.7 Constants table

| Name | Value | Citation |
|---|---|---|
| `CURRENT_MIGRATION_VERSION` | `11` | `main.tsx:325` |
| `MAX_IN_MEMORY_ERRORS` | `100` | `state.ts:1219` |
| `MAX_SLOW_OPERATIONS` | `10` | `state.ts:1566` |
| `SLOW_OPERATION_TTL_MS` | `10000` | `state.ts:1567` |
| `SCROLL_DRAIN_IDLE_MS` | `150` | `state.ts:794` |
| `READ_FILE_STATE_CACHE_SIZE` | `100` | `entrypoints/mcp.ts:42` |
| `MAX_RECEIVED_UUIDS` | `10_000` | `cli/print.ts:394` |
| `MAX_RESOLVED_TOOL_USE_IDS` | `1000` | `cli/structuredIO.ts:133` |
| `SANDBOX_NETWORK_ACCESS_TOOL_NAME` | `'SandboxNetworkAccess'` | `cli/structuredIO.ts:62` |
| stdin-data peek timeout | `3000` ms | `main.tsx:875` |
| stdin-data warning text | (see W-STDIN-TIMEOUT) | `main.tsx:878` |
| `--idle-timeout` (claude server) default | `'600000'` ms | `main.tsx:~3964` |
| `--max-sessions` (claude server) default | `'32'` | `main.tsx:~3964` |
| MCP server identity | `name: 'claude/tengu', version: MACRO.VERSION` | `entrypoints/mcp.ts:48-51` |
| MCP commands list | `[review]` | `entrypoints/mcp.ts:33` |
| Default `allowedSettingSources` | `['userSettings','projectSettings','localSettings','flagSettings','policySettings']` | `state.ts:313-319` |
| `MCP_COMMANDS` | `[review]` | `entrypoints/mcp.ts:33` |
| `JS_LINE_TERMINATORS` | `/ | /g` | `cli/ndjsonSafeStringify.ts:16` |

### 6.8 Environment variables consumed (verbatim names; semantics inline)

`USER_TYPE`, `NODE_ENV`, `IS_SANDBOX`, `GITHUB_ACTIONS`, `__CFBundleIdentifier` (macOS LaunchServices precise positive signal — see `main.tsx:666`), `NODE_OPTIONS` (debugger detection), `NODE_EXTRA_CA_CERTS`, `CLAUDE_CODE_CLIENT_CERT`, `CLAUDE_CODE_ACTION`, `CLAUDE_CODE_AGENT`, `CLAUDE_CODE_TASK_LIST_ID`, `CLAUDE_CODE_BRIEF`, `CLAUDE_CODE_BUBBLEWRAP`, `CLAUDE_CODE_COORDINATOR_MODE`, `CLAUDE_CODE_DISABLE_TERMINAL_TITLE`, `CLAUDE_CODE_ENTRYPOINT` (`mcp` / `claude-code-github-action` / `sdk-cli` / `cli` / `claude-vscode` / `local-agent` / `claude-desktop` / `remote` / `sdk-ts` / `sdk-py` / `bridge`-via-`CLAUDE_CODE_ENVIRONMENT_KIND`), `CLAUDE_CODE_ENVIRONMENT_KIND` (`bridge` → `setSessionSource('remote-control')`), `CLAUDE_CODE_EXIT_AFTER_FIRST_RENDER`, `CLAUDE_CODE_INCLUDE_PARTIAL_MESSAGES`, `CLAUDE_CODE_PROACTIVE`, `CLAUDE_CODE_QUESTION_PREVIEW_FORMAT` (`'markdown'` / `'html'`), `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_REMOTE_SESSION_ID`, `CLAUDE_CODE_SESSION_ACCESS_TOKEN`, `CLAUDE_CODE_SIMPLE` (set by `--bare` to `'1'`), `CLAUDE_CODE_SKIP_BEDROCK_AUTH`, `CLAUDE_CODE_SKIP_VERTEX_AUTH`, `CLAUDE_CODE_SYNC_PLUGIN_INSTALL`, `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, `CLAUDE_CODE_VERIFY_PLAN` (per registry — see overview §2.5), `CLAUDE_CODE_WEBSOCKET_AUTH_FILE_DESCRIPTOR`, `CLAUDE_CODE_MESSAGING_SOCKET` (exported by `setup` after UDS server starts), `MCP_CLIENT_SECRET` (mcp add-json), `NoDefaultCurrentDirectoryInExePath` (set by main to `'1'`).

### 6.9 OTel counter names (created by `setMeter`)

```
claude_code.session.count                     // 'Count of CLI sessions started'
claude_code.lines_of_code.count               // 'Count of lines of code modified, ...'
claude_code.pull_request.count                // 'Number of pull requests created'
claude_code.commit.count                      // 'Number of git commits created'
claude_code.cost.usage         (unit: 'USD')  // 'Cost of the Claude Code session'
claude_code.token.usage        (unit: 'tokens') // 'Number of tokens used'
claude_code.code_edit_tool.decision           // 'Count of code editing tool permission decisions ...'
claude_code.active_time.total  (unit: 's')    // 'Total active time in seconds'
```
(`state.ts:955-986`).

### 6.10 `profileCheckpoint` ordering inventory (verbatim names, in source order)

`main_tsx_entry`, `main_tsx_imports_loaded`, `eagerLoadSettings_start`, `eagerLoadSettings_end`, `main_function_start`, `main_warning_handler_initialized`, `main_client_type_determined`, `main_before_run`, `main_after_run` (× 2 — at end of `main()` and at end of `run()` for total-time calculation), `run_function_start`, `run_commander_initialized`, `preAction_start`, `preAction_after_mdm`, `preAction_after_init`, `preAction_after_sinks`, `preAction_after_migrations`, `preAction_after_remote_settings`, `preAction_after_settings_sync`, `run_main_options_built`, `run_before_parse`, `run_after_parse`, `action_handler_start`, `action_after_input_prompt`, `action_tools_loaded`, `action_before_setup`, `action_after_setup`, `action_commands_loaded`, `action_mcp_configs_loaded`, `action_after_plugins_init`, `before_validateForceLoginOrg`, `before_connectMcp`, `after_connectMcp`, `after_connectMcp_claudeai`, `before_print_import`, `after_print_import`, `action_after_hooks`, `init_function_start`, `init_configs_enabled`, `init_safe_env_vars_applied`, `init_after_graceful_shutdown`, `init_after_1p_event_logging`, `init_after_oauth_populate`, `init_after_jetbrains_detection`, `init_after_remote_settings_check`, `init_network_configured`, `init_function_end`, `setup_before_prefetch`, `setup_after_prefetch`.

### 6.11 Diagnostic-log identifiers (`logForDiagnosticsNoPII`)

`init_started`, `init_configs_enabled`, `init_safe_env_vars_applied`, `init_mtls_configured`, `init_proxy_configured`, `init_scratchpad_created`, `init_completed`; `setup_started`, `setup_hooks_captured`, `setup_background_jobs_starting`, `setup_background_jobs_launched`, `setup_prefetch_starting`, `worktree_resolved_to_main_repo`; `prefetch_system_context_non_interactive`, `prefetch_system_context_has_trust`, `prefetch_system_context_skipped_no_trust`.

### 6.12 Analytics events emitted from this subsystem

| Event | Citation | Notes |
|---|---|---|
| `tengu_started` | `setup.ts:378` | Session-success-rate denominator. Emitted **immediately after** `initSinks()`, before any throwing prefetch. inc-3694 (P0 CHANGELOG crash) is the historical reason this beacon is here. |
| `tengu_exit` | `setup.ts:454` | Logs the previous session's metrics (cost, durations, tokens, fps). Values are NOT cleared after logging — needed for cost restoration on `--resume`. |
| `tengu_managed_settings_loaded` | `main.tsx:221` | Counts policy-settings keys after `init()`. Wrapped in try/catch — silently ignored on error. |
| `tengu_startup_telemetry` | `main.tsx:310` | Includes `is_git`, `worktree_count`, `gh_auth_status`, `sandbox_enabled`, `are_unsandboxed_commands_allowed`, `is_auto_bash_allowed_if_sandbox_enabled`, `auto_updater_disabled`, `prefers_reduced_motion`, plus `getCertEnvVarTelemetry()` (`has_node_extra_ca_certs`, `has_client_cert`, `has_use_system_ca`, `has_use_openssl_ca`). Skipped if `isAnalyticsDisabled()`. |
| `tengu_init` | `main.tsx:4562-4607` | Per-action-handler entry beacon. Verbatim payload table below. |
| `tengu_worktree_created` | `setup.ts:246` | `{ tmux_enabled }`. |
| `tengu_code_prompt_ignored` | `main.tsx:1020` | (paraphrased — verify in source) |
| `tengu_single_word_prompt` | `main.tsx:1028` | |
| `tengu_claude_in_chrome_setup` / `tengu_claude_in_chrome_setup_failed` | `main.tsx:1536`, `:1553` | |
| `tengu_mcp_channel_flags` | `main.tsx:1713` | |
| `tengu_structured_output_enabled` / `tengu_structured_output_failure` | `main.tsx:1892`, `:1897` | |
| `tengu_agent_flag` | `main.tsx:2069` | |
| `tengu_agent_memory_loaded` | `main.tsx:2158` | |
| `tengu_timer` | `main.tsx:2235` | |
| `tengu_concurrent_sessions` | `main.tsx:2537` | |
| `tengu_startup_manual_model_config` | `main.tsx:2864` | |
| `tengu_continue` | `main.tsx:3114`, `:3129`, `:3149` | Three call sites. |
| `tengu_remote_create_session` / `tengu_remote_create_session_error` / `tengu_remote_create_session_success` | `main.tsx:3418`, `:3426`, `:3431` | |
| `tengu_teleport_interactive_mode` / `tengu_teleport_resume_session` | `main.tsx:3507`, `:3520` | |
| `tengu_session_resumed` | `main.tsx:3602`, `:3608`, `:3614`, `:3643`, `:3649`, `:3656`, `:3677`, `:3692`, `:3698` | Nine call sites — distinguish by call-site context. |
| `tengu_deep_link_opened` | `main.tsx:3783` | |
| `tengu_brief_mode_enabled` | `main.tsx:4647` | |

`tengu_init` payload (verbatim from `main.tsx:4563-4607`):

```
{
  entrypoint: 'claude',
  hasInitialPrompt, hasStdin, verbose, debug, debugToStderr, print,
  outputFormat, inputFormat,
  numAllowedTools, numDisallowedTools, mcpClientCount,
  worktree: worktreeEnabled,
  skipWebFetchPreflight,
  ...(githubActionInputs && { githubActionInputs }),
  dangerouslySkipPermissionsPassed,
  permissionMode,
  modeIsBypass,
  inProtectedNamespace: isInProtectedNamespace(),
  allowDangerouslySkipPermissionsPassed,
  thinkingType: thinkingConfig.type,
  ...(systemPromptFlag && { systemPromptFlag }),
  ...(appendSystemPromptFlag && { appendSystemPromptFlag }),
  is_simple: isBareMode() || undefined,
  is_coordinator: feature('COORDINATOR_MODE') && coordinatorModeModule?.isCoordinatorMode() ? true : undefined,
  ...(assistantActivationPath && { assistantActivationPath }),
  autoUpdatesChannel: getInitialSettings().autoUpdatesChannel ?? 'latest',
  ...("external" === 'ant' ? { relativeProjectPath: relative(gitRoot, cwd) || '.' } : {})
}
```

### 6.13 NDJSON safety helper (verbatim, `cli/ndjsonSafeStringify.ts:30-32`)

```ts
export function ndjsonSafeStringify(value: unknown): string {
  return escapeJsLineTerminators(jsonStringify(value))
}
```

Reasoning (`cli/ndjsonSafeStringify.ts:1-15`): U+2028/U+2029 are valid JSON characters but ECMA-262 §11.3 line terminators; receivers that split NDJSON by JS line-terminator semantics will cut messages mid-string. Replacing with `\uXXXX` is equivalent JSON and parses to the same string.

### 6.14 `cliError` / `cliOk` (verbatim, `cli/exit.ts:18-31`)

```ts
export function cliError(msg?: string): never {
  if (msg) console.error(msg)
  process.exit(1)
  return undefined as never
}

export function cliOk(msg?: string): never {
  if (msg) process.stdout.write(msg + '\n')
  process.exit(0)
  return undefined as never
}
```

Note (`cli/exit.ts:11-16` comment): `: never` return type narrows control flow at call sites without a trailing `return`. `process.exit` is allowed to "return" under tests that spy on it; the sentinel value lets subsequent code under mock not crash.

---

## 7. Side Effects & I/O

### 7.1 Filesystem

- `realpathSync(rawCwd)` at module load (`state.ts:271`).
- `readFileSync` of `--settings <file>` (`main.tsx:464`).
- `writeFileSync_DEPRECATED(<tmp>/claude-settings-<hash>.json)` for `--settings <json>` (`main.tsx:457`).
- Plutil/reg-query subprocess via `startMdmRawRead()` (`main.tsx:13-16`).
- macOS `security` keychain reads via `startKeychainPrefetch()` (`main.tsx:17-20`).
- `git` subprocess via `findCanonicalGitRoot`, `findGitRoot`, `getIsGit` (in `setup.ts` worktree branch and elsewhere).
- `tmux` subprocess via `createTmuxSessionForWorktree` (`setup.ts:250-269`).
- Worktree creation via `createWorktreeForSession` (spec 18).
- `process.chdir(...)` and `setCwd(...)` in worktree branch.
- `~/.claude/` reads/writes through `getGlobalConfig()` / `getCurrentProjectConfig()` / `saveGlobalConfig()`.
- Scratchpad directory creation via `ensureScratchpadDir()` if `isScratchpadEnabled()` (`init.ts:204`).
- `.claude/settings.json` re-read after worktree chdir via `updateHooksConfigSnapshot()` (`setup.ts:284`).
- Server-mode lockfile at `claude server` (`writeServerLock`/`removeServerLock`/`probeRunningServer`).

### 7.2 Network

- `preconnectAnthropicApi()` — TCP+TLS pre-connect to Anthropic API (`init.ts:159`); fire-and-forget; skipped under proxy/mTLS/unix/cloud-provider where the SDK dispatcher would not reuse the global pool.
- `loadRemoteManagedSettings()` (`main.tsx:957`) — non-blocking; failure is fail-open.
- `loadPolicyLimits()` (`main.tsx:958`).
- Optional `uploadUserSettingsInBackground()` (`main.tsx:963-965`) gated on `feature('UPLOAD_USER_SETTINGS')`.
- CCR upstream proxy `initUpstreamProxy()` if `CLAUDE_CODE_REMOTE` (`init.ts:167-183`).
- 1P event logger TCP via `firstPartyEventLogger` (`init.ts:94-105`).
- OAuth refresh inside `populateOAuthAccountInfoIfNeeded` (`init.ts:110`).

### 7.3 Process / signal

- `process.on('exit', resetCursor)` (`main.tsx:595`).
- `process.on('SIGINT', ...)` (`main.tsx:598`) — print mode delegates to `print.ts`'s own SIGINT handler; otherwise `process.exit(0)`.
- `setupGracefulShutdown()` registers terminal-cleanup, telemetry-flush, etc. (`init.ts:87`).
- `gracefulShutdownSync(1)` for SSH no-print-mode error path (`main.tsx:788`).
- `process.exit(...)` for fatal config errors and the dangerous-permissions enforcement.

### 7.4 Trust boundaries

- `applySafeConfigEnvironmentVariables()` (`init.ts:74`) is the **pre-trust** env application; `applyConfigEnvironmentVariables()` (`init.ts:269`) is called only after `waitForRemoteManagedSettingsToLoad()` resolves and is gated on `initializeTelemetryAfterTrust` after trust dialog acceptance.
- `prefetchSystemContextIfSafe()` (`main.tsx:360-380`) refuses to spawn `git` (which can run hooks → arbitrary code) unless `getIsNonInteractiveSession()` (which implies `-p` and the documented "trust dialog skipped" semantics) or `checkHasTrustDialogAccepted()` is true.
- `prefetchApiKeyFromApiKeyHelperIfSafe(getIsNonInteractiveSession())` (`setup.ts:380`) — only runs if trust is already confirmed.
- The `--dangerously-skip-permissions` enforcement (§5.6) is the single hard-gate for bypass mode.

### 7.5 Required external binaries

- `node` (>=18) or `bun` (Bun-specific debug detection at `main.tsx:236-247`).
- `git` (worktree creation, repo detection).
- `tmux` (when `tmuxEnabled`).
- macOS `security` (keychain prefetch).
- `plutil` / `reg query` (MDM raw read on macOS / Windows).
- `defaults` (terminal-backup restoration messaging — only referenced in user-facing recovery instructions, not invoked).

---

## 8. Feature Flags & Variants

This subsystem is the canonical home of feature-flag bookkeeping for the boot path. The full flag matrix lives in spec 00 §6.5 ("89 flags"); here we cover only the deltas that affect boot.

| Flag | Off behavior | On behavior |
|---|---|---|
| `feature('UDS_INBOX')` | No UDS server; `CLAUDE_CODE_MESSAGING_SOCKET` not exported. | `setup()` awaits `startUdsMessaging(...)` before any hook can spawn. Also adds `--messaging-socket-path` to action options. |
| `feature('CONTEXT_COLLAPSE')` | Skip `initContextCollapse`. | Run `initContextCollapse()` at end of setup background jobs. |
| `feature('COMMIT_ATTRIBUTION')` | Skip `registerAttributionHooks`. | `setImmediate(() => registerAttributionHooks())` (deferred to next tick after first render). |
| `feature('TEAMMEM')` | No team-memory watcher. | `void startTeamMemoryWatcher()` after sinks attach. |
| `feature('DIRECT_CONNECT')` | No `cc://` argv rewrite, no `server`/`open` subcommands. | argv rewrite, `_pendingConnect` slot, `server` and `open` commands registered. |
| `feature('SSH_REMOTE')` | No `ssh` argv rewrite, no `ssh` subcommand. | argv rewrite, `_pendingSSH` slot, `ssh` stub. |
| `feature('LODESTONE')` | No `--handle-uri` early exit, no macOS bundle URL handling. | Two early exits before `main()` finishes mode classification. |
| `feature('KAIROS')` | `_pendingAssistantChat` is `undefined`; no `assistant` rewrite; `assistantModule`/`kairosGate` are `null`. | argv rewrite, `assistant [sessionId]` subcommand registered. Also enables KAIROS-specific paths in setup (BriefTool, channels, push notifications). |
| `feature('BRIDGE_MODE')` | No `remote-control` subcommand; `replBridge` not connected. | `remote-control`/`rc` registered (always hidden). |
| `feature('TRANSCRIPT_CLASSIFIER')` | Skip auto-mode migrations and `auto-mode` subcommand. | Run `resetAutoModeOptInForDefaultOffer()` and register `auto-mode {defaults\|config\|critique}` subcommands. |
| `feature('UPLOAD_USER_SETTINGS')` | No background upload. | `void uploadUserSettingsInBackground()` in `preAction`. |
| `feature('COORDINATOR_MODE')` | `coordinatorModeModule` is `null`. | Enables coordinator activation via `CLAUDE_CODE_COORDINATOR_MODE` env. |
| `feature('HARD_FAIL')` | No `--hard-fail` flag visible. | Hidden flag exposed; if set, `logError` crashes instead of silently logging. |

**ANT vs production**:

- `"external" === 'ant'` (literal-substituted at build time so the bundler tree-shakes): 11 ANT-only paths in this subsystem (§2.3 table).
- `process.env.USER_TYPE === 'ant'` (runtime): 4 paths in this subsystem — `state.ts:391-395` (initial state), `state.ts:1570` (`addSlowOperation` early-return), `setup.ts:337-348` (auto-undercover commitAttribution prime), `setup.ts:417-441` (sandbox enforcement on `--dangerously-skip-permissions`).
- The two are **distinct**: `"external" === 'ant'` is a build-time gate that produces an entirely different binary; `USER_TYPE === 'ant'` is a runtime gate, available even in non-ANT builds (the env var can be set externally).
- `--dangerously-skip-permissions` exempts `local-agent` and `claude-desktop` `CLAUDE_CODE_ENTRYPOINT` from the ANT sandbox-or-die check, citing apps#29127 and PR #19116 (`setup.ts:421-425` comments).

**Non-`feature()` env gates** documented above include `CLAUDE_CODE_REMOTE` (CCR upstream proxy), `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` (skip plugin prefetch), `CLAUDE_CODE_BUBBLEWRAP` (Bubblewrap sandbox marker), `CLAUDE_CODE_USE_BEDROCK`/`CLAUDE_CODE_USE_VERTEX` and their `*_SKIP_*_AUTH` companions, `CLAUDE_CODE_DISABLE_TERMINAL_TITLE`, `CLAUDE_CODE_EXIT_AFTER_FIRST_RENDER`, `IS_SANDBOX`, `NODE_ENV === 'test'` (early-return from setup at `:444`).

---

## 9. Error Handling & Edge Cases

- **Config parse errors** (`init.ts:215-237`): `ConfigParseError` is intercepted; non-interactive sessions get a `stderr` message and `gracefulShutdownSync(1)`; interactive sessions show `InvalidConfigDialog` (dialog handles its own `process.exit`). Non-`ConfigParseError` errors rethrow and crash.
- **Inspector / debugger detection** (`main.tsx:266-271`): non-ANT builds `process.exit(1)` if `isBeingDebugged()` returns true. ANT builds skip this guard.
- **Node version <18** (`setup.ts:69-79`): `console.error(chalk.bold.red(...))` and `process.exit(1)`.
- **`--worktree` without git and without `WorktreeCreate` hook** (`setup.ts:181-189`): stderr error and `process.exit(1)`.
- **`--worktree` with no canonical git root** (`setup.ts:204-211`): stderr error and `process.exit(1)`.
- **Worktree creation throws** (`setup.ts:239-243`): stderr error and `process.exit(1)`.
- **`--dangerously-skip-permissions` as root on Unix outside Docker/Bubblewrap/IS_SANDBOX** (`setup.ts:402-414`): stderr error and `process.exit(1)`.
- **`--dangerously-skip-permissions` ANT outside sandbox or with internet** (`setup.ts:417-441`): stderr error and `process.exit(1)`. Exempted: `local-agent`, `claude-desktop` entrypoints.
- **`--settings` JSON parse failure** (`main.tsx:441-443`): stderr and `process.exit(1)`.
- **`--settings` file ENOENT** (`main.tsx:466-468`): stderr and `process.exit(1)`.
- **`--setting-sources` parse error** (`main.tsx:489-495`): stderr and `process.exit(1)`.
- **`claude ssh` with `-p`** (`main.tsx:786-790`): stderr 'Error: headless (-p/--print) mode is not supported with claude ssh' and `gracefulShutdownSync(1)`.
- **`claude server` but server already running** (`main.tsx:~4002`): stderr error and `process.exit(1)`.
- **CCR upstream proxy init failure** (`init.ts:177-181`): logged at `level: 'warn'`, **fail-open** — startup continues without proxy.
- **Terminal-backup restoration error** (`setup.ts:154-157`): logged via `logError`, swallowed (does not crash).
- **`migrateChangelogFromConfig` failure** (`main.tsx:349-351`): swallowed; will retry next startup.
- **Async migrations** in `runMigrations` are sync — the only async one is `migrateChangelogFromConfig` and it is fire-and-forget.
- **Race: `customSessionId` switch happens before any other state mutation** (`setup.ts:82-84`) — must precede `setCwd`, hooks-snapshot, and worktree creation so the worktree session uses the correct session ID.
- **`setCwd()` ordering** (`setup.ts:160-161`): MUST be before any code that depends on cwd. Worktree branch re-calls `setCwd` after `chdir`.
- **`captureHooksConfigSnapshot()` ordering** (`setup.ts:166`): MUST be after `setCwd()` so hooks are loaded from the correct directory; re-run via `updateHooksConfigSnapshot()` after worktree chdir (`setup.ts:284`).
- **`tengu_started` placement** (`setup.ts:378`): MUST be immediately after `initSinks()` and before any prefetch that could throw (inc-3694 root cause).
- **`bundledSkills` race** (`setup.ts:289-292` comment): bundled skills/plugins are registered in `main.tsx` before the parallel `getCommands()` kick — moving them into `setup()` would mean the `await startUdsMessaging` (~20ms) lets `getCommands()` race ahead and memoize an empty list.
- **`feature('COMMIT_ATTRIBUTION')` deferral** (`setup.ts:354-360` comment): the attribution hook install is wrapped in `setImmediate(...)` so the git subprocess spawn runs after first render.
- **`addSlowOperation` editor exception** (`state.ts:1572-1574`): operations whose key includes both `'exec'` and `'claude-prompt-'` are skipped (drafting in $EDITOR is intentionally slow).
- **`getRecentActivity()` cost** (`setup.ts:391`): up to 10 session JSONL files read; gated behind `hasReleaseNotes` and skipped under `--bare`.
- **MCP server output schema dropping** (`entrypoints/mcp.ts:74-82`): output schemas with non-`'object'` root (e.g., `z.union`) are silently skipped rather than rejected; gh-issue 8014.
- **`--print` SIGINT cooperation** (`main.tsx:598-606`): `main()`'s SIGINT handler explicitly defers to `print.ts` when `-p` or `--print` is in argv, so `print.ts` can abort the in-flight query and call `gracefulShutdown` (otherwise the synchronous `process.exit(0)` would preempt cleanup).
- **`gracefulShutdownSync(1)` vs `process.exit(1)`**: `gracefulShutdownSync` runs registered cleanups first; raw `process.exit` does not. The codebase uses `gracefulShutdownSync` in the SSH-no-print path; raw `process.exit` for pre-init errors (Node version, etc.).

---

## 10. Telemetry & Observability

### 10.1 Profile checkpoints

`profileCheckpoint(name)` is the boot tracer (`utils/startupProfiler.js`, owned by 26/42). The full ordered inventory in §6.10 (~50 names). After argv parsing completes, `profileReport()` is called (`main.tsx:~4509`) — sampled to Statsig if enabled.

### 10.2 OTel counters

8 counters created in `setMeter` (§6.9). Counters are lazy: telemetry initialization is deferred to `initializeTelemetryAfterTrust()` (`init.ts:247-286`) so counters may be `null` for the first few events. `setMeter` increments `getSessionCounter()?.add(1)` immediately after creation (`init.ts:338`) because the startup telemetry path runs before async telemetry initialization completes.

### 10.3 Diagnostic logs

`logForDiagnosticsNoPII(level, identifier, payload?)` — see §6.11 for the inventory of identifiers emitted from this subsystem.

### 10.4 Analytics events

See §6.12. Most relevant for boot:

- `tengu_started` (denominator).
- `tengu_init` (per-action-handler beacon with the verbatim payload above).
- `tengu_startup_telemetry` (deferred git/gh/sandbox/cert info).
- `tengu_managed_settings_loaded` (policy keys).
- `tengu_exit` (previous session's metrics — replayed at setup time).
- `tengu_worktree_created` (`{ tmux_enabled }`).
- `tengu_continue` / `tengu_session_resumed` / `tengu_remote_create_session*` / `tengu_teleport_*` / `tengu_deep_link_opened` (resumption variants).

### 10.5 Beta tracing

`isBetaTracingEnabled()` (`init.ts:48`) gates an eager telemetry init for SDK/headless mode (`init.ts:252-258`) so the tracer is ready before the first query. `doInitializeTelemetry` is guarded against double-init via `telemetryInitialized` (`init.ts:55-302`).

### 10.6 1P event logger

`initialize1PEventLogging()` is dynamically imported (`init.ts:94-98`) to defer ~400KB of OpenTelemetry sdk-logs. Subscribed to `onGrowthBookRefresh`: when the `tengu_1p_event_batch_config` config changes, `reinitialize1PEventLoggingIfConfigChanged()` rebuilds the logger provider (`init.ts:99-105`).

---

## 11. Reimplementation Checklist

A reimplementer of this subsystem must preserve:

- [ ] The three pre-module-eval side-effect calls (`profileCheckpoint('main_tsx_entry')`, `startMdmRawRead()`, `startKeychainPrefetch()`) running BEFORE any heavy import. The custom-rule disable comments are part of the contract.
- [ ] The `init()` `memoize` so it runs at most once per process; idempotency for repeated `program.parseAsync` retries.
- [ ] Order inside `init()`: configs → safe env → CA certs → graceful shutdown → 1P logging (deferred) → OAuth → JetBrains → repo detection → remote-managed-settings/policy-limits promise init → record first start → mTLS → proxy agents → API preconnect → CCR upstreamproxy (env-gated) → Windows shell → cleanup-registry registrations → scratchpad (gated).
- [ ] The early SIGINT cooperation between `main.tsx` and `print.ts` for `-p`.
- [ ] `process.env.NoDefaultCurrentDirectoryInExePath = '1'` set BEFORE any subprocess can launch (Windows PATH-hijack defense).
- [ ] `isBeingDebugged()` exit-on-inspector for non-ANT builds at top-level.
- [ ] Argv-rewrite ordering: DIRECT_CONNECT (cc://) → LODESTONE (--handle-uri / __CFBundleIdentifier) → KAIROS (assistant) → SSH_REMOTE (ssh) — and the `_pendingX` module-scoped slots gated by `feature(...)` for DCE.
- [ ] `eagerLoadSettings()` BEFORE `init()`; content-hash temp file naming for `--settings <json>` to preserve API prompt cache.
- [ ] Default-command flag table (§6.5) — every flag, every hidden marker, every `argParser` validation, every `choices(...)` gate, in source order.
- [ ] Subcommand registration order (§6.6); `mcp` first, ANT-only commands last.
- [ ] **Print-mode subcommand short-circuit** (`main.tsx:3875-3892`): when `-p`/`--print` is in argv (and no `cc://`), skip ALL subcommand registration before parsing — saves ~65ms.
- [ ] `setup()` ordering: `setCwd` → `captureHooksConfigSnapshot` → `initializeFileChangedWatcher` → optional worktree branch → bundled skills/plugins (in `main.tsx`, NOT `setup`) → `initSessionMemory` → optional `initContextCollapse` → `lockCurrentVersion` → plugin prefetch → ANT-only auto-undercover prime → `feature('COMMIT_ATTRIBUTION')` `setImmediate` deferral → session-file-access hooks → optional team-memory watcher → `initSinks()` → `tengu_started` → `prefetchApiKeyFromApiKeyHelperIfSafe` → release-notes/recent-activity (gated) → dangerous-permissions enforcement → `tengu_exit` for previous session.
- [ ] The `tengu_started` beacon AS THE FIRST EVENT after sinks attach (denominator semantic).
- [ ] The `--dangerously-skip-permissions` two-tier hard-gate (sudo on Unix; ANT-only Docker/Bubblewrap/IS_SANDBOX + no-internet) with `local-agent`/`claude-desktop` exemption.
- [ ] Worktree branch chdir + `clearMemoryFileCaches` + `updateHooksConfigSnapshot` after `originalCwd` change.
- [ ] `bootstrap/state.ts` as the module-scope singleton holder; the `STATE` object as `const`, mutated only through the published getters/setters; `resetStateForTests()` guarded by `NODE_ENV === 'test'`.
- [ ] All initial state values (allowedSettingSources, sessionId via `randomUUID()`, NFC-normalized cwd via `realpathSync`, ANT-only `replBridgeActive` field).
- [ ] Module-scope sub-state holders: turn-output-tokens trio, scroll drain debounce, `EMPTY_SLOW_OPERATIONS` stable empty reference, `MAX_RECEIVED_UUIDS` ring buffer, `MAX_RESOLVED_TOOL_USE_IDS`.
- [ ] `markScrollActivity` / `getIsScrollDraining` / `waitForScrollIdle` debounce (150ms) AND that background intervals consult it.
- [ ] Beta-header sticky-on latches: once flipped, only `clearBetaHeaderLatches()` (called on `/clear` and `/compact`) resets them.
- [ ] `markPostCompaction` / `consumePostCompaction` consumed-once semantics for tagging the next API success event.
- [ ] `MAX_IN_MEMORY_ERRORS = 100` ring buffer for `inMemoryErrorLog`.
- [ ] `addSlowOperation` ANT-only behavior with editor-prompt skip and TTL/length pruning that returns the stable `EMPTY_SLOW_OPERATIONS` reference for React `Object.is` bail.
- [ ] `runMigrations()` ordering with `CURRENT_MIGRATION_VERSION = 11` CAS save, plus the optional flag-gated migration (`resetAutoModeOptInForDefaultOffer`) and ANT-only one (`migrateFennecToOpus`).
- [ ] `stdin` 3000ms peek timeout with the warning text.
- [ ] `loadSettingsFromFlag` content-hash naming and the `12x token cost` rationale.
- [ ] `MCP serve`: list-tools schema conversion + `'object'`-only outputSchema gate; tool description fetched via `await tool.prompt(...)`; empty `MCP_COMMANDS` list except for `review`; identity `claude/tengu` + `MACRO.VERSION`.
- [ ] `cliError` / `cliOk` `: never` returns and `return undefined as never` post-exit pattern.
- [ ] `ndjsonSafeStringify` U+2028/U+2029 escape.
- [ ] All user-facing error messages verbatim (§6.2).
- [ ] All env vars (§6.8) consumed at the documented positions and with the documented truthy semantics (`isEnvTruthy`).
- [ ] All `profileCheckpoint` names (§6.10) emitted at the documented call sites — they form a stable startup-perf contract.

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. The 14 source-gap items here are inherently DEFERRED because they describe artifacts that do not exist in the leak (`main.tsx` is bundled; build macros; published bin) — they cannot be RESOLVED from source alone. The few items that *can* be resolved by cross-referencing other specs are marked RESOLVED below.

- **Source gap: `main.tsx` is bundled.** [DEFERRED — bundled artifact; cannot be resolved from leak] The action handler in `main.tsx:1007-3870` is ~2860 lines of minified, bundled output. We have grep-confirmed control-flow markers (`profileCheckpoint` names, `feature(...)` gates, `logEvent` events, command registrations) but the **precise sequencing between** those markers (e.g., the relative ordering of `getCommands()` vs `await mcpConfigPromise` vs `tools = getTools(...)` inside the handler) is documented at coarser granularity than other files. A reimplementer should be ready to refine this once a non-bundled source becomes available. (Citations preserved for every load-bearing claim that did surface in grep.)
- **Source gap: `bin/claude` shell wrapper / npm bin entry not in leak.** The leaked tree is `src/` only; the actual published bin script that invokes `await main()` is missing. The `MACRO.VERSION` substitution pattern (`entrypoints/mcp.ts:51`) tells us a build-time macro replacement is in play but the macro resolution rules are not in source.
- **Source gap: tooling for `bun:bundle` `feature(...)` resolution.** The DCE behavior is implicit; the substitution rules for `"external" === 'ant'` are inferred from build comments (`biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered` at the top of `main.tsx`).
- **`tengu_init` payload `entrypoint` field is a literal `'claude'`** (`main.tsx:4564`) regardless of the actual `process.env.CLAUDE_CODE_ENTRYPOINT` (`'cli'`/`'sdk-cli'`/etc.). This appears to be intentional — the analytics field is the entry-point binary name, not the runtime entrypoint identity. Confirmation requires a contract document not in source.
- **`assistantActivationPath` source** (referenced in `tengu_init` at `:4602` and as an action-handler input) — populated from `assistantModule?.getAssistantActivationPath()` (`main.tsx:2518`). The exact semantic ("which path through the KAIROS gate did this session take") is documented in spec 32 (KAIROS); not duplicated here.
- **`MAX_IN_MEMORY_ERRORS` shift semantics** (`state.ts:1219-1224`): the current implementation shifts the oldest error when the buffer is at capacity, BEFORE pushing the new one. The order is observable via `getInMemoryErrors` (in `utils/log.js`, owned by 26). Adversarial: a flood of errors at exactly the capacity threshold could mask a genuine subsequent error. No mitigation in source.
- **`telemetryInitialized` vs `setMeter` race** (`init.ts:288-302` + `state.ts:948-987`): `doInitializeTelemetry` flips the flag BEFORE awaiting `setMeterState`, then resets it on failure. If two callers fire `initializeTelemetryAfterTrust` within the same tick, only the first proceeds; subsequent calls early-return without retry. Acceptable for the current call sites (there is exactly one trust acceptance per session) but worth noting.
- **`--bare` and `feature('TEAMMEM')`/auto-undercover compatibility**: `setup.ts:336-369` wraps both inside `if (!isBareMode())`, but the `feature('COMMIT_ATTRIBUTION')` `setImmediate` deferral is also inside this block. A user who sets `CLAUDE_CODE_SIMPLE=1` directly without passing `--bare` will follow the bare-skip path; verify this is intentional (the env var IS the canonical signal — `--bare` just sets it).
- **`prefetchSystemContextIfSafe` non-interactive trust bypass**: under `-p`, `getSystemContext()` runs unconditionally even if the user has never accepted the trust dialog. The help text for `-p` documents this ("The workspace trust dialog is skipped..."). The git status it spawns can run `core.fsmonitor` / `diff.external` hooks → arbitrary code. This is a known design; documented in source at `main.tsx:363-368`. Reimplementer must preserve this trust-by-`-p` contract.
- **`tengu_session_resumed` 9 call sites**: each call site has slightly different payload shape based on the resume path (continue, resume by id, resume picker, fork, from-pr, teleport, etc.). Distinguishing which call site fires for which user input requires reading the bundled action handler at greater depth than was practical here.
- **`PROACTIVE` vs `KAIROS` overlap**: `main.tsx:2197` and `:4612` both treat `feature('PROACTIVE') || feature('KAIROS')` symmetrically for proactive activation. Whether both flags can be on simultaneously and whether activation is idempotent across re-entry depends on `proactiveModule.isProactiveActive()` (spec 31).
- ~~**`isInProtectedNamespace()` in `tengu_init`**~~ — **RESOLVED Phase 9.7**: defined at `src/utils/envUtils.ts:136`. Returns whether the current process is running inside an Anthropic-protected namespace. Owned by spec 27 (policy) consumer-side; the implementation is a small env predicate in `utils/envUtils.ts`.

- ~~**`MAX_RECEIVED_UUIDS = 10_000` ring buffer overflow**~~ — **RESOLVED Phase 9.7**: lives at `src/cli/print.ts:394-415`. Eviction is **FIFO** (`receivedMessageUuidsOrder.splice(0, overflow)`). Verified by direct read of `cli/print.ts:395-413`. Spec 04 / 35 (CLI print) owns this surface; spec 04 should reflect FIFO eviction.
- **Argv-rewrite unsafe interaction with quoted strings**: the rewrites (e.g., `rawCliArgs.findIndex(a => a.startsWith('--permission-mode='))`) operate on raw `process.argv` post-shell-tokenization, so quoting is already resolved. But `claude --debug "ssh user@host"` (quoted) would not match the position-0 `ssh` predicate — falls through to the stub. Documented at `main.tsx:679-684` comment.
- (resolved above — see RESOLVED entry for `MAX_RECEIVED_UUIDS` ring buffer.)
- **`MACRO.VERSION` literal value** (`entrypoints/mcp.ts:51`): bundler-replaced; not present in source. The MCP server identifies as `claude/tengu` + this string, which downstream MCP clients may key on.
