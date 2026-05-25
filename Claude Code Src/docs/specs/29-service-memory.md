# 29 — Service: Memory (extractMemories, teamMemorySync, SessionMemory)

> Owns the **producer / consumer surface** to persistent memory: the auto-memory
> background-extraction pipeline, the conversational session-memory
> hook+manual-trigger pipeline, and the bidirectional team-memory sync
> service. The on-disk MEMORY.md storage layer (memdir) is owned by spec **40**;
> CLAUDE.md / memory injection into the system prompt is owned by **05**;
> compaction is owned by **07**.

---

## 1. Purpose & Scope

This spec captures the three "memory" services under `src/services/`:

1. **`extractMemories/`** — fire-and-forget forked subagent that runs at every
   stop-hook on the main REPL thread and distills the trailing N model-visible
   messages into typed memory files under the auto-memory directory.
2. **`SessionMemory/`** — post-sampling hook that maintains a single
   `session-memory.md` template-driven file as the conversation grows, plus the
   manual `/summary`-style trigger and the `getSessionMemoryContent` /
   `waitForSessionMemoryExtraction` consumer surface used by compaction (07)
   and the away-summary (`services/awaySummary.ts`).
3. **`teamMemorySync/`** — repo-scoped GET/PUT bidirectional sync of the
   `team/` subtree of the auto-memory directory against
   `${BASE_API_URL}/api/claude_code/team_memory?repo=…`, including a debounced
   directory watcher, secret-scanner pre-filter, ETag/`If-Match` optimistic
   locking with 412-conflict probe-and-retry, structured 413 max-entry parsing,
   and PUT-batching by body bytes.

**Also covered (interaction surface only):**

- `memdir/paths.ts:isAutoMemoryEnabled / isExtractModeActive / hasAutoMemPathOverride / getAutoMemPath / isAutoMemPath / getAutoMemEntrypoint / getMemoryBaseDir`
  (helpers consumed by all three services; storage details own to 40).
- `memdir/memdir.ts:loadMemoryPrompt` and the four-type taxonomy in
  `memdir/memoryTypes.ts` — `loadMemoryPrompt` is the system-prompt builder
  cited by 03/05; the **§6** body of this spec inlines its full text because
  the extraction prompt is intentionally a perfect-fork sibling that reuses
  these blocks.
- `EXTRACT_MEMORIES` callsite gates at
  `src/utils/backgroundHousekeeping.ts:7,34` and `src/query/stopHooks.ts:42,142`.
- `MEMORY_SHAPE_TELEMETRY` flag at `src/memdir/findRelevantMemories.ts:66`.
- `TEAMMEM` setup-time activation at `src/setup.ts:294,365` and the
  watcher-init lazy import.
- `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` env var (consumed by paths.ts; spec 40
  owns full path resolution but this spec exposes `hasAutoMemPathOverride()`
  as a public-interface predicate).
- `recordSkillUsage` (`utils/suggestions/skillUsageTracking.ts:13`) — does not
  intersect any of these services (verified by grep; see §2.6).

**Delegation to spec 30 (coordinator/multi-agent)**: agent/away/toolUse/autoDream summary services live in spec 30; this spec owns the memory service proper (`extractMemories/`, `SessionMemory/`, `teamMemorySync/`) and `MEMORY.md` content lifecycle only. Specifically `services/AgentSummary/`, `services/awaySummary.ts`, `services/toolUseSummary/`, and `services/autoDream/` are coordinator-owned.

**Out of scope** (cite-only):

| Concern | Owner |
|---|---|
| MEMORY.md on-disk format, frontmatter parsing, `memdir/memoryScan.ts`, `memdir/findRelevantMemories.ts`, `memdir/teamMemPaths.ts`, `memdir/teamMemPrompts.ts` | 40 |
| CLAUDE.md walk + injection (`getClaudeMds`, `getMemoryFiles`, `filterInjectedMemoryFiles`) | 05 — canonical write site is `src/context.ts:176` `setCachedClaudeMdContent(claudeMd \|\| null)` (the `\|\|` is intentional — empty-string CLAUDE.md must collapse to `null`; `??` would propagate `""` and defeat the cache-miss signal). The setter itself lives in `src/bootstrap/state.ts:1207`. |
| `services/compact/sessionMemoryCompact.ts` consumer logic | 07 |
| Skill matching (`SkillTool`) | 17 |
| Persisted session/transcript state | 41 |
| `runForkedAgent`, `createCacheSafeParams`, `createSubagentContext` (used by both extractMemories and SessionMemory) | 03 (forked agent infrastructure) / 30 (multi-agent) |

---

## 2. Source Map

### 2.1 Owned files (read fully)

| Path | Lines | Coverage |
|---|---|---|
| `src/services/extractMemories/extractMemories.ts` | 615 | full |
| `src/services/extractMemories/prompts.ts` | 154 | full |
| `src/services/SessionMemory/sessionMemory.ts` | 495 | full |
| `src/services/SessionMemory/sessionMemoryUtils.ts` | 207 | full |
| `src/services/SessionMemory/prompts.ts` | 324 | full |
| `src/services/teamMemorySync/index.ts` | 1256 | full |
| `src/services/teamMemorySync/secretScanner.ts` | 324 | full |
| `src/services/teamMemorySync/teamMemSecretGuard.ts` | 44 | full |
| `src/services/teamMemorySync/types.ts` | 156 | full |
| `src/services/teamMemorySync/watcher.ts` | 387 | full |

(Line counts are `wc -l` output, i.e., newline counts; files end in a trailing
newline so the byte-final line is unterminated and not counted. Earlier
revisions of this table reported `wc -l + 1`; corrected here.)

### 2.2 Owned interaction-surface (cited only)

| Path | Cite | Purpose |
|---|---|---|
| `src/memdir/paths.ts:30-55` | `isAutoMemoryEnabled` | gate consumer |
| `src/memdir/paths.ts:69-77` | `isExtractModeActive` | extraction gate consumer |
| `src/memdir/paths.ts:194-196` | `hasAutoMemPathOverride` | predicate |
| `src/memdir/paths.ts:223-235` | `getAutoMemPath` (memoized) | dir resolution |
| `src/memdir/paths.ts:274-278` | `isAutoMemPath` | path matcher |
| `src/memdir/paths.ts:161-166` | `getAutoMemPathOverride` | env override |
| `src/memdir/memdir.ts:34-38` | `ENTRYPOINT_NAME`, `MAX_ENTRYPOINT_LINES`, `MAX_ENTRYPOINT_BYTES` | re-exported constants |
| `src/memdir/memdir.ts:419-507` | `loadMemoryPrompt` | system-prompt builder |
| `src/memdir/memoryTypes.ts` | full | four-type taxonomy + frontmatter, inlined in §6.2 |

### 2.3 Feature-flag and gate sites (verbatim)

| Gate | File:line | Mechanism |
|---|---|---|
| `feature('EXTRACT_MEMORIES')` (top-level conditional require) | `src/utils/backgroundHousekeeping.ts:7-9` | DCE; binds `extractMemoriesModule` |
| `feature('EXTRACT_MEMORIES')` (init call) | `src/utils/backgroundHousekeeping.ts:34-36` | runtime branch in `startBackgroundHousekeeping` |
| `feature('EXTRACT_MEMORIES')` (top-level conditional require) | `src/query/stopHooks.ts:42-44` | binds `extractMemoriesModule` |
| `feature('EXTRACT_MEMORIES') && !toolUseContext.agentId && isExtractModeActive()` | `src/query/stopHooks.ts:142-144` | per-stop-hook fire-and-forget gate |
| `feature('EXTRACT_MEMORIES')` (drain) | `src/cli/print.ts:374,968` | non-interactive drain |
| `feature('TEAMMEM')` (extractMemories internal) | `src/services/extractMemories/extractMemories.ts:65-67`, `:362-364`, `:402-413`, `:468-470`, `:492-494` | combined-prompt branch + team-count |
| `feature('TEAMMEM')` (prompts) | `src/services/extractMemories/prompts.ts:106-112` | dispatch to combined prompt |
| `feature('TEAMMEM')` (setup activation) | `src/setup.ts:365-369` | lazy import of `watcher.ts:startTeamMemoryWatcher` |
| `feature('TEAMMEM')` (watcher early-out) | `src/services/teamMemorySync/watcher.ts:253-255` | hard return |
| `feature('TEAMMEM')` (secret guard) | `src/services/teamMemorySync/teamMemSecretGuard.ts:19` | callable-but-inert wrapper |
| `feature('MEMORY_SHAPE_TELEMETRY')` | `src/memdir/findRelevantMemories.ts:66` | telemetry-only, owned by 40; recorded here per dispatch §scope |
| `process.env.USER_TYPE === 'ant'` (gate-disabled telemetry) | `src/services/extractMemories/extractMemories.ts:537-540`, `src/services/SessionMemory/sessionMemory.ts:286-289`, `:362-367` | once-per-session ANT telemetry |
| `process.env.CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` | `src/memdir/paths.ts:163` | env override consumed by `getAutoMemPathOverride` |
| `process.env.CLAUDE_CODE_DISABLE_AUTO_MEMORY` | `src/memdir/paths.ts:31-36` | env override of `isAutoMemoryEnabled` |
| `process.env.CLAUDE_CODE_REMOTE` + `CLAUDE_CODE_REMOTE_MEMORY_DIR` | `src/memdir/paths.ts:44-49`, `:86-89` | remote-mode disable + base override |
| `process.env.CLAUDE_CODE_SIMPLE` (hard-off) | `src/memdir/paths.ts:41-43` | overrides isAutoMemoryEnabled |
| `process.env.TEAM_MEMORY_SYNC_URL` | `src/services/teamMemorySync/index.ts:165` | endpoint override |

`feature('EXTRACT_MEMORIES')` does NOT have a third gate at
`memdir/paths.ts:65` — that line is a doc comment only (per HANDOFF §6).
The two real gates are `utils/backgroundHousekeeping.ts:7,34` and
`query/stopHooks.ts:42,142`. (HANDOFF §6 wording was "142" — the same line
still gates after the conditional-require addition.)

**Datadog telemetry coverage gap (cross-spec → 26).** The `teamMemorySync`
subsystem emits **six** distinct `tengu_team_mem_*` events from owned files
(plus `tengu_extract_memories_*`, `tengu_session_memory_*`,
`tengu_auto_mem_tool_denied` from the extraction side):

| Event | Site | Allow-listed in `services/analytics/datadog.ts:60-63`? |
|---|---|---|
| `tengu_team_mem_sync_pull` | `teamMemorySync/index.ts:1205` | yes (`:60`) |
| `tengu_team_mem_sync_push` | `teamMemorySync/index.ts:1233` | yes (`:61`) |
| `tengu_team_mem_sync_started` | `teamMemorySync/watcher.ts:298` | yes (`:62`) |
| `tengu_team_mem_entries_capped` | `teamMemorySync/index.ts:661` | yes (`:63`) |
| `tengu_team_mem_secret_skipped` | `teamMemorySync/index.ts:935` | **NO — emitted but not allow-listed** |
| `tengu_team_mem_push_suppressed` | `teamMemorySync/watcher.ts:112` | **NO — emitted but not allow-listed** |

Either the allow-list is incomplete (in which case spec 26's count of 44
should rise to **46**) or these two events are intentionally dropped at the
Datadog forwarder. Spec 29 records the gap; spec 26 owns the count
reconciliation. Phase 9.6c finding F1.

### 2.4 Imports from

extractMemories: `bun:bundle`, `path`, `bootstrap/state`, `hooks/useCanUseTool`,
`memdir/memdir`, `memdir/memoryScan`, `memdir/paths`, `memdir/teamMemPaths`
(via lazy require), `memdir/memoryTypes`, `Tool`, `tools/{Bash,FileEdit,FileRead,FileWrite,Glob,Grep,REPL}Tool/*` (name constants), `types/message`, `utils/abortController`, `utils/array`, `utils/debug`, `utils/forkedAgent` (`createCacheSafeParams`, `runForkedAgent`), `utils/hooks/postSamplingHooks` (`REPLHookContext`), `utils/messages` (`createMemorySavedMessage`, `createUserMessage`), `services/analytics/growthbook`, `services/analytics`, `services/analytics/metadata`.

SessionMemory: `fs/promises`, `lodash-es/memoize`, `path`, `bootstrap/state`,
`constants/prompts` (`getSystemPrompt`), `context` (`getSystemContext`,
`getUserContext`), `hooks/useCanUseTool`, `Tool`, `tools/FileEditTool/constants`,
`tools/FileReadTool/FileReadTool`, `types/message`, `utils/array`,
`utils/forkedAgent`, `utils/fsOperations`, `utils/hooks/postSamplingHooks`
(`registerPostSamplingHook`), `utils/messages`,
`utils/permissions/filesystem` (`getSessionMemoryDir`, `getSessionMemoryPath`),
`utils/sequential`, `utils/systemPromptType`, `utils/tokens` (`getTokenUsage`,
`tokenCountWithEstimation`), `utils/errors`, `utils/sleep`, `utils/log`,
`services/analytics`, `services/analytics/growthbook`, `services/compact/autoCompact` (`isAutoCompactEnabled`).

teamMemorySync (index): `axios`, `crypto`, `fs/promises`, `path`,
`constants/oauth` (`CLAUDE_AI_INFERENCE_SCOPE`, `CLAUDE_AI_PROFILE_SCOPE`,
`getOauthConfig`, `OAUTH_BETA_HEADER`), `memdir/teamMemPaths`
(`getTeamMemPath`, `PathTraversalError`, `validateTeamMemKey`),
`utils/array`, `utils/auth` (OAuth refresh + tokens), `utils/debug`,
`utils/errors` (`classifyAxiosError`), `utils/git` (`getGithubRepo`),
`utils/model/providers`, `utils/sleep`, `utils/slowOperations` (`jsonStringify`),
`utils/userAgent`, `services/analytics`, `services/analytics/metadata`,
`services/api/withRetry` (`getRetryDelay`), `./secretScanner`, `./types`.

teamMemorySync (watcher): `bun:bundle`, `fs.watch` + `mkdir/stat`, `path`,
`memdir/teamMemPaths`, `utils/cleanupRegistry` (`registerCleanup`),
`utils/debug`, `utils/errors`, `utils/git`,
`services/analytics`, `./index`, `./types`.

### 2.5 Imported by

| Importer | Symbol | Cite |
|---|---|---|
| `utils/backgroundHousekeeping.ts` | `initExtractMemories` | `:7-9,:34-36` |
| `query/stopHooks.ts` | `executeExtractMemories` | `:42-44,:149-152` |
| `cli/print.ts` | `drainPendingExtraction` | `:374,:968` |
| `setup.ts` | `initSessionMemory` | `:22,:294` |
| `setup.ts` (lazy import) | `startTeamMemoryWatcher` | `:365-369` |
| `services/awaySummary.ts` | `getSessionMemoryContent` | `:12,:38` |
| `skills/bundled/skillify.ts` | `getSessionMemoryContent` | `:1,:181` |
| `services/compact/sessionMemoryCompact.ts` | `getSessionMemoryContent`, `waitForSessionMemoryExtraction`, `truncateSessionMemoryForCompact` | `:28,:32,:33,:462,:527,:530` |

### 2.6 Negative coverage

`recordSkillUsage` is defined at `utils/suggestions/skillUsageTracking.ts:13`
and is NOT imported by any of the three owned services (verified by `grep -rn "recordSkillUsage" src/services/{extractMemories,SessionMemory,teamMemorySync}`).
The dispatch brief's "if any" question resolves to **no integration**.

`utils/sessionFileAccessHooks.ts` is registered alongside the team-memory
watcher in `setup.ts` but is owned by 41 (session state).

### 2.7 Missing-source ledger — none

All in-scope dirs are present in the leak. `memdir/teamMemPaths.ts`,
`memdir/teamMemPrompts.ts`, `memdir/findRelevantMemories.ts`,
`memdir/memoryShapeTelemetry.ts` are imported but owned by spec 40.

---

## 3. Public Interface

### 3.1 `extractMemories`

```ts
// src/services/extractMemories/extractMemories.ts:171-222
// CROSS-SPEC: also consumed by services/autoDream/ (spec 30) — see
// extractMemories.ts:169 ("shared by extractMemories and autoDream").
// This is a public export of spec-29 territory used by a spec-30 owner.
export function createAutoMemCanUseTool(memoryDir: string): CanUseToolFn

// :296-587
export function initExtractMemories(): void

// :598-603
export async function executeExtractMemories(
  context: REPLHookContext,
  appendSystemMessage?: AppendSystemMessageFn,
): Promise<void>

// :611-615
export async function drainPendingExtraction(timeoutMs?: number): Promise<void>
```

`AppendSystemMessageFn` is `(msg: Exclude<SystemMessage, SystemLocalCommandMessage>) => void` (`:275-277`).

Inside `initExtractMemories` (closure-scoped state):

```ts
const inFlightExtractions: Set<Promise<void>>
let lastMemoryMessageUuid: string | undefined
let hasLoggedGateFailure: boolean
let inProgress: boolean
let turnsSinceLastExtraction: number
let pendingContext: { context: REPLHookContext; appendSystemMessage?: AppendSystemMessageFn } | undefined
```

The module-level `extractor` and `drainer` variables are bound by
`initExtractMemories` (`:280-288`). Calling `executeExtractMemories` /
`drainPendingExtraction` before init is a silent no-op (`:602`, `:614`).

### 3.2 `SessionMemory`

```ts
// src/services/SessionMemory/sessionMemory.ts:104-106, 134-181, 357-375
export function resetLastMemoryMessageUuid(): void
export function shouldExtractMemory(messages: Message[]): boolean
export function initSessionMemory(): void

// :377-453
export type ManualExtractionResult = {
  success: boolean
  memoryPath?: string
  error?: string
}
export async function manuallyExtractSessionMemory(
  messages: Message[],
  toolUseContext: ToolUseContext,
): Promise<ManualExtractionResult>

// :460-482
export function createMemoryFileCanUseTool(memoryPath: string): CanUseToolFn
```

`sessionMemoryUtils.ts` (`:18-29, :32-36, :39-41, :44, :47, :50, :53, :58-60, :65-69, :74-76, :81-83, :89-105, :110-126, :131-138, :143-145, :151-153, :158-160, :165-167, :173-177, :184-189, :194-196, :201-207`):

```ts
export type SessionMemoryConfig = {
  minimumMessageTokensToInit: number
  minimumTokensBetweenUpdate: number
  toolCallsBetweenUpdates: number
}
export const DEFAULT_SESSION_MEMORY_CONFIG: SessionMemoryConfig
export function getLastSummarizedMessageId(): string | undefined
export function setLastSummarizedMessageId(id: string | undefined): void
export function markExtractionStarted(): void
export function markExtractionCompleted(): void
export async function waitForSessionMemoryExtraction(): Promise<void>
export async function getSessionMemoryContent(): Promise<string | null>
export function setSessionMemoryConfig(config: Partial<SessionMemoryConfig>): void
export function getSessionMemoryConfig(): SessionMemoryConfig
export function recordExtractionTokenCount(currentTokenCount: number): void
export function isSessionMemoryInitialized(): boolean
export function markSessionMemoryInitialized(): void
export function hasMetInitializationThreshold(currentTokenCount: number): boolean
export function hasMetUpdateThreshold(currentTokenCount: number): boolean
export function getToolCallsBetweenUpdates(): number
export function resetSessionMemoryState(): void
```

`prompts.ts`:

```ts
// src/services/SessionMemory/prompts.ts:11-41, :86-104, :111-129, :220-224, :226-247, :256-296
export const DEFAULT_SESSION_MEMORY_TEMPLATE: string
export async function loadSessionMemoryTemplate(): Promise<string>
export async function loadSessionMemoryPrompt(): Promise<string>
export async function isSessionMemoryEmpty(content: string): Promise<boolean>
export async function buildSessionMemoryUpdatePrompt(
  currentNotes: string,
  notesPath: string,
): Promise<string>
export function truncateSessionMemoryForCompact(content: string): {
  truncatedContent: string
  wasTruncated: boolean
}
```

### 3.3 `teamMemorySync`

```ts
// src/services/teamMemorySync/index.ts:100-127, :134-136
export type SyncState = {
  lastKnownChecksum: string | null
  serverChecksums: Map<string, string>
  serverMaxEntries: number | null
}
export function createSyncState(): SyncState
export function hashContent(content: string): string

// :426-460
export function batchDeltaByBytes(
  delta: Record<string, string>,
): Array<Record<string, string>>

// :762-764
export function isTeamMemorySyncAvailable(): boolean

// :770-867
export async function pullTeamMemory(
  state: SyncState,
  options?: { skipEtagCache?: boolean },
): Promise<{ success: boolean; filesWritten: number; entryCount: number; notModified?: boolean; error?: string }>

// :889-1146
export async function pushTeamMemory(state: SyncState): Promise<TeamMemorySyncPushResult>

// :1153-1191
export async function syncTeamMemory(state: SyncState): Promise<{ success: boolean; filesPulled: number; filesPushed: number; error?: string }>
```

Watcher (`watcher.ts:61-73, :252-305, :314-318, :327-352, :365-378, :385-387`):

```ts
export function isPermanentFailure(r: TeamMemorySyncPushResult): boolean
export async function startTeamMemoryWatcher(): Promise<void>
export async function notifyTeamMemoryWrite(): Promise<void>
export async function stopTeamMemoryWatcher(): Promise<void>
export function _resetWatcherStateForTesting(opts?: {
  syncState?: SyncState; skipWatcher?: boolean; pushSuppressedReason?: string | null
}): void
export function _startFileWatcherForTesting(dir: string): Promise<void>
```

Secret guard (`teamMemSecretGuard.ts:15-44`):

```ts
export function checkTeamMemSecrets(filePath: string, content: string): string | null
```

Secret scanner (`secretScanner.ts:23-37, :277-294, :301-303, :312-324`):

```ts
type SecretRule = { id: string; source: string; flags?: string }
export type SecretMatch = { ruleId: string; label: string }
export function scanForSecrets(content: string): SecretMatch[]
export function getSecretLabel(ruleId: string): string
export function redactSecrets(content: string): string
```

Types (`types.ts:16-24, :29-38, :47-57, :59, :66-72, :77-87, :94-102, :107-124, :129-156`): see §6.5 for verbatim Zod.

---

## 4. Data Model & State

### 4.1 `extractMemories` closure state (`extractMemories.ts:296-326`)

| Slot | Initial | Purpose |
|---|---|---|
| `inFlightExtractions: Set<Promise<void>>` | empty | drain target for `drainPendingExtraction` |
| `lastMemoryMessageUuid: string \| undefined` | undefined | message-cursor; advances on success and on `tengu_extract_memories_skipped_direct_write` |
| `hasLoggedGateFailure: boolean` | false | once-per-session ANT-only gate-disabled telemetry latch |
| `inProgress: boolean` | false | exclusion latch around `runExtraction` |
| `turnsSinceLastExtraction: number` | 0 | throttle counter; reset to 0 on every run start |
| `pendingContext` | undefined | holds at most one trailing context while `inProgress` |

`extractor`, `drainer` are module-level `let` bindings, rebound by every call
to `initExtractMemories()` (`:280-288`, `:569-587`).

### 4.2 SessionMemory module state

`sessionMemory.ts`:
- `lastMemoryMessageUuid: string | undefined` (`:99`) — cursor for `countToolCallsSince`.
- `hasLoggedGateFailure: boolean` (`:270`) — ANT once-per-session latch.
- `extractSessionMemory` is wrapped in `sequential(...)` (`:272-350`) — at most
  one extraction in flight across the registered post-sampling hook.

`sessionMemoryUtils.ts` module state (`:39-53`):
- `sessionMemoryConfig: SessionMemoryConfig`
- `lastSummarizedMessageId: string | undefined`
- `extractionStartedAt: number | undefined` (timestamp)
- `tokensAtLastExtraction: number = 0`
- `sessionMemoryInitialized: boolean = false`

Wait/timeout constants (`:12-13`):
```
EXTRACTION_WAIT_TIMEOUT_MS = 15000
EXTRACTION_STALE_THRESHOLD_MS = 60000
```

### 4.3 `teamMemorySync` `SyncState` (`index.ts:100-119`)

| Field | Init | Mutation point |
|---|---|---|
| `lastKnownChecksum: string \| null` | null | set to `responseChecksum` on every successful pull (`:252-254`); to upload-response checksum (`:502-505`); cleared to null on 404 (`:230`, `:333-335`); used as `If-None-Match` GET header (`:206-209`) and `If-Match` PUT header (`:480-482`) |
| `serverChecksums: Map<string,string>` | empty | cleared+repopulated from server `entryChecksums` on pull (`:828`, `:839-849`); cleared+repopulated from `?view=hashes` probe on 412 (`:1134-1137`); per-key updated on each successful PUT batch (`:1011-1014`) |
| `serverMaxEntries: number \| null` | null | learned from structured 413 response (`:1052-1058`); used as truncation cap in `readLocalTeamMemory` (`:654-672`) |

### 4.4 `teamMemorySync` watcher state (`watcher.ts:38-76`)

```
watcher: FSWatcher | null         = null
debounceTimer: setTimeout handle  = null
pushInProgress: boolean           = false
hasPendingChanges: boolean        = false
currentPushPromise: Promise<void> | null = null
watcherStarted: boolean           = false
pushSuppressedReason: string | null = null
syncState: SyncState | null       = null
```

`DEBOUNCE_MS = 2000` (`watcher.ts:35`).

### 4.5 In-band Zod schemas

See §6.5 — three lazy-loaded schemas in `teamMemorySync/types.ts`. The
extract/session pipelines define no on-the-wire Zod schemas (file content is
markdown-with-frontmatter; the frontmatter taxonomy is ledgered in
`memdir/memoryTypes.ts` — owned by 40 but inlined verbatim in §6.2 because the
extraction prompt embeds it directly).

---

## 5. Algorithm / Control Flow

### 5.1 extractMemories — outer pipeline

```
boot:
  startBackgroundHousekeeping()                 # utils/backgroundHousekeeping.ts:34
    if feature('EXTRACT_MEMORIES'):
      extractMemoriesModule.initExtractMemories()   # binds extractor/drainer

per-turn (handleStopHooks at query/stopHooks.ts:142):
  if feature('EXTRACT_MEMORIES')
     && !toolUseContext.agentId
     && isExtractModeActive():
      void executeExtractMemories(stopHookContext, appendSystemMessage)
                                                # fire-and-forget; promise tracked
                                                # in inFlightExtractions

shutdown (cli/print.ts:968 non-interactive):
  await drainPendingExtraction()                # races Promise.all vs 60s timer
```

`isExtractModeActive` (memdir/paths.ts:69-77): requires GB
`tengu_passport_quail = true`, AND
(`!getIsNonInteractiveSession()` OR GB `tengu_slate_thimble = true`).

### 5.2 extractMemories — `executeExtractMemoriesImpl` (`:527-567`)

```
function executeExtractMemoriesImpl(context, appendSystemMessage):
  if context.toolUseContext.agentId: return                  # subagent skip
  if !GB('tengu_passport_quail', false):
      if USER_TYPE === 'ant' and not hasLoggedGateFailure:
          hasLoggedGateFailure = true
          logEvent('tengu_extract_memories_gate_disabled', {})
      return
  if !isAutoMemoryEnabled(): return
  if getIsRemoteMode(): return
  if inProgress:
      logEvent('tengu_extract_memories_coalesced', {})
      pendingContext = { context, appendSystemMessage }       # overwrites
      return
  await runExtraction({ context, appendSystemMessage })
```

`extractor` (`:569-577`) wraps the impl with the `inFlightExtractions` Set.

### 5.3 extractMemories — `runExtraction` (`:329-523`)

```
function runExtraction({ context, appendSystemMessage, isTrailingRun }):
  messages    = context.messages
  memoryDir   = getAutoMemPath()
  newMessageCount = countModelVisibleMessagesSince(messages, lastMemoryMessageUuid)

  # Mutual exclusion vs. main agent: if main agent itself wrote any memory file
  if hasMemoryWritesSince(messages, lastMemoryMessageUuid):
      lastMemoryMessageUuid = lastMessage(messages)?.uuid       # advance cursor
      logEvent('tengu_extract_memories_skipped_direct_write',
               { message_count: newMessageCount })
      return

  teamMemoryEnabled = feature('TEAMMEM') ? teamMemPaths.isTeamMemoryEnabled() : false
  skipIndex         = GB('tengu_moth_copse', false)
  canUseTool        = createAutoMemCanUseTool(memoryDir)
  cacheSafeParams   = createCacheSafeParams(context)

  # Throttle (eligible-turns gate):
  if not isTrailingRun:
      turnsSinceLastExtraction++
      threshold = GB('tengu_bramble_lintel', null) ?? 1
      if turnsSinceLastExtraction < threshold: return
  turnsSinceLastExtraction = 0

  inProgress = true
  startTime = now()
  try:
      existingMemories = formatMemoryManifest(
          await scanMemoryFiles(memoryDir, createAbortController().signal))
      userPrompt = (feature('TEAMMEM') && teamMemoryEnabled)
        ? buildExtractCombinedPrompt(newMessageCount, existingMemories, skipIndex)
        : buildExtractAutoOnlyPrompt(newMessageCount, existingMemories, skipIndex)

      result = await runForkedAgent({
          promptMessages: [createUserMessage({ content: userPrompt })],
          cacheSafeParams,
          canUseTool,
          querySource: 'extract_memories',
          forkLabel:   'extract_memories',
          skipTranscript: true,
          maxTurns: 5,
      })

      lastMemoryMessageUuid = lastMessage(messages)?.uuid       # advance cursor

      writtenPaths = extractWrittenPaths(result.messages)
      turnCount    = count(result.messages, m=>m.type==='assistant')
      memoryPaths  = writtenPaths.filter(p => basename(p) !== ENTRYPOINT_NAME)
      teamCount    = feature('TEAMMEM')
                     ? count(memoryPaths, teamMemPaths.isTeamMemPath) : 0

      logEvent('tengu_extract_memories_extraction', {
          input_tokens, output_tokens,
          cache_read_input_tokens, cache_creation_input_tokens,
          message_count: newMessageCount,
          turn_count: turnCount,
          files_written: writtenPaths.length,
          memories_saved: memoryPaths.length,
          team_memories_saved: teamCount,
          duration_ms: now() - startTime,
      })

      if memoryPaths.length > 0:
          msg = createMemorySavedMessage(memoryPaths)
          if feature('TEAMMEM'): msg.teamCount = teamCount
          appendSystemMessage?.(msg)
  catch (error):
      logEvent('tengu_extract_memories_error', { duration_ms })
  finally:
      inProgress = false
      trailing = pendingContext
      pendingContext = undefined
      if trailing:
          await runExtraction({ ...trailing, isTrailingRun: true })
```

### 5.4 extractMemories — `createAutoMemCanUseTool` decision tree (`:171-222`)

```
on tool:
  if name === REPL_TOOL_NAME:       return allow
  if name in {FILE_READ, GREP, GLOB}: return allow
  if name === BASH:
      parsed = inputSchema.safeParse(input)
      if parsed.success && tool.isReadOnly(parsed.data): return allow
      return deny('Only read-only shell commands are permitted in this context (ls, find, grep, cat, stat, wc, head, tail, and similar)')
  if name in {FILE_EDIT, FILE_WRITE} && 'file_path' in input:
      if typeof input.file_path === 'string' && isAutoMemPath(input.file_path):
          return allow
  return deny('only <READ>, <GREP>, <GLOB>, read-only <BASH>, and <EDIT>/<WRITE> within ${memoryDir} are allowed')
```

Every deny path emits `tengu_auto_mem_tool_denied` with the sanitized tool
name (`:156-158`).

### 5.5 extractMemories — `hasMemoryWritesSince` (`:121-148`) and `extractWrittenPaths` (`:251-269`)

`getWrittenFilePath(block)` (`:232-249`):
- requires `block.type === 'tool_use'`
- name in `{FILE_EDIT, FILE_WRITE}`
- returns `string | undefined` (only truthy `string` file_paths)

`hasMemoryWritesSince` walks messages after the cursor; for each assistant
message, scans content blocks; any tool_use with a write to an
`isAutoMemPath()` path returns `true`. `extractWrittenPaths` collects all
such paths (`uniq`).

### 5.6 SessionMemory — outer pipeline

```
boot (setup.ts:294):
  initSessionMemory()
    if getIsRemoteMode(): return
    autoCompactEnabled = isAutoCompactEnabled()
    if USER_TYPE==='ant': logEvent('tengu_session_memory_init', { auto_compact_enabled })
    if !autoCompactEnabled: return
    registerPostSamplingHook(extractSessionMemory)         # sequential-wrapped
```

`extractSessionMemory` body (`:272-350`):

```
guard:
  if querySource !== 'repl_main_thread': return
  if !isSessionMemoryGateEnabled():                        # GB tengu_session_memory
      if USER_TYPE==='ant' && !hasLoggedGateFailure:
          hasLoggedGateFailure = true
          logEvent('tengu_session_memory_gate_disabled', {})
      return
  initSessionMemoryConfigIfNeeded()                        # memoized
  if !shouldExtractMemory(messages): return

work:
  markExtractionStarted()
  setupContext = createSubagentContext(toolUseContext)
  { memoryPath, currentMemory } = setupSessionMemoryFile(setupContext)
  userPrompt = buildSessionMemoryUpdatePrompt(currentMemory, memoryPath)
  await runForkedAgent({
      promptMessages: [createUserMessage({ content: userPrompt })],
      cacheSafeParams: createCacheSafeParams(context),
      canUseTool: createMemoryFileCanUseTool(memoryPath),
      querySource: 'session_memory',
      forkLabel:   'session_memory',
      overrides: { readFileState: setupContext.readFileState },
  })
  logEvent('tengu_session_memory_extraction', { ... })
  recordExtractionTokenCount(tokenCountWithEstimation(messages))
  updateLastSummarizedMessageIdIfSafe(messages)
  markExtractionCompleted()
```

`shouldExtractMemory` (`:134-181`):

```
currentTokenCount = tokenCountWithEstimation(messages)
if !isSessionMemoryInitialized():
    if !hasMetInitializationThreshold(currentTokenCount): return false
    markSessionMemoryInitialized()
hasMetTokenThreshold = hasMetUpdateThreshold(currentTokenCount)
toolCallsSinceLastUpdate = countToolCallsSince(messages, lastMemoryMessageUuid)
hasMetToolCallThreshold = toolCallsSinceLastUpdate >= getToolCallsBetweenUpdates()
hasToolCallsInLastTurn  = hasToolCallsInLastAssistantTurn(messages)
shouldExtract = (hasMetTokenThreshold && hasMetToolCallThreshold)
              || (hasMetTokenThreshold && !hasToolCallsInLastTurn)
if shouldExtract:
    lastMemoryMessageUuid = lastMessage(messages)?.uuid
    return true
return false
```

`countToolCallsSince` (`:108-132`): if `sinceUuid` is null/undefined,
`foundStart=true` immediately; otherwise scans for matching uuid; once
`foundStart`, sums `tool_use` blocks across assistant message contents.

`updateLastSummarizedMessageIdIfSafe` (`:488-495`): only sets
`lastSummarizedMessageId` if `!hasToolCallsInLastAssistantTurn(messages)`.

### 5.7 SessionMemory — `setupSessionMemoryFile` (`:183-233`)

```
sessionMemoryDir = getSessionMemoryDir()
fs.mkdir(sessionMemoryDir, { mode: 0o700 })
memoryPath = getSessionMemoryPath()
try:
    writeFile(memoryPath, '', { encoding: 'utf-8', mode: 0o600, flag: 'wx' })  # O_CREAT|O_EXCL
    template = await loadSessionMemoryTemplate()
    writeFile(memoryPath, template, { encoding: 'utf-8', mode: 0o600 })
catch e:
    if errno !== 'EEXIST': throw

toolUseContext.readFileState.delete(memoryPath)        # bust FileRead dedupe
result = FileReadTool.call({ file_path: memoryPath }, toolUseContext)
currentMemory = (output.type === 'text') ? output.file.content : ''
logEvent('tengu_session_memory_file_read', { content_length })
return { memoryPath, currentMemory }
```

### 5.8 SessionMemory — `createMemoryFileCanUseTool` (`:460-482`)

Deny-by-default; allow ONLY `FILE_EDIT_TOOL_NAME` whose `input.file_path` is
exactly the resolved memory path. Deny message:
`only ${FILE_EDIT_TOOL_NAME} on ${memoryPath} is allowed`.

### 5.9 SessionMemory prompts — token-budget guard (`prompts.ts:131-196`)

```
analyzeSectionSizes(content):
    walk lines; on '# ' header flush prior section to map[header] = roughTokenCountEstimation(prevContent)
generateSectionReminders(sectionSizes, totalTokens):
    overBudget        = totalTokens > MAX_TOTAL_SESSION_MEMORY_TOKENS  # 12000
    oversizedSections = [(s, t) for s,t in sectionSizes if t > MAX_SECTION_LENGTH]  # 2000
                        sorted desc by t
    if no oversized && !overBudget: return ''
    parts = []
    if overBudget:        parts.push(<critical-budget-message>)
    if oversizedSections: parts.push((overBudget?'Oversized…':'IMPORTANT:…') + bulleted list)
    return joined
buildSessionMemoryUpdatePrompt(currentNotes, notesPath):
    promptTemplate = await loadSessionMemoryPrompt()                # ~/.claude/session-memory/config/prompt.md or default
    sectionSizes   = analyzeSectionSizes(currentNotes)
    totalTokens    = roughTokenCountEstimation(currentNotes)
    sectionReminders = generateSectionReminders(sectionSizes, totalTokens)
    basePrompt     = substituteVariables(promptTemplate, { currentNotes, notesPath })
    return basePrompt + sectionReminders
substituteVariables(template, vars):
    template.replace(/\{\{(\w+)\}\}/g, (m, k) => hasOwnProperty(vars,k) ? vars[k] : m)   # single-pass
```

Constants (`:8-9`):
```
MAX_SECTION_LENGTH = 2000
MAX_TOTAL_SESSION_MEMORY_TOKENS = 12000
```

`truncateSessionMemoryForCompact` (`:256-324`):
- `maxCharsPerSection = MAX_SECTION_LENGTH * 4` (because
  `roughTokenCountEstimation` uses `length/4`)
- Walks lines; on each `# ` header, flushes prior section via
  `flushSessionSection`, which keeps lines while `charCount + line.length+1 <= maxCharsPerSection`,
  then appends literal `\n[... section truncated for length ...]`.

### 5.10 teamMemorySync — pull (`:770-867`, with `fetchTeamMemoryOnce` `:188-306`)

```
pullTeamMemory(state, { skipEtagCache=false }):
    if !isUsingOAuth():        log no_oauth, fail
    repoSlug = getGithubRepo();  if !repoSlug: log no_repo, fail
    etag = skipEtagCache ? null : state.lastKnownChecksum
    result = await fetchTeamMemory(state, repoSlug, etag)      # up to 4 attempts via getRetryDelay
    on 304 (notModified): return success with notModified=true
    on 404 (isEmpty)    : state.lastKnownChecksum = null; return success
    on body parse fail  : skipRetry=true, errorType='parse'
    on 200:
        responseChecksum = parsed.data.checksum
                        || header.etag.replace(/^"|"$/g,'')
        if responseChecksum: state.lastKnownChecksum = responseChecksum
        entries          = parsed.data.content.entries
        responseChecksums= parsed.data.content.entryChecksums
        state.serverChecksums.clear()
        for [k,h] in responseChecksums?: state.serverChecksums.set(k, h)
        filesWritten = await writeRemoteEntriesToLocal(entries)
        if filesWritten > 0: import('utils/claudemd').clearMemoryFileCaches()
        log tengu_team_mem_sync_pull
        return { success: true, filesWritten, entryCount: |entries| }
```

`fetchTeamMemoryOnce` retry policy: up to `MAX_RETRIES = 3` with delay
`getRetryDelay(attempt)`; `skipRetry: true` short-circuits (auth or parse).

`writeRemoteEntriesToLocal` (`:689-755`):
- For each entry, `validateTeamMemKey(relPath)` → may throw `PathTraversalError` (skipped with warning).
- `Buffer.byteLength(content,'utf8') > MAX_FILE_SIZE_BYTES` (250_000): skip.
- Compare-vs-disk: if existing content equals incoming, return false (no
  write — preserves mtime).
- Else `mkdir(parent, recursive: true)` + `writeFile(path, content, 'utf8')`,
  return true. Counts true returns.

### 5.11 teamMemorySync — push (`:889-1146`)

```
pushTeamMemory(state):
    auth/repo guards (same as pull)
    localRead = readLocalTeamMemory(state.serverMaxEntries)        # walks teamDir
    entries        = localRead.entries
    skippedSecrets = localRead.skippedSecrets
    if skippedSecrets.length > 0:
        logForDebugging warn
        logEvent('tengu_team_mem_secret_skipped', { file_count, rule_ids })
    localHashes = Map<key, hashContent(content)>                    # sha256:hex

    sawConflict = false
    for conflictAttempt = 0 .. MAX_CONFLICT_RETRIES (=2):
        delta = {}
        for [k, lh] in localHashes:
            if state.serverChecksums.get(k) !== lh:
                delta[k] = entries[k]
        if |delta|==0:                                             # nothing to upload
            log success; return
        batches = batchDeltaByBytes(delta)                         # see §5.12
        filesUploaded = 0; result = undefined
        for batch in batches:
            result = await uploadTeamMemory(state, repoSlug, batch, state.lastKnownChecksum)
            if !result.success: break
            for k in keys(batch): state.serverChecksums.set(k, localHashes.get(k))
            filesUploaded += |batch|
        if result.success:
            log info; logEvent push success; return
        if !result.conflict:                                       # non-412 fail
            if result.serverMaxEntries !== undefined:
                state.serverMaxEntries = result.serverMaxEntries   # cache for next push
            log push failed; return failure (filesUploaded may be > 0)
        # 412 conflict
        sawConflict = true
        if conflictAttempt >= MAX_CONFLICT_RETRIES:
            log give-up; return conflict failure
        conflictRetries++
        probe = await fetchTeamMemoryHashes(state, repoSlug)       # GET …&view=hashes
        if !probe.success || !probe.entryChecksums:
            log; return conflict failure
        state.serverChecksums.clear()
        for [k,h] in probe.entryChecksums: state.serverChecksums.set(k,h)
    log; return generic failure
```

`uploadTeamMemory` (`:462-553`):
- Headers: `Authorization: Bearer …`, `anthropic-beta: OAUTH_BETA_HEADER`,
  `User-Agent: getClaudeCodeUserAgent()`, `Content-Type: application/json`,
  `If-Match: "<ifMatchChecksum without quotes>"` if provided.
- `validateStatus`: 200 or 412 only; anything else throws into the catch.
- 412 → `{ success: false, conflict: true, error: 'ETag mismatch' }`.
- 200 → updates `state.lastKnownChecksum` from `response.data.checksum`.
- 413 with structured body → parse `TeamMemoryTooManyEntriesSchema`; populate
  `serverErrorCode`, `serverMaxEntries`, `serverReceivedEntries`.

### 5.12 teamMemorySync — `batchDeltaByBytes` (`:426-460`)

```
keys = sorted(Object.keys(delta))
EMPTY_BODY_BYTES = byteLength('{"entries":{}}')
entryBytes(k,v) = byteLength(jsonStringify(k)) + byteLength(jsonStringify(v)) + 2  # ':' + ','
greedy bin-pack:
  if currentBytes + entryBytes > MAX_PUT_BODY_BYTES (200_000) and current is non-empty:
      flush current to batches
      reset
  add entry
flush last
return batches
```

`MAX_FILE_SIZE_BYTES = 250_000` is enforced earlier in
`readLocalTeamMemory` and `writeRemoteEntriesToLocal`; a file
`MAX_FILE_SIZE_BYTES`-bytes long lands in its own solo batch (above the
200K soft cap but under the gateway threshold).

### 5.13 teamMemorySync — `readLocalTeamMemory` (`:567-673`)

```
walkDir(teamDir, recursive Promise.all):
  for each file:
    if stat.size > MAX_FILE_SIZE_BYTES: skip with info log
    content   = readFile(utf8)
    relPath   = relative(teamDir, fullPath).replaceAll('\\', '/')
    matches   = scanForSecrets(content)
    if matches.length > 0:
        push first match to skippedSecrets({ path, ruleId, label })
        warn-log "skipping {relPath} — detected {label}"
        continue
    entries[relPath] = content
on ENOENT/EACCES/EPERM at any level: silently swallow

# Truncate only if a server max_entries has been learned (state.serverMaxEntries):
keys = sorted(Object.keys(entries))
if state.serverMaxEntries !== null and keys.length > state.serverMaxEntries:
    dropped = keys[state.serverMaxEntries:]
    warn-log; logEvent('tengu_team_mem_entries_capped',
                       { total_entries, dropped_count, max_entries })
    keep first N keys
return { entries, skippedSecrets }
```

### 5.14 teamMemorySync — watcher (`watcher.ts:84-318`)

```
startTeamMemoryWatcher():
    if !feature('TEAMMEM'): return
    if !isTeamMemoryEnabled() || !isTeamMemorySyncAvailable(): return
    repoSlug = getGithubRepo();  if !repoSlug: return     # github.com only
    syncState = createSyncState()
    try:
        pullResult = pullTeamMemory(syncState)            # initial pull BEFORE watcher
        initialPullSuccess = pullResult.success
        serverHasContent   = pullResult.entryCount > 0
        initialFilesPulled = pullResult.filesWritten ?? 0
    catch: warn
    await startFileWatcher(getTeamMemPath())
    logEvent('tengu_team_mem_sync_started', {
        initial_pull_success, initial_files_pulled,
        watcher_started: true, server_has_content })

startFileWatcher(teamDir):
    if watcherStarted: return
    watcherStarted = true
    mkdir(teamDir, recursive: true)
    watcher = fs.watch(teamDir, { persistent: true, recursive: true }, (_evt, filename) =>
        if filename === null:                schedulePush();         return
        if pushSuppressedReason !== null:
            stat(join(teamDir, filename))
                .catch (err: NodeJS.ErrnoException) =>
                    if err.code !== 'ENOENT': return                  # ignore
                    info-log "unlink cleared suppression"
                    pushSuppressedReason = null
                    schedulePush()
            return
        schedulePush()
    )
    watcher.on('error', warn)
    registerCleanup(async () => stopTeamMemoryWatcher())

schedulePush():
    if pushSuppressedReason !== null: return
    hasPendingChanges = true
    clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() =>
        if pushInProgress: schedulePush(); return
        currentPushPromise = executePush()
    , DEBOUNCE_MS)                                                    # 2000ms

executePush():
    if !syncState: return
    pushInProgress = true
    try:
        result = await pushTeamMemory(syncState)
        if result.success: hasPendingChanges = false
        if result.success && filesUploaded>0: info-log
        else if !result.success:
            warn-log
            if isPermanentFailure(result) && pushSuppressedReason === null:
                pushSuppressedReason = (httpStatus !== undefined)
                                      ? `http_${httpStatus}`
                                      : (errorType ?? 'unknown')
                logEvent('tengu_team_mem_push_suppressed', {reason, status?})
    catch e: warn
    finally: pushInProgress=false; currentPushPromise=null

isPermanentFailure(r):
    if r.errorType in {no_oauth, no_repo}:                  return true
    if r.httpStatus in [400..500) && r.httpStatus not in {409, 429}: return true
    return false

stopTeamMemoryWatcher():
    clearTimeout(debounceTimer); watcher?.close()
    if currentPushPromise: await it (swallow errors)
    if hasPendingChanges && syncState && pushSuppressedReason===null:
        await pushTeamMemory(syncState).catch(()=>{})       # best-effort

notifyTeamMemoryWrite():
    if !syncState: return
    schedulePush()
```

**Sync trigger map (causality, all entry points to push/pull):**

| # | Trigger | Site | Path → effect | Notes |
|---|---|---|---|---|
| 1 | Initial pull on session start | `setup.ts:367` (lazy import of `watcher.startTeamMemoryWatcher`, gated on `feature('TEAMMEM')`) | `startTeamMemoryWatcher` → `pullTeamMemory(syncState)` once before installing watcher | Synchronous-await on first pull; watcher does not start if pull throws (caught + warned, watcher still starts) |
| 2 | Filesystem change (debounced) | `fs.watch({recursive: true})` registered in `watcher.ts:179-208` | callback → `schedulePush()` → 2000 ms debounce → `executePush()` → `pushTeamMemory` | All change types collapsed (add/change/unlink); `filename === null` path also schedules; ENOENT on unlink can clear `pushSuppressedReason` |
| 3 | PostToolUse explicit notify | `utils/sessionFileAccessHooks.ts:201,205` after FileEdit / FileWrite | `notifyTeamMemoryWrite()` → `schedulePush()` | Covers same-tick races where `fs.watch` may miss the write; **does NOT bypass `pushSuppressedReason`** — only fs-watch unlink ENOENT clears suppression (see §4.4 nit) |
| 4 | Shutdown flush | `stopTeamMemoryWatcher` (`watcher.ts:327-352`) via `registerCleanup(...)` (`:228`) | clear debounce, close watcher, await in-flight push, best-effort final `pushTeamMemory` if `hasPendingChanges` and not suppressed | Bounded by the cleanup-registry budget |

`utils/sessionFileAccessHooks.ts` is owned by spec 41; the
`notifyTeamMemoryWrite` callsite is the contract surface between 41 and 29.

### 5.15 secretScanner — `scanForSecrets` (`:277-294`)

Lazy compile into `compiledRules` on first call; iterate; first hit per rule
ID dedupes via `seen: Set<string>`. Returns `[{ ruleId, label }]` — never
returns matched text. `redactSecrets` (`:312-324`) compiles a separate
`redactRules` (always-`g` flag) and replaces only the captured group, leaving
boundary chars intact.

`ruleIdToLabel` (`:243-268`) splits on `-`, applies special-case map, else
capitalize.

### 5.16 secretGuard — `checkTeamMemSecrets` (`teamMemSecretGuard.ts:15-44`)

```
if feature('TEAMMEM'):
    isTeamMemPath = lazy require memdir/teamMemPaths.isTeamMemPath
    scanForSecrets= lazy require ./secretScanner.scanForSecrets
    if !isTeamMemPath(filePath): return null
    matches = scanForSecrets(content)
    if !matches.length: return null
    labels = matches.map(m=>m.label).join(', ')
    return `Content contains potential secrets (${labels}) and cannot be written to team memory. Team memory is shared with all repository collaborators. Remove the sensitive content and try again.`
return null
```

Callable unconditionally — feature flag is internal so call sites in
FileWriteTool/FileEditTool need no flag guard.

---

## 6. Verbatim Assets

### 6.1 Memory-mechanics system prompt (`memdir/memdir.ts:loadMemoryPrompt → buildMemoryLines`)

`loadMemoryPrompt` returns one of three branches:

1. **KAIROS daily-log mode** (`:432-438`) when `feature('KAIROS') && autoEnabled && getKairosActive()` → `buildAssistantDailyLogPrompt(skipIndex)`.
2. **Combined auto+team mode** (`:448-473`) when `feature('TEAMMEM') && teamMemPaths.isTeamMemoryEnabled()` → `teamMemPrompts.buildCombinedMemoryPrompt(extraGuidelines, skipIndex)` (owned by 40).
3. **Auto-only mode** (`:475-490`) when `autoEnabled` → `buildMemoryLines('auto memory', autoDir, extraGuidelines, skipIndex).join('\n')`.

#### `buildMemoryLines` (memdir.ts:199-266) — full text

```
# {displayName}

You have a persistent, file-based memory system at `{memoryDir}`. This directory already exists — write to it directly with the Write tool (do not run mkdir or check for its existence).

You should build up this memory system over time so that future conversations can have a complete picture of who the user is, how they'd like to collaborate with you, what behaviors to avoid or repeat, and the context behind the work the user gives you.

If the user explicitly asks you to remember something, save it immediately as whichever type fits best. If they ask you to forget something, find and remove the relevant entry.

{TYPES_SECTION_INDIVIDUAL}        ← see §6.2
{WHAT_NOT_TO_SAVE_SECTION}        ← see §6.2

{howToSave}                        ← see §6.1.1

{WHEN_TO_ACCESS_SECTION}          ← see §6.2

{TRUSTING_RECALL_SECTION}         ← see §6.2

## Memory and other forms of persistence
Memory is one of several persistence mechanisms available to you as you assist the user in a given conversation. The distinction is often that memory can be recalled in future conversations and should not be used for persisting information that is only useful within the scope of the current conversation.
- When to use or update a plan instead of memory: If you are about to start a non-trivial implementation task and would like to reach alignment with the user on your approach you should use a Plan rather than saving this information to memory. Similarly, if you already have a plan within the conversation and you have changed your approach persist that change by updating the plan rather than saving a memory.
- When to use or update tasks instead of memory: When you need to break your work in current conversation into discrete steps or keep track of your progress use tasks instead of saving to memory. Tasks are great for persisting information about the work that needs to be done in the current conversation, but memory should be reserved for information that will be useful in future conversations.

{extraGuidelines ?? []}            ← from CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES

{buildSearchingPastContextSection(memoryDir)}    ← see §6.1.3
```

#### 6.1.1 `howToSave` (`memdir.ts:205-234`)

`skipIndex` true (GB `tengu_moth_copse`):

```
## How to save memories

Write each memory to its own file (e.g., `user_role.md`, `feedback_testing.md`) using this frontmatter format:

{MEMORY_FRONTMATTER_EXAMPLE}      ← see §6.2

- Keep the name, description, and type fields in memory files up-to-date with the content
- Organize memory semantically by topic, not chronologically
- Update or remove memories that turn out to be wrong or outdated
- Do not write duplicate memories. First check if there is an existing memory you can update before writing a new one.
```

`skipIndex` false (default):

```
## How to save memories

Saving a memory is a two-step process:

**Step 1** — write the memory to its own file (e.g., `user_role.md`, `feedback_testing.md`) using this frontmatter format:

{MEMORY_FRONTMATTER_EXAMPLE}

**Step 2** — add a pointer to that file in `MEMORY.md`. `MEMORY.md` is an index, not a memory — each entry should be one line, under ~150 characters: `- [Title](file.md) — one-line hook`. It has no frontmatter. Never write memory content directly into `MEMORY.md`.

- `MEMORY.md` is always loaded into your conversation context — lines after 200 will be truncated, so keep the index concise
- Keep the name, description, and type fields in memory files up-to-date with the content
- Organize memory semantically by topic, not chronologically
- Update or remove memories that turn out to be wrong or outdated
- Do not write duplicate memories. First check if there is an existing memory you can update before writing a new one.
```

#### 6.1.2 `buildAssistantDailyLogPrompt` (`memdir.ts:327-370`) — KAIROS branch

```
# auto memory

You have a persistent, file-based memory system found at: `{memoryDir}`

This session is long-lived. As you work, record anything worth remembering by **appending** to today's daily log file:

`{memoryDir}/logs/YYYY/MM/YYYY-MM-DD.md`

Substitute today's date (from `currentDate` in your context) for `YYYY-MM-DD`. When the date rolls over mid-session, start appending to the new day's file.

Write each entry as a short timestamped bullet. Create the file (and parent directories) on first write if it does not exist. Do not rewrite or reorganize the log — it is append-only. A separate nightly process distills these logs into `MEMORY.md` and topic files.

## What to log
- User corrections and preferences ("use bun, not npm"; "stop summarizing diffs")
- Facts about the user, their role, or their goals
- Project context that is not derivable from the code (deadlines, incidents, decisions and their rationale)
- Pointers to external systems (dashboards, Linear projects, Slack channels)
- Anything the user explicitly asks you to remember

{WHAT_NOT_TO_SAVE_SECTION}

## MEMORY.md       (omitted entirely when skipIndex=true)
`MEMORY.md` is the distilled index (maintained nightly from your logs) and is loaded into your context automatically. Read it for orientation, but do not edit it directly — record new information in today's log instead.

{buildSearchingPastContextSection(memoryDir)}
```

#### 6.1.3 `buildSearchingPastContextSection` (`memdir.ts:375-407`)

Returns `[]` when GB `tengu_coral_fern` is false. Otherwise, with
`embedded = hasEmbeddedSearchTools() || isReplModeEnabled()`:

```
## Searching past context

When looking for past context:
1. Search topic files in your memory directory:
```
{memSearch}
```
2. Session transcript logs (last resort — large files, slow):
```
{transcriptSearch}
```
Use narrow search terms (error messages, file paths, function names) rather than broad keywords.
```

`memSearch` body alternates:
- embedded: `grep -rn "<search term>" {autoMemDir} --include="*.md"`
- non-embedded: `Grep with pattern="<search term>" path="{autoMemDir}" glob="*.md"`

Same pattern for `transcriptSearch` against `{getProjectDir(getOriginalCwd())}/` with `glob="*.jsonl"`.

#### 6.1.4 Truncation suffix (`memdir.ts:97`)

When `MEMORY.md` is truncated by `truncateEntrypointContent` (`:57-103`):

```
\n\n> WARNING: MEMORY.md is {reason}. Only part of it was loaded. Keep index entries to one line under ~200 chars; move detail into topic files.
```

`reason` is one of (`memdir.ts:87-92`):
- `{formatFileSize(byteCount)} (limit: {formatFileSize(MAX_ENTRYPOINT_BYTES)}) — index entries are too long` (byte-only)
- `{lineCount} lines (limit: {MAX_ENTRYPOINT_LINES})` (line-only)
- `{lineCount} lines and {formatFileSize(byteCount)}` (both)

### 6.2 Four-type taxonomy (`memdir/memoryTypes.ts`)

```ts
// memoryTypes.ts:14-19
export const MEMORY_TYPES = ['user', 'feedback', 'project', 'reference'] as const
```

#### 6.2.1 `TYPES_SECTION_INDIVIDUAL` (memoryTypes.ts:113-178)

```
## Types of memory

There are several discrete types of memory that you can store in your memory system:

<types>
<type>
    <name>user</name>
    <description>Contain information about the user's role, goals, responsibilities, and knowledge. Great user memories help you tailor your future behavior to the user's preferences and perspective. Your goal in reading and writing these memories is to build up an understanding of who the user is and how you can be most helpful to them specifically. For example, you should collaborate with a senior software engineer differently than a student who is coding for the very first time. Keep in mind, that the aim here is to be helpful to the user. Avoid writing memories about the user that could be viewed as a negative judgement or that are not relevant to the work you're trying to accomplish together.</description>
    <when_to_save>When you learn any details about the user's role, preferences, responsibilities, or knowledge</when_to_save>
    <how_to_use>When your work should be informed by the user's profile or perspective. For example, if the user is asking you to explain a part of the code, you should answer that question in a way that is tailored to the specific details that they will find most valuable or that helps them build their mental model in relation to domain knowledge they already have.</how_to_use>
    <examples>
    user: I'm a data scientist investigating what logging we have in place
    assistant: [saves user memory: user is a data scientist, currently focused on observability/logging]

    user: I've been writing Go for ten years but this is my first time touching the React side of this repo
    assistant: [saves user memory: deep Go expertise, new to React and this project's frontend — frame frontend explanations in terms of backend analogues]
    </examples>
</type>
<type>
    <name>feedback</name>
    <description>Guidance the user has given you about how to approach work — both what to avoid and what to keep doing. These are a very important type of memory to read and write as they allow you to remain coherent and responsive to the way you should approach work in the project. Record from failure AND success: if you only save corrections, you will avoid past mistakes but drift away from approaches the user has already validated, and may grow overly cautious.</description>
    <when_to_save>Any time the user corrects your approach ("no not that", "don't", "stop doing X") OR confirms a non-obvious approach worked ("yes exactly", "perfect, keep doing that", accepting an unusual choice without pushback). Corrections are easy to notice; confirmations are quieter — watch for them. In both cases, save what is applicable to future conversations, especially if surprising or not obvious from the code. Include *why* so you can judge edge cases later.</when_to_save>
    <how_to_use>Let these memories guide your behavior so that the user does not need to offer the same guidance twice.</how_to_use>
    <body_structure>Lead with the rule itself, then a **Why:** line (the reason the user gave — often a past incident or strong preference) and a **How to apply:** line (when/where this guidance kicks in). Knowing *why* lets you judge edge cases instead of blindly following the rule.</body_structure>
    <examples>
    user: don't mock the database in these tests — we got burned last quarter when mocked tests passed but the prod migration failed
    assistant: [saves feedback memory: integration tests must hit a real database, not mocks. Reason: prior incident where mock/prod divergence masked a broken migration]

    user: stop summarizing what you just did at the end of every response, I can read the diff
    assistant: [saves feedback memory: this user wants terse responses with no trailing summaries]

    user: yeah the single bundled PR was the right call here, splitting this one would've just been churn
    assistant: [saves feedback memory: for refactors in this area, user prefers one bundled PR over many small ones. Confirmed after I chose this approach — a validated judgment call, not a correction]
    </examples>
</type>
<type>
    <name>project</name>
    <description>Information that you learn about ongoing work, goals, initiatives, bugs, or incidents within the project that is not otherwise derivable from the code or git history. Project memories help you understand the broader context and motivation behind the work the user is doing within this working directory.</description>
    <when_to_save>When you learn who is doing what, why, or by when. These states change relatively quickly so try to keep your understanding of this up to date. Always convert relative dates in user messages to absolute dates when saving (e.g., "Thursday" → "2026-03-05"), so the memory remains interpretable after time passes.</when_to_save>
    <how_to_use>Use these memories to more fully understand the details and nuance behind the user's request and make better informed suggestions.</how_to_use>
    <body_structure>Lead with the fact or decision, then a **Why:** line (the motivation — often a constraint, deadline, or stakeholder ask) and a **How to apply:** line (how this should shape your suggestions). Project memories decay fast, so the why helps future-you judge whether the memory is still load-bearing.</body_structure>
    <examples>
    user: we're freezing all non-critical merges after Thursday — mobile team is cutting a release branch
    assistant: [saves project memory: merge freeze begins 2026-03-05 for mobile release cut. Flag any non-critical PR work scheduled after that date]

    user: the reason we're ripping out the old auth middleware is that legal flagged it for storing session tokens in a way that doesn't meet the new compliance requirements
    assistant: [saves project memory: auth middleware rewrite is driven by legal/compliance requirements around session token storage, not tech-debt cleanup — scope decisions should favor compliance over ergonomics]
    </examples>
</type>
<type>
    <name>reference</name>
    <description>Stores pointers to where information can be found in external systems. These memories allow you to remember where to look to find up-to-date information outside of the project directory.</description>
    <when_to_save>When you learn about resources in external systems and their purpose. For example, that bugs are tracked in a specific project in Linear or that feedback can be found in a specific Slack channel.</when_to_save>
    <how_to_use>When the user references an external system or information that may be in an external system.</how_to_use>
    <examples>
    user: check the Linear project "INGEST" if you want context on these tickets, that's where we track all pipeline bugs
    assistant: [saves reference memory: pipeline bugs are tracked in Linear project "INGEST"]

    user: the Grafana board at grafana.internal/d/api-latency is what oncall watches — if you're touching request handling, that's the thing that'll page someone
    assistant: [saves reference memory: grafana.internal/d/api-latency is the oncall latency dashboard — check it when editing request-path code]
    </examples>
</type>
</types>
```

#### 6.2.2 `TYPES_SECTION_COMBINED` (memoryTypes.ts:37-106)

Identical structure with an inserted preamble:

```
There are several discrete types of memory that you can store in your memory system. Each type below declares a <scope> of `private`, `team`, or guidance for choosing between the two.
```

…and `<scope>` lines per type:

| type | scope |
|---|---|
| user | `always private` |
| feedback | `default to private. Save as team only when the guidance is clearly a project-wide convention that every contributor should follow (e.g., a testing policy, a build invariant), not a personal style preference.` |
| project | `private or team, but strongly bias toward team` |
| reference | `usually team` |

(Examples are reworked to use `[saves private user memory: …]` / `[saves team feedback memory: …]` etc., and `feedback` adds the line `Before saving a private feedback memory, check that it doesn't contradict a team feedback memory — if it does, either don't save it or note the override explicitly.`)

#### 6.2.3 `WHAT_NOT_TO_SAVE_SECTION` (memoryTypes.ts:183-195)

```
## What NOT to save in memory

- Code patterns, conventions, architecture, file paths, or project structure — these can be derived by reading the current project state.
- Git history, recent changes, or who-changed-what — `git log` / `git blame` are authoritative.
- Debugging solutions or fix recipes — the fix is in the code; the commit message has the context.
- Anything already documented in CLAUDE.md files.
- Ephemeral task details: in-progress work, temporary state, current conversation context.

These exclusions apply even when the user explicitly asks you to save. If they ask you to save a PR list or activity summary, ask what was *surprising* or *non-obvious* about it — that is the part worth keeping.
```

#### 6.2.4 `WHEN_TO_ACCESS_SECTION` (memoryTypes.ts:201-222)

```
## When to access memories
- When memories seem relevant, or the user references prior-conversation work.
- You MUST access memory when the user explicitly asks you to check, recall, or remember.
- If the user says to *ignore* or *not use* memory: proceed as if MEMORY.md were empty. Do not apply remembered facts, cite, compare against, or mention memory content.
- Memory records can become stale over time. Use memory as context for what was true at a given point in time. Before answering the user or building assumptions based solely on information in memory records, verify that the memory is still correct and up-to-date by reading the current state of the files or resources. If a recalled memory conflicts with current information, trust what you observe now — and update or remove the stale memory rather than acting on it.
```

#### 6.2.5 `TRUSTING_RECALL_SECTION` (memoryTypes.ts:240-256)

```
## Before recommending from memory

A memory that names a specific function, file, or flag is a claim that it existed *when the memory was written*. It may have been renamed, removed, or never merged. Before recommending it:

- If the memory names a file path: check the file exists.
- If the memory names a function or flag: grep for it.
- If the user is about to act on your recommendation (not just asking about history), verify first.

"The memory says X exists" is not the same as "X exists now."

A memory that summarizes repo state (activity logs, architecture snapshots) is frozen in time. If the user asks about *recent* or *current* state, prefer `git log` or reading the code over recalling the snapshot.
```

#### 6.2.6 `MEMORY_FRONTMATTER_EXAMPLE` (memoryTypes.ts:261-271)

```
```markdown
---
name: {{memory name}}
description: {{one-line description — used to decide relevance in future conversations, so be specific}}
type: {{user, feedback, project, reference}}
---

{{memory content — for feedback/project types, structure as: rule/fact, then **Why:** and **How to apply:** lines}}
```
```

(Type list is interpolated from `MEMORY_TYPES.join(', ')` at module load.)

### 6.3 Auto-memory extraction prompt (`extractMemories/prompts.ts`)

#### 6.3.1 `opener(newMessageCount, existingMemories)` (`prompts.ts:29-44`)

```
You are now acting as the memory extraction subagent. Analyze the most recent ~{newMessageCount} messages above and use them to update your persistent memory systems.

Available tools: Read, Grep, Glob, read-only Bash (ls/find/cat/stat/wc/head/tail and similar), and Edit/Write for paths inside the memory directory only. Bash rm is not permitted. All other tools — MCP, Agent, write-capable Bash, etc — will be denied.

You have a limited turn budget. Edit requires a prior Read of the same file, so the efficient strategy is: turn 1 — issue all Read calls in parallel for every file you might update; turn 2 — issue all Write/Edit calls in parallel. Do not interleave reads and writes across multiple turns.

You MUST only use content from the last ~{newMessageCount} messages to update your persistent memories. Do not waste any turns attempting to investigate or verify that content further — no grepping source files, no reading code to confirm a pattern exists, no git commands.{manifest}
```

`manifest` is empty when `existingMemories.length === 0`, otherwise:

```
\n\n## Existing memory files\n\n{existingMemories}\n\nCheck this list before writing — update an existing file rather than creating a duplicate.
```

(Tool names interpolate from the actual `*_TOOL_NAME` constants at runtime —
shown above as their canonical strings `Read`, `Grep`, `Glob`, `Bash`, `Edit`,
`Write`.)

#### 6.3.2 `buildExtractAutoOnlyPrompt` (`prompts.ts:50-94`) — full assembled body

```
{opener(newMessageCount, existingMemories)}

If the user explicitly asks you to remember something, save it immediately as whichever type fits best. If they ask you to forget something, find and remove the relevant entry.

{TYPES_SECTION_INDIVIDUAL}        ← §6.2.1
{WHAT_NOT_TO_SAVE_SECTION}        ← §6.2.3

{howToSave}                        ← §6.1.1 (skipIndex variants)
```

#### 6.3.3 `buildExtractCombinedPrompt` (`prompts.ts:101-154`)

When `!feature('TEAMMEM')`, falls back to `buildExtractAutoOnlyPrompt`. Else:

```
{opener(newMessageCount, existingMemories)}

If the user explicitly asks you to remember something, save it immediately as whichever type fits best. If they ask you to forget something, find and remove the relevant entry.

{TYPES_SECTION_COMBINED}          ← §6.2.2
{WHAT_NOT_TO_SAVE_SECTION}
- You MUST avoid saving sensitive data within shared team memories. For example, never save API keys or user credentials.

{howToSave_combined}              ← below
```

`howToSave_combined`, `skipIndex=true`:

```
## How to save memories

Write each memory to its own file in the chosen directory (private or team, per the type's scope guidance) using this frontmatter format:

{MEMORY_FRONTMATTER_EXAMPLE}

- Organize memory semantically by topic, not chronologically
- Update or remove memories that turn out to be wrong or outdated
- Do not write duplicate memories. First check if there is an existing memory you can update before writing a new one.
```

`howToSave_combined`, `skipIndex=false`:

```
## How to save memories

Saving a memory is a two-step process:

**Step 1** — write the memory to its own file in the chosen directory (private or team, per the type's scope guidance) using this frontmatter format:

{MEMORY_FRONTMATTER_EXAMPLE}

**Step 2** — add a pointer to that file in the same directory's `MEMORY.md`. Each directory (private and team) has its own `MEMORY.md` index — each entry should be one line, under ~150 characters: `- [Title](file.md) — one-line hook`. They have no frontmatter. Never write memory content directly into a `MEMORY.md`.

- Both `MEMORY.md` indexes are loaded into your system prompt — lines after 200 will be truncated, so keep them concise
- Organize memory semantically by topic, not chronologically
- Update or remove memories that turn out to be wrong or outdated
- Do not write duplicate memories. First check if there is an existing memory you can update before writing a new one.
```

### 6.4 SessionMemory templates and prompts (`SessionMemory/prompts.ts`)

#### 6.4.1 `DEFAULT_SESSION_MEMORY_TEMPLATE` (`prompts.ts:11-41`)

```

# Session Title
_A short and distinctive 5-10 word descriptive title for the session. Super info dense, no filler_

# Current State
_What is actively being worked on right now? Pending tasks not yet completed. Immediate next steps._

# Task specification
_What did the user ask to build? Any design decisions or other explanatory context_

# Files and Functions
_What are the important files? In short, what do they contain and why are they relevant?_

# Workflow
_What bash commands are usually run and in what order? How to interpret their output if not obvious?_

# Errors & Corrections
_Errors encountered and how they were fixed. What did the user correct? What approaches failed and should not be tried again?_

# Codebase and System Documentation
_What are the important system components? How do they work/fit together?_

# Learnings
_What has worked well? What has not? What to avoid? Do not duplicate items from other sections_

# Key results
_If the user asked a specific output such as an answer to a question, a table, or other document, repeat the exact result here_

# Worklog
_Step by step, what was attempted, done? Very terse summary for each step_
```

`loadSessionMemoryTemplate` (`prompts.ts:86-104`) reads
`{getClaudeConfigHomeDir()}/session-memory/config/template.md` if present,
else returns the default; ENOENT silently falls back to default, other errors
log via `logError`.

#### 6.4.2 `getDefaultUpdatePrompt()` (`prompts.ts:43-81`) — full text

```
IMPORTANT: This message and these instructions are NOT part of the actual user conversation. Do NOT include any references to "note-taking", "session notes extraction", or these update instructions in the notes content.

Based on the user conversation above (EXCLUDING this note-taking instruction message as well as system prompt, claude.md entries, or any past session summaries), update the session notes file.

The file {{notesPath}} has already been read for you. Here are its current contents:
<current_notes_content>
{{currentNotes}}
</current_notes_content>

Your ONLY task is to use the Edit tool to update the notes file, then stop. You can make multiple edits (update every section as needed) - make all Edit tool calls in parallel in a single message. Do not call any other tools.

CRITICAL RULES FOR EDITING:
- The file must maintain its exact structure with all sections, headers, and italic descriptions intact
-- NEVER modify, delete, or add section headers (the lines starting with '#' like # Task specification)
-- NEVER modify or delete the italic _section description_ lines (these are the lines in italics immediately following each header - they start and end with underscores)
-- The italic _section descriptions_ are TEMPLATE INSTRUCTIONS that must be preserved exactly as-is - they guide what content belongs in each section
-- ONLY update the actual content that appears BELOW the italic _section descriptions_ within each existing section
-- Do NOT add any new sections, summaries, or information outside the existing structure
- Do NOT reference this note-taking process or instructions anywhere in the notes
- It's OK to skip updating a section if there are no substantial new insights to add. Do not add filler content like "No info yet", just leave sections blank/unedited if appropriate.
- Write DETAILED, INFO-DENSE content for each section - include specifics like file paths, function names, error messages, exact commands, technical details, etc.
- For "Key results", include the complete, exact output the user requested (e.g., full table, full answer, etc.)
- Do not include information that's already in the CLAUDE.md files included in the context
- Keep each section under ~2000 tokens/words - if a section is approaching this limit, condense it by cycling out less important details while preserving the most critical information
- Focus on actionable, specific information that would help someone understand or recreate the work discussed in the conversation
- IMPORTANT: Always update "Current State" to reflect the most recent work - this is critical for continuity after compaction

Use the Edit tool with file_path: {{notesPath}}

STRUCTURE PRESERVATION REMINDER:
Each section has TWO parts that must be preserved exactly as they appear in the current file:
1. The section header (line starting with #)
2. The italic description line (the _italicized text_ immediately after the header - this is a template instruction)

You ONLY update the actual content that comes AFTER these two preserved lines. The italic description lines starting and ending with underscores are part of the template structure, NOT content to be edited or removed.

REMEMBER: Use the Edit tool in parallel and stop. Do not continue after the edits. Only include insights from the actual user conversation, never from these note-taking instructions. Do not delete or change section headers or italic _section descriptions_.
```

The literal `~${MAX_SECTION_LENGTH}` interpolates to `~2000`.
`loadSessionMemoryPrompt` (`prompts.ts:111-129`) reads
`{getClaudeConfigHomeDir()}/session-memory/config/prompt.md` if present, else
returns the above default. Variable substitution is single-pass via
`/\{\{(\w+)\}\}/g`, replacing only known keys (`prompts.ts:200-213`).

#### 6.4.3 `generateSectionReminders` text (`prompts.ts:164-196`)

`overBudget` block:
```
\n\nCRITICAL: The session memory file is currently ~{totalTokens} tokens, which exceeds the maximum of 12000 tokens. You MUST condense the file to fit within this budget. Aggressively shorten oversized sections by removing less important details, merging related items, and summarizing older entries. Prioritize keeping "Current State" and "Errors & Corrections" accurate and detailed.
```

Oversized list, when `overBudget`:
```
\n\nOversized sections to condense:\n{- "{section}" is ~{tokens} tokens (limit: 2000)}*
```

Oversized list, when `!overBudget`:
```
\n\nIMPORTANT: The following sections exceed the per-section limit and MUST be condensed:\n{- "{section}" is ~{tokens} tokens (limit: 2000)}*
```

Truncation marker (`prompts.ts:322`):
```
\n[... section truncated for length ...]
```

### 6.5 teamMemorySync Zod schemas (`types.ts:16-57`)

```ts
// types.ts:16-24
export const TeamMemoryContentSchema = lazySchema(() =>
  z.object({
    entries: z.record(z.string(), z.string()),
    entryChecksums: z.record(z.string(), z.string()).optional(),
  }),
)

// types.ts:29-38
export const TeamMemoryDataSchema = lazySchema(() =>
  z.object({
    organizationId: z.string(),
    repo: z.string(),
    version: z.number(),
    lastModified: z.string(),                 // ISO 8601
    checksum: z.string(),                     // 'sha256:<hex>'
    content: TeamMemoryContentSchema(),
  }),
)

// types.ts:47-57
export const TeamMemoryTooManyEntriesSchema = lazySchema(() =>
  z.object({
    error: z.object({
      details: z.object({
        error_code: z.literal('team_memory_too_many_entries'),
        max_entries: z.number().int().positive(),
        received_entries: z.number().int().positive(),
      }),
    }),
  }),
)
```

`SkippedSecretFile`, `TeamMemorySyncFetchResult`, `TeamMemoryHashesResult`,
`TeamMemorySyncPushResult`, `TeamMemorySyncUploadResult` are TS-only types
listed verbatim in §3.3 references.

### 6.6 Constants tables

#### 6.6.1 extractMemories

| Name | Value | Site |
|---|---|---|
| Forked-agent `maxTurns` | `5` | `extractMemories.ts:425` |
| Drain default timeout | `60_000` ms | `:579` |
| Querysource | `'extract_memories'` | `:419` |
| Fork label | `'extract_memories'` | `:420` |
| `skipTranscript` | `true` | `:423` |
| Throttle GB key | `tengu_bramble_lintel`, fallback `null ?? 1` | `:381` |
| Master gate GB key | `tengu_passport_quail`, default `false` | `:536, paths.ts:70` |
| Skip-index GB key | `tengu_moth_copse`, default `false` | `:366` |
| Non-interactive override GB | `tengu_slate_thimble`, default `false` | `paths.ts:75` |
| Past-context section GB | `tengu_coral_fern`, default `false` | `memdir.ts:376` |
| Team-recall enable GB | `tengu_herring_clock`, default `false` | `memdir.ts:503` |

#### 6.6.2 SessionMemory

| Name | Value | Site |
|---|---|---|
| `EXTRACTION_WAIT_TIMEOUT_MS` | `15000` | `sessionMemoryUtils.ts:12` |
| `EXTRACTION_STALE_THRESHOLD_MS` | `60000` (1 min) | `:13` |
| `MAX_SECTION_LENGTH` | `2000` | `prompts.ts:8` |
| `MAX_TOTAL_SESSION_MEMORY_TOKENS` | `12000` | `prompts.ts:9` |
| `DEFAULT_SESSION_MEMORY_CONFIG.minimumMessageTokensToInit` | `10000` | `sessionMemoryUtils.ts:33` |
| `DEFAULT_SESSION_MEMORY_CONFIG.minimumTokensBetweenUpdate` | `5000` | `:34` |
| `DEFAULT_SESSION_MEMORY_CONFIG.toolCallsBetweenUpdates` | `3` | `:35` |
| Memory file `mkdir` mode | `0o700` | `sessionMemory.ts:190` |
| Memory file create mode | `0o600`, flag `'wx'` | `:198-200` |
| Querysource | `'session_memory'` (auto), `'session_memory_manual'` (manual fork label) | `:323`, `:431` |
| Gate GB key | `tengu_session_memory`, default `false` | `:81` |
| Remote config GB key | `tengu_sm_config` | `:89` |
| Sleep poll interval (wait) | `1000` ms | `sessionMemoryUtils.ts:103` |
| Truncation marker | `\n[... section truncated for length ...]` | `prompts.ts:322` |

#### 6.6.3 teamMemorySync

| Name | Value | Site |
|---|---|---|
| `TEAM_MEMORY_SYNC_TIMEOUT_MS` | `30_000` | `index.ts:71` |
| `MAX_FILE_SIZE_BYTES` | `250_000` (per-entry size cap) | `:75` |
| `MAX_PUT_BODY_BYTES` | `200_000` (gateway-aware batch cap) | `:89` |
| `MAX_RETRIES` | `3` | `:90` |
| `MAX_CONFLICT_RETRIES` | `2` | `:91` |
| `DEBOUNCE_MS` | `2000` | `watcher.ts:35` |
| Endpoint base | `process.env.TEAM_MEMORY_SYNC_URL \|\| getOauthConfig().BASE_API_URL` | `index.ts:165` |
| Endpoint path | `/api/claude_code/team_memory?repo={encodeURIComponent(repoSlug)}` | `:166` |
| Hashes-view query suffix | `&view=hashes` | `:326` |
| `validateStatus` (GET) | `200 \|\| 304 \|\| 404` | `:215-216` |
| `validateStatus` (PUT) | `200 \|\| 412` | `:491` |
| `validateStatus` (hashes GET) | `200 \|\| 404` | `:330` |
| Empty body bytes basis | `byteLength('{"entries":{}}')` | `:435` |
| Hash format | `sha256:<hex>` | `:135` |
| Required scopes | `CLAUDE_AI_INFERENCE_SCOPE`, `CLAUDE_AI_PROFILE_SCOPE` | `:155-160` |
| Beta header | `OAUTH_BETA_HEADER` | `:178` |

### 6.7 Event/log identifiers

extractMemories:
- `tengu_extract_memories_gate_disabled` (`extractMemories.ts:539`) — ANT-only, once.
- `tengu_extract_memories_skipped_direct_write` (`:356-358`)
- `tengu_extract_memories_coalesced` (`:561`)
- `tengu_extract_memories_extraction` (`:473-485`)
- `tengu_extract_memories_error` (`:500-502`)
- `tengu_auto_mem_tool_denied` (`:156-158`)

SessionMemory:
- `tengu_session_memory_init` (`sessionMemory.ts:364-366`) — ANT-only.
- `tengu_session_memory_gate_disabled` (`:288`) — ANT-only, once.
- `tengu_session_memory_extraction` (`:332-341`)
- `tengu_session_memory_manual_extraction` (`:436`)
- `tengu_session_memory_loaded` (`sessionMemoryUtils.ts:117-119`)
- `tengu_session_memory_file_read` (`sessionMemory.ts:228-230`)

teamMemorySync:
- `tengu_team_mem_sync_started` (`watcher.ts:298-304`)
- `tengu_team_mem_sync_pull` (`index.ts:1205-1216`)
- `tengu_team_mem_sync_push` (`:1233-1255`)
- `tengu_team_mem_secret_skipped` (`:935-944`)
- `tengu_team_mem_entries_capped` (`:661-665`)
- `tengu_team_mem_push_suppressed` (`watcher.ts:112-116`)

Memdir-side, surfaced by services:
- `tengu_memdir_loaded` (`memdir.ts:174-178, 182`)
- `tengu_memdir_disabled` (`memdir.ts:492-499`)
- `tengu_team_memdir_disabled` (`memdir.ts:503-505`)

Debug-channel `logForDebugging` strings (selected):
- `[autoMem] denied {tool}: {reason}`
- `[extractMemories] starting — {n} new messages, memoryDir={dir}`
- `[extractMemories] finished — {k} files written, cache: read=… create=… input=… ({hit}% hit)`
- `[extractMemories] memories saved: {paths}`, `[extractMemories] no memories saved this run`
- `[extractMemories] error: {error}`
- `[extractMemories] running trailing extraction for stashed context`
- `team-memory-sync: not modified (304)` etc., as in §5.10/5.11
- `team-memory-sync: skipping "{relPath}" — detected {label}`
- `team-memory-watcher: pushed {n} files`
- `team-memory-watcher: suppressing retry until next unlink or session restart ({reason})`

### 6.8 secretScanner rule catalog (`secretScanner.ts:48-224`)

Rule IDs (29 total, ordered as in source):

```
aws-access-token, gcp-api-key, azure-ad-client-secret, digitalocean-pat, digitalocean-access-token,
anthropic-api-key, anthropic-admin-api-key, openai-api-key, huggingface-access-token,
github-pat, github-fine-grained-pat, github-app-token, github-oauth, github-refresh-token,
gitlab-pat, gitlab-deploy-token,
slack-bot-token, slack-user-token, slack-app-token, twilio-api-key, sendgrid-api-token,
npm-access-token, pypi-upload-token, databricks-api-token, hashicorp-tf-api-token,
pulumi-api-token, postman-api-token,
grafana-api-key, grafana-cloud-api-token, grafana-service-account-token,
sentry-user-token, sentry-org-token,
stripe-access-token, shopify-access-token, shopify-shared-secret,
private-key
```

Anthropic prefix is assembled at runtime as `ANT_KEY_PFX = ['sk','ant','api'].join('-')` (`:46`) so the literal string never appears in the bundle. Patterns inline: `\\b(${ANT_KEY_PFX}03-[a-zA-Z0-9_\\-]{93}AA)…` and `\\b(sk-ant-admin01-[a-zA-Z0-9_\\-]{93}AA)…`. Specific `ruleIdToLabel` overrides (`:243-263`):

```
aws→AWS, gcp→GCP, api→API, pat→PAT, ad→AD, tf→TF, oauth→OAuth, npm→NPM,
pypi→PyPI, jwt→JWT, github→GitHub, gitlab→GitLab, openai→OpenAI,
digitalocean→DigitalOcean, huggingface→HuggingFace, hashicorp→HashiCorp,
sendgrid→SendGrid
```

`checkTeamMemSecrets` returns the deny string:

```
Content contains potential secrets ({label1, label2, …}) and cannot be written to team memory. Team memory is shared with all repository collaborators. Remove the sensitive content and try again.
```

### 6.9 `createMemorySavedMessage` deny-default systemContext message

`extractMemories` calls `createMemorySavedMessage(memoryPaths)` (a
`utils/messages.js` helper not owned here) and, when TEAMMEM is on, mutates
`msg.teamCount = teamCount` before passing to `appendSystemMessage` (extracted
verbatim at `extractMemories.ts:491-496`).

---

## 7. Side Effects & I/O

### 7.1 extractMemories

| Effect | Path | Trigger |
|---|---|---|
| Read | `getAutoMemPath()` (memoized) | per `runExtraction` (manifest scan) |
| Spawn | forked agent (`runForkedAgent`) | extraction; querySource `extract_memories` |
| Network | (transitive) Anthropic API for forked agent | extraction |
| Write | files under `getAutoMemPath()` | only via the forked agent's Edit/Write tool calls — the canUseTool denies anything else |
| Process exit hook | drain via `cli/print.ts:968` | non-interactive only |

Env consumed (transitive via memdir/paths): `CLAUDE_CODE_DISABLE_AUTO_MEMORY`,
`CLAUDE_CODE_SIMPLE`, `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_REMOTE_MEMORY_DIR`,
`CLAUDE_COWORK_MEMORY_PATH_OVERRIDE`, `USER_TYPE`,
`CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES` (consumed by `loadMemoryPrompt`,
not extractMemories itself). Permission boundary: `createAutoMemCanUseTool`
narrows the forked-agent tool set as in §5.4.

### 7.2 SessionMemory

| Effect | Path | Trigger |
|---|---|---|
| Mkdir | `getSessionMemoryDir()`, `mode: 0o700` | first extraction |
| Write | `getSessionMemoryPath()` (`'wx'` create + template overwrite when newly created), `mode: 0o600` | first extraction |
| Read | session memory file via `FileReadTool` | every extraction |
| Read (template) | `{getClaudeConfigHomeDir()}/session-memory/config/template.md` | each `loadSessionMemoryTemplate()` call (memoized: no — re-reads each time) |
| Read (prompt) | `{getClaudeConfigHomeDir()}/session-memory/config/prompt.md` | each `loadSessionMemoryPrompt()` call |
| Spawn | forked agent (`session_memory` / `session_memory_manual`) | every extraction |
| Network | (transitive) Anthropic API | every extraction |

Permission boundary: `createMemoryFileCanUseTool` allows only `Edit` on the
exact memoryPath. Trust: ANT once-per-session telemetry is the only
USER_TYPE-gated side effect.

### 7.3 teamMemorySync

| Effect | Path | Trigger |
|---|---|---|
| Mkdir | `getTeamMemPath()` (recursive) | watcher start; `writeRemoteEntriesToLocal` per-key parent |
| Walk + read | recursive `readdir` of `getTeamMemPath()`; per-file `stat` + `readFile` | every push |
| Write | files under `getTeamMemPath()` (after pull) | per-pull when content differs |
| `fs.watch` | `getTeamMemPath()` `{recursive:true, persistent:true}` | session lifetime |
| Network GET | `${BASE_API_URL}/api/claude_code/team_memory?repo=…` | pull / hashes probe |
| Network PUT | same endpoint, body `{entries: …}` | push, per batch |
| Cleanup | `registerCleanup(stopTeamMemoryWatcher)` | session end |
| Memory cache invalidation | dynamic `import('utils/claudemd').clearMemoryFileCaches()` after pull writes | `index.ts:852-855` |

Required headers: `Authorization: Bearer <oauth>`, `anthropic-beta:
{OAUTH_BETA_HEADER}`, `User-Agent: getClaudeCodeUserAgent()`. Trust boundary:
`isUsingOAuth()` requires first-party Anthropic base URL + both
`CLAUDE_AI_INFERENCE_SCOPE` and `CLAUDE_AI_PROFILE_SCOPE`. Path-traversal
defense: every key from server validates via
`memdir/teamMemPaths.validateTeamMemKey` (owned by 40); `PathTraversalError`
is logged + skipped, never thrown out of the per-entry handler
(`index.ts:697-702`).

External binaries: `git` (transitive via `getGithubRepo`).

---

## 8. Feature Flags & Variants

### 8.1 `feature('EXTRACT_MEMORIES')`

- **OFF**: `backgroundHousekeeping.ts:7` binds `extractMemoriesModule = null`,
  `:34` short-circuits, so `initExtractMemories` never runs and `extractor`
  stays `null`. `stopHooks.ts:42,142` short-circuit identically. `print.ts:968`
  drain is a no-op. `executeExtractMemories` and `drainPendingExtraction`
  remain callable but null-coalesce.
- **ON**: full pipeline as in §5.

### 8.2 `feature('TEAMMEM')`

| Site | OFF | ON |
|---|---|---|
| `setup.ts:365` | (no-op) | lazy-import + `startTeamMemoryWatcher()` |
| `extractMemories.ts:65-67` | `teamMemPaths = null` | `require('memdir/teamMemPaths.js')` |
| `extractMemories.ts:362-364` | `teamMemoryEnabled = false` | `teamMemPaths.isTeamMemoryEnabled()` |
| `extractMemories.ts:402-413` | always `buildExtractAutoOnlyPrompt` | branch on `teamMemoryEnabled` |
| `extractMemories.ts:468-470` | `teamCount = 0` | per-path `isTeamMemPath` count |
| `extractMemories.ts:492-494` | (no field) | sets `msg.teamCount` on saved-message |
| `prompts.ts:106-112` | `buildExtractCombinedPrompt` falls back to auto-only | combined prompt |
| `teamMemSecretGuard.ts:19` | always `null` | full scan |
| `watcher.ts:253-255` | hard return | starts watcher |
| `_resetWatcherStateForTesting` | only entry to drive watcher in `bun test` | normal |

### 8.3 `feature('MEMORY_SHAPE_TELEMETRY')`

`memdir/findRelevantMemories.ts:66-72` (owned by 40 — recorded here per
dispatch §scope): when ON, after `selectRelevantMemories`, lazy-requires
`./memoryShapeTelemetry.js` and calls `logMemoryRecallShape(memories,
selected)`. Fires even on empty selection (denominator). Owns no behavior in
the three services.

### 8.4 ANT-only paths

| Site | Behavior |
|---|---|
| `extractMemories.ts:537-540` | once-per-session `tengu_extract_memories_gate_disabled` log |
| `sessionMemory.ts:286-289` | once-per-session `tengu_session_memory_gate_disabled` log |
| `sessionMemory.ts:362-367` | per-init `tengu_session_memory_init` log |

No ANT-only behavior in teamMemorySync.

### 8.5 Non-`feature()` env gates (synthesis)

| Env | Effect |
|---|---|
| `CLAUDE_CODE_DISABLE_AUTO_MEMORY` (truthy) | `isAutoMemoryEnabled() === false` → all three pipelines no-op |
| `CLAUDE_CODE_SIMPLE` (truthy) | hard-off (paths.ts:41-43) |
| `CLAUDE_CODE_REMOTE` (truthy) without `CLAUDE_CODE_REMOTE_MEMORY_DIR` | hard-off |
| `CLAUDE_CODE_REMOTE_MEMORY_DIR` | replaces `~/.claude` base for memory storage |
| `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` | full-path override; `hasAutoMemPathOverride()→true` (filesystem write carve-out becomes inert — see paths.ts comment :267) |
| `CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES` | injected into `loadMemoryPrompt`'s `extraGuidelines` |
| `TEAM_MEMORY_SYNC_URL` | endpoint base override |
| `USER_TYPE === 'ant'` | once-per-session gate-disabled telemetry; `tengu_session_memory_init` |

GB-flag gates: `tengu_passport_quail` (extractMemories master gate),
`tengu_slate_thimble` (extractMemories non-interactive override),
`tengu_bramble_lintel` (per-turn throttle), `tengu_moth_copse` (skipIndex
mode), `tengu_session_memory` (sessionMemory master), `tengu_sm_config`
(sessionMemory remote config), `tengu_coral_fern` (past-context section),
`tengu_herring_clock` (team-recall enable telemetry).

---

## 9. Error Handling & Edge Cases

### 9.1 extractMemories

| Failure | Handling |
|---|---|
| `getMemoryFiles` / `formatMemoryManifest` throws | bubbles into outer try; caught at `:497-502`; logs `tengu_extract_memories_error`; cursor NOT advanced |
| `runForkedAgent` throws | same as above |
| canUseTool deny | logged via `tengu_auto_mem_tool_denied`; deny propagated to forked agent |
| `inProgress` overlap | second call stashes in `pendingContext` (latest-wins) and returns; trailing run scheduled in `finally` |
| `lastMemoryMessageUuid` not found in messages (compaction dropped it) | `countModelVisibleMessagesSince` falls back to counting all model-visible messages (`:106-108`) — not zero |
| Drain timeout | `drainPendingExtraction` races `Promise.all` against `setTimeout(60_000).unref()` (no awaiting after timeout) |
| `agentId` set | early return at `:533` (subagents do not extract) |
| Remote mode | early return at `:550-552` |

### 9.2 SessionMemory

| Failure | Handling |
|---|---|
| `setupSessionMemoryFile` `EEXIST` on initial write | swallowed (file exists; proceed) |
| Other errno | rethrown |
| `loadSessionMemoryTemplate` ENOENT | returns `DEFAULT_SESSION_MEMORY_TEMPLATE` |
| Other read errors | `logError` + return default |
| `manuallyExtractSessionMemory` exception | caught; returns `{ success: false, error: errorMessage(error) }`; `markExtractionCompleted` always runs in `finally` |
| Stale extraction | `waitForSessionMemoryExtraction` returns immediately if age > 60_000 ms |
| Wait timeout | returns after 15_000 ms wall-clock without throwing |

### 9.3 teamMemorySync

| Failure | Handling |
|---|---|
| OAuth missing | `auth.error` short-circuit; `errorType: 'auth'`, `skipRetry: true` |
| `getGithubRepo()` returns null (non-github.com) | `no_repo` early return |
| 304 | `notModified: true`, no writes |
| 404 | `lastKnownChecksum = null`, `isEmpty: true` |
| 412 (push) | `conflict: true`; up to 2 probe-and-retry attempts; on probe failure, retain conflict-failure semantics |
| 413 with structured body | parse → cache `serverMaxEntries` for next push; this push fails |
| 413 unstructured (gateway) | `errorType: 'unknown'` http error; `serverMaxEntries` NOT learned |
| Body parse fail | `errorType: 'parse'`, `skipRetry: true` |
| Network/timeout | retry up to `MAX_RETRIES` with `getRetryDelay(attempt)` |
| `PathTraversalError` from `validateTeamMemKey` | warn + skip per-entry; remainder proceeds |
| Oversized file (> 250_000 bytes) | skipped pre-PUT and pre-write; info-log only |
| Server entryChecksums missing | `serverChecksums` stays empty → next push uploads everything (self-correcting) |
| Probe (`view=hashes`) failure | push fails (conflict); watcher retries on next edit |
| `fs.watch` ENOENT/EACCES | warn-log; `watcherStarted = true` so explicit `notifyTeamMemoryWrite` still works |
| Permanent failure (`isPermanentFailure`) | sets `pushSuppressedReason`; cleared only by an `unlink` event (ENOENT on `stat`) within the watcher |
| Shutdown with `pushInProgress` | `stopTeamMemoryWatcher` awaits `currentPushPromise` (errors swallowed); flushes pending best-effort within 2s graceful shutdown |

Conflict-failure user-visible message (debug only): `team-memory-sync: giving up after 2 conflict retries`.

### 9.4 secretScanner false-negative class

Rule set is the gitleaks high-confidence subset only — generic
keyword-context rules are intentionally omitted (`:6-9`). Anthropic prefix
literal is assembled at runtime to evade external-bundle excluded-strings
check (`:46`).

---

## 10. Telemetry & Observability

All identifiers verbatim in §6.7. Notable shapes:

- `tengu_extract_memories_extraction` — full token usage including
  cache_read/cache_creation, plus `message_count`, `turn_count`,
  `files_written`, `memories_saved`, `team_memories_saved`, `duration_ms`.
- `tengu_team_mem_sync_pull` / `tengu_team_mem_sync_push` — uniform shape with
  `success`, `files_*`, `not_modified` / `conflict` / `conflict_retries`,
  optional `errorType`, `status`, `put_batches`, `error_code`,
  `server_max_entries`, `server_received_entries`. (`error_code` is the
  Datadog-filterable facet for `team_memory_too_many_entries`.)
- `tengu_team_mem_secret_skipped` — joins `rule_ids` comma-separated. Path is
  NOT logged in analytics (only in debug log).
- `tengu_session_memory_extraction` — token usage from the LAST conversation
  message (`getTokenUsage(lastMessage)`), not the forked-agent result.

Debug strings are gathered in §6.7 and verbatim across §5.

The setup-time `tengu_team_mem_sync_started` event is fired regardless of
pull outcome (after `await startFileWatcher`); fields capture initial pull
result and presence of server content (`watcher.ts:298-304`).

---

## 11. Reimplementation Checklist

- [ ] `feature('EXTRACT_MEMORIES')` actual gates are
  `utils/backgroundHousekeeping.ts:7,34` and `query/stopHooks.ts:42,142` —
  NOT `memdir/paths.ts:65` (comment).
- [ ] `extractor` and `drainer` are module-level `let` bindings rebound by
  every `initExtractMemories()` call; closure-scoped state is
  per-init-instance.
- [ ] `extractor` set inside `initExtractMemories` adds promises to
  `inFlightExtractions` ONLY for the outer impl; trailing runs are awaited
  inside `runExtraction`'s `finally` and do not need their own membership.
- [ ] `pendingContext` is overwritten (latest-wins), not queued.
- [ ] When the main agent itself wrote auto-memory paths in this turn range,
  the extraction agent is skipped AND the cursor advances past the range;
  `tengu_extract_memories_skipped_direct_write` is emitted.
- [ ] `isExtractModeActive()` requires `tengu_passport_quail` truthy AND
  (interactive OR `tengu_slate_thimble`).
- [ ] Auto-memory canUseTool: REPLTool always-allow; Read/Grep/Glob unrestricted;
  Bash only when `tool.isReadOnly(parsed)`; Edit/Write only inside
  `isAutoMemPath`. `BashTool rm` is excluded by the read-only check.
- [ ] `runForkedAgent` for extractMemories sets
  `querySource:'extract_memories'`, `forkLabel:'extract_memories'`,
  `skipTranscript:true`, `maxTurns:5`.
- [ ] Drain default is 60s; `setTimeout(...).unref()` so it never holds
  process exit.
- [ ] Throttle defaults to `(GB tengu_bramble_lintel) ?? 1` eligible turns
  per run; trailing runs bypass the throttle.
- [ ] `cli/print.ts:968` calls `drainPendingExtraction()` after flushing
  response, before `gracefulShutdownSync`.
- [ ] `initSessionMemory()` is synchronous, gate-checks lazily inside the
  hook, no-ops if `getIsRemoteMode()` or auto-compact off.
- [ ] `extractSessionMemory` is wrapped in `sequential(...)`; only main REPL
  thread queries proceed.
- [ ] Token thresholds use `tokenCountWithEstimation(messages)` (same metric
  as auto-compact); init threshold is `>=` total tokens; update threshold is
  growth since last extraction (`>= minimumTokensBetweenUpdate`).
- [ ] `shouldExtractMemory`: tokens AND tool-calls, OR tokens AND no
  tool-calls in last assistant turn. Tokens are ALWAYS required.
- [ ] Session memory file: dir mode 0o700; file mode 0o600; created with
  `flag:'wx'`, then template overwrites; `readFileState.delete(memoryPath)`
  before `FileReadTool.call`.
- [ ] `createMemoryFileCanUseTool` allows ONLY `FILE_EDIT_TOOL_NAME` on the
  exact memoryPath; everything else denied.
- [ ] `manuallyExtractSessionMemory` uses fork label `session_memory_manual`,
  builds its own `cacheSafeParams` (system+user+system context), always calls
  `markExtractionCompleted` in `finally`.
- [ ] `loadSessionMemoryTemplate`/`Prompt` read from
  `~/.claude/session-memory/config/{template,prompt}.md` with ENOENT
  fallback; no caching.
- [ ] `buildSessionMemoryUpdatePrompt` injects per-section + total-budget
  reminders before returning. Variable substitution is single-pass `\{\{(\w+)\}\}`.
- [ ] `truncateSessionMemoryForCompact` truncates at line boundary inside
  each section once `length > MAX_SECTION_LENGTH * 4`; appends
  `\n[... section truncated for length ...]`.
- [ ] `MAX_TOTAL_SESSION_MEMORY_TOKENS = 12000`, `MAX_SECTION_LENGTH = 2000`.
- [ ] `EXTRACTION_WAIT_TIMEOUT_MS = 15000`, `EXTRACTION_STALE_THRESHOLD_MS = 60000`.
- [ ] `DEFAULT_SESSION_MEMORY_CONFIG = { 10000, 5000, 3 }`.
- [ ] `setSessionMemoryConfig` only adopts remote values that are
  positive-defined; zero values fall back to defaults.
- [ ] `teamMemorySync` requires first-party Anthropic base URL AND both
  `CLAUDE_AI_INFERENCE_SCOPE` and `CLAUDE_AI_PROFILE_SCOPE`.
- [ ] Team-memory endpoint: `${TEAM_MEMORY_SYNC_URL || getOauthConfig().BASE_API_URL}/api/claude_code/team_memory?repo=${encodeURIComponent(repoSlug)}`.
- [ ] `If-Match` / `If-None-Match` headers strip and re-add quotes
  (`headers['…'] = "${etag.replace(/"/g,'')}"`).
- [ ] Pull `validateStatus` accepts 200/304/404 only; PUT accepts 200/412
  only; hashes GET accepts 200/404 only. Anything else throws into the
  `classifyAxiosError` path.
- [ ] Pull retry: up to `MAX_RETRIES = 3` retries via `getRetryDelay`, except
  `skipRetry` for auth/parse.
- [ ] Push conflict loop: up to `MAX_CONFLICT_RETRIES = 2` 412-probe-and-retry
  iterations; in each iteration, recompute delta from refreshed
  `serverChecksums`, NOT re-read disk.
- [ ] `batchDeltaByBytes`: alphabetically-sorted deterministic batching;
  `MAX_PUT_BODY_BYTES = 200_000`; entries count
  `byteLength(jsonStringify(k)) + byteLength(jsonStringify(v)) + 2`; empty
  body basis `byteLength('{"entries":{}}')`. A single entry over
  `MAX_PUT_BODY_BYTES` ships solo.
- [ ] `MAX_FILE_SIZE_BYTES = 250_000` enforced on both push read and pull
  write (pre-validate skip with debug log).
- [ ] Server-only `serverMaxEntries` is learned from a structured 413 body
  (`TeamMemoryTooManyEntriesSchema`); cached on `SyncState`; truncates the
  next push's local read deterministically (sorted keys, alphabetically-last
  dropped).
- [ ] `readLocalTeamMemory` scans every file with `scanForSecrets` BEFORE
  upload; first match per file is recorded; file is excluded from the
  payload entirely.
- [ ] `writeRemoteEntriesToLocal` skips writes when local content equals
  incoming content (mtime preservation).
- [ ] Pull writes invalidate `clearMemoryFileCaches()` only when at least one
  file was actually written.
- [ ] Watcher uses `fs.watch(teamDir, { persistent:true, recursive:true })`
  (NOT chokidar). On macOS Bun maps to FSEvents (O(1) fds); on Linux to
  inotify per-dir.
- [ ] `DEBOUNCE_MS = 2000`. While `pushInProgress`, the timer reschedules
  itself instead of firing.
- [ ] `pushSuppressedReason` is set when `isPermanentFailure(result)`
  returns true (no_oauth/no_repo, OR 4xx not 409/429); cleared only by
  ENOENT-stat on a watched filename, OR by next session restart.
- [ ] `notifyTeamMemoryWrite` is callable from PostToolUse hooks;
  short-circuits if `syncState===null`.
- [ ] `stopTeamMemoryWatcher` is registered with `registerCleanup` and
  best-effort flushes pending changes within the 2s graceful budget.
- [ ] `secretScanner` rule list ordered as in §6.8; rules compiled lazily on
  first `scanForSecrets`; `scanForSecrets` returns at most one match per rule
  ID and never returns matched text.
- [ ] `redactSecrets` compiles a separate always-`g` rule set lazily;
  preserves boundary chars by replacing only the captured group.
- [ ] `checkTeamMemSecrets` is callable unconditionally; internal
  `feature('TEAMMEM')` returns `null` when off; only fires for paths matching
  `isTeamMemPath`.
- [ ] All ANT-only telemetry is once-per-session (latch flags), not per-call.
- [ ] `loadMemoryPrompt` precedence: KAIROS daily-log → TEAMMEM combined →
  auto-only → null (with `tengu_memdir_disabled` event).
- [ ] `MAX_ENTRYPOINT_LINES = 200`, `MAX_ENTRYPOINT_BYTES = 25_000`;
  truncation suffix preserves the warning string verbatim with the leading
  `\n\n`.
- [ ] `getAutoMemPath` is memoized keyed on `getProjectRoot()`.
- [ ] Override resolution order: `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` env >
  `autoMemoryDirectory` setting (policy/flag/local/user — projectSettings
  excluded) > `{base}/projects/{sanitized-cwd}/memory/`.
- [ ] `validateMemoryPath` rejects relative, root/near-root (length<3),
  Windows drive-root (`/^[A-Za-z]:$/`), UNC (`\\` or `//` prefix), and null
  bytes. NFC-normalizes; ensures exactly one trailing separator.

---

## 12. Open Questions / Unknowns

1. `runForkedAgent` / `createCacheSafeParams` / `createSubagentContext`
   contracts — owned by 03 (forked-agent infrastructure). This spec assumes
   the existing `utils/forkedAgent.js` API surface; if 03 documents
   additional fields, this spec's prompt-cache statements remain accurate
   only by virtue of `cacheSafeParams` reuse.
2. `teamMemPaths.isTeamMemoryEnabled / isTeamMemPath / getTeamMemPath /
   validateTeamMemKey` definitions live in `memdir/teamMemPaths.ts` (owned
   by 40). Their behavior is treated here as oracular; the path-traversal
   contract is enforced by `validateTeamMemKey` (cited at `:696`).
3. `teamMemPrompts.buildCombinedMemoryPrompt` (cited at `memdir.ts:468`) is
   owned by 40; the per-type combined prompt body is **not** inlined here
   beyond `TYPES_SECTION_COMBINED` already provided in §6.2.2. Spec 40
   should document the assembled wrapper.
4. `getSessionMemoryDir` / `getSessionMemoryPath` resolution (path
   composition, env overrides) live in `utils/permissions/filesystem.ts` —
   owned by 11. The `0o700`/`0o600` permission contract is enforced HERE on
   first-write, not in those helpers.
5. The `recordSkillUsage` integration question resolves to **none**; if a
   future change wires extractMemories into skill-usage tracking, that change
   should be logged in §2 imports rather than this §12.
6. `MEMORY_SHAPE_TELEMETRY` flag is documented here per dispatch §scope but
   the actual behavior (`logMemoryRecallShape`) is owned by spec 40's
   `memoryShapeTelemetry.js`.
7. `setup.ts:365` lazy-imports `services/teamMemorySync/watcher.js` only when
   `feature('TEAMMEM')` is on — but `watcher.ts:253-255` also short-circuits
   on `!feature('TEAMMEM')`. The double-gate is intentional (the inner gate
   exists so test code that bypasses setup still gets the early-out).
   Spec 01 owns the outer gate.
8. The forked agent's tool list excludes `Glob`/`Grep` for ANT bundles via
   `hasEmbeddedSearchTools()` (see `memdir.ts:385-388` `Searching past
   context` text). The `createAutoMemCanUseTool` decision tree allows
   GREP/GLOB by name, so when those names are not registered as tools, the
   allow branch is unreachable; behavior is correct (allow is safe; deny is
   only reachable for actually-present tools).

