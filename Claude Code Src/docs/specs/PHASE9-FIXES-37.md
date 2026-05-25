# Phase 9.6c — Spec 37 (Ink UI Shell) Fix Log

**Date**: 2026-05-10
**Source review**: `PHASE9-ADVERSARIAL-37.md` + Phase 9.5 spec-37a/37c findings
**Target**: `docs/specs/37-ink-ui-shell.md`

## Findings applied

### HIGH 1 — React Compiler runtime promoted to core spec (new §10.5)

The Phase 9.5 review of spec 37a noted `react/compiler-runtime` (`_c(N)`
memo-cache symbol) usage, but the finding was confined to the catalog
companion. Verified at:

- `src/components/Spinner.tsx:1` → `import { c as _c } from "react/compiler-runtime";`
- `src/screens/REPL.tsx:1` → same.
- `src/hooks/useTypeahead.tsx` → compiled `_c`/`useMemoCache` slot calls.

Promoted to a new core section **§10.5 React Compiler Runtime**
covering: compiler emit form, build-contract status (not optional),
ANT import-order interaction, React-version coupling, and a deferral
to companions for per-file slot counts. Added a matching
reimplementation-checklist item.

### HIGH 2 — chalk-level singleton mutation promoted to §7

Phase 9.5 spec-37c flagged `ink/colorize.ts` chalk patches but they
were **not** in core §7 Side Effects. Verified at:

- `ink/colorize.ts:21-26` — `boostChalkLevelForXtermJs()` (TERM_PROGRAM=vscode, level 2 → 3).
- `ink/colorize.ts:52-54` — `clampChalkLevelForTmux()` (TMUX set, level >2 → 2).

Promoted as a dedicated §7 bullet covering: load-time global
singleton mutation, exact gate predicates, the `CLAUDE_CODE_TMUX_TRUECOLOR`
escape hatch, and the load-bearing boost-before-clamp order.
Reimplementer-hazard call-out included. Checklist item added.

### MED 3 — Frontmatter cross-link

Frontmatter now points to `37a-components-catalog.md`,
`37b-hooks-catalog.md`, `37c-ink-primitives-catalog.md` so readers
discover the Phase 9 catalog companions without spelunking. Last
updated bumped to 2026-05-10.

### MED 4 — Hooks giants named in §2.4

Added an explicit gap note for `src/hooks/useTypeahead.tsx`
(207KB / 1384 LOC) and `src/hooks/useReplBridge.tsx` (113KB / 722
LOC), with delegation to spec 37b for state-machine detail. Sizes
confirmed via `wc`.

### MED 5 — `src/screens/` reaffirmed

`ls src/screens/` confirmed: `REPL.tsx`, `Doctor.tsx`,
`ResumeConversation.tsx` — exactly three files, no registry.
Codified as a reimplementation-checklist item to forestall reviewers
adding a router/registry layer for parity.

## Verification

All citations verified live in the leaked tree before edit:

- `head -5 src/components/Spinner.tsx` → `_c` import present.
- `head -5 src/screens/REPL.tsx` → `_c` import present.
- `head -5 src/hooks/useTypeahead.tsx` → React/Ink imports (compiled
  slots present further down).
- `sed -n '1,60p' src/ink/colorize.ts` → both functions verbatim, gates
  exact, comments load-bearing.
- `wc -l src/hooks/useTypeahead.tsx src/hooks/useReplBridge.tsx` →
  1384 + 722 lines confirmed.
- `ls src/screens/` → 3 files confirmed.

## Diff scope

Five non-overlapping in-place edits to `docs/specs/37-ink-ui-shell.md`:

1. Frontmatter — cross-link block + date bump.
2. §2.4 — hooks-giants gap note.
3. §7 — chalk-level mutation bullet (~22 lines).
4. New §10.5 — React Compiler runtime (~25 lines).
5. §11 — three new checklist items (compiler runtime, chalk patches,
   `src/screens/` triplet).

No deletions, no renumbering of existing sections. All adversarial
findings from `PHASE9-ADVERSARIAL-37.md` HIGH and MED tiers
addressed.
