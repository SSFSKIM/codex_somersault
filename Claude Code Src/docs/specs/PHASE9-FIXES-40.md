# PHASE 9.6c Fixes — Spec 40 (Persistent Memory)

Source-of-truth: `src/memdir/{memdir,paths,memoryTypes,teamMemPaths,teamMemPrompts,memoryScan,findRelevantMemories,memoryAge}.ts`, `src/context.ts`. Adversarial review: `PHASE9-ADVERSARIAL-40.md`.

## F1 HIGH — Mis-attributed cache slot to spec 05 (FIXED)

Old draft asserted in §3 and §11 that `loadMemoryPrompt` output is wired into `setCachedClaudeMdContent` (`src/context.ts:176`). Verified against source — that cache holds the assembled CLAUDE.md project-instructions chain (`getClaudeMds(filterInjectedMemoryFiles(await getMemoryFiles()))`), unrelated to memdir. Spec already self-contradicted in §12.3.

Edits:
- §3 closing paragraph rewritten: explicitly disclaims memdir ownership of that cache slot; redirects to spec 05 and to the separate `systemPromptSection('memory', …)` cache.
- §2.4 first bullet rewritten: `src/context.ts` is no longer listed as a memdir consumer; the `:176` line is annotated as belonging to spec 05.
- §11 final checklist item inverted to a NEGATIVE directive ("Do NOT wire …").
- §12.3 open question marked **(resolved)** with the correct ownership.

## F2 MED — Missing `feature('EXTRACT_MEMORIES')` in flag table (FIXED)

`paths.ts:65-67` source comment mandates that callers gate on `feature('EXTRACT_MEMORIES')` because `feature()` only tree-shakes in a direct `if` condition. Spec §8 listed only the inner GB flags.

Edit:
- §8 flag table: added `feature('EXTRACT_MEMORIES')` row above `tengu_passport_quail`. Cross-references spec 29 (fork ownership). Inner GB flag rephrased as the second-stage gate.

## F3 MED (security) — `isAutoMemPath` `normalize()` SECURITY detail (FIXED)

`paths.ts:274-278` calls `normalize(absolutePath)` before `startsWith` with an explicit SECURITY comment about `..`-segment bypass. Original §3 / §5 / §11 elided this. A reimplementer copying the spec verbatim could reintroduce the exact path-traversal flaw the comment was added to prevent.

Edits:
- §3: added inline SECURITY comment to the `isAutoMemPath` signature.
- §5: inserted new subsection 5.7 covering `isAutoMemPath` with verbatim normalize-then-startsWith pseudocode and the attacker rationale. Subsequent §5.7 renumbered to §5.8.
- §11: added a checklist item explicitly preserving the normalize step and tying it to the `filesystem.ts` write carve-out gate.

## F4 MED — `bootstrap/state` import flattening (FIXED)

§2.3 originally listed all four `bootstrap/state` imports as one specifier, hiding the cross-file split. Source verified:
- `memdir.ts:11` → `{getKairosActive, getOriginalCwd}`
- `paths.ts:7` → `{getProjectRoot, getIsNonInteractiveSession}`

Edit:
- §2.3: header reorganized as "Core (cross-file)" + a separate paragraph disambiguating the `bootstrap/state` split with file/line citations.

## F5 LOW — Regex transcription (NOT TAKEN)

`/[\/\\]+$/` vs `/[/\\]+$/` are semantically identical. Source uses the unescaped form (`paths.ts:138`); spec already matches. No edit required.

## Cross-spec ripple

- **Spec 29** (extractMemories): F2 implies spec 29's flag table should also list `feature('EXTRACT_MEMORIES')` as the outer gate above `tengu_passport_quail` / `tengu_slate_thimble`. Flagging only — out of scope for this retry.
- **Spec 05** (CLAUDE.md chain): F1 confirms spec 05 owns the `setCachedClaudeMdContent` cache slot and its lifecycle. No edit needed in spec 40 beyond the disclaimer; spec 05 should retain (and is the right home for) the existing `:176` / `bootstrap/state.ts:1207` documentation.
