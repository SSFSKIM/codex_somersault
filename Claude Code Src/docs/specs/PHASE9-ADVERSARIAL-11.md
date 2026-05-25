# Phase 9.5 Adversarial Review — Spec 11 (File Tools)

Reviewer: Opus fallback (codex rate-limited).
Reference: `docs/specs/11-tool-files.md` (1191 lines) vs `src/tools/{FileReadTool,FileWriteTool,FileEditTool,NotebookEditTool}/`.

## Severity Counts

- Critical: 0
- High: 1
- Medium: 3
- Low: 4
- Informational: 3

## Top 5 Findings

### 1. [HIGH] §12 Open Question #2 is a *false alarm* and should be removed

Spec §12.2 (lines 1184–1185) raises a "may be a behavioral bug; flagged for spec 04 review" concern that `Math.min(Infinity, X)` at `toolResultStorage.ts:77` would defeat Read's `maxResultSizeChars: Infinity` carve-out unless `getPersistenceThreshold` special-cases `Infinity`.

It does. `src/utils/toolResultStorage.ts:62-64`:

```ts
if (!Number.isFinite(declaredMaxResultSizeChars)) {
  return declaredMaxResultSizeChars
}
```

The `Math.min` at line 77 is only reached after the early-return for non-finite values. The comment block at lines 59–61 explicitly states this is "checked before the GB override so tengu_satin_quoll can't force it back on." The §11 reimplementation checklist item 2 ("Read sets maxResultSizeChars: Infinity (never persists results to disk)") is correct as written. **§12.2 contradicts §11.2 and should be deleted** — it will mislead a re-implementer into adding redundant guards or re-investigating a non-bug.

### 2. [MEDIUM] NotebookEdit `PROMPT` references `cell_number` — but the input schema has no such field

`src/tools/NotebookEditTool/prompt.ts:3` (verbatim, quoted by spec §6.4.A line 1002):

> The `cell_number` is 0-indexed. Use edit_mode=insert to add a new cell at the index specified by `cell_number`. Use edit_mode=delete to delete the cell at the index specified by `cell_number`.

The input schema (`NotebookEditTool.ts:30-57`, spec §6.4.B) has **no `cell_number` field** — only `cell_id` (string ID, with a `cell-N` numeric fallback parsed by `parseCellId`). This is a stale prompt in `src/`, not a spec error per se — but the spec quotes the prompt verbatim without flagging the inconsistency. A reader trying to invoke the tool by `cell_number` will fail the strict schema. **Recommend adding a §9 edge-case note: "PROMPT contains stale `cell_number` language; the live field is `cell_id` (or `cell-N` numeric form)."**

### 3. [MEDIUM] Edit-tool error code 7 is dual-purpose; spec table conflates two distinct conditions

`FileEditTool.ts:307` (errorCode 7) fires in two situations: (a) `cell_id` provided but matched `cell-N` parses to an out-of-range index (NotebookEdit), and (b) for FileEdit, "File has been modified since read." Spec §6.3.D row 7 (`'File has been modified since read...'`) and spec §6.4.D row 7 (`Cell ID must be specified... | Cell with index ${parsedCellIndex} does not exist...`) document the codes per-tool, but row 7 in §6.4.D bundles two messages under one code without saying which path produces which. Verified: both messages use `errorCode: 7` (lines 265 and 280 of NotebookEditTool.ts). Acceptable, but the table should split: "7a = no cell_id on non-insert; 7b = cell-N parses to out-of-range index."

### 4. [MEDIUM] No coverage of symlink behavior anywhere in the spec

No mention of `fs.lstat` vs `fs.stat`, no discussion of whether reads/writes follow symlinks, no edge case for circular symlinks, dangling symlinks, or symlinks crossing permission boundaries (e.g., a symlink inside cwd pointing outside cwd, defeating `suggestPathUnderCwd`'s `getCwd()` containment intent). The §1 "out of scope" list does not mention symlinks. The `BLOCKED_DEVICE_PATHS` set (§5.1 step 6) covers `/dev/*` and `/proc/<pid>/fd/*` but doesn't address `os.tmpdir()` symlinks (a real macOS issue: `/tmp` → `/private/tmp`). This is a genuine spec gap given the "file safety" framing.

### 5. [LOW] Concurrent-write semantics are claimed atomic but the "critical section" is not actually atomic

§5.2 / §5.3 use the phrase "Critical section — no async between the staleness check and writeTextContent" but the implementation is single-threaded JS, not OS-level atomic. A second process writing between `getFileModificationTime` and `writeTextContent` is undetectable; `writeTextContent` is not atomic-rename. Spec should clarify: the invariant is **intra-process turn-ordering**, not OS atomicity. The phrase "atomic R-M-W" in §3.2 reinforces the misleading framing.

## Verdict

**Accept with edits.** No catastrophic errors. Spec is unusually thorough (1191 lines for 4 tools) and source-faithful: I verified ~20 specific claims (constants, line numbers, error codes, schemas, feature gates) and they all match `src/`. The major action items are: (a) delete the §12.2 false-positive open question, (b) flag the stale `cell_number` prompt language, (c) add symlink edge cases.

## Cross-Spec Impact

- **Spec 04 (turn pipeline / toolResultStorage)**: §12.2's spurious flag should be removed before spec 04 inherits the false claim. No real divergence — `getPersistenceThreshold` already honors `Infinity`.
- **Spec 09 (permissions)**: §12.7 flags NotebookEdit's missing `preparePermissionMatcher`. **Verified**: only Write (`FileWriteTool.ts:132`) and Edit (`FileEditTool.ts:122`) override it; NotebookEdit does not. Spec 09 should document the wildcard-grant divergence — this is a real cross-spec gap.
- **Spec 17 (skills)**: Read/Edit skill discovery is gated by `CLAUDE_CODE_SIMPLE`; **Write is not** (spec §5.2 correctly notes this divergence). Spec 17 should document why Write skips the env gate.
- **Spec 24 (LSP)**: NotebookEdit does not call `lspManager.changeFile/.saveFile` (§7 table). Confirmed in source — `.ipynb` is JSON, not source the LSP cares about. Spec 24 should explicitly exclude notebooks.
- **Spec 41 (file history)**: §6.5 cites `buildLargeToolResultMessage` but spec 41 owns it. Boundary is clean.

## Hardest-to-Verify Claim

**§5.1 image-token estimation factor 0.125 (= 1/8) for base64-encoded image bytes.** Verified literal: `FileReadTool.ts:1137` is `Math.ceil(result.file.base64.length * 0.125)`. But whether this matches Anthropic's actual server-side image tokenization (which depends on resolution, not base64 length) is **unverifiable from this leaked source** — there is no test, no doc, no calibration table. The constant could be off by 2× and the only symptom would be silent over-/under-budget rejections. The spec should mark this as "heuristic, not measured."

## Notes for Re-implementer

- All §6 verbatim assets (prompts, schemas, error strings, constants) verified against source.
- `MAX_EDIT_FILE_SIZE = 1 GiB`, `READ_FILE_STATE_CACHE_SIZE = 100`, `DEFAULT_MAX_CACHE_SIZE_BYTES = 25 MiB`, `DEFAULT_MAX_OUTPUT_TOKENS = 25000`, `MAX_LINES_TO_READ = 2000`, `IPYNB_INDENT = 1`, `MITIGATION_EXEMPT_MODELS = {'claude-opus-4-6'}` — all confirmed verbatim.
- Edit's 11 error codes (0–10) all confirmed at the cited line numbers.
- `IMAGE_EXTENSIONS = {png, jpg, jpeg, gif, webp}` — **confirmed; no SVG, no BMP, no TIFF, no AVIF, no HEIC**. Spec correctly enumerates.
- `shouldDefer: true` on NotebookEditTool confirmed (line 94).
- `Infinity` carve-out for Read confirmed at both `FileReadTool.ts:342` and `toolResultStorage.ts:62-64`.

— end —
