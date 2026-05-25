# Phase 9.6 B-full — Spec 02 Fix Log (Settings, Schemas, Migrations)

Source: `docs/specs/PHASE9-ADVERSARIAL-02.md`. All findings verified against `src/` before editing. `src/` is read-only — only the spec file was modified.

## Findings disposition

| Finding | Severity | Disposition | Evidence |
|---|---|---|---|
| H1 — claim about non-existent `src/services/configLoader.ts` | HIGH | **SKIP** (prompt-side issue, not spec) | Spec 02 already correctly maps subsystem under `src/utils/settings/`. `find src -name configLoader*` → empty. The bad path was a reviewer-prompt artifact; spec text contains no such reference. |
| M1 — `migrateFennecToOpus` is dual-gated | MEDIUM | **APPLIED** to §2.2 row + §5.10 entry 11 + new "Pattern C reminder" callout | `main.tsx:340` has `if ("external" === 'ant') { migrateFennecToOpus(); }` (build-time DCE marker, Bun `bun:bundle`-stripped). `migrateFennecToOpus.ts:18-19` has `if (process.env.USER_TYPE !== 'ant') return;` (runtime). Spec previously documented only inner gate. |
| M3 — `effortLevel` line cite `:705-708` → `:703-711` | MEDIUM | **VERIFIED ALREADY APPLIED** (prior session) | §2.2 line 112 reads `src/utils/settings/types.ts:703-711` with `(inline ternary at :705)` — matches spec body. No re-edit needed. |
| M4 — `flagSettings` collapses to `'policy_settings'` | MEDIUM | **APPLIED** to §5.5.3 — added "Information-loss collision" callout | `changeDetector.ts:262-264`: `case 'flagSettings': case 'policySettings': return 'policy_settings'`. Verified directly. Spec 09 cross-spec impact noted in callout. |
| L3 — `.describe()` "default: N" ≠ Zod default | LOW | **APPLIED** to §3.4 preamble — added "Default-column convention" callout enumerating affected keys | Confirmed in spec verbatim (§6) — no `.default(...)` call on `cleanupPeriodDays`, `respectGitignore`, `defaultShell`, etc. Reimplementer hazard explained: Zod-defaulting these keys would change cascade semantics. |
| Pattern F — read/write merge customizer asymmetry | (Hard-to-verify, flagged by review) | **APPLIED** to §5.4 — added "Pattern F" callout with side-by-side table and reimplementer warning | `settings.ts:478-494` (write-side, inline) returns `srcValue` for arrays + `undefined === delete`. `settings.ts:538-547` (`settingsMergeCustomizer`) calls `mergeArrays` → `uniq([...obj, ...src])`. Two customizers, opposite array behaviour, **both correct**. Verified by reading both functions. |

## Top 3 fixes (with src evidence)

### 1. Dual gate (M1) — build-time DCE plus runtime guard

**Spec change**: §2.2 row for `migrateFennecToOpus.ts:19-21` now states the migration is dual-gated; §5.10 entry 11 now reads "DUAL-GATED: outer if (\"external\" === 'ant') at main.tsx:340 (build-time DCE) + inner if (process.env.USER_TYPE !== 'ant') return at migrateFennecToOpus.ts:19 (runtime)" plus a new explanatory paragraph below the migration list.

**Source evidence**:
- `src/main.tsx:340`: `if ("external" === 'ant') { migrateFennecToOpus(); }`
- `src/migrations/migrateFennecToOpus.ts:18-19`: `export function migrateFennecToOpus(): void { if (process.env.USER_TYPE !== 'ant') {`

**Why both**: `"external"` is a literal that Bun's `bun:bundle` replaces at build time; the comparison evaluates to `false` for the external bundle and the entire `if` body (including the `import` reference to `migrateFennecToOpus`) is dead-code-eliminated. This is per CLAUDE.md "Two Dominant Patterns" — Pattern C (feature-flag/USER_TYPE-gated imports for DCE). The inner runtime gate is defense-in-depth.

### 2. ConfigChangeSource collision (M4)

**Spec change**: §5.5.3 mapping table now followed by an "Information-loss collision (reimplementer hazard)" callout that records both source-side identifiers map to one downstream identifier `'policy_settings'`, that hook subscribers cannot distinguish, that in practice this is benign because flagSettings is short-circuited by `changeDetector.ts:190-194`, and that spec 09 owns the union-surface documentation.

**Source evidence**: `src/utils/settings/changeDetector.ts:262-264`:
```ts
case 'flagSettings':
case 'policySettings':
  return 'policy_settings'
```
Plus `:190-194` short-circuit verified.

### 3. Read/write customizer asymmetry (Pattern F)

**Spec change**: §5.4 invariants now include a side-by-side table of the two customizers, their file locations, their array semantics, and a reimplementer-hazard warning that consolidating them in either direction silently breaks the permission cascade.

**Source evidence** (read directly):
- `src/utils/settings/settings.ts:478-494` (write-side inline): comment "For arrays, always replace with the provided array. This puts the responsibility on the caller to compute the desired final state."; `if (Array.isArray(srcValue)) { return srcValue }`.
- `src/utils/settings/settings.ts:529-531` (`mergeArrays` helper): `return uniq([...targetArray, ...sourceArray])`.
- `src/utils/settings/settings.ts:538-547` (`settingsMergeCustomizer`): wraps `mergeArrays` for read-side.

## Build-time DCE pattern note (CLAUDE.md cross-reference)

The `"external" === 'ant'` literal-string pattern is documented in CLAUDE.md as one of the "Two Dominant Patterns" of the codebase: optional/ANT-only subsystems are guarded by literal-string compares that Bun's `bun:bundle` collapses at build time. The leaked source preserves these gates as `"external" === 'ant'` (always false in the leaked external bundle) rather than as variables — so when grep returns the literal `"external"` it should be read as a build-time marker, not a runtime check. The inner `process.env.USER_TYPE === 'ant'` gates inside the migration bodies, MDM constants, and managed-path overrides are runtime defense-in-depth and are present even after DCE strips the outer wrapper.

Other `"external" === 'ant'` sites in spec 02's owned subsystem are already enumerated in §2.2 (types.ts, settings.ts, applySettingsChange.ts, managedPath.ts, mdm/constants.ts) — those are runtime gates inside files; the build-time-DCE wrapper is specifically the `main.tsx:340` call site for `migrateFennecToOpus`. No other migration in §5.10 has the same dual structure (the others all run for everyone or are gated by feature flags / billing-tier helpers, not USER_TYPE).

## Findings not addressed in this session

The following findings from `PHASE9-ADVERSARIAL-02.md` were not in the B-full retry scope: M2 (`pluginTrustMessage` line cite — already verified as consistent in review, no action needed), L1 (constants.ts off-by-one line count), L2 (`useAutoModeDuringPlan` "default true" wording — partially covered by L3 callout), L4 (`pluginSettingsBase` not flushed by `resetSettingsCache()` — already noted in §4.2), L5 ("first source wins" comment vs `mergeWith` — semantic clarity, no factual error), NIT-1..4 (verified-correct items, no action).

## Verifications run

- `wc -l docs/specs/02-settings-schemas-migrations.md` → 1902 (pre-edit) / grew by ~25 lines after callouts
- `grep "external" src/main.tsx | grep -i ant` → 19 hits, including `:340` for `migrateFennecToOpus`
- `grep USER_TYPE src/migrations/migrateFennecToOpus.ts` → `:19`, runtime gate confirmed
- `grep -rn "policy_settings\|flagSettings" src/utils/settings/` → confirms the `changeDetector.ts:262-264` collision and that flagSettings is short-circuited at `:190-194`
- Direct read of `src/utils/settings/settings.ts:470-547` → confirms two distinct customizers with opposite array semantics
