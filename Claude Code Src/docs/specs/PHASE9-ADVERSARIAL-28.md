# Phase 9.5b Adversarial Review — Spec 28 (Service: Plugins)

Reviewer: Skeptic. Read-only verification against `src/plugins/`,
`src/services/plugins/`, `src/utils/plugins/schemas.ts`, `src/types/plugin.ts`,
`src/utils/settings/settingsCache.ts`, `src/plugins/bundled/index.ts`.
Source reads used: ~12 of the 25 budget.

## Severity counts

| Severity | Count |
|---|---|
| CRITICAL (factual error / spec contradicts source) | 0 |
| HIGH (load-bearing claim with material gap) | 1 |
| MEDIUM (defensible but imprecise) | 3 |
| LOW (cosmetic / typo / minor over-claim) | 3 |
| Confirmed-correct spot checks | 8 |

## Top 5 findings

### 1. HIGH — Spec §6.22/§9.1 "24-variant PluginError" obscures a real source-side duplicate

`types/plugin.ts:101-283` lists **25 union arms**, but `'lsp-config-invalid'`
appears **twice** (lines 177 and 220) with identical shape — a copy/paste
duplicate in the discriminated union. Net unique = 24, so the spec's headline
count is correct, but neither §9.1 ("24 variants. Currently produced…") nor §6.23
("user-facing error strings, by error type") notes that the source has a
literal duplicate arm. Reimplementer who deduplicates by name would still get
24; reimplementer who copies the union verbatim gets a TS-legal but redundant
arm. Worth a sentence in §9.1 ("source has lsp-config-invalid declared twice;
TypeScript collapses identical members") to forestall a "did I miscount?" bug
report.

### 2. MEDIUM — `bundled/index.ts` is empty scaffolding, but spec §5.10/§17 ripple still asserts skill masquerade

`src/plugins/bundled/index.ts:20-23` shows `initBuiltinPlugins()` registers
**zero** built-in plugins ("No built-in plugins registered yet — this is the
scaffolding…"). Spec §5.10 documents the `getBuiltinPluginSkillCommands()` →
`source:'bundled'` masquerade and §5.17 places `builtinPluginSkills` SECOND in
the catalog ordering. Both are correct in principle, but the spec implies
real built-in plugin skills flow through this path today; in the leaked
source, `builtinPluginSkills` is always `[]`. Spec should add a one-line
"currently empty in shipped source; behavior verified by code path, not by
runtime presence" caveat at §5.10 / §3.3. Cross-spec 17 may overstate the
runtime importance of this masquerade.

### 3. MEDIUM — Spec §6.16 transcribes `BLOCKED_OFFICIAL_NAME_PATTERN` and `ALLOWED_OFFICIAL_MARKETPLACE_NAMES`, but does not call out `NO_AUTO_UPDATE_OFFICIAL_MARKETPLACES`

Spec lists the constant at §6.16 (`new Set(['knowledge-work-plugins'])`) but
the auto-update suppression behavior tied to that set is never described in
§5 or §11 invariants. The marketplace UI / install lifecycle treats
`knowledge-work-plugins` differently — that is a behavior-bearing constant,
not just a name. §11 invariant 6 enumerates the reserved-names rules but
omits this one.

### 4. MEDIUM — Spec §5.14 cache invalidation order is correct but the cross-spec hint to spec 02 is implicit

Verified `resetSettingsCache()` at `settingsCache.ts:55-59` clears
`sessionSettingsCache`, `perSourceCache`, `parseFileCache` only — does NOT
touch `pluginSettingsBase` (declared at line 66 with its own
`clear/get/setPluginSettingsBase` accessors). Spec §5.14 correctly orders
"`resetSettingsCache()` then `clearPluginSettingsBase()`" but never spells
out for spec 02's reader: **`pluginSettingsBase` survives a bare
`resetSettingsCache()` call**. Consumers in spec 02 who call only
`resetSettingsCache()` (e.g., on a settings file write) will see a stale
plugin base layer until `clearPluginCache()` runs. This is the core
cross-spec hazard the prompt flagged; spec 28 documents the *primitive* but
not the *hazard surface*. One sentence in §11 invariant 10 would close this.

### 5. LOW — `pluginOperations.ts` is 1088 lines and §5.6 does not enumerate `installPluginOp` / `uninstallPluginOp` / `enablePluginOp` / `disablePluginOp` / `setPluginEnabledOp` / `disableAllPluginsOp` / `updatePluginOp`

§3.5 lists `runHeadlessPluginInstall` and `performBackgroundPluginInstallations`
but the **CLI-surface** operations (`installPluginOp:321`, `uninstallPluginOp:427`,
`setPluginEnabledOp:573`, `enablePluginOp:756`, `disablePluginOp:770`,
`disableAllPluginsOp:782`, `updatePluginOp:829`) are never named. These are
the public verbs of the lifecycle the prompt asked us to verify (install /
enable / options / disable / uninstall). §2.1 lists the file at "1088 lines,
grep" but the public operation surface is not enumerated even by name. A
small subsection (§3.5.x or §5.x) listing these seven exports with line numbers
would make the spec self-sufficient for lifecycle reimplementation.

## Verdict

**ACCEPT WITH MINOR REVISIONS.** The spec is broadly accurate and
load-bearing source quotations (PluginManifestSchema composition,
PluginError taxonomy, identifier parsing, version-recompute invariant,
GCS atomic-swap path-safety, command/skill ordering at commands.ts:449-468)
match source verbatim. Phase 9.7 §5.17 verbatim-rewrite is preserved and
correct. The 5 findings above are additive (not corrections); none invalidates
the spec's core contract. Recommend adding (a) PluginError-duplicate note,
(b) bundled/index.ts empty-scaffolding caveat, (c) NO_AUTO_UPDATE constant
behavior, (d) settings-cache hazard sentence, (e) pluginOperations.ts
lifecycle-export enumeration.

## Cross-spec impact

- **Spec 02 (settings cascade)**: reader needs explicit warning that
  `resetSettingsCache()` does NOT clear `pluginSettingsBase`. Spec 28 §5.14
  documents the primitive; spec 02 should pull this hazard forward.
- **Spec 17 (skills)**: masquerade path described in §5.10 / §5.17 is
  *currently dead code* (zero built-in plugins registered). Spec 17 must not
  cite "Phase 9 has built-in plugin skills shipping" — it has the *machinery*
  only.
- **Spec 21 / 21d (catalog)**: §5.17 ordering note is accurate; ripple resolved.
- **Spec 23 (MCP)**: channel-server cross-validation deferred at §12.9 —
  unchanged, appropriate.
- **Spec 26 (analytics)**: `tengu_marketplace_background_install`,
  `tengu_plugin_remote_fetch` event shapes specified verbatim — no impact.

## Hardest-to-verify claim

**Spec §5.4 "Version-recompute invariant"** at `pluginLoader.ts:2331-2349`:
"if pre-clone version was deterministic (any of `source.sha`, `entry.version`,
`installedVersion`), REUSE it; recompute only when pre-clone was `'unknown'`.
Otherwise post-clone manifest.version (rank 1) supplants gitCommitSha (rank 3)
and the cache key would diverge from the warm-start probe key, causing
re-clone-forever."

Verifying this would require reading `calculatePluginVersion` end-to-end,
the `probeSeedCache` matching algorithm, AND reproducing a warm-start vs
cold-start scenario where `manifest.version` differs from `gitCommitSha` — a
multi-file flow inside a 3302-line `pluginLoader.ts` (declared "sampled by
section" in §2.1). The invariant *sounds* reasonable and matches the §11
invariant 9 wording, but the failure mode (silent re-clone every startup) is
exactly the kind of bug that hides behind "it works on my cache". Without
runtime reproduction (impossible — leak has no build), the claim rests on the
spec author's reading of two non-adjacent code regions. Flagged DEFERRED —
appropriate, but the highest-stakes unverified claim in the spec.
