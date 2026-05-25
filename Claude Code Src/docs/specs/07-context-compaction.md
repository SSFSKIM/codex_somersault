# 07 — Context Compaction Specification

> Owner: `sub-B5` · Last updated: 2026-05-08 · Adjacent: 03, 04, 05, 29, 41
> Source-coverage inventory at end of §2.

---

## 1. Purpose & Scope

Context compaction is the family of mechanisms that keep the assistant's prompt
within a model's context window across a long-running session. There are six
distinct pathways that all live under `src/services/compact/`:

1. **Manual `/compact`** — user-initiated full-history summarization (entry
   `compactConversation`).
2. **Auto-compact** — proactive token-threshold trigger run before each turn
   (`autoCompactIfNeeded` → `compactConversation`, with a `trySessionMemoryCompaction`
   short-circuit experiment).
3. **Partial compact** — message-selector-driven summarization of a prefix or
   suffix slice (`partialCompactConversation`).
4. **Microcompact (time-based + cached)** — pre-API tool-result clearing /
   server-side cache-edit elision, runs on every turn (`microcompactMessages`).
5. **Reactive compact** (ANT-only feature flag `REACTIVE_COMPACT`) — fallback
   compact triggered by an actual API `prompt_too_long` (HTTP 413) or media-size
   error after the request has been sent.
6. **Snip** (ANT-only feature flag `HISTORY_SNIP`) — granular tool-result/redo
   pruning applied per turn before microcompact. Tool implementation (`SnipTool`)
   is **missing-leaked-source**; the registry-side gate and caller-side
   integration are documented here. Algorithmic details of `snipCompact.ts` /
   `snipProjection.ts` are **missing-leaked-source** as well.

A seventh adjacent pathway, **context-collapse** (feature flag `CONTEXT_COLLAPSE`),
is mostly owned by an external (also missing-leaked-source) service
`src/services/contextCollapse/`; only the compaction-relevant deltas are
documented here.

In scope: trigger conditions, retention policy, prompts, attachments,
retry/backoff, hook fan-out, telemetry events, all feature flags including
ANT-gated paths.

Out of scope (refer by spec #):

- Tool-use loop generally → 04.
- Anthropic API streaming, retries, prompt-cache breakpoint placement → 03.
- System prompt assembly, `userContext` / `systemContext` blocks → 05.
- `MEMORY.md` / persistent memory storage → 40.
- Session history storage and `--resume` metadata → 41.
- Forked agent transport (`runForkedAgent`) → 29 (memory services) /
  30 (coordinator) — only invocation contract is repeated here.

---

## 2. Source Map

### 2.1 Owned files

| Path | Lines | Coverage | Notes |
|---|---|---|---|
| `src/services/compact/compact.ts` | 1705 | fully read | Manual + auto + partial entry points |
| `src/services/compact/autoCompact.ts` | 351 | fully read | Threshold math, entry `autoCompactIfNeeded`, circuit breaker |
| `src/services/compact/microCompact.ts` | 530 | fully read | Time-based + cached MC dispatch |
| `src/services/compact/apiMicrocompact.ts` | 153 | fully read | API-side `clear_tool_uses_20250919` / `clear_thinking_20251015` config |
| `src/services/compact/sessionMemoryCompact.ts` | 630 | fully read | SM-compact experiment |
| `src/services/compact/postCompactCleanup.ts` | 77 | fully read | Cache resets after every compact |
| `src/services/compact/prompt.ts` | 374 | fully read | All summarizer prompts |
| `src/services/compact/grouping.ts` | 63 | fully read | API-round grouping for PTL retry |
| `src/services/compact/timeBasedMCConfig.ts` | 43 | fully read | GrowthBook config for time-based MC |
| `src/services/compact/compactWarningHook.ts` | 16 | fully read | Ink hook for warning suppression |
| `src/services/compact/compactWarningState.ts` | 18 | fully read | External store for warning suppression |

### 2.2 Caller-side integration (read in scope)

| Path | Lines | Coverage |
|---|---|---|
| `src/query.ts` | 12, 15-20, 113-122, 207, 272, 314, 367, 396-426, 440-446, 472-518, 600-638, 800-820, 855-895, 1080-1180, 1718 | grep + targeted read |
| `src/QueryEngine.ts` | 122-128, 1276 | grep + targeted read |
| `src/utils/attachments.ts` | 672, 922-933, 3931-3955, 3958-3969 | grep + targeted read |
| `src/utils/messages.ts` | 4139-4147, 4530, 4557 (constructors) | grep + targeted read |
| `src/utils/hooks.ts` | 84-85, 1630-1631, 3961-4014, 4034-4080 | grep + targeted read |
| `src/commands/compact/compact.ts` | 10, 35, 66, 101, 197, 206 | grep |
| `src/utils/context.ts` | 12 (`COMPACT_MAX_OUTPUT_TOKENS`) | grep |

### 2.3 Missing-leaked-source

Caller imports a path that is not present in `src/`:

| Path | Citation | Gate |
|---|---|---|
| `src/services/compact/reactiveCompact.ts` | `query.ts:15-17` | `feature('REACTIVE_COMPACT')` |
| `src/services/compact/cachedMicrocompact.ts` | `microCompact.ts:56-67` (dynamic `import`) | `feature('CACHED_MICROCOMPACT')` |
| `src/services/compact/snipCompact.ts` | `query.ts:115-117`, `attachments.ts:3949-3955`, `messages.ts:2354,2424,4151-4152` | `feature('HISTORY_SNIP')` |
| `src/services/compact/snipProjection.ts` | `QueryEngine.ts:125-127`, `messages.ts:4651` | `feature('HISTORY_SNIP')` |
| `src/services/contextCollapse/index.ts` | `query.ts:18-20`, `autoCompact.ts:217-219`, `postCompactCleanup.ts:42-49` | `feature('CONTEXT_COLLAPSE')` |
| `src/tools/SnipTool/SnipTool.ts` | `tools.ts:123-124` | `feature('HISTORY_SNIP')` (also see overview §2.5) |
| `src/services/sessionTranscript/sessionTranscript.ts` | `compact.ts:6-8` (dynamic `require` under `KAIROS`) | `feature('KAIROS')` |
| `src/proactive/index.ts` | `prompt.ts:6-9` | `feature('PROACTIVE') \|\| feature('KAIROS')` |

These references must NOT be silently dropped during reimplementation; their
external public contracts (function signatures, returned shapes) are reproduced
in §3 from caller usage.

### 2.4 Imports from

`@anthropic-ai/sdk`, `bun:bundle`, `lodash-es/uniqBy`, `crypto.UUID`,
`src/bootstrap/state` (`markPostCompaction`, `getInvokedSkillsForAgent`,
`getSdkBetas`), `src/Tool` (`ToolUseContext`), `src/tools/FileReadTool/*`,
`src/tools/ToolSearchTool/ToolSearchTool`, `src/utils/attachments`
(`createAttachmentMessage`, `generateFileAttachment`,
`getAgentListingDeltaAttachment`, `getDeferredToolsDeltaAttachment`,
`getMcpInstructionsDeltaAttachment`), `src/utils/context.COMPACT_MAX_OUTPUT_TOKENS`,
`src/utils/contextAnalysis` (`analyzeContext`, `tokenStatsToStatsigMetrics`),
`src/utils/forkedAgent.runForkedAgent`, `src/utils/hooks`
(`executePreCompactHooks`, `executePostCompactHooks`),
`src/utils/messages` (`createCompactBoundaryMessage`,
`createMicrocompactBoundaryMessage`, `createUserMessage`,
`getAssistantMessageText`, `getLastAssistantMessage`,
`getMessagesAfterCompactBoundary`, `isCompactBoundaryMessage`,
`normalizeMessagesForAPI`), `src/utils/sessionStorage`
(`getTranscriptPath`, `reAppendSessionMetadata`),
`src/utils/sessionStart.processSessionStartHooks`,
`src/utils/tokens.{getTokenUsage,tokenCountFromLastAPIResponse,tokenCountWithEstimation}`,
`src/services/api/claude` (`getMaxOutputTokensForModel`,
`queryModelWithStreaming`),
`src/services/api/errors` (`getPromptTooLongTokenGap`,
`PROMPT_TOO_LONG_ERROR_MESSAGE`, `startsWithApiErrorPrefix`),
`src/services/api/promptCacheBreakDetection`
(`notifyCompaction`, `notifyCacheDeletion`),
`src/services/api/withRetry.getRetryDelay`,
`src/services/analytics/{growthbook,index}`,
`src/services/SessionMemory/{prompts,sessionMemoryUtils}`,
`src/services/internalLogging.logPermissionContextForAnts`,
`src/services/tokenEstimation.{roughTokenCountEstimation,roughTokenCountEstimationForMessages}`,
`src/utils/sessionActivity.{isSessionActivityTrackingActive,sendSessionActivitySignal}`,
`src/utils/path.expandPath`, `src/utils/plans.{getPlan,getPlanFilePath}`,
`src/utils/memory/types.MEMORY_TYPE_VALUES`,
`src/utils/toolSearch.{extractDiscoveredToolNames,isToolSearchEnabled}`,
`src/utils/fileStateCache.cacheToObject`,
`src/utils/sleep.sleep`, `src/utils/slowOperations.jsonStringify`,
`src/utils/log.logError`, `src/utils/debug.logForDebugging`,
`src/utils/errors.hasExactErrorMessage`,
`src/utils/systemPromptType.asSystemPrompt`,
`src/utils/task/diskOutput.getTaskOutputPath`.

### 2.5 Imported by

`src/query.ts`, `src/QueryEngine.ts`, `src/commands/compact/compact.ts`,
`src/commands/compact/messageSelector.tsx`, `src/utils/attachments.ts`,
`src/utils/messages.ts`, `src/utils/analyzeContext.ts`, `src/utils/sessionRestore.ts`,
`src/components/TokenWarning.tsx`, `src/screens/REPL.tsx`,
`src/screens/ResumeConversation.tsx`, `src/services/SessionMemory/*`,
`src/proactive/*` (when enabled).

### 2.6 Feature-flag guard locations

| Flag | Location | Effect |
|---|---|---|
| `EXPERIMENTAL_SKILL_SEARCH` | `compact.ts:212` | strip skill_discovery/skill_listing attachments before summarizer |
| `KAIROS` | `compact.ts:6-8`, `compact.ts:715-717`, `compact.ts:1059-1063` | enables `sessionTranscript.writeSessionTranscriptSegment` after compact |
| `PROMPT_CACHE_BREAK_DETECTION` | `compact.ts:698,1047`, `microCompact.ts:362-367,525-527`, `autoCompact.ts:302-304` | calls `notifyCompaction`/`notifyCacheDeletion` to suppress false-positive break events |
| `PROACTIVE` / `KAIROS` | `prompt.ts:6-9, 362-368` | autonomous-mode continuation suffix |
| `REACTIVE_COMPACT` | `query.ts:15-17`, `autoCompact.ts:195-199` | enables reactive compact module + suppresses proactive autocompact when `tengu_cobalt_raccoon` is true |
| `CACHED_MICROCOMPACT` | `microCompact.ts:56,276-286`, `query.ts:423,870` | enables cache-editing microcompact path |
| `CONTEXT_COLLAPSE` | `autoCompact.ts:179-183,215-223`, `postCompactCleanup.ts:42-49`, `query.ts:18-20,440-446,616-621,800,1090,1176` | suppresses autocompact in marble_origami subagent; resets collapse state on main-thread compact; prefers collapse drain over reactive compact for 413 |
| `HISTORY_SNIP` | `query.ts:115-118,401-410`, `QueryEngine.ts:122-127,1276`, `attachments.ts:934,3966`, `messages.ts:2351,2414,4149,4648` | enables snip pre-microcompact + context-efficiency nudge attachment + snipReplay |
| `COMPACTION_REMINDERS` | `attachments.ts:922-933` | enables `compaction_reminder` attachment under `tengu_marble_fox` GrowthBook gate (1M-window models only, ≥25% used) |
| `BG_SESSIONS` | `query.ts:118-121` | not compaction-owned but also imported in same block |
| `COMMIT_ATTRIBUTION` | `postCompactCleanup.ts:71-75` | sweep file content cache after compact |
| `COORDINATOR_MODE` | (caller side) | does not directly affect compact paths |

### 2.7 Env-variable guards

| Env var | Effect | Cite |
|---|---|---|
| `DISABLE_COMPACT` (truthy) | disables both manual and auto compact | `autoCompact.ts:148-150,253-255` |
| `DISABLE_AUTO_COMPACT` (truthy) | disables auto-compact only; `/compact` still works | `autoCompact.ts:151-154` |
| `CLAUDE_CODE_AUTO_COMPACT_WINDOW` (int) | clamps the effective context window used for threshold computation | `autoCompact.ts:40-46` |
| `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` (1..100) | sets autocompact threshold by percent of effective window (capped at default) | `autoCompact.ts:78-87` |
| `CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE` (int) | overrides hard-blocking limit | `autoCompact.ts:127-134` |
| `USE_API_CLEAR_TOOL_RESULTS` (ANT-only) | activates `clear_tool_uses_20250919` strategy with `clear_tool_inputs` set | `apiMicrocompact.ts:90-95,104-126` |
| `USE_API_CLEAR_TOOL_USES` (ANT-only) | activates `clear_tool_uses_20250919` strategy with `exclude_tools` set | `apiMicrocompact.ts:97-101,128-150` |
| `API_MAX_INPUT_TOKENS` / `API_TARGET_INPUT_TOKENS` | override API-side trigger/keep thresholds; default 180_000 / 40_000 | `apiMicrocompact.ts:16-17,105-110,129-132` |
| `ENABLE_CLAUDE_CODE_SM_COMPACT` (truthy) | force-enables session-memory compaction | `sessionMemoryCompact.ts:404-407` |
| `DISABLE_CLAUDE_CODE_SM_COMPACT` (truthy) | force-disables session-memory compaction | `sessionMemoryCompact.ts:408-410` |
| `USER_TYPE === 'ant'` | enables ANT-only telemetry flag-check log; gates API-microcompact tool-clearing strategies | `sessionMemoryCompact.ts:423-429`, `apiMicrocompact.ts:90` |
| `NODE_ENV === 'test'` | suppresses one HISTORY_SNIP path | `messages.ts:2351` |

### 2.8 Coverage summary

`src/services/compact/` — every file fully read.
`reactiveCompact.ts`, `cachedMicrocompact.ts`, `snipCompact.ts`,
`snipProjection.ts`, `contextCollapse/index.ts`, `tools/SnipTool/` — **missing
in leak**; documented from caller usage only.

---

## 3. Public Interface

### 3.1 Top-level exports from `src/services/compact/`

```ts
// compact.ts
export const POST_COMPACT_MAX_FILES_TO_RESTORE = 5
export const POST_COMPACT_TOKEN_BUDGET = 50_000
export const POST_COMPACT_MAX_TOKENS_PER_FILE = 5_000
export const POST_COMPACT_MAX_TOKENS_PER_SKILL = 5_000
export const POST_COMPACT_SKILLS_TOKEN_BUDGET = 25_000
export const ERROR_MESSAGE_NOT_ENOUGH_MESSAGES = 'Not enough messages to compact.'
export const ERROR_MESSAGE_PROMPT_TOO_LONG =
  'Conversation too long. Press esc twice to go up a few messages and try again.'
export const ERROR_MESSAGE_USER_ABORT = 'API Error: Request was aborted.'
export const ERROR_MESSAGE_INCOMPLETE_RESPONSE =
  'Compaction interrupted · This may be due to network issues — please try again.'

export interface CompactionResult {
  boundaryMarker: SystemMessage
  summaryMessages: UserMessage[]
  attachments: AttachmentMessage[]
  hookResults: HookResultMessage[]
  messagesToKeep?: Message[]
  userDisplayMessage?: string
  preCompactTokenCount?: number
  postCompactTokenCount?: number
  truePostCompactTokenCount?: number
  compactionUsage?: ReturnType<typeof getTokenUsage>
}

export type RecompactionInfo = {
  isRecompactionInChain: boolean
  turnsSincePreviousCompact: number
  previousCompactTurnId?: string
  autoCompactThreshold: number
  querySource?: QuerySource
}

export function stripImagesFromMessages(messages: Message[]): Message[]
export function stripReinjectedAttachments(messages: Message[]): Message[]
export function truncateHeadForPTLRetry(
  messages: Message[],
  ptlResponse: AssistantMessage,
): Message[] | null
export function buildPostCompactMessages(result: CompactionResult): Message[]
export function annotateBoundaryWithPreservedSegment(
  boundary: SystemCompactBoundaryMessage,
  anchorUuid: UUID,
  messagesToKeep: readonly Message[] | undefined,
): SystemCompactBoundaryMessage
export function mergeHookInstructions(
  user: string | undefined,
  hook: string | undefined,
): string | undefined

export async function compactConversation(
  messages: Message[],
  context: ToolUseContext,
  cacheSafeParams: CacheSafeParams,
  suppressFollowUpQuestions: boolean,
  customInstructions?: string,
  isAutoCompact?: boolean,           // default false
  recompactionInfo?: RecompactionInfo,
): Promise<CompactionResult>

export async function partialCompactConversation(
  allMessages: Message[],
  pivotIndex: number,
  context: ToolUseContext,
  cacheSafeParams: CacheSafeParams,
  userFeedback?: string,
  direction?: PartialCompactDirection,  // default 'from'
): Promise<CompactionResult>

export function createCompactCanUseTool(): CanUseToolFn

export async function createPostCompactFileAttachments(
  readFileState: Record<string, { content: string; timestamp: number }>,
  toolUseContext: ToolUseContext,
  maxFiles: number,
  preservedMessages?: Message[],          // default []
): Promise<AttachmentMessage[]>

export function createPlanAttachmentIfNeeded(agentId?: AgentId): AttachmentMessage | null
export function createSkillAttachmentIfNeeded(agentId?: string): AttachmentMessage | null
export async function createPlanModeAttachmentIfNeeded(
  context: ToolUseContext,
): Promise<AttachmentMessage | null>
export async function createAsyncAgentAttachmentsIfNeeded(
  context: ToolUseContext,
): Promise<AttachmentMessage[]>
```

```ts
// autoCompact.ts
export const AUTOCOMPACT_BUFFER_TOKENS = 13_000
export const WARNING_THRESHOLD_BUFFER_TOKENS = 20_000
export const ERROR_THRESHOLD_BUFFER_TOKENS = 20_000
export const MANUAL_COMPACT_BUFFER_TOKENS = 3_000

export type AutoCompactTrackingState = {
  compacted: boolean
  turnCounter: number
  turnId: string
  consecutiveFailures?: number
}

export function getEffectiveContextWindowSize(model: string): number
export function getAutoCompactThreshold(model: string): number
export function calculateTokenWarningState(
  tokenUsage: number,
  model: string,
): {
  percentLeft: number
  isAboveWarningThreshold: boolean
  isAboveErrorThreshold: boolean
  isAboveAutoCompactThreshold: boolean
  isAtBlockingLimit: boolean
}
export function isAutoCompactEnabled(): boolean
export async function shouldAutoCompact(
  messages: Message[], model: string,
  querySource?: QuerySource, snipTokensFreed?: number,
): Promise<boolean>
export async function autoCompactIfNeeded(
  messages: Message[], toolUseContext: ToolUseContext,
  cacheSafeParams: CacheSafeParams, querySource?: QuerySource,
  tracking?: AutoCompactTrackingState, snipTokensFreed?: number,
): Promise<{
  wasCompacted: boolean
  compactionResult?: CompactionResult
  consecutiveFailures?: number
}>
```

```ts
// microCompact.ts
export const TIME_BASED_MC_CLEARED_MESSAGE = '[Old tool result content cleared]'

export type PendingCacheEdits = {
  trigger: 'auto'
  deletedToolIds: string[]
  baselineCacheDeletedTokens: number
}
export type MicrocompactResult = {
  messages: Message[]
  compactionInfo?: { pendingCacheEdits?: PendingCacheEdits }
}

export function consumePendingCacheEdits(): CacheEditsBlock | null
export function getPinnedCacheEdits(): PinnedCacheEdits[]
export function pinCacheEdits(userMessageIndex: number, block: CacheEditsBlock): void
export function markToolsSentToAPIState(): void
export function resetMicrocompactState(): void
export function estimateMessageTokens(messages: Message[]): number
export function evaluateTimeBasedTrigger(
  messages: Message[], querySource: QuerySource | undefined,
): { gapMinutes: number; config: TimeBasedMCConfig } | null
export async function microcompactMessages(
  messages: Message[], toolUseContext?: ToolUseContext, querySource?: QuerySource,
): Promise<MicrocompactResult>
```

```ts
// apiMicrocompact.ts
export type ContextEditStrategy =
  | { type: 'clear_tool_uses_20250919'
      trigger?: { type: 'input_tokens'; value: number }
      keep?: { type: 'tool_uses'; value: number }
      clear_tool_inputs?: boolean | string[]
      exclude_tools?: string[]
      clear_at_least?: { type: 'input_tokens'; value: number } }
  | { type: 'clear_thinking_20251015'
      keep: { type: 'thinking_turns'; value: number } | 'all' }
export type ContextManagementConfig = { edits: ContextEditStrategy[] }
export function getAPIContextManagement(opts?: {
  hasThinking?: boolean
  isRedactThinkingActive?: boolean
  clearAllThinking?: boolean
}): ContextManagementConfig | undefined
```

```ts
// sessionMemoryCompact.ts
export const DEFAULT_SM_COMPACT_CONFIG = { minTokens: 10_000, minTextBlockMessages: 5, maxTokens: 40_000 }
export type SessionMemoryCompactConfig = { minTokens: number; minTextBlockMessages: number; maxTokens: number }
export function setSessionMemoryCompactConfig(c: Partial<SessionMemoryCompactConfig>): void
export function getSessionMemoryCompactConfig(): SessionMemoryCompactConfig
export function resetSessionMemoryCompactConfig(): void
export function hasTextBlocks(message: Message): boolean
export function adjustIndexToPreserveAPIInvariants(messages: Message[], startIndex: number): number
export function calculateMessagesToKeepIndex(messages: Message[], lastSummarizedIndex: number): number
export function shouldUseSessionMemoryCompaction(): boolean
export async function trySessionMemoryCompaction(
  messages: Message[], agentId?: AgentId, autoCompactThreshold?: number,
): Promise<CompactionResult | null>
```

```ts
// postCompactCleanup.ts
export function runPostCompactCleanup(querySource?: QuerySource): void
```

```ts
// prompt.ts
export function getPartialCompactPrompt(
  customInstructions?: string,
  direction?: PartialCompactDirection,   // default 'from'
): string
export function getCompactPrompt(customInstructions?: string): string
export function formatCompactSummary(summary: string): string
export function getCompactUserSummaryMessage(
  summary: string,
  suppressFollowUpQuestions?: boolean,
  transcriptPath?: string,
  recentMessagesPreserved?: boolean,
): string
```

```ts
// grouping.ts
export function groupMessagesByApiRound(messages: Message[]): Message[][]
```

```ts
// timeBasedMCConfig.ts
export type TimeBasedMCConfig = { enabled: boolean; gapThresholdMinutes: number; keepRecent: number }
export function getTimeBasedMCConfig(): TimeBasedMCConfig
```

```ts
// compactWarningState.ts / compactWarningHook.ts
export const compactWarningStore: ExternalStore<boolean> // initial false
export function suppressCompactWarning(): void
export function clearCompactWarningSuppression(): void
export function useCompactWarningSuppression(): boolean
```

### 3.2 Inferred external surfaces (caller usage)

From `query.ts`, `QueryEngine.ts`, `messages.ts`, `attachments.ts`:

```ts
// services/compact/reactiveCompact.ts (missing-leaked-source)
isReactiveCompactEnabled(): boolean
isWithheldPromptTooLong(msg: AssistantMessage): boolean
isWithheldMediaSizeError(msg: AssistantMessage): boolean
tryReactiveCompact(args: {
  hasAttempted: boolean
  querySource: QuerySource
  aborted: boolean
  messages: Message[]
  cacheSafeParams: CacheSafeParams
}): Promise<CompactionResult | null>

// services/compact/cachedMicrocompact.ts (missing-leaked-source)
type CachedMCState
type CacheEditsBlock
type PinnedCacheEdits
isCachedMicrocompactEnabled(): boolean
isModelSupportedForCacheEditing(model: string): boolean
getCachedMCConfig(): { triggerThreshold: number; keepRecent: number }
createCachedMCState(): CachedMCState
resetCachedMCState(state: CachedMCState): void
registerToolResult(state, toolUseId): void
registerToolMessage(state, ids: string[]): void
markToolsSentToAPI(state): void
getToolResultsToDelete(state): string[]
createCacheEditsBlock(state, toolIds: string[]): CacheEditsBlock | null

// services/compact/snipCompact.ts (missing-leaked-source)
// CROSS-REF: spec 41 (session history / snip) owns the authoritative
// surface for these functions. Signatures here are caller-derived
// (query.ts:401-410, QueryEngine.ts:122-127, 1276) and unverifiable
// against source. `boundaryMessage?` is documented optional but
// query.ts handles it as inline-yielded — treat as INFERRED.
SNIP_NUDGE_TEXT: string
isSnipRuntimeEnabled(): boolean
shouldNudgeForSnips(messages: Message[]): boolean
snipCompactIfNeeded(
  messages: Message[], opts?: { force?: boolean },
): { messages: Message[]; tokensFreed: number; boundaryMessage?: Message }

// services/compact/snipProjection.ts (missing-leaked-source)
isSnipBoundaryMessage(msg: Message): boolean

// services/contextCollapse/index.ts (missing-leaked-source)
isContextCollapseEnabled(): boolean
applyCollapsesIfNeeded(
  messages: Message[], context: ToolUseContext, querySource?: QuerySource,
): Promise<{ messages: Message[] }>
recoverFromOverflow(messages: Message[], querySource?: QuerySource):
  { messages: Message[]; committed: number }
resetContextCollapse(): void
```

---

## 4. Data Model & State

### 4.1 `CompactionResult` ordering invariant

`buildPostCompactMessages` (compact.ts:330-338) defines the one-true ordering for
post-compact messages:

```
[ boundaryMarker, ...summaryMessages, ...messagesToKeep, ...attachments, ...hookResults ]
```

Reactive compact, manual `/compact`, auto-compact and partial compact all
produce a `CompactionResult` and feed it through `buildPostCompactMessages`
(query.ts:1149).

### 4.2 Compact boundary metadata

`createCompactBoundaryMessage(trigger, preCompactTokenCount, lastPreCompactUuid,
userFeedback?, messagesSummarized?)` (`messages.ts:4530`) emits a
`SystemCompactBoundaryMessage`. The compaction layer then mutates two fields:

- `compactMetadata.preCompactDiscoveredTools = sortedArray(extractDiscoveredToolNames(messages))`
  — only when non-empty (compact.ts:606-611, 1023-1027, sessionMemoryCompact.ts:452-457).
  Carried so post-compact deferred-tool filtering still honors already-loaded schemas.
- `compactMetadata.preservedSegment = { headUuid, anchorUuid, tailUuid }` —
  added by `annotateBoundaryWithPreservedSegment` whenever `messagesToKeep`
  is non-empty (compact.ts:349-367, 1083-1087, sessionMemoryCompact.ts:488-492).
  `anchorUuid` rule:
  - **suffix-preserving** (`'up_to'` partial, session-memory, reactive that
    keeps a tail) → last summary message uuid.
  - **prefix-preserving** (`'from'` partial) → boundary marker uuid.

### 4.3 Microcompact state (module-level, ANT-only)

`microCompact.ts:56-60`:

```
let cachedMCModule: typeof import('./cachedMicrocompact.js') | null = null
let cachedMCState: CachedMCState | null = null
let pendingCacheEdits: CacheEditsBlock | null = null
```

State invariants:

- `cachedMCModule` is lazily resolved on first `microcompactMessages` call
  (`getCachedMCModule()` at lines 62-69). The dynamic `await import(...)` keeps
  the cached MC source out of external builds via DCE.
- `cachedMCState` is created once via `cachedMCModule.createCachedMCState()`
  (line 71-81) — `ensureCachedMCState()` throws if module is not loaded.
- `pendingCacheEdits` carries the next-request `cache_edits` block produced by
  the cached MC path. It is **single-shot**: `consumePendingCacheEdits()`
  reads-and-clears (lines 88-94). Caller (`query.ts:421-424`) reads it within
  the same turn before yielding the boundary message.
- `resetMicrocompactState()` (line 130-135) is invoked from
  `postCompactCleanup` (every successful compact) and from the time-based
  trigger when it mutates message content (line 517).

### 4.4 Auto-compact tracking state

Per `AutoCompactTrackingState` (autoCompact.ts:51-60). The query loop threads it
across iterations: `query.ts:207, 272, 314, 367, 1102, 1155, 1210, 1238, 1290,
1331, 1718`. `consecutiveFailures` is incremented on every catch path and reset
to `0` on success (autoCompact.ts:332-349). Circuit breaker fires at
`MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES = 3` (lines 70, 261-265).

### 4.5 Compact-warning suppression store

`compactWarningState.ts` exposes a Zustand-style external store
(`createStore<boolean>(false)`). Set to `true` after a successful microcompact
(both time-based and cached paths) and cleared at the start of every
`microcompactMessages` invocation (microCompact.ts:259, 359, 511). React UI
reads via `useCompactWarningSuppression`.

### 4.6 Session-memory-compact module state

Two module-level mutables in `sessionMemoryCompact.ts`:

```
let smCompactConfig = { ...DEFAULT_SM_COMPACT_CONFIG }   // line 64
let configInitialized = false                            // line 69
```

`initSessionMemoryCompactConfig` (lines 102-130) merges in the GrowthBook
`tengu_sm_compact_config` payload exactly once per session, accepting only
positive numeric overrides.

---

## 5. Algorithm / Control Flow

### 5.1 Per-turn compaction sequence (caller integration)

From `query.ts:396-518`, executed once per outer-turn iteration:

```
1. snip            (HISTORY_SNIP)
   queryCheckpoint('query_snip_start')
   snipResult = snipModule!.snipCompactIfNeeded(messagesForQuery)
   messagesForQuery = snipResult.messages
   snipTokensFreed = snipResult.tokensFreed
   if snipResult.boundaryMessage: yield it
   queryCheckpoint('query_snip_end')

2. microcompact
   queryCheckpoint('query_microcompact_start')
   microcompactResult = await microcompactMessages(
     messagesForQuery, toolUseContext, querySource)
   messagesForQuery = microcompactResult.messages
   pendingCacheEdits = (CACHED_MICROCOMPACT)
     ? microcompactResult.compactionInfo?.pendingCacheEdits
     : undefined
   queryCheckpoint('query_microcompact_end')

3. context-collapse projection (CONTEXT_COLLAPSE)
   collapseResult = await contextCollapse.applyCollapsesIfNeeded(
     messagesForQuery, toolUseContext, querySource)
   messagesForQuery = collapseResult.messages

4. autocompact (proactive)
   queryCheckpoint('query_autocompact_start')
   { compactionResult, consecutiveFailures } = await autoCompactIfNeeded(
     messagesForQuery, toolUseContext,
     { systemPrompt, userContext, systemContext, toolUseContext,
       forkContextMessages: messagesForQuery },
     querySource, tracking, snipTokensFreed)
   queryCheckpoint('query_autocompact_end')
```

A successful `compactionResult` causes the loop to:

- emit `tengu_auto_compact_succeeded` with full metric payload (query.ts:472-503).
- carry over task budget (`taskBudgetRemaining` decremented by
  `finalContextTokensFromLastResponse(messagesForQuery)`, query.ts:506-516).
- replace `messagesForQuery` with `buildPostCompactMessages(compactionResult)`.

### 5.2 Auto-compact threshold computation

`getEffectiveContextWindowSize(model)` (autoCompact.ts:33-49):

```
reservedTokensForSummary = min(getMaxOutputTokensForModel(model), 20_000)
contextWindow            = getContextWindowForModel(model, getSdkBetas())
if (CLAUDE_CODE_AUTO_COMPACT_WINDOW set):
   contextWindow = min(contextWindow, parsed)
return contextWindow - reservedTokensForSummary
```

`getAutoCompactThreshold(model)` (lines 72-91):

```
threshold = effectiveContextWindow - AUTOCOMPACT_BUFFER_TOKENS  // 13_000
if (CLAUDE_AUTOCOMPACT_PCT_OVERRIDE in (0,100]):
   threshold = min(floor(effectiveContextWindow * pct/100), threshold)
return threshold
```

`calculateTokenWarningState` (lines 93-145) returns five booleans plus
`percentLeft`:

```
percentLeft               = max(0, round((threshold - usage)/threshold * 100))
warningThreshold          = threshold - 20_000
errorThreshold            = threshold - 20_000
isAboveWarningThreshold   = usage >= warningThreshold
isAboveErrorThreshold     = usage >= errorThreshold
isAboveAutoCompactThreshold = isAutoCompactEnabled() && usage >= autoThreshold
defaultBlockingLimit      = effectiveContextWindow - 3_000
blockingLimit             = (CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE) ?? default
isAtBlockingLimit         = usage >= blockingLimit
```

Note: `WARNING_THRESHOLD_BUFFER_TOKENS` and `ERROR_THRESHOLD_BUFFER_TOKENS` are
both `20_000`; both currently flag the same threshold.

### 5.3 `shouldAutoCompact` decision tree (autoCompact.ts:160-239)

```
if querySource in {'session_memory', 'compact'}: return false
if feature('CONTEXT_COLLAPSE') and querySource == 'marble_origami': return false
if not isAutoCompactEnabled(): return false
if feature('REACTIVE_COMPACT') and gb('tengu_cobalt_raccoon', false):
   return false
if feature('CONTEXT_COLLAPSE') and isContextCollapseEnabled():
   return false
tokenCount = tokenCountWithEstimation(messages) - snipTokensFreed
return calculateTokenWarningState(tokenCount, model).isAboveAutoCompactThreshold
```

### 5.4 `autoCompactIfNeeded` orchestration (autoCompact.ts:241-351)

```
if env DISABLE_COMPACT: return { wasCompacted:false }
if tracking.consecutiveFailures >= 3: return { wasCompacted:false }   // circuit breaker
if not shouldAutoCompact(...): return { wasCompacted:false }

recompactionInfo = {
  isRecompactionInChain: tracking.compacted === true,
  turnsSincePreviousCompact: tracking.turnCounter ?? -1,
  previousCompactTurnId: tracking.turnId,
  autoCompactThreshold: getAutoCompactThreshold(model),
  querySource,
}

// Experiment: SM-compact short-circuit
sm = await trySessionMemoryCompaction(messages, agentId, autoCompactThreshold)
if sm:
   setLastSummarizedMessageId(undefined)
   runPostCompactCleanup(querySource)
   if feature('PROMPT_CACHE_BREAK_DETECTION'):
       notifyCompaction(querySource ?? 'compact', agentId)
   markPostCompaction()
   return { wasCompacted:true, compactionResult: sm }

try:
   result = await compactConversation(messages, ctx, csp,
            true, undefined, true, recompactionInfo)
   setLastSummarizedMessageId(undefined)
   runPostCompactCleanup(querySource)
   return { wasCompacted:true, compactionResult:result, consecutiveFailures:0 }
catch err:
   if not isUserAbort: logError(err)
   nextFailures = (tracking.consecutiveFailures ?? 0) + 1
   return { wasCompacted:false, consecutiveFailures: nextFailures }
```

### 5.5 `compactConversation` (compact.ts:387-763)

```
1. assert messages.length > 0 else throw NOT_ENOUGH_MESSAGES
2. preCompactTokenCount = tokenCountWithEstimation(messages)
3. logPermissionContextForAnts(appState.toolPermissionContext, 'summary')
4. onCompactProgress({ type:'hooks_start', hookType:'pre_compact' })
   setSDKStatus('compacting')
   hookResult = await executePreCompactHooks(
       { trigger: isAutoCompact?'auto':'manual',
         customInstructions: customInstructions ?? null },
       abortSignal)
   customInstructions = mergeHookInstructions(customInstructions,
                            hookResult.newCustomInstructions)
   userDisplayMessage = hookResult.userDisplayMessage
5. setStreamMode('requesting'); setResponseLength(()=>0)
   onCompactProgress({ type:'compact_start' })
6. promptCacheSharingEnabled = gb('tengu_compact_cache_prefix', true)
7. compactPrompt = getCompactPrompt(customInstructions)
   summaryRequest = createUserMessage({ content: compactPrompt })
8. PTL retry loop (max MAX_PTL_RETRIES = 3):
     summaryResponse = await streamCompactSummary({...})
     summary = getAssistantMessageText(summaryResponse)
     if !summary?.startsWith(PROMPT_TOO_LONG_ERROR_MESSAGE): break
     truncated = truncateHeadForPTLRetry(messagesToSummarize, summaryResponse)
     if !truncated: throw ERROR_MESSAGE_PROMPT_TOO_LONG (event: tengu_compact_failed prompt_too_long)
     log tengu_compact_ptl_retry { attempt, droppedMessages, remainingMessages }
     thread truncated set into both `messagesToSummarize` and
       `cacheSafeParams.forkContextMessages`
9. !summary -> log tengu_compact_failed{no_summary} -> throw
   startsWithApiErrorPrefix(summary) -> log {api_error} -> throw summary
10. preCompactReadFileState = cacheToObject(context.readFileState)
    context.readFileState.clear()
    context.loadedNestedMemoryPaths?.clear()
    // sentSkillNames intentionally NOT reset (compact.ts:524-529)
11. [fileAttachments, asyncAgentAttachments] = await Promise.all([
       createPostCompactFileAttachments(preCompactReadFileState, ctx, 5),
       createAsyncAgentAttachmentsIfNeeded(ctx) ])
    Build postCompactFileAttachments in this exact order
    (compact.ts:541-585) — `fileAttachments` and `asyncAgentAttachments`
    are seeded BEFORE planAttachment via the spread at lines 541-544:
      [
        ...fileAttachments,            // Promise.all result, FIRST
        ...asyncAgentAttachments,      // Promise.all result
        planAttachment?,               // if any
        planModeAttachment?,           // if appState.toolPermissionContext.mode == 'plan'
        skillAttachment?,              // if invokedSkills non-empty
        ...getDeferredToolsDeltaAttachment(... [], { callSite:'compact_full' }),
        ...getAgentListingDeltaAttachment(ctx, []),
        ...getMcpInstructionsDeltaAttachment(ctx.options.mcpClients,
                                            ctx.options.tools,
                                            ctx.options.mainLoopModel, []),
      ]
12. onCompactProgress({hooks_start, session_start})
    hookMessages = await processSessionStartHooks('compact',
                      { model: ctx.options.mainLoopModel })
13. boundaryMarker = createCompactBoundaryMessage(
       isAutoCompact?'auto':'manual', preCompactTokenCount,
       messages.at(-1)?.uuid)
    discovered = extractDiscoveredToolNames(messages)
    if discovered.size > 0:
       boundaryMarker.compactMetadata.preCompactDiscoveredTools =
          sorted([...discovered])
14. summaryMessages = [
       createUserMessage({
         content: getCompactUserSummaryMessage(summary,
                     suppressFollowUpQuestions, transcriptPath),
         isCompactSummary: true,
         isVisibleInTranscriptOnly: true })
    ]
15. compactionCallTotalTokens = tokenCountFromLastAPIResponse([summaryResponse])
    truePostCompactTokenCount = roughTokenCountEstimationForMessages(
       [boundaryMarker, ...summaryMessages, ...attachments, ...hookMessages])
    compactionUsage = getTokenUsage(summaryResponse)
16. logEvent('tengu_compact', { ...full metric payload, including
       analyzeContext(messages) → tokenStatsToStatsigMetrics IIFE
       deferred past the compact API await, swallowing errors })
17. if feature('PROMPT_CACHE_BREAK_DETECTION'):
       notifyCompaction(querySource ?? 'compact', ctx.agentId)
    markPostCompaction()
    reAppendSessionMetadata()
    if feature('KAIROS'):
       void sessionTranscriptModule?.writeSessionTranscriptSegment(messages)
18. onCompactProgress({hooks_start, post_compact})
    postCompactHookResult = await executePostCompactHooks(
        { trigger:'auto'|'manual', compactSummary: summary }, signal)
19. combinedUserDisplayMessage = filter([userDisplayMessage,
                                  postCompactHookResult.userDisplayMessage])
                                  .join('\n')
20. return { boundaryMarker, summaryMessages, attachments, hookResults,
            userDisplayMessage, preCompactTokenCount,
            postCompactTokenCount: compactionCallTotalTokens,
            truePostCompactTokenCount, compactionUsage }
21. catch -> if !isAutoCompact: addErrorNotificationIfNeeded; rethrow
22. finally: setStreamMode('requesting'); setResponseLength(()=>0)
            onCompactProgress({type:'compact_end'}); setSDKStatus(null)
```

### 5.6 `streamCompactSummary` (compact.ts:1136-1396)

Two paths gated by GrowthBook `tengu_compact_cache_prefix` (default `true`):

**Path A (forked-agent / cache sharing):**

- Activity keep-alive timer at 30_000 ms (lines 1167-1176): when
  `isSessionActivityTrackingActive()` is true, every 30 s ticks
  `sendSessionActivitySignal()` and re-emits SDK status `'compacting'`.
- `runForkedAgent({
    promptMessages: [summaryRequest],
    cacheSafeParams,
    canUseTool: createCompactCanUseTool(),
    querySource: 'compact',
    forkLabel: 'compact',
    maxTurns: 1,
    skipCacheWrite: true,
    overrides: { abortController: context.abortController }
  })` (lines 1188-1200). **No `maxOutputTokens` is set here**; setting it
  would break cache-key parity (comment lines 1182-1186).
- Success → emit `tengu_compact_cache_sharing_success` with cache
  hit-rate metrics; failure (no text response or error) → emit
  `tengu_compact_cache_sharing_fallback` with `reason: 'no_text_response' | 'error'`.
- Aborted forks return an `isApiErrorMessage` assistant — check that
  flag *before* `startsWithApiErrorPrefix` so ESC during cache-share isn't
  mis-recorded as a successful summary (lines 1208-1213).
- PTL response text passes through without success logging; the caller
  (`compactConversation`'s PTL retry loop) handles it.

**Path B (regular streaming, fallback):**

- Retry budget: `maxAttempts = retryEnabled ? MAX_COMPACT_STREAMING_RETRIES : 1`
  with `MAX_COMPACT_STREAMING_RETRIES = 2` (compact.ts:131) and `retryEnabled`
  bound to GrowthBook `tengu_compact_streaming_retry` default `false`.
- Tool list:
  ```
  if isToolSearchEnabled(model, ctx.options.tools, ()=>permCtx,
                        agentDefinitions.activeAgents, 'compact'):
     tools = uniqBy([FileReadTool, ToolSearchTool,
                    ...ctx.options.tools.filter(t => t.isMcp)], 'name')
  else:
     tools = [FileReadTool]
  ```
- `queryModelWithStreaming` invocation:
  ```
  messages: normalizeMessagesForAPI(
              stripImagesFromMessages(
                stripReinjectedAttachments([
                  ...getMessagesAfterCompactBoundary(messages),
                  summaryRequest,
                ])),
              context.options.tools)
  systemPrompt: asSystemPrompt(['You are a helpful AI assistant tasked with summarizing conversations.'])
  thinkingConfig: { type: 'disabled' }
  tools: <as above>
  signal: ctx.abortController.signal
  options: {
     getToolPermissionContext: () => appState.toolPermissionContext,
     model: ctx.options.mainLoopModel,
     toolChoice: undefined,
     isNonInteractiveSession: ctx.options.isNonInteractiveSession,
     hasAppendSystemPrompt: !!ctx.options.appendSystemPrompt,
     maxOutputTokensOverride:
        min(COMPACT_MAX_OUTPUT_TOKENS=20_000,
            getMaxOutputTokensForModel(model)),
     querySource: 'compact',
     agents: ctx.options.agentDefinitions.activeAgents,
     mcpTools: [],
     effortValue: appState.effortValue,
  }
  ```
- Stream loop: first `content_block_start` of type `text` flips
  `setStreamMode('responding')`; each `content_block_delta` of type
  `text_delta` increments `setResponseLength`; `event.type === 'assistant'`
  captures the response.
- Failure between attempts → emit `tengu_compact_streaming_retry`,
  `await sleep(getRetryDelay(attempt), abortSignal,
                { abortError: ()=>new APIUserAbortError() })`, then retry.
- Final failure → emit `tengu_compact_failed` with
  `reason: 'no_streaming_response'` and throw `ERROR_MESSAGE_INCOMPLETE_RESPONSE`.

### 5.7 PTL head-truncation algorithm (compact.ts:243-291)

```
groupMessagesByApiRound(input)         // input strips a leading
                                       // PTL_RETRY_MARKER user msg if present
if groups.length < 2: return null
tokenGap = getPromptTooLongTokenGap(ptlResponse)
if tokenGap !== undefined:
   acc = 0; dropCount = 0
   for g in groups:
      acc += roughTokenCountEstimationForMessages(g)
      dropCount++
      if acc >= tokenGap: break
else:
   dropCount = max(1, floor(groups.length * 0.20))
dropCount = min(dropCount, groups.length - 1)   // keep ≥1 group
if dropCount < 1: return null
sliced = groups.slice(dropCount).flat()
if sliced[0]?.type === 'assistant':
   return [createUserMessage({content: PTL_RETRY_MARKER, isMeta:true}),
           ...sliced]
return sliced
```

`PTL_RETRY_MARKER = '[earlier conversation truncated for compaction retry]'`
(compact.ts:228).

### 5.8 `partialCompactConversation` (compact.ts:772-1106)

Two `direction` values: `'from'` (default) summarizes suffix, keeps prefix;
`'up_to'` summarizes prefix, keeps suffix.

```
messagesToSummarize = direction === 'up_to'
   ? allMessages.slice(0, pivotIndex)
   : allMessages.slice(pivotIndex)

messagesToKeep =
  direction === 'up_to'
    ? allMessages.slice(pivotIndex)
        .filter(m => m.type !== 'progress'
                  && !isCompactBoundaryMessage(m)
                  && !(m.type === 'user' && m.isCompactSummary))
    : allMessages.slice(0, pivotIndex)
        .filter(m => m.type !== 'progress')

if messagesToSummarize.length === 0:
   throw 'Nothing to summarize before/after the selected message.'
```

`'up_to'` strips earlier compact boundaries from the kept tail to prevent
`findLastCompactBoundaryIndex`'s backward scan from dropping the new summary
(comments at lines 786-790). `'from'` keeps them: the new summary sits AFTER
kept and the backward scan still works.

PreCompact hook is `trigger: 'manual'`. Hook + user feedback merge:

```
if hookInstructions and userFeedback:
   custom = `${hookInstructions}\n\nUser context: ${userFeedback}`
elif hookInstructions: custom = hookInstructions
elif userFeedback:    custom = `User context: ${userFeedback}`
```

Cache strategy:

- `'up_to'`: send only `messagesToSummarize` to the API (prefix hits cache);
  `forkContextMessages = messagesToSummarize`.
- `'from'`: send the entire `allMessages` (tail wouldn't cache anyway);
  `cacheSafeParams` unchanged.

PTL retry: same algorithm and budget (`MAX_PTL_RETRIES = 3`); failure event is
`tengu_partial_compact_failed` with `path: 'partial'` on retries.

Anchor uuid for `annotateBoundaryWithPreservedSegment`:

```
direction === 'up_to' ? (summaryMessages.at(-1)?.uuid ?? boundaryMarker.uuid)
                      : boundaryMarker.uuid
```

`lastPreCompactUuid` (boundary `compactMetadata`):

```
direction === 'up_to'
  ? slice(0, pivotIndex).findLast(m => m.type !== 'progress')?.uuid
  : messagesToKeep.at(-1)?.uuid
```

Telemetry: `tengu_partial_compact` with `messagesKept`, `messagesSummarized`,
`direction`, `hasUserFeedback`, `trigger:'message_selector'`, plus compaction
usage breakdown.

### 5.9 `microcompactMessages` (microCompact.ts:253-293)

```
1. clearCompactWarningSuppression()
2. timeBased = maybeTimeBasedMicrocompact(messages, querySource)
   if timeBased: return timeBased       // short-circuits cached MC
3. if feature('CACHED_MICROCOMPACT'):
      mod = await getCachedMCModule()
      model = ctx?.options.mainLoopModel ?? getMainLoopModel()
      if mod.isCachedMicrocompactEnabled() and
         mod.isModelSupportedForCacheEditing(model) and
         isMainThreadSource(querySource):
         return await cachedMicrocompactPath(messages, querySource)
4. return { messages }    // legacy path removed; tengu_cache_plum_violet always true
   // NOTE: cached MC is skipped here, but time-based MC may already
   // have fired in step 2. External (non-ANT) builds and any source
   // failing isMainThreadSource still get time-based microcompact when
   // gapMinutes ≥ config.gapThresholdMinutes. The "no compaction here"
   // statement applies ONLY to the cached MC path; do not infer that
   // external builds receive zero microcompact pressure relief.
```

`isMainThreadSource(qs)` — true when `qs` is undefined or starts with
`'repl_main_thread'` (microCompact.ts:249-251). `evaluateTimeBasedTrigger` is
stricter and requires an explicit non-undefined main-thread source.

### 5.10 `maybeTimeBasedMicrocompact` (microCompact.ts:446-530)

```
trigger = evaluateTimeBasedTrigger(messages, querySource)
   // null if !config.enabled OR !querySource OR !isMainThreadSource
   //         OR no prior assistant OR gapMinutes < config.gapThresholdMinutes
if !trigger: return null
gapMinutes = trigger.gapMinutes; config = trigger.config
compactableIds = collectCompactableToolIds(messages)
keepRecent = max(1, config.keepRecent)        // floor at 1 (slice(-0) bug)
keepSet  = Set(compactableIds.slice(-keepRecent))
clearSet = Set(compactableIds.filter(id => !keepSet.has(id)))
if clearSet.size === 0: return null
result = messages.map(m =>
  m is user with array content and any tool_result whose id ∈ clearSet
    AND content !== TIME_BASED_MC_CLEARED_MESSAGE
    => replace those tool_result.content with TIME_BASED_MC_CLEARED_MESSAGE,
       accumulate calculateToolResultTokens(block) into tokensSaved)
if tokensSaved === 0: return null
logEvent('tengu_time_based_microcompact', {...})
suppressCompactWarning()
resetMicrocompactState()                       // clears cachedMC + pending
if feature('PROMPT_CACHE_BREAK_DETECTION') and querySource:
   notifyCacheDeletion(querySource)            // not notifyCompaction (CD reasons)
return { messages: result }
```

`TIME_BASED_MC_CLEARED_MESSAGE = '[Old tool result content cleared]'`
(line 36). `IMAGE_MAX_TOKEN_SIZE = 2000` per image/document block when
estimating tool-result tokens (line 38).

`COMPACTABLE_TOOLS` (line 41-50): `FileReadTool.name`, all
`SHELL_TOOL_NAMES`, `GrepTool.name`, `GlobTool.name`, `WebSearchTool.name`,
`WebFetchTool.name`, `FileEditTool.name`, `FileWriteTool.name`.

### 5.11 `cachedMicrocompactPath` (microCompact.ts:305-399)

```
mod    = await getCachedMCModule()
state  = ensureCachedMCState()
config = mod.getCachedMCConfig()
compactableToolIds = Set(collectCompactableToolIds(messages))

for m in messages:
  if m.type === 'user' and array content:
    groupIds = []
    for block in content:
      if block.type === 'tool_result'
         and compactableToolIds.has(block.tool_use_id)
         and !state.registeredTools.has(block.tool_use_id):
        mod.registerToolResult(state, block.tool_use_id)
        groupIds.push(block.tool_use_id)
    mod.registerToolMessage(state, groupIds)

toolsToDelete = mod.getToolResultsToDelete(state)
if toolsToDelete.length > 0:
   cacheEdits = mod.createCacheEditsBlock(state, toolsToDelete)
   if cacheEdits: pendingCacheEdits = cacheEdits     // module-level
   logForDebugging(`Cached MC deleting ${n} tool(s): ${ids}`)
   logEvent('tengu_cached_microcompact', {
     toolsDeleted, deletedToolIds, activeToolCount,
     triggerType:'auto',
     threshold: config.triggerThreshold,
     keepRecent: config.keepRecent,
   })
   suppressCompactWarning()
   if feature('PROMPT_CACHE_BREAK_DETECTION'):
      notifyCacheDeletion(querySource ?? 'repl_main_thread')
   baseline = lastAsst.message.usage.cache_deleted_input_tokens ?? 0
   return { messages,
     compactionInfo: { pendingCacheEdits: { trigger:'auto', deletedToolIds, baselineCacheDeletedTokens: baseline } } }

return { messages }
```

Caller (`query.ts:866-893`) defers the boundary message until **after** the API
response, then yields:

```
cumulativeDeleted = lastAsst?.usage.cache_deleted_input_tokens ?? 0
deletedTokens = max(0, cumulativeDeleted - baselineCacheDeletedTokens)
if deletedTokens > 0:
   yield createMicrocompactBoundaryMessage(
            pendingCacheEdits.trigger,    // 'auto'
            0, deletedTokens, deletedToolIds, [])
```

### 5.12 `trySessionMemoryCompaction` (sessionMemoryCompact.ts:514-630)

```
if !shouldUseSessionMemoryCompaction(): return null
   // env override OR (gb('tengu_session_memory') && gb('tengu_sm_compact'))
await initSessionMemoryCompactConfig()              // GB tengu_sm_compact_config
await waitForSessionMemoryExtraction()
lastSummarizedMessageId = getLastSummarizedMessageId()
sessionMemory          = await getSessionMemoryContent()
if !sessionMemory:           log tengu_sm_compact_no_session_memory; return null
if isSessionMemoryEmpty(sm): log tengu_sm_compact_empty_template;    return null

if lastSummarizedMessageId:
   lastSummarizedIndex = messages.findIndex(uuid === id)
   if -1: log tengu_sm_compact_summarized_id_not_found; return null
else:
   lastSummarizedIndex = messages.length - 1   // resumed-session path
   log tengu_sm_compact_resumed_session

startIndex = calculateMessagesToKeepIndex(messages, lastSummarizedIndex)
messagesToKeep = messages.slice(startIndex)
                  .filter(m => !isCompactBoundaryMessage(m))
hookResults = await processSessionStartHooks('compact', { model:getMainLoopModel() })
result = createCompactionResultFromSessionMemory(messages, sessionMemory,
            messagesToKeep, hookResults, getTranscriptPath(), agentId)
postCompact = buildPostCompactMessages(result)
postCompactTokenCount = estimateMessageTokens(postCompact)
if autoCompactThreshold !== undefined and postCompactTokenCount >= threshold:
   log tengu_sm_compact_threshold_exceeded; return null
return { ...result, postCompactTokenCount, truePostCompactTokenCount: same }
catch -> log tengu_sm_compact_error; return null
```

`calculateMessagesToKeepIndex` (lines 324-397):

```
config = getSessionMemoryCompactConfig()  // {minTokens, minTextBlockMessages, maxTokens}
startIndex = lastSummarizedIndex >= 0 ? lastSummarizedIndex+1 : messages.length
totalTokens, textBlockMessageCount = sum over [startIndex..messages.length)
if totalTokens >= config.maxTokens:
   return adjustIndexToPreserveAPIInvariants(messages, startIndex)
if totalTokens >= config.minTokens && textBlockMessageCount >= config.minTextBlockMessages:
   return adjust...
floor = lastIndex of compactBoundaryMessage + 1, or 0
for i = startIndex-1 downTo floor:
   add msg tokens; bump count if hasTextBlocks
   startIndex = i
   if totalTokens >= config.maxTokens: break
   if totalTokens >= config.minTokens && count >= config.minTextBlockMessages: break
return adjustIndexToPreserveAPIInvariants(messages, startIndex)
```

`adjustIndexToPreserveAPIInvariants` (lines 232-314): two passes.

1. Tool-pair pass — collect tool_result IDs from kept range, compute
   `neededToolUseIds` = ids not already paired in kept range, then walk
   backwards from `adjustedIndex - 1`; whenever a message has a `tool_use`
   matching, set `adjustedIndex = i` and remove satisfied IDs, until the set
   empties.
2. Thinking-block pass — collect `message.id`s of kept assistant messages,
   then walk backwards from `adjustedIndex - 1`; whenever an assistant message
   shares a `message.id` with any kept assistant, set `adjustedIndex = i`.

### 5.13 `runPostCompactCleanup` (postCompactCleanup.ts:31-77)

```
isMainThreadCompact = querySource === undefined
                     OR querySource.startsWith('repl_main_thread')
                     OR querySource === 'sdk'

resetMicrocompactState()
if feature('CONTEXT_COLLAPSE') and isMainThreadCompact:
   contextCollapse.resetContextCollapse()
if isMainThreadCompact:
   getUserContext.cache.clear?.()
   resetGetMemoryFilesCache('compact')
clearSystemPromptSections()
clearClassifierApprovals()
clearSpeculativeChecks()
// resetSentSkillNames is NOT called (cache_creation cost rationale)
clearBetaTracingState()
if feature('COMMIT_ATTRIBUTION'):
   import('utils/attributionHooks.js').then(m => m.sweepFileContentCache())
clearSessionMessagesCache()
```

**Cross-spec ripple — subagent compact ≠ main-thread compact (spec 05).**
The `isMainThreadCompact` gate (postCompactCleanup.ts:36-39) is not just a
defensive guard; it has a **functional consequence the reader must not
miss**. When `querySource` starts with `agent:*`, the cleanup
intentionally **skips both `getUserContext.cache.clear?.()` and
`resetGetMemoryFilesCache('compact')`** (postCompactCleanup.ts:51-61).

Concrete consequence: a subagent compaction that mutates `MEMORY.md` (via
a tool call inside the subagent) **leaves the main thread's
`getUserContext` cache stale** until the next main-thread compact, /clear,
or process restart. `getUserContext` is a memoized outer layer wrapping
`getClaudeMds() → getMemoryFiles()`; clearing only the inner
`getMemoryFiles` cache is insufficient because subsequent main-thread
turns hit the outer memo and never re-read disk.

This is the rationale for the gate — subagents share module-level state
in-process, and resetting it from a SUBAGENT compact would corrupt
main-thread state. The trade-off is accepted: subagents may observe
**different memory state than the main thread** between compacts. Spec 05
(`getUserContext` / system prompt) cross-references this gate. See also
spec 05 §X for the symmetric note on `getMemoryFiles` one-shot
`InstructionsLoaded` hook.

### 5.14 Reactive compact / 413 recovery (caller side, query.ts:1080-1180)

```
isWithheld413  = reactiveCompact?.isWithheldPromptTooLong(lastMessage)
isWithheldMedia = mediaRecoveryEnabled
                  && reactiveCompact?.isWithheldMediaSizeError(lastMessage)
mediaRecoveryEnabled = reactiveCompact?.isReactiveCompactEnabled() ?? false

if isWithheld413:
   if feature('CONTEXT_COLLAPSE') && contextCollapse
      && state.transition?.reason !== 'collapse_drain_retry':
       drained = contextCollapse.recoverFromOverflow(messagesForQuery, querySource)
       if drained.committed > 0:
          continue with messages=drained.messages,
                       transition={'collapse_drain_retry', committed}

if (isWithheld413 || isWithheldMedia) && reactiveCompact:
   compacted = await reactiveCompact.tryReactiveCompact({
     hasAttempted: hasAttemptedReactiveCompact,
     querySource,
     aborted: abortSignal.aborted,
     messages: messagesForQuery,
     cacheSafeParams: { systemPrompt, userContext, systemContext,
                        toolUseContext, forkContextMessages: messagesForQuery },
   })
   if compacted:
     decrement task budget by finalContextTokensFromLastResponse(messagesForQuery)
     postCompactMessages = buildPostCompactMessages(compacted)
     yield each postCompactMessages
     continue with hasAttemptedReactiveCompact=true,
                   transition={'reactive_compact_retry'}
   else:
     yield lastMessage
     void executeStopFailureHooks(lastMessage, ctx)
     return { reason: isWithheldMedia ? 'image_error' : 'prompt_too_long' }

elif feature('CONTEXT_COLLAPSE') && isWithheld413:
   yield lastMessage  // collapse withheld and could not recover
   ...
```

Pre-stream blocking-limit guard (query.ts:626-645): when reactive compact is
NOT eligible (compiled out, disabled, or autocompact disabled) and collapse
doesn't own it, the loop yields a synthetic
`PROMPT_TOO_LONG_ERROR_MESSAGE` and returns
`{ reason: 'blocking_limit' }` if `tokenCountWithEstimation(messagesForQuery) - snipTokensFreed`
exceeds `blockingLimit`.

### 5.15 Snip integration (caller side, query.ts:401-410, QueryEngine.ts:122-127, 1276)

`snipModule.snipCompactIfNeeded(store, opts?)` returns
`{ messages, tokensFreed, boundaryMessage? }`. Called pre-microcompact every
turn; the boundary message is yielded inline. `QueryEngine.ts` uses
`snipReplay` callback (lines 1268-1281): when the streaming-replay layer
yields a snip-boundary message, the SnipTool (registry-side) is allowed to
re-apply with `{ force: true }`. Detection helper:
`snipProjection.isSnipBoundaryMessage(yielded)`.

Other ways snip threads through:

- `attachments.ts:934` injects `getContextEfficiencyAttachment(messages)`
  whose attachment type `'context_efficiency'` resolves to the
  `SNIP_NUDGE_TEXT` from `snipCompact.ts` wrapped in a system-reminder
  (messages.ts:4148-4159).
- `snipTokensFreed` is plumbed to `shouldAutoCompact`/`autoCompactIfNeeded`
  so the threshold check accounts for what snip already removed
  (autoCompact.ts:165-168, 222-227).
- `tools.ts:123-124` registers `SnipTool` only when `feature('HISTORY_SNIP')`
  is on; the source file is missing in the leak (overview §2.5).
- `commands.ts:83` exposes `forceSnip` command-side under the same flag.

### 5.16 Compaction reminder attachment (`getCompactionReminderAttachment`,
attachments.ts:3931-3955)

```
if !gb('tengu_marble_fox', false):                    return []
if !isAutoCompactEnabled():                            return []
if getContextWindowForModel(model, getSdkBetas()) < 1_000_000: return []
effectiveWindow = getEffectiveContextWindowSize(model)
usedTokens      = tokenCountWithEstimation(messages)
if usedTokens < effectiveWindow * 0.25:                return []
return [{ type: 'compaction_reminder' }]
```

The attachment is wired in at `attachments.ts:922-933` only under
`feature('COMPACTION_REMINDERS')`. The renderer in `messages.ts:4139-4147`
emits a system-reminder-wrapped user message with verbatim text (see §6.4).

### 5.17 Context-collapse delta (caller side; service body missing)

Compaction-relevant integration points:

- `query.ts:440-446` — between microcompact and autocompact, calls
  `contextCollapse.applyCollapsesIfNeeded(messages, ctx, querySource)` and
  replaces `messagesForQuery` with the projected view.
- `autoCompact.ts:179-183` — `marble_origami` (the ctx-agent subagent) never
  triggers proactive autocompact; otherwise its compact would clobber the
  shared committed-collapse log.
- `autoCompact.ts:215-223` — when `isContextCollapseEnabled()` returns true,
  proactive autocompact is suppressed; the 90%/95% commit/blocking-spawn flow
  takes over.
- `query.ts:1090-1117` — on a real 413, drain staged collapses first; only on
  the second consecutive 413 (via `transition.reason === 'collapse_drain_retry'`)
  does control fall through to reactive compact.
- `postCompactCleanup.ts:42-49` — main-thread-only call to
  `resetContextCollapse()` after every successful compact.

### 5.18 Compact warning state machine

```
on entry to microcompactMessages:                       store := false
on cachedMicrocompactPath success / time-based success: store := true
       (suppressCompactWarning)
on next entry to microcompactMessages:                  store := false
React UI consumes via useCompactWarningSuppression()
```

---

## 6. Verbatim Assets

### 6.1 Summarizer system prompt

```
You are a helpful AI assistant tasked with summarizing conversations.
```

(compact.ts:1302-1304, single-element array passed to `asSystemPrompt`)

### 6.2 No-tools preamble (prepended to every summarizer user prompt)

```
CRITICAL: Respond with TEXT ONLY. Do NOT call any tools.

- Do NOT use Read, Bash, Grep, Glob, Edit, Write, or ANY other tool.
- You already have all the context you need in the conversation above.
- Tool calls will be REJECTED and will waste your only turn — you will fail the task.
- Your entire response must be plain text: an <analysis> block followed by a <summary> block.

```

(prompt.ts:19-26; trailing blank line is part of the literal)

### 6.3 No-tools trailer (appended to every summarizer user prompt)

```


REMINDER: Do NOT call any tools. Respond with plain text only — an <analysis> block followed by a <summary> block. Tool calls will be rejected and you will fail the task.
```

(prompt.ts:269-272; double-newline prefix is part of the trailer string)

### 6.4 Detailed-analysis instruction (BASE — full transcript)

```
Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like:
     - file names
     - full code snippets
     - function signatures
     - file edits
   - Errors that you ran into and how you fixed them
   - Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.
```

(prompt.ts:31-44)

### 6.5 Detailed-analysis instruction (PARTIAL — recent slice)

```
Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Analyze the recent messages chronologically. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like:
     - file names
     - full code snippets
     - function signatures
     - file edits
   - Errors that you ran into and how you fixed them
   - Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.
```

(prompt.ts:46-59)

### 6.6 BASE compact prompt (`getCompactPrompt`)

Full template body verbatim — `prompt.ts:61-143`. The runtime composition is:

```
NO_TOOLS_PREAMBLE
+ BASE_COMPACT_PROMPT
+ (customInstructions trim != '' ? '\n\nAdditional Instructions:\n' + customInstructions : '')
+ NO_TOOLS_TRAILER
```

`BASE_COMPACT_PROMPT` literal (prompt.ts:61-143):

```
Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.
This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing development work without losing context.

<DETAILED_ANALYSIS_INSTRUCTION_BASE>

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail
2. Key Technical Concepts: List all important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Pay special attention to the most recent messages and include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Errors and fixes: List all errors that you ran into, and how you fixed them. Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages that are not tool results. These are critical for understanding the users' feedback and changing intent.
7. Pending Tasks: Outline any pending tasks that you have explicitly been asked to work on.
8. Current Work: Describe in detail precisely what was being worked on immediately before this summary request, paying special attention to the most recent messages from both user and assistant. Include file names and code snippets where applicable.
9. Optional Next Step: List the next step that you will take that is related to the most recent work you were doing. IMPORTANT: ensure that this step is DIRECTLY in line with the user's most recent explicit requests, and the task you were working on immediately before this summary request. If your last task was concluded, then only list next steps if they are explicitly in line with the users request. Do not start on tangential requests or really old requests that were already completed without confirming with the user first.
                       If there is a next step, include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off. This should be verbatim to ensure there's no drift in task interpretation.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]
   - [...]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Summary of the changes made to this file, if any]
      - [Important Code Snippet]
   - [File Name 2]
      - [Important Code Snippet]
   - [...]

4. Errors and fixes:
    - [Detailed description of error 1]:
      - [How you fixed the error]
      - [User feedback on the error if any]
    - [...]

5. Problem Solving:
   [Description of solved problems and ongoing troubleshooting]

6. All user messages: 
    - [Detailed non tool use user message]
    - [...]

7. Pending Tasks:
   - [Task 1]
   - [Task 2]
   - [...]

8. Current Work:
   [Precise description of current work]

9. Optional Next Step:
   [Optional Next step to take]

</summary>
</example>

Please provide your summary based on the conversation so far, following this structure and ensuring precision and thoroughness in your response. 

There may be additional summarization instructions provided in the included context. If so, remember to follow these instructions when creating the above summary. Examples of instructions include:
<example>
## Compact Instructions
When summarizing the conversation focus on typescript code changes and also remember the mistakes you made and how you fixed them.
</example>

<example>
# Summary instructions
When you are using compact - please focus on test output and code changes. Include file reads verbatim.
</example>
```

(Note: the line ending sections 1–9 has a trailing space: "...response. " then `\n\n`. The interpolation `${DETAILED_ANALYSIS_INSTRUCTION_BASE}` is replaced by §6.4 verbatim.)

### 6.7 Partial compact prompt (`PARTIAL_COMPACT_PROMPT`, direction='from')

Full literal body (prompt.ts:145-204) — `getPartialCompactPrompt(custom, 'from')` composes
`NO_TOOLS_PREAMBLE + PARTIAL_COMPACT_PROMPT + Additional…? + NO_TOOLS_TRAILER`:

```
Your task is to create a detailed summary of the RECENT portion of the conversation — the messages that follow earlier retained context. The earlier messages are being kept intact and do NOT need to be summarized. Focus your summary on what was discussed, learned, and accomplished in the recent messages only.

<DETAILED_ANALYSIS_INSTRUCTION_PARTIAL>

Your summary should include the following sections:

1. Primary Request and Intent: Capture the user's explicit requests and intents from the recent messages
2. Key Technical Concepts: List important technical concepts, technologies, and frameworks discussed recently.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Errors and fixes: List errors encountered and how they were fixed.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages from the recent portion that are not tool results.
7. Pending Tasks: Outline any pending tasks from the recent messages.
8. Current Work: Describe precisely what was being worked on immediately before this summary request.
9. Optional Next Step: List the next step related to the most recent work. Include direct quotes from the most recent conversation.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Important Code Snippet]

4. Errors and fixes:
    - [Error description]:
      - [How you fixed it]

5. Problem Solving:
   [Description]

6. All user messages:
    - [Detailed non tool use user message]

7. Pending Tasks:
   - [Task 1]

8. Current Work:
   [Precise description of current work]

9. Optional Next Step:
   [Optional Next step to take]

</summary>
</example>

Please provide your summary based on the RECENT messages only (after the retained earlier context), following this structure and ensuring precision and thoroughness in your response.
```

### 6.8 Partial compact prompt (`PARTIAL_COMPACT_UP_TO_PROMPT`, direction='up_to')

prompt.ts:208-267 verbatim:

```
Your task is to create a detailed summary of this conversation. This summary will be placed at the start of a continuing session; newer messages that build on this context will follow after your summary (you do not see them here). Summarize thoroughly so that someone reading only your summary and then the newer messages can fully understand what happened and continue the work.

<DETAILED_ANALYSIS_INSTRUCTION_BASE>

Your summary should include the following sections:

1. Primary Request and Intent: Capture the user's explicit requests and intents in detail
2. Key Technical Concepts: List important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Errors and fixes: List errors encountered and how they were fixed.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages that are not tool results.
7. Pending Tasks: Outline any pending tasks.
8. Work Completed: Describe what was accomplished by the end of this portion.
9. Context for Continuing Work: Summarize any context, decisions, or state that would be needed to understand and continue the work in subsequent messages.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Important Code Snippet]

4. Errors and fixes:
    - [Error description]:
      - [How you fixed it]

5. Problem Solving:
   [Description]

6. All user messages:
    - [Detailed non tool use user message]

7. Pending Tasks:
   - [Task 1]

8. Work Completed:
   [Description of what was accomplished]

9. Context for Continuing Work:
   [Key context, decisions, or state needed to continue the work in subsequent messages.]

</summary>
</example>

Please provide your summary following this structure, ensuring precision and thoroughness in your response.
```

### 6.9 `getCompactUserSummaryMessage` template

(prompt.ts:337-374)

Base form:

```
This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.

<formatCompactSummary(summary)>
```

If `transcriptPath` provided, append:

```


If you need specific details from before compaction (like exact code snippets, error messages, or content you generated), read the full transcript at: <transcriptPath>
```

If `recentMessagesPreserved`, append:

```


Recent messages are preserved verbatim.
```

If `suppressFollowUpQuestions`, append (REPLACES the bare base; concatenated to it):

```

Continue the conversation from where it left off without asking the user any further questions. Resume directly — do not acknowledge the summary, do not recap what was happening, do not preface with "I'll continue" or similar. Pick up the last task as if the break never happened.
```

If proactive/Kairos active AND suppression-on, additionally append:

```


You are running in autonomous/proactive mode. This is NOT a first wake-up — you were already working autonomously before compaction. Continue your work loop: pick up where you left off based on the summary above. Do not greet the user or ask what to work on.
```

### 6.10 `formatCompactSummary` (prompt.ts:311-335)

Strips `<analysis>...</analysis>` (single regex `/<analysis>[\s\S]*?<\/analysis>/`), extracts
`<summary>...</summary>` and replaces with `Summary:\n<inner>`, then collapses
`/\n\n+/g → \n\n`, then `.trim()`.

### 6.11 Compaction reminder text (verbatim)

The renderer (messages.ts:4139-4147) wraps a meta user message in a
system-reminder envelope:

```
Auto-compact is enabled. When the context window is nearly full, older messages will be automatically summarized so you can continue working seamlessly. There is no need to stop or rush — you have unlimited context through automatic compaction.
```

(The `—` in the source decodes to em-dash `—`.)

### 6.12 Constants table

| Name | Value | Cite |
|---|---|---|
| `COMPACT_MAX_OUTPUT_TOKENS` | `20_000` | `utils/context.ts:12` |
| `MAX_OUTPUT_TOKENS_FOR_SUMMARY` | `20_000` | `autoCompact.ts:30` |
| `AUTOCOMPACT_BUFFER_TOKENS` | `13_000` | `autoCompact.ts:62` |
| `WARNING_THRESHOLD_BUFFER_TOKENS` | `20_000` | `autoCompact.ts:63` |
| `ERROR_THRESHOLD_BUFFER_TOKENS` | `20_000` | `autoCompact.ts:64` |
| `MANUAL_COMPACT_BUFFER_TOKENS` | `3_000` | `autoCompact.ts:65` |
| `MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES` | `3` | `autoCompact.ts:70` |
| `POST_COMPACT_MAX_FILES_TO_RESTORE` | `5` | `compact.ts:122` |
| `POST_COMPACT_TOKEN_BUDGET` | `50_000` | `compact.ts:123` |
| `POST_COMPACT_MAX_TOKENS_PER_FILE` | `5_000` | `compact.ts:124` |
| `POST_COMPACT_MAX_TOKENS_PER_SKILL` | `5_000` | `compact.ts:129` |
| `POST_COMPACT_SKILLS_TOKEN_BUDGET` | `25_000` | `compact.ts:130` |
| `MAX_COMPACT_STREAMING_RETRIES` | `2` | `compact.ts:131` |
| `MAX_PTL_RETRIES` | `3` | `compact.ts:227` |
| `PTL_RETRY_MARKER` | `'[earlier conversation truncated for compaction retry]'` | `compact.ts:228` |
| `IMAGE_MAX_TOKEN_SIZE` | `2000` | `microCompact.ts:38` |
| `TIME_BASED_MC_CLEARED_MESSAGE` | `'[Old tool result content cleared]'` | `microCompact.ts:36` |
| activity keep-alive interval | `30_000 ms` | `compact.ts:1167-1175` |
| token-estimate padding factor | `4/3` | `microCompact.ts:204` |
| `IMAGE_MAX_TOKEN_SIZE` (in apiMicrocompact context: `DEFAULT_MAX_INPUT_TOKENS`) | `180_000` | `apiMicrocompact.ts:16` |
| `DEFAULT_TARGET_INPUT_TOKENS` | `40_000` | `apiMicrocompact.ts:17` |
| `DEFAULT_SM_COMPACT_CONFIG.minTokens` | `10_000` | `sessionMemoryCompact.ts:58` |
| `DEFAULT_SM_COMPACT_CONFIG.minTextBlockMessages` | `5` | `sessionMemoryCompact.ts:59` |
| `DEFAULT_SM_COMPACT_CONFIG.maxTokens` | `40_000` | `sessionMemoryCompact.ts:60` |
| `TIME_BASED_MC_CONFIG_DEFAULTS.enabled` | `false` | `timeBasedMCConfig.ts:31` |
| `TIME_BASED_MC_CONFIG_DEFAULTS.gapThresholdMinutes` | `60` | `timeBasedMCConfig.ts:32` |
| `TIME_BASED_MC_CONFIG_DEFAULTS.keepRecent` | `5` | `timeBasedMCConfig.ts:33` |
| compaction-reminder window threshold | `1_000_000` | `attachments.ts:3946` |
| compaction-reminder usage gate | `effectiveWindow * 0.25` | `attachments.ts:3953` |

### 6.13 Skill-truncation marker

```


[... skill content truncated for compaction; use Read on the skill path if you need the full text]
```

(`compact.ts:1657-1658`; literal includes leading `\n\n`.)

### 6.14 GrowthBook gates consulted

| GB key | Default | Effect | Cite |
|---|---|---|---|
| `tengu_compact_cache_prefix` | `true` | enable forked-agent cache-sharing path | `compact.ts:435,1155-1158` |
| `tengu_compact_streaming_retry` | `false` | enable streaming-fallback retries (count = 2) | `compact.ts:1251-1253` |
| `tengu_cobalt_raccoon` | `false` | reactive-only mode — suppress proactive autocompact | `autoCompact.ts:196` |
| `tengu_session_memory` | `false` | (precondition) for SM-compact | `sessionMemoryCompact.ts:412` |
| `tengu_sm_compact` | `false` | enable SM-compact path | `sessionMemoryCompact.ts:417` |
| `tengu_sm_compact_config` | `{}` | tuning of `{minTokens, minTextBlockMessages, maxTokens}` | `sessionMemoryCompact.ts:111` |
| `tengu_slate_heron` | `TIME_BASED_MC_CONFIG_DEFAULTS` | time-based microcompact config | `timeBasedMCConfig.ts:39-42` |
| `tengu_marble_fox` | `false` | enable compaction reminder attachment | `attachments.ts:3935` |

### 6.15 Error messages (verbatim)

| Symbol | Text | Cite |
|---|---|---|
| `ERROR_MESSAGE_NOT_ENOUGH_MESSAGES` | `Not enough messages to compact.` | `compact.ts:225-226` |
| `ERROR_MESSAGE_PROMPT_TOO_LONG` | `Conversation too long. Press esc twice to go up a few messages and try again.` | `compact.ts:293-294` |
| `ERROR_MESSAGE_USER_ABORT` | `API Error: Request was aborted.` | `compact.ts:295` |
| `ERROR_MESSAGE_INCOMPLETE_RESPONSE` | `Compaction interrupted · This may be due to network issues — please try again.` | `compact.ts:296-297` |
| Notification body | `Error compacting conversation` (priority `'immediate'`, color `'error'`, key `'error-compacting-conversation'`) | `compact.ts:1116-1121` |
| Partial-compact PreCompact-only message | `Nothing to summarize before the selected message.` | `compact.ts:803-805` |
| Partial-compact (other dir) | `Nothing to summarize after the selected message.` | `compact.ts:806` |
| Compact CanUseTool deny | `Tool use is not allowed during compaction` (decisionReason: `compaction agent should only produce text summary`) | `compact.ts:1126-1133` |
| Compact summary not found | `Failed to generate conversation summary - response did not contain valid text content` | `compact.ts:504-505, 905-907` |

---

## 7. Side Effects & I/O

- **PreCompact / PostCompact / SessionStart hooks** — fan out via
  `executePreCompactHooks` (utils/hooks.ts:3961), `executePostCompactHooks`
  (utils/hooks.ts:4034), `processSessionStartHooks('compact', { model })`
  (utils/sessionStart.ts). Each runs once per `compactConversation` and once
  per `partialCompactConversation` invocation. `runPostCompactCleanup`
  is invoked **outside** `compactConversation` by all callers
  (`autoCompactIfNeeded`, command-side `/compact` per `commands/compact/compact.ts`,
  reactive compact per the comment at line 197). PreCompact hook input:
  `{ trigger: 'auto'|'manual', customInstructions: string|null }`.
  PostCompact hook input: `{ trigger, compactSummary }`.
- **Per-compact filesystem touches** via attachments:
  `generateFileAttachment` (re-Read recently-accessed files capped at 5 files
  / 5_000 tokens each / 50_000 tokens total), `getMemoryPath`,
  `getPlanFilePath`. Memory-type files and the plan file are excluded from
  post-compact restoration (compact.ts:1674-1705).
- **Per-compact session disk touches**: `reAppendSessionMetadata()`
  (compact.ts:711, 1057) — re-writes session title/tag entry within the 16KB
  tail window scanned by `--resume`'s `readLiteMetadata`.
- **Session transcript writeback** (KAIROS): `void
  sessionTranscriptModule?.writeSessionTranscriptSegment(messages)` —
  fire-and-forget, errors swallowed.
- **Network**: a single `runForkedAgent`-driven Anthropic API request (Path A)
  or a `queryModelWithStreaming` SSE request (Path B). Messages are stripped
  of images (replaced by `[image]` / `[document]`) and skill_discovery /
  skill_listing attachments before transmission (compact.ts:145-223).
- **In-process state mutations** during compaction:
  - `context.readFileState.clear()` and `context.loadedNestedMemoryPaths?.clear()`.
  - `appState.tasks` read for async-agent attachments.
  - `pendingCacheEdits` (module-level, microCompact.ts) is set/cleared.
  - `compactWarningStore` set true; cleared on next microcompact entry.
  - `markPostCompaction()` (`bootstrap/state.ts`).
- **Activity keep-alive timer** during forked-agent path: `setInterval`
  ticks every 30_000 ms re-emitting `setSDKStatus('compacting')` and
  `sendSessionActivitySignal()` to keep remote-session WS alive
  (compact.ts:1167-1175); cleared in `finally` at line 1394.
- **Notifications**: `addNotification({key:'error-compacting-conversation',
  text:'Error compacting conversation', priority:'immediate', color:'error'})`
  on manual-failure path only (compact.ts:1108-1123).
- **Trust boundary**: the summarizer prompt forbids tool calls; a
  `createCompactCanUseTool()` injects a hard `behavior:'deny'` and
  `decisionReason:{ type:'other', reason:'compaction agent should only produce
  text summary' }` (compact.ts:1125-1134). `FileReadTool` is the only tool in
  the tool list (or `[FileReadTool, ToolSearchTool, ...mcpTools]` when tool
  search is enabled), but the deny gate prevents any actual invocation.

### 7.1 Cost surface (Phase 9.7 §13.1 ripple)

Compaction is a structural cost-multiplier surface. The following
mechanisms exist explicitly to manage `cache_creation_input_tokens`
spike on the post-compact turn; cross-reference Phase 9.7 §13.1
(cost-multiplier audit) for fleet-level impact.

- **`sentSkillNames` is intentionally NOT reset post-compact** —
  compact.ts:524-529 + postCompactCleanup.ts:65-69. Re-injecting the full
  `skill_listing` (~4K tokens) every compact would be pure
  `cache_creation` with marginal benefit: the model still has `SkillTool`
  in its schema, the `invoked_skills` attachment preserves used-skill
  content, and ANTs with `EXPERIMENTAL_SKILL_SEARCH` skip re-injection
  entirely. Keeping the set populated across compacts is a deliberate
  cache-creation savings, not a bug.

- **`tengu_compact_cache_prefix` GrowthBook gate, 3P default `true`** —
  compact.ts:431-438 + 1154-1158. The `false` path bypasses the
  forked-agent / cache-sharing route and runs `queryModelWithStreaming`
  directly; per the source comment, that path is **98% cache miss and
  costs ~0.76% of fleet `cache_creation` (~38B tok/day, concentrated in
  ephemeral envs: CCR / GHA / SDK)**. Default-`true` keeps the cheap
  forked-agent path active; `false` is a kill-switch only.

- **`notifyCompaction()` + `markPostCompaction()` plumbing** —
  compact.ts:698-704, 1047-1053; microCompact.ts:362-367, 525-527;
  autoCompact.ts:302-304. These calls do NOT reduce token cost directly;
  they exist to **suppress false-positive cache-break events** in spec 03's
  `promptCacheBreakDetection`. Without them, every compact would log a
  spurious cache-break (because the prefix legitimately changes) and
  poison fleet-level cache-health telemetry. Gated by
  `feature('PROMPT_CACHE_BREAK_DETECTION')`.

- **Per-path cost telemetry**: `tengu_compact` event payload includes
  `compactionUsage.cache_creation_input_tokens` (compact.ts:674,
  compact.ts:1004, compact.ts:1219-1224); cache-sharing path emits
  `tengu_compact_cache_sharing_success` with hit-rate metrics for fleet
  attribution.

- **Compact uses `mainLoopModel`, NOT Haiku** — every API call inside
  `src/services/compact/` flows through `queryModelWithStreaming` with
  `model: ctx.options.mainLoopModel` (compact.ts:593, 982, 1313). Zero
  `queryHaiku` references in this subsystem. Cross-ref spec 22
  (`queryHaiku` / model dispatch) — compaction is excluded from Haiku
  routing; cost rolls up to the main-loop model's pricing tier.

---

## 8. Feature Flags & Variants

### 8.1 `feature('REACTIVE_COMPACT')`

- Off (external): `reactiveCompact` module is null. The pre-stream
  blocking-limit guard fires at `effectiveWindow - 3_000` and yields a
  synthetic `PROMPT_TOO_LONG_ERROR_MESSAGE`. There is no withhold/replay of
  413s; the API error surfaces directly.
- On (ANT-only): `reactiveCompact` is loaded. Withholding kicks in for
  `prompt_too_long` and (when `isReactiveCompactEnabled()`) media-size errors;
  on withhold the loop calls `tryReactiveCompact(...)`. If
  `tengu_cobalt_raccoon` is true under this flag, **proactive** autocompact is
  suppressed entirely, leaving reactive as the sole compactor. Pre-stream
  blocking-limit guard is also suppressed when reactive is enabled and
  `isAutoCompactEnabled()` is true.

### 8.2 `feature('CACHED_MICROCOMPACT')`

- Off: `microcompactMessages` returns the input messages unchanged after
  the time-based check; no cache_edits block is queued.
- On (ANT-only): `cachedMicrocompactPath` runs after time-based, but only for
  main-thread sources (`undefined` or `'repl_main_thread'`-prefixed) and only
  for models where `isModelSupportedForCacheEditing(model)` is true. Tool
  results in `COMPACTABLE_TOOLS` are deleted via API `cache_edits` instead of
  message-mutation; the boundary message is deferred until after the API
  response so `cache_deleted_input_tokens` (via baseline subtraction) drives
  the boundary's deletedTokens count.

### 8.3 `feature('HISTORY_SNIP')`

- Off: snip module is null; no per-turn snip; no snip nudge
  (`context_efficiency` attachment is empty); `SnipTool` is excluded from
  `tools.ts`.
- On (ANT-only): per-turn `snipCompactIfNeeded` runs before microcompact;
  `snipReplay` callback can re-apply snip on a snip-boundary replay
  (`QueryEngine.ts:1268-1281`); `getContextEfficiencyAttachment` injects the
  `SNIP_NUDGE_TEXT` after every N tokens of growth; `snipTokensFreed` flows
  into autocompact threshold check.

### 8.4 `feature('CONTEXT_COLLAPSE')`

- Off: no collapse projection; no marble_origami carve-out; reactive compact
  alone handles 413; `postCompactCleanup` skips `resetContextCollapse()`.
- On: `applyCollapsesIfNeeded` runs between microcompact and autocompact;
  `marble_origami` subagent can never trigger autocompact;
  `isContextCollapseEnabled()` short-circuits proactive autocompact entirely;
  on a real 413 collapse drains staged commits FIRST and only the second
  consecutive 413 hands off to reactive compact; main-thread compact resets
  collapse module state.

### 8.5 `feature('COMPACTION_REMINDERS')`

- Off: `getCompactionReminderAttachment` is never called; no reminder text is
  injected.
- On: gated additionally by GB `tengu_marble_fox` (default false), 1M+ context
  window models, and 25%-of-effective-window-used. When all gates pass, a
  `compaction_reminder` attachment is injected per turn.

### 8.6 `feature('PROACTIVE')` / `feature('KAIROS')`

- Off: `getCompactUserSummaryMessage` does NOT append the autonomous-mode
  paragraph regardless of `suppressFollowUpQuestions`.
- On + `proactiveModule.isProactiveActive()` true + `suppressFollowUpQuestions`:
  the autonomous-mode paragraph is appended.
- KAIROS additionally enables `sessionTranscriptModule.writeSessionTranscriptSegment`
  fire-and-forget in `compactConversation` and `partialCompactConversation`.

### 8.7 `feature('PROMPT_CACHE_BREAK_DETECTION')`

- Off: no `notifyCompaction` / `notifyCacheDeletion` calls.
- On: every successful compact (manual, auto, partial, SM, time-based MC, cached
  MC) calls the appropriate notify so the prompt-cache-break detector
  classifies the resulting cache_read drop as expected, not a break event.

### 8.8 `feature('EXPERIMENTAL_SKILL_SEARCH')`

- Off: `stripReinjectedAttachments` is identity.
- On: drops `skill_discovery` / `skill_listing` attachments before the
  summarizer.

### 8.9 `process.env.USER_TYPE === 'ant'` (paths in this spec)

- `apiMicrocompact.ts:90` — non-ANT short-circuits to thinking-only strategies;
  ANT may emit `clear_tool_uses_20250919` strategies based on
  `USE_API_CLEAR_TOOL_RESULTS` / `USE_API_CLEAR_TOOL_USES` env.
- `sessionMemoryCompact.ts:423-429, 625-627` — flag-check telemetry and
  debug logs only fire on ANT builds.
- `compact.ts:404` — `logPermissionContextForAnts` is a no-op on non-ANT.

### 8.10 `process.env.NODE_ENV === 'test'`

- Test environment skips one HISTORY_SNIP path (`messages.ts:2351`) — outside
  this spec's scope but flagged for cross-reference.

---

## 9. Error Handling & Edge Cases

- **Empty messages**: `compactConversation` throws `ERROR_MESSAGE_NOT_ENOUGH_MESSAGES`
  immediately; auto-compact path swallows this without notification (compact.ts:397-399, 753).
- **Partial-compact empty slice**: throws direction-specific message
  (compact.ts:802-808).
- **PTL on the compact request itself**: PTL retry loop drops the oldest
  20%-of-groups (or a token-gap-quantified count) and replays. After
  `MAX_PTL_RETRIES = 3`, throws `ERROR_MESSAGE_PROMPT_TOO_LONG`. Failure event
  carries `ptlAttempts` (compact.ts:469-491, 874-898).
- **No summary text in response**: throws
  `Failed to generate conversation summary - response did not contain valid text content`;
  emits `tengu_compact_failed` with `reason: 'no_summary'` (compact.ts:493-506,
  900-908). `tengu_partial_compact_failed` for partial path.
- **API error string in summary**: `startsWithApiErrorPrefix` matches; rethrows
  the summary string itself as the error message (compact.ts:507-515, 909-916).
- **Cache-share fork returns aborted/abort error**: detected via
  `assistantMsg.isApiErrorMessage`; falls through to streaming path with
  `tengu_compact_cache_sharing_fallback` reason `error` (compact.ts:1207-1247).
- **Streaming fallback retries**: `MAX_COMPACT_STREAMING_RETRIES = 2`
  controlled by GB `tengu_compact_streaming_retry` (default false → 1 attempt).
  Backoff via `getRetryDelay(attempt)`; abort throws `APIUserAbortError`
  (compact.ts:1257-1389).
- **No streaming response after retries**: throws
  `ERROR_MESSAGE_INCOMPLETE_RESPONSE`; `tengu_compact_failed` reason
  `no_streaming_response` (compact.ts:1379-1392).
- **Manual-only error notification**: `addErrorNotificationIfNeeded` is called
  on the manual path (compact.ts:1108-1123); auto-compact catch path silently
  increments the failure counter. User-abort and not-enough-messages errors
  never trigger a notification.
- **Auto-compact circuit breaker**: trips at 3 consecutive failures
  (autoCompact.ts:70, 261-265). `consecutiveFailures` is reset to 0 on every
  successful compact.
- **Compact-as-subagent recursion**: `shouldAutoCompact` returns false for
  `querySource in {'session_memory','compact'}` and (CONTEXT_COLLAPSE) for
  `'marble_origami'`.
- **DISABLE_COMPACT env**: `autoCompactIfNeeded` returns
  `{wasCompacted:false}` even past tracking checks (autoCompact.ts:253-255).
- **time-based MC with `slice(-0)` degenerate**: `keepRecent = max(1, config.keepRecent)`
  keeps at least one tool result (microCompact.ts:461-462).
- **Cached-MC stale state after time-based clearing**: `resetMicrocompactState()`
  is called inside the time-based path so cached MC doesn't try to delete
  already-cleared tools (microCompact.ts:517).
- **Partial-compact 'up_to' boundary stripping**: kept tail filters out old
  compact boundaries / summary user messages so the new summary wins the
  backward scan (compact.ts:790-799).
- **dropping group 0 leaves assistant-first sequence**: PTL truncate prepends
  a `PTL_RETRY_MARKER` synthetic user message; subsequent retries strip it
  back off before regrouping (compact.ts:251-256, 282-289).
- **`forkedAgent` cache-key parity**: must NOT set `maxOutputTokens` in Path
  A; doing so clamps `budget_tokens` and invalidates the cache
  (comment compact.ts:1182-1186).

---

## 10. Telemetry & Observability

### 10.1 logEvent calls

| Event | Triggered at | Key fields |
|---|---|---|
| `tengu_compact` | every successful `compactConversation` | preCompactTokenCount, postCompactTokenCount (compact-call total), truePostCompactTokenCount, autoCompactThreshold, willRetriggerNextTurn, isAutoCompact, querySource, queryChainId, queryDepth, isRecompactionInChain, turnsSincePreviousCompact, previousCompactTurnId, compaction{Input,Output,Total,CacheRead,CacheCreation}Tokens, promptCacheSharingEnabled, plus tokenStatsToStatsigMetrics(analyzeContext(messages)) |
| `tengu_partial_compact` | every successful partial compact | preCompactTokenCount, postCompactTokenCount, messagesKept, messagesSummarized, direction, hasUserFeedback, trigger='message_selector', compaction usage |
| `tengu_compact_failed` | full compact failure | reason ∈ {prompt_too_long, no_summary, api_error, no_streaming_response} + preCompactTokenCount + ptlAttempts/hasStartedStreaming/retryEnabled/attempts/promptCacheSharingEnabled as applicable |
| `tengu_partial_compact_failed` | partial compact failure | reason + direction + messagesSummarized + ptlAttempts |
| `tengu_compact_ptl_retry` | each PTL truncate-retry attempt | attempt, droppedMessages, remainingMessages, (path='partial' for partial path) |
| `tengu_compact_cache_sharing_success` | Path A (forked-agent) success | preCompactTokenCount, outputTokens, cacheReadInputTokens, cacheCreationInputTokens, cacheHitRate |
| `tengu_compact_cache_sharing_fallback` | Path A → Path B | reason ∈ {no_text_response, error}, preCompactTokenCount |
| `tengu_compact_streaming_retry` | Path B inter-attempt sleep | attempt, preCompactTokenCount, hasStartedStreaming |
| `tengu_auto_compact_succeeded` | caller side after `autoCompactIfNeeded` returns success | originalMessageCount, compactedMessageCount, pre/post/true tokens, compaction usage breakdown, queryChainId, queryDepth (query.ts:472-503) |
| `tengu_cached_microcompact` | cached MC commit | toolsDeleted, deletedToolIds, activeToolCount, triggerType='auto', threshold, keepRecent |
| `tengu_time_based_microcompact` | time-based MC commit | gapMinutes, gapThresholdMinutes, toolsCleared, toolsKept, keepRecent, tokensSaved |
| `tengu_sm_compact_no_session_memory` | SM-compact: no SM file | (empty) |
| `tengu_sm_compact_empty_template` | SM-compact: SM matches default template | (empty) |
| `tengu_sm_compact_summarized_id_not_found` | SM-compact: lastSummarizedMessageId not in messages | (empty) |
| `tengu_sm_compact_resumed_session` | SM-compact: no lastSummarizedMessageId, content present | (empty) |
| `tengu_sm_compact_threshold_exceeded` | SM-compact result still over threshold | postCompactTokenCount, autoCompactThreshold |
| `tengu_sm_compact_error` | SM-compact unhandled error | (empty) |
| `tengu_sm_compact_flag_check` | ANT-only flag-check log | tengu_session_memory, tengu_sm_compact, should_use |
| `tengu_post_compact_file_restore_success` | per-file Read on attachment generation | (analytics from `generateFileAttachment`) |
| `tengu_post_compact_file_restore_error` | per-file Read failure | as above |

### 10.2 Query-engine checkpoints

`query.ts` invokes `queryCheckpoint(...)` at `'query_snip_start' / '_end'`,
`'query_microcompact_start' / '_end'`, `'query_autocompact_start' / '_end'`
(query.ts:402, 410, 413, 426, 458, 470).

### 10.3 Debug logs

- `Compact failed: no summary text in response. Response: <json>` — error.
- `Compact cache sharing: no text in response, falling back. Response: <json>` — warn.
- `Compact streaming failed after N attempts. hasStartedStreaming=...` — error.
- `autocompact: tokens=... threshold=... effectiveWindow=... [snipFreed=...]` — info.
- `autocompact: circuit breaker tripped after N consecutive failures — skipping future attempts this session` — warn.
- `Cached MC deleting <N> tool(s): <ids>` — info.
- `[TIME-BASED MC] gap <N>min > <M>min, cleared <K> tool results (~<T> tokens), kept last <P>` — info.
- `Session memory compaction error: <msg>` — debug, ANT-only.

---

## 11. Reimplementation Checklist

A re-implementer must preserve, in order of importance:

1. **The `CompactionResult` ordering invariant** —
   `[boundaryMarker, summaryMessages, messagesToKeep, attachments, hookResults]`.
   Any reordering breaks downstream dedup, replay, and `--resume`.
2. **Compact boundary metadata** — `preCompactDiscoveredTools` (sorted) and
   `preservedSegment` (head/anchor/tail uuids) must be set when applicable.
   Anchor uuid rule for partial compact differs by direction.
3. **Pre-stream sequence**: snip → microcompact → context-collapse projection
   → autocompact, with `snipTokensFreed` plumbed into autocompact.
4. **Auto-compact threshold formula**: `min(maxOutputTokens, 20_000)` reserved
   for summary, `13_000` buffer below window, optional pct override clamped to
   threshold, optional window override.
5. **Circuit breaker**: 3 consecutive failures, reset on success.
6. **`shouldAutoCompact` recursion guards** — `querySource in
   {session_memory, compact, marble_origami}`, plus collapse and reactive-only
   suppression.
7. **PTL retry**: up to 3 attempts; truncate algorithm (token-gap-driven else
   20%-fallback, never empty); strip `PTL_RETRY_MARKER` before regrouping;
   prepend marker if leading message is assistant-typed.
8. **streamCompactSummary path order**: forked-agent first if GB on, else
   streaming with up to 2 attempts; activity keep-alive timer at 30 s; no
   `maxOutputTokens` on Path A.
9. **System prompt verbatim** (§6.1), no-tools preamble (§6.2), trailer
   (§6.3), three template bodies (§6.6, §6.7, §6.8), and customInstructions
   suffix `\n\nAdditional Instructions:\n…`.
10. **Compact CanUseTool** denies all tool calls during summary generation.
11. **Image / document stripping** before sending to summarizer (`[image]`,
    `[document]` markers, including in nested tool_result content).
12. **`stripReinjectedAttachments`** drops `skill_discovery` /
    `skill_listing` only when `EXPERIMENTAL_SKILL_SEARCH` is on.
13. **Post-compact file restoration**: top-5 by recency, capped at 5_000
    tokens/file and 50_000 total; skip plan and memory files; skip files
    already visible in `messagesToKeep` (via Read tool_use scan).
14. **Plan / plan-mode / skill / async-agent / deferred-tools / agent-listing
    / mcp-instructions delta attachments** — exact set, exact order
    (compact.ts:545-585, 940-975).
15. **Skill attachment**: most-recent-first, per-skill 5_000 tokens, total
    25_000 tokens, truncation marker `\n\n[... skill content truncated for
    compaction; use Read on the skill path if you need the full text]`.
16. **Hook fan-out**: PreCompact (input `{trigger, customInstructions}`) →
    SessionStart `'compact'` → PostCompact (input `{trigger, compactSummary}`),
    with `mergeHookInstructions` (user wins primary slot, hook appended).
17. **Cache-break suppression**: every successful compact path calls
    `notifyCompaction` (full compaction) or `notifyCacheDeletion` (cache-edit
    or content-clear) under `PROMPT_CACHE_BREAK_DETECTION`.
18. **`runPostCompactCleanup`**: subagent-aware (only main-thread compacts
    reset collapse and userContext caches); skill names intentionally NOT
    cleared.
19. **`reAppendSessionMetadata` after every full compact** so `--resume` can
    still read user-set title from the 16KB tail.
20. **Microcompact variants**:
    a. Time-based first; short-circuits cached MC.
    b. Cached MC requires main-thread source, model support, gate-on,
       defers boundary message until after API response, baseline-subtracts
       `cache_deleted_input_tokens`.
    c. Both paths suppress compact warning + reset module state.
21. **Reactive-compact 413 handoff**: collapse drains first when on; reactive
    compact on second consecutive 413; `hasAttemptedReactiveCompact` prevents
    spirals; failure surfaces lastMessage and skips stop hooks.
22. **API-microcompact**: `clear_thinking_20251015` (`'all'` or
    `{thinking_turns: 1}`) for thinking models; ANT-gated tool-clearing
    strategies via env variables, defaults `180_000` / `40_000` tokens.
23. **Compaction reminder gate**: `tengu_marble_fox` ON, autocompact enabled,
    1M-context model, ≥25% used.
24. **All telemetry events and field names** in §10.1 must remain wire-compatible.

---

## 12. Open Questions / Unknowns

- **`reactiveCompact.ts`** — full algorithm body absent from leak. Withhold
  predicates, retry budget, peel-from-tail strategy, and the "force-apply
  snip" interaction at QueryEngine.ts:1276 are inferred from caller usage
  and comments only. Re-implementer must derive these from observed
  behavior (or treat the stub as a TODO).
- **`cachedMicrocompact.ts`** — entire body missing. Documented exclusively
  from caller-side use and the `tengu_cached_microcompact` event. Open:
  what `triggerThreshold` units are (likely `tool_uses` count given the
  log field `activeToolCount`); how `getCachedMCConfig` integrates with
  GrowthBook; how `markToolsSentToAPI` interacts with retry.
- **`snipCompact.ts` and `snipProjection.ts`** — tool implementation and
  projection algorithm absent. Only the function signatures and the
  `SNIP_NUDGE_TEXT` consumer wiring are documented. The exact pacing of
  `shouldNudgeForSnips` (mentions a 10k token interval in a comment but the
  algorithm is opaque) is not recoverable from the leak.
- **`contextCollapse/index.ts`** — entire service body missing. Public
  surface (`isContextCollapseEnabled`, `applyCollapsesIfNeeded`,
  `recoverFromOverflow`, `resetContextCollapse`) is recovered. Internals
  (commit log structure, drain semantics, projection algorithm) are not.
- **Tool-side `SnipTool`** — `tools.ts:123-124` references the path; file
  itself missing (overview §2.5 records this). The token-replay-via-snip
  callback at QueryEngine.ts:1276 cannot be fully verified without the tool
  source.
- **`isModelSupportedForCacheEditing`** — referenced from `microCompact.ts`
  but list of supported models is in the missing `cachedMicrocompact.ts`.
- **Field `isSnipBoundaryMessage`** — `snipProjection` membership semantics
  unknown; presumably matches a snip-marker user/system message but the
  predicate is opaque.
- **`createMicrocompactBoundaryMessage` arguments** at query.ts:872-885 —
  the literal call uses `(trigger, 0, deletedTokens, deletedToolIds, [])`.
  Signature of `messages.ts:4557` was not read; arg #2 (`0`) and arg #5
  (`[]`) semantics inferred. Spec 04 should validate.
- **`tengu_cache_plum_violet`** — referenced as "always true" by comment in
  `microCompact.ts:288`; gate is implicit, not consulted from this module.
- **`tengu_compact_cache_prefix` 3P-default rationale** — comment claims
  `false` path costs 0.76% of fleet cache_creation; the kill-switch is
  retained but no tunable for cache-share off path is exposed in code; a
  re-implementer must duplicate the experiment-derived default.
- **`COMPACT_MAX_OUTPUT_TOKENS` (20_000) vs `MAX_OUTPUT_TOKENS_FOR_SUMMARY`
  (20_000)** — same value but separate constants in different files; they
  appear semantically distinct (compact-API output cap vs. context-window
  reservation) but are tied. Spec 03 / 06 should confirm one is not derived
  from the other.
- **`autoCompact.ts:113` warning vs. error threshold** —
  `WARNING_THRESHOLD_BUFFER_TOKENS` and `ERROR_THRESHOLD_BUFFER_TOKENS` are
  both `20_000`; both flags fire at the same usage. Likely a vestige
  awaiting separate tuning; documented as-is.
- **`tools.ts:123`-style `feature('HISTORY_SNIP')` + `commands.ts:83`
  `forceSnip`** — the command-side `forceSnip` invocation path is owned by
  spec 21 (command catalog); cross-reference here is informational.
