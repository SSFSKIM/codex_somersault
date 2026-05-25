# Phase 9.5b — Adversarial review of spec 12 (Search Tools)

Reviewer: Skeptic. Source verified read-only against
`src/tools/{Grep,Glob,ToolSearch}Tool/`, `src/utils/{glob,ripgrep,toolSearch,embeddedTools}.ts`, `src/Tool.ts`.

## Severity counts

| Severity | Count |
|---|---|
| Critical | 0 |
| High | 0 |
| Medium | 3 |
| Low | 4 |
| Nit | 3 |

## Top 5 findings

### F1 [Medium] Symlink behavior is undocumented (Phase 9.6 spec-11 ripple)

Spec 12 never states whether ripgrep follows symlinks during traversal. Source
shows:

- `glob()` (`src/utils/glob.ts:100-107`) and `GrepTool.call` (`GrepTool.ts:330-435`)
  build args without `-L` / `--follow`. ripgrep's documented default is **NOT**
  to follow symlinks. The spec is silent on this — a faithful reimplementer
  could add `--follow` and still claim conformance.
- `Promise.allSettled(fs.stat(...))` for the mtime sort (`GrepTool.ts:529-531`)
  uses `stat` (follows symlinks), not `lstat`. A symlink to a deleted target
  produces ENOENT → silently sorts as mtime 0 (spec only mentions ENOENT for
  files "deleted between rg and stat", not the broken-symlink case).
- `extractGlobBaseDirectory` operates purely on path strings; symlinked
  baseDirs are not normalized. If a user passes
  `pattern='/symlink/**/*.ts'` ripgrep walks the symlink target with the
  symlink path embedded — relative-path emission and `toRelativePath` may
  produce paths that don't round-trip with the user's cwd.

Spec must add a "symlink semantics" line under §5.1, §5.2, §9.1, §9.2.

### F2 [Medium] §5.2 understates `--max-columns 500` truncation behavior

`GrepTool.ts:338` adds `--max-columns 500` unconditionally. With ripgrep's
default behavior, lines longer than 500 cols print a truncation marker
(`[Omitted long matching line]`) — they are NOT silently dropped.

Spec says only:
> "Limit line length to prevent base64/minified content"

It omits:
1. There is **no** `--max-columns-preview` flag; matched-but-truncated lines
   produce a noise marker line.
2. This applies to `content` mode output but is silently ignored for `-l` /
   `-c` modes (rg short-circuits).
3. Reimplementer using a different value (e.g. 1000) would silently widen
   the noise band.

Fix: §6.4 constants table has `500` but §5.2 / §9.2 must call out the marker
behavior.

### F3 [Medium] ToolSearch ranking algorithm has a subtle bug the spec faithfully copies

`ToolSearchTool.ts:278-280`:

```ts
if (parsed.full.includes(term) && score === 0) {
  score += 3
}
```

The `score === 0` guard checks the running total **for the current term loop
iteration**, but `score` accumulates across terms. So term-1's part-match
(+10) prevents term-2's full-name fallback from ever firing. The spec's
pseudocode (§5.3) replicates this exactly:

```
if parsed.full.includes(term) && score === 0: score += 3
```

This is bit-exact, but the spec doesn't flag the cross-term coupling as
intentional vs. accidental. A reimplementer who refactors the loop into
per-term subscores would produce different rankings while believing they
implement the spec.

Spec must annotate this as **intentional cross-term state** or mark it as a
verbatim quirk.

### F4 [Low] §6.6 ordering is not bit-exact for the `args = ['--hidden']` initializer

Spec §6.6 says Grep order begins: `--hidden`, then VCS exclusions, then
`--max-columns 500`. Source confirms (`GrepTool.ts:330` initializer +
`:333-338`). However the spec's §5.2 pseudocode at line 404 writes:

```
args = ['--hidden']
for dir in VCS_DIRECTORIES_TO_EXCLUDE: args.push('--glob', `!${dir}`)
args.push('--max-columns', '500')
```

This matches. But §11 reimplementation checklist item "builds args in the
order in §6.6" doesn't explicitly cite the initial `['--hidden']` literal —
a reimplementer might add `--hidden` at a later position. **Minor**, but for
a "bit-exact" spec it matters.

### F5 [Low] Glob hidden/no-ignore env-default treats empty string as truthy via `||` quirk

`utils/glob.ts:98-99`:

```ts
const noIgnore = isEnvTruthy(process.env.CLAUDE_CODE_GLOB_NO_IGNORE || 'true')
const hidden = isEnvTruthy(process.env.CLAUDE_CODE_GLOB_HIDDEN || 'true')
```

The `||` (not `??`) means setting `CLAUDE_CODE_GLOB_HIDDEN=""` (empty string)
**falls through to `'true'`** (default-on), not "explicitly empty=disabled".
Spec §6.4 says default `'true'` and the env table says `'false'` excludes
hidden — but doesn't note that empty-string is bucketed with unset, which is
asymmetric vs. the documented `isEnvTruthy('false')` behavior. A user
exporting an empty value will not get what `isEnvDefinedFalsy` semantics
suggest. The source comment acknowledges this (`utils/glob.ts:97`); the spec
does not.

## Other findings (Low / Nit)

- **L6 [Low]** §5.5 `isToolSearchEnabledOptimistic` pseudocode omits the
  `loggedOptimistic` one-shot debug latch (`utils/toolSearch.ts:268,
  273-274, 304-314`). §10 telemetry table doesn't list it as an
  observability surface either. Reimplementer who logs every call will spam.

- **L7 [Low]** §9.3 ripgrep critical-error list says `ENOENT|EACCES|EPERM`
  reject. Source confirms (`utils/ripgrep.ts:384-388`), but **the same list
  also applies to embedded-mode `child.on('error', ...)` callback path**
  (`ripgrep.ts:204-211`) where `err: NodeJS.ErrnoException` is propagated as
  `ExecFileException` with the same `.code`. Spec doesn't separate the two
  call paths; if a reimplementer uses spawn for both, they need to manually
  bridge the error shapes.

- **L8 [Low]** Permission interaction with spec 09: §11 says "ToolSearch has
  no permission check". True (`ToolSearchTool` has no `checkPermissions`).
  But ToolSearch's `select:` parser can return tool names that the **target
  tool** later refuses on permission grounds. Spec doesn't note that
  ToolSearch is only a discovery layer — permission for the discovered tool
  is enforced when **invoked**, not during search. Cross-spec to 09 is
  thin.

- **N9 [Nit]** §6.4 says ToolSearch `max_results` default `5`. `inputSchema`
  uses `.default(5)` (`ToolSearchTool.ts:31`) but `call()` also has
  `max_results = 5` fallback (`ToolSearchTool.ts:329`) — the latter is
  defensive (handles cases where Zod default isn't applied, e.g., manual
  call). Spec doesn't mention the redundancy.

- **N10 [Nit]** Glob comment in `GlobTool.ts:149-150` references "UI.tsx:65"
  as a runtime React-compiler cache index. §12 acknowledges this in the
  open-questions list but §3.1.3 still cites "UI.tsx:53" for the cross-tool
  reuse — only one of the two citations can be source-of-truth. They
  reference different lines.

- **N11 [Nit]** §6.6 says "negated ignore patterns from the permission
  context, then plugin-cache exclusions". Source order
  (`GrepTool.ts:412-434`) is correct. But the pseudocode at §5.2 line 432
  writes the appState fetch and ignore-pattern push **after** the glob
  parameter loop, which is correct, but the prose at §6.6 says "expanded
  `--glob <pat>` entries from the `glob` parameter, then negated ignore
  patterns" — verifying. Confirmed match. No bug, just dense.

## Verdict

**ACCEPTABLE WITH FIXES.** Spec 12 is one of the more thorough specs in this
batch — every numeric constant, ripgrep flag, and prompt verbatim matches
source on the reads I performed. The omissions are concentrated in
**boundary behavior**: symlinks (F1), max-column truncation marker (F2),
intentional algorithmic quirks (F3). The bit-exact claim is mostly held; F4
and N10 are the closest things to falsifiers.

A reimplementer following spec 12 verbatim would produce a working tool that
passes a basic conformance test but would diverge from real behavior on:
broken symlinks, very long lines, multi-term keyword-search ranking, and
empty-string env vars. None of these are critical — none break security or
permissions — hence Medium-cap severity.

## Cross-spec impact

- **Spec 09 (Permissions):** §7.6 correctly defers — no leakage. ToolSearch
  has no permission boundary worth specifying. (L8 above is a
  documentation-clarity issue, not a permission gap.)
- **Spec 11 (Read/Edit/Write):** Spec 11 found symlink edge cases per the
  task brief; F1 confirms the same gap exists in spec 12. **Recommend a
  shared "symlink semantics" appendix** referenced from both 11 and 12,
  since both call into the FS layer with `stat`-not-`lstat`.
- **Spec 08 (Tool registry):** §3.4 correctly cites `tools.ts:201, :249`.
  `Tool.searchHint` (Tool.ts:378), `shouldDefer` (:442), `alwaysLoad`
  (:449), `isMcp` (:436) all verified — no drift.
- **Spec 17 (Skills):** §1 OUT-of-scope correctly punts skill discovery
  overlap. No conflict.
- **Spec 23 (MCP):** `appState.mcp.clients[i].type === 'pending'` is touched
  but not specified — correctly punted.
- **Spec 26 (Analytics / GrowthBook):** `tengu_glacier_2xr` flag punted.
  Acceptable.
- **Spec 28 (Plugins):** `getGlobExclusionsForPluginCache` punted. OK.

## Hardest-to-verify claim

§5.5: **"For `'tst-auto'` it calls `checkAutoThreshold`, which prefers the
exact token count (`getDeferredToolTokenCount`) and falls back to a char
heuristic (`calculateDeferredToolDescriptionChars`, threshold = `floor(contextWindow * percentage * CHARS_PER_TOKEN)`)."**

Verifying this requires reading `utils/toolSearch.ts:385-473` AND model
context-window resolution AND the GrowthBook flag override path AND the
fallback chain when token counting fails. The spec compresses 80+ lines of
control flow into one sentence. A reimplementer can't reproduce
`checkAutoThreshold` from this spec alone — they'd need to read source.
This is the only place where the "bit-exact" framing breaks down: it is a
**summary**, not a specification.

Recommend either expanding §5.5 with full pseudocode for `checkAutoThreshold`
or explicitly marking it as a pointer (out-of-scope summary, see source).
