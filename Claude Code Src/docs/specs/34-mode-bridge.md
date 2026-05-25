# 34 — Bridge Mode (`BRIDGE_MODE`, Remote Control) Specification

> **Adjacent specs (do not redocument)**: 16 (IDE/MCP allowlist), 23 (MCP service), 25 (OAuth/JWT/Keychain), 33 (daemon mode), 35 (remote/CCR runner side), 37 (Ink UI shell), 41 (session state).

---

## 1. Purpose & Scope

`src/bridge/` implements **Remote Control** — the CLI side of the bidirectional channel that lets a local Claude Code REPL be driven from `claude.ai/code` (web/mobile/iOS) and vice-versa. The product name in user-facing strings is **"Remote Control"**; the build flag is `BRIDGE_MODE`; the GrowthBook gate is `tengu_ccr_bridge`. CCR = "Code Cloud Runtime".

**Important**: `bridge/` does **not** implement an IDE plugin (VS Code/JetBrains) channel. It implements the cloud-relay channel between a CLI process and `claude.ai/code` over HTTPS+WebSocket(v1) or HTTPS+SSE(v2). The IDE-side MCP (`vscodeSdkMcp.ts`) is owned by spec 23, and its allowlist (`ALLOWED_IDE_TOOLS`) is owned by spec 16 — re-cited verbatim in §6.7 below per dispatcher direction. The bridge has two transport flavors:

- **v1 ("env-based")**: `POST /v1/environments/bridge` register → `/work/poll` → ack/heartbeat → `WebSocket session_ingress`.
- **v2 ("env-less")**: `POST /v1/code/sessions` → `POST /v1/code/sessions/{id}/bridge` → `SSE stream` reads + `POST /worker/events` writes via `CCRClient`.

Two independent runtimes live in this directory:

- **`initReplBridge` / `initBridgeCore`** (in-process REPL bridge attached to the live REPL — `src/bridge/initReplBridge.ts:110-545`, `replBridge.ts:1-2406`).
- **`runBridgeLoop` / `bridgeMain`** (the standalone `claude remote-control` server that polls for work and **spawns child** `claude` processes — `bridgeMain.ts:1-2999`).

**IN scope** (per dispatcher):
1. The whole of `src/bridge/` (32 files / 12,613 LOC).
2. Build/runtime gates: `BRIDGE_MODE`, `CCR_AUTO_CONNECT`, `CCR_MIRROR`, `KAIROS` interactions.
3. JWT-based session-ingress auth (cross-cite spec 25 for OAuth source).
4. The REPL bridge / IPC channel layout (transport adapters, control-request protocol).
5. `BRIDGE_SAFE_COMMANDS` allowlist (cross-cite spec 20 §6.3).
6. `bridge/bridgeEnabled.ts:186` — `CCR_AUTO_CONNECT`.
7. Lock / discovery: `bridge-pointer.json` (crash-recovery pointer, **not** an IDE→CLI lockfile).
8. Message protocol (SDK message envelope + `control_request`/`control_response`).
9. Editor-side handlers (no IDE-side code lives here — see §1 caveat).
10. Settings sync — handled by per-session `getMessages`/`onUserMessage` callbacks, the `/rename` →`updateBridgeSessionTitle` PATCH, and the `tengu_bridge_repl_v2_config` GrowthBook config (no on-disk settings sync).

**OUT of scope** (deferred to specs in parens):
- Generic tool dispatching → 04, 08.
- MCP service → 23.
- OAuth provider/Keychain mechanics → 25.
- Daemon mode entrypoint → 33.
- Remote/CCR runner & `claude remote-setup` → 35.
- Ink shell + bridge UI rendering → 37.
- Session/transcript history → 41.
- VS Code / JetBrains IDE MCP servers (`vscodeSdkMcp.ts`) → 23.

---

## 2. Source Map

### 2.1 Owned files (full inventory)

All paths under `src/bridge/`. Line totals from `wc -l` (12,613 LOC total).

| File | LOC | Coverage | Role |
|---|---:|---|---|
| `bridgeApi.ts` | 539 | sampled (`:1-200`, `:200-540`) | v1 environments-API HTTP client, `BridgeFatalError`, `validateBridgeId`, `withOAuthRetry` |
| `bridgeConfig.ts` | 48 | full | `getBridgeAccessToken`, `getBridgeBaseUrl`, ANT-only env overrides |
| `bridgeDebug.ts` | 135 | full | ANT-only fault-injection (`/bridge-kick`) |
| `bridgeEnabled.ts` | 202 | full | `feature('BRIDGE_MODE')` runtime gates, `getCcrAutoConnectDefault`, `isCcrMirrorEnabled` |
| `bridgeMain.ts` | 2999 | sampled (`:1-200`, `:600-700`, `:840-870`, `:2096`, `:3471`, `:3866-3879`, `:4317-4325`) | Standalone server: poll loop, multi-session spawn, capacity wake |
| `bridgeMessaging.ts` | 461 | full | Transport-agnostic ingress routing, `BoundedUUIDSet`, `handleServerControlRequest`, `makeResultMessage` |
| `bridgePermissionCallbacks.ts` | 43 | full | `BridgePermissionResponse` shape + predicate |
| `bridgePointer.ts` | 210 | full | `bridge-pointer.json` (crash-recovery) |
| `bridgeStatusUtil.ts` | 163 | full | UI status state machine constants and URL builders |
| `bridgeUI.ts` | 530 | sampled | Standalone-bridge live status display |
| `capacityWake.ts` | 56 | full | `createCapacityWake` — outer-signal × wake-controller merger |
| `codeSessionApi.ts` | 168 | full | v2 `POST /v1/code/sessions` + `POST /v1/code/sessions/{id}/bridge` |
| `createSession.ts` | 384 | full | v1-compat `POST /v1/sessions`, get/archive/rename |
| `debugUtils.ts` | 141 | full | Secret redaction, `logBridgeSkip`, `extractErrorDetail` |
| `envLessBridgeConfig.ts` | 165 | full | `EnvLessBridgeConfig` Zod schema + GrowthBook fetch |
| `flushGate.ts` | 71 | full | `FlushGate<T>` queue gate during initial history flush |
| `inboundAttachments.ts` | 175 | full | `file_uuid` resolution → `~/.claude/uploads/{sessionId}/` → `@path` refs |
| `inboundMessages.ts` | 80 | full | `extractInboundMessageFields`, `normalizeImageBlocks` |
| `initReplBridge.ts` | 569 | full | REPL-side bootstrap: gate checks, OAuth refresh, title derivation, v1/v2 branch |
| `jwtUtils.ts` | 256 | full | `decodeJwtPayload`, `decodeJwtExpiry`, `createTokenRefreshScheduler` |
| `pollConfig.ts` | 110 | full | Poll-interval config Zod schema + `getPollIntervalConfig` |
| `pollConfigDefaults.ts` | 82 | full | `DEFAULT_POLL_CONFIG`, public type |
| `remoteBridgeCore.ts` | 1008 | sampled (entry point only) | Env-less (v2) bridge core |
| `replBridge.ts` | 2406 | sampled (`:200-700`, `:1500-1610`, `:1880-2400`) | REPL bridge core (`initBridgeCore`) |
| `replBridgeHandle.ts` | 36 | full | Process-global `ReplBridgeHandle` pointer |
| `replBridgeTransport.ts` | 370 | full | `ReplBridgeTransport` adapter; v1/v2 variants |
| `sessionIdCompat.ts` | 57 | full | `cse_*` ↔ `session_*` retag shim |
| `sessionRunner.ts` | 550 | sampled (entry-point only) | Child-process spawner used by `bridgeMain` |
| `trustedDevice.ts` | 210 | full | `X-Trusted-Device-Token` keychain read + enrollment |
| `types.ts` | 262 | full | `BridgeConfig`, `WorkSecret`, `WorkResponse`, `SessionHandle`, `BridgeApiClient`, user-facing strings |
| `workSecret.ts` | 127 | full | `decodeWorkSecret`, `buildSdkUrl`, `buildCCRv2SdkUrl`, `registerWorker`, `sameSessionId` |

### 2.2 Build flags / gates governing this subsystem

| Gate | Type | Location | Effect |
|---|---|---|---|
| `feature('BRIDGE_MODE')` | `bun:bundle` | `bridgeEnabled.ts:32,51,71,127,142,164`; `commands.ts:73,77`; `main.tsx:2246,3866,4322` | Whole subsystem stripped if absent. |
| `feature('CCR_AUTO_CONNECT')` | `bun:bundle` | `bridgeEnabled.ts:186` | Default for `remoteControlAtStartup` ← `tengu_cobalt_harbor`. ANT-only. |
| `feature('CCR_MIRROR')` | `bun:bundle` | `bridgeEnabled.ts:198` | Outbound-only mirror mode. |
| `feature('KAIROS')` | `bun:bundle` | `initReplBridge.ts:477-485` | Sets `workerType='claude_code_assistant'` if `isAssistantMode()`. |
| `feature('DAEMON') && feature('BRIDGE_MODE')` | both | `commands.ts:77` | Loads `remoteControlServer` command. |
| `tengu_ccr_bridge` | GrowthBook gate | `bridgeEnabled.ts:34,53,81` | Per-account entitlement. |
| `tengu_bridge_repl_v2` | GrowthBook value (bool) | `bridgeEnabled.ts:128` | REPL-only env-less branch. |
| `tengu_bridge_repl_v2_cse_shim_enabled` | GrowthBook value (bool) | `bridgeEnabled.ts:143-147` | Defaults `true`; controls `cse_*`→`session_*` retag. |
| `tengu_bridge_min_version` | GrowthBook dynamic config | `bridgeEnabled.ts:165-167` | v1 semver floor. |
| `tengu_bridge_repl_v2_config` | GrowthBook value (object) | `envLessBridgeConfig.ts:131` | All v2 retry/heartbeat/timeout values (Zod-validated). |
| `tengu_bridge_poll_interval_config` | GrowthBook value (object) | `pollConfig.ts:103` | Poll cadence (Zod-validated). |
| `tengu_bridge_initial_history_cap` | GrowthBook value (int) | `initReplBridge.ts:381-384` | Default `200`. |
| `tengu_cobalt_harbor` | GrowthBook value (bool) | `bridgeEnabled.ts:187` | Auto-connect default. |
| `tengu_ccr_mirror` | GrowthBook value (bool) | `bridgeEnabled.ts:200` | CCR mirror default. |
| `tengu_ccr_bridge_multi_session` | GrowthBook gate (blocking) | `bridgeMain.ts:97` | Multi-session spawn entitlement. |
| `tengu_sessions_elevated_auth_enforcement` | GrowthBook value (bool) | `trustedDevice.ts:33-37` | Whether to send `X-Trusted-Device-Token`. |
| `USER_TYPE === 'ant'` | runtime env | `bridgeConfig.ts:20,29`; `bridgeDebug.ts:84`; `initReplBridge.ts:468` | Enables ANT env overrides + `/bridge-kick` + debug-log path display. |
| `process.env.CLAUDE_TRUSTED_DEVICE_TOKEN` | runtime env | `trustedDevice.ts:47` | Env-var override for trusted-device token. |
| `process.env.CLAUDE_CODE_CCR_MIRROR` | runtime env | `bridgeEnabled.ts:199` | Env-truthy local opt-in. |
| `process.env.CLAUDE_CODE_OAUTH_TOKEN` | runtime env | `bridgeEnabled.ts:64-68` (note) | Setup-token; lacks `user:profile` scope ⇒ disqualifies. |
| `process.env.CLAUDE_CODE_SESSION_ACCESS_TOKEN` | runtime env | `replBridgeTransport.ts:179-181` | Process-wide v2 session-ingress token. |

### 2.3 Imports from / Imported by

**Imports from**: `services/analytics/growthbook.js`, `services/oauth/client.js`, `services/policyLimits/index.js`, `utils/auth.js`, `utils/secureStorage/index.js`, `cli/transports/{HybridTransport,SSETransport,ccrClient}.js`, `entrypoints/agentSdkTypes.js`, `entrypoints/sdk/controlTypes.js`, `bootstrap/state.js`, `utils/concurrentSessions.js`, `constants/oauth.js`, `constants/product.js`.

**Imported by**: `commands.ts:73-78`, `commands/bridge/index.ts`, `cli/print.ts:1502,1781`, `hooks/useReplBridge.tsx:8,40,113,519,658`, `entrypoints/sdk/*`, `commands/remoteControlServer/*`, `commands/bridge-kick.ts`, daemon callers using env-var auth.

### 2.4 Cross-spec integration table

| Cross-cite to | What | Citation here |
|---|---|---|
| 16 / 23 | `ALLOWED_IDE_TOOLS = ['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']` | `services/mcp/client.ts:568` (verbatim §6.7) |
| 23 | MCP `vscodeSdkMcp.ts` — IDE-side bridge produces `sse-ide`/`ws-ide` transports | Out-of-scope here. |
| 25 | OAuth source for bridge auth | `getClaudeAIOAuthTokens()`; trusted-device via `secureStorage`. |
| 20 §6.3 | `BRIDGE_SAFE_COMMANDS` allowlist | `commands.ts:651-660` (verbatim §6.4). |
| 33 | Daemon paths skip OAuth flow / use IPC auth | `getBridgeTokenOverride()` bypass at `initReplBridge.ts:168`. |
| 35 | CCR runner side / `claude remote-setup` | This spec is the CLI client; worker contract owned by 35. |
| 41 | `bridge-pointer.json` lives next to JSONL transcripts | `bridgePointer.ts:52-54`. |

### 2.5 Unresolved / deferred ranges

See §12 for unread regions of `replBridge.ts`, `bridgeMain.ts`, `remoteBridgeCore.ts`, `bridgeUI.ts`, `sessionRunner.ts`.

---

## 3. Public Interface (Contract)

### 3.1 Runtime entry points

```ts
// src/bridge/initReplBridge.ts:110
export async function initReplBridge(
  options?: InitBridgeOptions,
): Promise<ReplBridgeHandle | null>
```

```ts
// src/bridge/initReplBridge.ts:75-108  (verbatim, full)
export type InitBridgeOptions = {
  onInboundMessage?: (msg: SDKMessage) => void | Promise<void>
  onPermissionResponse?: (response: SDKControlResponse) => void
  onInterrupt?: () => void
  onSetModel?: (model: string | undefined) => void
  onSetMaxThinkingTokens?: (maxTokens: number | null) => void
  onSetPermissionMode?: (
    mode: PermissionMode,
  ) => { ok: true } | { ok: false; error: string }
  onStateChange?: (state: BridgeState, detail?: string) => void
  initialMessages?: Message[]
  initialName?: string
  getMessages?: () => Message[]
  previouslyFlushedUUIDs?: Set<string>
  perpetual?: boolean
  outboundOnly?: boolean
  tags?: string[]
}
```

```ts
// src/bridge/bridgeMain.ts:141 (signature only)
export async function runBridgeLoop(
  config: BridgeConfig,
  environmentId: string,
  environmentSecret: string,
  api: BridgeApiClient,
  spawner: SessionSpawner,
  logger: BridgeLogger,
  signal: AbortSignal,
  backoffConfig: BackoffConfig = DEFAULT_BACKOFF,
  initialSessionId?: string,
  getAccessToken?: () => string | undefined | Promise<string | undefined>,
): Promise<void>
```

### 3.2 Capability gates (callable predicates)

```ts
// src/bridge/bridgeEnabled.ts
export function isBridgeEnabled(): boolean                             // :28
export async function isBridgeEnabledBlocking(): Promise<boolean>      // :50
export async function getBridgeDisabledReason(): Promise<string|null>  // :70
export function isEnvLessBridgeEnabled(): boolean                      // :126
export function isCseShimEnabled(): boolean                            // :141
export function checkBridgeMinVersion(): string | null                 // :160
export function getCcrAutoConnectDefault(): boolean                    // :185
export function isCcrMirrorEnabled(): boolean                          // :197
```

### 3.3 BridgeApiClient (v1 environments API)

`src/bridge/types.ts:133-176` — verbatim in §6.5. **Nine RPCs** (corrected Phase 9.6 from prior "Eight" miscount; heartbeat was added without count refresh) over `axios`: register, poll, ack, stop, deregister, sendPermissionResponseEvent, archive, reconnect, heartbeat.

### 3.4 Slash command surface

Defined in `src/commands/bridge/index.ts`:

```ts
{
  type: 'local-jsx',
  name: 'remote-control',
  aliases: ['rc'],
  description: 'Connect this terminal for remote-control sessions',
  argumentHint: '[name]',
  isEnabled, // = feature('BRIDGE_MODE') && isBridgeEnabled()
  immediate: true,
  load: () => import('./bridge.js'),
}
```

CLI flags exposed (`main.tsx:3866-3868`):

```
--remote-control [name]   Start an interactive session with Remote Control enabled (optionally named)
--rc [name]               Alias for --remote-control
```

Subcommand (`main.tsx:4322-4325`):

```
program.command('remote-control', { ... }).alias('rc')
  .description('Connect your local environment for remote-control sessions via claude.ai/code')
```

---

## 4. Data Model & State

### 4.1 Persistent state — `bridge-pointer.json`

Crash-recovery pointer (NOT a lockfile / NOT an IDE→CLI binding). Schema verbatim from `bridgePointer.ts:42-50`:

```ts
const BridgePointerSchema = lazySchema(() =>
  z.object({
    sessionId: z.string(),
    environmentId: z.string(),
    source: z.enum(['standalone', 'repl']),
  }),
)
```

Path: `getBridgePointerPath(dir) = join(getProjectsDir(), sanitizePath(dir), 'bridge-pointer.json')` (`bridgePointer.ts:52-54`).

TTL: `BRIDGE_POINTER_TTL_MS = 4 * 60 * 60 * 1000` (4 h, matches server `BRIDGE_LAST_POLL_TTL`) — `bridgePointer.ts:40`. Staleness anchored to file mtime, not embedded timestamp. Stale/invalid pointers are deleted (`bridgePointer.ts:106-110`). Worktree fanout cap `MAX_WORKTREE_FANOUT = 50` (`bridgePointer.ts:19`).

### 4.2 Persistent state — Inbound attachments

`~/.claude/uploads/{sessionId}/{8-char-prefix}-{safeName}` (`inboundAttachments.ts:60-62, 100-105`). `safeName = basename(name).replace(/[^a-zA-Z0-9._-]/g, '_')`.

### 4.3 Persistent state — Trusted-device token

Stored in `getSecureStorage()` (Keychain-backed, owned by spec 25) under field `trustedDeviceToken`. Cleared on logout / pre-`/login` re-enrollment (`trustedDevice.ts:72-87, 198`).

### 4.4 In-memory `BridgeConfig`

`types.ts:81-115` — verbatim §6.5.

### 4.5 In-memory state machines

**REPL bridge state** (`BridgeState`, exported from `replBridge.ts`):

```
idle → connecting → connected → attached
                        ↓           ↓
                    reconnecting   ended (archive → null handle)
                        ↓
                     failed (→ '/login' | "disabled by your organization's policy" | 'run `claude update` to upgrade')
```

`onStateChange?(state, detail)` fires on every transition (`initReplBridge.ts:84,151,162,224,394,418,460`). UI status states (`bridgeStatusUtil.ts:13-17`):

```ts
export type StatusState = 'idle' | 'attached' | 'titled' | 'reconnecting' | 'failed'
```

**Standalone-bridge per-session bookkeeping** (`bridgeMain.ts:163-194`):

```
activeSessions       Map<sessionId, SessionHandle>
sessionStartTimes    Map<sessionId, number>
sessionWorkIds       Map<sessionId, string>
sessionCompatIds     Map<sessionId, string>           // session_* form, frozen at spawn
sessionIngressTokens Map<sessionId, string>
sessionTimers        Map<sessionId, Timeout>
completedWorkIds     Set<workId>
sessionWorktrees     Map<sessionId, {...}>
timedOutSessions     Set<sessionId>
titledSessions       Set<compatSessionId>
capacityWake         CapacityWake
```

**`FlushGate<T>`** (`flushGate.ts:16-71`): three transitions — `start()` / `end()` returns drained items / `drop()` permanently / `deactivate()` (transport replaced).

**`BoundedUUIDSet`** (`bridgeMessaging.ts:429-461`): FIFO ring of UUIDs for echo / re-delivery dedup; `O(capacity)` memory.

**`createCapacityWake(outerSignal)`** (`capacityWake.ts:28-56`): merges outer-loop abort with per-sleep wake controller; `wake()` aborts current sleep AND re-arms the controller atomically.

**Token refresh scheduler** (`jwtUtils.ts:72-256`): per-`sessionId` map of `(timer, generation, failureCount)`; `nextGeneration(sessionId)` invalidates in-flight async work; `cancelAll()` clears.

### 4.6 Session-id tagging

Two co-existing tag prefixes for the same UUID:
- `cse_*` — infra/worker layer (work poll, `/v1/code/sessions/{id}/worker/*`).
- `session_*` — compat/v1 client layer (`/v1/sessions/{id}`, `/v1/sessions/{id}/archive`, `/v1/sessions/{id}/events`).

`sessionIdCompat.ts:38-42` `toCompatSessionId`, `:54-57` `toInfraSessionId`. Shim toggled by `setCseShimGate()` (default ON), gated by `tengu_bridge_repl_v2_cse_shim_enabled`.

---

## 5. Algorithm / Control Flow

### 5.1 REPL bridge bootstrap (`initReplBridge`)

```
1. setCseShimGate(isCseShimEnabled)
2. if !await isBridgeEnabledBlocking(): logBridgeSkip('not_enabled') → null
3. if !getBridgeAccessToken(): onStateChange('failed', '/login') → null
4. await waitForPolicyLimitsToLoad()
   if !isPolicyAllowed('allow_remote_control'):
       onStateChange('failed', "disabled by your organization's policy") → null
5. if !getBridgeTokenOverride():
   5a. cross-process backoff: if cfg.bridgeOauthDeadFailCount ≥ 3 AND
       getClaudeAIOAuthTokens()?.expiresAt === cfg.bridgeOauthDeadExpiresAt → null
   5b. await checkAndRefreshOAuthTokenIfNeeded()
   5c. tokens = getClaudeAIOAuthTokens()
       if tokens.expiresAt !== null AND tokens.expiresAt <= Date.now():
           save {bridgeOauthDeadExpiresAt, bridgeOauthDeadFailCount++}
           onStateChange('failed', '/login') → null
6. baseUrl = getBridgeBaseUrl()
7. derive title (precedence): initialName → getCurrentSessionTitle(getSessionId())
   → last meaningful user message via deriveTitle (filter meta/toolUseResult/
   isCompactSummary/origin.kind!=='human'/isSyntheticMessage, strip display tags,
   first sentence, collapse \s, slice 50 chars, append U+2026 if truncated)
   → "remote-control-" + generateShortWordSlug()
   onUserMessage = (text, bridgeSessionId) → boolean   // count-1 + count-3 derivations
8. initialHistoryCap = getFeatureValue('tengu_bridge_initial_history_cap', 200, 5min)
9. orgUUID = await getOrganizationUUID()
   if !orgUUID: onStateChange('failed', '/login') → null
10. if isEnvLessBridgeEnabled() AND !perpetual:
       if checkEnvLessBridgeMinVersion() returns error:
           onStateChange('failed', 'run `claude update` to upgrade') → null
       return await initEnvLessBridgeCore({...})           // v2 (remoteBridgeCore.ts)
11. if checkBridgeMinVersion() returns error:
       onStateChange('failed', 'run `claude update` to upgrade') → null
12. branch = await getBranch();  gitRepoUrl = await getRemoteUrl()
13. sessionIngressUrl = ANT && CLAUDE_BRIDGE_SESSION_INGRESS_URL ? env : baseUrl
14. workerType = (KAIROS && isAssistantMode()) ? 'claude_code_assistant' : 'claude_code'
15. return await initBridgeCore({...})                     // v1 env-based core
```

### 5.2 v1 env-based runtime (`initBridgeCore` summarized)

```
register POST /v1/environments/bridge → environmentId, environmentSecret
loop while !signal.aborted:
  work = await api.pollForWork(environmentId, environmentSecret, mergedSig,
                               reclaim_older_than_ms)
  if work === null: backoff per pollIntervalConfig (not_at_capacity)
  else if work.type === 'session':
    decoded = decodeWorkSecret(work.secret)               // version === 1, base64url
    sdkUrl  = buildSdkUrl(api_base_url, sessionId)         // ws(s)://.../session_ingress/ws/{id}
    transport = createV1ReplTransport(new HybridTransport(sdkUrl, ...))
    handle = spawner.spawn({sessionId, sdkUrl, accessToken, ...}, dir)
    api.acknowledgeWork(...)
    schedule jwt refresh via createTokenRefreshScheduler
    on transport.onClose(code):
       if code permanent → reconnectEnvironmentWithSession or env recreate (≤ 3 attempts)
    on session done → archiveSession(toCompatSessionId(sessionId))   // 1.5 s budget
                      capacityWake.wake()
```

Default backoff envelope (`bridgeMain.ts:72-79`): conn 2_000 / 120_000 / 600_000; gen 500 / 30_000 / 600_000. `MAX_ENVIRONMENT_RECREATIONS = 3` (`replBridge.ts:583, 1920`). `POLL_ERROR_MAX_DELAY_MS = 60_000` (`replBridge.ts:245`). Sleep-detection threshold = `2 * connCapMs` (`bridgeMain.ts:107-109`).

### 5.3 v2 env-less runtime (`initEnvLessBridgeCore`)

```
1. orgUUID + getEnvLessBridgeConfig() → cfg (Zod-validated; on parse fail → DEFAULT)
2. sessionId = await createCodeSession(baseUrl, accessToken, title, cfg.http_timeout_ms, tags)
   → POST /v1/code/sessions {title, bridge:{}, [tags]} → 200/201 { session: { id: cse_… } }
3. credentials = await fetchRemoteCredentials(sessionId, baseUrl, accessToken,
                  cfg.http_timeout_ms, getTrustedDeviceToken())
   → POST /v1/code/sessions/{sessionId}/bridge {} → 200 { worker_jwt, api_base_url,
                                                          expires_in, worker_epoch }
   adds X-Trusted-Device-Token if available
4. transport = await createV2ReplTransport({sessionUrl, ingressToken=worker_jwt,
                  sessionId, epoch=worker_epoch, heartbeat*, outboundOnly, getAuthToken})
   - SSE reads via SSETransport (URL = sessionUrl + '/worker/events/stream')
   - Writes via CCRClient (POST .../worker/events) over SerialBatchEventUploader
   - sse.setOnEvent → reportDelivery('received') AND reportDelivery('processed')
5. transport.connect() in parallel: void sse.connect() + void ccr.initialize(epoch).then(...)
   onConnectCb fires once ccr.initialize resolves
6. JWT refresh: scheduleFromExpiresIn(sessionId, expires_in)
   → on fire: fetch fresh credentials, transport.updateAccessToken
7. teardown:
   - signal close codes (CLIENT-SYNTHESIZED, never wire codes; see §9.3):
                         4090 = epoch superseded (409); 4091 = init failure;
                         4092 = SSE reconnect-budget exhausted
   - archive POST /v1/sessions/{compatId}/archive within
     teardown_archive_timeout_ms (default 1500 ms)
```

### 5.4 Title derivation (cosmetic, sent to claude.ai)

```
deriveTitle(raw):
  clean = stripDisplayTagsAllowEmpty(raw)
  firstSentence = /^(.*?[.!?])\s/.exec(clean)?.[1] ?? clean
  flat = firstSentence.replace(/\s+/g, ' ').trim()
  if !flat: return undefined
  TITLE_MAX_LEN = 50
  return flat.length > 50 ? flat.slice(0, 49) + '…' : flat
```

Two refresh points: count-1 (placeholder + Haiku upgrade) and count-3 (re-generate over `getMessagesAfterCompactBoundary(getMessages())`). `generateSessionTitle` uses `AbortSignal.timeout(15_000)`. `generateAndPatch` re-checks `getCurrentSessionTitle(getSessionId())` post-await so `/rename` always wins.

### 5.5 Inbound message routing (`handleIngressMessage`)

```
parsed = normalizeControlMessageKeys(jsonParse(data))
if isSDKControlResponse(parsed): onPermissionResponse(parsed); return
if isSDKControlRequest(parsed): onControlRequest(parsed); return
if !isSDKMessage(parsed): return
if uuid ∈ recentPostedUUIDs: log echo; return
if uuid ∈ recentInboundUUIDs: log redelivery; return
if parsed.type === 'user':
  recentInboundUUIDs.add(uuid)
  logEvent('tengu_bridge_message_received', { is_repl: true })
  void onInboundMessage(parsed)
else: log "Ignoring non-user inbound"
```

### 5.6 Server-initiated control_request fan-out (`handleServerControlRequest`)

Switch on `request.request.subtype`:

- `initialize` → success with `{ commands: [], output_style: 'normal', available_output_styles: ['normal'], models: [], account: {}, pid: process.pid }`.
- `set_model` → onSetModel(model); success.
- `set_max_thinking_tokens` → onSetMaxThinkingTokens(max); success.
- `set_permission_mode` → verdict from `onSetPermissionMode(mode) ?? { ok:false, error:'… not supported …' }`; success or error per verdict.
- `interrupt` → onInterrupt(); success.
- default → error `"REPL bridge does not handle control_request subtype: …"`.

When `outboundOnly` is true and subtype !== `'initialize'`: error reply with `OUTBOUND_ONLY_ERROR` (verbatim §6.6). All replies attach `session_id: sessionId`.

### 5.7 Trusted-device enrollment (post-`/login`)

```
if !await checkGate_CACHED_OR_BLOCKING('tengu_sessions_elevated_auth_enforcement'): return
if process.env.CLAUDE_TRUSTED_DEVICE_TOKEN: return
accessToken = getClaudeAIOAuthTokens()?.accessToken; if !: return
if isEssentialTrafficOnly(): return
POST /api/auth/trusted_devices { display_name: `Claude Code on ${hostname()} · ${process.platform}` }
   Authorization: Bearer ${accessToken}
   timeout 10_000
on 200/201 with body.device_token:
  secureStorage.read() → mutate { trustedDeviceToken } → secureStorage.update()
  readStoredToken.cache.clear()
```

Never blocks login flow (every error path logs and returns).

**`/login` clear-then-enroll race (`commands/login/login.tsx:40-42`)**: the `/login` flow calls `clearTrustedDeviceToken()` synchronously, then fires `void enrollTrustedDevice()` (unawaited). Between the cache clear and the new `device_token` landing in secure storage:
1. Bridge HTTP calls that read `getTrustedDeviceToken()` will see `undefined` and omit the `X-Trusted-Device-Token` header (correct: avoids sending the *previous* account's token).
2. The server gates new-device enrollment by `account_session.created_at < 10 min`, so any delay or failure pushes the call outside the eligibility window — enrollment for that login session is permanently lost (re-`/login` required). This is the reason `clearTrustedDeviceToken` is comment-anchored to "before re-enrollment on `/login`" (`trustedDevice.ts:67-72`) and to the 10-min server gate (`trustedDevice.ts:25-26, 95`).
3. Reimpls **must** preserve the clear-then-fire-and-forget order; awaiting `enrollTrustedDevice()` would hold the post-login UI on the network round-trip. Spec 25 owns the broader login lifecycle; cite this race when integrating.

### 5.8 OAuth retry in `withOAuthRetry` (v1 API client)

```
accessToken = resolveAuth() (throws BRIDGE_LOGIN_INSTRUCTION on missing)
response = await fn(accessToken)
if status !== 401: return response
if !deps.onAuth401: return response   // (*) no refresh handler → return original 401;
                                       //     caller's handleErrorStatus throws BridgeFatalError
refreshed = await deps.onAuth401(accessToken)
if refreshed:
  newToken = resolveAuth()
  retry = await fn(newToken)
  if retry.status !== 401: return retry
  // retry also got 401 → fall through, return original 401
// refresh failed OR retry==401 → return original 401
return response   // caller's handleErrorStatus throws BridgeFatalError
```

**Daemon-mode caveat** (refined Phase 9.5b → 9.6c): callers that omit `deps.onAuth401` (daemon callers, env-var-auth callers) take the `(*)` branch — `withOAuthRetry` will receive the 401 response unmodified at this layer; the fatal `BridgeFatalError(401)` is then thrown by `handleErrorStatus` downstream rather than at this control point. The behavioral outcome is the same (fatal + no retry) but the throw site is downstream — important for stack traces and any catch around this specific layer. There is no fallback refresh path for these callers. `withOAuthRetry` is internal to `createBridgeApiClient`; it is not part of the nine-RPC public `BridgeApiClient` table (§3.3, §6.5). Verified `bridgeApi.ts:106-139` (return-not-throw at `:117-120`; throw site `handleErrorStatus` per `:104` doc-comment). Earlier Phase 9.6 wording ("immediate `BridgeFatalError`") was overstated and is corrected here per Phase 9.5b adversarial review (cross-ref `PHASE9-FIXES-34.md` + `PHASE9-FIXES-34b.md`).

### 5.9 Status code → fatal-error mapping (`handleErrorStatus`)

| HTTP | BridgeFatalError | Message template |
|---:|---|---|
| 401 | yes | `"${context}: Authentication failed (401)…. ${BRIDGE_LOGIN_INSTRUCTION}"` |
| 403 | yes | If `isExpiredErrorType(errorType)` → expired-message; else `"${context}: Access denied (403)…. Check your organization permissions."` |
| 404 | yes | `detail ?? "${context}: Not found (404). Remote Control may not be available for this organization."` (server detail wins **only when non-null**; default fires only on null/undefined detail) |
| 410 | yes (`errorType ?? 'environment_expired'`) | `detail ?? 'Remote Control session has expired. Please restart with \`claude remote-control\` or /remote-control.'` (default expired-message fires only when detail null/undefined; `errorType` default `'environment_expired'` fires regardless of detail presence — verified `bridgeApi.ts:486-492`) |
| 429 | no (plain Error) | `"${context}: Rate limited (429). Polling too frequently."` |
| else | no (plain Error) | `"${context}: Failed with status ${status}…"` |

`isExpiredErrorType` matches substring `'expired'` OR substring `'lifetime'` on server-controlled `errorType` (`bridgeApi.ts:503-508`). **Brittleness**: this is a **substring match**, not a whitelist of known error types — any future server-side error-type containing the token `lifetime` (e.g. `'lifetime_extension_pending'`, `'lifetime_warning'`) will be classified as "expired" and trigger expired-session messaging at 403 (`bridgeApi.ts:473`) and at the 410-default branch path. Reimplementations should preserve the substring semantics for parity but flag this for future tightening (see BUGS-IN-SOURCE.md). `isSuppressible403` matches `'external_poll_sessions'` or `'environments:manage'` (`bridgeApi.ts:516-524`).

---

## 6. Verbatim Assets

### 6.1 Constants table

| Constant | Value | Location |
|---|---|---|
| `DEFAULT_SESSION_TIMEOUT_MS` | `24 * 60 * 60 * 1000` | `types.ts:2` |
| `BRIDGE_POINTER_TTL_MS` | `4 * 60 * 60 * 1000` | `bridgePointer.ts:40` |
| `MAX_WORKTREE_FANOUT` | `50` | `bridgePointer.ts:19` |
| `STATUS_UPDATE_INTERVAL_MS` | `1_000` | `bridgeMain.ts:82` |
| `SPAWN_SESSIONS_DEFAULT` | `32` | `bridgeMain.ts:83` |
| `TOOL_DISPLAY_EXPIRY_MS` | `30_000` | `bridgeStatusUtil.ts:20` |
| `SHIMMER_INTERVAL_MS` | `150` | `bridgeStatusUtil.ts:23` |
| `TITLE_MAX_LEN` | `50` | `initReplBridge.ts:547` |
| `MAX_CONSECUTIVE_INIT_FAILURES` | `3` | `hooks/useReplBridge.tsx:40` |
| `MAX_ENVIRONMENT_RECREATIONS` | `3` | `replBridge.ts:583, 1920` |
| `POLL_ERROR_MAX_DELAY_MS` | `60_000` | `replBridge.ts:245` |
| `DOWNLOAD_TIMEOUT_MS` | `30_000` | `inboundAttachments.ts:25` |
| `TOKEN_REFRESH_BUFFER_MS` | `5 * 60 * 1000` | `jwtUtils.ts:52` |
| `FALLBACK_REFRESH_INTERVAL_MS` | `30 * 60 * 1000` | `jwtUtils.ts:55` |
| `MAX_REFRESH_FAILURES` | `3` | `jwtUtils.ts:58` |
| `REFRESH_RETRY_DELAY_MS` | `60_000` | `jwtUtils.ts:61` |
| `DEBUG_MSG_LIMIT` | `2000` | `debugUtils.ts:9` |
| `REDACT_MIN_LENGTH` | `16` | `debugUtils.ts:24` |
| `BETA_HEADER` (v1) | `'environments-2025-11-01'` | `bridgeApi.ts:38` |
| `ANTHROPIC_VERSION` | `'2023-06-01'` | `codeSessionApi.ts:16`; `workSecret.ts:108` |
| `'anthropic-beta'` (compat sessions) | `'ccr-byoc-2025-07-29'` | `createSession.ts:140,213,290,353` |
| `SAFE_ID_PATTERN` | `/^[a-zA-Z0-9_-]+$/` | `bridgeApi.ts:41` |
| `EMPTY_POLL_LOG_INTERVAL` | `100` | `bridgeApi.ts:74` |
| Default `BackoffConfig` | conn 2_000/120_000/600_000; gen 500/30_000/600_000 | `bridgeMain.ts:72-79` |
| `POLL_INTERVAL_MS_NOT_AT_CAPACITY` | `2000` | `pollConfigDefaults.ts:13` |
| `POLL_INTERVAL_MS_AT_CAPACITY` | `600_000` | `pollConfigDefaults.ts:30` |
| `non_exclusive_heartbeat_interval_ms` default | `0` (disabled) | `pollConfigDefaults.ts:65` |
| `reclaim_older_than_ms` default | `5000` | `pollConfigDefaults.ts:76` |
| `session_keepalive_interval_v2_ms` default | `120_000` | `pollConfigDefaults.ts:81` |
| WS close codes — 4090 / 4091 / 4092 (client-synthesized; see §9.3) | epoch superseded / init fail / reconnect-budget | `replBridgeTransport.ts:220, 313, 365` |
| Most v1-API axios timeouts | `10_000` (poll, ack, stop, deregister, archive, reconnect, heartbeat, sendEvent); register `15_000` | `bridgeApi.ts:181, 220, 264, 311, 376, 407, 437` |
| `archiveBridgeSession` default | `10_000` (REPL teardown override `1500`) | `createSession.ts:303`; `initReplBridge.ts:514` |
| Default v2 heartbeat | `20_000 ms`, jitter `0` (transport defaults) | `replBridgeTransport.ts:138-140` |
| v2 heartbeat config schema | min 5_000, max 30_000, default 20_000; jitter 0..0.5, default 0.1 | `envLessBridgeConfig.ts:71-79` |
| v2 token-refresh buffer | min 30_000, max 1_800_000, default 300_000 | `envLessBridgeConfig.ts:86-91` |
| v2 connect timeout | min 5_000, max 60_000, default 15_000 | `envLessBridgeConfig.ts:103` |
| v2 archive teardown | min 500, max 2000, default 1500 | `envLessBridgeConfig.ts:94-99` |
| SSE read URL suffix | `'/worker/events/stream'` | `replBridgeTransport.ts:191` |
| `POST_MAX_RETRIES` (SSETransport write retries) | `10` | `cli/transports/SSETransport.ts:30` |
| `POST_BASE_DELAY_MS` (SSETransport retry base) | `500` | `cli/transports/SSETransport.ts:31` |
| `POST_MAX_DELAY_MS` (SSETransport retry cap) | `8000` | `cli/transports/SSETransport.ts:32` |
| SSETransport POST retry jitter | ±25% (computed at retry site) | `cli/transports/SSETransport.ts:648-649` |
| `buildSdkUrl` localhost / production | `ws + /v2/` / `wss + /v1/` | `workSecret.ts:42-48` |
| Trusted-device gate | `'tengu_sessions_elevated_auth_enforcement'` | `trustedDevice.ts:33` |
| Cross-process dead-token threshold | `failCount ≥ 3` | `initReplBridge.ts:180` |

### 6.2 Bridge message envelopes

**`WorkSecret` (v1, version 1)** — `types.ts:33-51`:

```ts
export type WorkSecret = {
  version: number
  session_ingress_token: string
  api_base_url: string
  sources: Array<{
    type: string
    git_info?: { type: string; repo: string; ref?: string; token?: string }
  }>
  auth: Array<{ type: string; token: string }>
  claude_code_args?: Record<string, string> | null
  mcp_config?: unknown | null
  environment_variables?: Record<string, string> | null
  use_code_sessions?: boolean
}
```

**`WorkResponse`** — `types.ts:23-31`:

```ts
export type WorkResponse = {
  id: string
  type: 'work'
  environment_id: string
  state: string
  data: WorkData
  secret: string // base64url-encoded JSON
  created_at: string
}
```

**`WorkData`** — `types.ts:18-21`:

```ts
export type WorkData = {
  type: 'session' | 'healthcheck'
  id: string
}
```

**`PermissionResponseEvent` (events API payload)** — `types.ts:124-131`:

```ts
export type PermissionResponseEvent = {
  type: 'control_response'
  response: {
    subtype: 'success'
    request_id: string
    response: Record<string, unknown>
  }
}
```

**`SessionEvent` (POST /v1/sessions)** — `createSession.ts:20-23`:

```ts
type SessionEvent = {
  type: 'event'
  data: SDKMessage
}
```

**v2 `RemoteCredentials`** — `codeSessionApi.ts:86-91`:

```ts
export type RemoteCredentials = {
  worker_jwt: string
  api_base_url: string
  expires_in: number
  worker_epoch: number
}
```

**`createCodeSession` request body** — `codeSessionApi.ts:39-41`:

```ts
{ title, bridge: {}, ...(tags?.length ? { tags } : {}) }
```

(`bridge: {}` is the positive oneof signal — omitting it now 400s.)

**`createBridgeSession` (v1 compat) request body** — `createSession.ts:125-136`:

```ts
{
  ...(title !== undefined && { title }),
  events,
  session_context: {
    sources: gitSource ? [gitSource] : [],
    outcomes: gitOutcome ? [gitOutcome] : [],
    model: getMainLoopModel(),
  },
  environment_id: environmentId,
  source: 'remote-control',
  ...(permissionMode && { permission_mode: permissionMode }),
}
```

**Bridge attachment schema** — `inboundAttachments.ts:31-37`:

```ts
const attachmentSchema = lazySchema(() =>
  z.object({
    file_uuid: z.string(),
    file_name: z.string(),
  }),
)
```

**`registerBridgeEnvironment` request body** — `bridgeApi.ts:156-178`:

```ts
{
  machine_name: config.machineName,
  directory: config.dir,
  branch: config.branch,
  git_repo_url: config.gitRepoUrl,
  max_sessions: config.maxSessions,
  metadata: { worker_type: config.workerType },
  ...(config.reuseEnvironmentId && { environment_id: config.reuseEnvironmentId }),
}
```

### 6.3 URL templates

| Endpoint | Template |
|---|---|
| Register env (v1) | `POST {baseUrl}/v1/environments/bridge` |
| Poll work | `GET {baseUrl}/v1/environments/{environmentId}/work/poll?reclaim_older_than_ms={n}` |
| Ack work | `POST {baseUrl}/v1/environments/{envId}/work/{workId}/ack` |
| Stop work | `POST {baseUrl}/v1/environments/{envId}/work/{workId}/stop  body:{force}` |
| Heartbeat | `POST {baseUrl}/v1/environments/{envId}/work/{workId}/heartbeat` |
| Deregister env | `DELETE {baseUrl}/v1/environments/bridge/{envId}` |
| Reconnect session | `POST {baseUrl}/v1/environments/{envId}/bridge/reconnect  body:{session_id}` |
| Send event | `POST {baseUrl}/v1/sessions/{sessionId}/events  body:{events:[…]}` |
| Compat session create | `POST {baseUrl}/v1/sessions  body: see §6.2` |
| Compat session get | `GET {baseUrl}/v1/sessions/{sessionId}` |
| Compat session archive | `POST {baseUrl}/v1/sessions/{sessionId}/archive` (idempotent — 409 = already archived) |
| Compat session rename | `PATCH {baseUrl}/v1/sessions/{compatId}  body:{title}` |
| v2 create code-session | `POST {baseUrl}/v1/code/sessions  body: see §6.2` |
| v2 fetch /bridge | `POST {baseUrl}/v1/code/sessions/{sessionId}/bridge  body:{}` |
| v2 worker register | `POST {sessionUrl}/worker/register` |
| v2 worker SSE read | `{sessionUrl}/worker/events/stream` |
| v2 worker write | `POST {sessionUrl}/worker/events` (via CCRClient) |
| v2 delivery ack | `POST {sessionUrl}/worker/events/{eventId}/delivery` |
| Trusted-device enroll | `POST {baseUrl}/api/auth/trusted_devices` |
| Inbound attachment fetch | `GET {baseUrl}/api/oauth/files/{file_uuid}/content` |

`buildSdkUrl(apiBaseUrl, sessionId)` (`workSecret.ts:41-48`):

```
isLocalhost = apiBaseUrl.includes('localhost') || apiBaseUrl.includes('127.0.0.1')
protocol = isLocalhost ? 'ws' : 'wss'
version  = isLocalhost ? 'v2' : 'v1'
host     = apiBaseUrl.replace(/^https?:\/\//, '').replace(/\/+$/, '')
return `${protocol}://${host}/${version}/session_ingress/ws/${sessionId}`
```

`buildCCRv2SdkUrl` (`workSecret.ts:81-87`):

```
return `${apiBaseUrl.replace(/\/+$/, '')}/v1/code/sessions/${sessionId}`
```

`buildBridgeConnectUrl` (`bridgeStatusUtil.ts:39-45`):

```
return `${getClaudeAiBaseUrl(undefined, ingressUrl)}/code?bridge=${environmentId}`
```

### 6.4 `BRIDGE_SAFE_COMMANDS` allowlist (verbatim, `commands.ts:651-660`)

```ts
export const BRIDGE_SAFE_COMMANDS: Set<Command> = new Set(
  [
    compact, // Shrink context — useful mid-session from a phone
    clear, // Wipe transcript
    cost, // Show session cost
    summary, // Summarize conversation
    releaseNotes, // Show changelog
    files, // List tracked files
  ].filter((c): c is Command => c !== null),
)
```

`isBridgeSafeCommand(cmd)` (`commands.ts:672-676`): `local-jsx` → false; `prompt` → true (skills expand to text); else `BRIDGE_SAFE_COMMANDS.has(cmd)`.

### 6.5 `BridgeApiClient` (verbatim, `types.ts:133-176`)

```ts
export type BridgeApiClient = {
  registerBridgeEnvironment(config: BridgeConfig): Promise<{
    environment_id: string
    environment_secret: string
  }>
  pollForWork(
    environmentId: string,
    environmentSecret: string,
    signal?: AbortSignal,
    reclaimOlderThanMs?: number,
  ): Promise<WorkResponse | null>
  acknowledgeWork(
    environmentId: string,
    workId: string,
    sessionToken: string,
  ): Promise<void>
  stopWork(environmentId: string, workId: string, force: boolean): Promise<void>
  deregisterEnvironment(environmentId: string): Promise<void>
  sendPermissionResponseEvent(
    sessionId: string,
    event: PermissionResponseEvent,
    sessionToken: string,
  ): Promise<void>
  archiveSession(sessionId: string): Promise<void>
  reconnectSession(environmentId: string, sessionId: string): Promise<void>
  heartbeatWork(
    environmentId: string,
    workId: string,
    sessionToken: string,
  ): Promise<{ lease_extended: boolean; state: string }>
}
```

`BridgeConfig` (verbatim, `types.ts:81-115`):

```ts
export type BridgeConfig = {
  dir: string
  machineName: string
  branch: string
  gitRepoUrl: string | null
  maxSessions: number
  spawnMode: SpawnMode
  verbose: boolean
  sandbox: boolean
  bridgeId: string
  workerType: string
  environmentId: string
  reuseEnvironmentId?: string
  apiBaseUrl: string
  sessionIngressUrl: string
  debugFile?: string
  sessionTimeoutMs?: number
}
```

`SpawnMode` (`types.ts:69`): `'single-session' | 'worktree' | 'same-dir'`.
`BridgeWorkerType` (`types.ts:79`): `'claude_code' | 'claude_code_assistant'` (backend treats as opaque string).
`SessionDoneStatus` (`types.ts:53`): `'completed' | 'failed' | 'interrupted'`.

### 6.6 User-facing strings (verbatim)

| String | Source |
|---|---|
| `'Remote Control is only available with claude.ai subscriptions. Please use \`/login\` to sign in with your claude.ai account.'` (`BRIDGE_LOGIN_INSTRUCTION`) | `types.ts:5-6` |
| `'Error: You must be logged in to use Remote Control.\n\n' + BRIDGE_LOGIN_INSTRUCTION` (`BRIDGE_LOGIN_ERROR`) | `types.ts:9-11` |
| `'Remote Control disconnected.'` (`REMOTE_CONTROL_DISCONNECTED_MSG`) | `types.ts:14` |
| `'Remote Control requires a claude.ai subscription. Run \`claude auth login\` to sign in with your claude.ai account.'` | `bridgeEnabled.ts:73` |
| `'Remote Control requires a full-scope login token. Long-lived tokens (from \`claude setup-token\` or CLAUDE_CODE_OAUTH_TOKEN) are limited to inference-only for security reasons. Run \`claude auth login\` to use Remote Control.'` | `bridgeEnabled.ts:76` |
| `'Unable to determine your organization for Remote Control eligibility. Run \`claude auth login\` to refresh your account information.'` | `bridgeEnabled.ts:79` |
| `'Remote Control is not yet enabled for your account.'` | `bridgeEnabled.ts:82` |
| `'Remote Control is not available in this build.'` | `bridgeEnabled.ts:86` |
| `` `Your version of Claude Code (${MACRO.VERSION}) is too old for Remote Control.\nVersion ${config.minVersion} or higher is required. Run \`claude update\` to update.` `` | `bridgeEnabled.ts:169`; v2 analog `envLessBridgeConfig.ts:150` |
| `'This session is outbound-only. Enable Remote Control locally to allow inbound control.'` (`OUTBOUND_ONLY_ERROR`) | `bridgeMessaging.ts:231-232` |
| `'Remote Control failed' / 'Remote Control reconnecting' / 'Remote Control active' / 'Remote Control connecting…'` | `bridgeStatusUtil.ts:114-120, 135-140` |
| `` `Code everywhere with the Claude app or ${url}` `` (idle footer) | `bridgeStatusUtil.ts:144` |
| `` `Continue coding in the Claude app or ${url}` `` (active footer) | `bridgeStatusUtil.ts:149` |
| `'Something went wrong, please try again'` (`FAILED_FOOTER_TEXT`) | `bridgeStatusUtil.ts:154` |
| `'Authentication failed (401)…'` / `'Access denied (403)…'` / `'Not found (404)…'` / `'Remote Control session has expired. Please restart with \`claude remote-control\` or /remote-control.'` / `'Rate limited (429). Polling too frequently.'` | `bridgeApi.ts:467-499` |
| `'Connect your local environment for remote-control sessions via claude.ai/code'` (Commander description) | `main.tsx:4322-4325` |
| `'Start an interactive session with Remote Control enabled (optionally named)'` | `main.tsx:3867` |
| `'Connect this terminal for remote-control sessions'` | `commands/bridge/index.ts:16` |
| `` `/remote-control is active. Code in CLI or at ${remoteSessionUrl}` `` | `main.tsx:3471` |
| `` `Claude Code on ${hostname()} · ${process.platform}` `` (trusted-device display name) | `trustedDevice.ts:150` |

### 6.7 IDE allowlist (cross-cited from spec 16, verbatim)

```ts
// services/mcp/client.ts:568
ALLOWED_IDE_TOOLS = ['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']
```

(Filters `mcp__ide__*` to those two. Producer side is `vscodeSdkMcp.ts`, owned by spec 23. The `bridge/` code does **not** use this allowlist.)

### 6.8 Critical regexes

| Regex | Purpose | Source |
|---|---|---|
| `/^[a-zA-Z0-9_-]+$/` | Validate server-provided IDs before URL interpolation | `bridgeApi.ts:41` |
| `/^(.*?[.!?])\s/` | First-sentence capture for session-title derivation (capture group avoids YARR JIT lookbehind cost) | `initReplBridge.ts:562` |
| `/\s+/` | Collapse whitespace for title flatten | `initReplBridge.ts:564` |
| `/[^a-zA-Z0-9._-]/g` (filename) and `/[^a-zA-Z0-9_-]/g` (prefix) | Sanitize attachment filename / UUID prefix | `inboundAttachments.ts:56, 103` |
| `/^https?:\/\//` and `/\/+$/` | Strip protocol / trailing slashes for SDK URL builder | `workSecret.ts:46-47, 86, 191` |
| `` `"(${SECRET_FIELD_NAMES.join('|')})"\\s*:\\s*"([^"]*)"` `` | Secret redaction in debug logs | `debugUtils.ts:19-22` |

### 6.9 Permission decision-tree pseudocode (bridge-side)

```
inbound control_request subtype === 'can_use_tool':
  forward to BridgePermissionCallbacks.sendRequest(requestId, toolName, input,
                                                    toolUseId, description,
                                                    permissionSuggestions, blockedPath)
  await onResponse(requestId) →
    BridgePermissionResponse = { behavior: 'allow' | 'deny',
                                  updatedInput?, updatedPermissions?, message? }
  api.sendPermissionResponseEvent(sessionId, {
    type: 'control_response',
    response: { subtype: 'success', request_id, response: <BridgePermissionResponse> },
  }, sessionToken)
```

`isBridgePermissionResponse(value)` predicate (`bridgePermissionCallbacks.ts:32-40`): rejects unless `behavior === 'allow' || 'deny'`.

---

## 7. Side Effects & I/O

### 7.1 Filesystem

- **Read/write**: `getProjectsDir()/sanitizePath(dir)/bridge-pointer.json` — write/refresh/clear (`bridgePointer.ts`).
- **Write**: `~/.claude/uploads/{sessionId}/{prefix}-{safeName}` for inbound attachments (`inboundAttachments.ts:104-109`).
- **Read**: `git worktree list` portable shell-out for pointer fanout (`getWorktreePathsPortable`, 5 s timeout, returns `[]` on error).
- **Spawn**: `getSecureStorage().read()` may spawn `security` (~40 ms on macOS) → memoized (`trustedDevice.ts:39-52`).
- **Spawn**: standalone bridge spawns child `claude` processes via `SessionSpawner` (`bridgeMain.ts:127-139`); arg layout depends on `isInBundledMode()` — bundled binary takes args directly, npm install (node + cli.js) prepends `process.argv[1]` else node misinterprets `--sdk-url` (gh-28334).

### 7.2 Network

All authenticated bearer auth + `Content-Type: application/json` + `anthropic-version: 2023-06-01`.

- v1 endpoints add `anthropic-beta: environments-2025-11-01` and `x-environment-runner-version` (`bridgeApi.ts:38, 76-89`).
- v1 compat sessions (`/v1/sessions`) add `anthropic-beta: ccr-byoc-2025-07-29` and `x-organization-uuid` (`createSession.ts:140-142, 213-215, 290-292, 353-355`).
- v2 worker endpoints validate `worker_jwt` via Authorization header.
- All bridge-API calls send `X-Trusted-Device-Token` when keychain has one and the gate is on (`bridgeApi.ts:84-87`; `codeSessionApi.ts:103`).

### 7.3 Environment variables consumed

| Var | Use |
|---|---|
| `USER_TYPE` | `=== 'ant'` enables ANT-only branches |
| `CLAUDE_BRIDGE_OAUTH_TOKEN` | ANT dev override for OAuth token |
| `CLAUDE_BRIDGE_BASE_URL` | ANT dev override for bridge API base URL |
| `CLAUDE_BRIDGE_SESSION_INGRESS_URL` | ANT dev override for session-ingress (WS) URL |
| `CLAUDE_TRUSTED_DEVICE_TOKEN` | Override / canary for trusted-device token (skips enrollment) |
| `CLAUDE_CODE_CCR_MIRROR` | Truthy to opt local sessions into CCR mirror mode |
| `CLAUDE_CODE_USE_CCR_V2` | Force v2 transport in env-based bridges |
| `CLAUDE_CODE_OAUTH_TOKEN` | Setup token; lacks `user:profile` scope ⇒ fails `getBridgeDisabledReason` |
| `CLAUDE_CODE_SESSION_ACCESS_TOKEN` | Process-wide v2 ingress token (legacy single-session) |

### 7.4 External binaries

- `git` (via `getBranch`/`getRemoteUrl`/`getDefaultBranch`/`getWorktreePathsPortable`).
- macOS Keychain via `secureStorage` (spec 25).
- Spawned child `claude` binary in standalone mode.

### 7.5 Trust boundaries

- Server-provided IDs must pass `validateBridgeId` regex `/^[a-zA-Z0-9_-]+$/` before URL path interpolation (`bridgeApi.ts:48-53`).
- Filename from network (`file_name`) is sanitized via `basename()` + `replace(/[^a-zA-Z0-9._-]/g,'_')` (`inboundAttachments.ts:55-58`).
- Inbound `image` blocks with missing/camelCase `media_type` are normalized via `detectImageFormatFromBase64` (`inboundMessages.ts:43-73`).
- Display tags (`<ide_opened_file>`, `<session-start-hook>`, etc.) stripped from inbound text before title derivation.

---

## 8. Feature Flags & Variants

### 8.1 `feature('BRIDGE_MODE')` ON vs OFF

OFF: `commands.ts:73` returns `null` → no `/remote-control` slash command; `main.tsx:3866-3868` skips `--remote-control`/`--rc` options; `main.tsx:4322` skips `program.command('remote-control')`. All `bridgeEnabled.ts` predicates short-circuit to `false`. `getBridgeDisabledReason` returns `'Remote Control is not available in this build.'`.

ON: subsystem activates per `tengu_ccr_bridge` GrowthBook gate, OAuth subscriber check, profile-scope check, org-uuid check.

### 8.2 `feature('CCR_AUTO_CONNECT')` ON vs OFF (ANT only)

OFF (default): `getCcrAutoConnectDefault()` → `false`.
ON: returns `tengu_cobalt_harbor` GB value, default `false`. User's explicit `remoteControlAtStartup` setting always wins (`bridgeEnabled.ts:178-180` comment).

### 8.3 `feature('CCR_MIRROR')` ON vs OFF

OFF: `isCcrMirrorEnabled()` → `false`.
ON: `process.env.CLAUDE_CODE_CCR_MIRROR` truthy OR `tengu_ccr_mirror` GB value true.

### 8.4 `feature('KAIROS')` ON vs OFF

OFF: `workerType = 'claude_code'`.
ON: lazy-require `assistant/index.js` → if `isAssistantMode()` → `'claude_code_assistant'` (`initReplBridge.ts:476-485`). KAIROS-specific `perpetual` flag forces v1 path (`initReplBridge.ts:407-410`) — see spec 32.

### 8.5 `feature('DAEMON') && feature('BRIDGE_MODE')`

Both ON → `commands.ts:77` loads `remoteControlServer` command (deferred to spec 33 / 35).

### 8.6 `tengu_bridge_repl_v2` (env-less)

OFF: `initReplBridge` uses env-based path (`initBridgeCore`).
ON (and `!perpetual`): uses `initEnvLessBridgeCore` (`remoteBridgeCore.ts`) — bypasses environments API. Daemon and `print.ts` (`enableRemoteControl`) intentionally stay on env-based regardless.

### 8.7 `USER_TYPE === 'ant'` deltas

- Enables `CLAUDE_BRIDGE_*` env-var overrides and skips OAuth refresh chain (`initReplBridge.ts:168`).
- `/bridge-kick` slash command (`bridgeDebug.ts`).
- Debug-log path display in standalone bridge (`types.ts:236-237`).
- `feature('CCR_AUTO_CONNECT')` only wires up under ANT builds.

---

## 9. Error Handling & Edge Cases

### 9.1 Init failure short-circuits

| Path | Reason logged | `onStateChange` detail |
|---|---|---|
| Bridge gate off | `'not_enabled'` | (no state change; returns null) |
| No OAuth | `'no_oauth'` | `'/login'` |
| Org policy denied | `'policy_denied'` | `"disabled by your organization's policy"` |
| Cross-process dead-token (≥3) | n/a | (silent — no event) |
| Token expired post-refresh | `'oauth_expired_unrefreshable'` | `'/login'` |
| No org UUID | `'no_org_uuid'` | `'/login'` |
| v1 version too old | `'version_too_old'` | `` 'run `claude update` to upgrade' `` |
| v2 version too old | `'version_too_old'` (v2=true) | `` 'run `claude update` to upgrade' `` |

### 9.2 HTTP failure handling

`handleErrorStatus` table — see §5.9. `BridgeFatalError` triggers teardown; other errors retry with backoff. `409` on archive is idempotent OK.

### 9.3 Transport close codes

- `1002 / 1006` (WS protocol / abnormal): real WebSocket close codes from the server / network layer; zombie-poll guard (`bridgeDebug.ts:11`).
- `4090`, `4091`, `4092`: **client-synthesized**, not wire-protocol close codes. They are produced by `replBridgeTransport.ts` itself when invoking the local `onClose` callback; they never traverse the network and **will never appear** in network traces / server logs / packet captures. Telemetry consumers that grep for these values must read CLI-side logs only.
  - `4090`: epoch superseded — synthesized in `onEpochMismatch` (`replBridgeTransport.ts:220 → onCloseCb?.(4090)`); close + throw to unwind, poll loop picks fresh dispatch.
  - `4091`: CCR `initialize` failed — synthesized in the init-rejection catch (`replBridgeTransport.ts:365 → onCloseCb?.(4091)`).
  - `4092`: SSE reconnect-budget exhausted — synthesized when `sse.setOnClose` fires with `code === undefined` (CCR's reconnect budget exhausted, mapped locally: `replBridgeTransport.ts:313 → cb(code ?? 4092)`).

### 9.4 OAuth expiry resilience

- `checkAndRefreshOAuthTokenIfNeeded` proactively refreshes ahead of 5 min buffer.
- 401 → `withOAuthRetry` → single retry; second 401 → `BridgeFatalError` (only when `deps.onAuth401` is provided; daemon-mode callers without an `onAuth401` refresh handler will receive the 401 response unmodified at this layer — the fatal `BridgeFatalError(401)` is then thrown by `handleErrorStatus` downstream rather than at this control point. Behavioral outcome is the same (fatal + no retry) but the throw site is downstream — important for stack traces and any catch around this specific layer. See §5.8; corrected Phase 9.5b → 9.6c from earlier "immediate" wording).
- Cross-process dead-token write `bridgeOauthDeadExpiresAt`/`bridgeOauthDeadFailCount` capped at 3 to avoid infinite write storm.
- **Token-refresh chain has no upper bound on the success path.** `MAX_REFRESH_FAILURES = 3` (`jwtUtils.ts:58`) caps only the *failure* path; on every successful refresh, `doRefresh` schedules a fresh follow-up timer at `FALLBACK_REFRESH_INTERVAL_MS = 30 min` (`jwtUtils.ts:217-226`). The chain runs forever for the lifetime of the scheduler. Permanent termination must come from explicit `cancelAll()` invocation. Verified call sites:
  - v2 (`remoteBridgeCore.ts:667`) — `refresh.cancelAll()` on bridge-core teardown.
  - v1 standalone (`bridgeMain.ts:1470`) — `tokenRefresh?.cancelAll()` on per-session cleanup.
  - v2 reuse-guard (`remoteBridgeCore.ts:503`) — early-return comment explicitly avoids `wire/connect/schedule` because they would re-arm timers after `cancelAll()`.
  - **No `cancelAll()` in v1 REPL bridge teardown path.** If `replBridge.ts` tears down without invoking the refresh scheduler's `cancelAll`, scheduled timers may keep firing against a dead transport (`onRefresh` becomes a no-op, but the 30-min follow-up keeps re-scheduling). Reimpls must wire `cancelAll()` into every transport-tear-down site or accept the leak.

### 9.5 Session ID confusion

`sessionIdCompat.ts` retag handles every server boundary that demands `cse_*` vs `session_*`. `sameSessionId(a,b)` (`workSecret.ts:62-73`) ignores prefix and stagings to compare bodies — guards a `≥4`-char minimum-suffix match.

### 9.6 Echo / re-delivery dedup

Two `BoundedUUIDSet` rings — `recentPostedUUIDs` (echoes of our own writes) and `recentInboundUUIDs` (server replays after seq-num cursor loss). Default `uuid_dedup_buffer_size = 2000` (`envLessBridgeConfig.ts:50`).

### 9.7 Initial-flush atomicity

`FlushGate<T>` queues new messages while the historical-flush HTTP POST is in flight. On permanent close → `drop()` discards queued items. On transport replacement → `deactivate()` (new transport will drain).

### 9.8 v2 epoch superseded mid-write

`onEpochMismatch` in `CCRClient` calls `ccr.close()` + `sse.close()` + fires `onClose(4090)` then **throws** to unwind the request (`replBridgeTransport.ts:208-231`). Cleanup is wrapped in `try`/`catch` so the throw always executes even if close throws.

### 9.9 Inbound-attachment failures

`resolveOne` returns `undefined` on any failure (no token, network, non-200, fs error); the message still reaches Claude without the `@path` prefix (`inboundAttachments.ts:67-117`). The `getOauthConfig()` call is intentionally inside the `try` so a bad `CLAUDE_CODE_CUSTOM_OAUTH_URL` degrades to "no @path" rather than crashing print.ts's reader loop.

### 9.10 Concurrent-process ownership

`updateSessionBridgeId(getSelfBridgeCompatId() ?? null)` published to local session record so peers can dedup the same bridge out of their list (`replBridgeHandle.ts:21-23`).

### 9.11 v2 SSETransport POST retry contract

v2 outbound writes (`POST /worker/events`) flow through `SSETransport`'s in-process retry loop (`cli/transports/SSETransport.ts:591-655`):

- `POST_MAX_RETRIES = 10` (`SSETransport.ts:30`).
- `POST_BASE_DELAY_MS = 500` (`SSETransport.ts:31`).
- `POST_MAX_DELAY_MS = 8000` (`SSETransport.ts:32`).
- Backoff = `min(POST_BASE_DELAY_MS * 2^(attempt-1), POST_MAX_DELAY_MS)` with **±25 % jitter** applied at the retry site (`SSETransport.ts:648-649`).
- After all 10 attempts the loop logs `"SSETransport: POST failed after 10 attempts, continuing"` and **drops the event** (does not throw, does not surface to `BridgeFatalError`). The bridge keeps running; outbound delivery is best-effort. Reimpls must preserve the cap **and** the silent-drop semantic — adding a fatal-throw on exhaustion will break long-running bridges across server-side blips.

---

## 10. Telemetry & Observability

Analytics events (`logEvent`, `logEventAsync`):

| Event | When |
|---|---|
| `tengu_bridge_repl_skipped` | `logBridgeSkip(reason, debugMsg, v2?)` (`debugUtils.ts:128-141`) |
| `tengu_bridge_token_refreshed` | After `onRefresh` (`jwtUtils.ts:214`) |
| `tengu_bridge_message_received` (`{is_repl: true}`) | Each user-typed inbound (`bridgeMessaging.ts:193-195`) |
| `tengu_bridge_repl_connect_timeout` | v2 onConnect deadline missed (default 15_000 ms) — `envLessBridgeConfig.ts:30-33` |

Diagnostic-no-PII logs:

| Log key | When |
|---|---|
| `bridge_token_refresh_no_oauth` | `getAccessToken` returned `undefined` during refresh (`jwtUtils.ts:192`) |

Debug-log prefixes (grep targets): `[bridge:repl]`, `[bridge:api]`, `[bridge:debug]`, `[bridge:pointer]`, `[bridge:inbound-attach]`, `[code-session]`, `[trusted-device]`.

Debug body redaction: fields `session_ingress_token`, `environment_secret`, `access_token`, `secret`, `token` redacted to `"${first8}...${last4}"` (or `"[REDACTED]"` if `< 16` chars). Truncation at `DEBUG_MSG_LIMIT = 2000` (`debugUtils.ts:11-53`).

---

## 11. Reimplementation Checklist

A bit-exact rebuild MUST preserve:

1. **`feature('BRIDGE_MODE')` positive ternary** at every gate — negative pattern leaves inline GB literals in external builds (`bridgeEnabled.ts:29-31` comment).
2. **`tengu_ccr_bridge` gate name** and the `isClaudeAISubscriber` + `hasProfileScope` + `oauthAccount.organizationUuid` cascade in `getBridgeDisabledReason`.
3. **`USER_TYPE === 'ant'`-gated env vars** (`CLAUDE_BRIDGE_OAUTH_TOKEN`, `CLAUDE_BRIDGE_BASE_URL`, `CLAUDE_BRIDGE_SESSION_INGRESS_URL`).
4. **`bridge: {}` positive oneof signal** in `POST /v1/code/sessions` body.
5. `BRIDGE_POINTER_TTL_MS = 4 h` and pointer staleness driven by **mtime**, not embedded timestamp.
6. **`MAX_WORKTREE_FANOUT = 50`** for `--continue` pointer search.
7. v1 `BETA_HEADER = 'environments-2025-11-01'` and compat `'ccr-byoc-2025-07-29'`.
8. v1 vs v2 URL builders (localhost ws/v2 vs prod wss/v1).
9. `cse_*` ↔ `session_*` retag at every compat-API site, gated by `tengu_bridge_repl_v2_cse_shim_enabled`.
10. `BRIDGE_SAFE_COMMANDS` set verbatim; `local-jsx` rejected, `prompt` accepted, `local` only if in set.
11. `OUTBOUND_ONLY_ERROR` reply for non-`initialize` control_requests when `outboundOnly`.
12. `initialize` control_response shape: `{ commands: [], output_style: 'normal', available_output_styles: ['normal'], models: [], account: {}, pid: process.pid }`.
13. `'Connect this terminal for remote-control sessions'` description and `'rc'` alias.
14. Cross-process dead-token backoff: `bridgeOauthDeadExpiresAt` / `bridgeOauthDeadFailCount` stop at ≥ 3.
15. WS close codes 4090 / 4091 / 4092 with their semantics — **client-synthesized in `replBridgeTransport.ts`**, never sent or received over the wire (§9.3).
16. `validateBridgeId(/^[a-zA-Z0-9_-]+$/)` before every URL-segment interpolation.
17. `BoundedUUIDSet` ring eviction — FIFO at write index, capacity-bounded.
18. v2 archive teardown timeout floor of 1500 ms (gracefulShutdown 2 s race).
19. `secureStorage` used for trusted-device token (Keychain, spec 25).
20. Title slug fallback `"remote-control-" + generateShortWordSlug()` with U+2026 truncation at 50 chars.
21. v2 SSE `setOnEvent` immediately reports both `'received'` and `'processed'` (workaround for in-process listeners).
22. v2 default heartbeat `20_000 ms` (3× margin under server 60 s TTL); jitter 0..0.5.
23. `onUserMessage` count-1 + count-3 title refresh, with `genSeq`/`lastBridgeSessionId` guards against late-resolving Haiku responses.
24. `setReplBridgeHandle(handle)` updates `concurrentSessions` to publish bridge ID for peer dedup.
25. Standalone-bridge spawn-script-arg quirk: bundled binary → no prefix; npm node → prepend `process.argv[1]`.

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. Most items are sampling-level deep dives that are not blocking.

1. ~~**`replBridge.ts:1-2406` reconnect state machine deep dive**~~ — **NOTE Phase 9.7**: cited line ranges anchor key behaviors; the full transition graph is sampling-only. No defect identified. Future revise pass can inline the full state machine if needed.
2. ~~**`bridgeMain.ts:200-2999` multi-session spawn lifecycle**~~ — **NOTE Phase 9.7**: spec 35 (remote-server) §5 documents the multi-session spawn lifecycle that overlaps with this surface; per-spec ownership boundary preserved. Coverage matrix (`PHASE10-COVERAGE.md`) confirms no double-claim.
3. ~~**`remoteBridgeCore.ts:1-1008` v2 core internal helpers**~~ — **NOTE Phase 9.7**: §5.3 summary preserved; per-line citations deferred. Sampling-level coverage acceptable.
4. ~~**`bridgeUI.ts:1-530` full UI tree**~~ — **RESOLVED Phase 9.7**: spec 37 (Ink UI) §3 owns the React/Ink rendering surface; only shimmer constants are referenced here, correctly bounded.
5. ~~**`sessionRunner.ts:1-550` `SessionSpawner` shape**~~ — **RESOLVED Phase 9.7**: spec 35 §5 documents the spawner shape and CCR v2 env-var injection; this spec correctly defers.
6. ~~**Dispatcher's "lock file / discovery mechanism" item**~~ — **RESOLVED Phase 9.7**: confirmed no IDE→CLI lockfile at bridge layer. `bridge-pointer.json` is for process-crash recovery; IDE-side MCP lockfile (`~/.claude/ide/`) is spec 23-owned. Documentation-cross-reference-mismatch closed.
7. ~~**`tengu_bridge_repl_v2_config.min_version` hard/soft floor**~~ — **DEFERRED**: GrowthBook server-side config; per spec 26 §12 known-unfalsifiable. `envLessBridgeConfig.ts:34-36` comment implies hard floor; behavior is server-policy.
8. ~~**Multi-bridge per host:dir lock-file semantics**~~ — **NOTE Phase 9.7**: `tengu_ccr_bridge_multi_environment` GrowthBook gate is documented at the comment level (`bridgeMain.ts:86-89`); conflict-resolution policy belongs to spec 35. Both specs cite consistently.
9. ~~**`INTERNAL_CLAUDE_BRIDGE_PARENT_PID` IPC channel**~~ — **NOTE Phase 9.7**: visible in `sessionRunner.ts` (sampled); spec 35 (remote-server) §5 documents the spawn-mode IPC at higher level. Spec-level boundary preserved.
10. ~~**Settings sync between IDE and CLI**~~ — **RESOLVED Phase 9.7**: confirmed no on-disk settings sync at this layer. Mechanisms are `updateBridgeSessionTitle` PATCH on `/rename` and GrowthBook fetch. Cross-IDE settings (if any) live in spec 23 / 28.
