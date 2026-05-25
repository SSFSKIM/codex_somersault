# Phase 9.5b Adversarial Review — Spec 18 (Tool Modes)

**Reviewer role:** Skeptic. **Source:** READ-ONLY src/. **Spec under review:** `docs/specs/18-tool-modes.md` (1132 lines).

## Severity Counts

- **Critical:** 0
- **High:** 0
- **Medium:** 2
- **Low:** 4
- **Nit:** 3

## Top 5 Findings

### 1. [Medium] §1 mis-states channels-gate scope vs §5.9
Spec §1 line 13: "Plan-mode tools are always registered (gated only by `--channels`)." This is correct in spirit but the table at §5.9 + §6.10 shows the gate is `(feature('KAIROS') || feature('KAIROS_CHANNELS')) && getAllowedChannels().length > 0` — i.e. requires *both* the KAIROS/KAIROS_CHANNELS feature flag AND a non-empty allowed-channels list. The phrase "gated only by `--channels`" understates the feature-flag dependency: under stripped builds (no KAIROS), `--channels` has no effect and plan tools remain enabled. Recommend rewording to "gated only by KAIROS+channels" for cross-spec consistency with 09 and the Phase 9.6 ripple.

### 2. [Medium] §5.4 "auto-name reuses session's plan slug" hides a side effect
Spec §5.4 step 3 says `slug = input.name ?? getPlanSlug()`. Verified at `EnterWorktreeTool.ts:90`. However, `getPlanSlug` (`plans.ts:32-49`) is *not pure*: on cache miss it generates a random slug, retries up to 10× to avoid `<plansDir>/<slug>.md` collisions, and *writes the slug into `getPlanSlugCache()` for the session* (`plans.ts:55`). The spec's §3 glossary mentions "lazily per session" but §5.4 step 3 should call out that calling EnterWorktreeTool without `name` *commits the session's plan slug*, which then leaks into all subsequent plan-file paths for the session. Cross-spec impact: 04/05 (plan attachment) and 41 (resume).

### 3. [Low] §5.5 step 2 contains a load-bearing semantic claim that needs a 09 cross-link
The comment at `ExitWorktreeTool.ts:244-249` says `getProjectRoot() === getOriginalCwd()` is the discriminator between `--worktree` startup and mid-session. This relies on `setup.ts:235/239` setting both back-to-back (cited in code comment, owned by spec 01). Spec §5.5 quotes the discriminator but only cites it as "see ExitWorktree §5.5" — the actual setup.ts line numbers are NOT verified in this spec. If 01 ever changes the realpath/setOriginalCwd ordering, this discriminator silently breaks (mid-session ExitWorktree would clobber projectRoot). Recommend explicit cross-spec contract note.

### 4. [Low] §5.7 deferred-tool claim verified, but spec misses the agentId-gate interaction with bubble forked subagents
EnterPlanMode rejects when `context.agentId` is set (`EnterPlanModeTool.ts:78`, spec §5.1 #1). Spec correctly notes "Subagents must not toggle the parent's mode." However, this couples to spec 09's "bubble forked subagents" mechanism (Phase 9.6 ripple): when a forked subagent is in scope, the deferred-tool list still announces EnterPlanMode regardless of agentId. The spec should explicitly note the bubble-up failure mode: a forked subagent calling EnterPlanMode throws, and the throw propagates as a normal tool error rather than as a permission decline. Cross-spec impact for 09.

### 5. [Low] §3 "Worktree tools are registered iff `isWorktreeModeEnabled()` returns true" but spec doesn't enumerate the failure mode
`isWorktreeModeEnabled()` is unconditionally `true` (`worktreeModeEnabled.ts:9`). The spec calls this out but does not document what happens if a downstream caller stubs it false (e.g. tests, alternate builds): `tools.ts:225` would emit `[]`, and any model attempt to call EnterWorktree/ExitWorktree would surface as "tool not found" rather than a clean disabled-state error. Recommend §8 row.

## Verdict

**APPROVED with minor revisions.** The spec is unusually high-fidelity. Every line-range citation I sampled matched source exactly:

- `EnterPlanModeTool.ts:77-94` (guards) ✓
- `ExitPlanModeV2Tool.ts:51-58` (TRANSCRIPT_CLASSIFIER requires) ✓
- `ExitPlanModeV2Tool.ts:202-218` (validateInput mode-check + errorCode:1) ✓
- `ExitPlanModeV2Tool.ts:357-403` (mode restore branch) ✓ (exact lines)
- `EnterWorktreeTool.ts:77-118` (call body) ✓
- `ExitWorktreeTool.ts:79-113, 174-224, 227-321` ✓
- `worktree.ts:48-49, 199-227` (slug regex, GIT_NO_PROMPT_ENV, flattenSlug, branch/path) ✓
- `permissionSetup.ts:1462-1493` (`prepareContextForPlanMode`), `:1502-1532` (`transitionPlanAutoMode`) ✓
- `bootstrap/state.ts:1349-1363` (`handlePlanModeTransition`) ✓
- `tools.ts:202, 213, 225` (registration sites) ✓
- `worktreeModeEnabled.ts:9-11` (unconditional true) ✓
- Channels gate predicate verbatim match at both sites ✓
- ExitPlanMode tool-result content variants (awaiting/agent/empty/default) verbatim ✓

Verbatim §6 strings (validateInput refusals, errorCodes 1/2/3, mailbox envelope, gate-fallback notification fields, analytics event names) all match source exactly.

## Cross-Spec Impact

- **09 (permissions):** Spec correctly defers `prepareContextForPlanMode` / `transitionPlanAutoMode` / `shouldPlanUseAutoMode` ownership to 09. The §6.5 verbatim quotes are accurate. **Bubble forked subagents** (Phase 9.6): see Finding #4 — agentId-throw needs cross-spec note in 09.
- **30 (coordinator/teammate plan approval):** Spec correctly defers `isTeammate()` / `isPlanModeRequired()` / `writeToMailbox` ownership. The plan_approval_request envelope (§6.6) is the contract surface; 30 must keep this stable.
- **08 (tool registry):** §6.12 ANT-only-import warning is correct; do not reorder.
- **04/05 (plan-mode attachment):** `getPlanSlug` side-effect (Finding #2) cross-cuts.
- **41 (session state):** `saveWorktreeState` / `restoreWorktreeSession` deferred correctly.
- **01 (--worktree startup):** `projectRootIsWorktree` discriminator depends on 01's ordering invariant (Finding #3).

## Hardest-to-Verify Claim

**§5.2 #4 fourth bullet:** *"Read `autoModeStateModule.isAutoModeActive()` BEFORE calling `setAutoModeActive(restoringToAuto)` — this is the authoritative signal because `prePlanMode`/`strippedDangerousRules` are stale after `transitionPlanAutoMode` deactivates mid-plan."*

This claim entangles three independently-mutating module-level state stores (`autoModeStateModule`, `toolPermissionContext.prePlanMode`, `toolPermissionContext.strippedDangerousRules`) with a non-obvious temporal ordering invariant. Verifying it requires:
1. Reading `applySettingsChange` (which calls `transitionPlanAutoMode`) — owned by 09.
2. Confirming that mid-plan deactivation does NOT clear `prePlanMode='auto'` or unset `strippedDangerousRules` (they persist until `ExitPlanMode` runs).
3. Confirming `autoModeStateModule.isAutoModeActive()` is the *only* state store that `transitionPlanAutoMode` actually writes through to (`autoModeState.js`).

The code at `ExitPlanModeV2Tool.ts:362-378` is consistent with the claim, and the open question in §12 #1 acknowledges this. But the claim is *load-bearing* for circuit-breaker correctness (without it, `setAutoModeActive(true)` would bypass the disabled gate). Spec 18 cannot fully verify this without 09's verification of `transitionPlanAutoMode`'s state-write boundary. **This should be flagged as a Phase 9.6 cross-spec contract test.**
