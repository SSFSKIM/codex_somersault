# Phase 9.6c — Spec 06 Fix Log

> Source spec: `docs/specs/06-cost-token-tracking.md`
> Adversarial input: `docs/specs/PHASE9-ADVERSARIAL-06.md`
> Date applied: 2026-05-10
> Methodology: VERIFY-BEFORE-EDIT against `src/cost-tracker.ts`, `src/utils/modelCost.ts`, `src/utils/advisor.ts`, `src/services/api/claude.ts`, `src/services/tokenEstimation.ts`.

## Severity scope applied

Major (2) + Medium (3) + STATE.hasUnknownModelCost persistence note. Nits (F6/F7/F9/F10) deferred — already verified accurate in adversarial review.

---

## F1 — MAJOR — Advisor "double-count" framing reconciled with §10 invariant 7

**Verified against source**:
- `cost-tracker.ts:304–321` — `addToTotalSessionCost` recurses on each `iterations[*]` of `type === 'advisor_message'`. The advisor's tokens are added to `STATE.modelUsage[advisorUsage.model]` (advisor's own model, not the parent's), and cost is `calculateUSDCost(advisorUsage.model, advisorUsage)`.
- `utils/advisor.ts:115–128` — `getAdvisorUsage` only filters `usage.iterations` for `type === 'advisor_message'`; no aggregation logic that would imply parent already includes advisor tokens.
- `services/api/claude.ts:2984, 3035` — `iterations` is propagated as-is in `updateUsage` (`partUsage.iterations ?? usage.iterations`) and `accumulateUsage` (`messageUsage.iterations`, most-recent wins). No code-path observed in-scope that rolls iteration tokens into the parent's `input_tokens`/`output_tokens`.

**Verdict**: §10 invariant 7 is correct (advisor event reflects only its own iteration cost); §5.1 Note was wrong to assert "double-counts at the model-usage and counter level by design".

**Fix applied**: Rewrote §5.1 Note to state advisor iterations are billed *separately* by the server, attributed to the advisor's own model entry, no double-count. Cross-references §10 invariant 7 and defers producer-side contract to spec 22.

## F2 — MAJOR — `calculateUSDCost` line range corrected

**Verified against source**:
- `modelCost.ts:177–180` — entire `calculateUSDCost` body (4 lines): single-line return delegating to `tokensToUSDCost(modelCosts, usage)`.
- `modelCost.ts:186–202` — `calculateCostFromTokens`: separate helper that synthesises a `Usage` and then calls `calculateUSDCost`; doc comment names "side queries (e.g. classifier)".

**Fix applied**:
1. §5.2 header changed from `:177–202` to `:177–180`; pseudocode now shows the actual two-line body and surfaces `tokensToUSDCost` as the helper.
2. New §5.2a documents `calculateCostFromTokens` (`:186–202`) and explicitly tags it as the only caller surface that bypasses `addToTotalSessionCost`, cross-referencing spec 09 (side queries) and §1/§8 out-of-scope notes.

## F3 — MEDIUM — `service_tier` / `inference_geo` non-consumption documented

**Verified against source**:
- `cost-tracker.ts`, `modelCost.ts`, `tokenEstimation.ts` — no references to `service_tier` or `inference_geo`.
- `services/api/claude.ts:2983, 3013, 3034` — both fields are populated by producer-side `updateUsage`/`accumulateUsage` with the asymmetric persistence (most-recent wins on accumulate, prior-preserved on update — flagged in spec 22).

**Fix applied**: §6.3 now explicitly enumerates `service_tier` and `inference_geo` as present-but-not-consumed, references spec 22 producer asymmetry, and notes that any pricing variance by tier/region would require reworking `getModelCosts` and `MODEL_COSTS`.

## F4 — MEDIUM — Image vs document estimate direction-asymmetry corrected

**Verified against source comment** (`tokenEstimation.ts:400–411`):
- Image: max 2000×2000 → ~5333 API-billed tokens; `2000` constant is conservative-low (mild under-estimate ~2.7×).
- Document (PDF base64 in `source.data`): comment says "1MB PDF is ~1.33M base64 chars → ~325k *estimated* tokens vs the ~2000 the API actually charges". The `2000` is **replacing** the catch-all `jsonStringify` path (which would over-estimate ~162×). So for documents the constant **prevents over-estimate**, not under-estimate.

**Verdict**: F4 in adversarial review framed direction wrong for documents — the source comment is being honest. The genuine risk is image under-estimate, not document under-estimate.

**Fix applied**: New §10 invariant 10 captures the asymmetry correctly with both image (~5333 → 2000 = mild under-estimate) and document (~325k catch-all → 2000 = over-estimate corrected) paths sourced from the comment. Cross-references spec 11.

## F5 — MEDIUM — Phase 9.7 C1 (latch) + C2 (midnight) callouts elevated; STATE.hasUnknownModelCost persistence

**Verified against source**:
- `state.ts:745–747` — `setHasUnknownModelCost` is a one-way latch (sets to true, never clears in normal flow).
- `state.ts:864–875` — `resetCostState()` does clear it.
- `cost-tracker.ts:144–174` — persistence keys written by `saveCurrentSessionCosts` enumerated; `hasUnknownModelCost` is **not** in the list.
- `cost-tracker.ts:228–233` — `formatTotalCost` appends "(costs may be inaccurate due to usage of unknown models)" suffix iff `hasUnknownModelCost()`.

**Fix applied**: New §10 invariants 11 (C1 — latch not persisted across resume), 12 (C2 — no midnight rollover on `STATE.totalCostUSD`), 13 (`contextWindow` reflects latest beta set per F8 from adversarial review). Added a callout block elevating these as Phase 9.7 cost-multiplier surfaces with spec 26 routing for the unknown-model signal.

---

## Top 3 fixes (with src evidence)

1. **F1 — Advisor double-count claim removed** (`cost-tracker.ts:304-321`, `utils/advisor.ts:115-128`). The recursion attributes advisor tokens to the advisor's own `STATE.modelUsage[model]` entry; nothing in the in-scope code suggests parent input/output tokens already include advisor totals. §5.1 Note now cross-references §10 invariant 7 and defers the producer-side contract to spec 22, eliminating the self-contradiction.

2. **F2 — `calculateUSDCost` line range corrected to `:177–180`; new §5.2a for `calculateCostFromTokens` (`:186–202`)** (`modelCost.ts:177–202`). The original `:177–202` swallowed an out-of-scope helper used precisely by side-query/classifier consumers that §1 and §8 mark as excluded; the split now matches the source and tags `calculateCostFromTokens` as the only `addToTotalSessionCost`-bypass surface.

3. **F5 — STATE.hasUnknownModelCost persistence + C1/C2 callouts** (`cost-tracker.ts:144–174`, `state.ts:745–747`, `state.ts:864–875`). The persistence keys list is concrete: `lastCost`, `lastAPIDuration`, …, `lastSessionId`. The unknown-cost latch is **not** among them. New invariants 11–13 plus an elevated Phase 9.7 callout document this so spec 26 (analytics) and resume audits can treat `tengu_unknown_model_cost` correctly as per-session-not-resumed.
