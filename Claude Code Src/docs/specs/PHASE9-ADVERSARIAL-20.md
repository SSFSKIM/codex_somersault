# Phase 9.5 Adversarial Review — Spec 20 (Command System)

Reviewer: Opus side, parallel review of 18 specs. Source read-only.

## Severity Counts
- Critical: 0
- High: 0
- Medium: 2
- Low: 4
- Nit: 3

## Top 5 Findings

### 1. [Medium] Symbol-name drift: `remoteControlServerCmd` vs `remoteControlServerCommand`
Spec §5.1 algorithm pseudocode names the local `remoteControlServerCmd`, but the source declares it as `remoteControlServerCommand` (`src/commands.ts:76, 327`). Same line in §8.1 also uses the actual name implicitly via line refs. Cosmetic in pseudocode but inconsistent — a reimplementer following §5.1 verbatim gets a different identifier than the source. Same minor drift: `webCmd` (spec) vs source `webCmd` (matches), `forkCmd`/`buddy`/`peersCmd` all match.

### 2. [Medium] §5.5 algorithm omits await on `loadAllCommands`
Spec line `allCommands = loadAllCommands(cwd)             // memoized` is shown as sync, but source (`commands.ts:477`) is `await loadAllCommands(cwd)`. The memoized return is still a `Promise<Command[]>` (lodash memoizes the promise). Reimplementer following §5.5 literally would get a Promise, not a Command[]. Pseudocode quirk, but worth a note since §3.7 correctly types it `Promise<Command[]>`.

### 3. [Low] §4.2 cache table claims `COMMANDS` and `builtInCommandNames` "not cleared individually" but `clearCommandsCache` doesn't clear them either
Verified: `clearCommandsCache` → `clearCommandMemoizationCaches` clears `loadAllCommands`, `getSkillToolCommands`, `getSlashCommandToolSkills`, `clearSkillIndexCache?.()` (`commands.ts:523-532`), and downstream plugin/skill caches. `COMMANDS()` and `builtInCommandNames()` are NEVER cleared post-startup. Spec acknowledges this, but the implication — that a feature-flag flip or env var change mid-process won't refresh `COMMANDS()` — should be made explicit as an invariant in §11. Currently buried in §4.2.

### 4. [Low] §6.3 comment "currently `summary` is in `INTERNAL_ONLY_COMMANDS` so could be null in non-ANT builds" is slightly wrong
`summary` is a top-level static `import` (`commands.ts:142`), not a feature-gated `null` constant. It is *included* in `INTERNAL_ONLY_COMMANDS` but the import binding itself is non-null in all builds. The actual nullable members of `BRIDGE_SAFE_COMMANDS` would be the feature-gated ones — but none of `compact, clear, cost, summary, releaseNotes, files` is currently feature-gated. The `.filter` is therefore defensive-only / future-proofing. Spec's specific claim about `summary` is inaccurate.

### 5. [Low] §8.1 lists `assistantCommand`'s flag as just `feature('KAIROS')` — verified, but the row "INTERNAL_ONLY_COMMANDS includes subscribePr only when KAIROS_GITHUB_WEBHOOKS" is collapsed into the KAIROS row. Confusing — `subscribePr` requires `KAIROS_GITHUB_WEBHOOKS` alone (`commands.ts:101-103`); `KAIROS` is unrelated to `subscribePr`. The row conflates two independent flags.

## Other Notes (Nits)
- §3.7 says `REMOTE_SAFE_COMMANDS` "~17 entries" — actual count is 17 exactly. Drop the "~".
- §3.7 says `BRIDGE_SAFE_COMMANDS` "~6 entries" — actual is 6 exactly.
- §11 checklist line on `getSkillToolCommands` filter omits the actual filter logic precisely. Source filter (`commands.ts:563-580`): `prompt && !disableModelInvocation && source !== 'builtin' && (loadedFrom ∈ {bundled, skills, commands_DEPRECATED} || hasUserSpecifiedDescription || whenToUse)`. The spec's gloss is correct but the bullet is dense; consider splitting.

## Verified-True Claims
- ANT-only banner at line 1: ✅
- `COMMANDS()` body & ordering: ✅ (lines 258-346)
- `INTERNAL_ONLY_COMMANDS` 29 entries with `.filter(Boolean)`: ✅ (lines 225-254)
- `loadAllCommands` ordering (bundled → builtinPlugin → skillDir → workflow → plugin → pluginSkill → COMMANDS): ✅ (lines 460-468)
- `getSkills` per-source try/catch + outer try/catch: ✅
- `meetsAvailabilityRequirement` not memoized, console excludes 3P + custom base URL: ✅
- `isBridgeSafeCommand` rules (`local-jsx`→false, `prompt`→true, else allowlist): ✅
- Lazy `usageReport` shim with `contentLength: 0`: ✅
- `findCommand` three-way match (name, getCommandName, aliases): ✅
- `getCommand` ReferenceError with sorted available list: ✅
- `formatDescriptionWithSource` ordering (workflow → plugin → builtin/mcp → bundled → settingSourceName): ✅
- `getDynamicSkills` insertion before first built-in: ✅
- Three-kind discriminated union `prompt | local | local-jsx`: ✅
- `commands.ts:1` comment about ANT import-order: ✅

## Verdict
**Ship-ready with cosmetic fixes.** The spec is exceptionally thorough — verbatim type extracts, line-anchored algorithm, full enumeration of feature flags, accurate side-effect inventory. The CLAUDE.md axiom "nothing is auto-discovered" is correctly reflected: every built-in command appears as a static import in `commands.ts`, and the spec correctly distinguishes that from skill/plugin/workflow *dynamic* loading (which IS file-system discovery, but for non-built-in commands). No contradictions with that axiom.

## Cross-Spec Impact
- **17 (Skills)**: Spec correctly defers `getDynamicSkills` trigger logic and `paths` glob activation to 17. Open Q #6 explicitly hands off.
- **21 (Catalog)**: Spec is the registry-layer anchor; per-command behavior deferred. Open Q #1 (insights size) and #10 (commit→INTERNAL split rationale) are 21's job.
- **28 (Plugins)**: `pluginInfo.pluginManifest` shape deferred. Spec correctly references `types/plugin.ts`.
- **34 (Bridge)**: `BRIDGE_SAFE_COMMANDS` set + `isBridgeSafeCommand` predicate documented here; 34 should consume not duplicate.
- **35 (Remote)**: `REMOTE_SAFE_COMMANDS` + `filterCommandsForRemoteMode` documented here.
- **19 (WorkflowTool)**: `getWorkflowCommands` lazy-loaded via `feature('WORKFLOW_SCRIPTS')`; spec correctly hands internals to 19.
- **02 (Settings)**: `SettingSource` type membership deferred (Open Q #2). 02 must enumerate to close Q.
- **23 (MCP)**: `getMcpSkillCommands` defers to MCP spec; `MCP_SKILLS` flag noted.
- **No drift detected** with INDEX.md adjacency claims (17, 21, 28, 31..36).

## Hardest-to-Verify Claim
**§5.3 / §11 invariant: "Insertion order resolves name collisions."** The spec asserts that when names collide between (e.g.) plugin command and built-in, the *first* in the array (plugin) wins. But `findCommand` uses `Array.prototype.find`, which DOES return the first match — verified in source. However, *consumers* of `getCommands()` may iterate differently (e.g., REPL typeahead might sort or dedupe). The spec doesn't (and shouldn't) guarantee downstream consumers respect insertion order. The invariant holds at this layer but is fragile across the wider system. No way to verify exhaustively without reading every consumer (REPL.tsx, query.ts, SkillTool, bridge, remote) — outside the 20-file budget. Listed in §11 as a checklist item but a reimplementer can preserve insertion order locally and still see "wrong" command win downstream.
