# PHASE 9.6c — Spec 18 Fix Log

Source: `docs/specs/PHASE9-ADVERSARIAL-18.md` (0 critical, 0 high, 2 medium, 4 low, 3 nit). Verdict was "APPROVED with minor revisions." Phase 9.6c targets the 2 medium and 3 of the low findings (F1, F2, F3, F4, F5).

## Coverage

All 5 priority findings applied. Nits not actioned (out of scope for 9.6c).

## Findings applied

### F1 — Medium · §1 channels-gate scope understated

**Original line (§1):** "Plan-mode tools are always registered (gated only by `--channels`)."

**Fix:** Replaced with full predicate `(feature('KAIROS') || feature('KAIROS_CHANNELS')) && getAllowedChannels().length > 0`, plus an explicit note that on stripped builds (no KAIROS feature flag) `--channels` has no effect and plan tools remain enabled. Cross-references §5.9, §6.10, §3, and §8 row added by F5.

**Evidence:** `EnterPlanModeTool.ts:56-67`, `ExitPlanModeV2Tool.ts:167-178` (verbatim predicate at both sites; spec §6.10 already had the exact text).

### F2 — Medium · §5.4 step 3 hidden side effect in `getPlanSlug()`

**Original line (§5.4 step 3):** "`slug = input.name ?? getPlanSlug()` — auto-name reuses the session's plan slug."

**Fix:** Added explicit "Side effect" callout: `getPlanSlug()` is not pure — on cache miss it generates a random word slug, retries up to 10× to avoid `<plansDir>/<slug>.md` collision, and writes the slug into `getPlanSlugCache()` for the session. So calling `EnterWorktreeTool` without `name` *materializes* the session's plan slug, leaking into all subsequent plan-file paths. Cross-spec impact noted for 04, 05, 41 (with `setPlanSlug` at `plans.ts:54-55` named as the resume-side reciprocal).

**Evidence (verified in source this pass):** `src/utils/plans.ts:32-49` — confirmed `getPlanSlugCache().set(id, slug!)` at `plans.ts:46`, retry loop at `plans.ts:39-45`. Adversarial review cited `plans.ts:55`, but the cache write is at `:46`; `:54-55` is `setPlanSlug` (the reciprocal). Fix uses the correct line numbers.

### F3 — Low · §5.5 projectRoot discriminator + stale line numbers

**Original line (§5.5 step 2):** Discriminator `getProjectRoot() === getOriginalCwd()` cited only as "see ExitWorktree §5.5"; §12 caveat #10 cited `setup.ts:235/239`.

**Verified:** `src/setup.ts` (NOT `src/bootstrap/setup.ts` — that path doesn't exist in this leak). The actual `--worktree` block setters are at:
- `:272` `setCwd(worktreeSession.worktreePath)`
- `:273` `setOriginalCwd(getCwd())`
- `:277` `setProjectRoot(getCwd())`

Lines 235/239 in the original adversarial finding (and in the inline comment at `ExitWorktreeTool.ts:244-248`) are **wrong** for this tree.

**Fix:** Pinned the contract by name (back-to-back `setOriginalCwd(getCwd()) … setProjectRoot(getCwd())` in the post-`createWorktreeForSession` block) rather than by line number. Updated both §5.5 step 2 and §12 caveat #10 with the corrected `setup.ts:272-277` citation and explicit note that the inline comment at `ExitWorktreeTool.ts:244-248` is stale.

### F4 — Low · §5.1 agentId throw + bubble forked subagents

**Fix:** Added explicit Phase 9.6 cross-link to spec 09 in §5.1 #1. The deferred-tool list announces `EnterPlanMode` regardless of `agentId` (registry-level, not context-aware), so a forked/"bubble" subagent can call it; the throw at `EnterPlanModeTool.ts:78` propagates as a normal tool error (no `checkPermissions` rejection, no permission UI). Spec 09's bubble-subagent contract must treat this as the documented failure mode.

**Evidence:** `EnterPlanModeTool.ts:77-78` (throw), §5.7 of this spec (deferred-tool announcement is mode-/context-blind), `ExitPlanModeV2Tool.ts:202-218` comment cited at §5.7 (analogous "deferred announces regardless").

### F5 — Low · §3/§8 isWorktreeModeEnabled stub failure mode

**Fix:** Added a new row to §8 Error Handling: when `isWorktreeModeEnabled()` is stubbed `false` (tests, alternate builds), `tools.ts:225` emits `[]`, and EnterWorktree/ExitWorktree calls surface as "tool not found" / `ToolNotFoundError` from the registry lookup — not as a clean disabled-state error. No user-facing "worktree mode is disabled" message exists. Currently moot in shipping builds (`worktreeModeEnabled.ts:9-11` is unconditional `true`). Also referenced from §1 as part of the F1 rewrite.

**Evidence:** `src/utils/worktreeModeEnabled.ts:9-11` (unconditional `true`), `src/tools.ts:225` (the spread-gate site).

## Top 5 fixes summary

| # | Finding | Location | Citation |
|---|---|---|---|
| 1 | F1 channels-gate scope | §1 | `EnterPlanModeTool.ts:56-67`, `ExitPlanModeV2Tool.ts:167-178` |
| 2 | F2 `getPlanSlug` side effect | §5.4 step 3 | `plans.ts:32-49` |
| 3 | F3 projectRoot discriminator + line-number correction | §5.5 step 2, §12 #10 | `src/setup.ts:272-277` (actual) vs `:235/:239` (stale) |
| 4 | F4 bubble forked subagent cross-link to 09 | §5.1 #1 | `EnterPlanModeTool.ts:77-78` |
| 5 | F5 worktree-disabled failure mode | §8 (new row) | `worktreeModeEnabled.ts:9-11`, `tools.ts:225` |

## Verify-before-edit notes

- The adversarial review cited `plans.ts:55` for the cache write; the actual cache write is at `:46`. The fix uses `:46` and notes `:54-55` as `setPlanSlug` (resume-side reciprocal). One source verification correction over what the adversarial review claimed.
- The adversarial review cited `setup.ts:235/239`; verified in this tree that `src/setup.ts` lines 272/273/277 are the actual setters. The fix re-pins by invariant ("back-to-back `setOriginalCwd → setProjectRoot`") and corrects both the §5.5 reference and the §12 #10 caveat that previously echoed the stale numbers.
- Confirmed `src/bootstrap/setup.ts` does not exist in this leak — only `src/setup.ts`.
- §6.10 already contained the verbatim KAIROS-channels predicate; F1 only fixed §1's understated paraphrase.

## Cross-spec ripple

- **01 (entrypoint/bootstrap):** F3 pins the back-to-back `setOriginalCwd → setProjectRoot` invariant in `--worktree` startup. If 01 reorders or interleaves `setCwd` between them, ExitWorktree's `projectRootIsWorktree` discriminator silently breaks.
- **04 / 05 / 41 (plan attachment, context, resume):** F2 documents that `EnterWorktreeTool` without `name` materializes the session plan slug. 41 owns `setPlanSlug` resume reciprocal.
- **09 (permissions / bubble forked subagents):** F4 documents the throw-vs-decline failure mode for forked subagents calling `EnterPlanMode`.

## Skipped / not-applied

- The 3 nits from the adversarial review.
- §12 open question #1 (`isAutoModeActive()` semantics) — left as-is per "hardest-to-verify claim" flag for cross-spec test in 09.

## Post-fix verdict

All 5 priority findings (2 medium + 3 low) applied with verify-before-edit. Spec 18 now correctly states the channels-gate scope, surfaces the `getPlanSlug` cache side effect, pins the projectRoot discriminator contract by invariant rather than stale line numbers, cross-links the bubble-subagent failure mode to spec 09, and documents the worktree-disabled silent-absence behavior.
