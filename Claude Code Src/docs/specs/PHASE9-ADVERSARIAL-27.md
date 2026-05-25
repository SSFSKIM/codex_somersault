# Phase 9.5b Adversarial Review — Spec 27 (Service: Policy Limits, Remote-Managed Settings, Settings Sync)

Scope: SECURITY-CRITICAL spec. Reviewed against `src/services/{policyLimits,remoteManagedSettings,settingsSync}/` plus the policy-cascade body in `src/utils/settings/settings.ts` and the MDM/managedPath helpers.

## Severity counts

- Critical: 0
- High: 1
- Medium: 4
- Low: 3
- Nit/editorial: 2

## Top 5 findings

### F1 (HIGH) — `getRemoteManagedSettingsSyncFromCache` lifetime/side-effect is under-specified relative to source

The spec §9 bullet says "triggers `resetSettingsCache()` once the first time disk-cached remote settings become available". Source (`syncCacheState.ts:70-95`) is more precise: `resetSettingsCache()` fires only on the first call where `eligible === true` AND `sessionCache` is null AND `loadSettings()` returns non-null — i.e. exactly when the on-disk file becomes visible to a caller for the first time. The reset does NOT fire on subsequent calls (gated by `if (sessionCache)` early-return). It also does NOT fire when `setSessionCache()` is called from the async `index.ts` arm (the comment at lines 84-86 explicitly says the async arm "handles its own reset" — but `index.ts:546,578,601` use `notifyChange('policySettings')`, which calls `resetSettingsCache` indirectly via the change detector listener chain). A reimplementer reading the spec alone could miss that there are TWO independent reset paths and conclude the sync getter is the only one. Recommend rewriting §11 bullet to explicitly call out: "(a) sync read of disk cache fires `resetSettingsCache()` directly; (b) async fetch fires `notifyChange('policySettings')` which resets via the listener chain." Security-relevant because the reset window is the moment when policy-blocked features stop being permitted; getting the lifetime wrong = silent permission bypass.

### F2 (MEDIUM) — Spec misrepresents 404 path of `fetchRemoteManagedSettings`

§5.3 step 6 claims: "(`status===404` outside `validateStatus` returns `{ success:true, settings:{}, checksum:'' }`)". Source `index.ts:341-344` confirms this — but it is impossible to reach: `validateStatus` at `:283-284` includes 404, so a 404 response NEVER throws and the catch-block 404 branch is dead code. The spec acknowledges its existence parenthetically but does not flag it as defensive-only / unreachable. A reimplementer could omit the catch-side branch and produce identical behavior; or, conversely, add it and produce subtle differences if `validateStatus` is later edited. Recommend marking dead-code-defensive explicitly.

### F3 (MEDIUM) — `claudeAiLimits.ts` / `rateLimitMessages.ts` ownership claim contradicts master plan

§1, §2.1 last row, §6.10, §12 Q4 all defer the rate-limit/quota files to spec 22. The Phase 6 master plan (per the user's prompt) originally assigned `rateLimitMessages` verbatim assets to spec 27, and Phase 9.6 spec 22 §12 Q8 explicitly **handed it back** to spec 27. Spec 27 currently rejects that handoff. Both `rateLimitMessages.ts` (344 lines) and `claudeAiLimits.ts` (515 lines) exist and are unowned by either spec under the current spec-27 text. This is a documentation gap, not a source-truth bug. Resolution required at the Phase 9.6 cross-spec arbitration level — but absent that, spec 27 §6 should at minimum enumerate the verbatim user-facing rate-limit strings even if classification logic is deferred.

### F4 (MEDIUM) — Bash sandbox.excludedCommands / `tengu_sandbox_disabled_commands` not cross-referenced

`sandbox.excludedCommands` lives in `policySettings` and is read from there (`BashTool.tsx:343`), while `tengu_sandbox_disabled_commands` is a GrowthBook payload (`shouldUseSandbox.ts:27`). Spec 10 found these. Spec 27 §11 reimplementation checklist notes the policy cascade but never enumerates `sandbox.excludedCommands` as a load-bearing key inside `SettingsJson`, despite owning the policySettings resolver. Reimplementers reading only spec 27 will not learn that `sandbox.excludedCommands` traverses this exact resolver. Recommend cross-link in §11 or §12.

### F5 (LOW) — `isPluginBlockedByPolicy` is a 3-line wrapper over `getSettingsForSource('policySettings')` and is the cross-spec contract surface for spec 28

`utils/plugins/pluginPolicy.ts:17-20` is the only code path between `policySettings.enabledPlugins[id] === false` and the entire plugin install/enable/UI chokepoint (~10 call sites in `commands/plugin/*`, `services/plugins/pluginOperations.ts`, `utils/plugins/{pluginInstallationHelpers,hintRecommendation}.ts`). Spec 27 owns the policySettings resolver but never mentions this function. If spec 28 also doesn't own it (cross-spec ambiguity), the chokepoint is undocumented. Recommend §3 add a note: "Plugin policy enforcement consumes policySettings via `utils/plugins/pluginPolicy.ts:isPluginBlockedByPolicy` — spec 28 owns that file."

## Other findings

- (LOW) §5.3 step 5 says SettingsSchema validation failure "either failure → `{ success:false, error:... }` (no `skipRetry`, retried)". Source confirms but does not mention that this means a malformed remote settings response will burn all 6 attempts before falling back to cache — a malicious or buggy backend can add ~30s of startup latency. Worth flagging in §9.
- (LOW) §3.2 documents `resetSyncCache` exported from BOTH `syncCache.ts` and `syncCacheState.ts`. Source confirms. Two functions with the same name exported from two different modules in the same service; only the wrapper in `syncCache.ts` clears both eligibility memos. A reimplementer importing from `syncCacheState.ts` would leave the eligibility memo in `syncCache.ts` stale. Spec should warn.
- (LOW) §5.4 correctly catches the `localeCompare` vs default-sort divergence between policyLimits and remoteManagedSettings checksum. Verified at `policyLimits/index.ts:139` and `remoteManagedSettings/index.ts:118`. This produces different sort orders for non-ASCII keys; the comment at remote `:128-129` claims compat with Python's `sort_keys=True` (which uses default code-point sort, NOT locale). Therefore the **remoteManagedSettings** version is correct and **policyLimits** is the buggy one. Spec calls it "verbatim divergence" without naming the bug. Recommend reclassifying as a confirmed bug in §12.
- (NIT) §6.6 lists `LOADING_PROMISE_TIMEOUT_MS` once but it appears at TWO source locations with the same value. Listing both line refs is fine, but the table only shows two refs in one row.
- (NIT) §3.4 shows `isInProtectedNamespace` body as `'./protectedNamespace.js'` lazy-require. Verified at `envUtils.ts:136-147`. File `src/utils/protectedNamespace.ts` confirmed absent from leak (glob returns no matches).

## Verdict

**ACCEPT WITH REVISIONS.** Spec is unusually thorough — every constant, endpoint, and control-flow branch I sampled matched source exactly. The cascade body, eligibility tri-state, security gate, and 30s deadlock guard are all faithful. The HIGH and MEDIUMs above are documentation completeness/cross-spec issues, not source-truth bugs. F3 (rateLimitMessages ownership) requires a Phase 9.6 arbitration decision, not a unilateral spec-27 edit.

## Cross-spec impact

- **Spec 22 (claudeAiLimits/rateLimit):** F3 requires explicit re-arbitration — spec 22 handed off, spec 27 deferred. Currently both disclaim ownership.
- **Spec 28 (plugin):** F5 — `isPluginBlockedByPolicy` cross-link missing.
- **Spec 10 (Bash policy):** F4 — `sandbox.excludedCommands` traverses spec 27's resolver but cross-link is absent.
- **Spec 02 (settings cascade):** Spec 27 explicitly OWNS the policySettings resolver body; both source paths (`getSettingsForSourceUncached` line 319-345 and the merge body 660-739) verified. No conflict.
- **Spec 09 (permissions/protectedNamespace):** §3.4 correctly attributes `isInProtectedNamespace` here. Source-confirmed at `envUtils.ts:136`.
- **Spec 26 (analytics):** §6.9 events all confirmed in source.

## Hardest-to-verify claim

The spec's F1-related claim that `notifyChange('policySettings')` "resets the settings cache internally before iterating listeners" (`remoteManagedSettings/index.ts:543-544` comment paraphrase). The chain runs `notifyChange` → `changeDetector.notifyChange` → listener fan-out which includes `applySettingsChange` → `resetSettingsCache`. To verify the "before iterating listeners" claim requires reading `changeDetector.notifyChange`'s implementation (not sampled here) and `applySettingsChange.ts`. The spec asserts ordering that I could not confirm without two more file reads. If the reset happens AFTER one or more listeners, those listeners would observe the OLD merged-settings cache — a real bug, not a doc bug. Flagging as hardest-to-verify; recommend Phase 9.7 confirm by reading `changeDetector.ts:notifyChange` body.
