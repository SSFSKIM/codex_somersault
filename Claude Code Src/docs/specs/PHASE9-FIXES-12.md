# Phase 9.6c — spec 12 fix log

Source: PHASE9-ADVERSARIAL-12.md (severity counts: 0C / 0H / 3M / 4L / 3N).

This pass applies all 3 Medium findings, 2 of 4 Low findings called out as priority,
and adds the requested out-of-scope pointer for the §5.5 compression hazard.

## Fixes applied

### F1 [Medium] — Symlink semantics undocumented (spec-11 ripple)

- §9.1 (Glob): added bullet — ripgrep args have no `-L` / `--follow` (default
  no-follow); `fs.stat` (not `lstat`) on validation; `extractGlobBaseDirectory`
  does not normalize symlinked baseDirs; cross-link to spec 11 §X shared
  symlink convention.
- §9.2 (Grep): added bullet — same `-L`/`--follow` absence; `Promise.allSettled(fs.stat)`
  follows symlinks for the mtime sort, broken symlinks bucket as mtime 0
  (same path as race-deletion); cross-link to spec 11 §X.

### F2 [Medium] — `--max-columns 500` truncation marker, not silent drop

- §9.2 (Grep): added bullet — `--max-columns 500` emits the literal
  `[Omitted long matching line]` marker (sourced from ripgrep itself, not
  from this codebase) for content mode; ignored for `-l` / `-c` modes.
  Reimplementer warning: changing the cap silently widens/narrows the
  noise band.

### F3 [Medium] — Cross-term scoring coupling (`score === 0` quirk)

- §5.3 (ToolSearch keyword scorer): pseudocode comment now flags
  "(see REIMPLEMENTER HAZARD below)"; added a hazard block after
  `compileTermPatterns` explaining that `score === 0` reads the running
  cross-term accumulator, not a per-term subscore. Refactoring into
  per-term subscores changes ranking on multi-term queries. Treated as
  load-bearing, not a bug — preserve verbatim.

### F4 [Low] — §6.6 `--hidden` initializer position

- §6.6 (CLI ordering): now states explicitly that `args` is **initialized**
  as `['--hidden']` at `GrepTool.ts:330` (position 0), not appended later.
  Reimplementer note added: appending `--hidden` later breaks bit-exactness
  even though ripgrep's flag order is semantically insensitive here.

### F5 [Low] — Glob env defaults `||` vs `??`

- §5.1 pseudocode: inline comment on the two `process.env.* || 'true'`
  lines noting empty-string env vars fall through to default
  (`CLAUDE_CODE_GLOB_HIDDEN=""` is **not** explicit-disable, it equals unset).
- §6.4 constants table: parenthetical added to both env rows describing
  the `||`-not-`??` quirk.

### §5.5 — `checkAutoThreshold` compression

- §5.5 expanded with an explicit OUT-OF-SCOPE POINTER block: shape pseudocode
  for `checkAutoThreshold` (token-count → char-heuristic fallback) and an
  enumerated list of edge cases the spec deliberately does NOT specify
  (GrowthBook overrides, token-count cache key, error swallowing,
  `resolveContextWindow` zero/undefined, partial-failure control flow).
  Reimplementers are now correctly directed to read `utils/toolSearch.ts:385-473`.

## Not addressed (deferred, per task scope)

- L6 `loggedOptimistic` debug-log latch — not in priority list.
- L7 embedded-mode `child.on('error', ...)` ENOENT/EACCES/EPERM bridge — not in priority list.
- L8 cross-spec 09 permission-on-invoke clarification — not in priority list.
- N9, N10, N11 — nits.

## Verification

- All edits applied via Edit tool with success acknowledgment.
- Spec 12 line count grew from 1025 → ~1050; structural sections (§1..§12) intact.
- Cross-spec reference to "spec 11 §X shared symlink section" is forward-pointing;
  spec 11 owns the canonical text.
