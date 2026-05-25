# Phase 9.5 Adversarial Review - Spec 04 Turn Pipeline

Severity counts: critical 0 / major 9 / minor 2

## Finding 1 - Spec 04 assigns the streaming API loop to QueryEngine, but source and spec 03 place it in `services/api/claude.ts`

- Section or line reference: spec 04 lines 8, 20, 53.
- Claim quote: "It is the bridge between the streaming LLM API loop (spec 03 - `QueryEngine.ts`)" and "`./QueryEngine.ts` (via `./query/deps.ts`) - `queryModelWithStreaming`".
- Src verification: `src/query/deps.ts:1-4` imports `queryModelWithStreaming` from `../services/api/claude.js`; `src/services/api/claude.ts:752-780` defines `queryModelWithStreaming`; `src/QueryEngine.ts:35` imports `query`, and `src/QueryEngine.ts:675-686` consumes `query()`.
- Severity: major.
- Type: contradicted-by-src / cross-spec drift.
- Recommended fix: Reword spec 04 so `QueryEngine` is the SDK/conversation consumer and `services/api/claude.ts` owns the streaming API loop, matching spec 03.

## Finding 2 - `src/query/transitions.ts` is cited as inspected and type-defining, but it is absent from this checkout

- Section or line reference: spec 04 lines 64, 92, 158, 1085.
- Claim quote: "`./query/{config,deps,transitions,stopHooks,tokenBudget}.js` - siblings" and "`src/query/transitions.ts` | grep-inspected".
- Src verification: `src/query.ts:104` imports `Terminal, Continue` from `./query/transitions.js`, but `src/query/transitions.ts` does not exist in the file inventory checked during review; `wc -l src/query/transitions.ts` and `nl -ba src/query/transitions.ts` failed with "No such file or directory".
- Severity: major.
- Type: stale code-organization claim / unverified type contract.
- Recommended fix: Add a missing-source ledger entry for transitions, or locate the real generated/source file before making exhaustive `Terminal` / `Continue` type claims.

## Finding 3 - The message type path is still unresolved; the spec asserts a generated resolution that is not present in `src/types/`

- Section or line reference: spec 04 lines 67, 94-99.
- Claim quote: "The path resolves via `src/types/generated/` per spec 08 §12" and "Pre-existing absent paths consumed indirectly".
- Src verification: `src/query.ts:30-39` imports `AssistantMessage`, `Message`, `StreamEvent`, `ToolUseSummaryMessage`, and `TombstoneMessage` from `./types/message.js`; `src/types/hooks.ts:15` also imports `src/types/message.js`; `rg --files src/types` showed no `message.ts`, `message.tsx`, or generated message module under `src/types/generated/`.
- Severity: major.
- Type: stale code-organization claim / unverified within budget.
- Recommended fix: Treat `src/types/message.js` as unresolved missing source unless the real generated path is found; do not assert it resolves through `src/types/generated/`.

## Finding 4 - Terminal reason enumeration says "12" but the observable source return sites contain 10 unique reasons

- Section or line reference: spec 04 lines 143-156 and 1059-1060.
- Claim quote: "The 12 terminal reasons (string set)".
- Src verification: `src/query.ts:646,977,996,1051,1175,1182,1264,1279,1357,1515,1520,1711` return these unique reasons: `blocking_limit`, `image_error`, `model_error`, `aborted_streaming`, `prompt_too_long`, `completed`, `stop_hook_prevented`, `aborted_tools`, `hook_stopped`, `max_turns`.
- Severity: major.
- Type: false enumeration.
- Recommended fix: Change "12 terminal reasons" to "10 observed return reasons" unless the absent `transitions` source provides additional real values and they are clearly marked unused.

## Finding 5 - `ToolUseSummaryMessage` is incorrectly described as ANT-only / StatSig-gated

- Section or line reference: spec 04 line 141.
- Claim quote: "`ToolUseSummaryMessage` (deferred summary from previous turn, ANT-only StatSig)".
- Src verification: `src/query/config.ts:36-38` gates summaries on `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES`; `src/query.ts:1416-1420` checks that env-derived gate, tool blocks, non-abort, and `!toolUseContext.agentId`; no ANT or StatSig check is involved for summary generation.
- Severity: major.
- Type: contradicted-by-src / naming-gating drift.
- Recommended fix: Replace "ANT-only StatSig" with "env-gated by `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES`, main thread only (`!agentId`)."

## Finding 6 - The reimplementation checklist overgeneralizes deferred context modifiers; streaming execution drops concurrent context modifiers

- Section or line reference: spec 04 lines 241, 243, 1068.
- Claim quote: "Read-only batches collect modifiers for application after the batch joins" and "`StreamingToolExecutor` ... replaces `runTools`".
- Src verification: `src/services/tools/toolOrchestration.ts:30-63` does defer modifiers for non-streaming concurrent batches, but `src/services/tools/StreamingToolExecutor.ts:379-395` only applies collected context modifiers when `!tool.isConcurrencySafe` and explicitly comments that concurrent-tool context modifiers are not currently supported.
- Severity: major.
- Type: missing edge case / hidden variant assumption.
- Recommended fix: Split the invariant by executor: `runTools` defers concurrent modifiers; `StreamingToolExecutor` currently ignores concurrent modifiers and only applies them for non-concurrency-safe tools.

## Finding 7 - `tool.call` throw output is not wrapped in `<tool_use_error>` in the inner call path

- Section or line reference: spec 04 lines 644 and 1004.
- Claim quote: "yield `<tool_use_error>${error}</tool_use_error>` with `is_error:true`".
- Src verification: `src/services/tools/toolExecution.ts:1691` sets `content = formatError(error)`; `src/services/tools/toolExecution.ts:1715-1727` puts that raw `content` into the `tool_result`; `src/utils/toolErrors.ts:5-21` returns plain formatted strings, not `<tool_use_error>` wrappers.
- Severity: major.
- Type: contradicted-by-src / verbatim-string drift.
- Recommended fix: Document the outer `runToolUse` catch wrapper separately from the inner `checkPermissionsAndCallTool` catch, which returns raw `formatError(error)` content.

## Finding 8 - PostToolUse `preventContinuation` is underspecified in the critical-path steps

- Section or line reference: spec 04 lines 641-643.
- Claim quote: "`shouldPreventContinuation`: if a PreToolUse hook said so, emit a `hook_stopped_continuation` attachment".
- Src verification: `src/services/tools/toolHooks.ts:117-129` emits `hook_stopped_continuation` for `PostToolUse` when `result.preventContinuation`; `src/query.ts:1388-1393` treats any such attachment as `shouldPreventContinuation = true`; `src/query.ts:1518-1520` returns `hook_stopped`.
- Severity: major.
- Type: underspecified edge case.
- Recommended fix: State that both PreToolUse and PostToolUse can stop continuation, with different `hookEvent` / `hookName` values.

## Finding 9 - Hook event-name string set is incomplete and cites the wrong authoritative source

- Section or line reference: spec 04 lines 723-750 and 1074.
- Claim quote: "Hook event-name string literals (verbatim) From `src/types/hooks.ts:73-160` and `src/utils/hooks.ts:1627-1646`".
- Src verification: `src/entrypoints/sdk/coreTypes.ts:25-53` defines the runtime `HOOK_EVENTS` array and includes additional events absent from the spec list, including `PostCompact`, `TaskCreated`, `ConfigChange`, `WorktreeRemove`, and `InstructionsLoaded`; `src/types/hooks.ts:70-163` is a hook output schema subset, not the event-name authority.
- Severity: major.
- Type: false enumeration / cross-spec drift with spec 09.
- Recommended fix: Cite `src/entrypoints/sdk/coreTypes.ts:25-53` for the full event set, then keep a separate table for only the events invoked by the turn pipeline.

## Finding 10 - `src/cli/print.ts` is listed as a direct downstream caller of `query()`, but it calls `ask()` through QueryEngine

- Section or line reference: spec 04 lines 69-77.
- Claim quote: "Imported by (downstream callers of `query()`): `src/cli/print.ts` - `-p` / non-interactive mode."
- Src verification: `src/cli/print.ts:91` imports `ask` from `src/QueryEngine.js`; `src/cli/print.ts:2147-2162` iterates `ask(...)`; `src/QueryEngine.ts:1186-1225` defines `ask`, and `src/QueryEngine.ts:675-686` is where `query()` is actually invoked.
- Severity: minor.
- Type: stale code-organization claim.
- Recommended fix: Mark `src/cli/print.ts` as an indirect caller via `QueryEngine.ask()`, not a direct `query()` importer.

## Finding 11 - The summary latency claim is a performance assumption, not a source-verifiable invariant

- Section or line reference: spec 04 lines 331-332.
- Claim quote: "Resolves under Haiku (~1s) during the next 5-30s stream".
- Src verification: `src/query.ts:1411-1482` creates a deferred `generateToolUseSummary(...)` promise, and `src/services/toolUseSummary/toolUseSummaryGenerator.ts:69-81` calls `queryHaiku`; there is no source-level timeout, SLA, or guarantee that the next stream lasts 5-30 seconds.
- Severity: minor.
- Type: untestable / unfalsifiable within source.
- Recommended fix: Rephrase as an observed performance heuristic and keep the invariant to "deferred promise generated after tool batch and awaited on the next iteration."

## Spec-Level Verdict

Spec 04 captures the broad turn-loop shape, but it is not safe as a behaviorally exact rebuild guide yet. The largest risks are stale source boundaries, false enumerations, and executor-variant semantics that would cause a reimplementation to preserve the wrong behavior.

## Cross-Spec Impact

- Spec 03: Spec 04 contradicts spec 03's boundary that `QueryEngine` consumes `query()` while `services/api/claude.ts` owns streaming/retry mechanics.
- Spec 08: The unresolved `src/types/message.js` and absent `src/query/transitions.ts` weaken any shared type-contract claims that spec 08 depends on.
- Spec 09: The hook event-name list should be reconciled with `src/entrypoints/sdk/coreTypes.ts:25-53` and permission/hook docs, especially `StopFailure`, `PostCompact`, `TaskCreated`, `ConfigChange`, and `InstructionsLoaded`.
- Spec 22: Streaming/retry/fallback ownership should point to `services/api/claude.ts` and `withRetry.ts`, not `QueryEngine.ts`.
