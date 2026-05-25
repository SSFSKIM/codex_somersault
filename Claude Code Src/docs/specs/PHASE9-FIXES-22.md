# Phase 9.6 Fix Log — Spec 22 (Service: Anthropic API Layer)

Source review: `PHASE9-ADVERSARIAL-22.md` (3 high, 5 medium, 4 low).
B-mini already fixed §12 Q5 stale "spec 02/06" → "spec 42a §3" pointer. This log
covers the remaining B-full work.

## Verification methodology

Sampled 10 cited line ranges from spec 22 and re-checked against source HEAD:

| Citation | Verified content | Result |
|---|---|---|
| `claude.ts:676-707` (Options) | `export type Options = { ... taskBudget?: { total: number; remaining?: number } }` ends line 707 | exact |
| `claude.ts:1979-2297` (SSE switch) | `switch (part.type)` at 1979, last `message_stop: break` at ~2295, yield ends 2304 | exact (matches "1979-2297" body) |
| `claude.ts:2382-2392` (`checkResponseForCacheBreak` call) | `void checkResponseForCacheBreak(...)` block | exact |
| `claude.ts:2607-2666` (404 fallback) | `is404StreamCreationError` at 2612, fallback through 2666 | exact |
| `claude.ts:2924-2987` (`updateUsage`) | `export function updateUsage` at 2924, returns object, ends 2987 | exact |
| `claude.ts:2993-3038` (`accumulateUsage`) | `export function accumulateUsage` at 2993 ends 3038 | exact |
| `claude.ts:3354` (`MAX_NON_STREAMING_TOKENS`) | `export const MAX_NON_STREAMING_TOKENS = 64_000` | exact |
| `claude.ts:3399` (`getMaxOutputTokensForModel`) | function declaration on this line | exact |
| `withRetry.ts:799` (`DEFAULT_FAST_MODE_FALLBACK_HOLD_MS`) | `const ... = 30 * 60 * 1000 // 30 minutes` | exact |
| `withRetry.ts:801` (`MIN_COOLDOWN_MS`) | `const MIN_COOLDOWN_MS = 10 * 60 * 1000 // 10 minutes` | exact |

**Drift sample: 0 of 10 cited line ranges were off.** The adversarial review's
"up to 30 lines" concern (which read §2 vs §5 internal-consistency) was a false
alarm: §2 cite `1979-2304` and §5.2 cite `1979-2297` describe the same SSE
switch — `2297` ends the last case body, `2304` ends the post-switch yield. Both
are correct for what they describe. Spec text now annotates this in §2
"Citation precision".

## Fixes applied (6 edits in spec 22)

1. **§2 "Citation precision" (new) — Pattern D HIGH.** Added a paragraph after
   `### Source-coverage status` documenting the verification methodology and
   asserting ±0 lines for the listed spans, ±5 lines elsewhere. Notes that the
   `Cite span: claude.ts:1017-2892` line in §5.1 is function-bracketing and
   approximate at bounds.
2. **§5.1 pseudocode — withResponse non-APIError fall-through HIGH.** Added a
   multi-line `# NOTE` comment inside the `withRetry` operation block,
   immediately after the `.withResponse()` call site, enumerating the two
   bespoke error branches (404 → CannotRetryError fallback at claude.ts:2612-2618;
   APIUserAbortError → claude.ts:2434-2462) and stating that ANY other throw
   (TypeError on malformed body, transport-level non-APIError, AggregateError)
   propagates uncaught to the outer `catch errorFromRetry`, where
   `getAssistantMessageFromError` + `classifyAPIError → 'unknown'` is the
   correct behavior. Explicit "MUST NOT add an early throw at this site"
   directive for reimplementers.
3. **§6.8 service_tier asymmetry HIGH (Pattern F).** Replaced the single-line
   "service_tier: keep prior (always)" bullet with a normative
   "REIMPLEMENTER HAZARD" callout explaining the intentional asymmetry with
   §6.9 `accumulateUsage`. Within a message: freeze tier at `message_start`.
   Across messages: take latest. Explicit "do NOT 'fix' either function"
   directive. Same callout extended to `inference_geo` (which has the same
   asymmetry pattern).
4. **§6.9 mirror callout HIGH (Pattern F).** Added a matching
   "REIMPLEMENTER HAZARD — intentional asymmetry with `updateUsage`" paragraph
   after the verbatim `accumulateUsage` listing, calling out that
   `service_tier`/`inference_geo` use most-recent here vs. kept-prior in
   `updateUsage`, while `iterations`/`speed` are most-recent in both. Explains
   the two-stage policy rationale (freeze within message, propagate across).
5. **§11 item 3 — `MIN_COOLDOWN_MS` literal expansion MED.** Rewrote item 3 to
   give every constant a unit and (where applicable) the source-equivalent
   arithmetic literal: `MIN_COOLDOWN_MS=600_000 ms (= 10 * 60 * 1000, 10 min — verbatim withRetry.ts:801)`,
   `DEFAULT_FAST_MODE_FALLBACK_HOLD_MS=1_800_000 ms (= 30 * 60 * 1000, 30 min — verbatim withRetry.ts:799)`,
   plus reader-aid annotations on all other ms-valued constants. Explicit note
   that the `(N min)` parentheticals are reader aids, not source text.
6. **§11 item 15 — extraBodyParams shadowing LOW/MED.** Appended "**Deliberate**:
   `...extraBodyParams` is spread *before* `output_config` and `speed` so
   user-provided `CLAUDE_CODE_EXTRA_BODY` cannot shadow them. Reordering this
   object literal is a security regression — preserve the spread position."

## Top 3 fixes by reimplementation impact

1. **§6.8/§6.9 Pattern F asymmetry callout** — without this, a reimplementer
   "cleaning up" the apparent inconsistency between `updateUsage` (keep prior
   `service_tier`) and `accumulateUsage` (take latest) would silently break
   per-message tier accounting and aggregate-tier reporting in opposite
   directions.
2. **§5.1 withResponse non-APIError clarification** — pseudocode previously
   showed the happy path with no commentary on the implicit fall-through to
   `getAssistantMessageFromError`. A reimplementer might add an explicit
   `try/catch (e instanceof TypeError)` and break the existing
   `classifyAPIError → 'unknown'` mapping that frontend rendering relies on.
3. **§11 item 3 literal expansion** — `600_000` and `1_800_000` now both carry
   their source-equivalent arithmetic forms, eliminating the
   "did the author intend `10 * 60 * 1000` or `60 * 10 * 1000`?" ambiguity.

## Findings skipped

- Adversarial finding §51 (branch 17 false-positive on Bedrock/Vertex
  `x-api-key` header dumps): not addressed — flagged in adversarial doc as
  "follow-up" requiring `errors.ts` re-read; out of scope for B-full.
- Adversarial finding §43-46 (latch-reset cross-check on `/clear`, `/compact`):
  cross-spec audit, not a spec 22 edit. Out of scope.
- Adversarial finding §34 (reverse-flag-cite checklist for spec 26): cross-spec
  edit (touches spec 26), not a spec 22 in-place fix. Out of scope.
- Adversarial finding §50 (Bedrock-vs-first-party `'x-api-key'` matching nuance):
  flagged for follow-up; not a HIGH/MED in B-full bucket.

## Net result

Spec 22 now contains explicit reimplementer-hazard callouts for the two most
load-bearing intentional contradictions in the source (`service_tier`
asymmetry, `withResponse` non-APIError fall-through), plus a normative
citation-precision section, ms-unit annotations on all retry constants, and
a security note on `extraBodyParams` ordering. No critical errors remained
after Phase 9.5; B-full hardens reimplementation safety.
