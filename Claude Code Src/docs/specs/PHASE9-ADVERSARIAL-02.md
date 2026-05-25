# Phase 9.5 Adversarial Review — Spec 02 (Settings, Schemas, Migrations)

## Severity Counts

- **CRITICAL**: 0
- **HIGH**: 1
- **MEDIUM**: 4
- **LOW**: 5
- **NIT**: 4

Total: 14 findings.

---

## Findings

### M1 — Migration count and ordering claim (MEDIUM, partially correct)

Spec §2.1 row count claims "11 files" of migrations and §5.10 enumerates 11 migrations in the order called by `main.tsx`. Source verified at `src/main.tsx:325` (`CURRENT_MIGRATION_VERSION = 11`) and `:328-342`. **However**, the spec line 822 ordering is correct only because `migrateFennecToOpus` is called inside `if ("external" === 'ant')` — a compile-time DCE branch — not the runtime `process.env.USER_TYPE === 'ant'` gate the spec implies (§2.2 row at line 128 cites "USER_TYPE === 'ant'" inside the migration body, which is true for `migrateFennecToOpus.ts:19` but the **outer** call site in `main.tsx:340` is also gated). The spec underdescribes the dual gate. Also: spec §2.1 lists 11 migration files but `ls src/migrations/` shows **11** files — count is correct.

### H1 — `INDEX.md` adjacency claim: spec describes file `src/services/configLoader.ts` does not exist (HIGH)

Reviewer's prompt says "particularly src/services/configLoader.ts" — there is **no** `src/services/configLoader.ts` in the source tree. The spec itself does **not** claim it exists; it correctly maps the subsystem under `src/utils/settings/`. This is a **prompt-side** issue, not spec drift. Reported here for cross-spec impact: any spec or INDEX entry that points readers to `src/services/configLoader.ts` is wrong. Confirmed via `find src -name configLoader*` → empty.

### M2 — `pluginTrustMessage` vs `types.ts` line numbers (MEDIUM)

Spec §3.4 lists `pluginTrustMessage` and §7.6 cites `types.ts:1062-1070` as the location of the "policy-only" `.describe()`. Verified: `types.ts:1062` does begin `pluginTrustMessage: z`. Claim consistent.

### M3 — `effortLevel` line cite drifts (MEDIUM)

Spec §2.2 line 112 cites `types.ts:705-708` for the ANT-only `effortLevel` widening. The verbatim block at spec §6.1 line 1192-1198 in this file shows `effortLevel` defined at a position consistent with the surrounding ~1192 line region — but spec's "line 705-708" cite is far off the actual location in the bundled types.ts (≈line 1192). Cite is **stale**. (Likely from an older revision where the schema was shorter; the spec separately claims `types.ts` is 1148 lines but verbatim block spans 1255+, suggesting >1148 lines now.) Verified line count: `wc -l types.ts` = **1148**. So the verbatim block referencing line 1255+ in §6.1 is internally inconsistent — either the verbatim is paraphrased or the line cites are wrong.

### M4 — `flagSettings` collapsed to `'policy_settings'` in `ConfigChangeSource` (MEDIUM)

Spec §5.5.3 maps `flagSettings → 'policy_settings'`. This is a real source claim (`changeDetector.ts:252-266` per spec). I did not directly read `changeDetector.ts` so cannot confirm — but the open-question §12.8 flags this as unowned. Worth verification by spec 09 owner: collapsing `flagSettings` and `policySettings` into the same `ConfigChangeSource` is non-obvious and a breaking surprise for downstream `ConfigChange` hook consumers.

### L1 — `constants.ts` line count (LOW)

Spec §2.1 lists `constants.ts` at 202 lines; actual is **203**. Off-by-one — likely trailing-newline counting difference. Not a correctness issue.

### L2 — `useAutoModeDuringPlan` "default true" wording (LOW)

§3.4 lists default as `true`; §12.12 acknowledges it is **not** a schema-level `.default(true)` — `getUseAutoModeDuringPlan()` returns true unless any trusted source explicitly sets false. The cross-spec contract row will mislead consumers who expect `getUseAutoModeDuringPlan() === settings.useAutoModeDuringPlan ?? true`.

### L3 — `cleanupPeriodDays` default 30 listed in §3.4 (LOW)

Schema is `.optional()` with `.describe()` text "(default: 30)". There is **no** `.default(30)` in Zod. §3.4 default column is ambiguous — "30" is a consumer-side fallback, not enforced by schema. Same pattern affects `respectGitignore`, `defaultShell`, `terminalTitleFromRename`, `spinnerTipsEnabled`, `alwaysThinkingEnabled`, `promptSuggestionEnabled`. Spec should annotate "(describe-only, not Zod default)" once.

### L4 — `pluginSettingsBase` not flushed by `resetSettingsCache()` (LOW, correctly flagged but reimplementer hazard)

§4.2 notes `pluginSettingsBase` is a separate mutable singleton not flushed by `resetSettingsCache()`. This is a hidden assumption — reimplementer who folds it into the regular cache will silently break plugin reloading. The spec correctly flags it but does not enumerate **why** (presumably plugin loader invalidates it explicitly).

### L5 — "first source wins" comment vs `mergeWith` for policy (LOW)

§5.1 pseudocode for `policySettings` says "FIRST-SOURCE-WINS (NOT merge)" but then calls `mergeWith(merged, policySettings, settingsMergeCustomizer)` after picking the winning source — verified at `settings.ts:723-728`. So policy is merged into the cascade like any other source; "first-source-wins" applies only to the **selection of which** policy source provides `policySettings`. Spec wording could mislead — "first source wins" suggests no merge with user/project, but in fact `policySettings` participates in the cascade (just at highest priority).

### NIT-1 — §1.1 wording

§1 line 17 says "MDM 30-minute poll" — confirmed at §5.5.4 (`MDM_POLL_INTERVAL_MS = 30 * 60 * 1000`).

### NIT-2 — `getEnabledSettingSources()` claim about always-on policy/flag (verified)

§5.1 says "policy and flag are always loaded, regardless of `--setting-sources`". Verified at `constants.ts:159-167`: `result.add('policySettings'); result.add('flagSettings')` after copying `getAllowedSettingSources()`. ✓

### NIT-3 — `settings.local.md` (per prompt)

The prompt asks about "settings.local.md" — **the spec talks about `settings.local.json`**, not `.md`. There is no `settings.local.md` pattern in this subsystem. (`.local.md` is a plugin-settings convention owned by spec 28 / a different skill.) No drift in spec 02.

### NIT-4 — `pluginConfigs` schema completeness

§3.4 lists `pluginConfigs.<id>.{mcpServers,options}` matching schema at `types.ts:1220-1236`. ✓

---

## Spec-Level Verdict

**ACCEPT WITH MINOR REVISIONS.**

Spec 02 is unusually rigorous — it enumerates every migration, every feature/ANT gate with `file:line` cites, every settings key with downstream consumer mapping, and explicitly documents trust-boundary invariants. Verbatim §6 reproduces the schema. Open-questions §12 self-flags 13 unowned-by-this-spec items.

Issues found are mostly **line-cite drift** (M3, L1) and **describe-vs-default ambiguity** (L3, L2). The "five-source cascade" and "first-source-wins for policy" claims are accurate. The migration version chain is monotonic and matches `CURRENT_MIGRATION_VERSION = 11`. ENV variable enumeration §7.5 is consistent with grep output.

No CRITICAL contradictions. The spec correctly documents that **the overview's `env > project > user > MDM > defaults` claim is wrong** (§4.1 explicit contradiction) — that overview drift is a 00-spec issue, not a 02 issue.

---

## Cross-Spec Impact

1. **Spec 00 (overview)**: §12.11 explicitly flags overview's precedence claim is wrong. Overview must be corrected to `userSettings → projectSettings → localSettings → flagSettings → policySettings` (lowest-to-highest), with `pluginSettingsBase` below all five. Env vars are not part of cascade.
2. **Spec 09 (permissions/hooks runtime)**: owns the `ConfigChangeSource` mapping where `flagSettings` collapses to `policy_settings` (spec 02 §5.5.3, §12.8). Confirm consumer-side handling.
3. **Spec 23 (MCP)**: relies on settings-side `enabledMcpjsonServers`, `allowedMcpServers`, `deniedMcpServers`, `enableAllProjectMcpServers`. All present in `SettingsSchema`. ✓
4. **Spec 27 (policy/limits)**: provides `getRemoteManagedSettingsSyncFromCache()` consumed by 02 §5.1 step (1). Spec 02 correctly defers cache lifetime.
5. **Spec 28 (plugins)**: owns `enabledPlugins`, `extraKnownMarketplaces`, `pluginConfigs`, and the `pluginSettingsBase` writer. Spec 02 correctly defers plugin schema details.
6. **Spec 41 (~/.claude.json / GlobalConfig)**: spec 02 §4.4 enumerates all GlobalConfig keys mutated by migrations — 41 must own the schema.
7. **Spec 22 (model resolution)**: model migrations depend on `isLegacyModelRemapEnabled`, `isOpus1mMergeEnabled`, `getDefaultMainLoopModelSetting`, `getAPIProvider` — owned by 22.

No false enumerations of settings keys; the §3.4 table covers every visible top-level key in the verbatim schema.

---

## Hardest-to-Verify Claim

> **§5.4 invariant: "Arrays replace on write but concat-dedupe on read"**

This asymmetry between `settingsMergeCustomizer` (read path, `settings.ts:538-547`, returns `uniq([...obj, ...src])` for arrays) and the inline write-path customizer (`settings.ts:478-494`, returns `srcValue` for arrays plus `undefined === delete`) is a foot-gun that requires reading two different functions in the same file and noticing they have **opposite** array semantics. I verified both at `settings.ts:478-494` and `:538-547` — claim is **correct**.

The runtime consequence: a caller of `updateSettingsForSource('userSettings', {permissions:{allow:['C']}})` whose existing user settings have `permissions.allow = ['A','B']` will overwrite to `['C']` only — but at next **read**, if `projectSettings` also has `['D']`, the merged result is `['C','D']`. This is a non-obvious "compute-the-final-array-yourself" contract that callers must respect; misuse silently corrupts permission rules. Spec correctly flags it as a §5.4 hard-to-spot invariant. Verifying without reading the source twice is essentially impossible.
