# Phase 9.6c Fix Log — Spec 07 (Context Compaction)

**Date:** 2026-05-10 · **Source review:** PHASE9-ADVERSARIAL-07.md
**Verification policy:** all 5 changes verified against source before edit.

---

## Severity Summary

| Severity | Finding | Resolution |
|---|---|---|
| HIGH    | §5.13 missed Phase 9.6 spec-05 ripple — subagent compacts skip `getUserContext.cache.clear` | **FIXED** — added explicit cross-spec note |
| MEDIUM  | §5.5 step 11 attachment-ordering ambiguity (planAttachment appears first in bullet list) | **FIXED** — explicit ordered list with `fileAttachments` + `asyncAgentAttachments` first |
| MEDIUM  | Cost-multiplier surface (Phase 9.7 §13.1) not consolidated | **FIXED** — added §7.1 "Cost surface" subsection |
| MEDIUM  | §5.9 step 4 understates that time-based MC still fires for external builds | **FIXED** — clarifying note appended to step 4 |
| LOW     | §3.2 `snipCompactIfNeeded` — no spec 41 cross-ref | **FIXED** — added inline cross-ref + INFERRED marker |
| —       | Verify queryHaiku NOT used in `services/compact/` | **VERIFIED + DOCUMENTED** in §7.1 (cross-ref to spec 22) |

---

## Source Verifications Performed

1. **`compact.ts:520-590`** — confirmed attachment ordering: `fileAttachments` and `asyncAgentAttachments` from `Promise.all` (line 532-539) are spread into `postCompactFileAttachments` BEFORE `planAttachment` (line 545). Spec §5.5 step 11 corrected.

2. **`postCompactCleanup.ts:31-77`** — confirmed `isMainThreadCompact` gate at lines 36-39 (`querySource === undefined || startsWith('repl_main_thread') || === 'sdk'`). `getUserContext.cache.clear?.()` and `resetGetMemoryFilesCache('compact')` at lines 59-60 are gated; subagent (`agent:*`) sources skip both. Cross-spec note added to §5.13.

3. **`compact.ts:431-438, 1154-1158`** — confirmed `tengu_compact_cache_prefix` 3P default `true`; comment at lines 433-434 cites "0.76% of fleet cache_creation (~38B tok/day, concentrated in ephemeral envs CCR/GHA/SDK)". Captured in §7.1.

4. **`compact.ts:524-529` + `postCompactCleanup.ts:65-69`** — confirmed `sentSkillNames` is intentionally NOT reset; rationale ("~4K tokens of pure cache_creation"). Captured in §7.1.

5. **`compact.ts:698-704, 1047-1053` + `microCompact.ts:362-367, 525-527`** — confirmed `notifyCompaction()` / `markPostCompaction()` exist to suppress spec 03 cache-break false positives. Captured in §7.1.

6. **`grep -rn "queryHaiku" src/services/compact/`** → 0 matches. **`grep "mainLoopModel"`** → 7 matches (compact.ts:569, 581, 593, 959, 971, 982, 1266, 1313, 1319). Confirmed in §7.1 with spec 22 cross-ref.

---

## Edits Applied

| § | File:lines | Change |
|---|---|---|
| 3.2 | line ~467-475 | Added cross-ref to spec 41 + INFERRED marker on `boundaryMessage?` |
| 5.5 step 11 | line ~750-765 | Replaced "Push (in this order): planAttachment ..." with explicit ordered array starting `[...fileAttachments, ...asyncAgentAttachments, planAttachment?, ...]` |
| 5.9 step 4 | line ~995 | Appended note: time-based MC may still have fired in step 2; "no compaction here" applies only to cached MC path |
| 5.13 | line ~1172 | Appended **"Cross-spec ripple — subagent compact ≠ main-thread compact"** subsection |
| 7.1 (new) | line ~1781 | Added "Cost surface (Phase 9.7 §13.1 ripple)" subsection: `sentSkillNames`, `tengu_compact_cache_prefix`, `notifyCompaction`/`markPostCompaction`, telemetry, mainLoopModel/no-Haiku |

---

## Cross-Spec Action Items Generated

- **spec 05** should reciprocally cite postCompactCleanup.ts:36-39 gate (subagent compacts leave main-thread `getUserContext` cache stale).
- **spec 22** should explicitly note compaction is `mainLoopModel`-only (zero `queryHaiku` calls in `src/services/compact/`).
- **spec 41** owns the unverified `snipCompactIfNeeded` signature; spec 07 now defers explicitly.
- **spec 03** already reciprocally cites the `notifyCompaction` plumbing; no further action.
- **Phase 9.7 §13.1** cost-multiplier audit can now reference spec 07 §7.1 directly.

---

## Verdict

All 5 adversarial findings resolved. Spec 07 remains APPROVED; revisions
are clarifications/cross-refs, not corrections. No new factual claims
introduced — all additions trace to verified source line ranges.
