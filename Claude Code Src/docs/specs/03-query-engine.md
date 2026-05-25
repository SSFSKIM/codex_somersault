# 03 — Query Engine Specification

> Owner: `sub-B1`. Source of truth for `src/QueryEngine.ts` (1295 lines). The engine is the algorithmic heart of the harness: it owns one conversation's API streaming lifecycle, accumulates usage, drives the turn loop via `query()`, normalises messages for the SDK, and emits the terminal `result` payload.
>
> Adjacent specs (refer rather than redocument): 04 (turn pipeline / `query.ts`), 05 (context assembly), 06 (cost / token aggregation), 07 (compaction trigger logic), 08 (Tool/`ToolUseContext`), 09 (permission), 22 (Anthropic client + retry/streaming), 26 (GrowthBook flag resolution), 29 (memory), 30 (multi-agent).

---

## 1. Purpose & Scope

### 1.1 Problem

`QueryEngine` is the per-conversation, multi-turn driver that:

1. Constructs the system prompt by composing `defaultSystemPrompt` (from `fetchSystemPromptParts`) with optional `customSystemPrompt`, the memory-mechanics prompt (when both `customSystemPrompt` is set AND `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` is set), and `appendSystemPrompt` — `src/QueryEngine.ts:284-325`.
2. Threads through the underlying turn pipeline (`query()` from `src/query.ts`, owned by spec 04) which performs the Anthropic streaming SSE call. QueryEngine is the **consumer** of stream events; the streaming/retry mechanics live in `services/api/claude.ts` (spec 22).
3. Accumulates per-message usage (`message_start` → `message_delta` → `message_stop`) into the running `totalUsage` (`QueryEngine.ts:788-816`).
4. Drives the SDK message protocol — emitting `system/init`, `system/api_retry`, `system/compact_boundary`, `tool_use_summary`, `assistant`, `user`, `stream_event`, and finally a terminal `result` (subtype: `success`, `error_max_turns`, `error_max_budget_usd`, `error_max_structured_output_retries`, `error_during_execution`).
5. Enforces (in QueryEngine) `maxBudgetUsd` (`:972-1002`) and `MAX_STRUCTURED_OUTPUT_RETRIES` (`:1004-1048`); the `maxTurns` count is enforced **upstream** by `query.ts` (spec 04), which emits a `max_turns_reached` attachment that QueryEngine then translates into the terminal `error_max_turns` SDK result (`:841-873`). QueryEngine forwards `maxTurns` and `taskBudget` into `query()` (`:683-685`).
6. Persists the transcript at strategic points and flushes when the host (Claude Desktop, cowork) is known to kill the process immediately after the result.

### 1.2 IN scope

- `src/QueryEngine.ts` (1295 lines) — full file, including the `QueryEngine` class, the `ask()` convenience wrapper, and feature-flag wiring.
- The contract between `QueryEngine` and downstream consumers: the SDK message envelope (`SDKMessage`).
- Per-message usage update semantics (`updateUsage` / `accumulateUsage` from `services/api/claude.ts:2924-3038`) — documented here because the watermark and reset rules are encoded in `QueryEngine.ts:788-816`.
- Thinking config gates that QueryEngine seeds: `shouldEnableThinkingByDefault()` and the `{ type: 'adaptive' }` default (`QueryEngine.ts:278-282`). Selection of `adaptive` vs `enabled { budget_tokens }` happens in `services/api/claude.ts:1596-1630` (spec 22).
- Feature-flag deltas wired into QueryEngine: `COORDINATOR_MODE` (line 115), `HISTORY_SNIP` (lines 122-127, 1276-1284), and the call sites it pokes (`feature('CONNECTOR_TEXT')`, `feature('CACHED_MICROCOMPACT')`, `feature('PROMPT_CACHE_BREAK_DETECTION')`, `feature('TRANSCRIPT_CLASSIFIER')` — ALL invoked from `services/api/claude.ts`, owned by spec 22 but documented here for traceability).

### 1.3 OUT of scope (cite, do not redocument)

| Concern | Owner |
|---|---|
| Tool dispatch / tool result lifecycle / system-reminder injection | 04 |
| Tool base interface (`Tool`, `ToolUseContext`) | 08 |
| Permission decision tree | 09 |
| `Anthropic` client construction, beta header set, prompt-cache breakpoint placement, retry algorithm, streaming SSE protocol details, `withRetry` outer loop | 22 |
| `cost-tracker` accumulation (`getTotalCost`, `getModelUsage`, `getTotalAPIDuration`) | 06 |
| Compaction trigger / microcompact / snip projection algorithm | 07 |
| `fetchSystemPromptParts` internals (CLAUDE.md walk, system context, currentDate, cache-breaker) | 05 |
| GrowthBook flag resolution (`getFeatureValue_CACHED_MAY_BE_STALE`) | 26 |
| Multi-agent fan-out (coordinator init lives in `setup.ts`; QueryEngine merely reads `getCoordinatorUserContext`) | 30 |

---

## 2. Source Map

### 2.1 Owned files

| Path | Lines | Coverage |
|---|---|---|
| `src/QueryEngine.ts` | 1295 | fully-read |

### 2.2 Imports from (upstream)

| Symbol(s) | Module | Site | Notes |
|---|---|---|---|
| `feature` | `bun:bundle` | `QueryEngine.ts:1` | Build-time DCE. |
| `ContentBlockParam` (type) | `@anthropic-ai/sdk/resources/messages.mjs` | `:2` | Prompt input type. |
| `randomUUID` | `crypto` | `:3` | Synthetic `uuid` for terminal `result` and `stream_event` SDK envelopes. |
| `last` | `lodash-es/last.js` | `:4` | Last assistant content extraction. |
| `getSessionId`, `isSessionPersistenceDisabled` | `src/bootstrap/state.js` | `:5-8` | Session identity + transcript gate. |
| `PermissionMode`, `SDKCompactBoundaryMessage`, `SDKMessage`, `SDKPermissionDenial`, `SDKStatus`, `SDKUserMessageReplay` (types) | `src/entrypoints/agentSdkTypes.js` | `:9-16` | SDK-facing envelope types. |
| `accumulateUsage`, `updateUsage` | `src/services/api/claude.js` | `:17` | Usage merge primitives — see §6.4. |
| `NonNullableUsage` (type), `EMPTY_USAGE` | `src/services/api/logging.js` | `:18-19` | Zero-init usage; aliased re-export. Actual definition `src/services/api/emptyUsage.ts:8-22`. |
| `stripAnsi` | `strip-ansi` | `:20` | Strips ANSI from local-command stdout/stderr in fast-path. |
| `Command` (type), `getSlashCommandToolSkills` | `./commands.js` | `:21-22` | Slash command list + skills index. |
| `LOCAL_COMMAND_STDOUT_TAG`, `LOCAL_COMMAND_STDERR_TAG` | `./constants/xml.js` | `:23-26` | XML wrapper tags used to detect local-command outputs. |
| `getModelUsage`, `getTotalAPIDuration`, `getTotalCost` | `./cost-tracker.js` | `:27-31` | Spec 06 — read-only accessors, accumulators owned there. |
| `CanUseToolFn` (type) | `./hooks/useCanUseTool.js` | `:32` | Permission entry point. Spec 09. |
| `loadMemoryPrompt` | `./memdir/memdir.js` | `:33` | Memory-mechanics prompt body. Spec 29 / 40. |
| `hasAutoMemPathOverride` | `./memdir/paths.js` | `:34` | Reads `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE`. Spec 29. |
| `query` | `./query.js` | `:35` | The turn pipeline generator (spec 04). |
| `categorizeRetryableAPIError` | `./services/api/errors.js` | `:36` | Maps APIError → `'rate_limit' \| 'authentication_failed' \| 'server_error' \| 'unknown'`. See §6.5. |
| `MCPServerConnection` (type) | `./services/mcp/types.js` | `:37` | Spec 23. |
| `AppState` (type) | `./state/AppState.js` | `:38` | Spec 41. |
| `Tools` (type), `ToolUseContext` (type), `toolMatchesName` | `./Tool.js` | `:39` | Spec 08. |
| `AgentDefinition` (type) | `./tools/AgentTool/loadAgentsDir.js` | `:40` | Spec 14. |
| `SYNTHETIC_OUTPUT_TOOL_NAME` | `./tools/SyntheticOutputTool/SyntheticOutputTool.js` | `:41` | Used to count structured-output retries. |
| `Message` (type) | `./types/message.js` | `:42` | Internal message type. **Source file is absent from the leaked tree** (`src/types/` contains no `message.ts` or `message.tsx`); see §12.3 — likely in `src/types/generated/`, to be located by spec 04/08. |
| `OrphanedPermission` (type) | `./types/textInputTypes.js` | `:43` | Spec 09. |
| `createAbortController` | `./utils/abortController.js` | `:44` | Default `AbortController` factory if caller did not pass one. |
| `AttributionState` (type) | `./utils/commitAttribution.js` | `:45` | Spec 10. |
| `getGlobalConfig` | `./utils/config.js` | `:46` | Theme resolution. |
| `getCwd` | `./utils/cwd.js` | `:47` | Used for `getSlashCommandToolSkills`. |
| `isBareMode`, `isEnvTruthy` | `./utils/envUtils.js` | `:48` | Bare-mode and env truthiness helpers. |
| `getFastModeState` | `./utils/fastMode.js` | `:49` | Fast-mode snapshot for terminal `result`. |
| `FileHistoryState` (type), `fileHistoryEnabled`, `fileHistoryMakeSnapshot` | `./utils/fileHistory.js` | `:50-54` | Spec 41. |
| `cloneFileStateCache`, `FileStateCache` (type) | `./utils/fileStateCache.js` | `:55-58` | Spec 11. Per-conversation read-file cache; `ask()` clones the caller-provided cache so updates don't bleed across conversations (`:1259`). |
| `headlessProfilerCheckpoint` | `./utils/headlessProfiler.js` | `:59` | Boot/turn profiling — cites: `before_getSystemPrompt`, `after_getSystemPrompt`, `before_skills_plugins`, `after_skills_plugins`, `system_message_yielded`. |
| `registerStructuredOutputEnforcement` | `./utils/hooks/hookHelpers.js` | `:60` | When `jsonSchema` is set AND `SyntheticOutputTool` exists, registers a function hook that forces the model to call the tool. |
| `getInMemoryErrors` | `./utils/log.js` | `:61` | Source for `error_during_execution.errors[]`. |
| `countToolCalls`, `SYNTHETIC_MESSAGES` | `./utils/messages.js` | `:62` | Synthetic message detection + retry counter. |
| `getMainLoopModel`, `parseUserSpecifiedModel` | `./utils/model/model.js` | `:63-66` | Initial model resolution. |
| `loadAllPluginsCacheOnly` | `./utils/plugins/pluginLoader.js` | `:67` | Plugin manifest read (CACHE-ONLY — see §7.2). |
| `ProcessUserInputContext` (type), `processUserInput` | `./utils/processUserInput/processUserInput.js` | `:68-71` | Slash-command processing + message construction. |
| `fetchSystemPromptParts` | `./utils/queryContext.js` | `:72` | Spec 05. |
| `setCwd` | `./utils/Shell.js` | `:73` | Mirrors config.cwd into shell environment. |
| `flushSessionStorage`, `recordTranscript` | `./utils/sessionStorage.js` | `:74-77` | Transcript persistence. |
| `asSystemPrompt` | `./utils/systemPromptType.js` | `:78` | Brand newtype around `string[]`. |
| `resolveThemeSetting` | `./utils/systemTheme.js` | `:79` | Theme resolution. |
| `shouldEnableThinkingByDefault`, `ThinkingConfig` (type) | `./utils/thinking.js` | `:80-83` | Initial thinking config gate. |
| `messageSelector` (lazy) | `./components/MessageSelector.js` (via factory at `:87-89`) | `:87` | Lazy because Ink/React; only loaded on demand. |
| `localCommandOutputToSDKAssistantMessage`, `toSDKCompactMetadata` | `./utils/messages/mappers.js` | `:91-94` | SDK envelope mappers. |
| `buildSystemInitMessage`, `sdkCompatToolName` | `./utils/messages/systemInit.js` | `:95-98` | Builds the leading `system/init` SDK message. |
| `getScratchpadDir`, `isScratchpadEnabled` | `./utils/permissions/filesystem.js` | `:99-102` | Coordinator-mode scratchpad path. |
| `handleOrphanedPermission`, `isResultSuccessful`, `normalizeMessage` | `./utils/queryHelpers.js` | `:104-108` | Per-message normalisation; `isResultSuccessful` is a type predicate (see §9). |
| `getCoordinatorUserContext` | `./coordinator/coordinatorMode.js` (gated `feature('COORDINATOR_MODE')`) | `:111-118` | Returns extra `userContext` keys when coordinator mode is active. |
| `snipModule` | `./services/compact/snipCompact.js` (gated `feature('HISTORY_SNIP')`) | `:122-124` | Snip-replay implementation. |
| `snipProjection` | `./services/compact/snipProjection.js` (gated `feature('HISTORY_SNIP')`) | `:125-127` | Boundary detector — `isSnipBoundaryMessage`. |

### 2.3 Imported by (downstream)

| Path | Citation |
|---|---|
| `src/entrypoints/sdk/sdk.ts` (and SDK glue) | `ask` re-exported as the SDK entry; verified by registry conventions (Phase 0). |
| `src/cli/print.ts` and headless `-p` mode | Consumes `SDKMessage` stream from `ask()`. (Indirect — see spec 01.) |
| Any caller using SDK headless query | `QueryEngine` and `ask` are the only public entry points of this module. |

(See spec 01 for the precise call site of `ask()` in headless / SDK / cowork dispatch.)

### 2.4 Feature-flag and ANT-guard sites in this file

| Site | Citation | Effect |
|---|---|---|
| `feature('COORDINATOR_MODE')` | `:115` | Conditional import of `getCoordinatorUserContext`; falsy build → `() => ({})`. |
| `feature('HISTORY_SNIP')` (snip module) | `:122-124` | Conditional import of `snipCompact` module. |
| `feature('HISTORY_SNIP')` (snip projection) | `:125-127` | Conditional import of `snipProjection`. |
| `feature('HISTORY_SNIP')` (`ask()` snipReplay injection) | `:1276-1284` | Caller-side wiring of `snipReplay` callback into engine config. |

QueryEngine itself does NOT contain `process.env.USER_TYPE === 'ant'` reads; ANT branches it depends on are downstream (in `claude.ts`, `setup.ts`, `tools.ts`, etc.).

### 2.5 Missing / absent source

QueryEngine itself (`src/QueryEngine.ts`) is fully present and read. However, two modules it imports via `feature('HISTORY_SNIP')`-gated `require()` are **absent from the leaked tree**:

- `src/services/compact/snipCompact.ts` — cited at `QueryEngine.ts:122-124` as `snipModule`. `find src/services/compact -name 'snipCompact*'` returns nothing; the directory ships `apiMicrocompact.ts, autoCompact.ts, compact.ts, compactWarningHook.ts, compactWarningState.ts, grouping.ts, microCompact.ts, postCompactCleanup.ts, prompt.ts, sessionMemoryCompact.ts, timeBasedMCConfig.ts` — no `snipCompact.ts`. Behavior described in this spec (`snipCompactIfNeeded(store, { force: true })`) is inferred from the `ask()` injection site at `:1276-1284` and the engine's `system` case at `:898-915`; the module body was not read.
- `src/services/compact/snipProjection.ts` — cited at `QueryEngine.ts:125-127` as `snipProjection`, exposing `isSnipBoundaryMessage`. Same `find` confirms absence. Spec 07 owns the canonical definition; this spec defers (see also §12.10).

Both modules are gated by `feature('HISTORY_SNIP')`, which is OFF in the public build, so dead-code elimination strips the `require()`s and the absence does not block the leaked-tree build. The lazy `require('src/components/MessageSelector.js')` (`:87-89`) is real; `src/components/MessageSelector.tsx` exists in the leaked tree.

Indirect dependencies on other absent sources (e.g. `WorkflowTool`, `MonitorTool`) are gated outside QueryEngine and tracked in 00 §2.5.

### 2.6 Source-coverage inventory

| File | Status |
|---|---|
| `src/QueryEngine.ts` (1295 lines) | fully-read |
| `src/services/api/claude.ts` (3419 lines) — `updateUsage`, `accumulateUsage`, SSE event switch, thinking gating | sampled (§§2920-3038 fully-read; §§1955-2400 fully-read; §§1580-1730 fully-read) |
| `src/services/api/withRetry.ts` (822 lines) | fully-read |
| `src/services/api/emptyUsage.ts` (22 lines) | fully-read |
| `src/services/api/errors.ts` (1207 lines) | sampled (`categorizeRetryableAPIError` §§1163-1182 fully-read) |
| `src/utils/thinking.ts` (162 lines) | fully-read |
| `src/utils/context.ts` (`getMaxThinkingTokensForModel` §§212-221) | sampled |
| `src/utils/queryHelpers.ts` | partially-read (`:56-68` `isResultSuccessful`, `:102-219` `normalizeMessage` including `tool_progress` gating at `:157-201`) |
| `src/utils/queryContext.ts` | grep-inspected |
| `src/utils/messages/systemInit.ts` | grep-inspected |

---

## 3. Public Interface

### 3.1 `QueryEngineConfig` (`QueryEngine.ts:130-173`) — verbatim

```ts
export type QueryEngineConfig = {
  cwd: string
  tools: Tools
  commands: Command[]
  mcpClients: MCPServerConnection[]
  agents: AgentDefinition[]
  canUseTool: CanUseToolFn
  getAppState: () => AppState
  setAppState: (f: (prev: AppState) => AppState) => void
  initialMessages?: Message[]
  readFileCache: FileStateCache
  customSystemPrompt?: string
  appendSystemPrompt?: string
  userSpecifiedModel?: string
  fallbackModel?: string
  thinkingConfig?: ThinkingConfig
  maxTurns?: number
  maxBudgetUsd?: number
  taskBudget?: { total: number }
  jsonSchema?: Record<string, unknown>
  verbose?: boolean
  replayUserMessages?: boolean
  /** Handler for URL elicitations triggered by MCP tool -32042 errors. */
  handleElicitation?: ToolUseContext['handleElicitation']
  includePartialMessages?: boolean
  setSDKStatus?: (status: SDKStatus) => void
  abortController?: AbortController
  orphanedPermission?: OrphanedPermission
  /**
   * Snip-boundary handler: receives each yielded system message plus the
   * current mutableMessages store. Returns undefined if the message is not a
   * snip boundary; otherwise returns the replayed snip result. Injected by
   * ask() when HISTORY_SNIP is enabled so feature-gated strings stay inside
   * the gated module (keeps QueryEngine free of excluded strings and testable
   * despite feature() returning false under bun test). SDK-only: the REPL
   * keeps full history for UI scrollback and projects on demand via
   * projectSnippedView; QueryEngine truncates here to bound memory in long
   * headless sessions (no UI to preserve).
   */
  snipReplay?: (
    yieldedSystemMsg: Message,
    store: Message[],
  ) => { messages: Message[]; executed: boolean } | undefined
}
```

### 3.2 `class QueryEngine` (`QueryEngine.ts:184-1177`)

Constructor: `new QueryEngine(config: QueryEngineConfig)`. Internals are private (private fields `config`, `mutableMessages`, `abortController`, `permissionDenials`, `totalUsage`, `hasHandledOrphanedPermission`, `readFileState`, `discoveredSkillNames`, `loadedNestedMemoryPaths`).

Methods:

| Method | Signature | Purpose |
|---|---|---|
| `submitMessage` | `async *submitMessage(prompt: string \| ContentBlockParam[], options?: { uuid?: string; isMeta?: boolean }): AsyncGenerator<SDKMessage, void, unknown>` (`:209-1156`) | One turn within the conversation. Each call yields the SDK message stream for that turn. Persists state across calls. |
| `interrupt` | `(): void` (`:1158-1160`) | Aborts the engine's `AbortController`. Idempotent if not started. |
| `getMessages` | `(): readonly Message[]` (`:1162-1164`) | Read-only view of `mutableMessages`. |
| `getReadFileState` | `(): FileStateCache` (`:1166-1168`) | Read-only view of the `readFileState`. |
| `getSessionId` | `(): string` (`:1170-1172`) | Delegates to `getSessionId()` from `bootstrap/state.js`. |
| `setModel` | `(model: string): void` (`:1174-1176`) | Mutates `config.userSpecifiedModel` between turns. |

### 3.3 `ask()` convenience wrapper (`QueryEngine.ts:1186-1295`)

`async function* ask(args): AsyncGenerator<SDKMessage, void, unknown>`

Constructs a single-call `QueryEngine`, threads `mutableMessages` and `readFileCache` callbacks (via `getReadFileCache`/`setReadFileCache`), runs one `submitMessage`, and `setReadFileCache(engine.getReadFileState())` in a `try/finally`. Wires `snipReplay` only under `feature('HISTORY_SNIP')`.

Argument list (verbatim, `:1186-1248`):

```
commands, prompt, promptUuid, isMeta, cwd, tools, mcpClients,
verbose=false, thinkingConfig, maxTurns, maxBudgetUsd, taskBudget,
canUseTool, mutableMessages=[], getReadFileCache, setReadFileCache,
customSystemPrompt, appendSystemPrompt, userSpecifiedModel,
fallbackModel, jsonSchema, getAppState, setAppState, abortController,
replayUserMessages=false, includePartialMessages=false,
handleElicitation, agents=[], setSDKStatus, orphanedPermission
```

### 3.4 SDK message envelope contract (output)

QueryEngine emits these SDK message shapes (envelope from `entrypoints/agentSdkTypes.ts`, types owned by spec 01 / SDK):

- `system / init` — `buildSystemInitMessage({ tools, mcpClients, model, permissionMode, commands, agents, skills, plugins, fastMode })` (`:540-551`).
- `user` (with `isReplay: true`) — initial user-message replay if `replayUserMessages` (`:737-749`); local-command stdout/stderr replay (`:567-580`); and `queued_command` attachment replay (`:879-891`).
- Synthetic `assistant` for local-command output (`:594`, via `localCommandOutputToSDKAssistantMessage`).
- `system / compact_boundary` (`:597-605`, `:935-941`).
- `system / api_retry` (`:943-955`) — emitted on `api_error` system messages from `query.ts`, carrying `attempt`, `max_retries`, `retry_delay_ms`, `error_status`, and the `error` enum from `categorizeRetryableAPIError`.
- `tool_use_summary` (`:961-967`).
- `tool_progress` — emitted indirectly through `normalizeMessage(progress)` (`utils/queryHelpers.ts:102` declaration; `bash_progress`/`powershell_progress` branch at `:157-201`; `type: 'tool_progress'` yield at `:189-200`). Surfaces bash/PowerShell streaming progress to the SDK consumer. Gating predicates verified in source at `:163-168`: emission requires either `isEnvTruthy(process.env.CLAUDE_CODE_REMOTE)` OR `process.env.CLAUDE_CODE_CONTAINER_ID` to be set, AND at least `TOOL_PROGRESS_THROTTLE_MS` (30s, `:177`) elapsed since the last emission for the same `parentToolUseID` tracking key (LRU eviction at `:178-186`). QueryEngine itself only `yield* normalizeMessage(message)`s the `progress` record; the `tool_progress` envelope shape and gating logic are owned by `queryHelpers`.
- `stream_event` (`:818-826`) — gated by `includePartialMessages`. Wraps each underlying SSE part; UUID is freshly minted per emission.
- `result` — terminal SDK message. Subtypes:
  - `success` (no-query path: `:618-637`; query path: `:1135-1155`).
  - `error_max_turns` (`:851-872`).
  - `error_max_budget_usd` (`:981-1001`).
  - `error_max_structured_output_retries` (`:1024-1046`).
  - `error_during_execution` (`:1083-1116`).

**Normalization scope.** `normalizeMessage` is invoked **only for** `assistant` (`:761-770`), `progress` (`:771-783`), and `user` (`:784-787`) records (see `utils/queryHelpers.ts:102-221`). Other internal record types are **not** pass-through SDK outputs:
- `attachment` (`:829-893`) is **side-effect only** — its three observed types each trigger a different action: `structured_output` populates `structuredOutputFromTool`; `max_turns_reached` yields the terminal `result(error_max_turns)`; `queued_command` (only when `replayUserMessages`) yields a synthetic `SDKUserMessageReplay`. The raw `attachment` record itself is **not** emitted to the SDK.
- `system` (`:897-958`) has bespoke handling: snip-replay short-circuit, `compact_boundary` → `SDKCompactBoundaryMessage`, `api_error` → `system / api_retry`. Other subtypes are dropped.
- `tombstone` and `stream_request_start` are dropped without emit.
- Pass-through `assistant`, `user`, `progress`, `attachment` (other types) — produced by `normalizeMessage(message)` (`:769`, `:782`, `:786`).

---

## 4. Data Model & State

### 4.1 Per-engine private state

| Field | Type | Lifetime | Purpose |
|---|---|---|---|
| `config` | `QueryEngineConfig` | conversation | Frozen (other than `setModel`). |
| `mutableMessages` | `Message[]` | conversation | Authoritative message list. Mutated on each turn (push of user input, assistant blocks, attachments, progress, system, tombstone-skipped). Reset to `config.initialMessages ?? []` at construction (`:202`). |
| `abortController` | `AbortController` | conversation | Single abort signal for all turns. `interrupt()` aborts it. |
| `permissionDenials` | `SDKPermissionDenial[]` | conversation | Pushed every time `wrappedCanUseTool` returns non-`allow` (`:262-268`). Reported on every terminal `result`. |
| `totalUsage` | `NonNullableUsage` | conversation | Accumulates each message's final usage (snapshot at `message_stop`) via `accumulateUsage` (`:812-816`). |
| `hasHandledOrphanedPermission` | `boolean` | conversation, write-once | Latches `true` after the first `submitMessage` consumes `orphanedPermission` (`:399`). |
| `readFileState` | `FileStateCache` | conversation | Tracks per-file content snapshots used by file tools. Mirrored to caller via `getReadFileState()` (and back via `ask()`'s `setReadFileCache`). |
| `discoveredSkillNames` | `Set<string>` | turn (cleared at each `submitMessage`, `:238`) | Used by the skill-tool telemetry path (`tengu_skill_tool_invocation.was_discovered`). Persists across the two `ProcessUserInputContext` rebuilds inside one turn (see §5.4). |
| `loadedNestedMemoryPaths` | `Set<string>` | conversation | Tracks which nested-memory paths have already been merged into context. Not cleared per turn. |

### 4.2 Per-turn local state (within `submitMessage`)

| Variable | Citation | Purpose |
|---|---|---|
| `startTime` | `:241` | `Date.now()` baseline for `duration_ms`. |
| `persistSession` | `:240` | `!isSessionPersistenceDisabled()`. |
| `wrappedCanUseTool` | `:244-271` | Wraps `config.canUseTool` to record `permissionDenials`. |
| `initialAppState` | `:273` | One-shot snapshot of `getAppState()` at turn start; used for permission mode + fastMode. |
| `initialMainLoopModel` | `:274-276` | `parseUserSpecifiedModel(config.userSpecifiedModel)` if set, else `getMainLoopModel()`. |
| `initialThinkingConfig` | `:278-282` | `config.thinkingConfig` if set; else `{ type: 'adaptive' }` if `shouldEnableThinkingByDefault() !== false`; else `{ type: 'disabled' }`. |
| `customPrompt` | `:286-287` | Narrowed `customSystemPrompt`. |
| `defaultSystemPrompt`, `baseUserContext`, `systemContext` | `:288-300` | From `fetchSystemPromptParts`. |
| `userContext` | `:302-308` | Spread merge of `baseUserContext` + `getCoordinatorUserContext(...)` (latter only non-empty under COORDINATOR_MODE + scratchpad). |
| `memoryMechanicsPrompt` | `:316-319` | Loaded via `loadMemoryPrompt()` iff `customPrompt !== undefined` AND `hasAutoMemPathOverride()`. |
| `systemPrompt` | `:321-325` | Concatenation order (verbatim — see §6.1). |
| `processUserInputContext` (1st build) | `:335-395` | Full slash-command-aware `ProcessUserInputContext`. |
| `messagesFromUserInput`, `shouldQuery`, `allowedTools`, `modelFromUserInput`, `resultText` | `:411-428` | Output of `processUserInput({ ..., querySource: 'sdk' })`. |
| `messages` | `:434` | Snapshot copy: `[...this.mutableMessages]`. |
| `replayableMessages`, `messagesToAck` | `:466-474` | Filtered subset re-emitted as `isReplay: true` if `replayUserMessages`. |
| `mainLoopModel` | `:488` | `modelFromUserInput ?? initialMainLoopModel`. |
| `processUserInputContext` (2nd build) | `:492-527` | Rebuilt with no-op `setMessages` (slash-command processing already done) and updated `mainLoopModel`. |
| `skills`, `enabledPlugins` | `:534-537` | Parallel `Promise.all([getSlashCommandToolSkills(getCwd()), loadAllPluginsCacheOnly()])`. **Cache-only**: must not hit network. |
| `currentMessageUsage` | `:658` | Reset to `EMPTY_USAGE` on every `message_start`. |
| `turnCount` | `:659` | Initialised to 1; incremented per `user` message (i.e. tool-result batch). |
| `hasAcknowledgedInitialMessages` | `:660` | Latches `true` after first transcript record. |
| `structuredOutputFromTool` | `:662` | Captures `attachment.data` from `structured_output` attachments. |
| `lastStopReason` | `:664` | Captured from `assistant.message.stop_reason` (synthetic) or `stream_event.message_delta.delta.stop_reason`. |
| `errorLogWatermark` | `:669` | Reference to last in-memory error at turn start; turn-scopes `error_during_execution.errors[]`. |
| `initialStructuredOutputCalls` | `:671-673` | Count of prior SyntheticOutputTool calls; baseline for retry limit. |

### 4.3 EMPTY_USAGE (`src/services/api/emptyUsage.ts:8-22`) — verbatim

```ts
export const EMPTY_USAGE: Readonly<NonNullableUsage> = {
  input_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_read_input_tokens: 0,
  output_tokens: 0,
  server_tool_use: { web_search_requests: 0, web_fetch_requests: 0 },
  service_tier: 'standard',
  cache_creation: {
    ephemeral_1h_input_tokens: 0,
    ephemeral_5m_input_tokens: 0,
  },
  inference_geo: '',
  iterations: [],
  speed: 'standard',
}
```

### 4.4 Lifecycle summary

```
new QueryEngine(config)
  └─ mutableMessages := initialMessages ?? []
     totalUsage := EMPTY_USAGE
     abortController := config.abortController ?? createAbortController()
     hasHandledOrphanedPermission := false

(per submitMessage(prompt, opts) call):

  setCwd(cwd)
  discoveredSkillNames.clear()
  ── compute initialThinkingConfig, mainLoopModel, customPrompt
  ── fetchSystemPromptParts (5)        [headlessProfilerCheckpoint pair]
  ── if customPrompt && hasAutoMemPathOverride: loadMemoryPrompt
  ── compose systemPrompt (verbatim §6.1)
  ── if jsonSchema && SyntheticOutputTool present:
       registerStructuredOutputEnforcement(...)
  ── build ProcessUserInputContext #1 (with mutating setMessages)
  ── if orphanedPermission && !hasHandledOrphanedPermission:
       hasHandledOrphanedPermission := true
       yield* handleOrphanedPermission(orphanedPermission, ...)
  ── processUserInput({ querySource: 'sdk', ... })
       → messagesFromUserInput, shouldQuery, allowedTools, modelFromUserInput, resultText
  ── mutableMessages.push(...messagesFromUserInput)
  ── messages := [...mutableMessages]
  ── if persistSession && messagesFromUserInput.length > 0:
       (bare-mode: fire-and-forget) recordTranscript(messages)
       else: await; CLAUDE_CODE_EAGER_FLUSH || CLAUDE_CODE_IS_COWORK ⇒ await flushSessionStorage()
  ── update toolPermissionContext.alwaysAllowRules.command := allowedTools
  ── build ProcessUserInputContext #2 (no-op setMessages, updated mainLoopModel)
  ── parallel: skills, plugins (CACHE-ONLY)
  ── yield buildSystemInitMessage(...)
  ── if !shouldQuery:
        replay local-command outputs / compact boundaries
        if persistSession: recordTranscript + maybe flush
        yield result(success)
        return
  ── if fileHistoryEnabled() && persistSession: snapshot per user message
  ── for await (message of query({ messages, systemPrompt, userContext, systemContext,
                                    canUseTool: wrappedCanUseTool, toolUseContext: pUIC2,
                                    fallbackModel, querySource: 'sdk', maxTurns, taskBudget })):
        ── push to messages (and persist) for assistant|user|compact_boundary
        ── normalize per-type (see §5.5)
        ── update totalUsage on stream_event{message_start | message_delta | message_stop}
        ── enforce maxBudgetUsd (yields error_max_budget_usd, returns)
        ── enforce MAX_STRUCTURED_OUTPUT_RETRIES on each user message (yields error_max_structured_output_retries, returns)
  ── flush if EAGER_FLUSH or COWORK
  ── derive `result` (last assistant|user)
  ── if !isResultSuccessful(result, lastStopReason):
        yield result(error_during_execution)
        return
  ── extract textResult (last text content, excluding SYNTHETIC_MESSAGES)
  ── yield result(success, structured_output: structuredOutputFromTool)
```

---

## 5. Algorithm / Control Flow

### 5.1 Turn entrypoint (`submitMessage`)

```pseudocode
fn submitMessage(prompt, opts):
  destructure config; clear discoveredSkillNames; setCwd(cwd)
  persistSession := !isSessionPersistenceDisabled()
  startTime := Date.now()

  wrappedCanUseTool := (tool, input, ctx, asst, id, force) =>
    let r = await canUseTool(tool, input, ctx, asst, id, force)
    if r.behavior !== 'allow':
      permissionDenials.push({ tool_name: sdkCompatToolName(tool.name), tool_use_id: id, tool_input: input })
    return r

  initialAppState := getAppState()
  initialMainLoopModel := userSpecifiedModel ? parseUserSpecifiedModel(userSpecifiedModel) : getMainLoopModel()

  initialThinkingConfig :=
    thinkingConfig ?? (shouldEnableThinkingByDefault() !== false
                        ? { type: 'adaptive' }
                        : { type: 'disabled' })

  // System prompt assembly (see §6.1 verbatim)
  customPrompt := typeof customSystemPrompt === 'string' ? customSystemPrompt : undefined
  { defaultSystemPrompt, userContext: baseUserContext, systemContext } :=
      await fetchSystemPromptParts({ tools, mainLoopModel: initialMainLoopModel,
        additionalWorkingDirectories, mcpClients, customSystemPrompt: customPrompt })
  userContext := { ...baseUserContext,
                   ...getCoordinatorUserContext(mcpClients,
                        isScratchpadEnabled() ? getScratchpadDir() : undefined) }
  memoryMechanicsPrompt := (customPrompt !== undefined && hasAutoMemPathOverride())
                            ? await loadMemoryPrompt() : null
  systemPrompt := asSystemPrompt([
    ...(customPrompt !== undefined ? [customPrompt] : defaultSystemPrompt),
    ...(memoryMechanicsPrompt ? [memoryMechanicsPrompt] : []),
    ...(appendSystemPrompt ? [appendSystemPrompt] : []),
  ])

  if jsonSchema && tools.some(toolMatchesName(SYNTHETIC_OUTPUT_TOOL_NAME)):
    registerStructuredOutputEnforcement(setAppState, getSessionId())

  pUIC1 := build(ProcessUserInputContext, mutating setMessages)

  if orphanedPermission && !hasHandledOrphanedPermission:
    hasHandledOrphanedPermission := true
    yield* handleOrphanedPermission(orphanedPermission, tools, mutableMessages, pUIC1)

  { messages: messagesFromUserInput, shouldQuery, allowedTools,
    model: modelFromUserInput, resultText } :=
        await processUserInput({ input: prompt, mode: 'prompt', setToolJSX: noop,
                                 context: { ...pUIC1, messages: mutableMessages },
                                 messages: mutableMessages,
                                 uuid: opts?.uuid, isMeta: opts?.isMeta,
                                 querySource: 'sdk' })

  mutableMessages.push(...messagesFromUserInput)
  messages := [...mutableMessages]

  if persistSession && messagesFromUserInput.length > 0:
    p := recordTranscript(messages)
    if isBareMode(): void p          // fire-and-forget
    else:
      await p
      if isEnvTruthy(CLAUDE_CODE_EAGER_FLUSH) || isEnvTruthy(CLAUDE_CODE_IS_COWORK):
        await flushSessionStorage()

  replayableMessages := messagesFromUserInput.filter(...)   // see :466-473
  messagesToAck := replayUserMessages ? replayableMessages : []

  setAppState(prev => { ...prev, toolPermissionContext: {
    ...prev.toolPermissionContext,
    alwaysAllowRules: { ...prev.toolPermissionContext.alwaysAllowRules,
                        command: allowedTools }
  }})

  mainLoopModel := modelFromUserInput ?? initialMainLoopModel

  pUIC2 := rebuild(ProcessUserInputContext, no-op setMessages, mainLoopModel updated)

  [skills, { enabled: enabledPlugins }] := await Promise.all([
    getSlashCommandToolSkills(getCwd()),
    loadAllPluginsCacheOnly(),
  ])

  yield buildSystemInitMessage({
    tools, mcpClients, model: mainLoopModel,
    permissionMode: initialAppState.toolPermissionContext.mode as PermissionMode,
    commands, agents, skills, plugins: enabledPlugins,
    fastMode: initialAppState.fastMode,
  })

  if !shouldQuery:
    fast-path replay (see §5.3)
    return

  if fileHistoryEnabled() && persistSession:
    for each selectable user message: void fileHistoryMakeSnapshot(...)

  drive query loop (see §5.5)
  finalisation (see §5.6)
```

### 5.2 System prompt composition order (`:321-325`) — verbatim concat

```
systemPrompt = asSystemPrompt([
  ...(customPrompt !== undefined ? [customPrompt] : defaultSystemPrompt),
  ...(memoryMechanicsPrompt ? [memoryMechanicsPrompt] : []),
  ...(appendSystemPrompt ? [appendSystemPrompt] : []),
])
```

Invariants:

1. If `customSystemPrompt` is provided (any string, including `''`), the harness's `defaultSystemPrompt` is **completely replaced**. Otherwise the harness default (multi-part array from `fetchSystemPromptParts`) is used.
2. The memory-mechanics prompt is **only** injected when (a) caller passed a `customSystemPrompt` AND (b) `hasAutoMemPathOverride()` is true (env `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE`). Spec 29 / 40.
3. `appendSystemPrompt` is appended last, regardless of which default was used.
4. The result is branded via `asSystemPrompt(...)` — a `string[]` newtype enforced by `src/utils/systemPromptType.ts:12`.

### 5.3 Fast path: `!shouldQuery` (`:556-639`)

If `processUserInput` returns `shouldQuery: false` (slash command produced no LLM call), QueryEngine yields:

- For each `messagesFromUserInput`:
  - If user message containing `<LOCAL_COMMAND_STDOUT_TAG>` or `<LOCAL_COMMAND_STDERR_TAG>`, or `isCompactSummary` → emit `SDKUserMessageReplay` (`:567-580`) with `stripAnsi`'d content, `isReplay: !msg.isCompactSummary`, `isSynthetic: msg.isMeta || msg.isVisibleInTranscriptOnly`.
  - If `system` `local_command` with the local-command tags → emit a synthetic assistant via `localCommandOutputToSDKAssistantMessage` (`:594`).
  - If `system / compact_boundary` → emit `SDKCompactBoundaryMessage` (`:597-605`).
- Persist transcript + maybe flush.
- Yield terminal `result` with `subtype: 'success'`, `is_error: false`, `num_turns: messages.length - 1`, `result: resultText ?? ''`, `stop_reason: null` (`:618-637`). Then `return`.

### 5.4 Why two `ProcessUserInputContext` builds

The first build (`:335-395`) has a real `setMessages` that mutates `this.mutableMessages` because slash commands like `/force-snip` legitimately rewrite the message array. After `processUserInput` runs, no caller past `:431` invokes `setMessages`, so the second build (`:492-527`) substitutes a no-op. Both builds keep the same `discoveredSkillNames` and `loadedNestedMemoryPaths` references so skill discovery within one turn survives the rebuild (`QueryEngine.ts:192-196` comment).

### 5.5 Per-message handling inside `for await (message of query(...))` (`:675-1049`)

Outer `query()` yields internal `Message` records. QueryEngine handles them in this switch:

- **`tombstone`** (`:758-760`): no-op (tombstones are removal signals).

- **`assistant`** (`:761-770`):
  - If `message.message.stop_reason != null`: `lastStopReason := message.message.stop_reason` (synthetic messages set this directly; streamed ones do not — see `stream_event` case below).
  - `mutableMessages.push(message)`; `yield* normalizeMessage(message)`.

- **`progress`** (`:771-783`):
  - Push to `mutableMessages`.
  - If `persistSession`: also push to `messages` and `void recordTranscript(messages)` (fire-and-forget — see comment at `:773-781`: "Without this, deferred progress interleaves with already-recorded tool_results in mutableMessages, and the dedup walk freezes startingParentUuid at the wrong message — forking the chain and orphaning the conversation on resume").
  - `yield* normalizeMessage(message)`.

- **`user`** (`:784-787`):
  - Push to `mutableMessages`; `yield* normalizeMessage(message)`. (`turnCount++` happens earlier at `:753-755`.)

- **`stream_event`** (`:788-828`): the SSE part dispatch.
  - `message_start`: `currentMessageUsage := EMPTY_USAGE`; then `currentMessageUsage := updateUsage(currentMessageUsage, message.event.message.usage)` (`:789-796`).
  - `message_delta`: `currentMessageUsage := updateUsage(currentMessageUsage, message.event.usage)` (`:797-801`); if `message.event.delta.stop_reason != null`, `lastStopReason := message.event.delta.stop_reason` (`:802-808`).
  - `message_stop`: `totalUsage := accumulateUsage(totalUsage, currentMessageUsage)` (`:810-816`).
  - If `includePartialMessages`, yield `{ type: 'stream_event', event: message.event, session_id: getSessionId(), parent_tool_use_id: null, uuid: randomUUID() }` (`:818-826`).

- **`attachment`** (`:829-893`):
  - Always push to `mutableMessages`; if `persistSession`, also push to `messages` + fire-and-forget `recordTranscript(messages)` (same rationale as `progress`).
  - If `attachment.type === 'structured_output'`: `structuredOutputFromTool := attachment.data`.
  - Else if `attachment.type === 'max_turns_reached'`: maybe `flushSessionStorage`; yield terminal `result(error_max_turns)` with `errors: ['Reached maximum number of turns (${maxTurns})']`; **return**.
  - Else if `replayUserMessages && attachment.type === 'queued_command'`: yield `SDKUserMessageReplay` with content from `attachment.prompt` (`:879-891`).

- **`stream_request_start`** (`:894-896`): no emit.

- **`system`** (`:897-958`):
  - First, give `snipReplay` a chance — if it returns `{ messages, executed }`, then if `executed`, replace `mutableMessages` contents and break. (Snip replay is only wired when `feature('HISTORY_SNIP')` is true — see §8.) See `:898-915` for the rationale comment.
  - Else push to `mutableMessages`.
  - If `subtype === 'compact_boundary' && compactMetadata`:
    - Splice out everything before the boundary in both `mutableMessages` and `messages` (GC of pre-compaction history; `:927-933`).
    - Yield `SDKCompactBoundaryMessage` (`:935-941`).
  - If `subtype === 'api_error'`:
    - Yield `system / api_retry` SDK message (`:944-954`) with: `attempt: message.retryAttempt`, `max_retries: message.maxRetries`, `retry_delay_ms: message.retryInMs`, `error_status: message.error.status ?? null`, `error: categorizeRetryableAPIError(message.error)`.
  - Other subtypes: not yielded in headless mode.

- **`tool_use_summary`** (`:959-968`): yield `{ type: 'tool_use_summary', summary, preceding_tool_use_ids, session_id, uuid }`.

#### Compact-boundary persistence ordering (`:701-715`)

Before pushing a `compact_boundary`, if `persistSession`, find `tailUuid := message.compactMetadata?.preservedSegment?.tailUuid` and `await recordTranscript(this.mutableMessages.slice(0, tailIdx + 1))`. This prevents the tail-walk-on-resume from finding an unwritten entry and failing `applyPreservedSegmentRelinks`.

#### Transcript write semantics (`:717-732`)

Inside the assistant/user/compact-boundary branch:

- For `assistant`: **fire-and-forget** `void recordTranscript(messages)` because `claude.ts` mutates the last yielded assistant's `message.usage`/`stop_reason` later via the lazy 100ms write queue; awaiting would block `ask()` from consuming subsequent blocks.
- Else: **awaited** `await recordTranscript(messages)`.

#### Initial-message acknowledgement (`:735-750`)

Latched once: after the first transcript record, replay each `messagesToAck` user message as `SDKUserMessageReplay` (with `isReplay: true`).

### 5.6 Termination conditions (per-loop and post-loop)

Per-loop guards (re-evaluated after every yielded message):

- **`maxBudgetUsd`** (`:972-1002`): if `maxBudgetUsd !== undefined && getTotalCost() >= maxBudgetUsd`, maybe flush, yield `result(error_max_budget_usd)` with `errors: ['Reached maximum budget ($${maxBudgetUsd})']`, return.
- **Structured-output retry budget** (`:1004-1048`): only on `message.type === 'user'` AND `jsonSchema`. Compute `currentCalls := countToolCalls(this.mutableMessages, SYNTHETIC_OUTPUT_TOOL_NAME)`; `callsThisQuery := currentCalls - initialStructuredOutputCalls`; `maxRetries := parseInt(process.env.MAX_STRUCTURED_OUTPUT_RETRIES || '5', 10)`. If `callsThisQuery >= maxRetries`, maybe flush, yield `result(error_max_structured_output_retries)` with `errors: ['Failed to provide valid structured output after ${maxRetries} attempts']`, return.

Post-loop (`:1051-1155`):

- Find the last assistant or user message: `result := messages.findLast(m => m.type === 'assistant' || m.type === 'user')` (`:1058-1060`). Allowlist intentional — Stop hooks can yield `progress`/`attachment` after the assistant response (see comment `:1051-1057`).
- Capture diagnostic strings even on the success-narrowing path: `edeResultType` and `edeLastContentType` (`:1064-1068`).
- If `persistSession` and `EAGER_FLUSH`/`COWORK`: `await flushSessionStorage()`.
- If `!isResultSuccessful(result, lastStopReason)`: yield `result(error_during_execution)` with `errors: ['[ede_diagnostic] result_type=... last_content_type=... stop_reason=...', ...turn-scoped error log slice]`, return.
- Else: extract `textResult := last(result.message.content).text` if last content is `text` AND not in `SYNTHETIC_MESSAGES`; otherwise `''`. Capture `isApiError := Boolean(result.isApiErrorMessage)`. Yield `result(success)` with `structured_output: structuredOutputFromTool`.

### 5.7 `isResultSuccessful` predicate (informational — owned by `utils/queryHelpers.ts:56`)

QueryEngine calls `isResultSuccessful(result, lastStopReason)`. It is a TypeScript type predicate (`message is Message`); inside the `false` branch the type narrows to `never`, which is why `edeResultType` and `edeLastContentType` are captured on a separate line **before** the predicate is evaluated (comment at `:1061-1063`).

**Edge: assistant success includes `redacted_thinking` content blocks.** The predicate at `utils/queryHelpers.ts:62-68` accepts assistant messages whose final content type is `text`, `thinking`, **or `redacted_thinking`**. The downstream `textResult` extraction (`QueryEngine.ts:1124-1133`), however, only accepts a final `text` block — so a successful turn that ended in `thinking`/`redacted_thinking` returns `subtype: 'success'` with `result: ''`. Reimplementers must mirror this asymmetry to avoid spurious `error_during_execution` results on thinking-only finishes.

### 5.8 `error_during_execution.errors[]` watermark

`errorLogWatermark := getInMemoryErrors().at(-1)` is captured at turn start (`:669`). On `error_during_execution`, errors are sliced from `lastIndexOf(errorLogWatermark) + 1` (or 0 if not found), so the report only includes errors logged during this turn. The 100-entry ring buffer can rotate the watermark out — in which case `lastIndexOf` returns `-1` and slice starts at 0 (safe fallback).

### 5.9 The `ask()` wrapper (`:1186-1295`)

```
fn ask(args):
  engine := new QueryEngine({
    ...args,
    initialMessages: mutableMessages,
    readFileCache: cloneFileStateCache(getReadFileCache()),
    ...(feature('HISTORY_SNIP')
      ? { snipReplay: (yielded, store) =>
            !snipProjection!.isSnipBoundaryMessage(yielded)
              ? undefined
              : snipModule!.snipCompactIfNeeded(store, { force: true }) }
      : {}),
  })

  try { yield* engine.submitMessage(prompt, { uuid: promptUuid, isMeta }) }
  finally { setReadFileCache(engine.getReadFileState()) }
```

Notes:

- Caller's `mutableMessages` array is passed by reference as `initialMessages`. QueryEngine mutates it across turns; `ask()` is single-turn so the mutation surfaces back via the same reference.
- `cloneFileStateCache` (`utils/fileStateCache.ts`) deep-clones the file cache so the engine's mutations are isolated from the caller until `setReadFileCache` runs in `finally`.
- `snipReplay` callback **only** lives here, never inside `QueryEngine`. The reason (verbatim from `QueryEngineConfig` JSDoc, `:158-170`): keeps feature-gated strings out of QueryEngine for testability when `feature()` returns `false` under `bun test` (excluded-strings check).

---

## 6. Verbatim Assets

### 6.1 System prompt assembly (`QueryEngine.ts:321-325`) — verbatim

```ts
const systemPrompt = asSystemPrompt([
  ...(customPrompt !== undefined ? [customPrompt] : defaultSystemPrompt),
  ...(memoryMechanicsPrompt ? [memoryMechanicsPrompt] : []),
  ...(appendSystemPrompt ? [appendSystemPrompt] : []),
])
```

QueryEngine itself does NOT contain the default system prompt body — that lives in `fetchSystemPromptParts` (spec 05). It does contain the **memory-mechanics gate** logic (`:316-319`):

```ts
const memoryMechanicsPrompt =
  customPrompt !== undefined && hasAutoMemPathOverride()
    ? await loadMemoryPrompt()
    : null
```

### 6.2 SSE event names handled (verbatim string literals from `services/api/claude.ts:1979-2297` and `QueryEngine.ts:789-810`)

QueryEngine reacts to a subset; the upstream `claude.ts` produces these:

```
'message_start'
'content_block_start'
'content_block_delta'
'content_block_stop'
'message_delta'
'message_stop'
```

Content-block sub-types in `content_block_start.content_block.type`:

```
'tool_use'
'server_tool_use'
'text'
'thinking'
(default: any other type, e.g. 'connector_text', 'advisor_tool_result')
```

Delta sub-types in `content_block_delta.delta.type`:

```
'citations_delta'
'input_json_delta'
'text_delta'
'signature_delta'
'thinking_delta'
'connector_text_delta'        // gated feature('CONNECTOR_TEXT')
```

QueryEngine's three-event interest is exactly: `message_start` (reset `currentMessageUsage`, then `updateUsage`), `message_delta` (`updateUsage` + capture `stop_reason`), `message_stop` (`accumulateUsage` into `totalUsage`).

### 6.3 Retry policy constants (`services/api/withRetry.ts`, owned by spec 22; replicated here for QueryEngine traceability)

| Constant | Value | Citation |
|---|---|---|
| `DEFAULT_MAX_RETRIES` | `10` | `withRetry.ts:52` |
| `FLOOR_OUTPUT_TOKENS` | `3000` | `withRetry.ts:53` |
| `MAX_529_RETRIES` | `3` | `withRetry.ts:54` |
| `BASE_DELAY_MS` | `500` | `withRetry.ts:55` |
| `PERSISTENT_MAX_BACKOFF_MS` | `5 * 60 * 1000` | `withRetry.ts:96` |
| `PERSISTENT_RESET_CAP_MS` | `6 * 60 * 60 * 1000` | `withRetry.ts:97` |
| `HEARTBEAT_INTERVAL_MS` | `30_000` | `withRetry.ts:98` |
| `DEFAULT_FAST_MODE_FALLBACK_HOLD_MS` | `30 * 60 * 1000` | `withRetry.ts:799` |
| `SHORT_RETRY_THRESHOLD_MS` | `20 * 1000` | `withRetry.ts:800` |
| `MIN_COOLDOWN_MS` | `10 * 60 * 1000` | `withRetry.ts:801` |
| Default `getRetryDelay.maxDelayMs` | `32000` | `withRetry.ts:533` |
| Jitter | `Math.random() * 0.25 * baseDelay` (i.e. up to +25 %) | `withRetry.ts:546` |

Backoff formula (`withRetry.ts:530-548`):

```
function getRetryDelay(attempt, retryAfterHeader?, maxDelayMs = 32000):
  if retryAfterHeader: return parseInt(retryAfterHeader, 10) * 1000  (if numeric)
  baseDelay := min(BASE_DELAY_MS * 2^(attempt - 1), maxDelayMs)
  jitter   := Math.random() * 0.25 * baseDelay
  return baseDelay + jitter
```

Retryable status codes (verbatim from `withRetry.ts:760-784`):

- `408` (request timeouts) — retry
- `409` (lock timeouts) — retry
- `429` (rate limit) — retry iff `!isClaudeAISubscriber() || isEnterpriseSubscriber()`
- `401` (token expired) — retry; clears API key cache, OAuth handled in main loop
- `403` "OAuth token has been revoked" — retry
- `5xx` — retry (`>= 500`)
- `529` ("overloaded_error" type, including in message body during streaming) — retry up to `MAX_529_RETRIES` for foreground sources; non-foreground sources bail immediately

`FOREGROUND_529_RETRY_SOURCES` (`withRetry.ts:62-82`, verbatim):

```ts
const FOREGROUND_529_RETRY_SOURCES = new Set<QuerySource>([
  'repl_main_thread',
  'repl_main_thread:outputStyle:custom',
  'repl_main_thread:outputStyle:Explanatory',
  'repl_main_thread:outputStyle:Learning',
  'sdk',
  'agent:custom',
  'agent:default',
  'agent:builtin',
  'compact',
  'hook_agent',
  'hook_prompt',
  'verification_agent',
  'side_question',
  'auto_mode',
  ...(feature('BASH_CLASSIFIER') ? (['bash_classifier'] as const) : []),
])
```

QueryEngine **always** drives the SDK path via `querySource: 'sdk'` (`:683`).

Env overrides (verbatim):

- `process.env.CLAUDE_CODE_MAX_RETRIES` → overrides `DEFAULT_MAX_RETRIES` (`withRetry.ts:790-794`).
- `process.env.CLAUDE_CODE_UNATTENDED_RETRY` (truthy + `feature('UNATTENDED_RETRY')`) → persistent retry mode (`withRetry.ts:100-104`).
- `process.env.FALLBACK_FOR_ALL_PRIMARY_MODELS` (truthy) → 529 fallback considered for all primary models, not just Opus (`withRetry.ts:331-333`).
- `process.env.CLAUDE_CODE_REMOTE` (truthy) → 401/403 are retryable (CCR JWT auth) (`withRetry.ts:712-717`).

### 6.4 Token-counting field set (`NonNullableUsage`, verbatim from `EMPTY_USAGE`)

```
input_tokens                                : number
cache_creation_input_tokens                 : number
cache_read_input_tokens                     : number
output_tokens                               : number
server_tool_use:
  web_search_requests                       : number
  web_fetch_requests                        : number
service_tier                                : 'standard'
cache_creation:
  ephemeral_1h_input_tokens                 : number
  ephemeral_5m_input_tokens                 : number
inference_geo                               : string
iterations                                  : (per-iteration array — model-side artifact)
speed                                       : 'standard'
```

Plus, when `feature('CACHED_MICROCOMPACT')` is true, an additional `cache_deleted_input_tokens` field is conditionally spread into `updateUsage`/`accumulateUsage` outputs (`claude.ts:2970-2982`, `:3024-3033`). This field is intentionally absent from the `NonNullableUsage` type so external builds tree-shake the string out (comment at `claude.ts:2965-2969`).

`updateUsage` semantics (`claude.ts:2924-2987`):

- For `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`: only update when `partUsage.field !== null && partUsage.field > 0` (the API explicitly sends 0 in `message_delta` for fields set in `message_start`; the `> 0` guard preserves the message_start value).
- `output_tokens`: always overwrite with `partUsage.output_tokens ?? usage.output_tokens`.
- `server_tool_use.{web_search_requests,web_fetch_requests}`: `?? usage.{...}`.
- `service_tier`: kept from `usage` (constant for the message).
- `cache_creation.ephemeral_{1h,5m}_input_tokens`: from `(partUsage as BetaUsage).cache_creation?.{...} ?? usage.cache_creation.{...}` (the SDK type is missing this; cast to recover).
- `inference_geo`: kept from `usage`.
- `iterations`: `partUsage.iterations ?? usage.iterations`.
- `speed`: `(partUsage as BetaUsage).speed ?? usage.speed`.

`accumulateUsage` (`claude.ts:2993-3038`):

- All numeric counters add.
- `service_tier`, `inference_geo`, `iterations`, `speed`: take the **most recent** message's value (so post-aggregate reflects the latest tier/geo).

### 6.5 `categorizeRetryableAPIError` — verbatim (`services/api/errors.ts:1163-1182`)

```ts
export function categorizeRetryableAPIError(
  error: APIError,
): SDKAssistantMessageError {
  if (
    error.status === 529 ||
    error.message?.includes('"type":"overloaded_error"')
  ) {
    return 'rate_limit'
  }
  if (error.status === 429) {
    return 'rate_limit'
  }
  if (error.status === 401 || error.status === 403) {
    return 'authentication_failed'
  }
  if (error.status !== undefined && error.status >= 408) {
    return 'server_error'
  }
  return 'unknown'
}
```

### 6.6 Thinking budget constants & gates

From `src/utils/thinking.ts`:

- `ThinkingConfig` (verbatim, `thinking.ts:10-13`):

```ts
export type ThinkingConfig =
  | { type: 'adaptive' }
  | { type: 'enabled'; budgetTokens: number }
  | { type: 'disabled' }
```

- `isUltrathinkEnabled()` (`thinking.ts:19-24`): `feature('ULTRATHINK') && getFeatureValue_CACHED_MAY_BE_STALE('tengu_turtle_carbon', true)`. (The GrowthBook flag name `tengu_turtle_carbon` is verbatim.)
- `hasUltrathinkKeyword(text)` regex: `/\bultrathink\b/i` (`thinking.ts:30`).
- `findThinkingTriggerPositions(text)` regex: `/\bultrathink\b/gi` (`thinking.ts:45`). The `/g` literal is **created fresh per call** (comment at `:42-44`) because `String.prototype.matchAll` copies `lastIndex`.
- `shouldEnableThinkingByDefault()` (`thinking.ts:146-162`): if env `MAX_THINKING_TOKENS` set, returns `parseInt(MAX_THINKING_TOKENS, 10) > 0`. Else `getSettingsWithErrors().settings.alwaysThinkingEnabled === false` → `false`. Else `true`.

Per-model thinking budget (used **only** when not adaptive; `claude.ts:1614-1628`):

- `getMaxThinkingTokensForModel(model)` (`utils/context.ts:219-221`): `getModelMaxOutputTokens(model).upperLimit - 1`.
- The selected `thinkingBudget` is then capped: `Math.min(maxOutputTokens - 1, thinkingBudget)`.
- Caller may override via `ThinkingConfig.budgetTokens` when `type === 'enabled'`.

Adaptive thinking selection rule (`claude.ts:1604-1629`):

```
if hasThinking && modelSupportsThinking(model):
  if !DISABLE_ADAPTIVE_THINKING && modelSupportsAdaptiveThinking(model):
    thinking := { type: 'adaptive' }
  else:
    let thinkingBudget := getMaxThinkingTokensForModel(model)
    if thinkingConfig.type === 'enabled' && thinkingConfig.budgetTokens !== undefined:
      thinkingBudget := thinkingConfig.budgetTokens
    thinkingBudget := min(maxOutputTokens - 1, thinkingBudget)
    thinking := { budget_tokens: thinkingBudget, type: 'enabled' }
```

Env overrides:

- `CLAUDE_CODE_DISABLE_THINKING` (truthy) → `hasThinking := false` (`claude.ts:1598`).
- `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING` (truthy) → forces budget mode (`claude.ts:1606`).
- `MAX_THINKING_TOKENS` (env) — used as gate in `shouldEnableThinkingByDefault()` only.
- `CLAUDE_CODE_MAX_OUTPUT_TOKENS` (env) — used downstream by `getMaxOutputTokensForModel` (referenced `claude.ts:3414`).

When thinking is enabled, the API requires `temperature: 1`; QueryEngine's choice of thinking config thus implicitly pins temperature (`claude.ts:1691-1695`).

### 6.7 SDK envelope shapes for terminal `result`

Common fields (every result variant):

```
duration_ms:        Date.now() - startTime
duration_api_ms:    getTotalAPIDuration()
session_id:         getSessionId()
total_cost_usd:     getTotalCost()
usage:              this.totalUsage
modelUsage:         getModelUsage()
permission_denials: this.permissionDenials
fast_mode_state:    getFastModeState(mainLoopModel, initialAppState.fastMode)
uuid:               randomUUID()
```

Variant-specific:

| Variant | `subtype` | `is_error` | Extras | Citation |
|---|---|---|---|---|
| Fast-path success (no query) | `'success'` | `false` | `num_turns: messages.length - 1`, `result: resultText ?? ''`, `stop_reason: null` | `:618-637` |
| `error_max_turns` | `'error_max_turns'` | `true` | `num_turns: message.attachment.turnCount`, `stop_reason: lastStopReason`, `errors: ['Reached maximum number of turns (${maxTurns})']` | `:851-872` |
| `error_max_budget_usd` | `'error_max_budget_usd'` | `true` | `num_turns: turnCount`, `stop_reason: lastStopReason`, `errors: ['Reached maximum budget ($${maxBudgetUsd})']` | `:981-1001` |
| `error_max_structured_output_retries` | `'error_max_structured_output_retries'` | `true` | `num_turns: turnCount`, `stop_reason: lastStopReason`, `errors: ['Failed to provide valid structured output after ${maxRetries} attempts']` | `:1024-1046` |
| `error_during_execution` | `'error_during_execution'` | `true` | `num_turns: turnCount`, `stop_reason: lastStopReason`, `errors: ['[ede_diagnostic] result_type=${edeResultType} last_content_type=${edeLastContentType} stop_reason=${lastStopReason}', ...turn-scoped error log slice]` | `:1083-1116` |
| Final success | `'success'` | `Boolean(result.isApiErrorMessage)` | `num_turns: turnCount`, `result: textResult`, `stop_reason: lastStopReason`, `structured_output: structuredOutputFromTool` | `:1135-1155` |

`error_during_execution.errors[0]` literal format (verbatim, `:1112`):

```
[ede_diagnostic] result_type=${edeResultType} last_content_type=${edeLastContentType} stop_reason=${lastStopReason}
```

### 6.8 `system / api_retry` SDK envelope (`:944-955`) — verbatim

```ts
yield {
  type: 'system',
  subtype: 'api_retry' as const,
  attempt: message.retryAttempt,
  max_retries: message.maxRetries,
  retry_delay_ms: message.retryInMs,
  error_status: message.error.status ?? null,
  error: categorizeRetryableAPIError(message.error),
  session_id: getSessionId(),
  uuid: message.uuid,
}
```

Source `Message` carries `retryAttempt`, `maxRetries`, `retryInMs`, and the underlying `error: APIError`. The `error` enum is one of `'rate_limit' | 'authentication_failed' | 'server_error' | 'unknown'` (see §6.5).

### 6.9 `tool_use_summary` envelope (`:961-967`) — verbatim

```ts
yield {
  type: 'tool_use_summary' as const,
  summary: message.summary,
  preceding_tool_use_ids: message.precedingToolUseIds,
  session_id: getSessionId(),
  uuid: message.uuid,
}
```

### 6.10 `stream_event` envelope (`:818-826`) — verbatim

```ts
if (includePartialMessages) {
  yield {
    type: 'stream_event' as const,
    event: message.event,
    session_id: getSessionId(),
    parent_tool_use_id: null,
    uuid: randomUUID(),
  }
}
```

`uuid` is freshly minted per emission (NOT the underlying `message.uuid`).

### 6.11 Local-command output detection — verbatim

(`:564-565` and `:591-592`)

```ts
msg.message.content.includes(`<${LOCAL_COMMAND_STDOUT_TAG}>`) ||
msg.message.content.includes(`<${LOCAL_COMMAND_STDERR_TAG}>`)
```

`LOCAL_COMMAND_STDOUT_TAG` and `LOCAL_COMMAND_STDERR_TAG` are XML tag names defined in `src/constants/xml.js` (spec 42 — exact values out of scope here, only their identity is load-bearing).

### 6.12 `MAX_STRUCTURED_OUTPUT_RETRIES` env — verbatim (`:1011-1014`)

```ts
const maxRetries = parseInt(
  process.env.MAX_STRUCTURED_OUTPUT_RETRIES || '5',
  10,
)
```

Default: 5. The check fires only on `message.type === 'user'` AND `jsonSchema` is set.

### 6.13 Profiler checkpoint names — verbatim

QueryEngine emits these to `headlessProfilerCheckpoint`:

- `'before_getSystemPrompt'` (`:284`)
- `'after_getSystemPrompt'` (`:301`)
- `'before_skills_plugins'` (`:529`)
- `'after_skills_plugins'` (`:538`)
- `'system_message_yielded'` (`:554`)

### 6.14 Eager-flush environment variables (verbatim string literals)

- `CLAUDE_CODE_EAGER_FLUSH`
- `CLAUDE_CODE_IS_COWORK`

Both gate `await flushSessionStorage()` in six places:

- After a queued user-message transcript record (`:457-461`).
- In the `!shouldQuery` fast path (`:611-615`).
- Before yielding `result(error_max_turns)` (`:844-849`).
- Before yielding `result(error_max_budget_usd)` (`:973-979`).
- Before yielding `result(error_max_structured_output_retries)` (`:1016-1022`).
- Before yielding any final post-loop result (`:1073-1080`).

---

## 7. Side Effects & I/O

### 7.1 Filesystem

- Transcript writes: `recordTranscript(messages)` (`:451`, `:609`, `:712`, `:728`, `:730`, `:780`, `:834`). On-disk path owned by `utils/sessionStorage.js` (spec 41).
- File-history snapshots: `fileHistoryMakeSnapshot(...)` for each selectable user message when `fileHistoryEnabled() && persistSession` (`:642-654`).
- `flushSessionStorage()` — resolves the lazy 100ms write queue (spec 41).

### 7.2 Network

QueryEngine itself does no direct network I/O. **All** network traffic happens inside `query()` (spec 04) → `services/api/claude.ts` (spec 22). The `loadAllPluginsCacheOnly()` call (`:534-537`) is documented to be cache-only — see comment at `:530-533`: "headless/SDK/CCR startup must not block on network for ref-tracked plugins. CCR populates the cache via `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` (`headlessPluginInstall`) or `CLAUDE_CODE_PLUGIN_SEED_DIR` before this runs; SDK callers that need fresh source can call `/reload-plugins`."

### 7.3 Process spawn

None directly. Tool-side spawns (Bash, git) come through `query()` and live in their tool dirs.

### 7.4 Signals / abort

Single `AbortController` is the abort hub. `interrupt()` aborts; the same controller is plumbed into `pUIC1`/`pUIC2.abortController` and, via `query()`, into every tool call.

### 7.5 Environment variables consumed

Direct (this file):

- `CLAUDE_CODE_EAGER_FLUSH` (`isEnvTruthy`) — see §6.14.
- `CLAUDE_CODE_IS_COWORK` (`isEnvTruthy`) — see §6.14.
- `MAX_STRUCTURED_OUTPUT_RETRIES` — see §6.12.

Indirectly consumed by callees referenced from QueryEngine:

- `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` (via `hasAutoMemPathOverride`).
- `MAX_THINKING_TOKENS`, `CLAUDE_CODE_DISABLE_THINKING`, `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING`, `CLAUDE_CODE_MAX_OUTPUT_TOKENS`, `CLAUDE_CODE_MAX_RETRIES`, `CLAUDE_CODE_UNATTENDED_RETRY`, `CLAUDE_CODE_REMOTE`, `FALLBACK_FOR_ALL_PRIMARY_MODELS`, `IS_SANDBOX`, `USER_TYPE`, `DISABLE_PROMPT_CACHING*`, `CLAUDE_CODE_EXTRA_BODY`, `CLAUDE_CODE_EXTRA_METADATA`, `API_TIMEOUT_MS`, `CLAUDE_ENABLE_STREAM_WATCHDOG`, `CLAUDE_STREAM_IDLE_TIMEOUT_MS`, `CLAUDE_CODE_ENTRYPOINT`, `CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK`, `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, `ENABLE_PROMPT_CACHING_1H_BEDROCK` — all gating decisions live in `claude.ts` / `withRetry.ts` (spec 22). Listed here only for traceability.

### 7.6 Trust boundaries

- `wrappedCanUseTool` is the only permission gate inside QueryEngine; every denial mutates `permissionDenials`. The decision tree itself is owned by spec 09.
- `setAppState` mutates the global `ToolPermissionContext.alwaysAllowRules.command` based on the slash-command processor's `allowedTools` output (`:477-486`).

---

## 8. Feature Flags & Variants

### 8.1 Direct `feature(...)` gates in this file

| Flag | Site | Behavioral delta |
|---|---|---|
| `COORDINATOR_MODE` | `:115` | Inactive: `getCoordinatorUserContext` is `() => ({})`; spread-merge into `userContext` is a no-op. Active: extra `userContext` keys (e.g., scratchpad path, peer info) are merged into the user context block sent to the model. Source `coordinator/coordinatorMode.ts` (spec 30). |
| `HISTORY_SNIP` | `:122-124, :125-127, :1276-1284` | Inactive: `snipReplay` is undefined; `snipModule`/`snipProjection` are `null`; the `system` case never replays. The conditional spread in `ask()` skips the `snipReplay` field. Active: `ask()` injects a `snipReplay` callback that calls `snipProjection.isSnipBoundaryMessage(yielded)`; if true, `snipCompactIfNeeded(store, { force: true })` is invoked. The engine's `system` case (`:898-915`) receives the result and (if `executed`) replaces `mutableMessages` contents. |

### 8.2 Indirect feature flags consumed by code QueryEngine drives

These flags do not appear textually in `QueryEngine.ts` but flip behavior the engine observes:

| Flag | Where it affects QueryEngine output | Citation |
|---|---|---|
| `ULTRATHINK` | When ON, `isUltrathinkEnabled()` returns true → `effort` interpretation (spec 06 / 21). Does not directly change the engine's thinking config but downstream `claude.ts` may select different `output_config.effort`. | `utils/thinking.ts:20`; `utils/effort.ts:322` |
| `TRANSCRIPT_CLASSIFIER` | Activates `auto` permission mode + AFK-mode beta. QueryEngine merely reports `permissionMode` from `initialAppState.toolPermissionContext.mode`; the `'auto'` value flows through unchanged. | `claude.ts:1413,1661`; `types/permissions.ts:35` |
| `CACHED_MICROCOMPACT` | Adds `cache_deleted_input_tokens` to `updateUsage`/`accumulateUsage` outputs (spec 22). QueryEngine's `totalUsage` carries this field through to the terminal `result.usage` if present. | `claude.ts:2970-2982,3024-3033` |
| `CHICAGO_MCP` | MCP-related dispatcher path inside `query.ts`. Does not appear in QueryEngine but messages it receives from `query()` may differ. | `query.ts:1033,1489` (spec 04) |
| `BG_SESSIONS` | Background-task summary attachments may surface as additional `attachment` messages QueryEngine pushes/persists. | `query.ts:118,1685` (spec 04) |
| `TOKEN_BUDGET` | Per-turn budget tracker — affects when `query()` emits `max_turns_reached` attachments. | `query.ts:280,1308` (spec 04) |
| `PROMPT_CACHE_BREAK_DETECTION` | Triggers `checkResponseForCacheBreak` after stream finishes; QueryEngine just sees the resulting `usage` numbers. | `claude.ts:2383` |
| `CONNECTOR_TEXT` | Adds `connector_text` content blocks. QueryEngine yields them via `normalizeMessage`. | `claude.ts:2067, 2129` |
| `ANTI_DISTILLATION_CC` | Adds an extra cache-control header. Transparent to QueryEngine. | `claude.ts:303-304` |
| `UNATTENDED_RETRY` | Enables persistent retry mode — emits chunked `system / api_retry` messages every `HEARTBEAT_INTERVAL_MS` (30 s). QueryEngine forwards each. | `withRetry.ts:101, 477-503` |
| `UDS_INBOX` | When ON, `buildSystemInitMessage` adds a hidden `messaging_socket_path` field to the `system / init` envelope QueryEngine yields at `:540-551`. QueryEngine forwards the field through transparently. | `utils/messages/systemInit.ts:87-94` |
| `REACTIVE_COMPACT` | Caller-side reactive recovery in `query.ts`: when set, the turn pipeline retries with reactive compaction on prompt-too-long; the resulting `compact_boundary` flows through QueryEngine's `system` case. | `query.ts:15` (spec 04) |
| `CONTEXT_COLLAPSE` | Multi-site collapse pipeline in `query.ts` that withholds prompt-too-long / media-size errors until recovery resolves; QueryEngine sees only the post-recovery message stream. | `query.ts:18,440,616,800,1090,1176` (spec 04) |
| `EXPERIMENTAL_SKILL_SEARCH` | When ON, `query.ts` may discover skills via local index; the resulting skill names are populated into `discoveredSkillNames` (turn-scoped Set) which QueryEngine clears at `submitMessage` entry. | `query.ts:66` (spec 04) |
| `TEMPLATES` | Job-classifier path in `query.ts`. Affects which template/classifier QueryEngine's downstream messages reflect; not visible in `QueryEngine.ts` directly. | `query.ts:69` (spec 04) |
| `HISTORY_SNIP` (query-side) | Beyond the QueryEngine-direct gates at `:122-127, :1276-1284`, `query.ts` itself uses `HISTORY_SNIP` at `:115, :401` to inject snip-related processing into the turn loop. The two gates compose. | `query.ts:115, 401` (spec 04) |
| `CACHED_MICROCOMPACT` (query-side) | In addition to the `cache_deleted_input_tokens` field path noted above, `query.ts:423, 870` uses `CACHED_MICROCOMPACT` to drive the cached-microcompact code path; QueryEngine sees the resulting `compact_boundary` stream. | `query.ts:423, 870` (spec 04) |

### 8.3 Non-`feature()` runtime gates affecting QueryEngine paths

| Gate | Effect | Citation |
|---|---|---|
| `isSessionPersistenceDisabled()` | Skips ALL transcript writes and flushes within this turn. | `:240` |
| `isBareMode()` | Initial transcript write becomes fire-and-forget. | `:452-454` |
| `isEnvTruthy(CLAUDE_CODE_EAGER_FLUSH \|\| CLAUDE_CODE_IS_COWORK)` | Adds `await flushSessionStorage()` at six points. | `:457-461,611-615,844-849,973-979,1016-1022,1073-1080` |
| `isScratchpadEnabled()` | Passes scratchpad dir to `getCoordinatorUserContext`. | `:306` |
| `fileHistoryEnabled()` | Per-user-message file-history snapshots. | `:641` |
| `shouldEnableThinkingByDefault()` | Default `ThinkingConfig` is `'adaptive'` if undefined-config-and-not-disabled; `'disabled'` otherwise. | `:280-282` |

### 8.4 ANT-only

QueryEngine itself contains zero `process.env.USER_TYPE === 'ant'` reads. Every ANT branch it depends on is downstream:

- ANT-only thinking model support widening (`utils/thinking.ts:95-99`).
- ANT-only fast-mode mock-rate-limits (`withRetry.ts:202-210`).
- ANT-only research field on assistant messages (`claude.ts:1986-1991, 2166, 2205-2207, 2220-2227`).
- ANT-only `cache_deleted_input_tokens` field gating (via `feature('CACHED_MICROCOMPACT')` — disjoint from `USER_TYPE`).

These pass through transparently in QueryEngine's normalisation (which preserves whatever the upstream emitted).

---

## 9. Error Handling & Edge Cases

### 9.1 Failure modes

1. **`processUserInput` returns `shouldQuery: false` but with no consumable output**: yields `result(success)` with `result: ''`. (`:618-637`)
2. **Empty stream**: covered by `claude.ts:2350-2364` — `query()` raises before reaching QueryEngine, surfaces as `error_during_execution` via `isResultSuccessful` returning false.
3. **API retry exhaustion**: `withRetry` throws `CannotRetryError`; bubbles out of `query()` as a synthetic assistant `isApiErrorMessage` or causes the loop to exit with `lastStopReason` unset → `error_during_execution`.
4. **Fallback model triggered**: `withRetry` throws `FallbackTriggeredError`; per spec 22 this restarts the streaming with the fallback model.
5. **Stop hooks emit progress/attachment after assistant**: handled by `findLast(m => m.type === 'assistant' || m.type === 'user')` (`:1058-1060`) so `last(messages)` doesn't return a hook artifact.
6. **Compact boundary written before tail message**: handled by `:701-715` (record up through tail before emitting boundary).
7. **`messages.length === 0` (no `mutableMessages` yet) AND `messagesFromUserInput.length === 0`**: `messagesToAck` is empty; `replayableMessages` filter returns empty; final `result` extracts `result === undefined` → `isResultSuccessful` returns false → `error_during_execution`.
8. **Orphaned permission consumed**: `hasHandledOrphanedPermission` latches; subsequent turns ignore it (`:398-408`).
9. **Stream `'tombstone'` and `'stream_request_start'`**: deliberately silent.
10. **Snip replay returns `executed: false`**: `mutableMessages` left untouched; engine treats as non-event and breaks (`:909-915`).

### 9.2 User-facing error strings (verbatim)

QueryEngine inlines exactly these:

- `` `Reached maximum number of turns (${message.attachment.maxTurns})` `` (`:870`)
- `` `Reached maximum budget ($${maxBudgetUsd})` `` (`:999`)
- `` `Failed to provide valid structured output after ${maxRetries} attempts` `` (`:1043`)
- `` `[ede_diagnostic] result_type=${edeResultType} last_content_type=${edeLastContentType} stop_reason=${lastStopReason}` `` (`:1112`)

It does NOT inline any other user-visible API error strings — those originate in `claude.ts` (e.g., `API_ERROR_MESSAGE_PREFIX`, refusal messages — spec 22).

### 9.3 Edge: `currentMessageUsage` reset semantics

Reset happens at every `message_start` event, NOT at the start of a turn. A turn may stream multiple assistant messages back-to-back (rare but legal under multi-message replies) — each gets its own watermark accumulated into `totalUsage`. This is intentional (`QueryEngine.ts:657-658` comment: "Track current message usage (reset on each message_start)").

### 9.4 Edge: assistant `stop_reason` capture order

The streaming flow yields the assistant message at `content_block_stop` with `stop_reason: null` (it hasn't arrived yet). The real value lands in `message_delta` (`claude.ts:2242-2247`). QueryEngine therefore captures `lastStopReason` in two places:

1. From `message.message.stop_reason` if non-null (synthetic / non-streaming assistant) — `:765-767`.
2. From `message.event.delta.stop_reason` on `message_delta` — `:806-808`.

If neither happens, `lastStopReason` stays `null` and the post-loop branch likely flags `error_during_execution`.

### 9.5 Edge: `error_during_execution.errors[]` watermark rotation

The in-memory error log is a 100-entry ring buffer (`utils/log.ts`). If 100+ errors occur during one turn, the watermark is rotated out. `lastIndexOf(errorLogWatermark)` returns `-1`; the slice starts at `0` and includes the entire (rotated) buffer (`:1106-1115`). Comment `:670-671`: "If this entry is rotated out, lastIndexOf returns -1 and we include everything (safe fallback)."

---

## 10. Telemetry & Observability

### 10.1 Profiler checkpoints emitted

`headlessProfilerCheckpoint(name)` calls (`:284, :301, :529, :538, :554`) — see §6.13 for the verbatim names.

### 10.2 Analytics events

QueryEngine itself does NOT call `logEvent`. All telemetry comes from downstream modules:

- `tengu_api_retry`, `tengu_api_529_background_dropped`, `tengu_api_opus_fallback_triggered`, `tengu_api_custom_529_overloaded_error`, `tengu_max_tokens_context_overflow_adjustment`, `tengu_api_persistent_retry_wait` (`withRetry.ts`).
- `tengu_max_tokens_reached`, `tengu_context_window_exceeded`, `tengu_streaming_error`, `tengu_streaming_stall_summary`, `tengu_stream_no_events`, `tengu_stream_loop_exited_after_watchdog`, `tengu_advisor_tool_call`, `tengu_refusal_api_response`, `tengu_ultrathink` (`claude.ts` / `attachments.ts`).

### 10.3 SDK status

`config.setSDKStatus?.(status: SDKStatus)` is plumbed through both `pUIC1` and `pUIC2`. QueryEngine itself does not call it; downstream pipeline phases do.

### 10.4 Permission denials

Recorded in `permissionDenials: SDKPermissionDenial[]`, surfaced on every terminal `result.permission_denials`. Format:

```ts
{
  tool_name: sdkCompatToolName(tool.name),
  tool_use_id: toolUseID,
  tool_input: input,
}
```

### 10.5 Cost reporting

Each terminal `result.total_cost_usd` is `getTotalCost()`; `result.modelUsage` is `getModelUsage()`. QueryEngine does not aggregate cost itself — that lives in `cost-tracker.ts` (spec 06). It does emit `result.usage = this.totalUsage` and `result.duration_api_ms = getTotalAPIDuration()`.

---

## 11. Reimplementation Checklist

A behaviorally equivalent QueryEngine must preserve:

- [ ] The exact construction order of `systemPrompt`: `[customPrompt OR defaultSystemPrompt, memoryMechanics?, append?]` (§6.1).
- [ ] Memory-mechanics injection ONLY when both `customSystemPrompt` is set AND `hasAutoMemPathOverride()` is true.
- [ ] Default `ThinkingConfig`: `{ type: 'adaptive' }` if `shouldEnableThinkingByDefault() !== false`, else `{ type: 'disabled' }`.
- [ ] Per-message usage watermark reset at each `message_start`, not at turn start.
- [ ] `accumulateUsage` ONLY at `message_stop`, not after every delta.
- [ ] `updateUsage`'s `> 0` guard for `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens` — do NOT overwrite from `message_delta` zeros.
- [ ] `lastStopReason` capture on both assistant-message stop_reason (synthetic) and stream_event message_delta delta.stop_reason.
- [ ] Two `ProcessUserInputContext` builds per turn (mutating + no-op) sharing the same `discoveredSkillNames` and `loadedNestedMemoryPaths` Sets.
- [ ] `discoveredSkillNames.clear()` at the start of every `submitMessage`.
- [ ] `loadedNestedMemoryPaths` is NOT cleared per turn (conversation-scoped).
- [ ] Plugin/skill loading on the turn boundary is **cache-only** — no network.
- [ ] Bare-mode initial transcript is fire-and-forget; otherwise awaited.
- [ ] CLAUDE_CODE_EAGER_FLUSH || CLAUDE_CODE_IS_COWORK gates `flushSessionStorage()` at exactly the six documented points.
- [ ] `recordTranscript` is **fire-and-forget for assistant** messages, **awaited** for user/compact_boundary.
- [ ] Compact-boundary persistence: write transcript through `tailUuid` BEFORE emitting the SDK boundary message.
- [ ] Compact-boundary GC: splice both `mutableMessages` and `messages` to discard pre-boundary entries.
- [ ] Snip replay is wired only when `feature('HISTORY_SNIP')` is true; QueryEngine itself stays free of snip-specific strings.
- [ ] `error_during_execution.errors[0]` carries the `[ede_diagnostic]` prefix string with `result_type`, `last_content_type`, `stop_reason`.
- [ ] `error_during_execution.errors[]` is turn-scoped via `errorLogWatermark`; rotation falls back to including the entire ring buffer.
- [ ] Permission denials are tracked per-conversation and reported on every terminal result.
- [ ] Initial-message acknowledgement (`messagesToAck`) replays only after the FIRST transcript record of the turn.
- [ ] `replayUserMessages` controls whether `messagesToAck` is non-empty.
- [ ] `find result := messages.findLast(assistant|user)` (allowlist), NOT `last(messages)` — Stop hooks may yield progress/attachment after the assistant.
- [ ] `MAX_STRUCTURED_OUTPUT_RETRIES` defaults to 5, parsed from env, checked only on user messages when `jsonSchema` is set.
- [ ] `maxBudgetUsd` checked after every yielded message (not only assistant).
- [ ] Final `textResult` excludes `SYNTHETIC_MESSAGES` content.
- [ ] `result.is_error` on the final success variant is `Boolean(result.isApiErrorMessage)` — meaning a successful turn that ended in a synthetic API-error message still returns `subtype: 'success'` but with `is_error: true`.
- [ ] `permission_denials`, `usage`, `modelUsage`, `fast_mode_state`, `total_cost_usd`, `duration_ms`, `duration_api_ms`, `session_id`, `uuid`, `stop_reason` populated on every terminal result variant.
- [ ] `setAppState`'s update of `toolPermissionContext.alwaysAllowRules.command := allowedTools` happens AFTER `processUserInput` and BEFORE the second context build.
- [ ] `setCwd(cwd)` and `discoveredSkillNames.clear()` are the first effects of every `submitMessage`.

---

## 12. Open Questions / Unknowns

1. **`SDKAssistantMessageError` enum closed set**. `categorizeRetryableAPIError` returns `'rate_limit' | 'authentication_failed' | 'server_error' | 'unknown'`. Other return sites in `errors.ts` (e.g., `categorizeApiError`, line 1140 fragment) include `'bedrock_model_access'`, `'ssl_cert_error'`, `'connection_error'`, `'client_error'` — verifying whether these can flow into the SDK envelope's `system / api_retry.error` field requires tracing every `Message.subtype === 'api_error'` producer in `query.ts` (spec 04). Estimated: only `categorizeRetryableAPIError`'s subset reaches QueryEngine.

2. **`SDKMessage` envelope type**: the actual closed union in `entrypoints/agentSdkTypes.js` was not read in full for this spec; required for spec 01 / SDK consumers to type-check against. Recommend spec 01 inline its full discriminated-union form.

3. **`Message` type definition path**: `src/types/message.ts` is referenced by import (`:42`) but does not exist at that path; per 00-overview §2.4, it is likely in `src/types/generated/`. Spec 04 / 08 must locate definitively.

4. **Backoff cap interaction with `Retry-After`**. `getRetryDelay` honours a `Retry-After` header by returning `seconds * 1000` directly with no jitter; meaning if the server sends a tiny value (e.g., `0`), there is effectively no backoff. The `maxDelayMs` cap (default 32 000 ms) is bypassed. In persistent mode there's an outer `PERSISTENT_RESET_CAP_MS` (6 h). Recommend spec 22 surface this asymmetry.

5. **Race between `setSDKStatus` callbacks across the two `pUIC` builds**: pUIC1 and pUIC2 share the same `setSDKStatus` reference but pUIC1 is short-lived (orphaned-permission + `processUserInput`). Whether status writes from pUIC1's lifecycle could be observed by `query()` reading via pUIC2 was not traced. Estimated: harmless because both write to the same caller-supplied function.

6. **Assistant-message `research` field clobbering across multi-message turns** (ANT-only). `claude.ts:2224-2227` walks `newMessages` and writes `research` back into each on `message_delta`. QueryEngine pushes the message into `mutableMessages` at `content_block_stop` time (before `message_delta`). Whether the `research` mutation hits the already-pushed message reliably (via shared object reference) was confirmed by reading `claude.ts:2192-2210` (object reference is shared) but the timing across persistence is fragile if `recordTranscript` fires between `content_block_stop` and `message_delta`. The fire-and-forget + lazy 100ms write queue mitigation appears sufficient; flagging for adversarial review.

7. **`isResultSuccessful` semantics**. Source predicate at `utils/queryHelpers.ts:56` was grep-inspected, not fully read for this spec. Required for verifying §5.6 narrowing claims. Spec 04 should surface its full body.

8. **Tombstone messages** are silently skipped (`:758-760`) but their semantics (when they're emitted, by whom) are owned by `query.ts` (spec 04). Whether QueryEngine must do anything beyond skipping (e.g., persistence side-effects) is unclear.

9. **`include_partial_messages` ordering**: `stream_event` emissions happen INSIDE the same loop iteration as the `assistant`/`message_delta`/`message_stop` handling, but BEFORE the `maxBudgetUsd` and structured-output-retry checks (`:1004-1048`). Whether this means a budget check on the same message can come AFTER a `stream_event` for that same message is, on close reading, yes — and is intentional (budget is checked at message granularity, stream events are partial). No bug, but worth verifying with adversarial review.

10. **Snip-projection module's `isSnipBoundaryMessage` definition** — referenced via `snipProjection!.isSnipBoundaryMessage(yielded)` (`:1279-1280`). Source not read for this spec. Spec 07 owns it; assumed: returns true for `system` messages with a snip-boundary subtype string.

---
