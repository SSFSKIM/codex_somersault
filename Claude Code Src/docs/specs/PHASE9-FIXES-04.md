# Phase 9.6 B-full — Spec 04 Turn Pipeline fix log

Source: PHASE9-ADVERSARIAL-04.md (verdict: "directionally useful but not safe as exact rebuild guide", 9 major + 2 minor findings).

## Findings applied

| # | Severity | Finding | Status | Spec edits |
|---|---|---|---|---|
| 1 | major | Streaming ownership wrongly attributed to `QueryEngine`; `queryModelWithStreaming` is in `src/services/api/claude.ts:752` and consumed via `query/deps.ts:2` (injected as `QueryDeps.callModel`); `QueryEngine.ts:675-686` consumes `query()`, not the API loop. | **Applied** | §1 Purpose & Scope reworded; §2.2 import line points to `services/api/claude.ts` and notes pipeline decoupling via `deps.callModel`. |
| 2 | major | `src/query/transitions.ts` is cited as inspected but does not exist in the checkout (`find src/query -type f` lists only `config.ts`, `deps.ts`, `stopHooks.ts`, `tokenBudget.ts`). | **Applied** | §2.4 row updated to "MISSING from this checkout"; §2.5 ledger gains a new entry cross-referencing spec 00 §2.5; §12 Q1 rewritten to match. |
| 3 | major | Spec asserted "12 terminal reasons" but only 10 unique reasons appear at `query.ts` return sites (verified via `grep -n "return.*reason:" src/query.ts`: blocking_limit, image_error, model_error, aborted_streaming, prompt_too_long, completed×2, stop_hook_prevented, aborted_tools, hook_stopped, max_turns). | **Applied** | §1 changed `~12` → `10 observed`; §3.1 preface changed to "There are 10 observed terminal reasons"; §11 invariant 2 rewritten to enumerate the 10 by name; §12 Q1 corrected. |
| 4 | major | `ToolUseSummaryMessage` was described as ANT-only / StatSig-gated; actual gate is `CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES` env var (`query/config.ts:36-38`) plus `!toolUseContext.agentId` (`query.ts:1416-1420`). | **Applied** | §3.1 generator-output bullet updated; §4.6 tool-use summary bullet updated; §11 invariant 21 rewritten with explicit "Not USER_TYPE / StatSig gated" note. |
| 5 | major | `StreamingToolExecutor.ts:379-395` only applies `contextModifier`s when `!tool.isConcurrencySafe`; concurrent context modifiers are silently dropped (explicit source comment). Spec previously implied universal deferred-modifier semantics. | **Applied** | §3.4 (contextModifier discussion) split into per-executor subsections quoting the source comment; §11 invariant 10 rewritten as a per-executor checklist with the asymmetry made explicit. |
| 6 | major | Hook event-name list cited the wrong authority (`src/types/hooks.ts:73-160` is a Zod output subset). True authority is `src/entrypoints/sdk/coreTypes.ts:25-53` (`HOOK_EVENTS`); list was missing `PostCompact`, `TaskCreated`, `ConfigChange`, `WorktreeRemove`, `InstructionsLoaded`. | **Applied** | §6.2 rewritten to cite `coreTypes.ts:25-53`, lists the full 27-event runtime set in declaration order, notes `types/hooks.ts` is a schema subset, and clarifies that pipeline only invokes a subset (existing call-site table preserved). §11 invariant 16 updated. |
| 7 | major | "yield `<tool_use_error>${error}</tool_use_error>` with is_error:true" was wrong for the inner call path: `toolExecution.ts:1691, 1715-1727` and `utils/toolErrors.ts:5-21` show inner returns plain `formatError(error)` content. The XML wrapper is applied by an outer catch. | **Applied** | §5.8 step 18 rewritten to document the inner/outer distinction and cite line numbers; rebuild requirement added to "preserve two-level wrapping". |
| 8 | major | PostToolUse `preventContinuation` underspecified — `toolHooks.ts:117-129` shows PostToolUse can also emit `hook_stopped_continuation`, with `hookEvent: 'PostToolUse'` rather than `'PreToolUse'`. | **Applied** | §5.8 step 17 split into Pre/Post bullets with distinct `hookName`/`hookEvent` payloads and the `query.ts:1388-1393` / `1518-1520` flow to `Terminal.reason='hook_stopped'`. |
| 10 | minor (kept; verdict-relevant) | `src/cli/print.ts` is an indirect, not direct, caller of `query()` — it imports `ask` from `QueryEngine.js`. | **Applied** | §2.3 bullet annotated as **indirect** with citation lines `print.ts:91` and `QueryEngine.ts:675-686`. |
| 11 | minor (kept; verdict-relevant) | "Resolves under Haiku (~1s) during the next 5-30s stream" is a perf heuristic, not a source invariant. | **Applied** | §4.6 tool-use summary bullet rephrased — keeps the deferred/awaited-next-iteration invariant, drops the timing claim, cites `toolUseSummaryGenerator.ts:69-81`. |

Plus: §2.5 ledger now also flags the unresolved `src/types/message.js` import (no `src/types/message.ts`, no `src/types/generated/` module).

## Findings skipped

None of the listed major/minor findings were skipped. Pure cosmetic LOW items in PHASE9-ADVERSARIAL-04.md were already absent from the explicit findings list.

## Top 5 fixes — src evidence

1. **Streaming ownership** — `src/query/deps.ts:2` `import { queryModelWithStreaming } from '../services/api/claude.js'`; `src/services/api/claude.ts:752` defines `export async function* queryModelWithStreaming`; `src/QueryEngine.ts:675-686` `for await (const message of query({ ... }))` is the consumer.
2. **`transitions.ts` missing** — `find /Users/new/Downloads/claude-code-main/src/query -type f` returns 4 files, none named `transitions.ts`; `src/query.ts:104` still imports `Terminal, Continue` from `./query/transitions.js`.
3. **10 terminal reasons** — `grep -n "return.*reason:" src/query.ts` produces exactly 12 lines but only 10 unique `reason:` literals: `blocking_limit, image_error, model_error, aborted_streaming, prompt_too_long, completed, stop_hook_prevented, aborted_tools, hook_stopped, max_turns` (`completed` and `prompt_too_long` each occur twice).
4. **Tool-use summary gate** — `src/query/config.ts:36-38` `emitToolUseSummaries: isEnvTruthy(process.env.CLAUDE_CODE_EMIT_TOOL_USE_SUMMARIES)`. No `USER_TYPE` or StatSig branch on that field.
5. **Streaming concurrent context-modifier drop** — `src/services/tools/StreamingToolExecutor.ts:388-395` literal: `// NOTE: we currently don't support context modifiers for concurrent / tools. None are actively being used, but if we want to use / them in concurrent tools, we need to support that here. // if (!tool.isConcurrencySafe && contextModifiers.length > 0) { for (const modifier of contextModifiers) { this.toolUseContext = modifier(this.toolUseContext) } }`.

## Post-fix verdict

**Improved — still treat with caution as a rebuild guide.**

Fixes raise the spec from "directionally useful but not safe as exact rebuild guide" to "structurally sound for rebuild, with clearly-flagged residual risks":

- Streaming/loop ownership now matches specs 03 and 22 — the `services/api/claude.ts` boundary is explicit and the pipeline is positioned as a `deps.callModel` consumer.
- All false enumerations corrected (terminal-reason count, hook-event authority).
- Executor-variant asymmetry (streaming context-modifier drop) is now load-bearing in invariant 10.
- Inner/outer error-wrapping distinction is documented; a rebuild can preserve VCR/transcript stability.

Residual risk that prevents an unconditional "safe as exact rebuild guide":

- `src/query/transitions.ts` is still missing — the `Terminal`/`Continue` union types may declare unused literals not derivable from return sites. Ledger flag is in place; rebuild MUST recover this source before claiming exhaustiveness.
- `src/types/message.ts` remains unresolved.
- §6.2 Zod-schema citations (`src/types/hooks.ts:73-160`) for legacy table rows were left untouched where they pointed at hook-output payload schemas, not the event-name set; cross-spec reconciliation with spec 09 is recommended but is out of scope for this pass.

Net: spec 04 is now safe to use as the **structural** rebuild guide and as input to spec 09's hook reconciliation, but the remaining missing-source items must be resolved before declaring behavioural exactness.
