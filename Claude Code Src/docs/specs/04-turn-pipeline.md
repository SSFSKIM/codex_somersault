# 04 — Turn Pipeline Specification

> Owner: sub-B2 · Adjacent: 03, 05, 07, 08, 09, 22, 26, 29, 41
> Last updated: 2026-05-08

## 1. Purpose & Scope

The **turn pipeline** is the per-turn driver that converts a user message into a sequence of assistant message events, dispatches every `tool_use` block the model emits to the right tool, threads the resulting `tool_result` blocks back in, and recurses until the model stops calling tools (or one of 10 observed terminal reasons fires). It sits **between** the SDK conversation driver (`QueryEngine` — spec 03 §3) which iterates `query()` and the streaming LLM API loop (`queryModelWithStreaming` in `src/services/api/claude.ts:752`, owned by spec 03 / spec 22). The pipeline does **not** own the streaming/retry mechanics; it consumes them via `deps.callModel` (`src/query/deps.ts:2`).

This spec covers:

- the `query()` / `queryLoop()` generator in `src/query.ts` (1 729 lines): cross-iteration `State`, the recovery DAG (`Continue` transitions), `Terminal` reasons, and every `feature(...)` gate wired into the loop;
- the helper modules under `src/query/` (`config.ts`, `deps.ts`, `stopHooks.ts`, `tokenBudget.ts`);
- the dispatch service surface under `src/services/tools/` (`toolOrchestration.ts`, `toolExecution.ts`, `toolHooks.ts`, `StreamingToolExecutor.ts`): partition-and-dispatch, per-tool input validation → permission → `tool.call()` → `PostToolUse`/`PostToolUseFailure` hooks, MCP and concurrency rules, synthetic error generation;
- the message → tool-use → tool-result lifecycle, including `system-reminder` injection (location and templates), backfill of observable input, content-replacement budgeting, queued-command drain, and skill / memory prefetch consume;
- the **hook fan-out** orchestrated by the pipeline (`PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionDenied`, `Stop`, `SubagentStop`, `TaskCompleted`, `TeammateIdle`, plus the post-sampling, stop-failure, post-tool-failure family) and the strict ANT-only timing-summary side-channel.

Out of scope (cross-references only):

- streaming SSE wire format, retry/backoff, thinking, prompt-cache breakpoint placement → spec **03**;
- `ToolUseContext` shape, `Tool` interface, registry construction, `assembleToolPool` ordering → spec **08**;
- the permission decision tree (rule matchers, modes, classifiers, dialog UI) → spec **09**;
- system prompt assembly (`appendSystemContext`, `userContext` rendering) → spec **05**;
- compaction triggers, microcompact algorithms, snip / collapse internals → spec **07**;
- usage / cost aggregation → spec **06**;
- multi-agent spawn algorithm — spec **30** (the pipeline only recurses through `runForkedAgent`-style callers transparently);
- tool-result on-disk overflow file format → specs **11** and **41** (this spec covers the budget application call site only).

## 2. Source Map

### 2.1 Owned files

| Path | Lines | Read status | Notes |
|---|---|---|---|
| `src/query.ts` | 1 729 | fully-read | The generator. Cross-iteration `State`, recovery DAG, all `feature()` gates. |
| `src/query/config.ts` | 47 | fully-read | `buildQueryConfig` snapshot of env/StatSig gates per turn. |
| `src/query/deps.ts` | 41 | fully-read | `QueryDeps` injection seam (`callModel`, `microcompact`, `autocompact`, `uuid`). |
| `src/query/stopHooks.ts` | 473 | fully-read | `handleStopHooks` orchestrator: Stop, SubagentStop, TaskCompleted, TeammateIdle. |
| `src/query/tokenBudget.ts` | 94 | fully-read | `checkTokenBudget` continuation/diminishing decision. |
| `src/services/tools/toolOrchestration.ts` | 188 | fully-read | `runTools`: partition-then-dispatch (concurrent vs serial). |
| `src/services/tools/toolExecution.ts` | 1 745 | fully-read | `runToolUse` → `checkPermissionsAndCallTool`: validation → hooks → permission → call → PostToolUse. |
| `src/services/tools/toolHooks.ts` | 651 | fully-read | `runPreToolUseHooks`, `runPostToolUseHooks`, `runPostToolUseFailureHooks`, `resolveHookPermissionDecision`. |
| `src/services/tools/StreamingToolExecutor.ts` | 530 | fully-read | Stream-time concurrent executor with sibling abort-on-error. |
| `src/utils/messages.ts` | sampled | grep + ranges | `wrapInSystemReminder`, `CANCEL_MESSAGE`, `REJECT_MESSAGE`, classifier denial prefix. |
| `src/utils/toolResultStorage.ts` | sampled | grep + ranges | `applyToolResultBudget` call site. |
| `src/utils/hooks.ts` (~3 000 lines) | sampled | grep + ranges | Hook executor entry points referenced by query.ts / stopHooks.ts. Bulk owned by spec 09. |

### 2.2 Imports from (upstream)

Selected upstream surfaces (citations are at the import line in `src/query.ts`):

- `./Tool.js` — `findToolByName`, `ToolUseContext` (08) — `query.ts:28`.
- `./services/api/claude.ts` (via `./query/deps.ts`) — `queryModelWithStreaming` (owned by spec 03; defined at `src/services/api/claude.ts:752`) — `src/query/deps.ts:2`. The pipeline injects this through `QueryDeps.callModel` and is therefore decoupled from the API loop's transport / retry mechanics.
- `./services/compact/autoCompact.js` — `calculateTokenWarningState`, `isAutoCompactEnabled`, `AutoCompactTrackingState` (07) — `query.ts:8-12`.
- `./services/compact/compact.js` — `buildPostCompactMessages` (07) — `query.ts:13`.
- `./services/compact/reactiveCompact.js` (require, gated `feature('REACTIVE_COMPACT')`) — `query.ts:14-17`.
- `./services/contextCollapse/index.js` (require, gated `feature('CONTEXT_COLLAPSE')`) — `query.ts:18-20`.
- `./services/compact/snipCompact.js` (require, gated `feature('HISTORY_SNIP')`) — `query.ts:115-117`.
- `./utils/taskSummary.js` (require, gated `feature('BG_SESSIONS')`) — `query.ts:118-120`.
- `./services/skillSearch/prefetch.js` (require, gated `feature('EXPERIMENTAL_SKILL_SEARCH')`) — `query.ts:66-68`.
- `./jobs/classifier.js` (require, gated `feature('TEMPLATES')`) — `query.ts:69-71`, also `query/stopHooks.ts:45-47`.
- `./services/api/withRetry.js` — `FallbackTriggeredError` (22) — `query.ts:7`.
- `./services/tools/StreamingToolExecutor.js`, `./services/tools/toolOrchestration.js` — `query.ts:96, 98`.
- `./query/{config,deps,transitions,stopHooks,tokenBudget}.js` — siblings.
- `./bootstrap/state.js` — `getCurrentTurnTokenBudget`, `getTurnOutputTokens`, `incrementBudgetContinuationCount` (41) — `query.ts:106-110`.
- `./hooks/useCanUseTool.js` — `CanUseToolFn` (09) — `query.ts:6`.
- `./types/message.js` — `AssistantMessage`, `Message`, `StreamEvent`, `ToolUseSummaryMessage`, `TombstoneMessage` (08/04). **Unresolved import** — `src/types/message.ts` is not present in this checkout; see §2.5.

### 2.3 Imported by (downstream callers of `query()`)

The pipeline is invoked by every code path that drives a single user-input → assistant-completion turn. Found references (spot-grep):

- `src/cli/print.ts` — `-p` / non-interactive mode (**indirect**: `src/cli/print.ts:91` imports `ask` from `src/QueryEngine.js`; `query()` is invoked inside `QueryEngine.ask` at `src/QueryEngine.ts:675-686`, not in print directly).
- `src/replLauncher.tsx`, `src/screens/REPL.tsx` — interactive REPL turns.
- `src/utils/forkedAgent.ts` and callers — compact / session_memory / agent_summary forks (`querySource: 'compact' | 'session_memory' | 'agent_summary'`).
- `src/tools/AgentTool/...` — sub-agent spawn turns (spec 14/30); each spawned agent owns its own `query()` invocation with `agentId` set.
- `src/server/...` and `src/remote/...` — remote / SDK control plane.

### 2.4 Source-coverage inventory (this spec's owned set)

| File / dir | Status |
|---|---|
| `src/query.ts` | fully-read |
| `src/query/config.ts` | fully-read |
| `src/query/deps.ts` | fully-read |
| `src/query/stopHooks.ts` | fully-read |
| `src/query/tokenBudget.ts` | fully-read |
| `src/services/tools/toolOrchestration.ts` | fully-read |
| `src/services/tools/toolExecution.ts` | fully-read (read in two ranges; total 1 745 lines) |
| `src/services/tools/toolHooks.ts` | fully-read |
| `src/services/tools/StreamingToolExecutor.ts` | partially read (1-200) + grep — covers public surface; internal scheduler details cited where needed |
| `src/query/transitions.ts` | **MISSING from this checkout** — `src/query.ts:104` imports `Terminal`, `Continue` from `./query/transitions.js` but no source file exists (`find src/query -type f` lists only `config.ts`, `deps.ts`, `stopHooks.ts`, `tokenBudget.ts`). The `Terminal.reason` and `Continue.reason` enumerations in §3.1 are derived purely from `query.ts` `return { reason: ... }` and `state.transition = { reason: ... }` sites; the type definitions in `transitions.ts` may declare additional unused literals. See §2.5 missing-source ledger. |

### 2.5 Missing-source ledger references

**New in this spec:**

- `src/query/transitions.ts` — imported at `src/query.ts:104` for `Terminal`, `Continue` types. File does not exist in this checkout (`find src/query -type f` returns only `config.ts`, `deps.ts`, `stopHooks.ts`, `tokenBudget.ts`). Cross-ref: spec 00 §2.5 (global missing-source ledger). Impact: type-level only — runtime behaviour is fully observable from `query.ts` return sites; rebuilds MUST infer the union by enumerating those sites (see §3.1 tables).

Pre-existing absent paths consumed indirectly:

- `src/types/message.ts` / `src/types/tools.ts` — `query.ts:30-39` imports `AssistantMessage`, `StreamEvent`, etc. from `./types/message.js`. **Path is unresolved**: `src/types/` has no `message.ts`/`message.tsx` file, and there is no generated module under `src/types/generated/` that supplies these names. Treat as missing source until the generator/path is identified; do not assert it resolves through `src/types/generated/`.
- `src/services/contextCollapse/index.js`, `src/services/compact/reactiveCompact.js`, `src/services/compact/snipCompact.js`, `src/services/skillSearch/prefetch.js`, `src/jobs/classifier.js`, `src/utils/taskSummary.js`, `src/utils/computerUse/cleanup.js` — required at runtime under feature gates; their code is owned by spec 07/29/30/etc., and citations from this spec target only the call sites in `query.ts` / `query/stopHooks.ts`.

## 3. Public Interface

### 3.1 `query()` — the entry point

```ts
// src/query.ts:181-199
export type QueryParams = {
  messages: Message[]
  systemPrompt: SystemPrompt
  userContext: { [k: string]: string }
  systemContext: { [k: string]: string }
  canUseTool: CanUseToolFn
  toolUseContext: ToolUseContext
  fallbackModel?: string
  querySource: QuerySource
  maxOutputTokensOverride?: number
  maxTurns?: number
  skipCacheWrite?: boolean
  taskBudget?: { total: number }
  deps?: QueryDeps
}

// src/query.ts:219-239
export async function* query(
  params: QueryParams,
): AsyncGenerator<
  | StreamEvent
  | RequestStartEvent
  | Message
  | TombstoneMessage
  | ToolUseSummaryMessage,
  Terminal
>
```

The async generator yields:

- `StreamEvent` and `RequestStartEvent` from the underlying API stream;
- `Message` values produced or normalized by the pipeline (assistant, user, system, attachment, progress);
- `TombstoneMessage` for orphaned partial assistant messages on streaming fallback (`query.ts:712-723`);
- `ToolUseSummaryMessage` (deferred summary from previous turn — gated on env `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES` AND `!toolUseContext.agentId`, **not** on USER_TYPE/StatSig) (`src/query/config.ts:36-38`, `src/query.ts:1054-1060, 1411-1482, 1416-1420`).

It returns a `Terminal` value documenting why the loop ended. There are **10 observed terminal reasons** in `src/query.ts` (verified by enumerating every `return { reason: ... }` site):

| `Terminal.reason` | Site | Trigger |
|---|---|---|
| `'completed'` | `query.ts:1264, 1357` | Model produced no tool_use blocks; stop hooks (if any) didn't block; budget didn't continue. |
| `'aborted_streaming'` | `query.ts:1051` | Abort signal fired during streaming (not Ctrl+C "interrupt" reason). |
| `'aborted_tools'` | `query.ts:1515` | Abort signal fired during tool execution. |
| `'blocking_limit'` | `query.ts:646` | Pre-stream prompt-too-long hard-block fired. |
| `'image_error'` | `query.ts:977, 1175` | `ImageSizeError` / `ImageResizeError`, or media error after recovery exhausts. |
| `'model_error'` | `query.ts:996` | `queryModelWithStreaming` threw (carries `error`). |
| `'prompt_too_long'` | `query.ts:1175, 1182` | PTL after recovery exhausts (collapse drain + reactive compact tried). |
| `'stop_hook_prevented'` | `query.ts:1279` | Stop / SubagentStop / TaskCompleted / TeammateIdle hook returned `preventContinuation: true`. |
| `'hook_stopped'` | `query.ts:1520` | PostToolUse / PreToolUse hook injected a `hook_stopped_continuation` attachment. |
| `'max_turns'` | `query.ts:1711` | `maxTurns` exceeded. |

`Continue` is the dual: every `continue` site assigns `state.transition = { reason: <name>, ... }` so tests can assert recovery without inspecting messages (`query.ts:214-217`). Observed:

| `Continue.reason` | Site |
|---|---|
| `'collapse_drain_retry'` | `query.ts:1110` |
| `'reactive_compact_retry'` | `query.ts:1162` |
| `'max_output_tokens_escalate'` | `query.ts:1217` |
| `'max_output_tokens_recovery'` (with `attempt: n`) | `query.ts:1245-1248` |
| `'stop_hook_blocking'` | `query.ts:1302` |
| `'token_budget_continuation'` | `query.ts:1338` |
| `'next_turn'` | `query.ts:1725` |

### 3.2 `QueryDeps` — the injection seam

```ts
// src/query/deps.ts:21-31
export type QueryDeps = {
  callModel: typeof queryModelWithStreaming
  microcompact: typeof microcompactMessages
  autocompact: typeof autoCompactIfNeeded
  uuid: () => string
}
```

`productionDeps()` (`deps.ts:33-40`) returns the four real implementations. Tests inject fakes via `params.deps`.

### 3.3 `QueryConfig` — per-turn snapshot

```ts
// src/query/config.ts:15-27
export type QueryConfig = {
  sessionId: SessionId
  gates: {
    streamingToolExecution: boolean   // StatSig 'tengu_streaming_tool_execution2'
    emitToolUseSummaries: boolean     // env CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES
    isAnt: boolean                    // process.env.USER_TYPE === 'ant'
    fastModeEnabled: boolean          // !env CLAUDE_CODE_DISABLE_FAST_MODE
  }
}
```

Built once per `query()` entry by `buildQueryConfig()` (`config.ts:29-46`); `feature(...)` gates are intentionally excluded so DCE keeps them inline at their guarded blocks.

### 3.4 Service surface

Functions consumed by `query.ts` from the dispatch services:

```ts
// src/services/tools/toolOrchestration.ts:19-24
export async function* runTools(
  toolUseMessages: ToolUseBlock[],
  assistantMessages: AssistantMessage[],
  canUseTool: CanUseToolFn,
  toolUseContext: ToolUseContext,
): AsyncGenerator<MessageUpdate, void>

// src/services/tools/toolOrchestration.ts:14-17
export type MessageUpdate = {
  message?: Message
  newContext: ToolUseContext
}

// src/services/tools/toolExecution.ts:337-342
export async function* runToolUse(
  toolUse: ToolUseBlock,
  assistantMessage: AssistantMessage,
  canUseTool: CanUseToolFn,
  toolUseContext: ToolUseContext,
): AsyncGenerator<MessageUpdateLazy, void>
```

`MessageUpdateLazy` carries an optional **deferred** context modifier (`toolUseExecution.ts:264-270`):

```ts
export type MessageUpdateLazy<M extends Message = Message> = {
  message: M
  contextModifier?: {
    toolUseID: string
    modifyContext: (context: ToolUseContext) => ToolUseContext
  }
}
```

**Executor-variant semantics for context modifiers** (do not generalize):

- **`runTools`** (non-streaming, `src/services/tools/toolOrchestration.ts:30-63`): read-only batches collect `contextModifier` callbacks during the concurrent batch and apply them **after** the batch joins, so concurrent reads cannot observe each other's mutations to a serial-only field.
- **`StreamingToolExecutor`** (`src/services/tools/StreamingToolExecutor.ts:379-395`): collects `contextModifier` callbacks per-tool, **but only applies them when `!tool.isConcurrencySafe`** — concurrency-safe (read-only) tools have their `contextModifier` callbacks **silently dropped**. The source comment is explicit: `"NOTE: we currently don't support context modifiers for concurrent tools. None are actively being used, but if we want to use them in concurrent tools, we need to support that here."` A rebuild MUST preserve this asymmetry; treat it as observable behaviour, not a bug to "fix" under the streaming gate.

`StreamingToolExecutor` (`StreamingToolExecutor.ts:40-62`) is constructed when `config.gates.streamingToolExecution` is true and replaces `runTools`; see §5.

## 4. Data Model & State

### 4.1 The mutable per-loop `State`

```ts
// src/query.ts:204-217
type State = {
  messages: Message[]
  toolUseContext: ToolUseContext
  autoCompactTracking: AutoCompactTrackingState | undefined
  maxOutputTokensRecoveryCount: number
  hasAttemptedReactiveCompact: boolean
  maxOutputTokensOverride: number | undefined
  pendingToolUseSummary: Promise<ToolUseSummaryMessage | null> | undefined
  stopHookActive: boolean | undefined
  turnCount: number
  transition: Continue | undefined
}
```

State is destructured at the top of every iteration (`query.ts:308-321`) and replaced wholesale at every `continue` site so callers don't have to update each field individually.

Loop-local (not on `State`) by intent (`query.ts:280-295`):

- `budgetTracker: BudgetTracker | null` — gated on `feature('TOKEN_BUDGET')`, see `query/tokenBudget.ts:6-20`;
- `taskBudgetRemaining: number | undefined` — `task_budget` carryover across compaction boundaries.

### 4.2 `BudgetTracker`

```ts
// src/query/tokenBudget.ts:6-20
export type BudgetTracker = {
  continuationCount: number
  lastDeltaTokens: number
  lastGlobalTurnTokens: number
  startedAt: number
}
```

Constants (`tokenBudget.ts:3-4`): `COMPLETION_THRESHOLD = 0.9`, `DIMINISHING_THRESHOLD = 500`. Diminishing returns require `continuationCount >= 3` AND the last two deltas both `< 500` tokens (`tokenBudget.ts:59-62`).

### 4.3 `ContentReplacementState` integration

Per-thread tool-result budget state (`src/utils/toolResultStorage.ts:390-462`). Owned by spec 11/41 for storage; **the pipeline applies the budget at the top of each iteration** before microcompact (`query.ts:376-394`):

```ts
const persistReplacements =
  querySource.startsWith('agent:') ||
  querySource.startsWith('repl_main_thread')
messagesForQuery = await applyToolResultBudget(
  messagesForQuery,
  toolUseContext.contentReplacementState,
  persistReplacements
    ? records => void recordContentReplacement(records, toolUseContext.agentId).catch(logError)
    : undefined,
  new Set(toolUseContext.options.tools
    .filter(t => !Number.isFinite(t.maxResultSizeChars))
    .map(t => t.name)),
)
```

The "exempt" set is the names of tools whose `maxResultSizeChars` is non-finite (i.e., tool opted out of size capping). Persistence occurs only for `querySource` starting with `'agent:'` or `'repl_main_thread'` — ephemeral forks (`compact`, `session_memory`, `agent_summary`) never touch disk.

### 4.4 Per-call objects in the loop body

Created fresh each iteration (`query.ts:551-558`):

- `assistantMessages: AssistantMessage[]` — accumulator for the streaming response;
- `toolResults: (UserMessage | AttachmentMessage)[]` — accumulator of normalized tool results that flow into the next iteration's input;
- `toolUseBlocks: ToolUseBlock[]` — flat list of `tool_use` blocks across all assistant messages this iteration; the **sole loop-exit signal** since `stop_reason` is unreliable (per the comment at `query.ts:553-557`);
- `needsFollowUp: boolean` — true iff any tool_use was streamed.

### 4.5 The `using pendingMemoryPrefetch` resource

```ts
// src/query.ts:301-304
using pendingMemoryPrefetch = startRelevantMemoryPrefetch(
  state.messages,
  state.toolUseContext,
)
```

`using` disposes on every generator exit path (return, throw, return()). `pendingMemoryPrefetch` is consumed once (`query.ts:1599-1614`) when settled and not yet consumed; `consumedOnIteration` is initialized to `-1` and set to `turnCount - 1` on consume. Cumulative `readFileState` filters memories the model already touched.

### 4.6 Per-iteration prefetches

- **Skill-discovery prefetch** (`query.ts:331-335`, `query.ts:1620-1628`) — gated `feature('EXPERIMENTAL_SKILL_SEARCH')`. Runs while model streams; consumed after tools and emitted as attachments.
- **Tool-use summary** (`query.ts:1411-1482, 1416-1420`) — fires when `config.gates.emitToolUseSummaries` AND there were tool blocks AND not aborted AND `!toolUseContext.agentId`. The gate is environment-driven via `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES` (`src/query/config.ts:36-38`); there is no USER_TYPE or StatSig branch on this code path. The summary is generated by a deferred `generateToolUseSummary(...)` promise (calls `queryHaiku` per `src/services/toolUseSummary/toolUseSummaryGenerator.ts:69-81`) and awaited at the **next** iteration top — there is no source-level timeout or stream-duration SLA. The "~1s under Haiku during the next 5-30s stream" timing in earlier drafts was a heuristic, not a source-verifiable invariant.

### 4.7 `queryTracking`

Carried on `ToolUseContext.queryTracking` (`query.ts:347-363`). Initialized with `chainId = deps.uuid()`, `depth = 0` if absent; otherwise `chainId` preserved and `depth++`. Every `logEvent` in the pipeline carries `queryChainId` and `queryDepth`.

## 5. Algorithm / Control Flow

### 5.1 Top-level pseudocode

```
query(params):
  consumedCommandUuids = []
  terminal = yield* queryLoop(params, consumedCommandUuids)
  // Reached only on normal return — skipped on throw and on .return()
  for uuid in consumedCommandUuids: notifyCommandLifecycle(uuid, 'completed')
  return terminal
```

(`query.ts:219-239`).

### 5.2 `queryLoop` — single iteration outline

```
state = initial(params)
budgetTracker = feature('TOKEN_BUDGET') ? createBudgetTracker() : null
taskBudgetRemaining = undefined
config = buildQueryConfig()
using pendingMemoryPrefetch = startRelevantMemoryPrefetch(messages, ctx)

loop forever:
  destructure state into bare names
  pendingSkillPrefetch = feature('EXPERIMENTAL_SKILL_SEARCH') ?
    skillPrefetch.startSkillDiscoveryPrefetch(null, messages, ctx) : undefined

  yield { type: 'stream_request_start' }
  ctx.queryTracking = nextChain(ctx.queryTracking)

  messagesForQuery = [...getMessagesAfterCompactBoundary(messages)]

  // -- 5.3 budget / snip / microcompact / collapse / autocompact pre-stream phase
  messagesForQuery = await applyToolResultBudget(...)
  if feature('HISTORY_SNIP'): { messagesForQuery, snipTokensFreed, boundaryMessage } = snipCompactIfNeeded(...)
  microResult = await deps.microcompact(messagesForQuery, ctx, querySource)
  messagesForQuery = microResult.messages
  pendingCacheEdits = feature('CACHED_MICROCOMPACT') ? microResult.compactionInfo?.pendingCacheEdits : undefined
  if feature('CONTEXT_COLLAPSE') && contextCollapse:
      messagesForQuery = await contextCollapse.applyCollapsesIfNeeded(messagesForQuery, ctx, querySource)
  fullSystemPrompt = asSystemPrompt(appendSystemContext(systemPrompt, systemContext))
  { compactionResult, consecutiveFailures } = await deps.autocompact(...)
  if compactionResult: yield post-compact messages; messagesForQuery = postCompact; tracking reset
  else if consecutiveFailures: tracking.consecutiveFailures = ...

  // -- prompt-too-long preempt (skipped for compact/session_memory/RC/collapse/just-compacted)
  if pre-stream-blocking-limit-hit:
    yield API error; return { reason: 'blocking_limit' }

  // -- 5.4 streaming + per-message handling
  attemptWithFallback = true
  while attemptWithFallback:
    attemptWithFallback = false
    try:
      for await message of deps.callModel({...}):
        if streaming-fallback-occurred: yield tombstones; reset accumulators; rebuild executor
        backfill tool_use inputs into a *clone* (never mutate original); pick clone iff fields ADDED
        withhold = is-PTL || is-media-error || is-max-output-tokens
        if !withhold: yield clonedOrOriginal
        if message.assistant: push; collect tool_use blocks; addTool to streaming executor
        if streamingExecutor: yield any completed results inline
      yield deferred microcompact boundary if pendingCacheEdits
    catch FallbackTriggeredError:
      switch to fallbackModel; reset; if ant: stripSignatureBlocks; yield 'Switched to <model>...' (warning); continue
  catch top-level:
    yield missing tool_result blocks; yield API error; return { reason: 'model_error' | 'image_error' }

  // -- post-stream
  void executePostSamplingHooks(... )      // ALWAYS fire-and-forget (no agentId restriction here)
  if aborted: yield remaining executor results / interruption; return { reason: 'aborted_streaming' }
  if pendingToolUseSummary: yield (await pendingToolUseSummary)

  if !needsFollowUp:
    // -- 5.5 recovery DAG
    if isWithheld413 and !already-drained: try collapse.recoverFromOverflow → continue 'collapse_drain_retry'
    if (isWithheld413 || isWithheldMedia) and reactiveCompact:
      compacted = await reactiveCompact.tryReactiveCompact(...)
      if compacted: yield post-compact; continue 'reactive_compact_retry'
      else: yield withheldError; void executeStopFailureHooks(...); return prompt_too_long | image_error
    if isWithheldMaxOutputTokens:
      if cap-experiment && override===undefined && !env: continue 'max_output_tokens_escalate' (64k)
      if recoveryCount < 3: inject recovery user message; continue 'max_output_tokens_recovery'
      else yield withheldError
    if lastMessage.isApiErrorMessage: void executeStopFailureHooks; return 'completed'

    // -- 5.6 stop-hook fan-out (yields to handleStopHooks generator)
    stopHookResult = yield* handleStopHooks(...)
    if preventContinuation: return 'stop_hook_prevented'
    if blockingErrors: continue 'stop_hook_blocking'

    // -- 5.7 token budget
    if feature('TOKEN_BUDGET'):
      decision = checkTokenBudget(...)
      if continue: inject nudge; continue 'token_budget_continuation'
      if completionEvent: logEvent('tengu_token_budget_completed')
    return { reason: 'completed' }

  // -- 5.8 tool dispatch (needsFollowUp)
  toolUpdates = streamingExecutor ? executor.getRemainingResults() : runTools(toolUseBlocks, asstMsgs, canUseTool, ctx)
  for await update of toolUpdates:
    if update.message: yield; toolResults.push(...normalizeMessagesForAPI([msg], tools).filter(user))
    if update.message.attachment.type === 'hook_stopped_continuation': shouldPreventContinuation = true
    if update.newContext: updatedToolUseContext = { ...newContext, queryTracking }
  if config.gates.emitToolUseSummaries and !ctx.agentId and tools-ran: nextPendingToolUseSummary = generateToolUseSummary(...)

  if aborted: yield interruption (unless reason === 'interrupt'); maxTurns check; return 'aborted_tools'
  if shouldPreventContinuation: return 'hook_stopped'
  if tracking.compacted: tracking.turnCounter++; logEvent('tengu_post_autocompact_turn')

  // -- 5.9 attachment / queue drain phase (BEFORE recursion)
  drainQueueByPriority(sleepRan ? 'later' : 'next', main-or-subagent-scoped)
  for await attachment of getAttachmentMessages(...): yield; toolResults.push(att)
  consume pendingMemoryPrefetch if settled
  consume pendingSkillPrefetch if any
  notifyCommandLifecycle('started') for prompt/task-notification commands; remove from queue

  if updatedToolUseContext.options.refreshTools: recompute tools (MCP late-arrivals)
  nextTurnCount = turnCount + 1
  if feature('BG_SESSIONS') and !agentId and shouldGenerateTaskSummary(): maybeGenerateTaskSummary(...)
  if maxTurns and nextTurnCount > maxTurns: yield max_turns_reached; return 'max_turns'

  state = next-iteration-state; transition = { reason: 'next_turn' }
  // continue (loop)
```

(All citations: `query.ts:241-1729`.)

### 5.3 Pre-stream phase ordering invariants

The relative order of pre-stream operations is load-bearing and documented inline:

1. **`applyToolResultBudget` runs BEFORE microcompact** (`query.ts:370-394`). Cached MC operates by `tool_use_id` and never inspects content, so content replacement is invisible to it and they compose cleanly.
2. **Snip runs BEFORE microcompact and BEFORE autocompact** (`query.ts:401-410`). `snipTokensFreed` is plumbed through to `autocompact` so the threshold check reflects what snip removed; `tokenCountWithEstimation` cannot see it independently.
3. **Microcompact runs BEFORE collapse** (`query.ts:413-426`).
4. **Collapse runs BEFORE autocompact** (`query.ts:428-447`). Inline comment: "if collapse gets us under the autocompact threshold, autocompact is a no-op and we keep granular context instead of a single summary."
5. **Autocompact** consumes `tracking` and `snipTokensFreed`; on success `tracking = { compacted: true, turnId: deps.uuid(), turnCounter: 0, consecutiveFailures: 0 }` (`query.ts:521-526`).
6. **Per-message-budget pre-stream blocking** (`query.ts:592-648`) is **skipped** when:
   - `compactionResult` already fired this iteration (stale token count);
   - `querySource === 'compact'` or `'session_memory'` (forked agents reducing tokens);
   - `reactiveCompact?.isReactiveCompactEnabled() && isAutoCompactEnabled()` (let RC handle real 413);
   - `collapseOwnsIt` (collapse's `recoverFromOverflow` drains on real 413).

### 5.4 Per-message handling (streaming)

For each message yielded by `deps.callModel`:

1. If `streamingFallbackOccurred`: yield tombstones for every accumulated assistant message, reset all accumulators, recreate `StreamingToolExecutor` (`query.ts:712-741`).
2. **Backfill clone** (`query.ts:747-787`): for each `tool_use` block whose tool has `backfillObservableInput`, call it on a copy of `block.input`. If new fields were added (not just overwrites), build a cloned message for yielding while leaving the original `message` untouched (mutation would invalidate prompt caching by changing serialized bytes, and overwrites would break VCR fixture hashes).
3. **Withhold gate** (`query.ts:798-822`): a message is withheld from yielding if any of:
   - `feature('CONTEXT_COLLAPSE')` and `contextCollapse?.isWithheldPromptTooLong(message, isPromptTooLongMessage, querySource)`;
   - `reactiveCompact?.isWithheldPromptTooLong(message)`;
   - `mediaRecoveryEnabled && reactiveCompact?.isWithheldMediaSizeError(message)`;
   - `isWithheldMaxOutputTokens(message)` — local helper (`query.ts:175-179`): assistant message with `apiError === 'max_output_tokens'`.
   Withheld messages are still pushed to `assistantMessages` so the recovery checks find them.
4. If `assistant`, push to `assistantMessages`, extract `tool_use` blocks, and (if streaming executor on AND not aborted) `streamingToolExecutor.addTool(toolBlock, message)`.
5. If streaming executor on, drain any completed results inline (`query.ts:847-862`) — yield message and append user-typed normalized blocks to `toolResults`.

### 5.5 Recovery DAG (single-shot, ordered)

When `!needsFollowUp` and the last assistant message is an API error, the order is:

```
isWithheld413
  └─ feature('CONTEXT_COLLAPSE') && state.transition?.reason !== 'collapse_drain_retry'
       └─ contextCollapse.recoverFromOverflow(...)
            ├── drained.committed > 0  →  continue 'collapse_drain_retry'
            └── otherwise              →  fall through

(isWithheld413 || isWithheldMedia) && reactiveCompact
  └─ reactiveCompact.tryReactiveCompact({hasAttempted, querySource, aborted, messages, cacheSafeParams})
       ├── compacted truthy → continue 'reactive_compact_retry' (sets hasAttemptedReactiveCompact=true)
       └── else → yield withheldError; void executeStopFailureHooks(...); return prompt_too_long|image_error
                  (do NOT fall through to stop hooks — death-spiral protection)

isWithheldMaxOutputTokens
  └─ tengu_otk_slot_v1 (StatSig, default false) && override===undefined && !env CLAUDE_CODE_MAX_OUTPUT_TOKENS
       → continue 'max_output_tokens_escalate' (sets override = ESCALATED_MAX_TOKENS, currently 64k via utils/context.ts)
  └─ recoveryCount < 3 → inject recovery user message; continue 'max_output_tokens_recovery'
  └─ else → yield withheldError, fall through

lastMessage.isApiErrorMessage → void executeStopFailureHooks; return 'completed'
otherwise                     → handleStopHooks(...)
```

(`query.ts:1062-1306`.)

`hasAttemptedReactiveCompact` is **deliberately preserved** across `'stop_hook_blocking'` continues (`query.ts:1297`); resetting it caused an infinite compact-loop bug.

### 5.6 Stop-hook fan-out (`handleStopHooks`)

`src/query/stopHooks.ts:65-473` orchestrates the full end-of-turn fan-out:

```
hookStartTime = Date.now()
stopHookContext = REPLHookContext{ messages, systemPrompt, userContext, systemContext, toolUseContext, querySource }
if querySource ∈ {'repl_main_thread','sdk'}: saveCacheSafeParams(createCacheSafeParams(stopHookContext))

// TEMPLATES dispatched job classifier (60s race timeout)
if feature('TEMPLATES') && env CLAUDE_JOB_DIR && querySource startsWith 'repl_main_thread' && !ctx.agentId:
  await Promise.race([classifyAndWriteState(...), setTimeout(60_000).unref()])

if !isBareMode():
  if !env-defined-falsy CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION: void executePromptSuggestion(...)
  if feature('EXTRACT_MEMORIES') && !ctx.agentId && isExtractModeActive(): void executeExtractMemories(...)
  if !ctx.agentId: void executeAutoDream(...)

if feature('CHICAGO_MCP') && !ctx.agentId:
  try await cleanupComputerUseAfterTurn(ctx)  // silent on failure

generator = executeStopHooks(permissionMode, signal, undefined, stopHookActive ?? false, ctx.agentId, ctx, [...messagesForQuery, ...assistantMessages], ctx.agentType)

for await result of generator:
  yield progress / blocking-error / hook_stopped_continuation messages
  collect hookErrors[], hookInfos[StopHookInfo]
  if abort signal: yield interruption; return preventContinuation:true

if hookCount > 0:
  yield createStopHookSummaryMessage(... 'suggestion', stopHookToolUseID)
  if hookErrors.length: ctx.addNotification?.({ key:'stop-hook-error', text:`Stop hook error occurred · ${ctrl+o} to see`, priority:'immediate' })

if preventContinuation: return preventContinuation:true
if blockingErrors.length > 0: return { blockingErrors, preventContinuation:false }

// teammate post-Stop chain
if isTeammate():
  for task in inProgressTasks owned by this teammate:
    executeTaskCompletedHooks(task.id, task.subject, task.description, teammateName, teamName, mode, signal, undefined, ctx)
  executeTeammateIdleHooks(teammateName, teamName, mode, signal)
  // each may set preventContinuation, contribute blockingErrors

return { blockingErrors:[], preventContinuation:false }

catch error:
  logEvent('tengu_stop_hook_error', { duration })
  yield createSystemMessage(`Stop hook failed: ${errorMessage(error)}`, 'warning')
  return { blockingErrors:[], preventContinuation:false }
```

The hook event names dispatched here are `'Stop'` (or `'SubagentStop'` when `agentId` is set per `utils/hooks.ts:3653`), `'TaskCompleted'`, `'TeammateIdle'`, plus the side-effect calls into `executePromptSuggestion`, `executeExtractMemories`, `executeAutoDream`, and the conditional `cleanupComputerUseAfterTurn`.

### 5.7 Tool dispatch (`runTools` + `runToolUse`)

`runTools` (`toolOrchestration.ts:19-82`) partitions the streamed `tool_use` blocks into batches:

```
partitionToolCalls:
  for each toolUse:
    tool = findToolByName(ctx.options.tools, toolUse.name)
    parsed = tool?.inputSchema.safeParse(toolUse.input)
    isConcurrencySafe = parsed.success && try { tool.isConcurrencySafe(parsed.data) } catch { false }
    extend last batch if (isConcurrencySafe && lastBatch.isConcurrencySafe), else open new batch
```

Each batch:

- **Concurrency-safe batch** (`toolOrchestration.ts:30-63`): run all tools concurrently via `runToolUse` interleaved by `all(..., maxConcurrency)`. `maxConcurrency` = `parseInt(env.CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY, 10) || 10` (`toolOrchestration.ts:8-12`). Context modifiers are queued by `toolUseID` and applied **after** the batch ends; only then is `newContext` yielded.
- **Non-concurrency-safe batch** (`toolOrchestration.ts:64-82`): single tool, `runToolsSerially` runs them one-by-one and applies modifiers as they arrive (`toolOrchestration.ts:118-150`).

Both call `markToolUseAsComplete(ctx, toolUseID)` (`toolOrchestration.ts:179-188`) which mutates `ctx.setInProgressToolUseIDs` to remove the id.

`runToolUse` (`toolExecution.ts:337-490`) dispatches one tool:

1. Resolve tool by name in `ctx.options.tools`; fall back to `getAllBaseTools()` only if found via `tool.aliases?.includes(toolName)` (legacy/alias path).
2. Compute `mcpServerType` and `mcpServerBaseUrl` for telemetry.
3. **No tool found** → emit `tengu_tool_use_error`, yield a synthetic `tool_result` with `is_error: true` and content `<tool_use_error>Error: No such tool available: ${toolName}</tool_use_error>` and return.
4. **Aborted before start** → emit `tengu_tool_use_cancelled`, yield a tool_result `createToolResultStopMessage` with content set to `withMemoryCorrectionHint(CANCEL_MESSAGE)`; `toolUseResult: CANCEL_MESSAGE` (verbatim string in §6).
5. Otherwise stream from `streamedCheckPermissionsAndCallTool` (`toolExecution.ts:492-570`) which wraps `checkPermissionsAndCallTool` in a `Stream<MessageUpdateLazy>` while emitting `tengu_tool_use_progress` per progress update.
6. On thrown error → yield a synthetic tool_result `<tool_use_error>Error calling tool (${tool.name}): ${errorMessage}</tool_use_error>`.

### 5.8 `checkPermissionsAndCallTool` — the inner critical path

(`toolExecution.ts:599-1745`.) Steps:

1. **Zod parse** `input` against `tool.inputSchema`. On failure: format Zod error → `<tool_use_error>InputValidationError: ${errorContent}</tool_use_error>`. If the tool is a deferred ToolSearch tool and not in the discovered set, append the schema-not-sent hint (`buildSchemaNotSentHint`, `toolExecution.ts:578-597`). Emit `tengu_tool_use_error` with `error: 'InputValidationError'` and the truncated detail.
2. **`tool.validateInput`** — if returns `{result:false, message, errorCode}`: yield `<tool_use_error>${message}</tool_use_error>`, log `tengu_tool_use_error`.
3. **Speculative bash classifier** — if `tool.name === BASH_TOOL_NAME`, kick off `startSpeculativeClassifierCheck(command, permissionContext, signal, isNonInteractive)` so it overlaps PreToolUse hooks and the dialog.
4. **Strip `_simulatedSedEdit`** defense-in-depth from Bash input (`toolExecution.ts:756-773`).
5. **Backfill clone** (`toolExecution.ts:774-793`): produce `processedInput = backfilledClone` for hooks/permission, while preserving `callInput` (the model's raw input) for `tool.call()` so the result string and VCR hashes embed the model's path verbatim.
6. **PreToolUse hooks** (`runPreToolUseHooks`, `toolHooks.ts:435-650`):
   - emit `'PreToolUse'` events with hook progress, `additionalContext`, `permissionBehavior`, `updatedInput`, `preventContinuation`, `stopReason`;
   - on `blockingError`, build a deny `PermissionResult` whose `decisionReason = { type:'hook', hookName: 'PreToolUse:${tool.name}', reason: denialMessage }`;
   - on abort during hook execution: emit `tengu_pre_tool_hooks_cancelled`, yield `hook_cancelled` attachment, then `{ type:'stop' }`.
   - The query loop captures `hookPermissionResult`, may overwrite `processedInput` from `hookUpdatedInput`, and tracks `shouldPreventContinuation`/`stopReason`.
   - Wall-clock duration recorded under `pre_tool_hook_duration_ms`. **ANT-only**: if total wall-clock duration > `HOOK_TIMING_DISPLAY_THRESHOLD_MS` (= 500), append a `createStopHookSummaryMessage` for `'PreToolUse'` (`toolExecution.ts:874-891`). `SLOW_PHASE_LOG_THRESHOLD_MS` = 2 000 triggers a `logForDebugging` warning (`toolExecution.ts:135-137, 865-870`).
7. **Resolve hook → permission** (`resolveHookPermissionDecision`, `toolHooks.ts:332-433`):
   - hook `'allow'` with no `updatedInput` and an interactive tool: re-route through `canUseTool` if `requiresInteraction && !interactionSatisfied || requireCanUseTool`;
   - otherwise still apply `checkRuleBasedPermissions(tool, hookInput, ctx)` — hook allow does NOT bypass deny/ask rules. If `ruleCheck === null` → use the hook decision; if `'deny'` → use the deny rule; if `'ask'` → run `canUseTool`;
   - hook `'deny'` → use it directly;
   - hook `'ask'` or no decision → call `canUseTool(tool, askInput, ctx, asstMsg, toolUseID, forceDecision)` (forceDecision present iff hook said `'ask'`).
8. Emit `tool_decision` OTel event (`toolExecution.ts:947-977`) with the source mapped via `decisionReasonToOTelSource` (`toolExecution.ts:204-250`). Increment code-edit counter for headless mode.
9. **PermissionRequest hook attribution** (`toolExecution.ts:980-993`): if the decision came from `'PermissionRequest'` hook, emit a `hook_permission_decision` attachment.
10. **Deny path** (`toolExecution.ts:995-1104`):
    - emit `tengu_tool_use_can_use_tool_rejected`;
    - build top-level content with the deny `errorMessage` (or `Execution stopped by PreToolUse hook${stopReason ? `: ${stopReason}` : ''}` if there's no detailed message and a hook prevented continuation);
    - append any reject `contentBlocks` (e.g. images) at the **top level** with sequential `imagePasteIds`;
    - if `feature('TRANSCRIPT_CLASSIFIER')` AND `decisionReason.type === 'classifier'` AND `classifier === 'auto-mode'`: run `executePermissionDeniedHooks`. If any hook returns `{retry:true}`, push a `createUserMessage` (isMeta) with the literal text `'The PermissionDenied hook indicated this command is now approved. You may retry it if you would like.'`.
    - Return.
11. **Allow path** (`toolExecution.ts:1105+`): emit `tengu_tool_use_can_use_tool_allowed`. If `permissionDecision.updatedInput !== undefined`, replace `processedInput`. Reconcile `callInput` so the tool sees the model's original `file_path` if the hook/permission didn't change it (transcript / VCR stability comment, `toolExecution.ts:1188-1205`).
12. **Tool call**: `await tool.call(callInput, { ...ctx, toolUseId, userModified: permissionDecision.userModified ?? false }, canUseTool, assistantMessage, progressCallback)` (`toolExecution.ts:1206-1222`). `addToToolDuration(durationMs)`.
13. **Span/event emission**: `endToolExecutionSpan({success:true})`, `endToolSpan(toolResultStr)`, `tool_result` OTel event with parameters/decision context (gated by `OTEL_LOG_TOOL_DETAILS`).
14. **Map result** via `tool.mapToolResultToToolResultBlockParam(result.data, toolUseID)` once and reuse.
15. **`addToolResult`** builds the user-typed `tool_result` with optional `acceptFeedback` text and reject `contentBlocks` (with sequential `imagePasteIds`). For non-MCP tools this fires immediately; for MCP tools it fires AFTER PostToolUse hooks so they can mutate `updatedMCPToolOutput`.
16. **PostToolUse hooks** (`runPostToolUseHooks`, `toolHooks.ts:39-191`): emit `'PostToolUse'` events; for MCP tools, accept `updatedMCPToolOutput` and re-run `addToolResult`. ANT-only timing summary mirrors PreToolUse.
17. **`shouldPreventContinuation`**: both `PreToolUse` AND `PostToolUse` hooks can stop continuation, with **different attachment metadata**:
    - `PreToolUse` deny path emits `hook_stopped_continuation` with `hookName: 'PreToolUse:${tool.name}', hookEvent: 'PreToolUse'` (see `toolExecution.ts:995-1104`).
    - `PostToolUse` path (`src/services/tools/toolHooks.ts:117-129`) emits `hook_stopped_continuation` when `result.preventContinuation` is true, with `hookName: 'PostToolUse:${tool.name}', hookEvent: 'PostToolUse'` and `message: result.stopReason || 'Execution stopped by PostToolUse hook'`.
    - Either attachment causes `query.ts:1388-1393` to set `shouldPreventContinuation = true`, which `query.ts:1518-1520` then surfaces as `Terminal.reason = 'hook_stopped'`.
18. On **call throw** in the inner `checkPermissionsAndCallTool` path: catch, `endToolExecutionSpan({success:false, error})`. MCP-auth errors flip the client status to `'needs-auth'` (per `toolExecution.ts:1599+` and `mcp/client.ts`). Run `runPostToolUseFailureHooks` (event `'PostToolUseFailure'`). The `tool_result` content here is **raw** `formatError(error)` (`src/services/tools/toolExecution.ts:1691, 1715-1727`; `src/utils/toolErrors.ts:5-21`) — it is **not** wrapped in `<tool_use_error>...</tool_use_error>`. The `<tool_use_error>` wrapper is applied separately by the **outer `runToolUse` catch** (spec 08 / `toolExecution.ts` outer scope) for non-call-path failures (validation, permission denial yielding via attachments, etc.). A rebuild must preserve this two-level wrapping discipline: inner call-path returns plain formatted strings; the outer catch adds the XML envelope.

### 5.9 `StreamingToolExecutor`

Constructor (`StreamingToolExecutor.ts:53-62`): captures `(toolDefinitions, canUseTool, toolUseContext)` and creates a child abort controller via `createChildAbortController(ctx.abortController)`. The child fires when a Bash tool errors so sibling subprocesses die immediately; **aborting the child does NOT abort the parent** (turn keeps running).

Tool tracking states: `'queued' | 'executing' | 'completed' | 'yielded'` (`StreamingToolExecutor.ts:19`).

Concurrency rule (`canExecuteTool`, `StreamingToolExecutor.ts:129-135`): start a tool iff there are no executing tools, OR the new tool is concurrency-safe AND **all** executing tools are concurrency-safe.

Synthetic error messages (`StreamingToolExecutor.ts:153-205`): three reasons map to verbatim payloads — see §6.

Public surface used by `query.ts`:

- `addTool(block, assistantMessage)` — queue or start.
- `getCompletedResults()` — drain finished tools inline during streaming (`query.ts:851-862`).
- `getRemainingResults()` — async iterator drained after streaming ends (`query.ts:1019-1023`, `query.ts:1381`).
- `discard()` — flag for fallback / abort cleanup.

When `streamingToolExecutor` is on and a streaming-fallback occurs, the executor is `discard()`ed and re-created with the same args (`query.ts:733-741`). On `FallbackTriggeredError`, same (`query.ts:912-919`).

### 5.10 Queued-command drain

After tool execution and before recursion (`query.ts:1547-1643`):

- `sleepRan = toolUseBlocks.some(b => b.name === SLEEP_TOOL_NAME)` (`SLEEP_TOOL_NAME` imported from `tools/SleepTool/prompt.js`);
- `isMainThread = querySource.startsWith('repl_main_thread') || querySource === 'sdk'`;
- `getCommandsByMaxPriority(sleepRan ? 'later' : 'next')`; `isSlashCommand` filtered out (slash commands go through `processSlashCommand` post-turn);
- main thread drains `agentId === undefined`; subagents drain only `mode === 'task-notification' && agentId === currentAgentId` (never user prompts);
- iterate `getAttachmentMessages(null, updatedToolUseContext, null, queuedCommandsSnapshot, [...messagesForQuery, ...assistantMessages, ...toolResults], querySource)` — yield each, push to `toolResults`;
- consume `pendingMemoryPrefetch` if settled and not previously consumed; filter via `filterDuplicateMemoryAttachments(...)` and `ctx.readFileState`;
- consume `pendingSkillPrefetch` (gated `EXPERIMENTAL_SKILL_SEARCH`);
- `removeFromQueue(consumedCommands)` for `mode === 'prompt' || mode === 'task-notification'`; for each push `uuid` into `consumedCommandUuids` and call `notifyCommandLifecycle(uuid, 'started')` synchronously (`'completed'` is fired only on normal `query()` return, `query.ts:235-237`).

### 5.11 Tool refresh between turns

`query.ts:1659-1671`: if `updatedToolUseContext.options.refreshTools` exists and returns a different array, replace `tools` in the next iteration's context. This is how late-arriving MCP servers become available without a full restart.

### 5.12 Stop-failure hooks

Whenever an API error is the terminal message (PTL after recovery exhaustion, media error after recovery exhaustion, generic `lastMessage.isApiErrorMessage`) the pipeline fires `executeStopFailureHooks(lastMessage, toolUseContext)` (`utils/hooks.ts:3594-3635`) before returning. It is fire-and-forget (`void`) and gated internally by `hasHookForEvent('StopFailure', ...)`. This is **distinct** from `Stop` hooks — `Stop` would death-spiral by re-injecting tokens on every retry.

### 5.13 Token-budget completion path

`query.ts:1308-1357`. When the model produced no tool_use this turn:

```
decision = checkTokenBudget(tracker, ctx.agentId, getCurrentTurnTokenBudget(), getTurnOutputTokens())
if action==='continue':
  incrementBudgetContinuationCount()
  state = next-state with messages = [...messagesForQuery, ...assistantMessages, createUserMessage({content: nudgeMessage, isMeta:true})]
  transition = 'token_budget_continuation'
  continue
if completionEvent:
  if diminishingReturns: logForDebugging('Token budget early stop: diminishing returns at ${pct}%')
  logEvent('tengu_token_budget_completed', { ...completionEvent, queryChainId, queryDepth })
return { reason:'completed' }
```

`checkTokenBudget` short-circuits to `stop` when `agentId` is set, `budget === null`, or `budget <= 0` (`tokenBudget.ts:51-53`).

## 6. Verbatim Assets

### 6.1 System-reminder injection

The pipeline does not directly emit `<system-reminder>` text inline; it relies on `wrapInSystemReminder` (`src/utils/messages.ts:3097-3099`) and `wrapMessagesInSystemReminder` (`messages.ts:3101-3134`) used by the **attachment renderer** that runs from inside `getAttachmentMessages` (called in §5.10) and the team-context attachment producer.

Wrapper template (verbatim):

```
<system-reminder>
${content}
</system-reminder>
```

Round-trip recognition: `if (b.text.startsWith('<system-reminder>'))` (`messages.ts:1800, 1808, 1849, 2502`). The pipeline depends on this prefix being a reliable discriminator; do not introduce variants.

The **team-context** template (`messages.ts:3470-3495`, owned by spec 14/30 but materialized here as a verbatim asset because it is injected by the attachment fan-out) is reproduced under `messages.ts:3470-3495` — refer to that source for the exact text.

### 6.2 Hook event-name string literals (verbatim)

The **authoritative** runtime event-name set lives in `src/entrypoints/sdk/coreTypes.ts:25-53` as `HOOK_EVENTS` (used as the source of truth by spec 09). The full set, in declaration order:

```
'PreToolUse', 'PostToolUse', 'PostToolUseFailure',
'Notification', 'UserPromptSubmit',
'SessionStart', 'SessionEnd',
'Stop', 'StopFailure',
'SubagentStart', 'SubagentStop',
'PreCompact', 'PostCompact',
'PermissionRequest', 'PermissionDenied',
'Setup',
'TeammateIdle',
'TaskCreated', 'TaskCompleted',
'Elicitation', 'ElicitationResult',
'ConfigChange',
'WorktreeCreate', 'WorktreeRemove',
'InstructionsLoaded',
'CwdChanged', 'FileChanged'
```

`src/types/hooks.ts:70-163` is a hook-output Zod schema subset, not the event-name authority — earlier drafts cited it incorrectly. Only a **strict subset** of these events is invoked from the turn pipeline; the table below enumerates the call sites this spec owns. Events such as `PostCompact`, `TaskCreated`, `ConfigChange`, `WorktreeRemove`, and `InstructionsLoaded` exist in the runtime set but are owned by other specs (07, 14/30, 41, 23, 02 respectively) and are out of scope for spec 04.

Sites the **turn pipeline** invokes hooks (event-name verbatim → caller):

| Event | Caller (in this spec's owned source) |
|---|---|
| `'PreToolUse'` | `toolExecution.ts:800-807` (`runPreToolUseHooks` → `executePreToolHooks`) |
| `'PostToolUse'` | `toolExecution.ts:1483-1493` (`runPostToolUseHooks` → `executePostToolHooks`) |
| `'PostToolUseFailure'` | `toolExecution.ts:1599+` (`runPostToolUseFailureHooks` on call throw) |
| `'PermissionDenied'` | `toolExecution.ts:1075-1101` (gated `feature('TRANSCRIPT_CLASSIFIER')` + classifier `'auto-mode'`) |
| `'PermissionRequest'` | `toolExecution.ts:980-993` (attribution attachment) |
| `'Stop'` | `query/stopHooks.ts:180-189` via `executeStopHooks(...)`; event chosen `'SubagentStop'` if `agentId` per `utils/hooks.ts:3653` |
| `'SubagentStop'` | same call site as above when `ctx.agentId` is truthy |
| `'TaskCompleted'` | `query/stopHooks.ts:353-356` |
| `'TeammateIdle'` | `query/stopHooks.ts:403-408` |
| `'StopFailure'` | `query.ts:1174, 1181, 1263` (3 sites; `executeStopFailureHooks(lastMessage, ctx)`) |
| Post-sampling | `query.ts:999-1009` (`executePostSamplingHooks`) — ad-hoc registry (not a settings-configurable hook, see `utils/hooks/postSamplingHooks.ts`) |
| `'PreCompact'` | NOT invoked from this spec; spec 07 owns. |
| `'SessionStart'`, `'SessionEnd'`, `'UserPromptSubmit'`, `'Notification'`, `'Setup'`, `'SubagentStart'` | NOT invoked from `query.ts`. Owned by spec 01 (entrypoint) / spec 41 (session) / etc. |

### 6.3 Tool-result error / reject strings (verbatim)

`src/utils/messages.ts:207-219, 240, 246-251`:

```
INTERRUPT_MESSAGE                       = '[Request interrupted by user]'
INTERRUPT_MESSAGE_FOR_TOOL_USE          = '[Request interrupted by user for tool use]'
CANCEL_MESSAGE                          = "The user doesn't want to take this action right now. STOP what you are doing and wait for the user to tell you how to proceed."
REJECT_MESSAGE                          = "The user doesn't want to proceed with this tool use. The tool use was rejected (eg. if it was a file edit, the new_string was NOT written to the file). STOP what you are doing and wait for the user to tell you how to proceed."
REJECT_MESSAGE_WITH_REASON_PREFIX       = "The user doesn't want to proceed with this tool use. The tool use was rejected (eg. if it was a file edit, the new_string was NOT written to the file). To tell you how to proceed, the user said:\n"
SUBAGENT_REJECT_MESSAGE                 = 'Permission for this tool use was denied. The tool use was rejected (eg. if it was a file edit, the new_string was NOT written to the file). Try a different approach or report the limitation to complete your task.'
SUBAGENT_REJECT_MESSAGE_WITH_REASON_PREFIX = 'Permission for this tool use was denied. The tool use was rejected (eg. if it was a file edit, the new_string was NOT written to the file). The user said:\n'
PLAN_REJECTION_PREFIX                   = 'The agent proposed a plan that was rejected by the user. The user chose to stay in plan mode rather than proceed with implementation.\n\nRejected plan:\n'
NO_RESPONSE_REQUESTED                   = 'No response requested.'
SYNTHETIC_TOOL_RESULT_PLACEHOLDER       = '[Tool result missing due to internal error]'
AUTO_MODE_REJECTION_PREFIX              = 'Permission for this action has been denied. Reason: '
```

`DENIAL_WORKAROUND_GUIDANCE` (`messages.ts:226-232`) — verbatim multiline string at that range.

`AUTO_REJECT_MESSAGE(toolName)` and `DONT_ASK_REJECT_MESSAGE(toolName)` — formula at `messages.ts:234-239`.

`buildYoloRejectionMessage(reason)` is built from `AUTO_MODE_REJECTION_PREFIX`, the reason, and the `BASH_CLASSIFIER`-gated rule hint (`messages.ts:267-280`).

`isClassifierDenial(content)` is `content.startsWith(AUTO_MODE_REJECTION_PREFIX)` (`messages.ts:257-259`); the UI uses this to render a short summary instead of the full denial.

### 6.4 Tool-not-found / streaming-fallback / sibling-error messages

Verbatim payloads emitted from the pipeline:

```
// toolExecution.ts:401, 406  (also StreamingToolExecutor.ts:91, 96)
content:        '<tool_use_error>Error: No such tool available: ${toolName}</tool_use_error>'
toolUseResult:  'Error: No such tool available: ${toolName}'

// toolExecution.ts:670, 675
content:        '<tool_use_error>InputValidationError: ${errorContent}</tool_use_error>'
toolUseResult:  'InputValidationError: ${parsedInput.error.message}'

// toolExecution.ts:480, 485
detailedError:  'Error calling tool (${tool.name}): ${errorMessage}'
content:        '<tool_use_error>${detailedError}</tool_use_error>'

// StreamingToolExecutor.ts:179-186
content:        '<tool_use_error>Error: Streaming fallback - tool execution discarded</tool_use_error>'
toolUseResult:  'Streaming fallback - tool execution discarded'

// StreamingToolExecutor.ts:189-200  (sibling abort due to parallel error)
msg = erroredToolDescription
        ? `Cancelled: parallel tool call ${erroredToolDescription} errored`
        : 'Cancelled: parallel tool call errored'
content:        '<tool_use_error>${msg}</tool_use_error>'

// StreamingToolExecutor.ts:160-172  (user interrupt)
content:        withMemoryCorrectionHint(REJECT_MESSAGE)   // see REJECT_MESSAGE above
toolUseResult:  'User rejected tool use'
```

PermissionDenied retry message (verbatim, `toolExecution.ts:1093-1098`):

```
'The PermissionDenied hook indicated this command is now approved. You may retry it if you would like.'
```

Max-output-tokens recovery message (verbatim, `query.ts:1224-1229`):

```
`Output token limit hit. Resume directly — no apology, no recap of what you were doing. Pick up mid-thought if that is where the cut happened. Break remaining work into smaller pieces.`
```

Streaming-fallback system warning (verbatim, `query.ts:945-948`):

```
`Switched to ${renderModelName(innerError.fallbackModel)} due to high demand for ${renderModelName(innerError.originalModel)}`
```

Stop-hook notification (verbatim, `query/stopHooks.ts:317-321`):

```
text: `Stop hook error occurred · ${expandShortcut} to see`   // · is the · separator
key:  'stop-hook-error'
priority: 'immediate'
```

Stop-hook system-message error (verbatim, `query/stopHooks.ts:467-470`):

```
`Stop hook failed: ${errorMessage(error)}`   // 'warning' level
```

`Schema-not-sent` hint (verbatim, `toolExecution.ts:578-597`):

```
`\n\nThis tool's schema was not sent to the API — it was not in the discovered-tool set derived from message history. Without the schema in your prompt, typed parameters (arrays, numbers, booleans) get emitted as strings and the client-side parser rejects them. Load the tool first: call ${TOOL_SEARCH_TOOL_NAME} with query "select:${tool.name}", then retry this call.`
```

### 6.5 Constants table

| Constant | Value | Site |
|---|---|---|
| `MAX_OUTPUT_TOKENS_RECOVERY_LIMIT` | 3 | `query.ts:164` |
| `HOOK_TIMING_DISPLAY_THRESHOLD_MS` | 500 | `toolExecution.ts:134` |
| `SLOW_PHASE_LOG_THRESHOLD_MS` | 2 000 | `toolExecution.ts:137` |
| Default `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` | 10 | `toolOrchestration.ts:8-12` |
| `COMPLETION_THRESHOLD` | 0.9 | `tokenBudget.ts:3` |
| `DIMINISHING_THRESHOLD` | 500 | `tokenBudget.ts:4` |
| Diminishing-returns `continuationCount` floor | 3 | `tokenBudget.ts:60` |
| TEMPLATES classifier race timeout | 60 000 ms | `query/stopHooks.ts:128-131` |
| `SUBAGENT_REJECT_MESSAGE` exact text | (above) | `messages.ts:216-217` |
| Memory-correction wrapper (`withMemoryCorrectionHint`) | (defined in `messages.ts`) | applied at `toolExecution.ts:444`, `StreamingToolExecutor.ts:165` |

### 6.6 Permission-decision pseudocode (resolveHookPermissionDecision)

```
function resolveHookPermissionDecision(hookResult, tool, input, ctx, canUseTool, asstMsg, toolUseID):
  requiresInteraction = tool.requiresUserInteraction?.()
  requireCanUseTool   = ctx.requireCanUseTool

  if hookResult?.behavior === 'allow':
    hookInput = hookResult.updatedInput ?? input
    interactionSatisfied = requiresInteraction && hookResult.updatedInput !== undefined
    if (requiresInteraction && !interactionSatisfied) || requireCanUseTool:
      return { decision: await canUseTool(tool, hookInput, ctx, asstMsg, toolUseID), input: hookInput }
    ruleCheck = await checkRuleBasedPermissions(tool, hookInput, ctx)
    if ruleCheck === null:    return { decision: hookResult, input: hookInput }
    if ruleCheck.behavior==='deny': return { decision: ruleCheck, input: hookInput }
    // ask rule
    return { decision: await canUseTool(tool, hookInput, ctx, asstMsg, toolUseID), input: hookInput }

  if hookResult?.behavior === 'deny':
    return { decision: hookResult, input }

  forceDecision = hookResult?.behavior === 'ask' ? hookResult : undefined
  askInput      = (hookResult?.behavior === 'ask' && hookResult.updatedInput) ? hookResult.updatedInput : input
  return { decision: await canUseTool(tool, askInput, ctx, asstMsg, toolUseID, forceDecision), input: askInput }
```

(`toolHooks.ts:332-433`.)

### 6.7 OTel `source` mapping for `tool_decision`

```
ruleSource → otelSource
  'session'                             → behavior==='allow' ? 'user_temporary' : 'user_reject'
  'localSettings' | 'userSettings'      → behavior==='allow' ? 'user_permanent' : 'user_reject'
  default                               → 'config'

decisionReason.type:
  'permissionPromptTool' → toolResult.decisionClassification ∈ {'user_temporary','user_permanent','user_reject'} else fallback by behavior
  'rule'                 → ruleSourceToOTelSource(rule.source, behavior)
  'hook'                 → 'hook'
  'mode' | 'classifier' | 'subcommandResults' | 'asyncAgent' | 'sandboxOverride' | 'workingDir' | 'safetyCheck' | 'other' → 'config'
  undefined              → 'config'
```

(`toolExecution.ts:181-250`.)

## 7. Side Effects & I/O

| Effect | Site | Notes |
|---|---|---|
| `recordContentReplacement(records, agentId)` (disk write) | `query.ts:382-388` | Only when `querySource.startsWith('agent:')` or `'repl_main_thread'`. Errors swallowed via `.catch(logError)`. |
| `saveCacheSafeParams(...)` | `query/stopHooks.ts:96-98` | Only when `querySource ∈ {'repl_main_thread','sdk'}`. Used by `/btw` and `side_question` SDK control_request. |
| `notifyCommandLifecycle(uuid, 'started' \| 'completed')` | `query.ts:236, 1639` | `'completed'` only on normal `query()` return — skipped on throw and `.return()`. |
| `addNotification({key:'stop-hook-error', text, priority:'immediate'})` | `query/stopHooks.ts:317-322` | Only when `hookErrors.length > 0`. |
| `cleanupComputerUseAfterTurn(ctx)` (chicago MCP cleanup) | `query.ts:1031-1042, 1486-1498`; `query/stopHooks.ts:164-173` | `feature('CHICAGO_MCP') && !ctx.agentId`. Silent on failure. Auto-unhide + lock release. |
| `executePromptSuggestion`, `executeExtractMemories`, `executeAutoDream` | `query/stopHooks.ts:138-156` | Fire-and-forget, gated by `!isBareMode()` and respective env/feature checks. |
| `classifyAndWriteState(CLAUDE_JOB_DIR, ...)` | `query/stopHooks.ts:118-132` | Awaited with 60 s race; writes `state.json` for `claude list`. |
| `OpenTelemetry spans / events` | `toolExecution.ts:909-914, 1176-1395, 1593-1597` | `tool_decision`, `tool_result`, `tool.output`. Tool-content-events gated by `OTEL_LOG_TOOL_DETAILS`. |
| `cleanupComputerUseAfterTurn` on streaming abort and tool-aborted | (above) | |
| `recordContentReplacement` errors | logError only | does not throw out of the loop |

Environment variables consumed by this spec:

| Var | Site | Effect |
|---|---|---|
| `USER_TYPE === 'ant'` | `config.ts:39`; `toolExecution.ts:874-891, 1546-1563`; `query.ts:927` | StatSig-cached gate; ANT-only timing summaries; on `FallbackTriggeredError` strip thinking-signature blocks before retry. |
| `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES` | `config.ts:36-37` | Truthy enables tool-use summary generation per turn. |
| `CLAUDE_CODE_DISABLE_FAST_MODE` | `config.ts:43` | Truthy disables fast-mode flag in `gates`. |
| `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` | `toolOrchestration.ts:8-12` | Per-batch read-only concurrency cap (default 10). |
| `CLAUDE_CODE_MAX_OUTPUT_TOKENS` | `query.ts:1202` | When set, suppresses the 64 k auto-escalation path. |
| `CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION` | `query/stopHooks.ts:138` | If env-defined-falsy, suppresses prompt-suggestion fork. |
| `CLAUDE_JOB_DIR` | `query/stopHooks.ts:108-132` | When set + `feature('TEMPLATES')` + main thread + no agentId, runs the classifier. |

External binaries: none invoked directly from this spec — tool implementations (specs 10..19) own their subprocess work; the pipeline only spawns hook executables via `utils/hooks.ts` (spec 09).

## 8. Feature Flags & Variants

Per-flag delta surface tied to citations in this spec's owned source.

| Flag | Off (default in non-ant builds) | On | Sites in scope |
|---|---|---|---|
| `REACTIVE_COMPACT` | `reactiveCompact = null`; PTL recovery limited to collapse + truncation. | Module loaded; `tryReactiveCompact` invoked on withheld 413 / media error; withholds PTL & media error from stream until recovery resolves. | `query.ts:14-17, 627, 633, 811-819, 1119-1175` |
| `CONTEXT_COLLAPSE` | `contextCollapse = null`; no collapse projection or staged-drain recovery. | Pre-stream `applyCollapsesIfNeeded`; PTL withholding via `isWithheldPromptTooLong`; PTL recovery first tries `recoverFromOverflow` (single-shot, gated on prior `transition.reason`). | `query.ts:18-20, 440-447, 615-620, 800-810, 1086-1117, 1176-1183` |
| `HISTORY_SNIP` | snip module not loaded; `snipTokensFreed` always 0. | `snipCompactIfNeeded` runs before microcompact; tokens-freed fed to autocompact + blocking-limit calc. | `query.ts:115-117, 401-410, 638` |
| `BG_SESSIONS` | task-summary module not loaded. | Mid-turn `maybeGenerateTaskSummary` runs for non-agent main turns when `shouldGenerateTaskSummary()` returns true. | `query.ts:118-120, 1685-1702` |
| `EXPERIMENTAL_SKILL_SEARCH` | skill-discovery prefetch not started; no skill-discovery attachments. | Per-iteration `startSkillDiscoveryPrefetch`; consumed/yielded as attachments after tools. | `query.ts:66-68, 331-335, 1620-1628` |
| `TEMPLATES` | classifier module not loaded; `CLAUDE_JOB_DIR` ignored. | When `CLAUDE_JOB_DIR` set + main thread + no agentId, await classifier (60 s timeout) before stop-hook fan-out. | `query.ts:69-71`; `query/stopHooks.ts:42-49, 108-132` |
| `TOKEN_BUDGET` | `budgetTracker = null`; no continuation nudge; `'token_budget_continuation'` transition unused. | `createBudgetTracker()` per query; `checkTokenBudget` after each completed turn; injects nudge user-message and continues until `>=90 %` or diminishing returns. | `query.ts:280, 1308-1357`; `query/tokenBudget.ts` |
| `CACHED_MICROCOMPACT` | `pendingCacheEdits` always undefined; deferred boundary message never emitted. | After streaming, emit `createMicrocompactBoundaryMessage(trigger, 0, deletedTokens, deletedToolIds, [])` using the API-reported `cache_deleted_input_tokens` delta. | `query.ts:423-425, 866-892` |
| `CHICAGO_MCP` | no computer-use cleanup. | `cleanupComputerUseAfterTurn(ctx)` runs at three sites (turn end, streaming abort, mid-tool abort). Main thread only. | `query.ts:1031-1042, 1486-1498`; `query/stopHooks.ts:164-173` |
| `EXTRACT_MEMORIES` | no `executeExtractMemories` background fork. | If main thread + `isExtractModeActive()`, void-execute the extractor. | `query/stopHooks.ts:42-44, 142-156` |
| `TRANSCRIPT_CLASSIFIER` | no `PermissionDenied` hook fan-out on auto-mode classifier denials. | When the deny `decisionReason.type === 'classifier'` and `classifier === 'auto-mode'`, run hooks; on `{retry:true}` add the retry user-message. | `toolExecution.ts:1075-1101` |
| `BASH_CLASSIFIER` | bash classifier rule hint omitted from yolo rejection text. | Adds the `Bash(prompt: ...)` recommendation. | `messages.ts:270-274` |

ANT-only behaviors in pipeline:

- `process.env.USER_TYPE === 'ant'` (`config.ts:39`): exposed as `config.gates.isAnt`, used to enable `dumpPromptsFetch` per-query (`query.ts:586-590`).
- `query.ts:927-929`: on `FallbackTriggeredError`, `messagesForQuery = stripSignatureBlocks(messagesForQuery)` — thinking-signature blocks are model-bound and would 400 on the fallback.
- `toolExecution.ts:874-891` and `1546-1563`: ANT-only PreToolUse / PostToolUse hook timing summary surfaces (`HOOK_TIMING_DISPLAY_THRESHOLD_MS = 500`).
- StatSig gate `tengu_streaming_tool_execution2` (config.gates.streamingToolExecution) is **per-user** per the cached gate; functionally enables `StreamingToolExecutor`. Logged as `tengu_streaming_tool_execution_used` / `_not_used` per turn (`query.ts:1366-1378`).
- StatSig gate `tengu_otk_slot_v1` (`getFeatureValue_CACHED_MAY_BE_STALE`) — when true (and override unset and env `CLAUDE_CODE_MAX_OUTPUT_TOKENS` unset), the first `max_output_tokens` hit retries at 64 k once.

The **auto-mode classifier path at `query.ts:927`** referenced in the dispatch prompt is the `process.env.USER_TYPE === 'ant'` branch on `FallbackTriggeredError` recovery (signature stripping for ANT thinking-protected models). No "auto-mode classifier" code lives in `query.ts` itself; the auto-mode classifier ANT semantics live in `tools/BashTool/bashPermissions.ts` and `messages.ts`, and are invoked via `canUseTool` (spec 09).

## 9. Error Handling & Edge Cases

### 9.1 Failure modes

| Failure | Pipeline reaction | Site |
|---|---|---|
| `queryModelWithStreaming` throws | log; emit missing tool_results; yield API error message; return `model_error` | `query.ts:955-997` |
| `ImageSizeError` / `ImageResizeError` | yield API error with `error.message`; return `image_error` | `query.ts:970-978` |
| `FallbackTriggeredError` | switch model, reset accumulators, recreate executor, ANT signature-strip, log `tengu_model_fallback_triggered`, emit warning system message, retry inner loop | `query.ts:893-952` |
| Streaming fallback (mid-stream tombstoning) | yield `tombstone` per accumulated assistant message; reset accumulators; recreate executor | `query.ts:712-741` |
| Pre-stream blocking limit | yield API error with `PROMPT_TOO_LONG_ERROR_MESSAGE`; return `blocking_limit` | `query.ts:636-647` |
| Withheld 413 with no recovery available | yield withheld; void-execute StopFailure hooks; return `prompt_too_long` | `query.ts:1119-1183` |
| Withheld media-size-error with no recovery | yield withheld; void-execute StopFailure hooks; return `image_error` | same range |
| Max-output-tokens recovery exhausted | yield withheld error; fall through to stop-hook path | `query.ts:1188-1256` |
| `lastMessage.isApiErrorMessage` after stream | void-execute StopFailure hooks; return `completed` (no Stop hook fan-out — death-spiral protection) | `query.ts:1258-1265` |
| Stop-hook generator throws | log `tengu_stop_hook_error`; yield warning system message `Stop hook failed: ${errorMessage(error)}`; return `{blockingErrors:[], preventContinuation:false}` | `query/stopHooks.ts:455-471` |
| Tool not found | `<tool_use_error>Error: No such tool available: ${toolName}</tool_use_error>` | `toolExecution.ts:396-409`; same in `StreamingToolExecutor.ts:91-99` |
| Zod input validation failure | `<tool_use_error>InputValidationError: ${...}</tool_use_error>`; optional schema-not-sent hint appended | `toolExecution.ts:617-680` |
| `tool.validateInput` returns `{result:false}` | `<tool_use_error>${message}</tool_use_error>` | `toolExecution.ts:683-733` |
| `tool.call` throws | `endToolExecutionSpan({success:false, error})`; classify via `classifyToolError`; run `runPostToolUseFailureHooks`; yield `<tool_use_error>${error}</tool_use_error>` | `toolExecution.ts:1589+` |
| Sibling tool error (StreamingToolExecutor) | abort sibling controller; yield "Cancelled: parallel tool call …" synthetic results for queued/in-progress | `StreamingToolExecutor.ts:188-205` |
| Streaming-fallback discard | yield `<tool_use_error>Error: Streaming fallback - tool execution discarded</tool_use_error>` for each in-flight tool | `StreamingToolExecutor.ts:174-188` |
| Aborted before tool start | yield `createToolResultStopMessage` with `withMemoryCorrectionHint(CANCEL_MESSAGE)`; emit `tengu_tool_use_cancelled` | `toolExecution.ts:415-453` |
| PreToolUse hook throws | `tengu_pre_tool_hook_error`; yield `hook_error_during_execution` attachment; yield `{type:'stop'}` | `toolHooks.ts:604-643` |
| PostToolUse hook throws | `tengu_post_tool_hook_error`; yield `hook_error_during_execution` attachment | `toolHooks.ts:152-191` |
| PostToolUseFailure hook throws | `tengu_post_tool_failure_hook_error`; yield `hook_error_during_execution` attachment | `toolHooks.ts:281-318` |
| Abort mid-pre-hook | `tengu_pre_tool_hooks_cancelled`; yield `hook_cancelled` attachment + `{type:'stop'}` | `toolHooks.ts:582-603` |
| Abort mid-post-hook | `tengu_post_tool_hooks_cancelled`; yield `hook_cancelled` attachment | `toolHooks.ts:67-89` |
| Abort during tool stream | yield user-interruption (unless reason `'interrupt'`); emit `max_turns_reached` if `nextTurnCountOnAbort > maxTurns`; return `aborted_tools` | `query.ts:1485-1516` |
| Abort during streaming | yield remaining executor results / interruption (unless reason `'interrupt'`); cleanup CHICAGO_MCP; return `aborted_streaming` | `query.ts:1015-1052` |

### 9.2 Death-spiral protections

Documented in source comments — preserve verbatim:

1. **Stop hooks skipped on API errors** (`query.ts:1258-1265`): "Skip stop hooks when the last message is an API error … hooks evaluating it create a death spiral: error → hook blocking → retry → error → ...".
2. **`hasAttemptedReactiveCompact` preserved across stop-hook-blocking** (`query.ts:1297`): resetting caused infinite loop "compact → still too long → error → stop hook blocking → compact → … burning thousands of API calls".
3. **Reactive-compact path returns directly** (`query.ts:1168-1175`): "Do NOT fall through to stop hooks: the model never produced a valid response, so hooks have nothing meaningful to evaluate."
4. **Pre-stream blocking limit bypassed when RC/collapse own the path** (`query.ts:626-636`): preserves user-set `DISABLE_AUTO_COMPACT` while letting the API 413 hit so the recovery subsystem sees it.

### 9.3 Memory-correction hint

`withMemoryCorrectionHint` (`messages.ts`) wraps cancel/reject content; details owned by spec 11/41. The hint exists so a transcript `Read` retry sees the correct file content even if the tool was cancelled mid-write.

## 10. Telemetry & Observability

Analytics events emitted from this spec's code:

| Event | Site |
|---|---|
| `tengu_orphaned_messages_tombstoned` | `query.ts:719-723` |
| `tengu_query_error` | `query.ts:959-967` |
| `tengu_model_fallback_triggered` | `query.ts:932-941` |
| `tengu_auto_compact_succeeded` | `query.ts:478-502` |
| `tengu_query_before_attachments` | `query.ts:1539-1545` |
| `tengu_query_after_attachments` | `query.ts:1652-1657` |
| `tengu_post_autocompact_turn` | `query.ts:1525-1532` |
| `tengu_token_budget_completed` | `query.ts:1349-1354` |
| `tengu_max_tokens_escalate` | `query.ts:1204-1206` |
| `tengu_streaming_tool_execution_used` / `_not_used` | `query.ts:1366-1378` |
| `tengu_tool_use_cancelled`, `tengu_tool_use_error`, `tengu_tool_use_progress`, `tengu_tool_use_can_use_tool_rejected`, `tengu_tool_use_can_use_tool_allowed`, `tengu_tool_use_success`, `tengu_deferred_tool_schema_not_sent` | `toolExecution.ts:372, 416, 522, 1001, 1105, 1331, 625` |
| `tengu_pre_tool_hooks_cancelled`, `tengu_post_tool_hooks_cancelled`, `tengu_post_tool_failure_hooks_cancelled`, `tengu_pre_tool_hook_error`, `tengu_post_tool_hook_error`, `tengu_post_tool_failure_hook_error` | `toolHooks.ts:583, 72, 228, 607, 154, 283` |
| `tengu_stop_hook_error`, `tengu_pre_stop_hooks_cancelled` | `query/stopHooks.ts:458, 284` |

Profiler checkpoints (`query.ts` via `queryCheckpoint(...)`): `query_fn_entry`, `query_snip_start`/`_end`, `query_microcompact_start`/`_end`, `query_autocompact_start`/`_end`, `query_setup_start`/`_end`, `query_api_loop_start`, `query_api_streaming_start`/`_end`, `query_tool_execution_start`/`_end`, `query_recursive_call`. `headlessProfilerCheckpoint('query_started')` for top-level main-thread queries.

OTel spans/events from `toolExecution.ts`: `startToolSpan`, `startToolBlockedOnUserSpan`, `endToolBlockedOnUserSpan`, `startToolExecutionSpan`, `endToolExecutionSpan`, `endToolSpan`, `addToolContentEvent`, `logOTelEvent('tool_decision'|'tool_result')`. Beta-tracing-gated input dump.

`StatsStore` observations (`getStatsStore()?.observe`): `pre_tool_hook_duration_ms` recorded twice (early-return on stop, and end-of-hook). `addToToolDuration(durationMs)` aggregates across the turn.

## 11. Reimplementation Checklist

A behaviorally equivalent rebuild MUST preserve:

1. **Generator contract**: `query()` is an async generator yielding `StreamEvent | RequestStartEvent | Message | TombstoneMessage | ToolUseSummaryMessage` and **returning** a `Terminal` (per §3.1).
2. **The 10 observed terminal reasons** (string set: `completed`, `aborted_streaming`, `aborted_tools`, `blocking_limit`, `image_error`, `model_error`, `prompt_too_long`, `stop_hook_prevented`, `hook_stopped`, `max_turns`) and the 7 continue transitions (string set) — tests assert these directly. The actual `Terminal` / `Continue` union types live in `src/query/transitions.ts` which is **absent from this checkout**; if that file declares additional unused literals they must be cited at rebuild time.
3. **Pre-stream phase order**: budget → snip → microcompact → collapse → autocompact (§5.3 invariants 1-5). Reordering breaks composability and threshold accounting.
4. **`ContentReplacementState` persistence policy**: persist iff `querySource.startsWith('agent:')` or `'repl_main_thread'`; never for `compact|session_memory|agent_summary|...`.
5. **Backfill clone discipline**: never mutate `assistantMessages[*]` content blocks (prompt-cache byte stability); yield a clone iff backfill **adds** fields, never on overwrite (VCR fixture stability).
6. **Withhold gate composition**: ANY of {collapse PTL, RC PTL, RC media (gated by `mediaRecoveryEnabled`), max-output-tokens} → withhold; both subsystems independently sufficient.
7. **Recovery DAG order**: collapse drain → reactive compact → max-output-tokens (escalate, then 3-attempt recovery) → fall through to stop hooks (§5.5). Stop hooks **never** evaluate API-error terminal messages.
8. **`hasAttemptedReactiveCompact` preservation across stop-hook-blocking continues** (death-spiral fix).
9. **Stop / SubagentStop / TaskCompleted / TeammateIdle fan-out chain** with `stopHookActive: true` continue carryover after blocking errors.
10. **Tool dispatch invariants — split by executor:**
    - `runTools` (non-streaming): read-only batches run concurrently up to `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` with **deferred** context modifiers (applied after batch join); non-read-only batches run serially with **immediate** modifiers.
    - `StreamingToolExecutor` (streaming gate on): only applies collected `contextModifier`s for **non-concurrency-safe** tools (`StreamingToolExecutor.ts:379-395`); concurrency-safe (read-only) tools have their `contextModifier` callbacks **dropped on the floor**. This asymmetry is observable behaviour and must be preserved.
    - Both executors: `markToolUseAsComplete` mutates `setInProgressToolUseIDs` even on error paths.
11. **Permission decision composition**: hook 'allow' still applies `checkRuleBasedPermissions` (deny/ask rules win over hook allow). Interactive-tool guard re-routes to `canUseTool` unless hook supplied `updatedInput`.
12. **PermissionDenied hook only fires for auto-mode classifier denials and only when `feature('TRANSCRIPT_CLASSIFIER')` is on.**
13. **Stop-failure (NOT Stop) hooks fire on API-error terminals** (PTL after recovery, media error, generic API error).
14. **Verbatim error/cancel/reject strings** (§6.3 / §6.4) — these are user-visible AND model-trained signals.
15. **System-reminder wrapper format**: `<system-reminder>\n${content}\n</system-reminder>` and the `startsWith('<system-reminder>')` discriminator.
16. **Hook event-name string set** is the runtime `HOOK_EVENTS` array at `src/entrypoints/sdk/coreTypes.ts:25-53` (NOT the Zod subset in `src/types/hooks.ts:70-163`). The turn pipeline only invokes a subset (§6.2 table). Branching: `agentId` truthy → `'SubagentStop'` instead of `'Stop'`.
17. **Token budget short-circuit** (`agentId || budget===null || budget<=0`) and the diminishing-returns rule (3 continuations, last two deltas <500 tokens).
18. **`StreamingToolExecutor` semantics**: child abort controller (sibling-only), discard-on-fallback, three synthetic-error-message reasons.
19. **ANT-only**: signature stripping on FallbackTriggeredError, `dumpPromptsFetch` per-query, hook timing summaries above 500 ms wall-clock.
20. **MCP-tool special-case**: `addToolResult` is delayed until after PostToolUse so hooks can replace `updatedMCPToolOutput`.
21. **Tool-use summary**: gated by env `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES` (`src/query/config.ts:36-38`), only on main thread (`!toolUseContext.agentId`, checked at `src/query.ts:1416-1420`); generated post-tool batch as a deferred promise calling `queryHaiku`, awaited at the **next** iteration top. **Not** USER_TYPE / StatSig gated.
22. **Queue drain after tools, before recursion**: priority `'next'` if Sleep ran else `'later'`; main-thread vs subagent scoping; slash commands excluded; `notifyCommandLifecycle('started')` synchronous, `'completed'` only on natural `query()` return.
23. **Tool refresh between turns** for late-arriving MCP servers (`refreshTools()`).

## 12. Open Questions / Unknowns

1. **`Continue` / `Terminal` exhaustive type** — the union types are imported from `./query/transitions.js`, but `src/query/transitions.ts` is **absent from this checkout** (see §2.5 missing-source ledger). The seven `Continue.reason` literals and **ten** `Terminal.reason` literals enumerated in §3.1 are derived from `query.ts` `state.transition = { ... }` and `return { reason: ... }` sites only. Earlier drafts asserted "12 terminal reasons" — corrected to 10 here. If the missing `transitions.ts` declares additional unused reasons, this spec will need a §12 update once that source is recovered.
2. **`ESCALATED_MAX_TOKENS`** value — imported from `./utils/context.js` (`query.ts:89`). Section 5.5 documents the path; the literal is owned by spec 05 / 06. Estimated from inline comments to be `64k`; the actual constant should be cited in the rebuild.
3. **`PROMPT_TOO_LONG_ERROR_MESSAGE`** — imported from `./services/api/errors.js` (`query.ts:42`). Owned by spec 22.
4. **`isAutoCompactEnabled`** semantics — referenced at `query.ts:619, 633`; behavior owned by spec 07. The `collapseOwnsIt` pre-stream-skip predicate depends on its return value.
5. **`finalContextTokensFromLastResponse`** semantics — referenced at `query.ts:510, 1140`. Used for `task_budget.remaining` carryover; reads `iterations[-1]` per inline comment. Owned by spec 03/06 — referenced from this spec but not specified.
6. **`getMessagesAfterCompactBoundary`** (`utils/messages.ts`) — owned by spec 07/04. Pipeline depends on it returning the surviving tail; exact algorithm not enumerated here.
7. **Auto-mode classifier path at `query.ts:927`** — the dispatch prompt cited a specific line. At line 927 of `query.ts` the only ANT branch is `process.env.USER_TYPE === 'ant'` for thinking-signature stripping on `FallbackTriggeredError` (§5.4 / §8). The auto-mode classifier itself lives in `tools/BashTool/bashPermissions.ts` (per HANDOFF.md gotcha) and is consumed by `canUseTool` — outside this spec. **Estimated from evidence**: the dispatch prompt referred to the ANT signature-strip site, not a separate classifier site.
8. **`HookProgress`, `StopHookInfo`** type details — imported from `./types/hooks.js` and `./types/message.js`. Owned by spec 09.
9. **`createMicrocompactBoundaryMessage`** signature — imported from `./utils/messages.js` (`query.ts:54`). Used at `query.ts:884-891`; spec 07 owns the constructor.
10. **`refreshTools()` reentrancy** — `query.ts:1660-1671` calls it once per turn end. Whether MCP tool removal is supported (the comment only mentions additions) is not directly verifiable without spec 23.
11. **`StreamingToolExecutor` internal scheduler** (`StreamingToolExecutor.ts:200-530`) was sampled, not fully read. Public surface is verified; the in-flight progress signaling via `progressAvailableResolve` is described from class-level fields and the public methods — internal queue-walking subtleties may need spec 14 / 16 confirmation if multi-tool streaming proves to interact with MCP transports unexpectedly.
12. **`executeStopFailureHooks` event-name** — citation says `'StopFailure'` (`utils/hooks.ts:1640, 3604, 3613-3615`); the Zod schema for hooks (in `types/hooks.ts`) does not include `StopFailure` in the surveyed range (`73-160`). Either it lives elsewhere in `types/hooks.ts` or is registered through a programmatic path. Spec 09 should confirm.

— end —
