# Phase 9.5b Adversarial Review — Spec 40 (Persistent Memory `memdir/`)

**Reviewer**: Skeptic mode. Source-of-truth: `src/memdir/` (8 files, 1736 LOC verified).
**Verdict**: Spec is **factually accurate** on virtually every cited line, constant, and control-flow branch. One mis-stated cross-spec claim, one missed feature flag, and a handful of small omissions.

## Severity counts

| Severity   | Count |
|------------|-------|
| Critical   | 0     |
| High       | 1     |
| Medium     | 3     |
| Low        | 4     |
| Nit        | 3     |

## Top 5 findings

### F1 (High) — Wrong consumer cross-link for `setCachedClaudeMdContent`
Spec §3 (line ~126) and Reimplementation Checklist §11 last bullet assert: *"when `loadMemoryPrompt` returns non-null, the consumer (system-prompt builder) is expected to call `setCachedClaudeMdContent` with the resolved CLAUDE.md chain"* and *"reimplementer must wire memdir output into the same cache slot."* This is **wrong**. At `src/context.ts:170-185`, `setCachedClaudeMdContent(claudeMd || null)` is called with the **CLAUDE.md chain** value (`shouldDisableClaudeMd ? '' : <claudemd content>`), not memdir's `loadMemoryPrompt` output. The cache slot is owned by spec 05 (CLAUDE.md), not spec 40. The Open Question §12.3 hedges this, but §3 and §11 still state it as contract. Recommend: delete the bullet from §11 and rewrite §3 to say "memdir does **not** populate `setCachedClaudeMdContent`; that cache belongs to spec 05's CLAUDE.md chain."

### F2 (Medium) — Missing `feature('EXTRACT_MEMORIES')` gate in §8
`paths.ts:65` comment explicitly states *"Callers must also gate on `feature('EXTRACT_MEMORIES')` — that check cannot live inside this helper because feature() only tree-shakes when used directly in an `if` condition."* Spec §8 enumerates feature flags but omits `EXTRACT_MEMORIES`. Spec §2.2 ("Feature-flag and ANT guard locations") also misses it. Add a row to §8 and a citation in §2.2.

### F3 (Medium) — `isAutoMemPath` is missing a security-critical detail
Spec §3 documents `isAutoMemPath(absolutePath: string): boolean` as a simple prefix check. Source `paths.ts:274-278` adds `normalize(absolutePath)` *before* the prefix check, with the comment *"SECURITY: Normalize to prevent path traversal bypasses via .. segments."* This normalization is load-bearing for the write carve-out and is not noted in §3, §5, or §11. A reimplementer who copied the spec's signature alone could omit the normalization and re-introduce the bypass.

### F4 (Medium) — `getKairosActive` missing from §2.3 import list
Spec §2.3 lists `bootstrap/state.{getKairosActive,getOriginalCwd,getProjectRoot,getIsNonInteractiveSession}`. That is correct for the union across files but the same line claims `getKairosActive` is imported alongside `getOriginalCwd` from `bootstrap/state`; in `memdir.ts:11` only `getKairosActive, getOriginalCwd` are imported, while `getProjectRoot` is imported in `paths.ts:6` and `getIsNonInteractiveSession` in `paths.ts:5`. The flat list is fine but obscures per-file structure — minor.

### F5 (Low) — `validateMemoryPath` regex transcription
Spec §5.5 writes `/^[A-Za-z]:$/` and the source `paths.ts:142` matches. Spec §5.5 prose says rejects `'C:\\'` → `'C:'` after strip. Source's strip is `.replace(/[/\\]+$/, '')` (i.e. forward and backward slashes) which the spec writes as `/[\/\\]+$/`. Both forms are equivalent; nit only.

## Other findings

- **Low** — Spec §6.3 lists `MEMORY_FRONTMATTER_EXAMPLE` as 261-271 but actual location is `memoryTypes.ts:261-271` (correct) — verified.
- **Low** — Spec §6.5 says `TYPES_SECTION_COMBINED` is 37-106, source matches (`memoryTypes.ts:37-106`); `TYPES_SECTION_INDIVIDUAL` 113-178, matches. Spec accurately notes "duplication is intentional" per source comment at `:9-12`.
- **Low** — Spec §10 lists telemetry fields including `memory_type: 'auto'|'team'|'agent'`. Source confirms: `memdir.ts:297` chooses `'auto' | 'agent'` based on `displayName === AUTO_MEM_DISPLAY_NAME`; `'team'` is fired separately at `:464-467`. Accurate.
- **Nit** — Spec §4.2 says cache "intentionally NOT invalidated on midnight rollover" — source comment (`memdir.ts:329-336`) confirms verbatim.
- **Nit** — Spec §6.13 quotes the team-scope preamble; verified verbatim against `teamMemPrompts.ts:69-74`.
- **Nit** — `memoryShapeTelemetry.ts` is correctly flagged `missing-leaked-source` at §2.1; the lazy require at `findRelevantMemories.ts:69` confirms.

## Cross-spec impact

- **Spec 05 (CLAUDE.md / context)**: F1 must be reconciled. Spec 40's §3 and §11 currently misattribute ownership of the `setCachedClaudeMdContent` cache slot. Spec 05 should be authoritative; spec 40 should only describe what memdir produces (a string from `loadMemoryPrompt`) and delegate cache-injection semantics outward.
- **Spec 29 (extractMemories)**: F2 — spec 40 enumerates 5 GB flags but omits the feature-gate `EXTRACT_MEMORIES` that paths.ts:65 explicitly requires extractMemories callers to add. Spec 29 should pick this up; spec 40 should at least cite it.
- **Spec 41 (session state)**: Spec 40 cleanly delegates session transcripts (`session-memory/`) to spec 41; no conflict observed. The `buildSearchingPastContextSection` correctly references `getProjectDir(getOriginalCwd())` for transcript search, properly scoped.
- **Spec 26 (feature flags)**: Open Q §12.4 correctly defers GB-flag → ANT-IN coupling to spec 26. No issue.

## Auto-extract triggers

Spec correctly states `isExtractModeActive` gates extractMemories agent fork (deferred to spec 29). Source: `paths.ts:69-77` — gate on `tengu_passport_quail`; if interactive return true, else require `tengu_slate_thimble`. **Verified verbatim.** Spec 40 stays out of the extractMemories pipeline and points to spec 29 — correct delegation.

## Hardest-to-verify claim

§5.4's claim that the recall-shape telemetry "fires even on empty selection because selection-rate needs the denominator and `-1` ages distinguish 'ran, picked nothing' from 'never ran'." The `-1` age semantics live in the missing `memoryShapeTelemetry.ts` source (Open Q §12.1). The spec asserts a contract anchor (`logMemoryRecallShape(memories, selected)`) that cannot be confirmed from leaked source — only the call-site signature at `findRelevantMemories.ts:66-72`. A reimplementer must reverse-engineer the `-1` convention, the field shape, and whether selection-rate denominator is `memories.length` or `min(memories.length, 5)`. This is unrecoverable from this tree.

## Summary verdict

Spec 40 is **high-fidelity** at the implementation-detail level — constants, file:line citations, feature flags, control-flow branches, telemetry payloads, and verbatim asset transcriptions all check out against source. The sole substantive defect is the cross-spec mis-attribution of `setCachedClaudeMdContent` (F1), which is internally inconsistent with the spec's own Open Question §12.3. Action items: fix F1, add F2, add the `normalize()` note to F3. Low/nit findings are documentation polish.
