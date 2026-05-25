# 12 — Search Tools (Glob, Grep, ToolSearch) Specification

> Bit-exact spec for the three "search" tools registered via `src/tools.ts`: filename-pattern search (`Glob`), file-content regex search (`Grep`, ripgrep wrapper), and the deferred-tool discovery mechanism (`ToolSearch`). Anchored by 08 (registry) and 09 (permissions). Read 00-overview.md and 08-tool-base-registry.md first.

---

## 1. Purpose & Scope

### IN scope
- `src/tools/GlobTool/` — full directory: `GlobTool.ts`, `prompt.ts`, `UI.tsx`. Pattern syntax (ripgrep `--glob`), modification-time sort, 100-result default cap, env overrides for hidden / no-ignore.
- `src/tools/GrepTool/` — full directory: `GrepTool.ts`, `prompt.ts`, `UI.tsx`. ripgrep CLI construction (every flag), three output modes (`content` / `files_with_matches` / `count`), context flags (`-A` / `-B` / `-C` / `context`), `multiline`, `head_limit` + `offset` pagination, type / glob filter parsing, ignore-pattern injection, ANT-bundle replacement (`bfs` / `ugrep`) per `hasEmbeddedSearchTools()`.
- `src/tools/ToolSearchTool/` — full directory: `ToolSearchTool.ts`, `prompt.ts`, `constants.ts`. The deferred-tool discovery mechanism: `select:` query parser, keyword scorer (`+required` / optional terms; word-boundary scoring), description cache invalidation, `tool_reference` result block, ANT-only prompt-text variant.
- The relationship between `Tool.shouldDefer` / `Tool.alwaysLoad` / `Tool.isMcp` and ToolSearch (cite from 08).
- Supporting utilities owned by these tools: `src/utils/glob.ts` (the `glob()` helper Glob calls), `src/utils/ripgrep.ts` (the `ripGrep()` wrapper Grep + `glob()` call), `src/utils/toolSearch.ts` (mode resolution, `isToolSearchEnabledOptimistic`, deferred-tools delta scan), `src/utils/embeddedTools.ts` (`hasEmbeddedSearchTools` predicate).

### OUT of scope
- Permission decision tree → 09. (`checkReadPermissionForTool`, `getFileReadIgnorePatterns`, mode rules, ask-once policy.)
- The `Tool` interface and registry assembly pipeline → 08. (`buildTool`, `TOOL_DEFAULTS`, `getAllBaseTools`, `assembleToolPool`.)
- File reads themselves — Read/Edit/Write tools → 11.
- Skill-as-prompt-command surface and the overlap between skill matching and ToolSearch keyword search → 17. ToolSearch returns `tool_reference` blocks; skill discovery is a separate code path.
- MCP server connection lifecycle → 23 (only the `mcp.clients.filter(c => c.type === 'pending')` read is touched here).

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/tools/GlobTool/GlobTool.ts` | 199 | `GlobTool` definition: input/output Zod schemas, `call()` dispatching to `glob()`, `extractSearchText`, render wiring, permission check |
| `src/tools/GlobTool/prompt.ts` | 8 | `GLOB_TOOL_NAME = 'Glob'`, `DESCRIPTION` template (5 bullets) |
| `src/tools/GlobTool/UI.tsx` | ~62 | `userFacingName='Search'`, `renderToolUseMessage`, `renderToolUseErrorMessage`, `getToolUseSummary`; reuses `GrepTool.renderToolResultMessage` |
| `src/tools/GrepTool/GrepTool.ts` | 578 | `GrepTool` definition: input/output Zod schemas, ripgrep arg construction, three output-mode branches, head_limit/offset pagination, mtime sort, ignore-pattern injection |
| `src/tools/GrepTool/prompt.ts` | 19 | `GREP_TOOL_NAME = 'Grep'`, `getDescription()` template (multi-line) |
| `src/tools/GrepTool/UI.tsx` | ~200 | `SearchResultSummary` Ink component, mode-aware `renderToolResultMessage`, `renderToolUseMessage`, `renderToolUseErrorMessage`, `getToolUseSummary` |
| `src/tools/ToolSearchTool/ToolSearchTool.ts` | 472 | `ToolSearchTool` definition: input/output Zod, `parseToolName`, `compileTermPatterns`, `searchToolsWithKeywords`, `select:` parser, `mapToolResultToToolResultBlockParam` emitting `tool_reference` blocks, telemetry |
| `src/tools/ToolSearchTool/prompt.ts` | 122 | Default + ANT prompt assembly via `getPrompt()`, `isDeferredTool` predicate, `formatDeferredToolLine`, `getToolLocationHint` env/flag branch |
| `src/tools/ToolSearchTool/constants.ts` | 1 | `TOOL_SEARCH_TOOL_NAME = 'ToolSearch'` |
| `src/utils/glob.ts` | 130 | `extractGlobBaseDirectory`, `glob()` — wraps `ripGrep --files --glob` with sort/ignore/hidden flags |
| `src/utils/ripgrep.ts` | 679 | `getRipgrepConfig` (system / builtin / embedded), `ripGrep`, `ripGrepStream`, `ripGrepFileCount`, `RipgrepTimeoutError`, EAGAIN retry, codesign dance, `countFilesRoundedRg`, `getRipgrepStatus` |
| `src/utils/toolSearch.ts` | 757 | `getToolSearchMode`, `isToolSearchEnabledOptimistic`, `isToolSearchEnabled`, `extractDiscoveredToolNames`, `getDeferredToolsDelta`, `isDeferredToolsDeltaEnabled`, `modelSupportsToolReference`, threshold helpers |
| `src/utils/embeddedTools.ts` | 30 | `hasEmbeddedSearchTools`, `embeddedSearchToolsBinaryPath` |

### 2.2 Source coverage

| Source | Read fully | Sampled | Grep-inspected | Notes |
|---|---|---|---|---|
| `src/tools/GlobTool/GlobTool.ts` | ✅ | | | All 199 lines read |
| `src/tools/GlobTool/prompt.ts` | ✅ | | | 8 lines |
| `src/tools/GlobTool/UI.tsx` | ✅ | | | TS portion read; sourcemap base64 ignored |
| `src/tools/GrepTool/GrepTool.ts` | ✅ | | | All 578 lines read |
| `src/tools/GrepTool/prompt.ts` | ✅ | | | 19 lines |
| `src/tools/GrepTool/UI.tsx` | ✅ | | | TS portion read; the React-compiler `_c(26)` cache scaffolding is structural, captured as Ink-render side notes |
| `src/tools/ToolSearchTool/ToolSearchTool.ts` | ✅ | | | All 472 lines read |
| `src/tools/ToolSearchTool/prompt.ts` | ✅ | | | All 122 lines read |
| `src/tools/ToolSearchTool/constants.ts` | ✅ | | | 1 line |
| `src/utils/glob.ts` | ✅ | | | All 130 lines read |
| `src/utils/ripgrep.ts` | ✅ | | | All 679 lines read |
| `src/utils/toolSearch.ts` | ✅ | | | All 757 lines read |
| `src/utils/embeddedTools.ts` | ✅ | | | 30 lines |
| `src/Tool.ts` | | ✅ | | Owned by 08; here only as far as `searchHint` (`Tool.ts:372-378`), `shouldDefer` (`Tool.ts:438-442`), `alwaysLoad` (`Tool.ts:443-449`), `isMcp` (`Tool.ts:436`), `isSearchOrReadCommand` (`Tool.ts:417-433`) |

### 2.3 Imports from

`GlobTool.ts` imports: `zod/v4`, `Tool.ts` (`buildTool`, `ToolDef`, `ValidationResult`), `utils/cwd.js` (`getCwd`), `utils/errors.js` (`isENOENT`), `utils/file.js` (`FILE_NOT_FOUND_CWD_NOTE`, `suggestPathUnderCwd`), `utils/fsOperations.js` (`getFsImplementation`), `utils/glob.js` (`glob`), `utils/lazySchema.js`, `utils/path.js` (`expandPath`, `toRelativePath`), `utils/permissions/filesystem.js` (`checkReadPermissionForTool`), `utils/permissions/PermissionResult.js`, `utils/permissions/shellRuleMatching.js` (`matchWildcardPattern`), `./prompt.js`, `./UI.js`.

`GrepTool.ts` imports: `zod/v4`, `Tool.ts`, the same cwd / errors / file / fsOperations / lazySchema / path / permissions modules as Glob, plus `utils/permissions/filesystem.js` (`getFileReadIgnorePatterns`, `normalizePatternsToPath`), `utils/plugins/orphanedPluginFilter.js` (`getGlobExclusionsForPluginCache`), `utils/ripgrep.js` (`ripGrep`), `utils/semanticBoolean.js`, `utils/semanticNumber.js`, `utils/stringUtils.js` (`plural`), `./prompt.js`, `./UI.js`.

`ToolSearchTool.ts` imports: `@anthropic-ai/sdk/resources/index.mjs` (`ToolResultBlockParam`), `lodash-es/memoize.js`, `zod/v4`, `services/analytics/index.js` (`logEvent`, `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS`), `Tool.js` (`buildTool`, `findToolByName`, `Tool`, `ToolDef`, `Tools`), `utils/debug.js` (`logForDebugging`), `utils/lazySchema.js`, `utils/stringUtils.js` (`escapeRegExp`), `utils/toolSearch.js` (`isToolSearchEnabledOptimistic`), `./prompt.js` (`getPrompt`, `isDeferredTool`, `TOOL_SEARCH_TOOL_NAME`).

`ToolSearchTool/prompt.ts` imports: `bun:bundle` (`feature`), `bootstrap/state.js` (`isReplBridgeActive`), `services/analytics/growthbook.js` (`getFeatureValue_CACHED_MAY_BE_STALE`), `Tool.js` (`Tool`), `tools/AgentTool/constants.js` (`AGENT_TOOL_NAME`), `./constants.js` (`TOOL_SEARCH_TOOL_NAME`). Conditional `require()` of `BriefTool/prompt.js` (gated by `feature('KAIROS') || feature('KAIROS_BRIEF')`) and `SendUserFileTool/prompt.js` (gated by `feature('KAIROS')`); inside `isDeferredTool`, lazy `require()` of `AgentTool/forkSubagent.js` gated by `feature('FORK_SUBAGENT')` (`prompt.ts:76-81`).

### 2.4 Imported by

- `src/tools.ts:9` imports `GlobTool`, `:59` imports `GrepTool`, `:77` imports `ToolSearchTool`, `:87` imports `isToolSearchEnabledOptimistic`, `:138` imports `hasEmbeddedSearchTools`. The registration sites are `tools.ts:201` (Glob/Grep dropped iff `hasEmbeddedSearchTools()` is true) and `tools.ts:249` (ToolSearch added iff `isToolSearchEnabledOptimistic()` is true).
- `src/utils/toolSearch.ts` re-imports `formatDeferredToolLine`, `isDeferredTool`, `TOOL_SEARCH_TOOL_NAME` from `tools/ToolSearchTool/prompt.js` (`toolSearch.ts:22-26`).
- The `glob()` helper is also called by skill / file-state / memdir code paths (out of scope; cited only in §11).

---

## 3. Public Interface (Contract)

### 3.1 Glob

#### 3.1.1 Input schema (verbatim from `GlobTool.ts:26-36`)

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    pattern: z.string().describe('The glob pattern to match files against'),
    path: z
      .string()
      .optional()
      .describe(
        'The directory to search in. If not specified, the current working directory will be used. IMPORTANT: Omit this field to use the default directory. DO NOT enter "undefined" or "null" - simply omit it for the default behavior. Must be a valid directory path if provided.',
      ),
  }),
)
```

#### 3.1.2 Output schema (verbatim from `GlobTool.ts:39-52`)

```ts
const outputSchema = lazySchema(() =>
  z.object({
    durationMs: z
      .number()
      .describe('Time taken to execute the search in milliseconds'),
    numFiles: z.number().describe('Total number of files found'),
    filenames: z
      .array(z.string())
      .describe('Array of file paths that match the pattern'),
    truncated: z
      .boolean()
      .describe('Whether results were truncated (limited to 100 files)'),
  }),
)
```

#### 3.1.3 Builder fields (`GlobTool.ts:57-198`)

- `name = GLOB_TOOL_NAME` (`'Glob'`).
- `searchHint = 'find files by name pattern or wildcard'` (`GlobTool.ts:59`).
- `maxResultSizeChars = 100_000` (`GlobTool.ts:60`).
- `userFacingName = 'Search'` (from `UI.tsx:11-13`, exported reused name).
- `getActivityDescription`: `summary ? "Finding ${summary}" : "Finding files"` (`GlobTool.ts:66-69`).
- `isConcurrencySafe()` returns `true`; `isReadOnly()` returns `true` (`GlobTool.ts:76-81`).
- `toAutoClassifierInput(input) = input.pattern` (`GlobTool.ts:82-84`).
- `isSearchOrReadCommand() = { isSearch: true, isRead: false }` (`GlobTool.ts:85-87`).
- `getPath({ path }) = path ? expandPath(path) : getCwd()` (`GlobTool.ts:88-90`).
- `preparePermissionMatcher({ pattern })` returns `rulePattern => matchWildcardPattern(rulePattern, pattern)` (`GlobTool.ts:91-93`).
- `extractSearchText({ filenames }) = filenames.join('\n')` (`GlobTool.ts:151-153`).
- `description()` returns `DESCRIPTION` (`GlobTool.ts:61-63`); `prompt()` returns `DESCRIPTION` (`GlobTool.ts:143-145`).
- Reuses `GrepTool.renderToolResultMessage` via `UI.tsx:53` (one of the few cross-tool render reuses in the codebase).

### 3.2 Grep

#### 3.2.1 Input schema (verbatim from `GrepTool.ts:33-90`)

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    pattern: z
      .string()
      .describe(
        'The regular expression pattern to search for in file contents',
      ),
    path: z
      .string()
      .optional()
      .describe(
        'File or directory to search in (rg PATH). Defaults to current working directory.',
      ),
    glob: z
      .string()
      .optional()
      .describe(
        'Glob pattern to filter files (e.g. "*.js", "*.{ts,tsx}") - maps to rg --glob',
      ),
    output_mode: z
      .enum(['content', 'files_with_matches', 'count'])
      .optional()
      .describe(
        'Output mode: "content" shows matching lines (supports -A/-B/-C context, -n line numbers, head_limit), "files_with_matches" shows file paths (supports head_limit), "count" shows match counts (supports head_limit). Defaults to "files_with_matches".',
      ),
    '-B': semanticNumber(z.number().optional()).describe(
      'Number of lines to show before each match (rg -B). Requires output_mode: "content", ignored otherwise.',
    ),
    '-A': semanticNumber(z.number().optional()).describe(
      'Number of lines to show after each match (rg -A). Requires output_mode: "content", ignored otherwise.',
    ),
    '-C': semanticNumber(z.number().optional()).describe('Alias for context.'),
    context: semanticNumber(z.number().optional()).describe(
      'Number of lines to show before and after each match (rg -C). Requires output_mode: "content", ignored otherwise.',
    ),
    '-n': semanticBoolean(z.boolean().optional()).describe(
      'Show line numbers in output (rg -n). Requires output_mode: "content", ignored otherwise. Defaults to true.',
    ),
    '-i': semanticBoolean(z.boolean().optional()).describe(
      'Case insensitive search (rg -i)',
    ),
    type: z
      .string()
      .optional()
      .describe(
        'File type to search (rg --type). Common types: js, py, rust, go, java, etc. More efficient than include for standard file types.',
      ),
    head_limit: semanticNumber(z.number().optional()).describe(
      'Limit output to first N lines/entries, equivalent to "| head -N". Works across all output modes: content (limits output lines), files_with_matches (limits file paths), count (limits count entries). Defaults to 250 when unspecified. Pass 0 for unlimited (use sparingly — large result sets waste context).',
    ),
    offset: semanticNumber(z.number().optional()).describe(
      'Skip first N lines/entries before applying head_limit, equivalent to "| tail -n +N | head -N". Works across all output modes. Defaults to 0.',
    ),
    multiline: semanticBoolean(z.boolean().optional()).describe(
      'Enable multiline mode where . matches newlines and patterns can span lines (rg -U --multiline-dotall). Default: false.',
    ),
  }),
)
```

#### 3.2.2 Output schema (verbatim from `GrepTool.ts:144-155`)

```ts
const outputSchema = lazySchema(() =>
  z.object({
    mode: z.enum(['content', 'files_with_matches', 'count']).optional(),
    numFiles: z.number(),
    filenames: z.array(z.string()),
    content: z.string().optional(),
    numLines: z.number().optional(), // For content mode
    numMatches: z.number().optional(), // For count mode
    appliedLimit: z.number().optional(), // The limit that was applied (if any)
    appliedOffset: z.number().optional(), // The offset that was applied
  }),
)
```

#### 3.2.3 Builder fields (`GrepTool.ts:160-310`)

- `name = GREP_TOOL_NAME` (`'Grep'`).
- `searchHint = 'search file contents with regex (ripgrep)'` (`GrepTool.ts:162`).
- `maxResultSizeChars = 20_000` (`GrepTool.ts:164` — "20K chars - tool result persistence threshold").
- `strict = true` (`GrepTool.ts:165`).
- `userFacingName() = 'Search'` (`GrepTool.ts:169-171`).
- `getActivityDescription(input)` formats `summary ? "Searching for ${summary}" : "Searching"` (`GrepTool.ts:173-176`).
- `isConcurrencySafe() = true`, `isReadOnly() = true` (`GrepTool.ts:183-188`).
- `toAutoClassifierInput(input) = input.path ? "${input.pattern} in ${input.path}" : input.pattern` (`GrepTool.ts:189-191`).
- `isSearchOrReadCommand() = { isSearch: true, isRead: false }` (`GrepTool.ts:192-194`).
- `getPath({ path }) = path || getCwd()` (`GrepTool.ts:195-197`). (Distinct from Glob, which calls `expandPath`.)
- `preparePermissionMatcher({ pattern }) = rulePattern => matchWildcardPattern(rulePattern, pattern)` (`GrepTool.ts:198-200`).
- `extractSearchText`: returns `content` when `mode==='content'`, else `filenames.join('\n')` (`GrepTool.ts:250-253`).

### 3.3 ToolSearch

#### 3.3.1 Input schema (verbatim from `ToolSearchTool.ts:21-34`)

```ts
export const inputSchema = lazySchema(() =>
  z.object({
    query: z
      .string()
      .describe(
        'Query to find deferred tools. Use "select:<tool_name>" for direct selection, or keywords to search.',
      ),
    max_results: z
      .number()
      .optional()
      .default(5)
      .describe('Maximum number of results to return (default: 5)'),
  }),
)
```

#### 3.3.2 Output schema (verbatim from `ToolSearchTool.ts:37-44`)

```ts
export const outputSchema = lazySchema(() =>
  z.object({
    matches: z.array(z.string()),
    query: z.string(),
    total_deferred_tools: z.number(),
    pending_mcp_servers: z.array(z.string()).optional(),
  }),
)
```

#### 3.3.3 Builder fields (`ToolSearchTool.ts:304-471`)

- `name = TOOL_SEARCH_TOOL_NAME` (`'ToolSearch'`).
- `isEnabled() = isToolSearchEnabledOptimistic()` (`ToolSearchTool.ts:305-307`).
- `isConcurrencySafe() = true`, `isReadOnly() = true` (`ToolSearchTool.ts:308-313`).
- `maxResultSizeChars = 100_000` (`ToolSearchTool.ts:315`).
- `description()` and `prompt()` both return `getPrompt()` (`ToolSearchTool.ts:316-321`).
- `userFacingName = () => ''` (`ToolSearchTool.ts:438`); `renderToolUseMessage()` returns `null` (`ToolSearchTool.ts:435-437`). The tool is intentionally invisible to the user.

### 3.4 Registration sites (verbatim from `src/tools.ts`)

- `tools.ts:201` — `...(hasEmbeddedSearchTools() ? [] : [GlobTool, GrepTool]),`
- `tools.ts:247-249` —
  ```ts
  // Include ToolSearchTool when tool search might be enabled (optimistic check)
  ...(isToolSearchEnabledOptimistic() ? [ToolSearchTool] : []),
  ```

---

## 4. Data Model & State

### 4.1 Glob shared data

- `Output` type derived from `outputSchema` (`GlobTool.ts:55`): `{ durationMs, numFiles, filenames, truncated }`.
- `glob()` return shape (`utils/glob.ts:72`): `Promise<{ files: string[]; truncated: boolean }>`.
- `extractGlobBaseDirectory(pattern)` returns `{ baseDir: string; relativePattern: string }` (`utils/glob.ts:17-64`).

### 4.2 Grep shared data

- `Output` derived from `outputSchema` (`GrepTool.ts:158`).
- `VCS_DIRECTORIES_TO_EXCLUDE = ['.git', '.svn', '.hg', '.bzr', '.jj', '.sl']` as const (`GrepTool.ts:95-102`).
- `DEFAULT_HEAD_LIMIT = 250` (`GrepTool.ts:108`).

### 4.3 ripgrep configuration state

- `RipgrepConfig` (`utils/ripgrep.ts:24-29`): `{ mode: 'system' | 'builtin' | 'embedded'; command: string; args: string[]; argv0?: string }`.
- `MAX_BUFFER_SIZE = 20_000_000` (20MB) (`utils/ripgrep.ts:80`).
- `getRipgrepConfig` is memoized (`utils/ripgrep.ts:31`) — picks system rg first (controlled by `USE_BUILTIN_RIPGREP=false`), then embedded (when `isInBundledMode()`), else vendored binary at `./vendor/ripgrep/<arch>-<platform>/rg{,.exe}`.
- `RipgrepTimeoutError` (`utils/ripgrep.ts:98-106`) carries `partialResults: string[]`.
- macOS-only codesign dance (`utils/ripgrep.ts:619-679`): runs once per process for builtin-mode rg, signs with `codesign --sign -` and removes `com.apple.quarantine` xattr.
- `ripgrepStatus` singleton (`utils/ripgrep.ts:524-528`) records `{ working, lastTested, config }` after the first `--version` probe.

### 4.4 ToolSearch in-memory state

- `cachedDeferredToolNames: string | null` (`ToolSearchTool.ts:50`) — sentinel for memoized description cache.
- `getToolDescriptionMemoized` (`ToolSearchTool.ts:66-86`) — `lodash-es/memoize` keyed by tool name, calls `tool.prompt({ getToolPermissionContext, tools, agents: [] })` with a stub permission context.
- `maybeInvalidateCache(deferredTools)` (`ToolSearchTool.ts:91-100`) — compares sorted, comma-joined tool-name list; clears cache when changed.
- `clearToolSearchDescriptionCache()` (`ToolSearchTool.ts:102-105`) — exported for external callers (e.g., MCP reconnects).

### 4.5 ToolSearch mode resolution (utils)

- `ToolSearchMode = 'tst' | 'tst-auto' | 'standard'` (`utils/toolSearch.ts:161`).
- `DEFAULT_AUTO_TOOL_SEARCH_PERCENTAGE = 10` (`utils/toolSearch.ts:49`).
- `CHARS_PER_TOKEN = 2.5` fallback heuristic (`utils/toolSearch.ts:99`).
- `DEFAULT_UNSUPPORTED_MODEL_PATTERNS = ['haiku']` (`utils/toolSearch.ts:204`).
- `loggedOptimistic` one-shot debug-log latch (`utils/toolSearch.ts:268`).

---

## 5. Algorithm / Control Flow

### 5.1 Glob (`GlobTool.call`, `GlobTool.ts:154-176`; `utils/glob.ts:66-130`)

Pseudocode:

```
GlobTool.call(input, ctx):
  start = Date.now()
  appState = ctx.getAppState()
  limit = ctx.globLimits?.maxResults ?? 100
  { files, truncated } = await glob(
    input.pattern,
    GlobTool.getPath(input),       // expandPath(path) or getCwd()
    { limit, offset: 0 },
    ctx.abortController.signal,
    appState.toolPermissionContext,
  )
  filenames = files.map(toRelativePath)
  return {
    data: {
      filenames,
      durationMs: Date.now() - start,
      numFiles: filenames.length,
      truncated,
    }
  }

glob(filePattern, cwd, { limit, offset }, abortSignal, toolPermissionContext):
  searchDir = cwd
  searchPattern = filePattern
  if isAbsolute(filePattern):
    { baseDir, relativePattern } = extractGlobBaseDirectory(filePattern)
    if baseDir:
      searchDir = baseDir
      searchPattern = relativePattern
  ignorePatterns = normalizePatternsToPath(
    getFileReadIgnorePatterns(toolPermissionContext),
    searchDir,
  )
  noIgnore = isEnvTruthy(env.CLAUDE_CODE_GLOB_NO_IGNORE || 'true')   # NOTE: || not ?? — empty string falls through to default 'true'
  hidden   = isEnvTruthy(env.CLAUDE_CODE_GLOB_HIDDEN    || 'true')   # same; CLAUDE_CODE_GLOB_HIDDEN="" behaves like unset, NOT explicit-disable
  args = [
    '--files',
    '--glob', searchPattern,
    '--sort=modified',
    ...(noIgnore ? ['--no-ignore'] : []),
    ...(hidden   ? ['--hidden']    : []),
  ]
  for p in ignorePatterns: args.push('--glob', `!${p}`)
  for x in await getGlobExclusionsForPluginCache(searchDir): args.push('--glob', x)
  allPaths = await ripGrep(args, searchDir, abortSignal)
  absolutePaths = allPaths.map(p => isAbsolute(p) ? p : join(searchDir, p))
  truncated = absolutePaths.length > offset + limit
  files = absolutePaths.slice(offset, offset + limit)
  return { files, truncated }
```

Pattern → match algorithm: `extractGlobBaseDirectory` (`utils/glob.ts:17-64`) finds the first character matching `/[*?[{]/`; everything before it is the static prefix. The split is at the last `/` or platform separator within that prefix; `lastSepIndex === 0` becomes baseDir `'/'`; on Windows, `^[A-Za-z]:$` becomes `<drive>:<sep>`. If pattern has no glob characters, `dirname()`/`basename()` are used (literal-path fallback). The relative pattern is then handed to ripgrep's `--glob`.

Sort order: `--sort=modified` (oldest first per ripgrep) (`utils/glob.ts:95-104`). `Glob.mapToolResultToToolResultBlockParam` returns the raw filename list, no re-sort (`GlobTool.ts:177-197`).

`mapToolResultToToolResultBlockParam` (`GlobTool.ts:177-197`): returns `'No files found'` when `filenames.length === 0`; otherwise joins all filenames, appending `'(Results are truncated. Consider using a more specific path or pattern.)'` iff `truncated` (verbatim).

Validation (`GlobTool.ts:94-134`): if `path` is provided, expand it; UNC paths (`\\\\` / `//`) skip stat (NTLM-leak guard). On ENOENT, suggests under-cwd alternative via `suggestPathUnderCwd`; non-directory paths return `errorCode: 2` with message `'Path is not a directory: ${path}'`.

### 5.2 Grep (`GrepTool.call`, `GrepTool.ts:310-576`)

Pseudocode (top-level):

```
GrepTool.call({pattern, path, glob, type, output_mode='files_with_matches',
               '-B':cb, '-A':ca, '-C':cc, context, '-n':showLines=true,
               '-i':ci=false, head_limit, offset=0, multiline=false},
              { abortController, getAppState }):
  absolutePath = path ? expandPath(path) : getCwd()
  args = ['--hidden']
  for dir in VCS_DIRECTORIES_TO_EXCLUDE: args.push('--glob', `!${dir}`)
  args.push('--max-columns', '500')                         # 500-col cap on output lines
  if multiline: args.push('-U', '--multiline-dotall')
  if ci:        args.push('-i')
  if output_mode === 'files_with_matches': args.push('-l')
  elif output_mode === 'count':            args.push('-c')
  if showLines && output_mode === 'content': args.push('-n')
  if output_mode === 'content':
    if context !== undefined:    args.push('-C', String(context))
    elif cc      !== undefined:  args.push('-C', String(cc))
    else:
      if cb !== undefined: args.push('-B', String(cb))
      if ca !== undefined: args.push('-A', String(ca))
  # Pattern dash-escape:
  if pattern.startsWith('-'): args.push('-e', pattern)
  else:                       args.push(pattern)
  if type: args.push('--type', type)
  if glob:
    rawPatterns = glob.split(/\s+/)
    globPatterns = []
    for raw in rawPatterns:
      if raw.includes('{') && raw.includes('}'):
        globPatterns.push(raw)
      else:
        globPatterns.push(...raw.split(',').filter(Boolean))
    for g in globPatterns.filter(Boolean):
      args.push('--glob', g)
  appState = getAppState()
  for p in normalizePatternsToPath(
            getFileReadIgnorePatterns(appState.toolPermissionContext),
            getCwd()):
    rgIgnorePattern = p.startsWith('/') ? `!${p}` : `!**/${p}`
    args.push('--glob', rgIgnorePattern)
  for x in await getGlobExclusionsForPluginCache(absolutePath):
    args.push('--glob', x)

  results = await ripGrep(args, absolutePath, abortController.signal)

  switch output_mode:
    case 'content':       return formatContentMode(results, head_limit, offset)
    case 'count':         return formatCountMode(results, head_limit, offset)
    default:              return formatFilesMode(results, head_limit, offset)  # files_with_matches
```

Per-mode pseudocode:

```
formatContentMode(results, head_limit, offset):
  { items, appliedLimit } = applyHeadLimit(results, head_limit, offset)
  finalLines = items.map(line => {
    i = line.indexOf(':')
    if i > 0:
      return toRelativePath(line.slice(0, i)) + line.slice(i)
    return line
  })
  return { data: {
    mode: 'content', numFiles: 0, filenames: [],
    content: finalLines.join('\n'),
    numLines: finalLines.length,
    ...(appliedLimit !== undefined && { appliedLimit }),
    ...(offset > 0 && { appliedOffset: offset }),
  }}

formatCountMode(results, head_limit, offset):
  { items, appliedLimit } = applyHeadLimit(results, head_limit, offset)
  finalCountLines = items.map(line => {
    i = line.lastIndexOf(':')
    if i > 0: return toRelativePath(line.slice(0, i)) + line.slice(i)
    return line
  })
  totalMatches = 0; fileCount = 0
  for line in finalCountLines:
    i = line.lastIndexOf(':')
    if i > 0:
      n = parseInt(line.slice(i+1), 10)
      if !isNaN(n): totalMatches += n; fileCount += 1
  return { data: {
    mode: 'count', numFiles: fileCount, filenames: [],
    content: finalCountLines.join('\n'),
    numMatches: totalMatches,
    ...(appliedLimit !== undefined && { appliedLimit }),
    ...(offset > 0 && { appliedOffset: offset }),
  }}

formatFilesMode(results, head_limit, offset):
  stats = await Promise.allSettled(results.map(p => fs.stat(p)))   # ENOENT → mtime 0
  sorted = results
    .map((p, i) => [p, stats[i].status === 'fulfilled' ? (stats[i].value.mtimeMs ?? 0) : 0])
    .sort((a, b) =>
      NODE_ENV === 'test'
        ? a[0].localeCompare(b[0])
        : (b[1] - a[1] !== 0 ? b[1] - a[1] : a[0].localeCompare(b[0])))
    .map(([p]) => p)
  { items: finalMatches, appliedLimit } = applyHeadLimit(sorted, head_limit, offset)
  relative = finalMatches.map(toRelativePath)
  return { data: {
    mode: 'files_with_matches', filenames: relative, numFiles: relative.length,
    ...(appliedLimit !== undefined && { appliedLimit }),
    ...(offset > 0 && { appliedOffset: offset }),
  }}

applyHeadLimit(items, limit, offset=0):
  if limit === 0: return { items: items.slice(offset), appliedLimit: undefined }   # 0 = unlimited
  effective = limit ?? DEFAULT_HEAD_LIMIT                                          # 250
  sliced = items.slice(offset, offset + effective)
  truncated = (items.length - offset) > effective
  return { items: sliced, appliedLimit: truncated ? effective : undefined }
```

Result formatting (`mapToolResultToToolResultBlockParam`, `GrepTool.ts:254-309`):

- Content: emits raw `content || 'No matches found'`, suffixing `'\n\n[Showing results with pagination = limit: N, offset: M]'` only when at least one of `appliedLimit`/`appliedOffset` is set (`formatLimitInfo` builds parts conditionally to avoid the literal `'limit: undefined'`, `GrepTool.ts:131-142`).
- Count: emits raw count lines plus a summary line `\n\nFound ${matches} total ${matches === 1 ? 'occurrence' : 'occurrences'} across ${files} ${files === 1 ? 'file' : 'files'}.${limitInfo ? ' with pagination = ' + limitInfo : ''}` (verbatim `:285`).
- files_with_matches: returns `'No files found'` for zero-result; otherwise `Found ${numFiles} ${plural(numFiles, 'file')}${limitInfo ? ' ' + limitInfo : ''}\n${filenames.join('\n')}` (`GrepTool.ts:303`).

`ripGrep()` retry/timeout flow (`utils/ripgrep.ts:345-462`):

```
ripGrep(args, target, abortSignal):
  await codesignRipgrepIfNecessary()
  void testRipgrepOnFirstUse()
  return new Promise:
    handleResult(error, stdout, stderr, isRetry):
      if !error: resolve(splitLines(stdout)); return
      if error.code === 1: resolve([]); return                    # "no matches"
      if error.code in {ENOENT, EACCES, EPERM}: reject(error); return
      if !isRetry && isEagainError(stderr):
        logEvent('tengu_ripgrep_eagain_retry')
        ripGrepRaw(args, target, abortSignal, cb, /*singleThread*/ true)
        return
      lines = stdout ? splitLines(stdout) : []
      isTimeout = error.signal in {SIGTERM, SIGKILL} || error.code === 'ABORT_ERR'
      isBufOver = error.code === 'ERR_CHILD_PROCESS_STDIO_MAXBUFFER'
      if lines.length > 0 && (isTimeout || isBufOver): lines = lines.slice(0, -1)
      if error.code !== 2 && error.code !== 'ABORT_ERR': logError(error)
      if isTimeout && lines.length === 0:
        reject(new RipgrepTimeoutError(
          `Ripgrep search timed out after ${platform==='wsl' ? 60 : 20} seconds. ...`,
          lines))
        return
      resolve(lines)
    ripGrepRaw(args, target, abortSignal, (e, out, err) => handleResult(e, out, err, false))

ripGrepRaw(args, target, abortSignal, callback, singleThread=false):
  { rgPath, rgArgs, argv0 } = ripgrepCommand()
  threadArgs = singleThread ? ['-j', '1'] : []
  fullArgs = [...rgArgs, ...threadArgs, ...args, target]
  defaultTimeout = platform==='wsl' ? 60_000 : 20_000
  parsedSeconds = parseInt(env.CLAUDE_CODE_GLOB_TIMEOUT_SECONDS || '', 10) || 0
  timeout = parsedSeconds > 0 ? parsedSeconds*1000 : defaultTimeout
  if argv0:
    spawn(rgPath, fullArgs, { argv0, signal: abortSignal, windowsHide: true })
    # SIGTERM after timeout; if !win32, escalate to SIGKILL after +5_000ms
    # 0/1 → callback(null, stdout, stderr); else error code
  else:
    execFile(rgPath, fullArgs, {
      maxBuffer: 20_000_000,
      signal: abortSignal,
      timeout,
      killSignal: platform==='win32' ? undefined : 'SIGKILL',
    }, callback)
```

Validation (`GrepTool.ts:201-232`): on ENOENT, message `'Path does not exist: ${path}. ${FILE_NOT_FOUND_CWD_NOTE} ${getCwd()}.'` (plus optional `'Did you mean ${cwdSuggestion}?'`). UNC paths skip stat (NTLM guard).

### 5.3 ToolSearch

Top-level `call` (`ToolSearchTool.ts:328-433`):

```
ToolSearchTool.call(input, { options: { tools }, getAppState }):
  query = input.query
  max_results = input.max_results ?? 5
  deferredTools = tools.filter(isDeferredTool)
  maybeInvalidateCache(deferredTools)

  selectMatch = query.match(/^select:(.+)$/i)
  if selectMatch:
    requested = selectMatch[1].split(',').map(trim).filter(Boolean)
    found = []; missing = []
    for name in requested:
      tool = findToolByName(deferredTools, name) ?? findToolByName(tools, name)
      if tool:
        if tool.name not in found: found.push(tool.name)
      else:
        missing.push(name)
    if found.length === 0:
      logEvent('tengu_tool_search_outcome', { ..., queryType: 'select', matchCount: 0 })
      return buildSearchResult([], query, deferredTools.length, getPendingServerNames())
    logEvent('tengu_tool_search_outcome', { ..., queryType: 'select', matchCount: found.length })
    return buildSearchResult(found, query, deferredTools.length)

  matches = await searchToolsWithKeywords(query, deferredTools, tools, max_results)
  logEvent('tengu_tool_search_outcome', { ..., queryType: 'keyword', matchCount: matches.length })
  if matches.length === 0:
    return buildSearchResult(matches, query, deferredTools.length, getPendingServerNames())
  return buildSearchResult(matches, query, deferredTools.length)

getPendingServerNames():
  pending = getAppState().mcp.clients.filter(c => c.type === 'pending')
  return pending.length > 0 ? pending.map(s => s.name) : undefined
```

Query syntax (`ToolSearchTool.ts:186-302`):

```
searchToolsWithKeywords(query, deferredTools, tools, maxResults):
  q = query.toLowerCase().trim()
  exact = deferredTools.find(t => t.name.toLowerCase() === q)
       ?? tools.find(t => t.name.toLowerCase() === q)
  if exact: return [exact.name]                    # bare-name fast path
  if q.startsWith('mcp__') && q.length > 5:
    prefix = deferredTools.filter(t => t.name.toLowerCase().startsWith(q))
                          .slice(0, maxResults).map(t => t.name)
    if prefix.length > 0: return prefix
  terms = q.split(/\s+/).filter(t => t.length > 0)
  required = []; optional = []
  for t in terms:
    if t.startsWith('+') && t.length > 1: required.push(t.slice(1))
    else: optional.push(t)
  scoringTerms = required.length > 0 ? [...required, ...optional] : terms
  termPatterns = compileTermPatterns(scoringTerms)            # \b<escaped>\b
  candidateTools = deferredTools
  if required.length > 0:
    candidateTools = (await for each tool: matchesAllRequired(tool)).filter(non-null)
  scored = await for each candidateTool:
    parsed = parseToolName(tool.name)
    desc = await getToolDescriptionMemoized(tool.name, tools)
    descNorm = desc.toLowerCase()
    hintNorm = tool.searchHint?.toLowerCase() ?? ''
    score = 0
    for term in scoringTerms:
      pat = termPatterns.get(term)
      if parsed.parts.includes(term): score += parsed.isMcp ? 12 : 10     # exact part
      elif parsed.parts.some(part => part.includes(term)): score += parsed.isMcp ? 6 : 5
      if parsed.full.includes(term) && score === 0: score += 3            # full-name fallback (see REIMPLEMENTER HAZARD below)
      if hintNorm && pat.test(hintNorm): score += 4                       # searchHint match
      if pat.test(descNorm): score += 2                                   # description match
    return { name: tool.name, score }
  return scored.filter(s => s.score > 0)
               .sort((a,b) => b.score - a.score)
               .slice(0, maxResults)
               .map(s => s.name)

parseToolName(name):
  if name.startsWith('mcp__'):
    body = name.replace(/^mcp__/, '').toLowerCase()
    parts = body.split('__').flatMap(p => p.split('_'))
    return { parts: parts.filter(Boolean), full: body.replace(/__/g, ' ').replace(/_/g, ' '), isMcp: true }
  parts = name
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/_/g, ' ')
    .toLowerCase()
    .split(/\s+/).filter(Boolean)
  return { parts, full: parts.join(' '), isMcp: false }

compileTermPatterns(terms):
  patterns = Map()
  for t in terms: if !patterns.has(t): patterns.set(t, new RegExp(`\\b${escapeRegExp(t)}\\b`))
  return patterns
```

**REIMPLEMENTER HAZARD — cross-term scoring coupling (`ToolSearchTool.ts:278-280`).** The `score === 0` guard on the full-name fallback (+3) reads the **running accumulator across all preceding terms**, not a per-term subscore. Concretely: if term-1 has any part-match, substring, hint, or description hit (which all produce a positive score), term-2's full-name fallback can never fire, even if term-2 only matches the full name. The pseudocode replicates this verbatim — preserving it is a bit-exact requirement. A reimplementer who refactors the inner loop into per-term subscores (the obvious "cleanup") will produce different rankings on multi-term queries while believing they implement the spec. **Do not factor `score === 0` into a per-iteration variable.** Treat the cross-term coupling as load-bearing, not a bug.


Result block (`ToolSearchTool.ts:444-470`): returns `tool_result` content as an array of `{ type: 'tool_reference', tool_name }` blocks; on zero matches, returns plain text `'No matching deferred tools found'`, optionally suffixed with `'. Some MCP servers are still connecting: ${names.join(', ')}. Their tools will become available shortly — try searching again.'` when `pending_mcp_servers` is non-empty.

### 5.4 `isDeferredTool` decision (verbatim from `ToolSearchTool/prompt.ts:62-108`)

```
isDeferredTool(tool):
  if tool.alwaysLoad === true: return false                 # explicit opt-out wins
  if tool.isMcp === true:      return true                  # MCP always deferred
  if tool.name === TOOL_SEARCH_TOOL_NAME: return false      # never defer self
  if feature('FORK_SUBAGENT') && tool.name === AGENT_TOOL_NAME:
    require AgentTool/forkSubagent.js
    if isForkSubagentEnabled(): return false                # Agent must be turn-1
  if (feature('KAIROS') || feature('KAIROS_BRIEF'))
     && BRIEF_TOOL_NAME && tool.name === BRIEF_TOOL_NAME:
    return false                                            # Brief is comm channel
  if feature('KAIROS') && SEND_USER_FILE_TOOL_NAME
     && tool.name === SEND_USER_FILE_TOOL_NAME && isReplBridgeActive():
    return false                                            # SendUserFile sibling
  return tool.shouldDefer === true
```

`formatDeferredToolLine(tool) = tool.name` (`prompt.ts:115-117`). Search hints are intentionally not rendered here ("hints A/B `exp_xenhnnmn0smrx4`, stopped Mar 21 — no benefit", `prompt.ts:111-114`).

### 5.5 ToolSearch mode resolution (`utils/toolSearch.ts:172-198`)

```
getToolSearchMode():
  if isEnvTruthy(env.CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS): return 'standard'
  v = env.ENABLE_TOOL_SEARCH
  autoPercent = v ? parseAutoPercentage(v) : null   # parseAutoPercentage parses 'auto:N'
  if autoPercent === 0:   return 'tst'
  if autoPercent === 100: return 'standard'
  if isAutoToolSearchMode(v): return 'tst-auto'    # 'auto' or 'auto:1..99'
  if isEnvTruthy(v):      return 'tst'
  if isEnvDefinedFalsy(env.ENABLE_TOOL_SEARCH): return 'standard'
  return 'tst'                                     # default

isToolSearchEnabledOptimistic():
  if getToolSearchMode() === 'standard': return false
  if !env.ENABLE_TOOL_SEARCH
     && getAPIProvider() === 'firstParty'
     && !isFirstPartyAnthropicBaseUrl():
    return false                                   # proxy heuristic, default-only
  return true
```

`isToolSearchEnabled(model, tools, getCtx, agents, source?)` (`utils/toolSearch.ts:385-473`) is the request-time check: rejects when `!modelSupportsToolReference(model)`, when `!isToolSearchToolAvailable(tools)`, or when mode resolves to `'standard'`. For `'tst-auto'` it calls `checkAutoThreshold` (see below).

> **OUT-OF-SCOPE POINTER — `checkAutoThreshold`.** The auto-threshold gate's full control flow (~80 lines, `utils/toolSearch.ts:385-473` plus the helpers below) is not specified bit-exactly here — this section provides the shape only. A reimplementer cannot reproduce the exact gating decision from this spec and must read source. The shape is:
>
> ```
> checkAutoThreshold(model, tools, getCtx, agents, percentage):
>   contextWindow = resolveContextWindow(model)            # MODEL → number lookup; failure ⇒ disabled
>   deferredTools = tools.filter(isDeferredTool)
>   exact = getDeferredToolTokenCount(deferredTools, getCtx, agents)   # may use anthropic API token-count (cached)
>   if exact !== null:
>     return exact >= floor(contextWindow * percentage / 100)
>   # Fallback: char heuristic
>   chars = calculateDeferredToolDescriptionChars(deferredTools, getCtx, agents)
>   threshold = floor(contextWindow * percentage * CHARS_PER_TOKEN / 100)   # CHARS_PER_TOKEN = 2.5
>   return chars >= threshold
> ```
>
> Edge cases that this spec does **not** enumerate but exist in source: GrowthBook flag overrides for the percentage value; the token-count cache key (model + tool set hash); how `getCtx` errors are swallowed; behavior when `resolveContextWindow` returns 0 or undefined; and the precise control-flow when the exact-count path partially fails (returns `null` vs. throwing). For bit-exact reproduction, treat `checkAutoThreshold` as an opaque dependency and consult `utils/toolSearch.ts:385-473` directly.

---

## 6. Verbatim Assets

### 6.1 Glob prompt (verbatim, `GlobTool/prompt.ts:1-7`)

```
- Fast file pattern matching tool that works with any codebase size
- Supports glob patterns like "**/*.js" or "src/**/*.ts"
- Returns matching file paths sorted by modification time
- Use this tool when you need to find files by name patterns
- When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the Agent tool instead
```

### 6.2 Grep prompt (verbatim, `GrepTool/prompt.ts:6-18`, with the `${GREP_TOOL_NAME}` / `${BASH_TOOL_NAME}` / `${AGENT_TOOL_NAME}` interpolations literally as `Grep`, `Bash`, `Agent`)

```
A powerful search tool built on ripgrep

  Usage:
  - ALWAYS use Grep for search tasks. NEVER invoke `grep` or `rg` as a Bash command. The Grep tool has been optimized for correct permissions and access.
  - Supports full regex syntax (e.g., "log.*Error", "function\s+\w+")
  - Filter files with glob parameter (e.g., "*.js", "**/*.tsx") or type parameter (e.g., "js", "py", "rust")
  - Output modes: "content" shows matching lines, "files_with_matches" shows only file paths (default), "count" shows match counts
  - Use Agent tool for open-ended searches requiring multiple rounds
  - Pattern syntax: Uses ripgrep (not grep) - literal braces need escaping (use `interface\{\}` to find `interface{}` in Go code)
  - Multiline matching: By default patterns match within single lines only. For cross-line patterns like `struct \{[\s\S]*?field`, use `multiline: true`
```

### 6.3 ToolSearch prompt — assembly (verbatim, `ToolSearchTool/prompt.ts:27-51`, `:119-121`)

```
PROMPT_HEAD = `Fetches full schema definitions for deferred tools so they can be called.\n\n`

getToolLocationHint():
  deltaEnabled = (process.env.USER_TYPE === 'ant' ||
                  getFeatureValue_CACHED_MAY_BE_STALE('tengu_glacier_2xr', false))
  return deltaEnabled
    ? 'Deferred tools appear by name in <system-reminder> messages.'
    : 'Deferred tools appear by name in <available-deferred-tools> messages.'

PROMPT_TAIL = ` Until fetched, only the name is known — there is no parameter schema, so the tool cannot be invoked. This tool takes a query, matches it against the deferred tool list, and returns the matched tools' complete JSONSchema definitions inside a <functions> block. Once a tool's schema appears in that result, it is callable exactly like any tool defined at the top of the prompt.

Result format: each matched tool appears as one <function>{"description": "...", "name": "...", "parameters": {...}}</function> line inside the <functions> block — the same encoding as the tool list at the top of this prompt.

Query forms:
- "select:Read,Edit,Grep" — fetch these exact tools by name
- "notebook jupyter" — keyword search, up to max_results best matches
- "+slack send" — require "slack" in the name, rank by remaining terms`

getPrompt() = PROMPT_HEAD + getToolLocationHint() + PROMPT_TAIL
```

ANT-only behavioral diff (`prompt.ts:35-42`): `USER_TYPE === 'ant'` (or GrowthBook flag `tengu_glacier_2xr=true`) replaces the substring `'<available-deferred-tools>'` with `'<system-reminder>'`. No other content differs.

### 6.4 Constants table

| Constant | Value | Source |
|---|---|---|
| Glob default `limit` | `100` (via `globLimits?.maxResults ?? 100`) | `GlobTool.ts:157` |
| Glob `maxResultSizeChars` | `100_000` | `GlobTool.ts:60` |
| Grep `maxResultSizeChars` | `20_000` | `GrepTool.ts:164` |
| Grep `DEFAULT_HEAD_LIMIT` | `250` | `GrepTool.ts:108` |
| Grep ripgrep `--max-columns` | `500` | `GrepTool.ts:338` |
| Grep VCS exclusions | `['.git', '.svn', '.hg', '.bzr', '.jj', '.sl']` | `GrepTool.ts:95-102` |
| ripgrep default timeout | `20_000`ms (linux/mac/win); `60_000`ms on wsl | `utils/ripgrep.ts:130` |
| ripgrep timeout env override | `CLAUDE_CODE_GLOB_TIMEOUT_SECONDS` (seconds) | `utils/ripgrep.ts:131-133` |
| ripgrep SIGTERM → SIGKILL escalation | 5_000ms (non-Windows, embedded mode) | `utils/ripgrep.ts:175-181` |
| ripgrep `MAX_BUFFER_SIZE` | `20_000_000` (20MB) | `utils/ripgrep.ts:80` |
| Glob env: `CLAUDE_CODE_GLOB_NO_IGNORE` | default `'true'` (resolved with `\|\|`, not `??` — empty-string env var falls through to `'true'`) | `utils/glob.ts:98` |
| Glob env: `CLAUDE_CODE_GLOB_HIDDEN`    | default `'true'` (same `\|\|` quirk; setting the var to `""` is **not** an explicit-disable, behaves like unset) | `utils/glob.ts:99` |
| Embedded ripgrep flags | `['--no-config']` | `utils/ripgrep.ts:53` |
| Builtin rg vendored path | `vendor/ripgrep/<arch>-<platform>/rg{,.exe}` | `utils/ripgrep.ts:58-62` |
| ToolSearch `max_results` default | `5` | `ToolSearchTool.ts:31-32` |
| ToolSearch `maxResultSizeChars` | `100_000` | `ToolSearchTool.ts:315` |
| ToolSearch keyword scoring weights | exact-part: MCP 12 / non-MCP 10; substring-part: MCP 6 / non-MCP 5; full-name fallback (only when `score === 0`): 3; `searchHint` match: 4; description match: 2 | `ToolSearchTool.ts:271-289` |
| ToolSearch auto threshold default | `10%` of context window (token-based) | `utils/toolSearch.ts:49`, `:104-109` |
| ToolSearch `CHARS_PER_TOKEN` fallback | `2.5` | `utils/toolSearch.ts:99` |
| Tools forced to never defer | `ToolSearch` always; `Agent` when `feature('FORK_SUBAGENT')` && `isForkSubagentEnabled()`; Brief tool when `feature('KAIROS') \|\| feature('KAIROS_BRIEF')`; SendUserFile when `feature('KAIROS')` && `isReplBridgeActive()` | `ToolSearchTool/prompt.ts:62-107` |

### 6.5 ANT prompt-text variant — exact diff

| Where | Default text | ANT (`USER_TYPE === 'ant'`) text |
|---|---|---|
| `getToolLocationHint()` (`prompt.ts:35-42`) | `Deferred tools appear by name in <available-deferred-tools> messages.` | `Deferred tools appear by name in <system-reminder> messages.` |

### 6.6 ripgrep CLI invocation construction — verbatim ordering

Order in which `args` is built before `target` is appended (Grep): the array is **initialized literally** as `args = ['--hidden']` (`GrepTool.ts:330`) — `--hidden` is *position 0*, not appended later — then `--glob '!.git'`, `--glob '!.svn'`, `--glob '!.hg'`, `--glob '!.bzr'`, `--glob '!.jj'`, `--glob '!.sl'` (`GrepTool.ts:333-336`), then `--max-columns 500` (`:338`), then optionally `-U --multiline-dotall`, then `-i`, then mode flag (`-l` or `-c`), then `-n` (content+show_line_numbers only), then context flags (`-C` precedence: `context > -C > (-B then -A)`), then pattern (with `-e` if it starts with `-`), then `--type <type>`, then expanded `--glob <pat>` entries from the `glob` parameter, then negated ignore patterns from the permission context, then plugin-cache exclusions. Finally `target` is appended by `ripGrepRaw` (`utils/ripgrep.ts:127`). Reimplementers must initialize the array with `'--hidden'` already present; appending it later (e.g. between mode flags and the pattern) breaks bit-exactness even though ripgrep's behavior is order-insensitive for these flags.

For `glob()` (Glob tool): `--files`, `--glob <pattern>`, `--sort=modified`, optionally `--no-ignore`, optionally `--hidden`, then negated ignore patterns, then plugin-cache exclusions.

### 6.7 ANT-bundle replacement (`hasEmbeddedSearchTools`, verbatim from `utils/embeddedTools.ts:15-21`)

```ts
export function hasEmbeddedSearchTools(): boolean {
  if (!isEnvTruthy(process.env.EMBEDDED_SEARCH_TOOLS)) return false
  const e = process.env.CLAUDE_CODE_ENTRYPOINT
  return (
    e !== 'sdk-ts' && e !== 'sdk-py' && e !== 'sdk-cli' && e !== 'local-agent'
  )
}
```

Effect (per file header, `embeddedTools.ts:3-14`): when true, `find` and `grep` in Claude's Bash shell are shadowed by shell functions invoking the bun binary with `argv0='bfs'` / `argv0='ugrep'`; the dedicated `Glob`/`Grep` tools are removed from the registry (`tools.ts:201`); steering prompts are omitted (Bash prompt and Agent prompts gate on this — see `BashTool/prompt.ts:278`, `AgentTool/prompt.ts:222`, `constants/prompts.ts:289`, `:360`, `memdir/memdir.ts:385`).

### 6.8 Embedded-ripgrep dispatch (verbatim `utils/ripgrep.ts:48-57`)

```ts
if (isInBundledMode()) {
  return {
    mode: 'embedded',
    command: process.execPath,
    args: ['--no-config'],
    argv0: 'rg',
  }
}
```

---

## 7. Side Effects & I/O

### 7.1 Filesystem

- Glob: `fs.stat` on the optional `path` (UNC-skipped); ripgrep traversal of `searchDir`.
- Grep: `fs.stat` on the optional `path`; ripgrep traversal of `absolutePath`; `Promise.allSettled(fs.stat)` over each result for `files_with_matches` mtime sort (`GrepTool.ts:529-552`).
- ripgrep traversal honors `--hidden` always (Grep) or env-toggled (Glob); `.git` family excluded (Grep) or follows `--no-ignore` (Glob default).
- macOS-only: codesigning the vendored `rg` (`utils/ripgrep.ts:619-679`) — runs `codesign --sign -` and `xattr -d com.apple.quarantine` on first call when `config.mode === 'builtin'` and the binary is `linker-signed`.

### 7.2 Process spawn

- `execFile` (non-embedded) or `spawn` (embedded with `argv0`); both honor `abortSignal`. Non-Windows `execFile` uses `killSignal: 'SIGKILL'`. Embedded path manages its own SIGTERM-then-SIGKILL escalation (`utils/ripgrep.ts:174-182`).

### 7.3 Network — none.

### 7.4 Environment variables

| Var | Read by | Effect |
|---|---|---|
| `EMBEDDED_SEARCH_TOOLS` | `hasEmbeddedSearchTools()` | enables the bfs/ugrep replacement bundle |
| `CLAUDE_CODE_ENTRYPOINT` | `hasEmbeddedSearchTools()` | excluded entrypoints disable the replacement |
| `USE_BUILTIN_RIPGREP` | `getRipgrepConfig` | `false` lets system rg take precedence (security: command name `rg`, not absolute path) |
| `CLAUDE_CODE_GLOB_NO_IGNORE` | `glob()` | `'false'` makes glob respect `.gitignore` |
| `CLAUDE_CODE_GLOB_HIDDEN` | `glob()` | `'false'` excludes hidden files |
| `CLAUDE_CODE_GLOB_TIMEOUT_SECONDS` | `ripGrepRaw` | overrides the 20s/60s default |
| `ENABLE_TOOL_SEARCH` | `getToolSearchMode` / `isToolSearchEnabledOptimistic` | `'true'` / `'auto'` / `'auto:N'` / `'false'` / unset |
| `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS` | `getToolSearchMode` | hard-forces `'standard'` |
| `USER_TYPE` | `getToolLocationHint`, `isDeferredToolsDeltaEnabled` | `'ant'` switches the prompt-text variant and DTD attachments |
| `ANTHROPIC_BASE_URL` | `isToolSearchEnabledOptimistic` (via `isFirstPartyAnthropicBaseUrl`) | non-first-party default-only disables ToolSearch |
| `NODE_ENV` | Grep `files_with_matches` sort | `'test'` switches to deterministic filename sort |

### 7.5 External binaries

- `rg` (system) or vendored `rg` or embedded-ripgrep through `bun` self-spawn with `argv0='rg'`.
- `codesign`, `xattr` (macOS-only one-shot, builtin mode).

### 7.6 Trust boundaries

- All three tools' `checkPermissions` call `checkReadPermissionForTool` (Glob `:135-142`, Grep `:233-240`). Permission decision tree itself → 09.
- Both Glob and Grep call `getFileReadIgnorePatterns(toolPermissionContext)` to apply the user's denylist as negated ripgrep `--glob` patterns (`utils/glob.ts:86-89`, `GrepTool.ts:412-427`).
- ToolSearch makes no filesystem reads. It reads `appState.mcp.clients` (read-only) for the pending-server hint (`ToolSearchTool.ts:336-339`).

---

## 8. Feature Flags & Variants

### 8.1 Glob/Grep registration

- `tools.ts:201`: `hasEmbeddedSearchTools()` — when true, `[GlobTool, GrepTool]` are NOT added to the base tools list. The model uses `Bash` with shell functions backed by embedded `bfs`/`ugrep` instead. `EMBEDDED_SEARCH_TOOLS` env required AND `CLAUDE_CODE_ENTRYPOINT` must not be one of `'sdk-ts'|'sdk-py'|'sdk-cli'|'local-agent'`.

### 8.2 ToolSearch registration

- `tools.ts:249`: `isToolSearchEnabledOptimistic()` — returns false in `'standard'` mode OR (default-only) when `ANTHROPIC_BASE_URL` is non-first-party. Otherwise `[ToolSearchTool]` is added.

### 8.3 ToolSearch behavioral matrix (`ENABLE_TOOL_SEARCH`)

| Value | Mode | Behavior |
|---|---|---|
| unset / empty | `'tst'` (subject to base-url heuristic) | always defer MCP + `shouldDefer` tools; gate with the proxy heuristic |
| `'true'` (any truthy) | `'tst'` | always defer; user-asserts proxy support |
| `'false'` (any falsy) | `'standard'` | no deferral; ToolSearchTool not registered; all tools inline |
| `'auto'` | `'tst-auto'` | enable iff deferred-tool tokens ≥ 10% of context window |
| `'auto:0'` | `'tst'` | always enabled |
| `'auto:100'` | `'standard'` | always disabled |
| `'auto:N'` (1..99) | `'tst-auto'` | threshold = N% |
| `CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=true` | force `'standard'` | proxy kill switch |

### 8.4 ToolSearch prompt variants

- ANT vs. non-ANT: only the substring `'<available-deferred-tools>'` ↔ `'<system-reminder>'` differs (`prompt.ts:35-42`). GrowthBook `tengu_glacier_2xr=true` flips non-ANT to the ANT variant.

### 8.5 `isDeferredTool` overrides (per `prompt.ts:62-108`)

- `feature('FORK_SUBAGENT')` + `isForkSubagentEnabled()` ⇒ `Agent` not deferred.
- `feature('KAIROS') || feature('KAIROS_BRIEF')` + `BRIEF_TOOL_NAME` present ⇒ Brief not deferred.
- `feature('KAIROS')` + `SEND_USER_FILE_TOOL_NAME` + `isReplBridgeActive()` ⇒ SendUserFile not deferred.

### 8.6 `modelSupportsToolReference` (`utils/toolSearch.ts:204`, `:239-252`)

`DEFAULT_UNSUPPORTED_MODEL_PATTERNS = ['haiku']`; GrowthBook flag `tengu_tool_search_unsupported_models` may replace the list. Negative test: a model is supported unless it `.includes()` any pattern (case-insensitive).

### 8.7 `isDeferredToolsDeltaEnabled` (`utils/toolSearch.ts:629-634`)

True iff `USER_TYPE === 'ant'` OR GrowthBook `tengu_glacier_2xr=true`. Shapes whether the deferred-tool announcement uses persisted attachments or a per-call header prepend (the latter is the prompt-text variant in §6.5).

---

## 9. Error Handling & Edge Cases

### 9.1 Glob

- Validation `errorCode: 1` for ENOENT path (`GlobTool.ts:118`); `errorCode: 2` for non-directory (`:128`).
- UNC paths (`\\\\` or `//`) skip stat — NTLM credential-leak guard (`GlobTool.ts:101-103`).
- ENOENT fallback: `suggestPathUnderCwd(absolutePath)` may append `'Did you mean ${cwdSuggestion}?'`.
- Empty-result tool_result content: `'No files found'`.
- Truncation tail: `'(Results are truncated. Consider using a more specific path or pattern.)'` joined onto the filename list (`GlobTool.ts:189-194`).
- **Symlink semantics:** `glob()` builds ripgrep args without `-L` / `--follow` (`utils/glob.ts:100-117`); ripgrep's documented default is **NOT** to follow symlinks during traversal. The optional `path` validation uses `fs.stat` (follows symlinks), not `lstat` — a symlink to a non-existent target produces ENOENT and triggers the validation-error path; a symlink to a directory passes the `isDirectory()` check and ripgrep then walks its target (without following further symlinks inside). `extractGlobBaseDirectory` operates on path strings only and does not normalize symlinked baseDirs — patterns like `'/symlink/**/*.ts'` walk through the symlink path literally; `toRelativePath` may emit paths whose round-trip through cwd diverges from the user-visible filesystem. See spec 11 §X for the shared `stat`-not-`lstat` symlink convention used across read tools.

### 9.2 Grep

- ENOENT path → `errorCode: 1` with prefix `'Path does not exist: '`. Non-directory paths are not separately rejected (Grep accepts files and directories — `rg PATH`).
- UNC skip identical to Glob.
- Pattern-leading-dash handling: `args.push('-e', pattern)` to avoid being parsed as flag (`GrepTool.ts:380-383`).
- `glob` parameter parsing preserves brace alternations: split by whitespace; for each chunk, if it contains both `'{'` and `'}'`, push as-is; else split by commas (`GrepTool.ts:391-409`). Empty chunks filtered.
- Ignore-pattern negation: absolute (leading `/`) → `'!${pat}'`; relative → `'!**/${pat}'` (per ripgrep gitignore semantics, `GrepTool.ts:412-427`).
- `head_limit === 0` is an explicit unlimited escape hatch (`applyHeadLimit`, `GrepTool.ts:115-117`).
- Mtime sort: `Promise.allSettled` so one ENOENT (file deleted between rg and stat) doesn't reject the whole batch — failed stats sort at mtime 0 (`GrepTool.ts:529-538`). The same ENOENT bucket also catches **broken symlinks** (`fs.stat` follows the link and fails; the entry sorts as mtime 0 instead of being elided).
- `--max-columns 500` (unconditional, `GrepTool.ts:338`) **does not silently drop** long matching lines: in `content` mode, ripgrep emits a sentinel marker line `[Omitted long matching line]` in place of any line whose match exceeds 500 columns (no `--max-columns-preview` is set). For `files_with_matches` (`-l`) and `count` (`-c`) modes the option is ignored by ripgrep (file/count emission short-circuits before column truncation). A reimplementer raising the cap (e.g. to 1000) silently widens the noise band; lowering it widens the marker output. The verbatim marker text comes from ripgrep itself, not from this codebase.
- `NODE_ENV === 'test'` branch sorts by filename for determinism (`GrepTool.ts:543-545`).
- Empty-result tool_result content: `'No files found'` (files_with_matches), `'No matches found'` (content/count).
- **Symlink semantics:** `GrepTool.call` builds ripgrep args without `-L` / `--follow` (§6.6); ripgrep's default is **NOT** to follow symlinks during traversal. The `Promise.allSettled(fs.stat(...))` mtime sort uses `stat` (follows symlinks), not `lstat`: a symlink whose target was deleted between rg's emission and the stat call produces ENOENT and the entry sorts as mtime 0 (same bucket as race-deleted files). See spec 11 §X for the shared `stat`-not-`lstat` symlink convention.

### 9.3 ripgrep wrapper

- Exit code 1 = "no matches" → resolves with `[]` (`utils/ripgrep.ts:377-380`).
- Critical errors `ENOENT|EACCES|EPERM` reject (`:384-388`).
- EAGAIN ("os error 11" or "Resource temporarily unavailable" in stderr) triggers single-threaded retry once (`-j 1`); telemetry `tengu_ripgrep_eagain_retry` (`:394-409`).
- Timeout: signal `SIGTERM`/`SIGKILL` or code `'ABORT_ERR'`; if no partial output, throws `RipgrepTimeoutError` with text `'Ripgrep search timed out after ${platform==='wsl' ? 60 : 20} seconds. The search may have matched files but did not complete in time. Try searching a more specific path or pattern.'`. The trailing line of partial stdout is dropped on timeout / `ERR_CHILD_PROCESS_STDIO_MAXBUFFER` to avoid torn lines (`:413-431`).
- Code 2 (rg usage error) and `'ABORT_ERR'` are not `logError`'d (`:438-442`).
- Windows: `child.kill()` (default) instead of `'SIGTERM'` because the latter throws on Windows; `execFile` `killSignal` left undefined on Windows for the same reason (`:174-181`, `:226-228`).

### 9.4 ToolSearch

- Empty `select:` results: when none of the requested names resolve (in either `deferredTools` or full `tools`), returns `matches=[]`, optionally `pending_mcp_servers`. Logs `'ToolSearchTool: select failed — none found: ${missing}'`.
- Partial select: `found` returned; `missing` is logged but not surfaced to the model.
- Bare-name fast path: an exact (case-insensitive) match against `deferredTools` first, then full `tools`. Selecting an already-loaded tool is a "harmless no-op" (`ToolSearchTool.ts:198-204`).
- Empty keyword search returns plain-text `'No matching deferred tools found'`, optionally suffixed with the pending-server hint (`ToolSearchTool.ts:448-460`).
- The `tool_reference` block format "works on 1P/Foundry. Bedrock/Vertex may not support client-side tool_reference expansion yet" (`ToolSearchTool.ts:439-443`) — the proxy heuristic in `isToolSearchEnabledOptimistic` is the only client-side gate.

### 9.5 User-facing error messages (UI; verbatim)

- Glob+Grep error render (`UI.tsx`): on `tool_use_error` containing `FILE_NOT_FOUND_CWD_NOTE` → `<Text color="error">File not found</Text>`; otherwise → `<Text color="error">Error searching files</Text>` (Grep `UI.tsx:147-164`, Glob `UI.tsx:33-50`).

---

## 10. Telemetry & Observability

| Event | Site | Payload |
|---|---|---|
| `tengu_tool_search_outcome` | `ToolSearchTool.ts:346-355` | `{ query, queryType: 'select'\|'keyword', matchCount, totalDeferredTools, maxResults, hasMatches }` |
| `tengu_tool_search_mode_decision` | `utils/toolSearch.ts:401-415` | `{ enabled, mode, reason, checkedModel, mcpToolCount, userType, ...metrics }` |
| `tengu_deferred_tools_pool_change` | `utils/toolSearch.ts:685-699` | `{ addedCount, removedCount, priorAnnouncedCount, messagesLength, attachmentCount, dtdCount, callSite, querySource, attachmentTypesSeen }` |
| `tengu_ripgrep_eagain_retry` | `utils/ripgrep.ts:398` | `{}` |
| `tengu_ripgrep_availability` | `utils/ripgrep.ts:605-608` | `{ working: 0\|1, using_system: 0\|1 }` |
| `logForDebugging` | many | `'ToolSearchTool: cache invalidated …'` (`:94`); `'ToolSearchTool: select failed — none found: …'` (`:386`); `'ToolSearchTool: partial select — found: …, missing: …'` / `'ToolSearchTool: selected …'` (`:399-403`); `'ToolSearchTool: keyword search for "${query}", found ${n} matches'` (`:417`); `'rg EAGAIN error detected, retrying with single-threaded mode (-j 1)'` (`utils/ripgrep.ts:395`); `'rg error (signal=…, code=…, stderr: …), N results'` (`:433-435`); `'Ripgrep first use test: PASSED|FAILED (mode=…, path=…)'` (`:600-602`) |

`getRipgrepStatus()` (`utils/ripgrep.ts:535-546`) surfaces the cached `{ mode, path, working }` to consumers (e.g., a startup banner).

---

## 11. Reimplementation Checklist

- [ ] `Glob` builds args in this exact order: `--files`, `--glob <pattern>`, `--sort=modified`, env-gated `--no-ignore` and `--hidden`, then negated user ignores, then plugin-cache exclusions (`utils/glob.ts:100-117`).
- [ ] `Glob` returns at most `globLimits?.maxResults ?? 100` files; `truncated = absolutePaths.length > offset + limit` (`utils/glob.ts:126`).
- [ ] `Glob` extracts a static base directory using the regex `/[*?[{]/` and the algorithm in §5.1; preserves Windows drive-root semantics (`utils/glob.ts:53-61`).
- [ ] `Grep` builds args in the order in §6.6; passes patterns starting with `-` via `-e`.
- [ ] `Grep` parses the `glob` parameter splitting on whitespace, then commas (preserving brace alternations), and adds one `--glob` per chunk (`GrepTool.ts:391-409`).
- [ ] `Grep` excludes the six VCS dirs `.git .svn .hg .bzr .jj .sl` (`GrepTool.ts:95-102`).
- [ ] `Grep` files-mode sort: by mtime descending using `Promise.allSettled(fs.stat)`, with filename `localeCompare` tiebreak; `NODE_ENV === 'test'` switches to filename-only.
- [ ] `applyHeadLimit` semantics: `limit === 0` ⇒ unlimited (still respects `offset`); else `effective = limit ?? 250`; only report `appliedLimit` when truncation occurred (`GrepTool.ts:110-128`).
- [ ] `formatLimitInfo` only emits parts when set, never the literal `'limit: undefined'` (`GrepTool.ts:131-142`).
- [ ] `Grep` content-mode tool result body suffix: `\n\n[Showing results with pagination = …]` (only when limit info is non-empty).
- [ ] `Grep` count-mode summary line: `\n\nFound ${n} total ${n===1?'occurrence':'occurrences'} across ${f} ${f===1?'file':'files'}.${limitInfo?' with pagination = ' + limitInfo : ''}` (`GrepTool.ts:285`).
- [ ] `ripGrep` wrapper: 20s/60s default timeout (wsl); 20MB max buffer; SIGTERM-then-SIGKILL embedded escalation; exit code 1 ⇒ no-matches success; EAGAIN single-threaded retry; `RipgrepTimeoutError` only when timeout AND zero partial lines.
- [ ] `getRipgrepConfig`: prefer system rg (when `USE_BUILTIN_RIPGREP=false`), then bundled embedded (`argv0='rg'`, args `['--no-config']`), else vendored `rg`/`rg.exe`. SECURITY: command name (not absolute path) for system rg to defeat PATH hijacking.
- [ ] macOS codesign dance runs once per process and only for builtin mode when `linker-signed` is present.
- [ ] `ToolSearch` `select:` parser: case-insensitive prefix, comma-separated names; trims; preserves found-order without duplicates; resolves missing names against full `tools` to allow already-loaded selections.
- [ ] `ToolSearch` keyword scorer: `parseToolName`, word-boundary-anchored `\b<term>\b` regex per term, the exact weights in §6.4, sort descending, slice to `max_results`.
- [ ] `+`-prefix terms are required (pre-filter before scoring); when present, they participate in scoring alongside optional terms.
- [ ] Cache invalidation: sort+join deferred tool names; when changed, clear `getToolDescriptionMemoized`.
- [ ] `ToolSearch` result block: zero matches → plain text (with optional pending-MCP suffix); otherwise array of `{ type: 'tool_reference', tool_name }` blocks.
- [ ] `isDeferredTool` precedence per §5.4 — `alwaysLoad` wins, then `isMcp`, then ToolSearch self, then feature-gated escape hatches, then `shouldDefer`.
- [ ] `getToolSearchMode` precedence: experimental-betas kill switch → `auto:N` edge cases → auto modes → boolean truthy/falsy → default `'tst'`.
- [ ] `isToolSearchEnabledOptimistic` proxy heuristic only fires when `ENABLE_TOOL_SEARCH` is unset/empty AND provider is `'firstParty'` AND base URL is not first-party.
- [ ] ANT prompt-text variant swaps `<available-deferred-tools>` for `<system-reminder>`. No other prompt difference.
- [ ] Glob+Grep both implement `isSearchOrReadCommand` returning `{ isSearch: true, isRead: false }` so the UI shell can collapse them.
- [ ] Permission check delegates to `checkReadPermissionForTool` for both Glob and Grep (09 owns the decision); ToolSearch has no permission check (read-only metadata).

---

## 12. Open Questions / Unknowns

- `Tool.globLimits` (read at `GlobTool.ts:157` as `ctx.globLimits?.maxResults`) — origin and lifetime not in scope here. Defined on `ToolUseContext` (08); not investigated for caller-side configuration.
- `extractGlobBaseDirectory` is called by Glob but also exported (used elsewhere?). Downstream consumers not enumerated; out of scope for this spec.
- `GrepTool.preparePermissionMatcher` matches the *regex* against rule patterns via `matchWildcardPattern`; semantics for users specifying regex permission rules vs. wildcard rules — clarification belongs to 09.
- `ToolSearchTool.mapToolResultToToolResultBlockParam` casts via `as unknown as ToolResultBlockParam` because `tool_reference` is a beta block not in the SDK types (`ToolSearchTool.ts:467-469`). Whether the cast is preserved across all model APIs (Bedrock/Vertex/Foundry/1P) — see 03 / 22 for routing.
- The interplay between `ToolSearch` keyword search and the Skill discovery code path (overlap noted in master plan) — referred to spec 17.
- The GrowthBook flag `tengu_glacier_2xr` (gates `<system-reminder>` vs. `<available-deferred-tools>`) — payload schema and rollout shape not in this spec; see 26.
- The exact shape of `appState.mcp.clients[i].type` (used by `getPendingServerNames`) — owned by 23.
- `getGlobExclusionsForPluginCache(absolutePath)` returns plugin-cache exclusion globs but the algorithm is in `utils/plugins/orphanedPluginFilter.ts` — owned by 28.
- `Tool.searchHint` field is exercised here for scoring but its catalog (which tools set what hint) is partial; full inventory is per-tool (10..19).
- `isInBundledMode()` and `isReplBridgeActive()` predicates are referenced; their full definitions live in `utils/bundledMode.ts` and `bootstrap/state.ts` respectively (out of scope here; relevant to 01).
- The header note "Reuses Grep's render (UI.tsx:65)" in `GlobTool.ts:149-150` references a line in a peer file — the UI.tsx files are React-compiler-output and the `:65` line citation is the runtime cache-key index, not a logical row. This is a comment-as-code-coordinates pattern; reproducing requires preserving the React-compiler scaffold — see 37.
