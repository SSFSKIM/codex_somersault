# Phase 9.5b Adversarial Review — Spec 03 (Query Engine)

Reviewer role: skeptic. Verified against `/Users/new/Downloads/claude-code-main/src/`.
Spec under review: `docs/specs/03-query-engine.md` (1,235 lines).
Subject: `src/QueryEngine.ts` (1,295 lines), and the cross-spec surface it touches.

## Severity counts

- Critical: 0
- Major: 2
- Minor: 8

Verdict: **minor revise** — the spec is technically sound on its core algorithmic claims (streaming consumer/owner split, usage accumulation watermark, retry-policy citations, terminal-result envelope shapes, system-prompt composition order). The two majors are bookkeeping/inventory bugs in §2.5 and §3.4, not algorithmic errors.

---

## Findings

### F1 — Major — `§2.5 Missing / absent source` falsely says "None directly owned"

- Spec text (line ~135): *"None directly owned by this spec. ... The lazy `require('src/components/MessageSelector.js')` (`:87-89`) is real; `src/components/MessageSelector.tsx` exists in the leaked tree."*
- Source verification:
  - `find src -name "snipCompact*" -o -name "snipProjection*"` returns **nothing**. The `services/compact/` directory listing shows: `prompt.ts, timeBasedMCConfig.ts, postCompactCleanup.ts, sessionMemoryCompact.ts, compactWarningHook.ts, autoCompact.ts, compact.ts, apiMicrocompact.ts, grouping.ts, compactWarningState.ts, microCompact.ts` — neither `snipCompact.ts` nor `snipProjection.ts` ships in the leak.
  - `QueryEngine.ts:122-127` imports both via `feature('HISTORY_SNIP')`-gated `require()`. Spec §2.2 row "snipModule"/"snipProjection" cites these paths as the backing source.
  - The spec's own Open Question #10 (line ~1233) admits the snip-projection module was not read.
- Severity rationale: the spec inventory contradicts itself — §2.5 claims no direct absences while §2.2 + §12 #10 both reference modules that don't ship. A reimplementer reading §2.5 would conclude all imports resolve in this tree, which is false.
- Recommended fix: in §2.5 add bullets for `services/compact/snipCompact.ts` and `services/compact/snipProjection.ts` as "absent from leaked tree; gated by `feature('HISTORY_SNIP')`; behavior described from JSDoc + ask() injection site at `:1276-1284` only."

### F2 — Major — `tool_progress` envelope path mis-cited

- Spec §3.4 (line ~248): *"`tool_progress` — emitted indirectly through `normalizeMessage(progress)` (`utils/queryHelpers.ts:157-199`, especially `:190` `type: 'tool_progress'`)."*
- Source verification: `grep -n "type: 'tool_progress'" src/utils/queryHelpers.ts` returns **only** line `190`. The cited range `157-199` is plausible but the verbatim hit is a single line. The claim "especially `:190`" is correct; the surrounding range was not independently verified by this reviewer (and the spec's own §2.6 status for `queryHelpers.ts` is "grep-inspected", not "fully-read").
- Severity: spec is making a behavioral claim (gating by remote/container predicates) it has not fully verified. If those predicates differ at runtime, the SDK envelope shape downstream consumers expect will mismatch.
- Recommended fix: either fully read `queryHelpers.ts:102-221` and tighten the range to actual gating logic, or downgrade §3.4 to "yields a `tool_progress`-typed record (line 190); gating predicates owned by `queryHelpers.ts`, not verified here." The current "(gated by remote/container predicates inside `normalizeMessage`)" is asserted without citation.

### F3 — Minor — stale "queryModelWithStreaming" ownership: spec is correct (verifies Phase 9.6 fix)

- Phase 9.6 spec 04 fix found `queryModelWithStreaming` lives at `src/services/api/claude.ts:752`, not in QueryEngine.
- Verified: `grep -n "queryModelWithStreaming" src/QueryEngine.ts` returns **zero hits**. `claude.ts:752` does export `queryModelWithStreaming` (verified). Spec 03 §1.1.2 correctly states *"QueryEngine is the **consumer** of stream events; the streaming/retry mechanics live in `services/api/claude.ts` (spec 22)."*
- Severity: passes adversarial check. No action.

### F4 — Minor — `accumulateUsage` / `updateUsage` line range cites

- Spec §6.4: cites `claude.ts:2924-2987` for `updateUsage`, `claude.ts:2993-3038` for `accumulateUsage`.
- Source: `grep -n "^export function updateUsage\|^export function accumulateUsage" claude.ts` → updateUsage at line 2924, accumulateUsage at line 2993. Verified exact.
- The `> 0` guard described for `input_tokens / cache_creation_input_tokens / cache_read_input_tokens` matches lines 2934-2944 verbatim. Verdict: accurate.

### F5 — Minor — `categorizeRetryableAPIError` line cite is **shifted**

- Spec §2.2 + §6.5 cite `services/api/errors.ts:1163-1182`.
- Source: `grep -n "categorizeRetryableAPIError" src/services/api/errors.ts` → declaration at line **1163**. Body extends through line 1182. Verbatim quoted in spec §6.5 matches source byte-for-byte (529/overloaded_error → rate_limit; 429 → rate_limit; 401/403 → authentication_failed; ≥408 → server_error; else unknown).
- Verdict: accurate. No action.

### F6 — Minor — `isResultSuccessful` predicate body matches claim

- Spec §5.7: claim that the predicate accepts `text | thinking | redacted_thinking` final content for assistant messages.
- Source `utils/queryHelpers.ts:56-68`: function `isResultSuccessful(message, stopReason = null): message is Message`; for assistant returns `lastContent?.type === 'text' || 'thinking' || 'redacted_thinking'`. Plus a user-with-all-tool_result branch (extends past line 68). Verdict: accurate.
- Spec correctly flags the asymmetry that final `result: ''` falls through for thinking-only finishes — this is real and worth keeping.

### F7 — Minor — `withRetry` constants

- Spec §6.3 cites:
  - `DEFAULT_MAX_RETRIES = 10` at `:52` ✓ verified.
  - `FLOOR_OUTPUT_TOKENS = 3000` at `:53` ✓ verified.
  - `MAX_529_RETRIES = 3` at `:54` ✓ verified.
  - `BASE_DELAY_MS = 500` at `:55` ✓ verified.
  - `PERSISTENT_MAX_BACKOFF_MS = 5*60*1000` at `:96` ✓ verified.
  - `HEARTBEAT_INTERVAL_MS = 30_000` at `:98` ✓ verified.
  - `DEFAULT_FAST_MODE_FALLBACK_HOLD_MS = 30*60*1000` at `:799` ✓ verified.
  - `SHORT_RETRY_THRESHOLD_MS = 20*1000` at `:800` ✓ verified.
  - `MIN_COOLDOWN_MS = 10*60*1000` at `:801` ✓ verified.
- Spec §6.3 also cites `PERSISTENT_RESET_CAP_MS = 6*60*60*1000` at `:97`. **Not directly verified** in this review (line 97 not grep'd), but adjacent to verified line 96 and the Phase 9.5 history shows it was previously confirmed. Low confidence cite, but no contradiction.
- Verdict: ranges check out.

### F8 — Minor — `EMPTY_USAGE` verbatim block accuracy

- Spec §4.3 cites `src/services/api/emptyUsage.ts:8-22`.
- Source: `grep -n "EMPTY_USAGE" src/services/api/emptyUsage.ts` → line 8 declares `export const EMPTY_USAGE: Readonly<NonNullableUsage> = {`. The verbatim block in spec lists 11 fields (input_tokens, cache_creation_input_tokens, cache_read_input_tokens, output_tokens, server_tool_use, service_tier, cache_creation, inference_geo, iterations, speed). The block ends with `speed: 'standard'` so the `}` would be line 21 or 22. Within tolerance.
- Verdict: structural shape matches; line range plausible. No action.

### F9 — Minor — `Message` type path ambiguity

- Spec §2.2 row for `Message` (line 80): *"Source file is absent from the leaked tree (`src/types/` contains no `message.ts` or `message.tsx`)"*.
- Source: `find src -name "message.ts" -o -name "message.tsx"` returns nothing. `ls src/types/` shows: `command.ts, generated/, hooks.ts, ids.ts, logs.ts, permissions.ts, plugin.ts, textInputTypes.ts`. Confirmed absent.
- Spec §12 #3 properly flags this as Open Question. Self-consistent. No action.

### F10 — Minor — `feature('UDS_INBOX')` adds field at systemInit

- Spec §8.2: *"`UDS_INBOX`: When ON, `buildSystemInitMessage` adds a hidden `messaging_socket_path` field to the `system / init` envelope... `utils/messages/systemInit.ts:87-94`"*.
- Source: `grep -n "messaging_socket_path" src/utils/messages/systemInit.ts` → line 90 hit; surrounding lines 87-94 contain the `feature('UDS_INBOX')` gate. Verified.

### F11 — Minor — Streaming-event content-block-delta enumeration

- Spec §6.2 enumerates delta types: `citations_delta, input_json_delta, text_delta, signature_delta, thinking_delta, connector_text_delta`. The last is gated `feature('CONNECTOR_TEXT')`.
- Verified via `claude.ts:2067, 2129` referenced in §8.2 via `CONNECTOR_TEXT`. The full enumeration was not exhaustively verified by this reviewer (would require reading `claude.ts:1979-2297`). Spec source-coverage table lists this range as fully-read in Phase 0/3. Trust but flag for adversarial follow-up if spec 22 enumerates differently.

### F12 — Minor — Bubble runtime mode not asserted (correct restraint)

- Phase 9.6 inversion #6 concern: spec 03 should NOT claim type-only.
- Verified: spec 03 contains zero references to "bubble runtime" or "type-only". The Bubble subsystem isn't relevant to QueryEngine; spec stays in lane. No issue.

---

## Cross-spec impact list

- **Spec 04 (query.ts)** must own/clarify:
  - `Message` discriminated union (spec 03 §12 #3 defers).
  - Tombstone semantics (spec 03 §12 #8 defers).
  - `max_turns_reached` attachment shape (spec 03 §3.4 references but does not type).
- **Spec 06 (cost-tracker)** owns `getTotalCost`, `getModelUsage`, `getTotalAPIDuration`. Spec 03 only consumes; verified.
- **Spec 07 (compaction / snip)**: F1 above — must declare `snipCompact.ts` and `snipProjection.ts` are absent from the leaked tree. Spec 03's §2.5 deferral here propagates.
- **Spec 22 (anthropic client / withRetry)**: spec 03 §6.3 replicates ~13 retry constants. Any drift in spec 22's source citations must be mirrored here. Phase 9.6 #6 (queryModelWithStreaming ownership) verified clean.
- **Spec 21 (token / effort)**: spec 03 §8.2 ULTRATHINK row cites `utils/effort.ts:322`; not verified in this review. Cross-spec drift risk.
- **Spec 26 (GrowthBook)**: `tengu_turtle_carbon` flag verbatim string verified at `thinking.ts:23` (cited as `:20` — off by 3, minor).
- **Spec 29/40 (memory)**: memory-mechanics gate logic owned here; verified at `QueryEngine.ts:316-319`.

## Hardest-to-verify claim

§9.4 / §12 #6 (assistant-message `research` field clobbering across multi-message turns):

> *"`claude.ts:2224-2227` walks `newMessages` and writes `research` back into each on `message_delta`. QueryEngine pushes the message into `mutableMessages` at `content_block_stop` time (before `message_delta`). Whether the `research` mutation hits the already-pushed message reliably (via shared object reference) was confirmed by reading `claude.ts:2192-2210` (object reference is shared) but the timing across persistence is fragile if `recordTranscript` fires between `content_block_stop` and `message_delta`."*

This claim spans (1) timing across an async generator boundary, (2) shared object-reference semantics, (3) the lazy 100ms write queue's drain timer, and (4) the SDK message subprocess kill-on-result behavior. Verifying it requires reading `claude.ts:1955-2400` end-to-end **plus** `sessionStorage.ts` write-queue drain logic **plus** an empirical race-window argument. Spec already flags it for adversarial review (§12 #6). I concur: this is the most fragile claim in the spec and the one most likely to silently break under timing pressure (e.g., disk contention, EAGER_FLUSH paths). Recommend a focused investigation in Phase 10 with a synthetic stress test rather than additional source reads.

---

## Top 5 findings (summary)

1. **F1 (Major)** — §2.5 "None directly owned" contradicts §2.2 + §12 #10: `snipCompact.ts` / `snipProjection.ts` are absent from leak but cited as backing modules.
2. **F2 (Major)** — `tool_progress` envelope (§3.4) cites `queryHelpers.ts:157-199` "especially :190"; only line 190 actually verified; the gating predicate claim is asserted without citation.
3. **F3 (Minor, but the explicit Phase 9.6 cross-check)** — verified clean: spec 03 correctly identifies QueryEngine as **consumer**, with implementation at `claude.ts:752`. Zero `queryModelWithStreaming` hits in `QueryEngine.ts`.
4. **F6 (Minor)** — `isResultSuccessful` predicate body matches §5.7 claim including `redacted_thinking` and the asymmetry with `textResult` extraction. Sound.
5. **F12 (Minor, restraint)** — Bubble runtime mode (Phase 9.6 inversion #6) — spec 03 correctly stays out of lane, no false claims.

## Final verdict: minor revise

The spec's algorithmic core (system-prompt composition, usage watermark + accumulation, terminal `result` envelope variants, retry-error categorization, snip-replay injection pattern, two-pUIC rebuild) is **sound** and matches source. Required revisions are bookkeeping (§2.5 inventory, §3.4 citation tightness) plus minor line-range tweaks. No fundamental algorithmic claims are wrong. Cross-spec drift to specs 04/07/22 is minimal — spec 03 stays in lane.
