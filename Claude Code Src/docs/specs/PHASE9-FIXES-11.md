# Phase 9.6 B-full — Spec 11 (File Tools) Fix Log

Source review: `docs/specs/PHASE9-ADVERSARIAL-11.md`
Target: `docs/specs/11-tool-files.md`
Severity counts in source review: 0 Critical / 1 High / 3 Medium / 4 Low / 3 Informational.

## Findings applied

### [HIGH] §12 Open Question #2 — false alarm
**Status: APPLIED (resolved in place).**

Verified `src/utils/toolResultStorage.ts:62-64`: `getPersistenceThreshold` early-returns `declaredMaxResultSizeChars` when `!Number.isFinite(...)`. The `Math.min(declared, DEFAULT_MAX_RESULT_SIZE_CHARS)` at line 77 is never reached for Read's `Infinity`. The verbatim guard comment at `:59-61` ("Checked before the GB override so tengu_satin_quoll can't force it back on") confirms intent.

Spec edit: §12 Open Question #2 rewritten to **RESOLVED**, citing the early-return and the guard comment, and explicitly stating that §11.2's "Read sets `maxResultSizeChars: Infinity` (never persists results to disk)" invariant is correct as written. No spec 04 follow-up needed; reviewer's catastrophic-bug worry retracted.

### [MEDIUM] NotebookEdit `cell_number` stale prompt
**Status: APPLIED as a §9 edge case + flagged for `BUGS-IN-SOURCE.md` (Phase 9.7).**

Verified:
- `NotebookEditTool/prompt.ts:3` `PROMPT` references `cell_number` three times.
- Schema (`NotebookEditTool.ts:30-57`) only has `cell_id` (no `cell_number`).
- Additional stale comment at `NotebookEditTool.ts:418` ("validateInput ensures cell_number is in bounds") corroborates the drift.

Spec edit: added a §9 bullet documenting the prompt drift (with verbatim quote noting both `prompt.ts:3` and the line-418 comment) and explicitly tagging it for `BUGS-IN-SOURCE.md`. We do **not** mutate `src/`. A re-implementer should rewrite the prompt to use `cell_id`.

### [MEDIUM] §6.4.D row 7 conflates two error paths
**Status: APPLIED.**

Verified both paths fire `errorCode: 7`:
- `NotebookEditTool.ts:265` — no `cell_id` on non-insert.
- `NotebookEditTool.ts:280` — `cell-N` numeric form parses to out-of-range.

Spec edit: row 7 in §6.4.D split into 7a (no cell_id on non-insert) and 7b (out-of-range `cell-N`), each citing source line. Both retain numeric `errorCode: 7`. §9 bullet on `parseCellId` updated to reference the 7a/7b split.

### [MEDIUM] No symlink coverage anywhere
**Status: APPLIED.**

Verified `grep -nE "symlink|lstat|realpath"` across `src/tools/{FileReadTool,FileWriteTool,FileEditTool,NotebookEditTool}/` returns zero matches. The tools transparently follow symlinks (Node's default `fs.readFile`/`fs.stat` semantics), and `expandPath` does not `realpath`.

Spec edit: §9 bullet added covering five concrete consequences:
1. Reads/writes silently follow symlinks; cwd-suggestion intent is degraded but **not** a permission bypass (perms evaluate the user-supplied path, not the resolved target).
2. `BLOCKED_DEVICE_PATHS` is a literal-string match, so a symlink to `/dev/zero` defeats it (re-implementers should `realpath` first if they care).
3. macOS `/tmp` → `/private/tmp` produces distinct `FileStateCache` entries (cache key is `path.normalize`d, not `realpath`'d).
4. Dangling symlinks → ENOENT (handled by standard not-found branch).
5. Circular symlinks → `EMFILE`/`ELOOP` from `fs.readFile`, surfacing as a generic thrown error.

### [LOW] "Atomic R-M-W" / "critical section" overclaiming
**Status: APPLIED.**

Spec edits:
- §2.1 source-map row for `FileWriteTool.ts`: "atomic R-M-W" → "intra-process turn-ordered R-M-W".
- §5.2 critical-section comment: rewritten to clarify single-threaded JS turn-ordering vs OS-level atomicity, and to note `writeTextContent` is plain truncate-write (not atomic-rename).
- §5.3 critical-section comment: cross-refs the §5.2 caveat.
- §5.5 decision-overview pseudo-diagram: "critical R-M-W" → "R-M-W (intra-process turn-ordered)" (twice — Write and Edit).
- §11.7 reimplementation checklist: expanded to spell out the actual invariant and explicitly disclaim OS-level atomicity.

## Findings skipped

None. All HIGH + MEDIUM + LOW (atomicity framing) items addressed. The 3 Informational items in the review (cross-spec impact, hardest-to-verify claim, re-implementer notes) describe coordinations or non-actionable observations and require no spec-11 edits.

## Top 3 fixes (impact-ordered)

1. **§12.2 RESOLVED** — Removes a misleading "behavioral bug" flag that contradicted §11.2 and would have led re-implementers to add redundant Infinity guards or reopen the question on every cross-spec sweep.
2. **Symlink edge cases (§9)** — Closes a genuine spec gap; clarifies that the device-path filter is bypassable by symlink and that `FileStateCache` keys collide for `/tmp` vs `/private/tmp`.
3. **`cell_number` prompt drift documented + tagged for Phase 9.7** — Prevents re-implementers from copying the stale prompt verbatim; surfaces a real `src/` bug for later remediation.

## Source bugs surfaced (for `BUGS-IN-SOURCE.md` in Phase 9.7)

1. **`src/tools/NotebookEditTool/prompt.ts:3`** — `PROMPT` references the non-existent `cell_number` field (occurs 3×). Schema accepts only `cell_id`. The model can be misled into emitting `{cell_number: ...}` which fails strict-schema validation. Fix: rewrite to use `cell_id` and document the `cell-N` numeric fallback.
2. **`src/tools/NotebookEditTool/NotebookEditTool.ts:418`** — comment "`// validateInput ensures cell_number is in bounds`" repeats the stale wording. Fix: rename in-place to `cell_id`.

(Both are documentation/prompt drift, not logic bugs. No runtime behavior change required to fix.)

## New findings raised during this pass

- **Cache-key collision via symlinks**: `FileStateCache` uses `path.normalize`d keys, not `realpath`'d. On macOS, `/tmp/x` and `/private/tmp/x` produce distinct cache entries even though they are the same inode. A Read of one path followed by a Write of the other would incorrectly trip the "not read yet" guard. Documented in §9; not flagged as a src bug because it interacts with the user's mental model of paths (re-implementers may legitimately want either behavior). Worth a note in Phase 10's enumeration sweep if not already covered.
- **`BLOCKED_DEVICE_PATHS` symlink bypass**: a user could create `~/zero -> /dev/zero` and Read it; the device-path string filter would not match. Not exploitable for permission escalation (still subject to read-permission checks), but defeats the "infinite output" safeguard. Borderline src bug; left as a §9 caveat rather than a `BUGS-IN-SOURCE.md` entry because the stated invariant ("device file would block or produce infinite output") describes a class the literal-string filter cannot fully cover.
