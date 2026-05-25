# Phase 9.5b Adversarial Review — Spec 06 (Cost & Token Tracking)

> Reviewer role: skeptic · Target: `docs/specs/06-cost-token-tracking.md`
> Verified against: `src/cost-tracker.ts`, `src/costHook.ts`, `src/services/tokenEstimation.ts`, `src/utils/modelCost.ts`, `src/utils/billing.ts`, `src/utils/advisor.ts`, `src/query/tokenBudget.ts`, `src/utils/tokenBudget.ts`, `src/services/vcr.ts`, `src/bootstrap/state.ts` (cost slice).

## Severity Counts

| Severity | Count |
|---|---|
| Critical (factually wrong / contradicts source) | 0 |
| Major (misleading / falsifiable claim that fails) | 2 |
| Minor (stale / loose / under-qualified) | 5 |
| Nit (cosmetic / line-number drift) | 3 |
| **Total** | **10** |

Verdict: **PASS WITH FIXES**. No critical defects. Two major findings require correction before phase exit; minors are cleanup.

---

## Findings

### F1 — MAJOR — Advisor "double-count" claim is overstated

**Spec §5.1 Notes**: *"the parent's `usage.input_tokens`/`usage.output_tokens` already include advisor tokens at the API layer (see §10), so this recursion **double-counts** at the model-usage and counter level by design"*.

**Source check**: `cost-tracker.ts:304-321` recurses on each `iterations[]` entry of `type === 'advisor_message'`. Both the parent BetaUsage and each advisor BetaUsage are added in full to `STATE.modelUsage[model]`, and `STATE.totalCostUSD` accumulates parent cost + advisor cost.

**Problem**: The spec asserts the API layer already rolls advisor tokens into the parent's `input_tokens`/`output_tokens` and that this code intentionally double-counts. There is **no source-level evidence** in the in-scope files for that API-layer aggregation claim — `getAdvisorUsage` (`utils/advisor.ts:115-128`) just filters `usage.iterations`. §10 (cited) makes the *opposite* observation in invariant 7 ("`tengu_advisor_tool_token_usage` … reflects only the advisor's own iteration cost"). And the same §5.1 also calls `addToTotalModelUsage` "mutating: increments each per-model field" — so if parent already includes advisor, every advisor recursion *would* break monotonicity vs reported API spend.

**Recommendation**: Either cite the API-layer aggregation in `services/api/claude.ts` to support the double-count claim, or rewrite the note to match the more defensible read: "advisor iterations are billed *separately* by the server and the recursion attributes them to their own model — there is no double-count when the server's `iterations[*]` carry the advisor's own token totals." This matters because the §5.1 note directly contradicts §10 invariant 7. Cross-spec impact: spec 22 (SDK/server contract).

### F2 — MAJOR — `calculateUSDCost` line range is wrong

**Spec §5.2 header**: *"`calculateUSDCost(resolvedModel, usage)` — `src/utils/modelCost.ts:177–202`"*.

**Source check**: `modelCost.ts:177-180` is the entire `calculateUSDCost` function (4 lines). Lines 182-202 are `calculateCostFromTokens`, an unrelated helper for side-query / classifier consumers (the very call site §1's "out of scope" bullet points to). The spec then describes only `tokensToUSDCost` body in pseudocode without ever naming `calculateCostFromTokens`.

**Problem**: §2 "fully-read 232 lines" is plausible, but §5.2 conflates two functions, and §1/§8 *explicitly say* classifier accounting is out of scope while §5.2 silently widens its line range to swallow exactly that helper. A reader using §2/§5.2 to navigate will hit the wrong function.

**Recommendation**: Split into §5.2 `calculateUSDCost` (`:177-180`) and a new §5.2a `calculateCostFromTokens` (`:186-202`), and note in §5.2a that this is the *only* caller surface that bypasses `addToTotalSessionCost`.

### F3 — MINOR — `service_tier` / `inference_geo` asymmetry not addressed

**Phase 9.6 spec 22 finding**: `BetaUsage` carries `service_tier` and `inference_geo` fields whose persistence policies differ between `updateUsage` (keeps prior) and `accumulateUsage` (most-recent wins).

**Source check**: Neither field appears anywhere in `cost-tracker.ts`, `modelCost.ts`, `tokenEstimation.ts`. The cost subsystem reads only `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `server_tool_use.web_search_requests`, `speed`, `iterations`.

**Problem**: Spec 06 §6.3 "BetaUsage field names consumed (verbatim)" enumerates seven fields and is *correct* — but this is exactly the kind of enumeration that lets the consumer-side asymmetry hide. The spec should explicitly note that `service_tier` and `inference_geo` are **not consumed** here, and direct the reader to spec 22 for the producer-side asymmetry. Currently the omission could be read as "those fields don't exist."

**Recommendation**: Add one line under §6.3 or §10 invariant: *"`service_tier` and `inference_geo` are present on `BetaUsage` but not consumed by cost-tracker; spec 22 owns producer-side accumulation."*

### F4 — MINOR — Image/document `2000` token estimate falsifiability

**Spec §5.7 / §6.10**: image+document blocks → constant `2000` tokens, with prose "matches `microCompact`'s `IMAGE_MAX_TOKEN_SIZE`; chosen to **avoid underestimating**".

**Source check**: `tokenEstimation.ts:400-412` confirms the constant `2000`, and the source comment itself says *"Use a conservative estimate that matches microCompact's IMAGE_MAX_TOKEN_SIZE to avoid underestimating and triggering auto-compact too late"*. So the spec is faithful to the comment.

**Problem (skeptic angle, parallel to spec 11 finding)**: The spec doesn't quantify the *upper-bound* this estimate covers. Source comment: 1MB PDF → ~325k API-billed tokens vs ~2000 estimated → **162× under-estimate** for documents. The spec inherits the source's "avoid underestimating" framing without flagging that for `document` blocks the constant **routinely** under-estimates by orders of magnitude. This is a load-bearing claim because compaction triggering depends on it.

**Recommendation**: Add an §8 edge-case bullet: "image blocks: 2000-token estimate is conservative-high (real images cap ~5333 tokens). document (PDF): 2000-token estimate can be 100× too low for large PDFs — auto-compact may fire late on document-heavy turns."

### F5 — MINOR — Phase 9.7 cost-multiplier invariants (C1 latch, C2 midnight) not acknowledged

**Spec §10**: lists 9 invariants; none mention C1 or C2.

**Source check (in-scope files)**: `STATE.hasUnknownModelCost` *is* a one-way latch (§10 invariant 1 captures the cost-state monotonicity but not the unknown-cost latch). `formatTotalCost` adds the "(costs may be inaccurate due to usage of unknown models)" suffix permanently after first hit (`cost-tracker.ts:228-233`). No date/midnight rollover logic in the in-scope files.

**Problem**: Phase 9.7 flagged C1 (cost-multiplier latch persistence) and C2 (midnight) as 10× risk areas. Spec 06 owns the latch surface (`setHasUnknownModelCost`) and the cumulative `STATE.totalCostUSD` (no midnight reset) but doesn't surface either as a 10× hazard. The session persistence path (`saveCurrentSessionCosts` / `restoreCostStateForSession`) further means the latch can survive an exit if `lastSessionId` matches on resume — though `hasUnknownModelCost` itself is **not persisted** (only `lastCost`/`lastModelUsage`/etc. are written; see `:144-174`). On resume the latch starts false again.

**Recommendation**: Add §10 invariants:
- *C1: `STATE.hasUnknownModelCost` is a one-way latch within a session, **not persisted** across sessions; resume via `restoreCostStateForSession` does not re-hydrate the latch even if the prior session hit an unknown model.*
- *C2: `STATE.totalCostUSD` has no midnight/day rollover; cumulative session cost grows monotonically until `resetCostState()` or process exit.*

### F6 — MINOR — `formatModelUsage` line range drift

**Spec §4 / §5.3**: `formatModelUsage` `:181–226`.

**Source check**: `formatModelUsage` is `:181-226` — correct. But §5.3 prose says `padStart(21)` "padded to width 21 (`${shortName}:`.padStart(21)`)" — source line is `:223` (`${shortName}:`.padStart(21)`), confirmed. **No drift here**, but §6.10 says "shortName padding | width 21 | `cost-tracker.ts:223`" while §5.3 says `:181–226`. Acceptable but inconsistent granularity.

**Recommendation**: Cosmetic; no action required.

### F7 — MINOR — `getStoredSessionCosts` line range slightly off

**Spec §4 row "Persisted-resume snapshot"**: `getStoredSessionCosts` (`:87–123`).

**Source check**: function signature at `:87`, returns at `:122`, closes at `:123`. **Spec correct.** `restoreCostStateForSession (`:130–137`) — verified `:130-137`. `saveCurrentSessionCosts (`:143–175`) — verified `:143-175`. All three are accurate.

**Recommendation**: None — flagging as verified.

### F8 — MINOR — `addToTotalModelUsage` "overwrites contextWindow / maxOutputTokens on every call" — verify wording

**Spec §5.1 Notes**: *"`addToTotalModelUsage` (`:250–276`) is **mutating**: it reads `getUsageForModel(model)` (or initialises a zero-filled `ModelUsage`), increments each per-model field by the new usage's fields, and **overwrites** `contextWindow = getContextWindowForModel(model, getSdkBetas())` and `maxOutputTokens = getModelMaxOutputTokens(model).default` on every call (i.e. these reflect the *latest* known values, not the value at first observation)."*

**Source check**: `cost-tracker.ts:266-275` — every field increment uses `+=` except `contextWindow` and `maxOutputTokens`, which use direct `=`. Spec is **exactly correct**.

**Skeptic concern**: `getSdkBetas()` is read on every call. If beta header set changes mid-session (e.g. an advisor enables a 1P-only beta), `contextWindow` for the *advisor's* model could differ from the one cached for the parent. The spec doesn't call this out.

**Recommendation**: Append to §10 invariants: "`contextWindow` reflects the SDK beta set at the time of the latest API call for that model. Beta-header changes between calls can alter the displayed window."

### F9 — NIT — `tokenEstimation.ts:411` says image **AND document** = 2000

**Spec §6.10 / §5.7**: "Image/document block tokens | `2000` | `tokenEstimation.ts:411`".

**Source check**: line 400 is `if (block.type === 'image' || block.type === 'document') {`, line 411 is the `return 2000`. Spec correct on the value but the line cited is the `return`, not the dispatch — fine for navigation.

**Recommendation**: None.

### F10 — NIT — `incrementBudgetContinuationCount` vs `tracker.continuationCount++`

**Spec §5.6 + §10 invariant 5**: token-budget continuation injects `isMeta: true` user message and "calls `incrementBudgetContinuationCount()`".

**Source check**: `tokenBudget.ts:65` does `tracker.continuationCount++` on the *tracker* object (local), and the spec text at §5.6 also says the integration site calls `incrementBudgetContinuationCount()` (state-level counter at `bootstrap/state.ts:741-743`). Both counters exist and are independent — `tracker.continuationCount` is per-decision, `STATE.budgetContinuationCount` is per-turn. Spec §4 lists both. The integration call is described in the integration paragraph but not annotated as "two distinct counters" — risk of conflation.

**Recommendation**: Add one line: "Two distinct continuation counters exist: `BudgetTracker.continuationCount` (local, per checkTokenBudget call chain) and `STATE.budgetContinuationCount` (global, reset by `snapshotOutputTokensForTurn`). Both are incremented on `'continue'`."

---

## Verdict

**PASS WITH FIXES.** Spec 06 is largely accurate — pricing tables, format strings, OTel counter names, regex patterns, and line ranges are reproduced verbatim and verifiable. Two majors (F1 advisor double-count framing self-contradicts §10; F2 `calculateUSDCost` line range bleeds into out-of-scope helper) and five minors should be addressed before phase exit. No critical defects; no source contradictions on pricing math, persistence keys, or OTel attribute schemas.

## Cross-Spec Impact

| Spec | Touch | Reason |
|---|---|---|
| **22** (SDK client) | F1, F3 | Advisor-iteration server contract; `service_tier`/`inference_geo` producer-side asymmetry — spec 06 should defer to 22 instead of asserting double-count |
| **11** (Compaction) | F4 | Image/document 2000-token estimate is the same shared constant; spec 11's unfalsifiability finding for image base64 ratio mirrors document-PDF case here |
| **04** (Turn pipeline) | F10 | Two-counter naming distinction matters for turn-budget UI rendering |
| **26** (Analytics) | F5 | `tengu_unknown_model_cost` is the only signal that surfaces the C1 latch; analytics consumers need to know it's per-session-not-resumed |
| **09** (Side queries) | F2 | `calculateCostFromTokens` is the side-query bypass; should be cited explicitly as the boundary |

## Hardest-to-Verify Claim

**§10 invariant 8** — *"Side-query / classifier costs are intentionally excluded (`utils/permissions/permissions.ts:766` — comment confirms classifier path 'does NOT call addToTotalSessionCost')."*

This is verifiable only by reading a file outside spec 06's stated coverage and trusting a comment, not a structural absence — the spec does not enumerate the full call graph reaching `addToTotalSessionCost` to prove no side-query path leaks in. Spec 09 owns the policy but spec 06 cannot independently confirm exclusion without a global call-site audit. The `calculateCostFromTokens` helper (F2) shows the bypass surface exists; whether anything *else* (advisor's nested calls, VCR replay re-attribution, the new server-side `task_budget` tracker referenced in §12 OQ#5) might reach it is not closed off.
