# 11 — File Tools (Read / Write / Edit / NotebookEdit) Specification

> The four "file" Tools the model uses to view and mutate the local filesystem. This spec is multi-tool: §3 / §5 / §6 each carry one sub-heading per tool. The shared subsystems (read-before-edit cache, file persistence, image-paste) live in §4 / §6.

Anchor: see `00-overview.md` §6.1 (template), §4 (`FileStateCache`, `MAX_OUTPUT_SIZE` & `Infinity` carve-out at §11), the flag matrix (`FILE_PERSISTENCE`, `NATIVE_CLIPBOARD_IMAGE`).
Tool framework: `08-tool-base-registry.md` (the `Tool<I,O,P>` shape, `buildTool`, `ToolUseContext`).
Permission framework: `09-permission-system.md` (the decision tree itself; this spec only documents what each tool's `checkPermissions` body **calls**).

Adjacent specs (do **not** redocument): 08, 09, 12 (search), 24 (LSP — `getLspServerManager().changeFile/.saveFile`), 37 (UI — `FileEditToolUpdatedMessage`, `HighlightedCode`, `FilePathLink`), 41 (`fileHistoryEnabled` / `fileHistoryTrackEdit`).

---

## 1. Purpose & Scope

These four tools are how the assistant model interacts with the user's filesystem:

- **`Read`** (`FILE_READ_TOOL_NAME = 'Read'`) — returns text/notebook/image/PDF content. Read **always primes the read-before-edit cache** that gates Write/Edit/NotebookEdit.
- **`Write`** (`FILE_WRITE_TOOL_NAME = 'Write'`) — overwrites or creates a file with full content. Edit-preferred for in-place changes.
- **`Edit`** (`FILE_EDIT_TOOL_NAME = 'Edit'`) — single-string substitution (`old_string` → `new_string`), with optional `replace_all`.
- **`NotebookEdit`** (`NOTEBOOK_EDIT_TOOL_NAME = 'NotebookEdit'`) — replaces / inserts / deletes a Jupyter `.ipynb` cell.

In scope:
- Each tool's input/output Zod schema, prompt, validation pipeline, and execution algorithm (verbatim where required).
- The shared `FileStateCache` (`src/utils/fileStateCache.ts`) and read-before-edit gating.
- File-persistence orchestration (`src/utils/filePersistence/filePersistence.ts`, gated by `feature('FILE_PERSISTENCE')`).
- `imagePaste.ts` only the `NATIVE_CLIPBOARD_IMAGE` gates at lines 101 and 132.
- The Read-only `maxResultSizeChars: Infinity` carve-out and the Write/Edit/NotebookEdit `100_000` cap, plus how the path-only preview behaves when persistence kicks in (path-only preview owned by `toolResultStorage.ts`; the divergence is documented here).

Out of scope (pointers only):
- Permission decision tree → spec **09**. (Each `checkPermissions` here just calls `checkReadPermissionForTool` / `checkWritePermissionForTool`.)
- Tool registry, `buildTool`, `ToolUseContext` shape → spec **08**.
- Glob / Grep search → spec **12**.
- LSP file diagnostics — these tools call `getLspServerManager().changeFile/.saveFile/.clearDeliveredDiagnosticsForFile`; the manager itself is **24**.
- UI rendering of diffs and image previews — `<FileEditToolUpdatedMessage>`, `<HighlightedCode>`, `<MessageResponse>` are **37**.
- File history snapshots and session-state file restore — `fileHistory.ts`'s storage layer and its restore pipeline are **41**. This spec only cites where the four tools call `fileHistoryEnabled()` / `fileHistoryTrackEdit(...)`.

---

## 2. Source Map

### 2.1 Owned files (read fully unless noted)

| Path | Purpose | Coverage |
|---|---|---|
| `src/tools/FileReadTool/FileReadTool.ts` | Read entrypoint, schemas, validation, dispatch (text/image/PDF/notebook) | full read |
| `src/tools/FileReadTool/prompt.ts` | Read prompt template + constants | full |
| `src/tools/FileReadTool/limits.ts` | `getDefaultFileReadingLimits()` (env > GrowthBook > defaults) | full |
| `src/tools/FileReadTool/UI.tsx` | Tool-use / result / error rendering, `userFacingName` | full |
| `src/tools/FileReadTool/imageProcessor.ts` | `sharp` / `image-processor-napi` lazy loader | full |
| `src/tools/FileWriteTool/FileWriteTool.ts` | Write entrypoint, intra-process turn-ordered R-M-W, LSP+VSCode notify | full |
| `src/tools/FileWriteTool/prompt.ts` | Write description template | full |
| `src/tools/FileWriteTool/UI.tsx` | Result / error rendering | sampled (404 lines; key entry points cited) |
| `src/tools/FileEditTool/FileEditTool.ts` | Edit entrypoint, validation w/ uniqueness disambiguation | full |
| `src/tools/FileEditTool/prompt.ts` | Edit description (default + ANT variant) | full |
| `src/tools/FileEditTool/utils.ts` | `findActualString`, `preserveQuoteStyle`, `getPatchForEdit(s)`, `applyEditToFile`, `normalizeQuotes`, `getSnippet*`, desanitization, `areFileEditsInputsEquivalent` | full |
| `src/tools/FileEditTool/types.ts` | Zod schemas for input/output + `hunkSchema`, `gitDiffSchema` | full |
| `src/tools/FileEditTool/constants.ts` | Tool name + permission patterns + `FILE_UNEXPECTEDLY_MODIFIED_ERROR` | full |
| `src/tools/FileEditTool/UI.tsx` | `userFacingName`, `renderToolUseMessage`, `renderToolResultMessage`, `renderToolUseRejectedMessage` | sampled (288 lines; behavioral entry points cited) |
| `src/tools/NotebookEditTool/NotebookEditTool.ts` | NotebookEdit entrypoint, replace/insert/delete | full |
| `src/tools/NotebookEditTool/prompt.ts` | Description + `PROMPT` | full |
| `src/tools/NotebookEditTool/constants.ts` | Tool name | full |
| `src/tools/NotebookEditTool/UI.tsx` | Tool-use / result / error rendering | full |
| `src/utils/fileStateCache.ts` | `FileStateCache`, `FileState`, factory, helpers | full |
| `src/utils/filePersistence/filePersistence.ts` | Turn-end file persistence; `isFilePersistenceEnabled()` gates on `feature('FILE_PERSISTENCE')` | full |
| `src/utils/imagePaste.ts:101` and `:132` | `NATIVE_CLIPBOARD_IMAGE` gate (clipboard image fast path) | targeted |
| `src/utils/toolResultStorage.ts:189-199` | Path-only preview format `buildLargeToolResultMessage` (cited; spec **41/04** owns) | targeted |

### 2.2 Imports / Imported by

Imports from (per-tool):
- `src/Tool.ts` (`buildTool`, `ToolDef`, `ToolUseContext`) — spec **08**
- `src/utils/permissions/filesystem.ts` (`checkReadPermissionForTool`, `checkWritePermissionForTool`, `matchingRuleForInput`) — spec **09**
- `src/utils/permissions/PermissionResult.ts`, `permissions/shellRuleMatching.ts` (`matchWildcardPattern`) — spec **09**
- `src/utils/path.ts` (`expandPath`), `src/utils/cwd.ts` (`getCwd`) — spec **42** (utils residual)
- `src/utils/file.ts` (`writeTextContent`, `getFileModificationTime`, `addLineNumbers`, `findSimilarFile`, `suggestPathUnderCwd`, `convertLeadingTabsToSpaces`, `MAX_OUTPUT_SIZE`, `FILE_NOT_FOUND_CWD_NOTE`, `isCompactLinePrefixEnabled`)
- `src/utils/fileRead.ts` (`readFileSyncWithMetadata`, `LineEndingType`)
- `src/utils/fsOperations.ts` (`getFsImplementation`)
- `src/utils/fileHistory.ts` (`fileHistoryEnabled`, `fileHistoryTrackEdit`) — spec **41**
- `src/utils/diff.ts` (`countLinesChanged`, `getPatchFromContents`, `getPatchForDisplay`, `DIFF_TIMEOUT_MS`, `adjustHunkLineNumbers`, `CONTEXT_LINES`)
- `src/utils/notebook.ts`, `src/utils/pdf.ts`, `src/utils/pdfUtils.ts` — readers (residual)
- `src/utils/imageResizer.ts` — resize/compress + `ImageDimensions`/`ImageResizeError`
- `src/utils/readFileInRange.ts` — single async streaming read
- `src/utils/lazySchema.ts`, `src/utils/semanticBoolean.ts`, `src/utils/semanticNumber.ts` — Zod helpers
- `src/services/lsp/manager.ts` (`getLspServerManager`) and `lsp/LSPDiagnosticRegistry.ts` — spec **24**
- `src/services/mcp/vscodeSdkMcp.ts` (`notifyVscodeFileUpdated`) — spec **23**/**34**
- `src/services/teamMemorySync/teamMemSecretGuard.ts` (`checkTeamMemSecrets`) — spec **29**
- `src/services/diagnosticTracking.ts` (`diagnosticTracker.beforeFileEdited`) — spec **26**
- `src/skills/loadSkillsDir.ts` (`discoverSkillDirsForPaths`, `addSkillDirectories`, `activateConditionalSkillsForPaths`) — spec **17**
- `src/services/analytics/growthbook.ts` (`getFeatureValue_CACHED_MAY_BE_STALE`) — spec **26**
- `src/services/tokenEstimation.ts` (`countTokensWithAPI`, `roughTokenCountEstimationForFileType`) — spec **06**
- `src/memdir/memoryAge.ts` (`memoryFreshnessNote`), `src/utils/memoryFileDetection.ts` (`isAutoMemFile`) — spec **40**
- `src/constants/apiLimits.ts` — `PDF_AT_MENTION_INLINE_THRESHOLD`, `PDF_EXTRACT_SIZE_THRESHOLD`, `PDF_MAX_PAGES_PER_READ`
- `src/tools/BashTool/toolName.js` (`BASH_TOOL_NAME`) — used in Read prompt + notebook overflow message
- `src/utils/settings/validateEditTool.ts` (`validateInputForSettingsFileEdit`) — spec **02**

Imported by:
- Registry (`src/tools.ts`) — spec **08**
- Permission filesystem helpers reach back into `FileEditTool/constants.ts` for `CLAUDE_FOLDER_PERMISSION_PATTERN` and `GLOBAL_CLAUDE_FOLDER_PERMISSION_PATTERN` — spec **09**
- Compact / session-state pipelines via `cacheToObject(readFileState)` and `mergeFileStateCaches` — specs **07** / **41**
- The Read tool's `registerFileReadListener(...)` is consumed by services that subscribe to file reads (notably memdir & teamMemorySync); spec **29** owns those listeners.

### 2.3 Feature-flag and ANT guards (this spec)

| Where | Gate | Effect |
|---|---|---|
| `FileEditTool/prompt.ts:17` | `process.env.USER_TYPE === 'ant'` | Adds the `minimalUniquenessHint` line to the Edit description (verbatim in §6.3). |
| `FileReadTool/FileReadTool.ts:540-573` | `getFeatureValue_CACHED_MAY_BE_STALE('tengu_read_dedup_killswitch', false)` | When **not** killed: same-file same-range repeat reads return `'file_unchanged'` stub. |
| `FileReadTool/FileReadTool.ts:730-738` | `MITIGATION_EXEMPT_MODELS = {'claude-opus-4-6'}` | Suppresses `CYBER_RISK_MITIGATION_REMINDER` for that canonical model. |
| `FileReadTool/limits.ts:53-92` | `getFeatureValue_CACHED_MAY_BE_STALE<Partial<FileReadingLimits> \| null>('tengu_amber_wren', {})` | Per-field GrowthBook override for `maxSizeBytes` / `maxTokens` / `includeMaxSizeInPrompt` / `targetedRangeNudge`. |
| `FileReadTool/FileReadTool.ts:578` and `FileEditTool.ts:407` | `isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)` | Skips skill discovery / activation. |
| `FileWriteTool.ts:346-357` and `FileEditTool.ts:546-558` | `isEnvTruthy(process.env.CLAUDE_CODE_REMOTE) && getFeatureValue_CACHED_MAY_BE_STALE('tengu_quartz_lantern', false)` | Computes single-file `gitDiff` and attaches it to the result. |
| `FileEditTool/FileEditTool.ts:84` | none — hard constant | `MAX_EDIT_FILE_SIZE = 1 GiB` (stat bytes) hard cap on editable file size. |
| `NotebookEditTool.ts:116-120` | `feature('TRANSCRIPT_CLASSIFIER')` | When **on**, `toAutoClassifierInput` returns `${path} ${edit_mode}: ${new_source}`; otherwise `''`. |
| `utils/filePersistence/filePersistence.ts:279` | `feature('FILE_PERSISTENCE')` | Combined with `getEnvironmentKind()==='byoc'`, session token, and `CLAUDE_CODE_REMOTE_SESSION_ID`. |
| `utils/imagePaste.ts:101` and `:132` | `feature('NATIVE_CLIPBOARD_IMAGE')` | Native NSPasteboard fast path (~0.03ms warm) instead of `osascript`. Fallback always available. |
| `FileReadTool` prompt `isPDFSupported()` | model-gated | PDF clause in description appears only when supported. |
| `FileReadTool` prompt `getDefaultFileReadingLimits().includeMaxSizeInPrompt` | GrowthBook | Conditional `maxSizeInstruction` clause. |
| `FileReadTool` prompt `targetedRangeNudge` | GrowthBook | Switches `OFFSET_INSTRUCTION_DEFAULT` ⇄ `OFFSET_INSTRUCTION_TARGETED`. |

### 2.4 Source-coverage inventory (verdict)

All four tool dirs and the supporting utilities listed in §2.1 were read fully or in targeted slices sufficient to enumerate every behavioral branch documented below. Open gaps tracked in §12.

---

## 3. Public Interface

Every tool conforms to `Tool<Input, Output>` from `src/Tool.ts` (spec 08); below: each tool's `name`, `searchHint`, `maxResultSizeChars`, `strict`, `shouldDefer`, `userFacingName`, and any optional Tool-interface members it overrides.

### 3.1 `FileReadTool`
- `name`: `'Read'` (`FileReadTool/prompt.ts:5`)
- `searchHint`: `'read files, images, PDFs, notebooks'`
- `maxResultSizeChars`: **`Infinity`** (`FileReadTool.ts:342`) — comment: "Output is bounded by `maxTokens` (`validateContentTokens`). Persisting to a file the model reads back with Read is circular — never persist."
- `strict`: `true`
- `userFacingName(input)` (`UI.tsx:165-173`):
  - `'Reading Plan'` when `file_path.startsWith(getPlansDirectory())`
  - `'Read agent output'` when path matches the agent-output regex (see §5.1)
  - else `'Read'`
- `isReadOnly() => true`; `isConcurrencySafe() => true`; `isSearchOrReadCommand() => { isSearch:false, isRead:true }`
- `getPath({file_path}) => file_path || getCwd()`
- `extractSearchText() => ''` (UI never indexes content)
- Input schema (`FileReadTool.ts:227-243`, verbatim §6.1)
- Output schema (`FileReadTool.ts:248-332`, discriminated union: `text | image | notebook | pdf | parts | file_unchanged`)
- `mapToolResultToToolResultBlockParam` (`FileReadTool.ts:652-717`) handles each output `type` (verbatim §6.1).

### 3.2 `FileWriteTool`
- `name`: `'Write'`
- `searchHint`: `'create or overwrite files'`
- `maxResultSizeChars`: `100_000`
- `strict`: `true`
- `userFacingName` (`UI.tsx`): see §6.2 (`'Write'`/`'Updating Plan'` etc. — sampled).
- `getPath(input) => input.file_path`
- `toAutoClassifierInput(input) => `${input.file_path}: ${input.content}``
- `extractSearchText() => ''`
- Input schema (`FileWriteTool.ts:56-65`, verbatim §6.2)
- Output schema (`FileWriteTool.ts:68-88`): `{ type: 'create' | 'update', filePath, content, structuredPatch, originalFile, gitDiff? }`
- Result content (`FileWriteTool.ts:418-433`):
  - `create`: `'File created successfully at: ' + filePath`
  - `update`: `'The file ' + filePath + ' has been updated successfully.'`

### 3.3 `FileEditTool`
- `name`: `'Edit'`
- `searchHint`: `'modify file contents in place'`
- `maxResultSizeChars`: `100_000`
- `strict`: `true`
- `userFacingName(input)` (`FileEditTool/UI.tsx:24-45`):
  - `'Updated plan'` when path under plans dir
  - `'Update'` when `input.edits != null`
  - `'Create'` when `old_string === ''`
  - else `'Update'`
- `getPath(input) => input.file_path`
- `toAutoClassifierInput(input) => `${input.file_path}: ${input.new_string}``
- `inputsEquivalent` delegates to `areFileEditsInputsEquivalent` from `utils.ts` (semantic equality via re-applying edits).
- Input schema (`FileEditTool/types.ts:6-19`, verbatim §6.3)
- Output schema (`types.ts:63-80`): `{ filePath, oldString, newString, originalFile, structuredPatch, userModified, replaceAll, gitDiff? }`
- Result content (`FileEditTool.ts:575-594`): `replaceAll` ⇒ "All occurrences were successfully replaced." else "has been updated successfully" (verbatim §6.3).

### 3.4 `NotebookEditTool`
- `name`: `'NotebookEdit'`
- `searchHint`: `'edit Jupyter notebook cells (.ipynb)'`
- `maxResultSizeChars`: `100_000`
- `shouldDefer`: **`true`** (`NotebookEditTool.ts:94`) — registry can defer-load this tool.
- `userFacingName` is fixed `'Edit Notebook'`.
- `getPath(input) => input.notebook_path`
- `toAutoClassifierInput(input)` is `''` unless `feature('TRANSCRIPT_CLASSIFIER')` is on, in which case `${notebook_path} ${edit_mode||'replace'}: ${new_source}`.
- Input schema (`NotebookEditTool.ts:30-57`, verbatim §6.4)
- Output schema (`NotebookEditTool.ts:60-85`)
- Result content per `edit_mode` (`NotebookEditTool.ts:133-170`, verbatim §6.4).

---

## 4. Data Model & State

### 4.1 `FileStateCache` (shared across all four tools)

`src/utils/fileStateCache.ts` — verbatim type:

```ts
export type FileState = {
  content: string
  timestamp: number
  offset: number | undefined
  limit: number | undefined
  isPartialView?: boolean
}
```

Cache invariants (`fileStateCache.ts:30-93`):
- `LRUCache<string, FileState>` keyed on `path.normalize(key)` so `/foo/../bar` and mixed `\` / `/` collide.
- `max = READ_FILE_STATE_CACHE_SIZE = 100`.
- `maxSize = DEFAULT_MAX_CACHE_SIZE_BYTES = 25 * 1024 * 1024` (25 MiB).
- `sizeCalculation = value => Math.max(1, Buffer.byteLength(value.content))`.
- `isPartialView=true` is set when an entry was populated by **auto-injection** (not by an explicit `Read` call) and the in-context view is not byte-identical to disk (CLAUDE.md HTML/frontmatter strip; truncated MEMORY.md). Edit/Write **must require an explicit Read** when `isPartialView` is set (Edit/Write validators check this — see §5.2/§5.3).
- `cacheToObject` / `cloneFileStateCache` / `mergeFileStateCaches` exist for compact + session-state pipelines (specs 07, 41). `mergeFileStateCaches` keeps the entry with the larger `timestamp`.

The tool-use context exposes this as `context.readFileState` (shape per spec 08 ToolUseContext).

### 4.2 What writes to `readFileState`

| Tool | Where | Entry written |
|---|---|---|
| Read (text) | `FileReadTool.ts:1032-1037` | `{ content, timestamp: floor(mtimeMs), offset, limit }` |
| Read (notebook) | `FileReadTool.ts:842-847` | same shape, `content = jsonStringify(cells)` |
| Read (image, PDF, parts) | NOT cached. The dedup early-return in §5.1 won't match; comment at `limits.ts:7` and `FileReadTool.ts:528-530` confirms. |
| Write | `FileWriteTool.ts:332-337` | `{ content, timestamp: getFileModificationTime(path), offset: undefined, limit: undefined }` |
| Edit | `FileEditTool.ts:520-525` | `{ content: updatedFile, timestamp: getFileModificationTime(path), offset: undefined, limit: undefined }` |
| NotebookEdit | `NotebookEditTool.ts:437-442` | `{ content: updatedContent, timestamp: getFileModificationTime(path), offset: undefined, limit: undefined }` |

Entries written by Write/Edit/NotebookEdit have `offset === undefined`; the Read-side dedup (§5.1) explicitly skips these because their `timestamp` reflects the post-edit mtime and the `content` is the post-edit content, not what the model saw.

### 4.3 Result-overflow persistence (divergence)

For all tools other than Read, `processToolResultBlock(tool, result, id)` (`utils/toolResultStorage.ts`) wraps `mapToolResultToToolResultBlockParam` with `maybePersistLargeToolResult(...)` using `getPersistenceThreshold(name, tool.maxResultSizeChars)`. The path-only preview message is built by `buildLargeToolResultMessage` (cited verbatim in §6.5).

Read overrides this: `maxResultSizeChars: Infinity` — comment in `FileReadTool.ts:340-342`:
> Output is bounded by maxTokens (validateContentTokens). Persisting to a file the model reads back with Read is circular — never persist.

The Read content cap is enforced **before** the result is built, by `validateContentTokens(content, ext, maxTokens)` (`FileReadTool.ts:755-772`), which throws `MaxFileReadTokenExceededError` when an API token count exceeds `maxTokens`. The ext-aware estimator returns early when `tokenEstimate <= maxTokens / 4` to avoid an API roundtrip on small reads.

### 4.4 File persistence (turn-end)

Module: `src/utils/filePersistence/filePersistence.ts`. Public API:
- `runFilePersistence(turnStartTime, signal?)` — orchestrator (BYOC or Cloud).
- `executeFilePersistence(turnStartTime, signal, onResult)` — error-swallowing wrapper.
- `isFilePersistenceEnabled()` (`:278-287`):

```ts
export function isFilePersistenceEnabled(): boolean {
  if (feature('FILE_PERSISTENCE')) {
    return (
      getEnvironmentKind() === 'byoc' &&
      !!getSessionIngressAuthToken() &&
      !!process.env.CLAUDE_CODE_REMOTE_SESSION_ID
    )
  }
  return false
}
```

BYOC mode (`:150-240`) scans `{cwd}/{sessionId}/outputs` for files modified after `turnStartTime`, drops anything whose `relative()` starts with `..`, and uploads with `DEFAULT_UPLOAD_CONCURRENCY` to the Files API. Cloud mode is currently a no-op stub. Hard cap `FILE_COUNT_LIMIT` rejects the entire batch if exceeded; the failure object is `{ filename: outputsDir, error: 'Too many files modified (N). Maximum: K.' }`. `tengu_file_persistence_started`, `_completed`, `_limit_exceeded` events fire (spec 26).

### 4.5 `imagePaste.ts` flag points (only)

This spec covers only the `NATIVE_CLIPBOARD_IMAGE` gates in clipboard image lookup (used by paste, not by the Read tool). `imagePaste.ts:101` and `:132` both wrap a `getNativeModule()?.hasClipboardImage` / pixel-reader fast path; failure at any layer falls through to `osascript -e 'the clipboard as «class PNGf»'`. Full paste behavior (event integration, terminal pasting) belongs to spec 37; ownership of the file as a whole is spec 42.

---

## 5. Algorithm / Control Flow

Every tool's call() must be read together with its `validateInput`. `validateInput` is purely logical (no `writeTextContent`); `call` performs filesystem mutation. Both paths share `expandPath(file_path)` from `utils/path.ts` to normalize `~`, relatives, and Windows `\`.

### 5.1 `FileReadTool.call` (text/notebook/image/PDF dispatch)

`validateInput` (FileReadTool.ts:418-495):
1. If `pages !== undefined`, run `parsePDFPageRange(pages)`. On null, error `errorCode:7` (verbatim §6.1.E). If span > `PDF_MAX_PAGES_PER_READ`, error `errorCode:8`.
2. `fullFilePath = expandPath(file_path)`.
3. Deny-rule check: `matchingRuleForInput(fullFilePath, ctx, 'read', 'deny')`. On match → `result:false, errorCode:1` (verbatim).
4. UNC short-circuit: paths starting `\\` or `//` skip filesystem operations and return `result:true` (NTLM credential leak prevention; same pattern in Edit/Write/NotebookEdit).
5. Binary extension check (`hasBinaryExtension(...)` minus PDF + image allowlist): error `errorCode:4`.
6. Blocked device path check (path-only set: `/dev/zero`, `/dev/random`, `/dev/urandom`, `/dev/full`, `/dev/stdin`, `/dev/tty`, `/dev/console`, `/dev/stdout`, `/dev/stderr`, `/dev/fd/{0,1,2}`, plus `/proc/self/fd/0-2` and `/proc/<pid>/fd/0-2`): error `errorCode:9`.

`call` outer (FileReadTool.ts:496-651):

```
defaults = getDefaultFileReadingLimits()
maxSizeBytes = ctx.fileReadingLimits?.maxSizeBytes ?? defaults.maxSizeBytes
maxTokens    = ctx.fileReadingLimits?.maxTokens    ?? defaults.maxTokens
ext = path.extname(file_path).toLowerCase().slice(1)
fullFilePath = expandPath(file_path)

# Dedup (early-return) — only when killswitch off, and existing entry is from a prior Read
if !tengu_read_dedup_killswitch:
  existing = readFileState.get(fullFilePath)
  if existing && !existing.isPartialView && existing.offset !== undefined &&
     existing.offset === offset && existing.limit === limit:
     mtimeMs = await getFileModificationTimeAsync(fullFilePath)
     if mtimeMs === existing.timestamp:
        logEvent('tengu_file_read_dedup', {ext?})
        return { data: { type:'file_unchanged', file:{ filePath: file_path } } }

# Skill discovery (skip if CLAUDE_CODE_SIMPLE)
if !env('CLAUDE_CODE_SIMPLE'):
  newSkillDirs = await discoverSkillDirsForPaths([fullFilePath], cwd)
  for d in newSkillDirs: ctx.dynamicSkillDirTriggers?.add(d)
  addSkillDirectories(newSkillDirs).catch(()=>{})    # fire-and-forget
  activateConditionalSkillsForPaths([fullFilePath], cwd)

try: return await callInner(file_path, fullFilePath, fullFilePath, ext, offset, limit, pages, maxSizeBytes, maxTokens, readFileState, ctx, parentMessage?.message.id)
catch e if errno=='ENOENT':
   altPath = getAlternateScreenshotPath(fullFilePath)   # AM/PM thin-space heuristic
   if altPath: try: return await callInner(... altPath ...) ; catch ENOENT: fall through
   similar = findSimilarFile(fullFilePath); cwdSug = await suggestPathUnderCwd(fullFilePath)
   msg = `File does not exist. ${FILE_NOT_FOUND_CWD_NOTE} ${getCwd()}.`
   if cwdSug: msg += ` Did you mean ${cwdSug}?`
   else if similar: msg += ` Did you mean ${similar}?`
   throw new Error(msg)
```

`callInner` dispatch (FileReadTool.ts:804-1086):

- **`ext === 'ipynb'`** (notebook): `cells = await readNotebook(...)`, `cellsJson = jsonStringify(cells)`. If `Buffer.byteLength(cellsJson) > maxSizeBytes`, throw the multi-line jq error (verbatim §6.1.F). Else `validateContentTokens(cellsJson, ext, maxTokens)`. Cache `{content: cellsJson, timestamp: floor(mtimeMs), offset, limit}`. Add to `nestedMemoryAttachmentTriggers`. Return `{type:'notebook', file:{filePath, cells}}`.
- **`IMAGE_EXTENSIONS.has(ext)`** (`png|jpg|jpeg|gif|webp`): call `readImageWithTokenBudget(resolvedFilePath, maxTokens)` (single read; see below). Return `{type:'image', file:{base64, type, originalSize, dimensions?}}`. If `dimensions`, attach a meta user-message via `createImageMetadataText(...)`.
- **`isPDFExtension(ext)`**:
  - If `pages` provided: `parsedRange = parsePDFPageRange(pages)`, `await extractPDFPages(file, parsedRange)`. On failure, throw the underlying error message. Read every `.jpg` from `outputDir` (sorted), resize each via `maybeResizeAndDownsampleImageBuffer(..., 'jpeg')`, and inject as a meta message of `type: 'image'` blocks. Logs `tengu_pdf_page_extraction { success:true, pageCount, fileSize, hasPageRange:true }`.
  - Else: `pageCount = await getPDFPageCount(...)`. If `> PDF_AT_MENTION_INLINE_THRESHOLD (10)`, throw the "too many pages" error (verbatim §6.1.F).
  - `shouldExtractPages = !isPDFSupported() || stats.size > PDF_EXTRACT_SIZE_THRESHOLD (3 MB)`.
  - If `!isPDFSupported()`: throw the "Reading full PDFs is not supported with this model..." error (verbatim §6.1.F).
  - Else `readPDF(...)` and inject the doc as a meta user-message of `type:'document'` source `application/pdf` base64. Return `{type:'pdf', file:{filePath, base64, originalSize}}`.
- **Text default** (FileReadTool.ts:1019-1085): `lineOffset = offset===0 ? 0 : offset-1`. Single async `readFileInRange(resolvedFilePath, lineOffset, limit, limit===undefined ? maxSizeBytes : undefined, abortSignal)`. Then `validateContentTokens(...)`, cache `{content, floor(mtimeMs), offset, limit}`, add to `nestedMemoryAttachmentTriggers`, broadcast to the **snapshot** of `fileReadListeners` (snapshot before iterating to avoid splice-skip), build the `'text'` output, set `memoryFileMtimes` if `isAutoMemFile(...)`. Logs `tengu_session_file_read` with `is_session_memory` / `is_session_transcript` set per `detectSessionFileType` (filename-based: `~/.claude/session-memory/*.md`; `~/.claude/projects/*/*.jsonl`).

`readImageWithTokenBudget` (FileReadTool.ts:1097-1183):
1. `readFileBytes(filePath, maxBytes)` — single read.
2. Empty buffer ⇒ throw `'Image file is empty: '+filePath`.
3. `detectImageFormatFromBuffer(...)` → `image/<format>`. Default `'png'` when split fails.
4. `maybeResizeAndDownsampleImageBuffer(buffer, originalSize, format)` (try/catch — re-throw `ImageResizeError`; otherwise log + use raw buffer).
5. `estimatedTokens = ceil(base64.length * 0.125)`. If over budget, attempt `compressImageBufferWithTokenLimit(buffer, maxTokens, mediaType)`; on failure, fallback `sharp(buffer).resize(400,400,{fit:'inside',withoutEnlargement:true}).jpeg({quality:20})`; ultimate fallback returns the original buffer at the detected format.

`mapToolResultToToolResultBlockParam` (FileReadTool.ts:652-717):
- `image` ⇒ `tool_result` with a single `image` block (`source.type: 'base64'`).
- `notebook` ⇒ `mapNotebookCellsToToolResult(cells, id)` (per spec 42 utils).
- `pdf` ⇒ string `'PDF file read: '+filePath+' ('+formatFileSize(originalSize)+')'`.
- `parts` ⇒ string `'PDF pages extracted: '+count+' page(s) from '+filePath+' ('+formatFileSize(originalSize)+')'`.
- `file_unchanged` ⇒ `FILE_UNCHANGED_STUB` (verbatim §6.1.G).
- `text` ⇒ if non-empty: `memoryFileFreshnessPrefix(data) + addLineNumbers(file) + (shouldIncludeFileReadMitigation() ? CYBER_RISK_MITIGATION_REMINDER : '')`. If empty content, `<system-reminder>` warning either "exists but the contents are empty" or "shorter than the provided offset (N). The file has K lines." (verbatim §6.1.G).

`shouldIncludeFileReadMitigation()`: `!MITIGATION_EXEMPT_MODELS.has(getCanonicalName(getMainLoopModel()))`; exempt set = `{ 'claude-opus-4-6' }`.

**Permission body** (`FileReadTool.ts:398-405`): `await checkReadPermissionForTool(FileReadTool, input, appState.toolPermissionContext)` (decision tree in spec 09).

### 5.2 `FileWriteTool.call`

`validateInput` (FileWriteTool.ts:153-222):
1. `expandPath` ⇒ `fullFilePath`.
2. `checkTeamMemSecrets(fullFilePath, content)` ⇒ if non-null, error code 0 (spec 29 owns secret-detection rules; the message string is whatever that helper returns).
3. Deny-rule for `'edit'`: error code 1 (string verbatim §6.2.D).
4. UNC ⇒ skip filesystem ops, return `{result:true}`.
5. `await fs.stat(fullFilePath)`; on `ENOENT` ⇒ `{result:true}` (new file). On other error: rethrow.
6. `readTimestamp = readFileState.get(fullFilePath)`. If absent OR `isPartialView` ⇒ error code 2: **"File has not been read yet. Read it first before writing to it."**
7. `lastWriteTime = floor(stat.mtimeMs)`. If `> readTimestamp.timestamp` ⇒ error code 3: **"File has been modified since read, either by the user or by a linter. Read it again before attempting to write it."**

Note: Validate uses the cached `lastWriteTime` from the `stat` it already did, NOT a redundant `getFileModificationTime` call.

`call` (FileWriteTool.ts:223-417):

```
fullFilePath = expandPath(file_path); dir = dirname(fullFilePath)

# Skill discovery (no CLAUDE_CODE_SIMPLE gate here; only Read/Edit have it)
newDirs = await discoverSkillDirsForPaths([fullFilePath], cwd)
for d in newDirs: dynamicSkillDirTriggers?.add(d)
addSkillDirectories(newDirs).catch(()=>{})
activateConditionalSkillsForPaths([fullFilePath], cwd)

await diagnosticTracker.beforeFileEdited(fullFilePath)
await fs.mkdir(dir)                                       # OUTSIDE the critical section
if fileHistoryEnabled():
   await fileHistoryTrackEdit(updateFileHistoryState, fullFilePath, parentMessage.uuid)

# Intra-process critical section — no `await` between the staleness check and writeTextContent.
# This is single-threaded JS turn-ordering, NOT OS-level atomicity: a concurrent
# external writer between getFileModificationTime and writeTextContent is undetectable,
# and writeTextContent is plain truncate-write (not atomic-rename).
try meta = readFileSyncWithMetadata(fullFilePath)
catch ENOENT: meta = null

if meta !== null:
   lastWriteTime = getFileModificationTime(fullFilePath)
   lastRead = readFileState.get(fullFilePath)
   if !lastRead || lastWriteTime > lastRead.timestamp:
      isFullRead = lastRead && lastRead.offset===undefined && lastRead.limit===undefined
      # On Windows mtime can change without content change (cloud sync, AV).
      # For full reads, fall back to content compare; meta.content is CRLF-normalized.
      if !isFullRead || meta.content !== lastRead.content:
         throw new Error(FILE_UNEXPECTEDLY_MODIFIED_ERROR)

enc = meta?.encoding ?? 'utf8'; oldContent = meta?.content ?? null
writeTextContent(fullFilePath, content, enc, 'LF')        # ALWAYS write LF (no preserve)

# LSP didChange + didSave (best-effort)
clearDeliveredDiagnosticsForFile(`file://${fullFilePath}`)
lspManager.changeFile(...).catch(...)
lspManager.saveFile(...).catch(...)
notifyVscodeFileUpdated(fullFilePath, oldContent, content)

readFileState.set(fullFilePath, { content, timestamp: getFileModificationTime(fullFilePath), offset: undefined, limit: undefined })
if fullFilePath.endsWith(`${sep}CLAUDE.md`): logEvent('tengu_write_claudemd', {})

if env('CLAUDE_CODE_REMOTE') && gb('tengu_quartz_lantern', false):
   gitDiff = await fetchSingleFileGitDiff(fullFilePath)
   logEvent('tengu_tool_use_diff_computed', { isWriteTool:true, durationMs, hasDiff: !!diff })

if oldContent:
   patch = getPatchForDisplay({ filePath, fileContents: oldContent, edits:[{old_string:oldContent, new_string:content, replace_all:false}] })
   countLinesChanged(patch)
   logFileOperation({operation:'write', tool:'FileWriteTool', filePath, type:'update'})
   return { type:'update', filePath, content, structuredPatch:patch, originalFile:oldContent, gitDiff? }

# create
countLinesChanged([], content)
logFileOperation({operation:'write', tool:'FileWriteTool', filePath, type:'create'})
return { type:'create', filePath, content, structuredPatch:[], originalFile:null, gitDiff? }
```

Critical invariant (verbatim comment): line endings are NOT preserved from disk; `writeTextContent(... , 'LF')` always writes LF. Earlier behavior preserved old endings or sampled the repo via ripgrep, which silently corrupted bash scripts and CRLF↔LF conversions.

**Permission body** (`FileWriteTool.ts:135-142`): `await checkWritePermissionForTool(FileWriteTool, input, appState.toolPermissionContext)` (decision tree in spec 09). `preparePermissionMatcher({file_path}) => pattern => matchWildcardPattern(pattern, file_path)`.

### 5.3 `FileEditTool.call`

The validator is the algorithmically richest of the four (`FileEditTool.ts:137-362`). Pseudocode:

```
{ file_path, old_string, new_string, replace_all = false } = input
fullFilePath = expandPath(file_path)

secretError = checkTeamMemSecrets(fullFilePath, new_string)
if secretError: return {result:false, message:secretError, errorCode:0}

if old_string === new_string:
   return {result:false, behavior:'ask', errorCode:1,
     message:'No changes to make: old_string and new_string are exactly the same.'}

deny = matchingRuleForInput(fullFilePath, ctx, 'edit', 'deny')
if deny: return {result:false, behavior:'ask', errorCode:2,
   message:'File is in a directory that is denied by your permission settings.'}

# UNC short-circuit
if startsWith('\\\\') || startsWith('//'): return {result:true}

# Hard 1 GiB stat-bytes cap (V8/Bun string limit guard)
try { size } = await fs.stat(fullFilePath)
   if size > MAX_EDIT_FILE_SIZE: return {result:false, behavior:'ask', errorCode:10,
      message:`File is too large to edit (${formatFileSize(size)}). Maximum editable file size is ${formatFileSize(MAX_EDIT_FILE_SIZE)}.`}
catch ENOENT: pass

# Read bytes (sniff BOM for utf16le, else utf8); CRLF→LF normalize content
buf = await fs.readFileBytes(fullFilePath)
encoding = (len>=2 && buf[0]==0xff && buf[1]==0xfe) ? 'utf16le' : 'utf8'
fileContent = buf.toString(encoding).replaceAll('\r\n','\n')   # null on ENOENT

if fileContent === null:
   if old_string === '': return {result:true}      # new file via Edit
   sim = findSimilarFile(...); cwdSug = await suggestPathUnderCwd(...)
   msg = `File does not exist. ${FILE_NOT_FOUND_CWD_NOTE} ${getCwd()}.`
   if cwdSug: msg += ` Did you mean ${cwdSug}?`; else if sim: msg += ` Did you mean ${sim}?`
   return {result:false, behavior:'ask', errorCode:4, message: msg}

if old_string === '':
   if fileContent.trim() !== '':
      return {result:false, behavior:'ask', errorCode:3,
         message:'Cannot create new file - file already exists.'}
   return {result:true}        # empty file overwrite

if fullFilePath.endsWith('.ipynb'):
   return {result:false, behavior:'ask', errorCode:5,
     message:`File is a Jupyter Notebook. Use the ${NOTEBOOK_EDIT_TOOL_NAME} to edit this file.`}

readTimestamp = readFileState.get(fullFilePath)
if !readTimestamp || readTimestamp.isPartialView:
   return {result:false, behavior:'ask', errorCode:6,
     message:'File has not been read yet. Read it first before writing to it.',
     meta:{isFilePathAbsolute:String(isAbsolute(file_path))}}

if readTimestamp:
   lastWriteTime = getFileModificationTime(fullFilePath)
   if lastWriteTime > readTimestamp.timestamp:
      isFullRead = readTimestamp.offset===undefined && readTimestamp.limit===undefined
      if !(isFullRead && fileContent === readTimestamp.content):
         return {result:false, behavior:'ask', errorCode:7,
           message:'File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.'}

# Quote-normalized search
actualOldString = findActualString(fileContent, old_string)
if !actualOldString:
   return {result:false, behavior:'ask', errorCode:8,
     message:`String to replace not found in file.\nString: ${old_string}`,
     meta:{isFilePathAbsolute:String(isAbsolute(file_path))}}

matches = file.split(actualOldString).length - 1   # = count of occurrences
if matches > 1 && !replace_all:
   return {result:false, behavior:'ask', errorCode:9,
     message:`Found ${matches} matches of the string to replace, but replace_all is false. To replace all occurrences, set replace_all to true. To replace only one occurrence, please provide more context to uniquely identify the instance.\nString: ${old_string}`,
     meta:{isFilePathAbsolute:..., actualOldString}}

# Settings-file extra validation (spec 02)
settingsResult = validateInputForSettingsFileEdit(fullFilePath, file,
   simulate = () => replace_all
       ? file.replaceAll(actualOldString, new_string)
       : file.replace(actualOldString, new_string))
if settingsResult !== null: return settingsResult

return {result:true, meta:{actualOldString}}
```

`call` (FileEditTool.ts:387-595):

```
fs = getFsImplementation(); absoluteFilePath = expandPath(file_path)
# Skill discovery (gated by CLAUDE_CODE_SIMPLE)
await diagnosticTracker.beforeFileEdited(absoluteFilePath)
await fs.mkdir(dirname(absoluteFilePath))             # OUTSIDE critical section
if fileHistoryEnabled(): await fileHistoryTrackEdit(updateFileHistoryState, absoluteFilePath, parentMessage.uuid)

# Intra-process critical section — synchronous between staleness check and writeTextContent.
# Same caveat as §5.2: single-threaded JS turn-ordering, not OS atomicity.
{ content: original, fileExists, encoding, lineEndings: endings } = readFileForEdit(absoluteFilePath)
   # readFileSyncWithMetadata; on ENOENT returns {'',false,'utf8','LF'}

if fileExists:
   lastWriteTime = getFileModificationTime(absoluteFilePath)
   lastRead = readFileState.get(absoluteFilePath)
   if !lastRead || lastWriteTime > lastRead.timestamp:
      isFullRead = lastRead && lastRead.offset===undefined && lastRead.limit===undefined
      contentUnchanged = isFullRead && original === lastRead.content
      if !contentUnchanged: throw new Error(FILE_UNEXPECTEDLY_MODIFIED_ERROR)

actualOldString = findActualString(original, old_string) || old_string
actualNewString = preserveQuoteStyle(old_string, actualOldString, new_string)
{ patch, updatedFile } = getPatchForEdit({ filePath:absoluteFilePath, fileContents:original,
   oldString:actualOldString, newString:actualNewString, replaceAll:replace_all })
writeTextContent(absoluteFilePath, updatedFile, encoding, endings)
# LSP didChange + didSave; notifyVscodeFileUpdated; readFileState.set(...)
if absoluteFilePath.endsWith(`${sep}CLAUDE.md`): logEvent('tengu_write_claudemd',{})
countLinesChanged(patch)
logFileOperation({operation:'edit', tool:'FileEditTool', filePath:absoluteFilePath})
logEvent('tengu_edit_string_lengths', { oldStringBytes, newStringBytes, replaceAll })
# Optional gitDiff under CLAUDE_CODE_REMOTE && gb('tengu_quartz_lantern')
return { filePath, oldString:actualOldString, newString:new_string, originalFile:original,
        structuredPatch:patch, userModified: userModified ?? false, replaceAll:replace_all, gitDiff? }
```

Encoding/endings preservation (Edit only — Write is always LF):
- `readFileSyncWithMetadata` returns `{content, encoding, lineEndings}`. Edit threads `encoding`/`endings` straight back into `writeTextContent(...)`.

`getPatchForEdit` → `getPatchForEdits` (utils.ts:234-350) — the string-replace algorithm:

```
updatedFile = fileContents
appliedNewStrings = []

# Special case: empty file + single empty-edit
if !fileContents && edits.length===1 && edits[0].old_string==='' && edits[0].new_string==='':
   patch = getPatchForDisplay(filePath, fileContents, [{old_string:fileContents, new_string:updatedFile, replace_all:false}])
   return { patch, updatedFile:'' }

for edit in edits:
   oldStringToCheck = edit.old_string.replace(/\n+$/, '')   # strip trailing \n for substring guard
   for prev in appliedNewStrings:
      if oldStringToCheck !== '' and prev.includes(oldStringToCheck):
         throw new Error('Cannot edit file: old_string is a substring of a new_string from a previous edit.')
   prevContent = updatedFile
   updatedFile = (edit.old_string === '')
                  ? edit.new_string
                  : applyEditToFile(updatedFile, edit.old_string, edit.new_string, edit.replace_all)
   if updatedFile === prevContent:
      throw new Error('String not found in file. Failed to apply edit.')
   appliedNewStrings.push(edit.new_string)

if updatedFile === fileContents:
   throw new Error('Original and edited file match exactly. Failed to apply edit.')

patch = getPatchFromContents({ filePath, oldContent:convertLeadingTabsToSpaces(fileContents),
                               newContent:convertLeadingTabsToSpaces(updatedFile) })
return { patch, updatedFile }
```

`applyEditToFile` (utils.ts:206-228):

```
f = replaceAll
   ? (c,s,r) => c.replaceAll(s, () => r)        # callback form prevents $ replacement-string semantics
   : (c,s,r) => c.replace(s, () => r)

if newString !== '': return f(originalContent, oldString, newString)

# Empty newString = deletion. If oldString does NOT end with '\n' but the file
# has '\n' immediately after the match, swallow that newline so deletion doesn't
# leave a blank line.
stripTrailingNewline = !oldString.endsWith('\n') && originalContent.includes(oldString + '\n')
return stripTrailingNewline ? f(originalContent, oldString + '\n', newString)
                            : f(originalContent, oldString, newString)
```

`findActualString` (utils.ts:73-93): exact `includes(searchString)` first; else normalize curly→straight in BOTH file and search and `indexOf` in normalized file; if found, return `fileContent.substring(idx, idx + searchString.length)` — i.e. the **original-style** slice, preserving curly quotes.

`preserveQuoteStyle` (utils.ts:104-199): If `oldString===actualOldString`, no normalization happened — return `newString`. Else inspect `actualOldString` for `LEFT/RIGHT_DOUBLE_CURLY_QUOTE` / `LEFT/RIGHT_SINGLE_CURLY_QUOTE` and re-encode the matching `"` / `'` in `newString` using an open/close heuristic (`isOpeningContext`: prev char in `' ', '\t', '\n', '\r', '(', '[', '{', U+2014 em dash, U+2013 en dash`). For `'`, contractions (letter-`'`-letter, Unicode `\p{L}`) always use `RIGHT_SINGLE_CURLY_QUOTE`.

`normalizeFileEditInput` (utils.ts:581-657) is a multi-edit normalizer used by other code paths (not called directly by the single-edit `FileEditTool.call`): for `.md`/`.mdx` the trailing whitespace strip is **skipped** (markdown two-trailing-space hard line break would silently change semantics); else `stripTrailingWhitespace`. If exact `old_string` match fails, it tries `desanitizeMatchString` against the table at utils.ts:531-549 (`<fnr>→<function_results>`, `<n>→<name>`, `\n\nH:→\n\nHuman:`, `\n\nA:→\n\nAssistant:`, etc.).

`areFileEditsInputsEquivalent` (utils.ts:732-775): fast path same-path + literal-equal; else read file (or `''` on ENOENT) and apply both edit lists, compare results.

**Permission body** (`FileEditTool.ts:125-132`): same `checkWritePermissionForTool` shape as Write.

### 5.4 `NotebookEditTool.call`

`validateInput` (NotebookEditTool.ts:176-294):

```
fullPath = isAbsolute(notebook_path) ? notebook_path : resolve(getCwd(), notebook_path)
if startsWith('\\\\') || startsWith('//'): return {result:true}
if extname(fullPath) !== '.ipynb': errorCode 2 — "File must be a Jupyter notebook (.ipynb file). For editing other file types, use the FileEdit tool."
if edit_mode not in {'replace','insert','delete'}: errorCode 4 — "Edit mode must be replace, insert, or delete."
if edit_mode === 'insert' && !cell_type: errorCode 5 — "Cell type is required when using edit_mode=insert."
readTimestamp = ctx.readFileState.get(fullPath)
if !readTimestamp: errorCode 9 — "File has not been read yet. Read it first before writing to it."
if getFileModificationTime(fullPath) > readTimestamp.timestamp: errorCode 10 — "File has been modified since read..."

content = readFileSyncWithMetadata(fullPath).content       # ENOENT → errorCode 1
notebook = safeParseJSON(content)                          # null → errorCode 6 — "Notebook is not valid JSON."
if !cell_id:
   if edit_mode !== 'insert': errorCode 7 — "Cell ID must be specified when not inserting a new cell."
else:
   idx = notebook.cells.findIndex(c => c.id === cell_id)
   if idx === -1:
      parsed = parseCellId(cell_id)                        # 'cell-N' → N
      if parsed !== undefined:
         if !notebook.cells[parsed]: errorCode 7 — `Cell with index ${parsed} does not exist in notebook.`
      else: errorCode 8 — `Cell with ID "${cell_id}" not found in notebook.`
return {result:true}
```

`call` (NotebookEditTool.ts:295-490):

```
fullPath = isAbsolute(...) ? notebook_path : resolve(cwd, notebook_path)
if fileHistoryEnabled(): await fileHistoryTrackEdit(...)

try:
  { content, encoding, lineEndings } = readFileSyncWithMetadata(fullPath)
  # Non-memoized parse: jsonParse from slowOperations (NOT safeParseJSON which caches).
  # Mutating a shared cached object would poison validateInput.
  notebook = jsonParse(content)                            # JSON err → return {data: { ..., error:'Notebook is not valid JSON.', ... }}

  cellIndex = (!cell_id) ? 0
            : notebook.cells.findIndex(c => c.id === cell_id) (== -1 ⇒ parseCellId(cell_id))
  if originalEditMode === 'insert': cellIndex += 1

  edit_mode = originalEditMode
  if edit_mode === 'replace' && cellIndex === notebook.cells.length:
     edit_mode = 'insert'
     if !cell_type: cell_type = 'code'

  language = notebook.metadata.language_info?.name ?? 'python'

  # nbformat ≥ 4.5 — generate a 13-char alphanumeric id for inserts; otherwise reuse cell_id (or undefined for delete)
  new_cell_id = (nbformat>4 || (nbformat===4 && nbformat_minor>=5))
                ? (edit_mode==='insert' ? Math.random().toString(36).substring(2,15)
                                        : (cell_id !== null ? cell_id : undefined))
                : undefined

  switch edit_mode:
     'delete' → notebook.cells.splice(cellIndex, 1)
     'insert' → cell = (cell_type === 'markdown')
                       ? { cell_type:'markdown', id:new_cell_id, source:new_source, metadata:{} }
                       : { cell_type:'code',     id:new_cell_id, source:new_source, metadata:{}, execution_count:null, outputs:[] }
                notebook.cells.splice(cellIndex, 0, cell)
     'replace' → target = cells[cellIndex]
                target.source = new_source
                if target.cell_type === 'code': target.execution_count = null; target.outputs = []
                if cell_type && cell_type !== target.cell_type: target.cell_type = cell_type

  IPYNB_INDENT = 1
  updated = jsonStringify(notebook, null, IPYNB_INDENT)
  writeTextContent(fullPath, updated, encoding, lineEndings)
  readFileState.set(fullPath, { content:updated, timestamp:getFileModificationTime(fullPath), offset:undefined, limit:undefined })
  return { data:{ new_source, cell_type ?? 'code', language, edit_mode ?? 'replace',
                  cell_id:new_cell_id||undefined, error:'',
                  notebook_path:fullPath, original_file:content, updated_file:updated } }
catch error:
  return { data:{ ... error: error.message || 'Unknown error occurred while editing notebook', ... } }
```

`mapToolResultToToolResultBlockParam` (NotebookEditTool.ts:133-170): if `error` set, `is_error:true` with the error string. Else by `edit_mode`:
- `replace` → `'Updated cell ${cell_id} with ${new_source}'`
- `insert` → `'Inserted cell ${cell_id} with ${new_source}'`
- `delete` → `'Deleted cell ${cell_id}'`
- default → `'Unknown edit mode'`

**Permission body** (`NotebookEditTool.ts:125-132`): same `checkWritePermissionForTool` as Write/Edit. There is **no** `preparePermissionMatcher` override (Write/Edit have one).

### 5.5 Decision overview (cross-tool)

```
                       ┌─ Read ──────────────► dedup hit?  ► file_unchanged stub
ToolUse(Read)          │                       │
                       │                       └─► dispatch by ext: ipynb / image / pdf / text
                       │
ToolUse(Write)         ┌─ validate ──► UNC?  ENOENT?  read-cache present?  mtime stale?
                       │
                       └─ call ──► R-M-W (intra-process turn-ordered); LSP didChange/Save; cache update; gitDiff?

ToolUse(Edit)          ┌─ validate ──► (… 11 errorCodes …) ► uniqueness ► settings-file extra
                       └─ call ──► R-M-W (intra-process turn-ordered) via getPatchForEdits + writeTextContent

ToolUse(NotebookEdit)  ┌─ validate ──► extension; edit_mode; insert needs cell_type; read-before; mtime; JSON; cell lookup
                       └─ call ──► splice/insert/replace; jsonStringify(IPYNB_INDENT=1); writeTextContent
```

---

## 6. Verbatim Assets

### 6.1 FileReadTool

#### 6.1.A Description / prompt template (`FileReadTool/prompt.ts:12-49`)

`DESCRIPTION = 'Read a file from the local filesystem.'`

```ts
export function renderPromptTemplate(
  lineFormat: string,
  maxSizeInstruction: string,
  offsetInstruction: string,
): string {
  return `Reads a file from the local filesystem. You can access any file directly by using this tool.
Assume this tool is able to read all files on the machine. If the User provides a path to a file assume that path is valid. It is okay to read a file that does not exist; an error will be returned.

Usage:
- The file_path parameter must be an absolute path, not a relative path
- By default, it reads up to ${MAX_LINES_TO_READ} lines starting from the beginning of the file${maxSizeInstruction}
${offsetInstruction}
${lineFormat}
- This tool allows Claude Code to read images (eg PNG, JPG, etc). When reading an image file the contents are presented visually as Claude Code is a multimodal LLM.${
    isPDFSupported()
      ? '\n- This tool can read PDF files (.pdf). For large PDFs (more than 10 pages), you MUST provide the pages parameter to read specific page ranges (e.g., pages: "1-5"). Reading a large PDF without the pages parameter will fail. Maximum 20 pages per request.'
      : ''
  }
- This tool can read Jupyter notebooks (.ipynb files) and returns all cells with their outputs, combining code, text, and visualizations.
- This tool can only read files, not directories. To read a directory, use an ls command via the ${BASH_TOOL_NAME} tool.
- You will regularly be asked to read screenshots. If the user provides a path to a screenshot, ALWAYS use this tool to view the file at the path. This tool will work with all temporary file paths.
- If you read a file that exists but has empty contents you will receive a system reminder warning in place of file contents.`
}
```

Constants (`prompt.ts:5-22`):

| Symbol | Value |
|---|---|
| `FILE_READ_TOOL_NAME` | `'Read'` |
| `MAX_LINES_TO_READ` | `2000` |
| `LINE_FORMAT_INSTRUCTION` | `'- Results are returned using cat -n format, with line numbers starting at 1'` |
| `OFFSET_INSTRUCTION_DEFAULT` | `"- You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters"` |
| `OFFSET_INSTRUCTION_TARGETED` | `'- When you already know which part of the file you need, only read that part. This can be important for larger files.'` |
| `FILE_UNCHANGED_STUB` | `'File unchanged since last read. The content from the earlier Read tool_result in this conversation is still current — refer to that instead of re-reading.'` |

`maxSizeInstruction` is computed at runtime as `". Files larger than ${formatFileSize(limits.maxSizeBytes)} will return an error; use offset and limit for larger files"` only when `getDefaultFileReadingLimits().includeMaxSizeInPrompt` is true.

#### 6.1.B Input schema (`FileReadTool.ts:227-243`)

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    file_path: z.string().describe('The absolute path to the file to read'),
    offset: semanticNumber(z.number().int().nonnegative().optional()).describe(
      'The line number to start reading from. Only provide if the file is too large to read at once',
    ),
    limit: semanticNumber(z.number().int().positive().optional()).describe(
      'The number of lines to read. Only provide if the file is too large to read at once.',
    ),
    pages: z
      .string()
      .optional()
      .describe(
        `Page range for PDF files (e.g., "1-5", "3", "10-20"). Only applicable to PDF files. Maximum ${PDF_MAX_PAGES_PER_READ} pages per request.`,
      ),
  }),
)
```

#### 6.1.C Output schema discriminator (`FileReadTool.ts:248-332`)

Discriminated union on `type`: `'text' | 'image' | 'notebook' | 'pdf' | 'parts' | 'file_unchanged'`. Image media-type enum: `['image/jpeg','image/png','image/gif','image/webp']`. Cells array is `z.array(z.any())`.

#### 6.1.D Constants table

| Constant | Value | Source |
|---|---|---|
| `MAX_LINES_TO_READ` | 2000 | `FileReadTool/prompt.ts:10` |
| `IMAGE_EXTENSIONS` | `{'png','jpg','jpeg','gif','webp'}` | `FileReadTool.ts:188` |
| `MITIGATION_EXEMPT_MODELS` | `{'claude-opus-4-6'}` | `FileReadTool.ts:733` |
| `THIN_SPACE` | `String.fromCharCode(8239)` (U+202F) | `FileReadTool.ts:131` |
| `BLOCKED_DEVICE_PATHS` | exact set in §5.1 step 6 | `FileReadTool.ts:98-115` |
| `MAX_OUTPUT_SIZE` | `0.25 * 1024 * 1024` (256 KiB) | `utils/file.ts:48` |
| `DEFAULT_MAX_OUTPUT_TOKENS` | `25000` | `FileReadTool/limits.ts:18` |
| `READ_FILE_STATE_CACHE_SIZE` | 100 | `utils/fileStateCache.ts:18` |
| `DEFAULT_MAX_CACHE_SIZE_BYTES` | `25*1024*1024` | `utils/fileStateCache.ts:22` |
| `PDF_AT_MENTION_INLINE_THRESHOLD` | 10 | `constants/apiLimits.ts:83` |
| `PDF_EXTRACT_SIZE_THRESHOLD` | `3 * 1024 * 1024` (3 MB) | `constants/apiLimits.ts:66` |
| `PDF_MAX_PAGES_PER_READ` | 20 | `constants/apiLimits.ts:77` |
| `maxResultSizeChars` | **`Infinity`** | `FileReadTool.ts:342` |
| `tengu_amber_wren` | GrowthBook key for limits override | `limits.ts:56` |
| `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS` | env override (precedence: env > GB > default) | `limits.ts:25-33` |
| `tengu_read_dedup_killswitch` | GrowthBook killswitch | `FileReadTool.ts:537` |

#### 6.1.E Validation error strings (verbatim)

- `errorCode 7`: `Invalid pages parameter: "${pages}". Use formats like "1-5", "3", or "10-20". Pages are 1-indexed.`
- `errorCode 8`: `Page range "${pages}" exceeds maximum of ${PDF_MAX_PAGES_PER_READ} pages per request. Please use a smaller range.`
- `errorCode 1`: `'File is in a directory that is denied by your permission settings.'`
- `errorCode 4`: `This tool cannot read binary files. The file appears to be a binary ${ext} file. Please use appropriate tools for binary file analysis.`
- `errorCode 9`: `Cannot read '${file_path}': this device file would block or produce infinite output.`

#### 6.1.F Runtime error strings (verbatim)

- Notebook overflow (`FileReadTool.ts:828-835`):
  ```
  Notebook content (${formatFileSize(cellsJsonBytes)}) exceeds maximum allowed size (${formatFileSize(maxSizeBytes)}). Use ${BASH_TOOL_NAME} with jq to read specific portions:
    cat "${file_path}" | jq '.cells[:20]' # First 20 cells
    cat "${file_path}" | jq '.cells[100:120]' # Cells 100-120
    cat "${file_path}" | jq '.cells | length' # Count total cells
    cat "${file_path}" | jq '.cells[] | select(.cell_type=="code") | .source' # All code sources
  ```
- Image empty: `Image file is empty: ${filePath}`
- PDF too many pages: `This PDF has ${pageCount} pages, which is too many to read at once. Use the pages parameter to read specific page ranges (e.g., pages: "1-5"). Maximum ${PDF_MAX_PAGES_PER_READ} pages per request.`
- PDF unsupported: `Reading full PDFs is not supported with this model. Use a newer model (Sonnet 3.5 v2 or later), or use the pages parameter to read specific page ranges (e.g., pages: "1-5", maximum ${PDF_MAX_PAGES_PER_READ} pages per request). Page extraction requires poppler-utils: install with \`brew install poppler\` on macOS or \`apt-get install poppler-utils\` on Debian/Ubuntu.`
- File-not-found message: `File does not exist. ${FILE_NOT_FOUND_CWD_NOTE} ${getCwd()}.` (+ optional cwd suggestion / similar-file suggestion). `FILE_NOT_FOUND_CWD_NOTE = 'Note: your current working directory is'` (`utils/file.ts:213`).
- Token cap (`MaxFileReadTokenExceededError`): `File content (${tokenCount} tokens) exceeds maximum allowed tokens (${maxTokens}). Use offset and limit parameters to read specific portions of the file, or search for specific content instead of reading the whole file.`

#### 6.1.G Result-content templates (verbatim)

`CYBER_RISK_MITIGATION_REMINDER` (`FileReadTool.ts:729-730`):

```
\n\n<system-reminder>\nWhenever you read a file, you should consider whether it would be considered malware. You CAN and SHOULD provide analysis of malware, what it is doing. But you MUST refuse to improve or augment the code. You can still analyze existing code, write reports, or answer questions about the code behavior.\n</system-reminder>\n
```

Empty-file warnings (`FileReadTool.ts:706-707`):
- empty: `<system-reminder>Warning: the file exists but the contents are empty.</system-reminder>`
- past-end: `<system-reminder>Warning: the file exists but is shorter than the provided offset (${data.file.startLine}). The file has ${data.file.totalLines} lines.</system-reminder>`

PDF result strings:
- `PDF file read: ${data.file.filePath} (${formatFileSize(data.file.originalSize)})`
- `PDF pages extracted: ${data.file.count} page(s) from ${data.file.filePath} (${formatFileSize(data.file.originalSize)})`

### 6.2 FileWriteTool

#### 6.2.A Description (`FileWriteTool/prompt.ts:10-18`)

```ts
export function getWriteToolDescription(): string {
  return `Writes a file to the local filesystem.

Usage:
- This tool will overwrite the existing file if there is one at the provided path.${getPreReadInstruction()}
- Prefer the Edit tool for modifying existing files — it only sends the diff. Only use this tool to create new files or for complete rewrites.
- NEVER create documentation files (*.md) or README files unless explicitly requested by the User.
- Only use emojis if the user explicitly requests it. Avoid writing emojis to files unless asked.`
}

function getPreReadInstruction(): string {
  return `\n- If this is an existing file, you MUST use the ${FILE_READ_TOOL_NAME} tool first to read the file's contents. This tool will fail if you did not read the file first.`
}
```

`DESCRIPTION = 'Write a file to the local filesystem.'` (`prompt.ts:4`)

#### 6.2.B Input schema (`FileWriteTool.ts:56-65`)

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    file_path: z
      .string()
      .describe(
        'The absolute path to the file to write (must be absolute, not relative)',
      ),
    content: z.string().describe('The content to write to the file'),
  }),
)
```

#### 6.2.C Result content strings (verbatim — `FileWriteTool.ts:418-433`)

- `File created successfully at: ${filePath}`
- `The file ${filePath} has been updated successfully.`

#### 6.2.D Validation error strings (verbatim)

| `errorCode` | Message |
|---|---|
| 0 | (string returned by `checkTeamMemSecrets(...)` — owned by spec 29) |
| 1 | `'File is in a directory that is denied by your permission settings.'` |
| 2 | `'File has not been read yet. Read it first before writing to it.'` |
| 3 | `'File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.'` |

Runtime error: `FILE_UNEXPECTEDLY_MODIFIED_ERROR = 'File has been unexpectedly modified. Read it again before attempting to write it.'` (`FileEditTool/constants.ts:10`).

### 6.3 FileEditTool

#### 6.3.A Description (`FileEditTool/prompt.ts`)

```ts
function getDefaultEditDescription(): string {
  const prefixFormat = isCompactLinePrefixEnabled()
    ? 'line number + tab'
    : 'spaces + line number + arrow'
  const minimalUniquenessHint =
    process.env.USER_TYPE === 'ant'
      ? `\n- Use the smallest old_string that's clearly unique — usually 2-4 adjacent lines is sufficient. Avoid including 10+ lines of context when less uniquely identifies the target.`
      : ''
  return `Performs exact string replacements in files.

Usage:${getPreReadInstruction()}
- When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces) as it appears AFTER the line number prefix. The line number prefix format is: ${prefixFormat}. Everything after that is the actual file content to match. Never include any part of the line number prefix in the old_string or new_string.
- ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required.
- Only use emojis if the user explicitly requests it. Avoid adding emojis to files unless asked.
- The edit will FAIL if \`old_string\` is not unique in the file. Either provide a larger string with more surrounding context to make it unique or use \`replace_all\` to change every instance of \`old_string\`.${minimalUniquenessHint}
- Use \`replace_all\` for replacing and renaming strings across the file. This parameter is useful if you want to rename a variable for instance.`
}

function getPreReadInstruction(): string {
  return `\n- You must use your \`${FILE_READ_TOOL_NAME}\` tool at least once in the conversation before editing. This tool will error if you attempt an edit without reading the file. `
}
```

`description()` returns `'A tool for editing files'` (`FileEditTool.ts:91`).

#### 6.3.B Input schema (`FileEditTool/types.ts:6-19`)

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    file_path: z.string().describe('The absolute path to the file to modify'),
    old_string: z.string().describe('The text to replace'),
    new_string: z
      .string()
      .describe(
        'The text to replace it with (must be different from old_string)',
      ),
    replace_all: semanticBoolean(
      z.boolean().default(false).optional(),
    ).describe('Replace all occurrences of old_string (default false)'),
  }),
)
```

#### 6.3.C Constants (`FileEditTool/constants.ts`, `FileEditTool.ts:84`, `utils.ts:21-24`, `:355`, `:408`)

| Constant | Value |
|---|---|
| `FILE_EDIT_TOOL_NAME` | `'Edit'` |
| `CLAUDE_FOLDER_PERMISSION_PATTERN` | `'/.claude/**'` |
| `GLOBAL_CLAUDE_FOLDER_PERMISSION_PATTERN` | `'~/.claude/**'` |
| `FILE_UNEXPECTEDLY_MODIFIED_ERROR` | `'File has been unexpectedly modified. Read it again before attempting to write it.'` |
| `MAX_EDIT_FILE_SIZE` | `1024*1024*1024` (1 GiB stat-bytes hard cap) |
| `LEFT_SINGLE_CURLY_QUOTE` / `RIGHT_SINGLE_CURLY_QUOTE` / `LEFT_DOUBLE_CURLY_QUOTE` / `RIGHT_DOUBLE_CURLY_QUOTE` | `'‘' '’' '“' '”'` |
| `DIFF_SNIPPET_MAX_BYTES` | `8192` |
| `CONTEXT_LINES` (snippet) | `4` |

`DESANITIZATIONS` table (verbatim, `utils.ts:531-549`): `<fnr>→<function_results>`, `<n>→<name>`, `</n>→</name>`, `<o>→<output>`, `</o>→</output>`, `<e>→<error>`, `</e>→</error>`, `<s>→<system>`, `</s>→</system>`, `<r>→<result>`, `</r>→</result>`, `< META_START >→<META_START>`, `< META_END >→<META_END>`, `< EOT >→<EOT>`, `< META >→<META>`, `< SOS >→<SOS>`, `\n\nH:→\n\nHuman:`, `\n\nA:→\n\nAssistant:`.

#### 6.3.D Validation error strings (verbatim — all `behavior:'ask'` except where noted)

| code | message |
|---|---|
| 0 | (secret-guard string, spec 29) |
| 1 | `'No changes to make: old_string and new_string are exactly the same.'` |
| 2 | `'File is in a directory that is denied by your permission settings.'` |
| 3 | `'Cannot create new file - file already exists.'` |
| 4 | `File does not exist. ${FILE_NOT_FOUND_CWD_NOTE} ${getCwd()}.` (+ optional `Did you mean ${cwdSuggestion}?` / `Did you mean ${similarFilename}?`) |
| 5 | `File is a Jupyter Notebook. Use the ${NOTEBOOK_EDIT_TOOL_NAME} to edit this file.` |
| 6 | `'File has not been read yet. Read it first before writing to it.'` |
| 7 | `'File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.'` |
| 8 | `String to replace not found in file.\nString: ${old_string}` |
| 9 | `Found ${matches} matches of the string to replace, but replace_all is false. To replace all occurrences, set replace_all to true. To replace only one occurrence, please provide more context to uniquely identify the instance.\nString: ${old_string}` |
| 10 | `File is too large to edit (${formatFileSize(size)}). Maximum editable file size is ${formatFileSize(MAX_EDIT_FILE_SIZE)}.` |

`getPatchForEdits` runtime errors (`utils.ts:307-336`):
- `'Cannot edit file: old_string is a substring of a new_string from a previous edit.'`
- `'String not found in file. Failed to apply edit.'`
- `'Original and edited file match exactly. Failed to apply edit.'`

#### 6.3.E Result content strings (verbatim — `FileEditTool.ts:575-594`)

- replaceAll true: `The file ${filePath} has been updated${modifiedNote}. All occurrences were successfully replaced.`
- replaceAll false: `The file ${filePath} has been updated successfully${modifiedNote}.`
- where `modifiedNote = userModified ? '.  The user modified your proposed changes before accepting them. ' : ''`.

### 6.4 NotebookEditTool

#### 6.4.A Description / prompt (`NotebookEditTool/prompt.ts`)

```ts
export const DESCRIPTION =
  'Replace the contents of a specific cell in a Jupyter notebook.'
export const PROMPT = `Completely replaces the contents of a specific cell in a Jupyter notebook (.ipynb file) with new source. Jupyter notebooks are interactive documents that combine code, text, and visualizations, commonly used for data analysis and scientific computing. The notebook_path parameter must be an absolute path, not a relative path. The cell_number is 0-indexed. Use edit_mode=insert to add a new cell at the index specified by cell_number. Use edit_mode=delete to delete the cell at the index specified by cell_number.`
```

#### 6.4.B Input schema (`NotebookEditTool.ts:30-57`)

```ts
export const inputSchema = lazySchema(() =>
  z.strictObject({
    notebook_path: z.string().describe(
      'The absolute path to the Jupyter notebook file to edit (must be absolute, not relative)',
    ),
    cell_id: z.string().optional().describe(
      'The ID of the cell to edit. When inserting a new cell, the new cell will be inserted after the cell with this ID, or at the beginning if not specified.',
    ),
    new_source: z.string().describe('The new source for the cell'),
    cell_type: z.enum(['code', 'markdown']).optional().describe(
      'The type of the cell (code or markdown). If not specified, it defaults to the current cell type. If using edit_mode=insert, this is required.',
    ),
    edit_mode: z.enum(['replace', 'insert', 'delete']).optional().describe(
      'The type of edit to make (replace, insert, delete). Defaults to replace.',
    ),
  }),
)
```

#### 6.4.C Constants

| Constant | Value | Source |
|---|---|---|
| `NOTEBOOK_EDIT_TOOL_NAME` | `'NotebookEdit'` | `constants.ts:2` |
| `IPYNB_INDENT` | `1` | `NotebookEditTool.ts:430` |
| nbformat threshold for ID generation | `>4 || (===4 && minor>=5)` | `NotebookEditTool.ts:382-384` |
| ID generator | `Math.random().toString(36).substring(2, 15)` | `:386` |

#### 6.4.D Validation error strings (verbatim)

| code | message |
|---|---|
| 1 | `'Notebook file does not exist.'` |
| 2 | `'File must be a Jupyter notebook (.ipynb file). For editing other file types, use the FileEdit tool.'` |
| 4 | `'Edit mode must be replace, insert, or delete.'` |
| 5 | `'Cell type is required when using edit_mode=insert.'` |
| 6 | `'Notebook is not valid JSON.'` |
| 7a | `'Cell ID must be specified when not inserting a new cell.'` (no `cell_id` on non-insert; `NotebookEditTool.ts:265`) |
| 7b | `Cell with index ${parsedCellIndex} does not exist in notebook.` (`cell-N` numeric form parses to out-of-range; `NotebookEditTool.ts:280`) — same `errorCode: 7` |
| 8 | `Cell with ID "${cell_id}" not found in notebook.` |
| 9 | `'File has not been read yet. Read it first before writing to it.'` |
| 10 | `'File has been modified since read, either by the user or by a linter. Read it again before attempting to write it.'` |

Result-content strings (verbatim — `NotebookEditTool.ts:145-170`):
- `Updated cell ${cell_id} with ${new_source}` (replace)
- `Inserted cell ${cell_id} with ${new_source}` (insert)
- `Deleted cell ${cell_id}` (delete)
- `'Unknown edit mode'` (default)

### 6.5 Path-only preview format (cited from `utils/toolResultStorage.ts:189-199`, owned by spec 41/04)

```ts
export function buildLargeToolResultMessage(
  result: PersistedToolResult,
): string {
  let message = `${PERSISTED_OUTPUT_TAG}\n`
  message += `Output too large (${formatFileSize(result.originalSize)}). Full output saved to: ${result.filepath}\n\n`
  message += `Preview (first ${formatFileSize(PREVIEW_SIZE_BYTES)}):\n`
  message += result.preview
  message += result.hasMore ? '\n...\n' : '\n'
  message += PERSISTED_OUTPUT_CLOSING_TAG
  return message
}
```

This message is what the model sees for **Write/Edit/NotebookEdit** results that exceed `getPersistenceThreshold(name, 100_000)`. **Read never goes through this path** — it sets `maxResultSizeChars: Infinity` and enforces tokens via `MaxFileReadTokenExceededError` instead.

---

## 7. Side Effects & I/O

| Tool | Filesystem | Network | Process | Subscribers / Notifications |
|---|---|---|---|---|
| Read | `readFile`/`readdir`/`stat` only; possible PDF page extraction to a temp dir; macOS sharp/`image-processor-napi` for image work | API call to `countTokensWithAPI` (spec 06) when token estimate > maxTokens/4 | poppler-utils (`pdftoppm`) when `extractPDFPages` invoked | `fileReadListeners` (snapshot iter), `nestedMemoryAttachmentTriggers`, `dynamicSkillDirTriggers`, `activateConditionalSkillsForPaths` |
| Write | `mkdir(dir)`, `readFileSyncWithMetadata` (synchronous), `writeTextContent(...,'LF')` | optional single-file `fetchSingleFileGitDiff` under env+GB | none | `diagnosticTracker.beforeFileEdited`, `clearDeliveredDiagnosticsForFile`, `lspManager.changeFile/.saveFile`, `notifyVscodeFileUpdated`, skills discovery, `tengu_write_claudemd` |
| Edit | identical to Write but preserves disk encoding + endings | same | none | identical to Write |
| NotebookEdit | `readFileSyncWithMetadata`, `writeTextContent(... encoding, lineEndings)` | none | none | `fileHistoryTrackEdit` only; no LSP or VSCode notify |

Environment vars consumed by this spec:

| Var | Where | Effect |
|---|---|---|
| `USER_TYPE === 'ant'` | `FileEditTool/prompt.ts:17` | adds `minimalUniquenessHint` |
| `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS` | `FileReadTool/limits.ts:25-32` | overrides `maxTokens` (positive int parse) |
| `CLAUDE_CODE_SIMPLE` | `FileReadTool.ts:578`, `FileEditTool.ts:407` | disables skill discovery / activation |
| `CLAUDE_CODE_REMOTE` | Write/Edit | gates `tengu_quartz_lantern` gitDiff capture |
| `CLAUDE_CODE_REMOTE_SESSION_ID` | `filePersistence.ts:65, :283` | required for `runFilePersistence` and `isFilePersistenceEnabled` |

External binaries: `poppler-utils` (PDF page extraction); `git` (single-file diff); macOS `osascript` fallback for clipboard image (spec 37 paste path only).

UNC trust boundary: every tool short-circuits paths beginning with `\\` or `//` to skip `fs.stat`/`fs.readFileBytes` (NTLM credential leak prevention).

Permission trust boundary: all four tools delegate to `checkRead/WritePermissionForTool` (spec 09).

---

## 8. Feature Flags & Variants

| Flag / Var | Effect | Cite |
|---|---|---|
| `USER_TYPE === 'ant'` | Adds the `minimalUniquenessHint` line to the Edit description. | `FileEditTool/prompt.ts:17` |
| `feature('TRANSCRIPT_CLASSIFIER')` | NotebookEdit's `toAutoClassifierInput` returns the verbose form vs `''`. | `NotebookEditTool.ts:116-120` |
| `feature('FILE_PERSISTENCE')` | Required by `isFilePersistenceEnabled()` (combined with BYOC env + session token + remote session id). | `filePersistence.ts:279` |
| `feature('NATIVE_CLIPBOARD_IMAGE')` | Enables the native NSPasteboard fast paths in `imagePaste.ts`; falls back to `osascript` on any failure. | `imagePaste.ts:101, :132` |
| GB `tengu_amber_wren` | Per-field override for `maxSizeBytes`/`maxTokens`/`includeMaxSizeInPrompt`/`targetedRangeNudge`. | `limits.ts:53-92` |
| GB `tengu_read_dedup_killswitch` | When **true**, disables Read same-file dedup. | `FileReadTool.ts:537-573` |
| GB `tengu_quartz_lantern` | Combined with `CLAUDE_CODE_REMOTE`, attaches `gitDiff` to Write/Edit output. | `FileWriteTool.ts:347, FileEditTool.ts:548` |
| GB `tengu_collage_kaleidoscope` | Combined with `NATIVE_CLIPBOARD_IMAGE`, gates the native clipboard PNG check. | `imagePaste.ts:102` |
| Model gate `MITIGATION_EXEMPT_MODELS = {'claude-opus-4-6'}` | Suppresses `CYBER_RISK_MITIGATION_REMINDER`. | `FileReadTool.ts:733-738` |
| `isCompactLinePrefixEnabled()` | Flips Edit description's `prefixFormat` between `'line number + tab'` and `'spaces + line number + arrow'`. | `FileEditTool/prompt.ts:13` |

---

## 9. Error Handling & Edge Cases

- **Read of nonexistent file**: ENOENT first tries `getAlternateScreenshotPath` (macOS thin/regular space U+202F before AM/PM in `*.png`); if that path also ENOENTs, throws a `'File does not exist. '+FILE_NOT_FOUND_CWD_NOTE+...` error, optionally suffixed with `findSimilarFile` or `suggestPathUnderCwd` text. Errors thrown from `call()` lack the `<tool_use_error>` wrapper; the UI's `renderToolUseErrorMessage` (FileReadTool/UI.tsx:144-163) sniffs `result.includes(FILE_NOT_FOUND_CWD_NOTE)` directly to produce `"File not found"` chrome.
- **Edit empty/empty-on-empty**: `getPatchForEdits` has a special early-return for `fileContents===''` + single edit `{old:'',new:''}` returning `{patch, updatedFile:''}`.
- **Edit deletion swallows trailing newline**: `applyEditToFile` with `newString===''` will treat `oldString + '\n'` as the actual match if the file contains that pattern AND `oldString` itself doesn't end with `\n` (avoids leaving blank lines).
- **Edit with curly quotes in file**: `findActualString` normalizes both sides; `preserveQuoteStyle` re-encodes `new_string` to match. Apostrophes inside contractions (letter–`'`–letter) always become `RIGHT_SINGLE_CURLY_QUOTE` regardless of opening context.
- **Edit on `.ipynb`**: rejected at validate (errorCode 5) pointing at NotebookEditTool.
- **Edit on huge file (>1 GiB stat bytes)**: rejected at validate (errorCode 10) — this is BEFORE the file-read attempt to avoid OOM via `buf.toString(...)`.
- **Edit "old_string substring of previous new_string"**: thrown by multi-edit driver; not reachable from single-edit `FileEditTool.call` (only the multi-edit driver collects an `appliedNewStrings` history).
- **Write+Edit Windows mtime false positive**: when `lastWriteTime > readTimestamp.timestamp` but `isFullRead && content === lastRead.content`, proceed (cloud sync / AV touches).
- **NotebookEdit on bad JSON**: `safeParseJSON` returns null at validate (`errorCode 6`); inside `call`, `jsonParse` throw is caught and surfaced via the result data with `error: 'Notebook is not valid JSON.'` (NOT a thrown `Error`).
- **NotebookEdit replace at end-of-cells**: `cellIndex === cells.length` while `edit_mode === 'replace'` → coerced to `'insert'`, defaulting `cell_type='code'` if missing.
- **NotebookEdit `cell_id` numeric form `cell-N`**: `parseCellId` parses the numeric suffix; out-of-range → errorCode 7 (split per §6.4.D as 7b; the no-`cell_id`-on-non-insert path is 7a — both share the numeric `errorCode: 7`).
- **NotebookEdit stale `cell_number` prompt language (src bug)**: `NotebookEditTool/prompt.ts:3` `PROMPT` references `cell_number` ("The `cell_number` is 0-indexed... at the index specified by `cell_number`") but the strict input schema (§6.4.B) only accepts `cell_id`. A code comment at `NotebookEditTool.ts:418` ("validateInput ensures cell_number is in bounds") repeats the stale wording. The model must use `cell_id` (or its `cell-N` numeric form parsed by `parseCellId`); a literal `cell_number` field will fail strict-schema validation. Tracked for `BUGS-IN-SOURCE.md` (Phase 9.7) — a re-implementer should rewrite the prompt to say `cell_id`.
- **Symlinks (spec gap, no special handling)**: none of Read/Write/Edit/NotebookEdit calls `fs.lstat`, `fs.realpath`, or otherwise distinguishes symlinks from regular files. `expandPath` only resolves `~`/relatives; it does not collapse symlinks. Consequences:
  - Reads and writes silently follow symlinks; a symlink inside `getCwd()` pointing outside `getCwd()` defeats the cwd-suggestion intent of `suggestPathUnderCwd` but is **not** a permission bypass (permissions are evaluated on the user-supplied path, not the resolved target — see spec 09).
  - `BLOCKED_DEVICE_PATHS` matches the literal path string; a symlink whose target is `/dev/zero` is **not** caught by the device-path filter. Re-implementers who care about device-symlink rejection must `realpath` before the check.
  - macOS `os.tmpdir()` returns `/var/folders/...` but `/tmp` → `/private/tmp`; tools resolve whichever path the caller passed in. Cache keys in `FileStateCache` are `path.normalize`d but not `realpath`'d, so `/tmp/x` and `/private/tmp/x` get distinct cache entries.
  - Circular and dangling symlinks: dangling → ENOENT (handled by the standard not-found branch). Circular → `EMFILE`/`ELOOP` from `fs.readFile`; surfaces as a generic thrown error (no special chrome).
- **Read same-file dedup**: only fires when prior entry has `offset !== undefined` (i.e. came from a Read, not a Write/Edit). Killswitch GB `tengu_read_dedup_killswitch` disables.
- **Read image OOM guard**: `readFileBytes(filePath, maxBytes)` accepts an optional cap. Default callsite passes none; PDF-pages branch creates resized JPEGs first.
- **Read text past EOF**: result `data.file.content === ''` ⇒ `mapToolResultToToolResultBlockParam` emits one of the two `<system-reminder>` warnings, NOT line-numbered content.
- **Read PDF older models**: `!isPDFSupported()` throws the verbatim "Reading full PDFs is not supported with this model..." message.
- **Read of `/dev/zero`, `/dev/random`, etc.**: rejected at validate (errorCode 9). `/dev/null` is intentionally allowed.

---

## 10. Telemetry & Observability

| Event | Where | Notes |
|---|---|---|
| `tengu_file_read_limits_override` | `FileReadTool.ts:512` | only fires when caller overrides (low volume) |
| `tengu_file_read_dedup` | `FileReadTool.ts:559` | with optional `ext` |
| `tengu_pdf_page_extraction` | `FileReadTool.ts:904, :965, :971` | success/failure + sizes |
| `tengu_session_file_read` | `FileReadTool.ts:1069-1083` | per-text-read with `is_session_memory`/`is_session_transcript` |
| `tengu_write_claudemd` | Write/Edit when path ends `${sep}CLAUDE.md` | |
| `tengu_tool_use_diff_computed` | Write/Edit when `tengu_quartz_lantern` && `CLAUDE_CODE_REMOTE` | `isWriteTool`/`isEditTool` discriminator |
| `tengu_edit_string_lengths` | `FileEditTool.ts:539-543` | `oldStringBytes`, `newStringBytes`, `replaceAll` |
| `tengu_file_persistence_started/_completed/_limit_exceeded` | `filePersistence.ts:89,112,176` | BYOC mode |
| `logFileOperation` | every Write/Edit/Read text/notebook | unified through `utils/fileOperationAnalytics.ts` |

LSP-side observable: `clearDeliveredDiagnosticsForFile` clears the diagnostic registry so subsequent `didChange/didSave` deliveries aren't deduped.

---

## 11. Reimplementation Checklist

A reimplementation is correct iff:

1. **`FileStateCache` invariants** — LRU keyed on `path.normalize`, `max=100`, `maxSize=25 MiB`, `sizeCalculation = max(1, Buffer.byteLength(content))`. `merge` keeps newer-`timestamp` entry. `isPartialView` blocks Edit/Write at validate.
2. **Read** sets `maxResultSizeChars: Infinity` (never persists results to disk) and enforces tokens via `validateContentTokens`/`MaxFileReadTokenExceededError` with the verbatim message.
3. **Read prompt** is rendered through `renderPromptTemplate(LINE_FORMAT_INSTRUCTION, maybeMaxSize, OFFSET_*)` with `MAX_LINES_TO_READ=2000`, conditional PDF clause behind `isPDFSupported()`, and the conditional `maxSizeInstruction` / `OFFSET_*` clauses tied to `tengu_amber_wren`.
4. **Read dispatch order** matches `callInner`: notebook (`ipynb`) → image (extension allowlist) → PDF (`isPDFExtension`) → text. Only text and notebook write `readFileState`. Cyber-risk mitigation reminder applied unless model is in the exempt set. `fileReadListeners` are iterated over a snapshot.
5. **Read dedup** only against prior-Read entries (offset !== undefined and !isPartialView), with mtime equality, behind GB killswitch.
6. **PDF**: page-range branch resizes via `maybeResizeAndDownsampleImageBuffer(...,'jpeg')` and injects images as a meta user-message; full-PDF branch attaches an `application/pdf` document block. Cap `PDF_AT_MENTION_INLINE_THRESHOLD=10`, `PDF_MAX_PAGES_PER_READ=20`, `PDF_EXTRACT_SIZE_THRESHOLD=3 MB`.
7. **Write**: validate enforces read-before-edit and mtime; `call` performs an intra-process turn-ordered R-M-W (single-threaded JS, no `await` between staleness check and `writeTextContent` — NOT OS-level atomic; a concurrent external writer is undetectable) with `mkdir` and `fileHistoryTrackEdit` OUTSIDE the critical section. Always writes LF (no preserve). `readFileState.set(...)` clears `offset/limit`.
8. **Edit**: validate covers all 11 error codes; `findActualString` quote-normalizes; `replace_all=false` + multiple matches ⇒ errorCode 9 with `actualOldString` in `meta`; `validateInputForSettingsFileEdit` is consulted last. `call` uses `getPatchForEdit(s)` whose substring guard, empty-newString trailing-newline-swallow, and convertLeadingTabsToSpaces semantics are all preserved. Encoding/lineEndings are taken from `readFileSyncWithMetadata` and preserved on write.
9. **NotebookEdit**: validate enforces `.ipynb`, edit_mode whitelist, insert-needs-cell_type, read-before, mtime; `call` uses non-memoized `jsonParse` (NOT `safeParseJSON`), generates 13-char IDs only when nbformat ≥ 4.5, special-cases replace-at-end → insert with default code type, indents with `IPYNB_INDENT=1`, and preserves encoding/lineEndings. JSON parse errors inside `call` surface via `data.error`, NOT a thrown `Error`.
10. **Permission delegation**: each tool's `checkPermissions` body is a one-liner calling `checkRead/WritePermissionForTool(toolRef, input, ctx.toolPermissionContext)`. Read also calls `matchingRuleForInput(...,'read','deny')` at validate; Write and Edit call the same with action `'edit'`. NotebookEdit does NOT have a deny-rule check at validate (delegated to `checkWritePermissionForTool`).
11. **UNC short-circuit**: every tool returns `{result:true}` for paths starting `\\` or `//` BEFORE any filesystem op.
12. **CLAUDE.md write detection**: matches `path.endsWith(`${sep}CLAUDE.md`)` (sep, not `/`).
13. **`writeTextContent` invariants**: Write always LF; Edit/NotebookEdit pass through `encoding` + `lineEndings` from the metadata read.
14. **Tool defaults**: Read `userFacingName` checks plansDir / agent-output prefix; Edit's `userFacingName` returns `'Create'` only when `old_string===''`. NotebookEdit's `userFacingName` is fixed `'Edit Notebook'`. NotebookEdit has `shouldDefer:true`.
15. **File persistence** is enabled iff `feature('FILE_PERSISTENCE') && getEnvironmentKind()==='byoc' && getSessionIngressAuthToken() && CLAUDE_CODE_REMOTE_SESSION_ID`. BYOC scans `{cwd}/{sessionId}/outputs`, drops `..`-relatives, enforces `FILE_COUNT_LIMIT`, uploads with `DEFAULT_UPLOAD_CONCURRENCY`. Cloud mode is a no-op stub.
16. **Result-overflow path-only preview** is built by `buildLargeToolResultMessage` with the verbatim "Output too large (...). Full output saved to: ... Preview (first ...): ..." template. **Read never enters this path.**

---

## 12. Open Questions / Unknowns

1. **`utils/file.ts:48-549` `MAX_OUTPUT_SIZE`** — `0.25 MiB` is the default for `getDefaultFileReadingLimits().maxSizeBytes` when `tengu_amber_wren` doesn't override. The `limits.ts:7-13` table says "256 KB"; both refer to the same value (256 KiB ≠ 256 KB strictly). Recorded as informational; no behavioral ambiguity.
2. **RESOLVED — `processToolResultBlock` honors Read's `Infinity`**: `getPersistenceThreshold` (`utils/toolResultStorage.ts:62-64`) early-returns when `!Number.isFinite(declaredMaxResultSizeChars)`, so the `Math.min(declared, DEFAULT_MAX_RESULT_SIZE_CHARS)` at line 77 is never reached for Read. The verbatim guard comment at `:59-61` ("Checked before the GB override so tengu_satin_quoll can't force it back on") confirms this is intentional. §11.2's invariant ("Read sets `maxResultSizeChars: Infinity` (never persists results to disk)") is correct as written. No bug; no spec 04 follow-up needed.
3. **`CLAUDE_FOLDER_PERMISSION_PATTERN` consumers**: defined in `FileEditTool/constants.ts` but referenced from elsewhere. Spec 09 should confirm session-permission grant flow consumes both patterns identically.
4. **`fileHistoryEnabled()` source-of-truth**: this spec only cites where it is called. The flag itself is owned by spec 41.
5. **Auto-memory `memoryFileMtimes` WeakMap**: keyed on the `data` object identity (`FileReadTool.ts:740-753`). Cleared by GC when the data leaves scope. This is correct only if `mapToolResultToToolResultBlockParam` is called **before** the data object becomes unreachable. Spec 04 should confirm the lifetime guarantee.
6. **Read prompt's `pickLineFormatInstruction()`**: currently a constant returning `LINE_FORMAT_INSTRUCTION`. Naming suggests an ablation point that has been collapsed; recorded for traceability.
7. **NotebookEditTool absent `preparePermissionMatcher`**: Write/Edit override it; NotebookEdit does not. Behaviour difference under wildcard-pattern session grants must be verified by spec 09.
8. **Settings file edit (`validateInputForSettingsFileEdit`)** is owned by spec **02**; this spec only documents that Edit invokes it after the uniqueness check, before returning success.
