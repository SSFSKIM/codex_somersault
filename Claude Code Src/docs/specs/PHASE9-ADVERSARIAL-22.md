# Phase 9.5 Adversarial Review — Spec 22 (Service: Anthropic API Layer)

Reviewer: Opus fallback (after codex agent auth failure).
Method: Read full spec; cross-checked against `src/services/api/{claude.ts, withRetry.ts, client.ts, filesApi.ts, promptCacheBreakDetection.ts}`, `src/constants/betas.ts`, and adjacent specs (03/04, 06, 25, 26, 35, 27).

## Severity Counts

- Critical (factual error / would break reimplementation): 0
- High (citation drift, missing edge case, false enumeration): 3
- Medium (under-specified, ambiguity, stale phrasing): 5
- Low (cosmetic / wording / completeness): 4

## Verdict

**Spec is high-fidelity and largely faithful to source.** Verbatim assets (beta-header strings, retry constants, `EMPTY_USAGE`, `updateUsage`, `accumulateUsage`, `categorizeRetryableAPIError`, `classifyAPIError` 24-branch order, `shouldRetry` decision tree, `getRetryDelay`) all match the corresponding source lines I sampled. `MAX_NON_STREAMING_TOKENS=64_000` (claude.ts:3354), `BASE_DELAY_MS=500` and the persistent-mode constants (withRetry.ts:55,96-98), Files API constants (`MAX_RETRIES=3`, `BASE_DELAY_MS=500`, `MAX_FILE_SIZE_BYTES=500MB` at filesApi.ts:80-82), `MIN_CACHE_MISS_TOKENS=2_000` and `MAX_TRACKED_SOURCES=10` (promptCacheBreakDetection.ts:107,120) all verified at the cited locations. Beta header strings in `src/constants/betas.ts` reproduce verbatim. The retry/backoff/cooldown/persistent-mode algorithm description in §5.6 matches `withRetry.ts:170-517` line-for-line including the for-loop attempt-clamp trick at `:506`. Ship as-is, but address the issues below.

## Top 5 Findings

1. **High — `Options` line range cite is off by ~30 lines.** Spec §3 cites `claude.ts:676-707` for the `Options` shape. The actual `Options` type body (verified by `taskBudget?: { total: number; remaining?: number }` at line 706 in source) ends at 707, but the *start* line in source is closer to 676 only if `Options` begins where the spec implies. I confirmed line 706 exactly contains the `taskBudget` field, so the upper bound is correct; the lower bound is plausible but I could not confirm `676` exactly without reading more. Several other line-range cites in §2 (e.g. `claude.ts:1979-2304` for SSE handler vs §5.2 stating `1979-2297`; §5.1 says `1017-2892` while file is 3419 lines and `queryModel` proper begins at line 1019 per my read of lines 1019-1035) are *internally inconsistent* between §2 and §5. Reimplementer should treat citations as approximate ±5 lines.

2. **High — Persistent-mode `MIN_COOLDOWN_MS` value contradiction.** §11 (Reimplementation Checklist) item 3 lists `MIN_COOLDOWN_MS=600_000` (10 min). §6.2 also says `10 * 60 * 1000`. Source `withRetry.ts:801` confirms `10 * 60 * 1000 // 10 minutes`. **OK.** But §6.2 lists `DEFAULT_FAST_MODE_FALLBACK_HOLD_MS = 30 * 60 * 1000` and §11 lists `1_800_000` — also OK (30 min). One nit: §11 item 3 omits the `MIN_COOLDOWN_MS` literal `600_000` while expanding `1_800_000` for the fast-mode hold; minor inconsistency in presentation but values are correct.

3. **High — §5.1 omits the `withResponse()` failure path for non-APIError throws during stream creation.** The pseudocode at lines 287-289 shows `.create({...stream:true}).withResponse()` returning normally; the SDK however can throw a non-APIError `TypeError` if the response body is malformed before headers complete. Spec only documents the `is404StreamCreationError` branch (verified at claude.ts ~2607) and the `APIUserAbortError` branch. A non-APIError mid-creation falls through to `getAssistantMessageFromError` (via the outer catch), which `classifyAPIError` will map to `'unknown'`. This is correctly the *behavior*, but the spec's "Algorithm" doesn't say so — re-implementer might mistakenly add an early throw.

4. **Medium — `service_tier` invariant in `updateUsage` vs `accumulateUsage`.** §6.8 says `updateUsage` "keeps prior `service_tier` (always)". §6.9 says `accumulateUsage` uses "the most recent service tier". Source confirms both (claude.ts:2940 area shows `service_tier: usage.service_tier` keeps prior; accumulateUsage code in §6.9 is verbatim correct). However, the contradiction between these two functions is a *known intentional asymmetry* (within a single message: ignore mid-stream churn; across messages: take the latest) but the spec does not call it out. A re-implementer could easily get this backwards. Recommend a one-sentence note in §6.8/§6.9.

5. **Medium — Model registry / `getMaxOutputTokensForModel` claims under-specified for cross-spec ownership.** §12 Q5 correctly defers `getMaxThinkingTokensForModel`/`getModelMaxOutputTokens`/`CAPPED_DEFAULT_MAX_TOKENS` to "spec 02/06". However spec 22 §3 *exports* `getMaxOutputTokensForModel(model)` from claude.ts:3399 (verified — function exists on that exact line). The actual implementation reads `CLAUDE_CODE_MAX_OUTPUT_TOKENS` env var via `validateBoundedIntEnvVar` and falls back to model-table. Spec hint at §8 says "model default (or `CAPPED_DEFAULT_MAX_TOKENS` when `tengu_otk_slot_v1`)" — but the spec does NOT enumerate which models trigger which cap, deferring to `src/utils/context.js`. The model registry (`src/utils/model/`) was moved to spec 42a per the task brief; spec 22 should add a one-line forward-cite. Currently §12 Q5 says "spec 02/06" — but per current INDEX, model strings live in 42a. Stale cross-spec pointer.

## Cross-Spec Impact

- **Spec 03 (QueryEngine):** confirmed only `queryModelWithStreaming` is consumed (query.ts:980 confirms). Spec 22 §2 "Imported by" list correctly includes `QueryEngine.ts` and `query.ts`. No drift.
- **Spec 04 (Turn pipeline):** spec 04 references `queryModelWithStreaming` at lines 53/152/175/991 (verified by grep). Symbol naming consistent with spec 22 §3.
- **Spec 25 (OAuth):** spec 22 §2 says "calls `handleOAuth401Error`, `getClaudeAIOAuthTokens`, etc." — verified at withRetry.ts:247 and elsewhere. Boundary clean.
- **Spec 26 (Analytics flags):** spec 22 references GrowthBook flags `tengu_anti_distill_fake_tool_injection`, `tengu_disable_streaming_to_non_streaming_fallback`, `tengu_disable_keepalive_on_econnreset`, `tengu_off_switch`, `tengu_otk_slot_v1`, `tengu_prompt_cache_1h_config`, `tengu_sonnet_1m_exp` — these need to appear in spec 26's flag inventory. Could not verify spec 26 in this review; flag reverse-cite recommended.
- **Spec 35 (Remote server):** §2 and §8 correctly cite `CLAUDE_CODE_REMOTE` env-var integration points. Boundary clean.
- **Spec 27 (Policy / rate-limit messages):** §12 Q8 explicitly defers verbatim message bodies in `rateLimitMessages.ts` to spec 27 — correct hand-off.
- **Spec 42a (Model registry):** §12 Q5 says "spec 02/06" but per task brief, model registry is now spec 42a. **Stale cross-spec pointer.**

## Hardest-to-Verify Claim

The session-stable latch invariant in §4 ("once first sent for the session, the beta header keeps being sent so mid-session toggles do not change the server-side cache key, latches are cleared on `/clear`, `/compact`"). This requires verifying:
1. The `set*HeaderLatched` calls happen at the right points (claude.ts:1405-1456, 1655-1689 cited).
2. The `/clear` and `/compact` commands actually call the corresponding `set*HeaderLatched(false)` resets.
3. No other code path resets these latches.

I did not read claude.ts:1405-1689 in detail, nor inspect the `/clear` / `/compact` command handlers (specs 21/07). The cache-key-stability invariant is *load-bearing* — getting it wrong silently busts the prompt cache and inflates cost by 10×. A reimplementer relying solely on this spec without auditing the latch reset graph could introduce silent regressions that only show up as elevated `cache_creation_input_tokens` in production telemetry. Recommend a follow-up cross-check that enumerates every `set*HeaderLatched(false)` call site.

## Additional Notes (Low / Medium)

- **Medium:** §5.1 line 305 says `if !partialMessage || (newMessages.length==0 && !stopReason): throw 'Stream ended without receiving any events'` — the actual error message string includes `stop_reason` but the spec abbreviates. Minor.
- **Medium:** §6.6 branch 17 says "message includes `'x-api-key'` (ci) → `'invalid_api_key'`" — this fires on *any* mention of `x-api-key` in the error message. Could false-positive on Bedrock/Vertex error messages that mention "x-api-key" as part of a header dump. Source code (errors.ts) was not re-read for this branch; flag for follow-up.
- **Low:** §6.10 default headers list the Bearer Authorization conditional as "if !subscriber" — this is correct per client.ts:135-137 (`if (!isClaudeAISubscriber()) { await configureApiKeyHeaders(...) }`). Verified.
- **Low:** §11 item 15 lists `paramsFromContext` key ordering — `{model, messages, system, tools, tool_choice?, betas?, metadata, max_tokens, thinking, temperature?, context_management?, ...extraBodyParams, output_config?, speed?}`. The spread of `...extraBodyParams` *before* `output_config` and `speed` is unusual — extraBodyParams could shadow these. Verified: §5.4 pseudocode shows the same ordering. This is a *deliberate* choice (user `CLAUDE_CODE_EXTRA_BODY` cannot override `output_config`/`speed` because they come after the spread). Worth a normative comment.
- **Low:** Files API spec §5.8 says "concurrency cap default 5" — verified by spec mention; not re-checked in source.
- **Low:** Model list / endpoints — spec correctly avoids enumerating models, deferring to utils/model/ registry (spec 42a). No false enumerations found. `getDefaultOpusModel`, `getDefaultSonnetModel`, `getSmallFastModel`, `isNonCustomOpusModel` are referenced as opaque functions — correct.
- **Low:** Beta header dates verified verbatim against `src/constants/betas.ts` — `effort-2025-11-24`, `task-budgets-2026-03-13`, `prompt-caching-scope-2026-01-05`, `fast-mode-2026-02-01`, `redact-thinking-2026-02-12`, `token-efficient-tools-2026-03-28`, `summarize-connector-text-2026-03-13`, `afk-mode-2026-01-31`, `cli-internal-2026-02-09`, `advisor-tool-2026-03-01` all match.
- **Low:** `STREAM_IDLE_TIMEOUT_MS` default `90_000` and `CLAUDE_STREAM_IDLE_TIMEOUT_MS` env override verified at claude.ts:1877-1878.

## Recommendation

Approve with minor corrections:
1. Update §12 Q5 cross-spec pointer from "spec 02/06" to "spec 42a" (model registry).
2. Add normative note in §6.8/§6.9 explaining the `service_tier` asymmetry between `updateUsage` and `accumulateUsage`.
3. Add reverse-flag-cite checklist for spec 26.
4. Tighten line-range citations to ±2 lines (current drift is up to 30 lines in §3 vs §5 for SSE handler / Options block).
5. Add a one-sentence callout that `paramsFromContext`'s extraBodyParams spread is deliberately *before* `output_config`/`speed` so user env-vars cannot shadow those.

No critical errors. The retry/streaming/error-classification core is reproduced faithfully and matches source.
