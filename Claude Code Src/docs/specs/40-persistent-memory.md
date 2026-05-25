# Persistent Memory (`memdir/`) Specification

> **Scope owner**: persistent on-disk MEMORY.md / memdir storage layer. Adjacent specs: 05 (CLAUDE.md chain consumer), 29 (extraction pipeline), 41 (session state).

## 1. Purpose & Scope

The persistent-memory subsystem owns the file-based memory store that survives across sessions. It (a) resolves a project-scoped on-disk directory rooted at `~/.claude/projects/<sanitized-git-root>/memory/`, (b) materializes the typed-memory behavioral prompt that the model writes against, (c) reads `MEMORY.md` (the index) into context with line+byte truncation, (d) scans memory `.md` files into a header manifest for relevance-driven recall, and (e) when `feature('TEAMMEM')` is on, manages a per-project shared `team/` subdirectory with hardened symlink-safe write validation.

In scope: the entire `src/memdir/` directory, the on-disk path layout, the `MEMORY.md` line/byte caps, the four-type memory taxonomy and prompt assets, `loadMemoryPrompt`, `buildMemoryPrompt`, `buildAssistantDailyLogPrompt`, `scanMemoryFiles`, `findRelevantMemories`, memory-age helpers, team-memory path validation, the `MEMORY_SHAPE_TELEMETRY` telemetry hook, and the `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` env var.

Out of scope (refer): CLAUDE.md project-instructions chain assembly → spec 05; the auto-extraction pipeline that populates memory at turn end (`extractMemories`) → spec 29; session transcripts and `session-memory/` paths → spec 41.

## 2. Source Map

### 2.1 Source coverage inventory

| File | Lines | Coverage |
|---|---:|---|
| `src/memdir/memdir.ts` | 507 | full |
| `src/memdir/paths.ts` | 278 | full |
| `src/memdir/memoryTypes.ts` | 271 | full |
| `src/memdir/teamMemPaths.ts` | 292 | full |
| `src/memdir/teamMemPrompts.ts` | 100 | full |
| `src/memdir/memoryScan.ts` | 94 | full |
| `src/memdir/findRelevantMemories.ts` | 141 | full |
| `src/memdir/memoryAge.ts` | 53 | full |
| `src/memdir/memoryShapeTelemetry.ts` | — | **missing-leaked-source**; referenced via lazy `require('./memoryShapeTelemetry.js')` at `src/memdir/findRelevantMemories.ts:69` under `feature('MEMORY_SHAPE_TELEMETRY')` (`:66`) |

Adjacent files cited but owned elsewhere: `src/utils/frontmatterParser.ts` (parseFrontmatter, owned by spec 02/05), `src/utils/memoryFileDetection.ts` (consumer-side detection), `src/bootstrap/state.ts` (cache setter, spec 03), `src/context.ts` (consumer of cache setter, spec 05).

### 2.2 Feature-flag and ANT guard locations

- `feature('TEAMMEM')` lazy require: `src/memdir/memdir.ts:7-9` and `:106-108`; `src/utils/memoryFileDetection.ts:17-19`.
- `feature('KAIROS')` daily-log dispatch: `src/memdir/memdir.ts:432`.
- `feature('MEMORY_SHAPE_TELEMETRY')` recall-shape logging: `src/memdir/findRelevantMemories.ts:66-72`.
- GrowthBook flags consumed: `tengu_passport_quail` (`paths.ts:70`), `tengu_slate_thimble` (`paths.ts:75`), `tengu_coral_fern` (`memdir.ts:376`), `tengu_moth_copse` (`memdir.ts:423`), `tengu_herring_clock` (`memdir.ts:503`, `teamMemPaths.ts:77`).
- No `USER_TYPE === 'ant'` gates inside `memdir/`.

### 2.3 Imports from

Core (cross-file): `bun:bundle` (feature), `path`, `os.homedir`, `fs/promises` (`readdir`, `lstat`, `realpath`), `lodash-es/memoize`, `../utils/fsOperations.getFsImplementation`, `../utils/frontmatterParser.parseFrontmatter`, `../utils/readFileInRange`, `../utils/git.findCanonicalGitRoot`, `../utils/path.sanitizePath`, `../utils/envUtils.{getClaudeConfigHomeDir,isEnvDefinedFalsy,isEnvTruthy}`, `../utils/settings/settings.{getInitialSettings,getSettingsForSource}`, `../utils/sideQuery.sideQuery`, `../utils/model/model.getDefaultSonnetModel`, `../utils/slowOperations.jsonParse`, `../utils/format.formatFileSize`, `../utils/sessionStorage.getProjectDir`, `../utils/embeddedTools.hasEmbeddedSearchTools`, `../tools/GrepTool/prompt.GREP_TOOL_NAME`, `../tools/REPLTool/constants.isReplModeEnabled`, `../services/analytics/{growthbook,index}`.

`bootstrap/state` is split across two memdir files (do not collapse): `memdir.ts:11` imports `{getKairosActive, getOriginalCwd}`; `paths.ts:7` imports `{getProjectRoot, getIsNonInteractiveSession}`.

### 2.4 Imported by (downstream consumers)

- `src/context.ts` — does NOT consume any memdir export. The `setCachedClaudeMdContent(claudeMd || null)` call at `:176` caches the CLAUDE.md chain (spec 05), unrelated to memdir.
- `src/utils/memoryFileDetection.ts` — `getAutoMemPath`, `isAutoMemPath`, `isAutoMemoryEnabled`, `teamMemPaths.isTeamMemPath`, `teamMemPaths.isTeamMemFile`.
- `src/services/extractMemories/*` — `scanMemoryFiles`, `formatMemoryManifest`, `parseMemoryType`, `getAutoMemPath`, `isAutoMemoryEnabled`, `isExtractModeActive`, all prompt-asset exports from `memoryTypes.ts` (spec 29).
- `src/tools/AgentTool/agentMemory.ts` — uses `buildMemoryPrompt` for agent memory.
- `src/tools/FileWriteTool` / `src/utils/filesystem.ts` — `isAutoMemPath`, `hasAutoMemPathOverride`, `isTeamMemPath`, `validateTeamMemWritePath`, `validateTeamMemKey` (write carve-out + path-traversal hardening).
- `src/utils/collapseReadSearch.ts` — `isAutoManagedMemoryFile` for render-path collapse.
- `src/services/extractMemories` and team-sync watcher import `getAutoMemPath`, `getTeamMemPath`, `getTeamMemEntrypoint`.

## 3. Public Interface (Contract)

Exports — file-level signatures verbatim:

```ts
// memdir.ts
export const ENTRYPOINT_NAME = 'MEMORY.md'
export const MAX_ENTRYPOINT_LINES = 200
export const MAX_ENTRYPOINT_BYTES = 25_000
export const DIR_EXISTS_GUIDANCE: string
export const DIRS_EXIST_GUIDANCE: string
export type EntrypointTruncation = {
  content: string; lineCount: number; byteCount: number
  wasLineTruncated: boolean; wasByteTruncated: boolean
}
export function truncateEntrypointContent(raw: string): EntrypointTruncation
export async function ensureMemoryDirExists(memoryDir: string): Promise<void>
export function buildMemoryLines(displayName: string, memoryDir: string, extraGuidelines?: string[], skipIndex?: boolean): string[]
export function buildMemoryPrompt(p: { displayName: string; memoryDir: string; extraGuidelines?: string[] }): string
export function buildSearchingPastContextSection(autoMemDir: string): string[]
export async function loadMemoryPrompt(): Promise<string | null>

// paths.ts
export function isAutoMemoryEnabled(): boolean
export function isExtractModeActive(): boolean
export function getMemoryBaseDir(): string
export function hasAutoMemPathOverride(): boolean
export const getAutoMemPath: (() => string) /* memoized on getProjectRoot() */
export function getAutoMemDailyLogPath(date?: Date): string
export function getAutoMemEntrypoint(): string
export function isAutoMemPath(absolutePath: string): boolean
// SECURITY (paths.ts:274-278): MUST `normalize(absolutePath)` BEFORE the
// startsWith check. Naive startsWith on the raw path lets attackers bypass
// the auto-mem boundary by injecting `..` segments (e.g. "/auto/../etc/...").

// memoryTypes.ts
export const MEMORY_TYPES = ['user','feedback','project','reference'] as const
export type MemoryType = (typeof MEMORY_TYPES)[number]
export function parseMemoryType(raw: unknown): MemoryType | undefined
export const TYPES_SECTION_COMBINED: readonly string[]
export const TYPES_SECTION_INDIVIDUAL: readonly string[]
export const WHAT_NOT_TO_SAVE_SECTION: readonly string[]
export const MEMORY_DRIFT_CAVEAT: string
export const WHEN_TO_ACCESS_SECTION: readonly string[]
export const TRUSTING_RECALL_SECTION: readonly string[]
export const MEMORY_FRONTMATTER_EXAMPLE: readonly string[]

// memoryScan.ts
export type MemoryHeader = { filename: string; filePath: string; mtimeMs: number; description: string|null; type: MemoryType|undefined }
export async function scanMemoryFiles(memoryDir: string, signal: AbortSignal): Promise<MemoryHeader[]>
export function formatMemoryManifest(memories: MemoryHeader[]): string

// findRelevantMemories.ts
export type RelevantMemory = { path: string; mtimeMs: number }
export async function findRelevantMemories(query: string, memoryDir: string, signal: AbortSignal, recentTools?: readonly string[], alreadySurfaced?: ReadonlySet<string>): Promise<RelevantMemory[]>

// memoryAge.ts
export function memoryAgeDays(mtimeMs: number): number
export function memoryAge(mtimeMs: number): string
export function memoryFreshnessText(mtimeMs: number): string
export function memoryFreshnessNote(mtimeMs: number): string

// teamMemPaths.ts (feature('TEAMMEM') only)
export class PathTraversalError extends Error
export function isTeamMemoryEnabled(): boolean
export function getTeamMemPath(): string
export function getTeamMemEntrypoint(): string
export function isTeamMemPath(filePath: string): boolean
export async function validateTeamMemWritePath(filePath: string): Promise<string>
export async function validateTeamMemKey(relativeKey: string): Promise<string>
export function isTeamMemFile(filePath: string): boolean

// teamMemPrompts.ts (feature('TEAMMEM') only)
export function buildCombinedMemoryPrompt(extraGuidelines?: string[], skipIndex?: boolean): string
```

No indirect cache contract is owned by memdir. `setCachedClaudeMdContent` at `src/context.ts:176` caches the assembled **CLAUDE.md project-instructions chain** (`getClaudeMds(filterInjectedMemoryFiles(await getMemoryFiles()))`), not the output of `loadMemoryPrompt`. That cache slot is owned by spec 05; memdir's prompt is wired into the system prompt via a separate `systemPromptSection('memory', …)` cache (see §4.2).

## 4. Data Model & State

### 4.1 On-disk layout

```
<memoryBase>/                             # getMemoryBaseDir()
  projects/
    <sanitized-git-root>/                 # sanitizePath(getAutoMemBase())
      memory/                              # AUTO_MEM_DIRNAME, getAutoMemPath()
        MEMORY.md                          # AUTO_MEM_ENTRYPOINT_NAME index, getAutoMemEntrypoint()
        <topic>.md                         # individual memory files
        ...
        team/                              # feature('TEAMMEM'): getTeamMemPath()
          MEMORY.md                        # team-scope index, getTeamMemEntrypoint()
          <topic>.md
        logs/                              # feature('KAIROS'): getAutoMemDailyLogPath()
          YYYY/MM/YYYY-MM-DD.md            # append-only daily logs
```

`memoryBase` resolution: `CLAUDE_CODE_REMOTE_MEMORY_DIR` env var if set, else `getClaudeConfigHomeDir()` (typically `~/.claude`) — `paths.ts:85-90`. Project key uses `findCanonicalGitRoot(getProjectRoot()) ?? getProjectRoot()` so all worktrees share one memdir (`paths.ts:203-205`, anthropics/claude-code#24382). Trailing separator and NFC normalization are part of the path contract (`paths.ts:230-232`, `teamMemPaths.ts:84-86`).

Two override paths short-circuit the layout: `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` env (no `~` expansion, opaque-trust marker for the SDK; `paths.ts:161-166,194-196`) and `autoMemoryDirectory` in settings.json (with `~/` expansion, sourced only from `policySettings | flagSettings | localSettings | userSettings` — `projectSettings` is intentionally excluded for security; `paths.ts:179-186`).

### 4.2 In-memory state

- `getAutoMemPath` is `memoize`d on `getProjectRoot()`: render-path callers (`collapseReadSearchGroups → isAutoManagedMemoryFile`) hit it per Messages re-render and each miss costs `getSettingsForSource × 4 → parseSettingsFile (realpathSync + readFileSync)` (`paths.ts:223-235`).
- `loadMemoryPrompt` is invoked once per session via the `systemPromptSection('memory', …)` cache (consumer-side). The cache prefix is intentionally NOT invalidated on midnight rollover — KAIROS daily-log path is described as a `YYYY/MM/YYYY-MM-DD.md` *pattern* rather than a literal so the model derives the date from the `date_change` attachment (`memdir.ts:329-336`).

### 4.3 Frontmatter contract (per memory file)

Frontmatter is YAML between `---` delimiters, parsed by `parseFrontmatter` (`src/utils/frontmatterParser.ts:130`). Memory files use the shared `FrontmatterData` shape; the relevant fields and their narrowing rules (`frontmatterParser.ts:10-58`):

```ts
type FrontmatterData = {
  description?: string | null
  type?: string | null   // narrowed via parseMemoryType()
  // …other shared fields not used by memdir
  [key: string]: unknown
}
```

`parseMemoryType` accepts only the four literal strings; legacy files without `type:` keep working, unknown values degrade to `undefined` (`memoryTypes.ts:28-31`). `MAX_MEMORY_FILES = 200`, `FRONTMATTER_MAX_LINES = 30` cap the scan (`memoryScan.ts:21-22`).

## 5. Algorithm / Control Flow

### 5.1 `loadMemoryPrompt()` dispatch (`memdir.ts:419-507`)

```
autoEnabled = isAutoMemoryEnabled()
skipIndex   = GBflag('tengu_moth_copse', false)

if feature('KAIROS') and autoEnabled and getKairosActive():
    logMemoryDirCounts(getAutoMemPath(), {memory_type: 'auto'})
    return buildAssistantDailyLogPrompt(skipIndex)   # KAIROS pre-empts TEAMMEM

extra = process.env.CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES (if non-empty → [val])

if feature('TEAMMEM') and teamMemPaths.isTeamMemoryEnabled():
    autoDir = getAutoMemPath(); teamDir = getTeamMemPath()
    await ensureMemoryDirExists(teamDir)             # recursive mkdir creates auto+team
    logMemoryDirCounts(autoDir, {memory_type: 'auto'})
    logMemoryDirCounts(teamDir, {memory_type: 'team'})
    return teamMemPrompts.buildCombinedMemoryPrompt(extra, skipIndex)

if autoEnabled:
    autoDir = getAutoMemPath()
    await ensureMemoryDirExists(autoDir)
    logMemoryDirCounts(autoDir, {memory_type: 'auto'})
    return buildMemoryLines('auto memory', autoDir, extra, skipIndex).join('\n')

logEvent('tengu_memdir_disabled', {disabled_by_env_var, disabled_by_setting})
if GBflag('tengu_herring_clock', false):
    logEvent('tengu_team_memdir_disabled', {})
return null
```

`isAutoMemoryEnabled` priority chain (`paths.ts:30-55`): `CLAUDE_CODE_DISABLE_AUTO_MEMORY` env (truthy → off, falsy → on); `CLAUDE_CODE_SIMPLE` truthy → off; `CLAUDE_CODE_REMOTE` truthy without `CLAUDE_CODE_REMOTE_MEMORY_DIR` → off; `settings.autoMemoryEnabled` if defined; else default true.

`isExtractModeActive` (gates extractMemories agent fork; spec 29 owns the fork) — `paths.ts:69-77`: gate on `tengu_passport_quail`; if interactive return true, else require `tengu_slate_thimble`.

### 5.2 `truncateEntrypointContent(raw)` (`memdir.ts:57-103`)

```
trimmed = raw.trim()
lines   = trimmed.split('\n'); lineCount = lines.length; byteCount = trimmed.length
wasLine = lineCount > MAX_ENTRYPOINT_LINES
wasByte = byteCount > MAX_ENTRYPOINT_BYTES        # check ORIGINAL byte count
if not wasLine and not wasByte: return {content: trimmed, ...}
truncated = wasLine ? lines.slice(0, 200).join('\n') : trimmed
if truncated.length > MAX_ENTRYPOINT_BYTES:
    cutAt = truncated.lastIndexOf('\n', MAX_ENTRYPOINT_BYTES)
    truncated = truncated.slice(0, cutAt > 0 ? cutAt : MAX_ENTRYPOINT_BYTES)
return {content: truncated + '\n\n> WARNING: …', wasLine, wasByte, …}
```

Warning suffix names which cap fired (`memdir.ts:87-97`).

### 5.3 `scanMemoryFiles(memoryDir, signal)` (`memoryScan.ts:35-77`)

```
entries = readdir(memoryDir, {recursive: true})
mdFiles = entries.filter(f => f.endsWith('.md') and basename(f) !== 'MEMORY.md')
results = await Promise.allSettled(mdFiles.map async rel => {
    filePath = join(memoryDir, rel)
    {content, mtimeMs} = await readFileInRange(filePath, 0, 30, undefined, signal)
    {frontmatter} = parseFrontmatter(content, filePath)
    return {filename: rel, filePath, mtimeMs,
            description: frontmatter.description || null,
            type: parseMemoryType(frontmatter.type)}
})
return results.filter(fulfilled).map(value).sort(by mtimeMs desc).slice(0, 200)
on any throw: return []   # readdir-level failure swallowed
```

Single-pass (read-then-sort) is intentional: `readFileInRange` stats internally; halves syscalls for `N ≤ 200`.

### 5.4 `findRelevantMemories(query, memoryDir, signal, recentTools=[], alreadySurfaced=∅)` (`findRelevantMemories.ts:39-75`)

```
memories = (await scanMemoryFiles(memoryDir, signal))
              .filter(m => not alreadySurfaced.has(m.filePath))
if memories.empty: return []
selectedFilenames = await selectRelevantMemories(query, memories, signal, recentTools)
byFilename = Map(memories[i].filename -> memories[i])
selected   = selectedFilenames.map(byFilename.get).filter(defined)
if feature('MEMORY_SHAPE_TELEMETRY'):
    require('./memoryShapeTelemetry.js').logMemoryRecallShape(memories, selected)
return selected.map(m => {path: m.filePath, mtimeMs: m.mtimeMs})
```

`selectRelevantMemories` (`:77-141`): formats manifest, builds optional `\n\nRecently used tools: …` suffix, calls `sideQuery({model: getDefaultSonnetModel(), system: SELECT_MEMORIES_SYSTEM_PROMPT, skipSystemPromptPrefix: true, max_tokens: 256, output_format: json_schema {selected_memories: string[]}, querySource: 'memdir_relevance'})`, parses via `jsonParse`, intersects with `validFilenames`. On exception: if `signal.aborted` return `[]`; else `logForDebugging('[memdir] selectRelevantMemories failed: …', warn)` and return `[]`.

### 5.5 `validateMemoryPath(raw, expandTilde)` (`paths.ts:109-150`)

```
if !raw: return undefined
if expandTilde and (raw.startsWith('~/') or raw.startsWith('~\\')):
    rest = raw.slice(2)
    restNorm = normalize(rest || '.')
    if restNorm in {'.', '..'}: return undefined
    raw = join(homedir(), rest)
normalized = normalize(raw).replace(/[\/\\]+$/, '')
reject if !isAbsolute(normalized) or normalized.length<3
       or /^[A-Za-z]:$/.test(normalized) or normalized.startsWith('\\\\')
       or normalized.startsWith('//') or normalized.includes('\0')
return (normalized + sep).normalize('NFC')
```

`getAutoMemPathOverride` calls with `expandTilde=false`; `getAutoMemPathSetting` with `expandTilde=true` (`paths.ts:161-186`).

### 5.6 Team-memory write validation (`teamMemPaths.ts`)

`validateTeamMemWritePath(filePath)` (`:228-256`): reject `\0`; first-pass `resolve(filePath).startsWith(getTeamMemPath())` (string-level, fast); second-pass `realpathDeepestExisting(resolvedPath)` then `isRealPathWithinTeamDir(realPath)` to defeat symlink escape (PSR M22186).

`validateTeamMemKey(relativeKey)` (`:265-284`): `sanitizePathKey` rejects null bytes, URL-encoded `..` / `/`, NFKC-normalized fullwidth `../` traversals, backslashes, absolute paths; then same first-pass + second-pass containment.

`realpathDeepestExisting` (`:109-171`): walk up `dirname` until `realpath()` succeeds, accumulating non-existing tail; on `ENOENT` distinguish dangling symlink (lstat succeeds, `isSymbolicLink()`) → throw `PathTraversalError`; on `ELOOP` throw; on `EACCES`/`EIO` (anything other than `ENOTDIR`/`ENAMETOOLONG`) throw fail-closed; loop terminates when `current === parent` (root).

`isRealPathWithinTeamDir` (`:183-206`): `realpath(getTeamMemPath().replace(/[\/\\]+$/, ''))`; on `ENOENT`/`ENOTDIR` return `true` (no symlink possible); on other errors return `false`. Match requires equality OR `realCandidate.startsWith(realTeamDir + sep)` (prefix-attack guard).

### 5.7 `isAutoMemPath(absolutePath)` (`paths.ts:274-278`)

```
normalizedPath = normalize(absolutePath)            # SECURITY: collapse `..`
return normalizedPath.startsWith(getAutoMemPath())
```

The `normalize()` call is load-bearing: a naive `absolutePath.startsWith(autoMemPath)` lets an attacker pass `<autoMemPath>/../../etc/passwd` and pass the prefix check while the actual filesystem read targets a path outside the auto-mem boundary. Reimplementer MUST keep the normalize step.

### 5.8 `ensureMemoryDirExists` and writer-side guarantees

`memdir.ts:129-147`: `await fs.mkdir(memoryDir)` (recursive default; `EEXIST` swallowed in `fs.mkdir`). Errors that escape (`EACCES`/`EPERM`/`EROFS`) are debug-logged, prompt building continues. Only the team dir is created in TEAMMEM mode because `team/` is a child of the auto dir, so recursive mkdir creates both (`memdir.ts:454-458`).

## 6. Verbatim Assets

### 6.1 Constants table (memdir-owned)

| Constant | Value | Location |
|---|---|---|
| `ENTRYPOINT_NAME` | `'MEMORY.md'` | `memdir.ts:34` |
| `MAX_ENTRYPOINT_LINES` | `200` | `memdir.ts:35` |
| `MAX_ENTRYPOINT_BYTES` | `25_000` | `memdir.ts:38` |
| `AUTO_MEM_DISPLAY_NAME` | `'auto memory'` | `memdir.ts:39` |
| `AUTO_MEM_DIRNAME` | `'memory'` | `paths.ts:92` |
| `AUTO_MEM_ENTRYPOINT_NAME` | `'MEMORY.md'` | `paths.ts:93` |
| `MAX_MEMORY_FILES` | `200` | `memoryScan.ts:21` |
| `FRONTMATTER_MAX_LINES` | `30` | `memoryScan.ts:22` |
| Sonnet selector `max_tokens` | `256` | `findRelevantMemories.ts:108` |
| `memoryAgeDays` divisor | `86_400_000` ms/day | `memoryAge.ts:7` |
| Path validation min length | `3` | `paths.ts:141` |
| `getAutoMemPath` cache key | `getProjectRoot()` | `paths.ts:234` |

### 6.2 On-disk path constants

```
AUTO_MEM_DIRNAME          = 'memory'                        # paths.ts:92
AUTO_MEM_ENTRYPOINT_NAME  = 'MEMORY.md'                     # paths.ts:93
team subdir               = 'team'                          # teamMemPaths.ts:84,93
team entrypoint           = '<auto>/team/MEMORY.md'         # teamMemPaths.ts:93
KAIROS daily log shape    = '<auto>/logs/YYYY/MM/YYYY-MM-DD.md'  # paths.ts:250 / memdir.ts:335
override env              = CLAUDE_COWORK_MEMORY_PATH_OVERRIDE  # paths.ts:163
remote base env           = CLAUDE_CODE_REMOTE_MEMORY_DIR   # paths.ts:86-87
extra guidelines env      = CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES  # memdir.ts:442
disable env               = CLAUDE_CODE_DISABLE_AUTO_MEMORY  # paths.ts:31
simple-mode disable       = CLAUDE_CODE_SIMPLE              # paths.ts:41
remote disable            = CLAUDE_CODE_REMOTE              # paths.ts:45
config home               = getClaudeConfigHomeDir()        # paths.ts:89 (default ~/.claude)
```

### 6.3 Memory-types taxonomy (verbatim)

`MEMORY_TYPES` literal (`memoryTypes.ts:14-19`):

```ts
export const MEMORY_TYPES = [
  'user',
  'feedback',
  'project',
  'reference',
] as const
```

`parseMemoryType` (`:28-31`):

```ts
export function parseMemoryType(raw: unknown): MemoryType | undefined {
  if (typeof raw !== 'string') return undefined
  return MEMORY_TYPES.find(t => t === raw)
}
```

### 6.4 Frontmatter format example (verbatim — `memoryTypes.ts:261-271`)

```ts
export const MEMORY_FRONTMATTER_EXAMPLE: readonly string[] = [
  '```markdown',
  '---',
  'name: {{memory name}}',
  'description: {{one-line description — used to decide relevance in future conversations, so be specific}}',
  `type: {{${MEMORY_TYPES.join(', ')}}}`,
  '---',
  '',
  '{{memory content — for feedback/project types, structure as: rule/fact, then **Why:** and **How to apply:** lines}}',
  '```',
]
```

The `type` line interpolates to `type: {{user, feedback, project, reference}}` at render time.

### 6.5 Per-type memory description templates (verbatim)

The four `<type>` blocks exist in two parallel forms — `TYPES_SECTION_COMBINED` (`memoryTypes.ts:37-106`, with `<scope>` tags and team/private qualifiers in examples) and `TYPES_SECTION_INDIVIDUAL` (`memoryTypes.ts:113-178`, no `<scope>` tags, plain `[saves X memory: …]` examples). Per the in-source comment (`:9-12`) the duplication is intentional.

Both arrays open with:

```
## Types of memory

There are several discrete types of memory that you can store in your memory system…

<types>
```

Each `<type>` block — verbatim structure for `user` / `feedback` / `project` / `reference`:

```
<type>
    <name>{user|feedback|project|reference}</name>
    [combined-only:] <scope>{always private|default to private…|private or team, but strongly bias toward team|usually team}</scope>
    <description>…</description>
    <when_to_save>…</when_to_save>
    <how_to_use>…</how_to_use>
    [feedback,project only:] <body_structure>Lead with the rule…**Why:**…**How to apply:**…</body_structure>
    <examples>
    user: …
    assistant: [saves {COMBINED:`(private|team)? `}{type} memory: …]
    …
    </examples>
</type>
```

The arrays close with `</types>` and a blank line (`memoryTypes.ts:104-106,176-178`). Each line of every block is owned by the spec source — see `memoryTypes.ts:37-106` (combined) and `:113-178` (individual) for the full text; reproducing every bullet inline here would duplicate ~140 lines of source. The verbatim contract is: arrays of strings, exactly as exported, no whitespace edits.

### 6.6 `WHAT_NOT_TO_SAVE_SECTION` (verbatim, `memoryTypes.ts:183-195`)

```
## What NOT to save in memory

- Code patterns, conventions, architecture, file paths, or project structure — these can be derived by reading the current project state.
- Git history, recent changes, or who-changed-what — `git log` / `git blame` are authoritative.
- Debugging solutions or fix recipes — the fix is in the code; the commit message has the context.
- Anything already documented in CLAUDE.md files.
- Ephemeral task details: in-progress work, temporary state, current conversation context.

These exclusions apply even when the user explicitly asks you to save. If they ask you to save a PR list or activity summary, ask what was *surprising* or *non-obvious* about it — that is the part worth keeping.
```

### 6.7 `MEMORY_DRIFT_CAVEAT` and `WHEN_TO_ACCESS_SECTION` (verbatim, `memoryTypes.ts:201-222`)

`MEMORY_DRIFT_CAVEAT` is the single bullet:

```
- Memory records can become stale over time. Use memory as context for what was true at a given point in time. Before answering the user or building assumptions based solely on information in memory records, verify that the memory is still correct and up-to-date by reading the current state of the files or resources. If a recalled memory conflicts with current information, trust what you observe now — and update or remove the stale memory rather than acting on it.
```

`WHEN_TO_ACCESS_SECTION`:

```
## When to access memories
- When memories seem relevant, or the user references prior-conversation work.
- You MUST access memory when the user explicitly asks you to check, recall, or remember.
- If the user says to *ignore* or *not use* memory: proceed as if MEMORY.md were empty. Do not apply remembered facts, cite, compare against, or mention memory content.
- {MEMORY_DRIFT_CAVEAT}
```

### 6.8 `TRUSTING_RECALL_SECTION` (verbatim, `memoryTypes.ts:240-256`)

```
## Before recommending from memory

A memory that names a specific function, file, or flag is a claim that it existed *when the memory was written*. It may have been renamed, removed, or never merged. Before recommending it:

- If the memory names a file path: check the file exists.
- If the memory names a function or flag: grep for it.
- If the user is about to act on your recommendation (not just asking about history), verify first.

"The memory says X exists" is not the same as "X exists now."

A memory that summarizes repo state (activity logs, architecture snapshots) is frozen in time. If the user asks about *recent* or *current* state, prefer `git log` or reading the code over recalling the snapshot.
```

### 6.9 Selector system prompt (verbatim, `findRelevantMemories.ts:18-24`)

```
You are selecting memories that will be useful to Claude Code as it processes a user's query. You will be given the user's query and a list of available memory files with their filenames and descriptions.

Return a list of filenames for the memories that will clearly be useful to Claude Code as it processes the user's query (up to 5). Only include memories that you are certain will be helpful based on their name and description.
- If you are unsure if a memory will be useful in processing the user's query, then do not include it in your list. Be selective and discerning.
- If there are no memories in the list that would clearly be useful, feel free to return an empty list.
- If a list of recently-used tools is provided, do not select memories that are usage reference or API documentation for those tools (Claude Code is already exercising them). DO still select memories containing warnings, gotchas, or known issues about those tools — active use is exactly when those matter.
```

User-message template (`:103-105`): `Query: ${query}\n\nAvailable memories:\n${manifest}${toolsSection}` where `toolsSection = recentTools.length > 0 ? '\n\nRecently used tools: <comma-joined>' : ''`. Output format `json_schema` `{selected_memories: string[]}` (`:109-119`). `formatMemoryManifest` produces, per file: `- [<type>] <filename> (<isoTs>): <description>` or, when description is null, `- [<type>] <filename> (<isoTs>)` (`memoryScan.ts:84-94`).

### 6.10 Standalone prompt strings (verbatim, `memdir.ts:116-119`)

```
DIR_EXISTS_GUIDANCE  = 'This directory already exists — write to it directly with the Write tool (do not run mkdir or check for its existence).'
DIRS_EXIST_GUIDANCE  = 'Both directories already exist — write to them directly with the Write tool (do not run mkdir or check for their existence).'
```

Truncation warning (`memdir.ts:97`):

```
\n\n> WARNING: ${ENTRYPOINT_NAME} is ${reason}. Only part of it was loaded. Keep index entries to one line under ~200 chars; move detail into topic files.
```

`reason` is one of `${formatFileSize(byteCount)} (limit: ${formatFileSize(MAX_ENTRYPOINT_BYTES)}) — index entries are too long`, `${lineCount} lines (limit: ${MAX_ENTRYPOINT_LINES})`, or `${lineCount} lines and ${formatFileSize(byteCount)}` (`memdir.ts:88-92`).

### 6.11 Frontmatter Zod-equivalent schema

There is no Zod schema for memory frontmatter — `parseFrontmatter` returns the broad `FrontmatterData` typescript type (`utils/frontmatterParser.ts:10-58`), and `parseMemoryType` is the only field-level narrower applied by memdir. The contract enforced is:

```
{
  description?: string | null,   // surfaced as null when absent or empty
  type?: 'user' | 'feedback' | 'project' | 'reference' | <ignored>,  // unknown → undefined
  // arbitrary additional keys are allowed and ignored by memdir
}
```

(See spec 02 for the canonical Zod settings schemas and spec 17 for the frontmatter usage in skills.)

### 6.12 Empty-entrypoint placeholder (verbatim, `memdir.ts:308-312`)

```
## MEMORY.md

Your MEMORY.md is currently empty. When you save new memories, they will appear here.
```

### 6.13 Combined-prompt scope preamble (verbatim excerpt, `teamMemPrompts.ts:69-74`)

```
## Memory scope

There are two scope levels:

- private: memories that are private between you and the current user. They persist across conversations with only this specific user and are stored at the root `<autoDir>`.
- team: memories that are shared with and contributed by all of the users who work within this project directory. Team memories are synced at the beginning of every session and they are stored at `<teamDir>`.
```

The combined prompt also appends a sensitive-data caveat after `WHAT_NOT_TO_SAVE_SECTION` (`teamMemPrompts.ts:78`):

```
- You MUST avoid saving sensitive data within shared team memories. For example, never save API keys or user credentials.
```

### 6.14 KAIROS daily-log prompt body (verbatim excerpt, `memdir.ts:337-365`)

The full text is owned by `buildAssistantDailyLogPrompt`. The non-templated body opens:

```
# auto memory

You have a persistent, file-based memory system found at: `<memoryDir>`

This session is long-lived. As you work, record anything worth remembering by **appending** to today's daily log file:

`<memoryDir>/logs/YYYY/MM/YYYY-MM-DD.md`

Substitute today's date (from `currentDate` in your context) for `YYYY-MM-DD`. When the date rolls over mid-session, start appending to the new day's file.

Write each entry as a short timestamped bullet. Create the file (and parent directories) on first write if it does not exist. Do not rewrite or reorganize the log — it is append-only. A separate nightly process distills these logs into `MEMORY.md` and topic files.
```

Followed by `## What to log` (5 bullets verbatim at `:351-355`) → `WHAT_NOT_TO_SAVE_SECTION` → `## MEMORY.md` block (skipped under `skipIndex`) → `buildSearchingPastContextSection`.

### 6.15 Searching-past-context section (verbatim, `memdir.ts:392-407`)

Gated on `tengu_coral_fern`. Two variants:

```
## Searching past context

When looking for past context:
1. Search topic files in your memory directory:
```
{memSearch}
```
2. Session transcript logs (last resort — large files, slow):
```
{transcriptSearch}
```
Use narrow search terms (error messages, file paths, function names) rather than broad keywords.
```

`memSearch` / `transcriptSearch` swap between Grep-tool form and shell `grep -rn` based on `hasEmbeddedSearchTools() || isReplModeEnabled()` (`memdir.ts:382-391`).

## 7. Side Effects & I/O

- **Filesystem reads**: `readdir(memoryDir, {recursive: true})` (`memoryScan.ts:40`), `readFileInRange(filePath, 0, 30, undefined, signal)` per memory file (`memoryScan.ts:48-54`), sync `readFileSync(entrypoint, {encoding: 'utf-8'})` for MEMORY.md inside `buildMemoryPrompt` (`memdir.ts:288`, eslint-disabled `custom-rules/no-sync-fs` because prompt building is synchronous). `lstat` + `realpath` for team-write validation (`teamMemPaths.ts`).
- **Filesystem writes (this layer)**: `fs.mkdir(memoryDir)` (recursive default) inside `ensureMemoryDirExists` (`memdir.ts:131`). All memory-content writes happen via the `Write` tool driven by the model — memdir does not write content itself.
- **Network**: `sideQuery` to Sonnet for relevance selection, `querySource: 'memdir_relevance'` (`findRelevantMemories.ts:121`).
- **Env vars consumed**: `CLAUDE_CODE_DISABLE_AUTO_MEMORY`, `CLAUDE_CODE_SIMPLE`, `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_REMOTE_MEMORY_DIR`, `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE`, `CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES`. Indirect via `getClaudeConfigHomeDir`: `CLAUDE_CONFIG_DIR`.
- **Trust boundaries**: write carve-out in `filesystem.ts` fires when `isAutoMemPath(path) && !hasAutoMemPathOverride()`; settings-sourced `autoMemoryDirectory` keeps `hasAutoMemPathOverride()` false so it gets the carve-out, but env-override does NOT (`paths.ts:194-196,261-272`). `projectSettings` is intentionally excluded from `getAutoMemPathSetting` so a malicious repo cannot set `autoMemoryDirectory: "~/.ssh"` (`paths.ts:172-186`).

## 8. Feature Flags & Variants

| Flag | On behavior | Off behavior | Citation |
|---|---|---|---|
| `feature('TEAMMEM')` | `teamMemPaths` / `teamMemPrompts` requires resolved; `loadMemoryPrompt` may dispatch to `buildCombinedMemoryPrompt`; `isAutoManagedMemoryFile` includes `isTeamMemFile` | bothy lazy `require()`s evaluate to `null`; combined prompt unreachable; team carve-out skipped | `memdir.ts:7-9,106-108`; `memoryFileDetection.ts:17-19,137` |
| `feature('KAIROS')` + `getKairosActive()` + `autoEnabled` | `loadMemoryPrompt` returns `buildAssistantDailyLogPrompt(skipIndex)`; pre-empts TEAMMEM | individual or combined branch as normal | `memdir.ts:432-438` |
| `feature('MEMORY_SHAPE_TELEMETRY')` | recall fires `memoryShapeTelemetry.logMemoryRecallShape(memories, selected)` (selection-rate denominator + ages) | no recall-shape event | `findRelevantMemories.ts:66-72` |
| `feature('EXTRACT_MEMORIES')` | required outer gate for the extract-memories agent fork (must wrap callers of `isExtractModeActive()`; the inner GB check cannot tree-shake unless `feature()` is in the `if` directly) | extract pipeline stripped at build time | `paths.ts:65-67` (mandate in source comment); fork itself owned by spec 29 |
| GB `tengu_passport_quail` | extract-memories agent enabled (inner gate consumed by spec 29 *after* `feature('EXTRACT_MEMORIES')`) | extract agent skipped | `paths.ts:70` |
| GB `tengu_slate_thimble` | extract-mode active in non-interactive sessions too | non-interactive disables extract | `paths.ts:75` |
| GB `tengu_coral_fern` | append `## Searching past context` section to all memory prompts | section omitted | `memdir.ts:376` |
| GB `tengu_moth_copse` | `skipIndex=true`: drops MEMORY.md two-step / index instructions from save guidance | full two-step save flow | `memdir.ts:422-425` |
| GB `tengu_herring_clock` | team-memory cohort gate (used by `isTeamMemoryEnabled` and as denominator-only signal in disabled branch) | team memory off even if `feature('TEAMMEM')` is on | `teamMemPaths.ts:77`; `memdir.ts:503` |
| `hasEmbeddedSearchTools() || isReplModeEnabled()` | searching-past-context emits `grep -rn` shell form | searching-past-context emits Grep-tool form | `memdir.ts:382-391` |

No `USER_TYPE === 'ant'` branch is owned by memdir.

## 9. Error Handling & Edge Cases

- **`scanMemoryFiles`** returns `[]` on outer `readdir` throw (`memoryScan.ts:74-76`); per-file failures are dropped via `Promise.allSettled` filter.
- **`buildMemoryPrompt`** sync-reads `MEMORY.md` and swallows any error (`memdir.ts:286-291`); falls through to the empty-entrypoint placeholder (`:307-312`).
- **`ensureMemoryDirExists`** logs (`debug` level) but does not throw on `EACCES`/`EPERM`/`EROFS`; `FileWriteTool` re-runs mkdir of parent and surfaces the real error (`memdir.ts:138-146`).
- **`findRelevantMemories`/`selectRelevantMemories`** swallow Sonnet failures; if `signal.aborted` returns `[]` silently, otherwise warn-logs (`findRelevantMemories.ts:131-140`).
- **`validateMemoryPath`** rejects relative, root-too-short, Windows drive-root, UNC, and null-byte paths returning `undefined` (`paths.ts:139-148`).
- **`validateTeamMemKey` / `validateTeamMemWritePath`** throw `PathTraversalError` on null bytes, URL-encoded `..`, NFKC-normalized fullwidth traversals, backslashes, absolute keys, prefix attacks like `/foo/team-evil` vs `/foo/team`, dangling symlinks, symlink loops, and any `realpath` errno other than `ENOENT`/`ENOTDIR`/`ENAMETOOLONG` (fail-closed). When the team dir does not exist, `isRealPathWithinTeamDir` returns `true` because no symlink can have been planted (`teamMemPaths.ts:183-206`).
- **Truncation**: line-truncate first, then byte-truncate at the last newline before the cap (or at the cap if no newline) — never cuts mid-line (`memdir.ts:78-85`). Warning text always names which cap fired.
- **`tengu_memdir_disabled` telemetry** fires when both `autoEnabled` is false and the prompt is absent (`memdir.ts:492-498`); team-cohort denominator emitted via `tengu_team_memdir_disabled` even when `isAutoMemoryEnabled` is the gating factor (`memdir.ts:500-505`).

## 10. Telemetry & Observability

- `tengu_memdir_loaded` (`memdir.ts:174,182`) — `{memory_type: 'auto'|'team'|'agent', total_file_count, total_subdir_count, content_length?, line_count?, was_truncated?, was_byte_truncated?}`. Fire-and-forget; `total_*` omitted on `readdir` failure.
- `tengu_memdir_disabled` (`memdir.ts:492`) — `{disabled_by_env_var, disabled_by_setting}`.
- `tengu_team_memdir_disabled` (`memdir.ts:504`) — `{}`. Cohort-denominator only.
- Recall-shape event via `feature('MEMORY_SHAPE_TELEMETRY')` → `memoryShapeTelemetry.logMemoryRecallShape(memories, selected)`. Fires even on empty selection because selection-rate needs the denominator and `-1` ages distinguish "ran, picked nothing" from "never ran" (`findRelevantMemories.ts:64-72`).
- Side-query channel `'memdir_relevance'` is the analytics tag for the Sonnet selector call.
- `logForDebugging`: `ensureMemoryDirExists` debug, `selectRelevantMemories` warn-level diagnostic.

## 11. Reimplementation Checklist

- [ ] Path resolution preserves the priority chain: `CLAUDE_COWORK_MEMORY_PATH_OVERRIDE` → `autoMemoryDirectory` setting (policy/flag/local/user only — NOT projectSettings) → `<memoryBase>/projects/<sanitized-canonical-git-root>/memory/`. Always trailing `sep`, NFC-normalized.
- [ ] `validateMemoryPath` rejects: empty, non-absolute, length<3, Windows drive-root, UNC, null byte; `~/`/`~\` expansion with `restNorm in {'.', '..'}` rejection only when `expandTilde=true`.
- [ ] `getAutoMemPath` is memoized on `getProjectRoot()`.
- [ ] `MEMORY.md` line cap = 200, byte cap = 25,000; line-truncate → byte-truncate-at-last-newline; warning suffix names which cap fired with `formatFileSize` for byte cap and raw counts for line cap.
- [ ] `scanMemoryFiles`: recursive readdir, drop `MEMORY.md`, parse first 30 lines for frontmatter, sort by mtime desc, slice 200; failures swallowed.
- [ ] Selector: Sonnet via `sideQuery`, `skipSystemPromptPrefix: true`, `max_tokens: 256`, `output_format` JSON schema, querySource `memdir_relevance`; intersect output with `validFilenames`; warn-log on non-abort failure.
- [ ] Four memory types — `user`, `feedback`, `project`, `reference` — exactly; legacy/unknown → undefined.
- [ ] Frontmatter example MUST interpolate `MEMORY_TYPES.join(', ')` into the `type:` line so future taxonomy edits stay in sync.
- [ ] Both `TYPES_SECTION_*` arrays kept as flat string arrays (intentional duplication per source comment); per-mode prompt assembly chooses one.
- [ ] `ensureMemoryDirExists` is recursive `mkdir`, idempotent, swallows EEXIST, debug-logs other errno; only the team dir is mkdir'd in TEAMMEM mode (auto dir created as side effect).
- [ ] `buildMemoryPrompt` reads `MEMORY.md` synchronously; missing/empty file falls through to empty-entrypoint placeholder.
- [ ] KAIROS daily-log path described as `YYYY/MM/YYYY-MM-DD.md` *pattern* (not literal) in the prompt to preserve cache prefix across midnight.
- [ ] `loadMemoryPrompt` dispatch order: KAIROS+auto → TEAMMEM+team-enabled → auto-only → null. No team-only branch.
- [ ] `CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES` threaded into all three builders via `extraGuidelines`.
- [ ] `hasAutoMemPathOverride()` returns true ONLY for env-var override (settings override returns false) so the `filesystem.ts` write carve-out applies to settings-sourced dirs.
- [ ] **`isAutoMemPath` MUST `normalize()` the input before `startsWith`** (`paths.ts:275-277`). A raw `startsWith` lets `..`-segment paths bypass the auto-mem boundary; this is load-bearing for the `filesystem.ts` write carve-out gate.
- [ ] Team write validation does first-pass `resolve()`-startsWith and second-pass `realpathDeepestExisting`+real-prefix check. Trailing-sep stripped before `realpath`. Prefix match requires `+ sep` (no `team-evil` vs `team`).
- [ ] `validateTeamMemKey` runs `sanitizePathKey` (null byte, URL-encoded traversal, NFKC traversal, backslash, absolute) BEFORE join.
- [ ] `realpathDeepestExisting` distinguishes dangling symlink (lstat succeeds + isSymbolicLink) from real ENOENT; fails closed on EACCES/EIO/ELOOP.
- [ ] `memoryAge` returns `'today'` / `'yesterday'` / `'<n> days ago'`; `memoryFreshnessText/Note` return `''` for `d ≤ 1`.
- [ ] All telemetry events and field names match §10 exactly. Recall-shape fires even on empty selection.
- [ ] `feature('MEMORY_SHAPE_TELEMETRY')` invocation uses lazy `require('./memoryShapeTelemetry.js')` inside the `if` so dead-code elimination strips it.
- [ ] Do NOT wire memdir output into `setCachedClaudeMdContent` — that slot caches the CLAUDE.md project-instructions chain (spec 05), not memdir output. Memdir's prompt reaches the system prompt via the separate `systemPromptSection('memory', …)` cache (consumer-side).

## 12. Open Questions / Unknowns

1. **`memoryShapeTelemetry.ts` is missing from the leaked tree.** Referenced via lazy `require('./memoryShapeTelemetry.js')` at `findRelevantMemories.ts:69` under `feature('MEMORY_SHAPE_TELEMETRY')`. The named export `logMemoryRecallShape(memories, selected)` and the implication that empty-selection emits a `-1` age for "ran, picked nothing" (`:64-65`) are the only contract anchors. Recorded as `missing-leaked-source`.
2. **`buildMemoryPrompt` displayName branching.** `memdir.ts:297-304` distinguishes `'auto memory'` from any other display name as `memory_type: 'auto' | 'agent'` for telemetry. The agent-memory caller is owned by `src/tools/AgentTool/agentMemory.ts` (spec 14); the exact alternate `displayName` value used in production is not visible inside `memdir/`.
3. **(resolved)** Earlier draft mis-attributed `setCachedClaudeMdContent` (`src/context.ts:176`) to memdir. That cache slot holds the CLAUDE.md project-instructions chain (`getClaudeMds(filterInjectedMemoryFiles(...))`) and is owned by spec 05. Memdir is unrelated to it.
4. **GB-flag → ANT-IN coupling.** All five GrowthBook flags (`tengu_passport_quail`, `tengu_slate_thimble`, `tengu_coral_fern`, `tengu_moth_copse`, `tengu_herring_clock`) are referenced through `getFeatureValue_CACHED_MAY_BE_STALE` with `false` defaults. Whether any are forced-on in ANT-internal builds is not visible from memdir source; spec 26 owns the flag matrix.
5. **`FRONTMATTER_REGEX`** is referenced by `parseFrontmatter` at `frontmatterParser.ts:134` but its declaration sits earlier in that file — its anchor handling (CRLF, leading BOM, multi-doc YAML) is owned by spec 02/05; memdir relies on whatever it accepts.
6. **Empty-frontmatter coercion**: `frontmatter.description || null` (`memoryScan.ts:60`) will turn empty strings, `0`, and `false` into `null`. Memory `description` is typed `string | null`, so the practical risk is empty strings being normalized to null — confirmed intentional but worth noting because the manifest format (`memoryScan.ts:88-91`) drops the `: <description>` suffix when description is null.
