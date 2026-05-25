# PHASE9-FIXES-28b — spec 28 service-plugins (Phase 9.6c)

Companion to `PHASE9-FIXES-28.md` (Phase 9.7 pass). This file logs the second-pass fixes applied during Phase 9.6c after the adversarial review identified five additional issues.

Source: `docs/specs/PHASE9-ADVERSARIAL-28.md` (Phase 9.6c findings); spec edited in-place at `docs/specs/28-service-plugins.md`.

---

## Fixes applied

### F-28b.1 (HIGH) — `lsp-config-invalid` declared twice in source

**Finding.** `src/types/plugin.ts:177-182` and `:220-225` declare structurally-identical arms of the `PluginError` discriminated union. Spec 28 §6.22 / §9.1 cite "24 variants"; the count is correct only because TypeScript de-duplicates structurally-identical union members. A reimplementation that enumerates arms via macro / codegen rather than via `keyof` would double-count.

**Verified.** `grep -n "type: 'lsp-config-invalid'" src/types/plugin.ts` → two declaration matches at `:177` and `:220`, plus a `case 'lsp-config-invalid':` consumer at `:335` (not a third declaration).

**Action.**
1. Added entry **6** to `docs/specs/BUGS-IN-SOURCE.md` (Confirmed bugs section) — severity `cosmetic`, with reproduction grep and suggested fix (delete the second occurrence at `:220-225`).
2. Updated header total from "5 confirmed" to "6 confirmed".
3. Added source caveat sentence to spec 28 §9.1 explaining why the "24 variants" count is correct only by TS dedup.

---

### F-28b.2 (MED) — `bundled/index.ts` registers zero built-in plugins

**Finding.** `src/plugins/bundled/index.ts:20-23` ships with `initBuiltinPlugins()` body comment "No built-in plugins registered yet — this is the scaffolding…". Spec 28 §5.10 / §5.17 and the cross-ref to spec 17 imply runtime activity through `getBuiltinPluginSkillCommands()`; in shipped source `BUILTIN_PLUGINS` is empty and `builtinPluginSkills` always `[]`.

**Verified.** Read `src/plugins/bundled/index.ts` in full — single function with a comment-only body. No `registerBuiltinPlugin()` call sites exist in the leaked tree.

**Action.** Added a "Caveat (shipped source)" callout to spec 28 §5.10, immediately after the `getBuiltinPluginSkillCommands()` paragraph. The callout makes explicit that:
- `BUILTIN_PLUGINS` is empty at runtime;
- `getBuiltinPluginSkillCommands()` always returns `[]`;
- the documented for-loop is dead in the shipped source;
- the contract holds when entries are added but a faithful reimplementation produces no built-in plugin commands today.

---

### F-28b.3 (MED) — `NO_AUTO_UPDATE_OFFICIAL_MARKETPLACES` behavioral consequence undocumented

**Finding.** `src/plugins/schemas.ts:~78` (cited as §6.16, line shown verbatim at spec line 917) declares `NO_AUTO_UPDATE_OFFICIAL_MARKETPLACES = new Set(['knowledge-work-plugins'])`. The constant appears verbatim in §6.16 but its behavioral consequence is not described in §5 (control flow) or §11 (reimplementation checklist).

**Verified.** Set declaration present in §6.16 verbatim block at line 917. No descriptive prose in §5.11 (background reconciliation), §5.12 (GCS fast-path), or §11.

**Action.** Inserted an inline behavioral-consequence comment immediately under the verbatim declaration in §6.16:
- marketplaces in this set are excluded from background auto-update reconciliation despite passing official-name source-validation;
- only `'knowledge-work-plugins'` is opted out today;
- cross-cuts to §5.11 / §5.12;
- reimplementations MUST honor the opt-out (rationale: this marketplace is curated externally).

---

### F-28b.4 (MED) — `resetSettingsCache()` does not clear `pluginSettingsBase` (cross-spec 02 hazard)

**Finding.** `src/utils/settings/settingsCache.ts:55-59` (`resetSettingsCache()`) clears `sessionSettingsCache`, `perSourceCache`, and `parseFileCache` but does NOT touch `pluginSettingsBase` (declared at `:66`, mutated by `setPluginSettingsBase()` / `clearPluginSettingsBase()` at `:72` / `:78`). Spec 28 §5.14 has the correct primitive ordering for `clearPluginCache()` (calls `resetSettingsCache()` then `clearPluginSettingsBase()`) but does not warn that any **other** caller of `resetSettingsCache()` (settings write, `--add-dir`, hooks refresh — per `settingsCache.ts:18` JSDoc) leaves a stale `pluginSettingsBase` behind.

**Verified.** Read `src/utils/settings/settingsCache.ts` in full. Confirmed the function body of `resetSettingsCache()` does not reference `pluginSettingsBase`; the only clearer is `clearPluginSettingsBase()` at `:78`.

**Action.** Added a "Cross-spec hazard (→ spec 02 settings cascade)" callout immediately after the §5.14 pseudocode block. The callout:
- describes the omission in `resetSettingsCache()`;
- notes the correct ordering inside `clearPluginCache()` (the only call site that pairs both clears);
- warns reimplementers of spec 02's cascade to either extend `resetSettingsCache()` to also call `clearPluginSettingsBase()` OR document that every cascade-invalidation site must call both;
- highlights that the current source does the latter implicitly via only one site (`clearPluginCache()`), making non-plugin invalidation paths leave stale plugin settings cached.

---

### F-28b.5 (LOW) — `pluginOperations.ts` public lifecycle exports not enumerated

**Finding.** `src/services/plugins/pluginOperations.ts` (1088 LOC) hosts the `/plugin` slash-command and headless-install entry points. Spec 28 §3.5 listed `refresh` / `PluginInstallationManager` / `reconciler` / `headlessPluginInstall` entries but did not enumerate the seven public operation functions in `pluginOperations.ts` by name and line number.

**Verified.** `grep -nE '^export (async )?function' src/services/plugins/pluginOperations.ts` →
- `installPluginOp` `:321`
- `uninstallPluginOp` `:427`
- `setPluginEnabledOp` `:573`
- `enablePluginOp` `:756`
- `disablePluginOp` `:770`
- `disableAllPluginsOp` `:782`
- `updatePluginOp` `:829`

Internal helper `performPluginUpdate` at `:896` (not exported).

Result type exports: `PluginOperationResult` `:141`, `PluginUpdateResult` `:154`. Scope helpers: `assertInstallableScope` `:90`, `isInstallableScope` `:104`, `getProjectPathForScope` `:114`, `isPluginEnabledAtProjectScope` `:128`. Constants: `VALID_INSTALLABLE_SCOPES` `:72`, `VALID_UPDATE_SCOPES` `:78`.

**Action.** Appended an enumeration block to spec 28 §3.5 — the seven public op functions with line numbers, plus the result-type / scope-helper / constants triad. Notes that `enablePluginOp` and `disablePluginOp` are thin wrappers over `setPluginEnabledOp`, and that `updatePluginOp` delegates to internal `performPluginUpdate`.

---

## Files changed

- `docs/specs/28-service-plugins.md` — five in-place edits (§3.5 enumeration, §5.10 caveat, §5.14 cross-spec hazard, §6.16 NO_AUTO_UPDATE comment, §9.1 dedup caveat).
- `docs/specs/BUGS-IN-SOURCE.md` — added entry 6, updated header total `5 → 6`.
- `docs/specs/PHASE9-FIXES-28b.md` — this file (new).

## Files NOT changed

- No source patches. `BUGS-IN-SOURCE.md` is documentation-only; source remains as leaked.
- `PHASE9-FIXES-28.md` (Phase 9.7 first pass) untouched.

## Verification

All five claims grep-verified against `src/` before editing:
- `lsp-config-invalid` duplicate: two decl matches at `:177`, `:220`.
- `bundled/index.ts`: 23 lines total, function body is a comment.
- `NO_AUTO_UPDATE_OFFICIAL_MARKETPLACES`: present in §6.16 verbatim, absent in §5 / §11 narrative.
- `resetSettingsCache()` body: read in full, does not touch `pluginSettingsBase`.
- `pluginOperations.ts` exports: enumerated via `grep '^export (async )?function'`.
