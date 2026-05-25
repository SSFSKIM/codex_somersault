# Phase 9.6c Fix Log — Spec 03 (Query Engine)

Spec: `docs/specs/03-query-engine.md`
Adversarial review: `docs/specs/PHASE9-ADVERSARIAL-03.md`
Date: 2026-05-10

## Findings applied

### F1 (Major) — §2.5 self-contradiction on absent sources — APPLIED

**Adversarial claim:** §2.5 said "None directly owned by this spec" while §2.2 cites `snipCompact.ts` and `snipProjection.ts` as backing modules and §12.10 admits the projection module was not read.

**Source verification:**
- `find /Users/new/Downloads/claude-code-main/src/services/compact -name 'snipCompact*' -o -name 'snipProjection*'` — returns nothing.
- `ls src/services/compact/` — yields `apiMicrocompact.ts, autoCompact.ts, compact.ts, compactWarningHook.ts, compactWarningState.ts, grouping.ts, microCompact.ts, postCompactCleanup.ts, prompt.ts, sessionMemoryCompact.ts, timeBasedMCConfig.ts`. Both snip files confirmed absent.
- `QueryEngine.ts:122-127` imports both via `feature('HISTORY_SNIP')`-gated `require()` (verified in spec §2.2 lines 109-110).

**Fix:** Rewrote §2.5 to explicitly enumerate `snipCompact.ts` and `snipProjection.ts` as absent from the leaked tree, note the `feature('HISTORY_SNIP')` gate explains why DCE strips the `require()`s and the absence doesn't block the build, and cross-reference §12.10. Kept the existing `MessageSelector` note. Severity self-contradiction resolved.

### F2 (Major) — §3.4 `tool_progress` envelope cite tightened — APPLIED

**Adversarial claim:** Cite was `queryHelpers.ts:157-199, especially :190` with the gating-predicate claim asserted without source backing.

**Source verification (read `:100-219`):**
- `normalizeMessage` declared at `:102` (not `:157` — `:157` is the start of the `bash_progress`/`powershell_progress` branch within the `progress` case).
- `type: 'tool_progress'` yield occupies `:189-200` (single string match for `'tool_progress'` at `:190`, full envelope spans `:189-200`).
- Gating predicates verified at `:163-168`: requires `isEnvTruthy(process.env.CLAUDE_CODE_REMOTE)` OR `process.env.CLAUDE_CODE_CONTAINER_ID`. Throttle constant `TOOL_PROGRESS_THROTTLE_MS` enforced at `:177`. LRU eviction at `:178-186`. Branch closes at `:201`.

**Fix:** Replaced the loose cite with verified ranges and named the actual gating env vars (`CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_CONTAINER_ID`) plus the throttle and LRU mechanics. Also updated §2.6 inventory: `queryHelpers.ts` upgraded from `grep-inspected` to `partially-read` for the verified ranges (`:56-68`, `:102-219`).

### F3 / Pattern A2 ripple — verified clean, no edit needed

**Check:** Spec 03 §1.1.2 (line 16) reads *"QueryEngine is the **consumer** of stream events; the streaming/retry mechanics live in `services/api/claude.ts` (spec 22)."* — consumer-not-implementer framing intact.

**Source verification:**
- `grep -n queryModelWithStreaming src/QueryEngine.ts` — zero hits.
- `grep -n queryModelWithStreaming src/services/api/claude.ts` — exactly one hit at `:752` (`export async function* queryModelWithStreaming(`). Matches Phase 9.6 spec 04 fix placement.

No edit applied; spec already correct.

### F12 — verified clean, no edit needed

Spec 03 contains zero `bubble runtime` or `type-only` claims. Restraint correctly maintained per Phase 9.6 inversion #6.

## Findings skipped

- **F4–F11 (Minor)** — out of scope per Phase 9.6c instructions ("Skip cosmetic"). All were verified clean by the adversarial reviewer with verbatim line matches; no contradictions to resolve.
- **F-cosmetic** — none applied per instructions.

## Top 3 fixes summary

1. **§2.5 inventory honesty** — `snipCompact.ts` / `snipProjection.ts` now explicitly listed as absent with verified `find`/`ls` evidence; HISTORY_SNIP gate explained.
2. **§3.4 `tool_progress` provenance tightened** — actual line ranges (`:102` decl, `:157-201` branch, `:189-200` yield), real gating env-vars (`CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_CONTAINER_ID`), and the 30s throttle + LRU mechanics now cited from source.
3. **§2.6 inventory upgrade** — `queryHelpers.ts` reclassified `grep-inspected` → `partially-read` reflecting the new verified read range, eliminating the gap that motivated F2.

## Cross-spec ripple

- **Spec 07 (compaction / snip)** must own canonical definition of `snipCompactIfNeeded` and `isSnipBoundaryMessage`; spec 03 now correctly defers. No additional ripple needed.
- **Spec 22 (anthropic client)** Phase 9.6 cross-check on `queryModelWithStreaming` ownership — verified clean here, no further action.
