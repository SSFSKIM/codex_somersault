# PHASE 9.7 Agent D — Spec 28 Fix Log

Source trigger: PHASE 9.6 spec 17 fix log (`PHASE9-FIXES-17.md` Finding 9)
flagged spec 28 §5.17 as inconsistent with `src/commands.ts:460-468`.

## Fix 1 — §5.17 Plugin/skill ordering (CONFIRMED FINDING, FIXED)

**Reviewer's claim verified.** Spec 28 §5.17 (formerly lines 633-651) showed
the array-spread order as:

```
...bundledSkills,
...skillDirCommands,
...workflowCommands,
...pluginCommands,
...pluginSkills,
...builtinPluginSkills,    // WRONG: spec placed last
...COMMANDS(),
```

`src/commands.ts:460-468` actually has:

```
...bundledSkills,
...builtinPluginSkills,    // SECOND, not last
...skillDirCommands,
...workflowCommands,
...pluginCommands,
...pluginSkills,
...COMMANDS(),
```

Additional drift uncovered while verifying:
- The destructure form: spec used `[skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills] = await getSkills(cwd)` (array). Source uses `{ skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills } = ...` (object) — `getSkills(cwd)` returns an object literal.
- Citation range was `commands.ts:451-466`; corrected to `commands.ts:449-468`.

**Replacement** (now lines 633-672): inlines the verbatim `commands.ts` snippet,
calls out the second-position placement of `builtinPluginSkills` in prose,
notes that built-in plugin skills emit `source:'bundled', loadedFrom:'bundled'`
per `builtinPlugins.ts:145-150` (cross-checked while verifying), and explains
why the ordering colocates them with bundled skills (first-wins dedup).

Cross-spec impact: spec 17 §5.7 (ordering note) is already consistent with
source post-Phase 9.6. Spec 20 owns the global merge ordering.

## Fix 2 — §12 Open Questions sweep

Applied Phase 9.7 status markings (DEFERRED / RESOLVED Phase 9.7 / NOTE Phase
9.7) to all 9 open questions, mirroring Agent C's pattern for other specs:

| # | Topic | Marking | Reason |
|---|---|---|---|
| 1 | `probeSeedCacheAnyVersion` BYOC behavior | NOTE Phase 9.7 | Source comment confirms; intentional |
| 2 | `getPluginByIdCacheOnly` raw cast | DEFERRED | Implementation not read; residual-risk caveat sufficient |
| 3 | `cachePlugin` signature | DEFERRED | Boundary signature sufficient |
| 4 | `copyPluginToVersionedCache` fallback | NOTE Phase 9.7 | Documented in §5.4 + §11 invariant 8 |
| 5 | No signature/integrity verification | RESOLVED Phase 9.7 | Explicit at §6.22 |
| 6 | No max plugin count | RESOLVED Phase 9.7 | Absent from source — not a defect |
| 7 | 24-variant PluginError vs 12 active | RESOLVED Phase 9.7 | §9.1 enumerates; matches source comment |
| 8 | Cross-marketplace dep closure | DEFERRED | `dependencyResolver.ts` is grep-only |
| 9 | Channel/MCP cross-validation | DEFERRED to spec 23 | Belongs to MCP service spec |

## Fix 3 — Phase 10 ripple gap sweep (new §12.10)

Searched spec 28 for spec 21 / 21d / marketplace UI gaps similar to spec 21's
catalog companion mismatch:

- **21d cross-ref**: spec 28 had only generic "catalog → 21" ref at §1 line 22.
  Added §12.10 confirming spec 28 owns the LOADER and 21d owns the catalog of
  registered plugin commands. No body-level enumeration drift requiring fix
  (spec 28 deliberately defers all catalog detail to 21).
- **Marketplace UI**: spec 28 already lists UI consumers in §2.3
  (`cli/handlers/plugins.ts`, `hooks/useManagePlugins.ts`,
  `hooks/notifs/usePluginInstallationStatus.tsx`). No UI duplication; ownership
  delegated cleanly. §12.10 documents this.
- **`PluginManifest` schema fields**: §6.1 already composes via
  `PluginManifestSchema()` calling `PluginManifestMetadataSchema/HooksSchema/
  CommandsSchema/AgentsSchema/SkillsSchema/OutputStylesSchema/ChannelsSchema/
  McpServerSchema/LspServerSchema/SettingsSchema/UserConfigSchema`. §6.2-6.13
  inline each. NO MISSING FIELDS. §12.10 documents this affirmatively.

## Verify-before-edit notes

- Read `src/commands.ts:440-499` to verify destructure form, ordering, and
  exact line numbers (`449-468`).
- Read `src/plugins/builtinPlugins.ts:130-160` to confirm `source:'bundled',
  loadedFrom:'bundled'` for built-in plugin skills (used in §5.17 prose).
- Confirmed spec 21d uses `userConfig` references but does NOT redocument
  loader internals — clean ownership boundary.

## Cross-spec impact

- Spec 17's "Open Question" flagging of spec 28 §5.17 drift can now be marked
  as RESOLVED in any future Phase 9.x sweep.
- No other spec body needs updating: spec 20 cites the merge ordering at the
  level of "first-wins dedup", which is unaffected by which array position
  `builtinPluginSkills` occupies — the dedup invariant is the same regardless.

## Reviewer-correctness verdict

Codex Phase 9.5 reviewer for spec 17 was **CORRECT**. Spec 28 genuinely had
the ordering and destructure-shape wrong. Both have been fixed.

## Source-read budget

5 sequential reads (under the ~15 budget): commands.ts (×2 narrow ranges),
spec 28 (×4 chunks for full read), builtinPlugins.ts (×1), plus 3 grep-only
checks via Bash for cross-spec references and existing fix-log inventory.
