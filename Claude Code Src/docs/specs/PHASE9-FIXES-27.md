# Phase 9.6c Fix Log — Spec 27 (Service: Policy Limits, Remote-Managed Settings, Settings Sync)

**Source review:** `docs/specs/PHASE9-ADVERSARIAL-27.md` (Phase 9.5b adversarial findings).
**Severity context:** SECURITY-CRITICAL spec — F1 governs the moment policy-blocked features stop being permitted.
**Verification mode:** VERIFY-BEFORE-EDIT. All findings re-checked against `src/` before editing the spec.

## Summary

| ID | Severity | Status | Action |
|---|---|---|---|
| F1 | HIGH | Fixed | Spec §11 reimplementation checklist now explicitly enumerates BOTH `resetSettingsCache` paths (sync getter at `syncCacheState.ts:92` direct; async fan-out via `notifyChange('policySettings')`) and explains why both are required. |
| F2 | MED | Fixed | §5.3 step 6 now flags the `index.ts:341-344` 404 catch-branch as **dead defensive code** (validateStatus already includes 404), with a note on the divergent `checksum: ''` vs `undefined` for source-compat preservation. |
| F3 | MED | Fixed (arbitration: rejected handoff) | §1 OUT-of-scope block now arbitrates ownership: `claudeAiLimits.ts`, `claudeAiLimitsHook.ts`, `rateLimitMessages.ts`, `rateLimitMocking.ts`, `mockRateLimits.ts` remain owned by **spec 22**. Rationale documented inline (no `policySettings`/`policyLimits` dep; orthogonal to enterprise control planes). |
| F4 | MED | Fixed | §1 OUT-of-scope and §11 cross-link now name `BashTool.tsx:343`'s `sandbox.excludedCommands` read as a consumer of this spec's `policySettings` cascade resolver. Spec 10 owns the consumer, spec 27 owns the resolver. |
| F5 | LOW | Fixed | §1 (new "Plugin policy chokepoint cross-link" paragraph) and §11 cross-link enumerate `utils/plugins/pluginPolicy.ts:isPluginBlockedByPolicy` as the single chokepoint over `policySettings.enabledPlugins[id]` for the ~10 plugin call sites. Spec 28 owns the wrapper. |
| §5.4 | (was nit) | Fixed + cataloged | §5.4 now classifies the `localeCompare` vs default-sort divergence as a **confirmed source bug** in `policyLimits/index.ts:139-141` (Python `sort_keys=True` is code-point, matches default `.sort()`, NOT `localeCompare`). Cross-referenced from §12 (open question now resolved). New entry **#7** added to `BUGS-IN-SOURCE.md` with severity `minor`, suggested fix, reproduction, and consequence (forced-cache-miss + bandwidth churn for non-ASCII keys; no security impact). |

## Verification record

- F1: `syncCacheState.ts:92` — direct `resetSettingsCache()` import-and-call confirmed; gated by `eligible !== true` early return + `if (sessionCache)` early return so fires exactly once per cold cache load. Comment at `:84-86` self-attests the async arm uses its own reset path. `index.ts` confirmed to call `notifyChange('policySettings')` (multiple sites) which fans out via `changeDetector` listeners. The two paths are architecturally distinct and must both be preserved.
- F2: `index.ts:283-284` `validateStatus: status => status === 200 || status === 204 || status === 304 || status === 404` — 404 is allowed without throwing. `index.ts:299-306` handles 204/404 in the success path returning `checksum: undefined`. `index.ts:341-344` catch-branch handles 404 with `checksum: ''` — dead, but textually preserves a guard against future `validateStatus` edits.
- F3: `services/claudeAiLimits.ts` (515 lines), `services/rateLimitMessages.ts` (344 lines), `services/claudeAiLimitsHook.ts`, `services/rateLimitMocking.ts`, `services/mockRateLimits.ts` confirmed present. None import any `policy*` or `remoteManaged*` symbol — orthogonal subsystems.
- F4: `BashTool.tsx:343` — `getSettings().policySettings.sandbox?.excludedCommands` read confirmed as a consumer of the `policySettings` resolver this spec OWNS. Distinct from `tengu_sandbox_disabled_commands` (GrowthBook payload at `shouldUseSandbox.ts:27`, owned by spec 26 / spec 10).
- F5: `utils/plugins/pluginPolicy.ts:isPluginBlockedByPolicy` confirmed as 3-line wrapper over `getSettingsForSource('policySettings').enabledPlugins[id] === false`. Spec 28 ownership confirmed.
- §5.4 checksum bug: `policyLimits/index.ts:139-141` `Object.entries(obj).sort(([a],[b]) => a.localeCompare(b))` — `localeCompare` confirmed locale-sensitive (diverges from code-point sort for non-ASCII). `remoteManagedSettings/index.ts:118` `Object.keys(obj).sort()` — confirmed default lexicographic / code-point sort, matches Python `sort_keys=True`. Bug confirmed in `policyLimits`; remoteManagedSettings is correct.

## Files modified

- `docs/specs/27-service-policy.md` — F1 (§11 expanded resetSettingsCache lifecycle), F2 (§5.3 step 6 dead-code note), F3 (§1 OUT-of-scope arbitration), F4 (§1 + §11 cross-link), F5 (§1 + §11 cross-link), §5.4 (confirmed bug classification), §12 (open question resolved).
- `docs/specs/BUGS-IN-SOURCE.md` — count updated 6→7, entry **#7** added for `policyLimits` checksum `localeCompare` bug.

## Cross-spec follow-ups (not actioned here)

- Phase 9.5 §F1 hardest-to-verify claim about `notifyChange` listener-ordering — flagged in spec but not investigated (`changeDetector.ts` listener fan-out order vs `applySettingsChange.ts`). Defer to Phase 9.7 / a `changeDetector` audit.
- Spec 22 should not be edited by this fix agent; the F3 arbitration is unilateral on the spec-27 side ("we keep refusing the handoff") and consistent with spec 22's prior refusal — net effect: ownership stays with spec 22.

## Out of scope (intentionally not changed)

- F3-related: enumerating verbatim user-facing rate-limit strings — those belong in spec 22 §6.x, not spec 27.
- §6.6 nit (LOADING_PROMISE_TIMEOUT_MS appears at two source locations) — editorial, not a fix-cycle item.
- §3.4 nit (`isInProtectedNamespace` body) — already correct.
- The "other findings" §5.3 retry-burn observation (malformed remote response burns 6 attempts) — informational; no code-side change recommended; spec already documents `skipRetry:true` only on auth.
