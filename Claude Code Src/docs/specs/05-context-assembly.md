# 05 — Context Assembly Specification

> Owner: sub-B3. Adjacent specs: 03 (query engine), 04 (turn pipeline), 07 (compaction), 29 (memory services), 38 (output styles), 40 (persistent memory).

## 1. Purpose & Scope

This spec describes the **context-prefix assembly** that runs once per conversation and is fed to every API call as the cache-key prefix: the **system context** (git status block + ant-only cache breaker) and the **user context** (CLAUDE.md chain + current date). Assembly is performed by `src/context.ts` (189 lines, read in full) which exposes three memoized async producers (`getGitStatus`, `getSystemContext`, `getUserContext`) and one ANT-only injection setter (`setSystemPromptInjection`).

In scope:

- `src/context.ts` end-to-end.
- The CLAUDE.md producer chain `getMemoryFiles → filterInjectedMemoryFiles → getClaudeMds` (high-level invocation; per-file content semantics owned by spec 29/40).
- The system-prompt injection cache-breaker mechanism (gated by `feature('BREAK_CACHE_COMMAND')`).
- Memoization, cache invalidation across `/clear` and post-compact.
- Env gates: `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_DISABLE_CLAUDE_MDS`, `NODE_ENV === 'test'`, `CLAUDE_CODE_OVERRIDE_DATE`, `CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS`, `--bare` / `CLAUDE_CODE_SIMPLE`.
- `RenderedSystemPrompt` snapshot field on `ToolUseContext` for fork subagent cache-identical replay.
- Helpers `appendSystemContext` / `prependUserContext` from `src/utils/api.ts` (the single sites that serialize the two maps into actual API surface text).

Out of scope (refer by spec #):

- The Anthropic API call itself, retry loop, streaming, prompt-cache breakpoint placement → 03.
- Hook fan-out (including the `InstructionsLoaded` hook fired by `getMemoryFiles`) → 04 / 29.
- Per-CLAUDE.md memory-file content rules, the `@include` directive, frontmatter, conditional rules, exclude-patterns, walk algorithm → 29.
- `MEMORY.md` / `memdir/` persistence and AutoMem/TeamMem on-disk schema → 40.
- `getSystemPrompt()` build (the static system prompt) and output-style assistant role injection → 38.
- Notification helpers under `src/context/` (notifications.tsx, mailbox.tsx, modalContext.tsx, overlayContext.tsx, promptOverlayContext.tsx, QueuedMessageContext.tsx, stats.tsx, voice.tsx, fpsMetrics.tsx) — these are React-context UI providers, not part of the LLM context-prefix assembly. They are listed in spec 37 (Ink UI shell). This spec confirms they are NOT in the API cache-key prefix.

## 2. Source Map

Primary file (read fully):

- `src/context.ts:1-189` — entire module.

Upstream imports (read fully or cited verbatim where load-bearing):

- `src/utils/claudemd.ts:790-1018` (`getMemoryFiles`), `:1142-1151` (`filterInjectedMemoryFiles`), `:1153-1195` (`getClaudeMds`), `:89-90` (`MEMORY_INSTRUCTION_PROMPT`).
- `src/bootstrap/state.ts:500-505` (`getOriginalCwd`), `:1207-1213` (`setCachedClaudeMdContent` / `getCachedClaudeMdContent`), `:1666` (`getAdditionalDirectoriesForClaudeMd`).
- `src/constants/common.ts:4-15` (`getLocalISODate`), `:24` (`getSessionStartDate`).
- `src/utils/git.ts:212-216` (`gitExe`), `:218-228` (`getIsGit`), `:261-263` (`getBranch`), `:265-267` (`getDefaultBranch`).
- `src/utils/gitSettings.ts:13-18` (`shouldIncludeGitInstructions`).
- `src/utils/envUtils.ts:32-37` (`isEnvTruthy`), `:60-65` (`isBareMode`).
- `src/utils/execFileNoThrow.ts` (entire — invoked with `gitExe()` and `--no-optional-locks`).
- `src/utils/diagLogs.ts` (`logForDiagnosticsNoPII`).
- `src/utils/systemPromptType.ts:1-14` (`SystemPrompt` brand + `asSystemPrompt`, used to thread the rendered prompt to fork subagents).

Downstream consumers (grep-inspected):

- `src/utils/queryContext.ts:14, 70-71` (`fetchSystemPromptParts`) — the canonical caller in QueryEngine.ts and SDK fallback.
- `src/QueryEngine.ts:288-308, 670-686` — `userContext` is augmented with `getCoordinatorUserContext(...)` before send.
- `src/utils/api.ts:437-447` (`appendSystemContext`), `:449-474` (`prependUserContext`).
- `src/screens/REPL.tsx:2535, 2543, 2772, 2788, 4942` — REPL and queued-task paths fetch context and snapshot the rendered system prompt onto `toolUseContext.renderedSystemPrompt`.
- `src/tools/AgentTool/runAgent.ts:381-382` — fork/sub-agent path can override `userContext`/`systemContext`.
- `src/tools/AgentTool/forkSubagent.ts:44-71` — uses the snapshot to skip a re-build.
- `src/Tool.ts:285-300` — defines `renderedSystemPrompt?: SystemPrompt` on `ToolUseContext`.
- `src/commands/clear/caches.ts:13-17, 52-66` — `/clear` invalidates all three memoize caches and resets the injection.
- `src/services/compact/postCompactCleanup.ts:31-77` — auto-compact / `/compact` invalidates `getUserContext` for main-thread compacts only.
- `src/commands/compact/compact.ts:5, 63, 117, 203, 277-278` — `/compact` clears the user-context cache at three points and re-fetches both maps for the compaction prompt build.
- `src/utils/analyzeContext.ts:279, 329`, `src/utils/doctorContextWarnings.ts:7,44`, `src/utils/status.tsx:7,117` — read-only callers (UI/diagnostics).
- `src/main.tsx:31, 367, 375, 405, 1972, 1977, 1978, 1981, 1983` — `void getSystemContext()` and `void getUserContext()` warm-up calls placed early to overlap fs/git I/O with module evaluation.

Source-coverage inventory:

| Path | Coverage |
|---|---|
| `src/context.ts` | fully-read |
| `src/utils/systemPromptType.ts` | fully-read |
| `src/constants/common.ts` | fully-read |
| `src/utils/gitSettings.ts` | fully-read |
| `src/utils/envUtils.ts` | fully-read |
| `src/commands/clear/caches.ts` | fully-read |
| `src/services/compact/postCompactCleanup.ts` | fully-read |
| `src/utils/queryContext.ts` | fully-read |
| `src/utils/claudemd.ts` (relevant ranges 1-200, 780-1018, 1080-1220) | sampled |
| `src/utils/git.ts` (lines 210-275 + grep for callees) | sampled |
| `src/Tool.ts` (lines 285-300) | sampled |
| `src/utils/api.ts` (lines 425-563) | sampled |
| `src/QueryEngine.ts` (lines 280-308, 670-700) | sampled |
| `src/screens/REPL.tsx` (lines 2530-2547, 2780-2790) | sampled |
| `src/tools/AgentTool/forkSubagent.ts` (lines 40-80) | sampled |
| `src/context/*.tsx` (notifications, mailbox, etc.) | grep-inspected (out of scope) |
| `src/utils/diagLogs.ts`, `src/utils/execFileNoThrow.ts` | grep-inspected (treated as opaque) |

Imports from: `bun:bundle` (`feature`), `lodash-es/memoize.js`, `./bootstrap/state.js`, `./constants/common.js`, `./utils/claudemd.js`, `./utils/diagLogs.js`, `./utils/envUtils.js`, `./utils/execFileNoThrow.js`, `./utils/git.js`, `./utils/gitSettings.js`, `./utils/log.js`.

Imported by: `src/utils/queryContext.ts`, `src/utils/api.ts`, `src/utils/analyzeContext.ts`, `src/tools/AgentTool/runAgent.ts`, `src/components/agents/generateAgent.ts`, `src/screens/REPL.tsx`, `src/interactiveHelpers.tsx`, `src/commands/btw/btw.tsx`, `src/commands/compact/compact.ts`, `src/commands/clear/caches.ts`, `src/services/compact/postCompactCleanup.ts`, `src/main.tsx`.

## 3. Public Interface

Verbatim signatures (copied from `src/context.ts:25-189`, abbreviated to declarations):

```ts
export function getSystemPromptInjection(): string | null
export function setSystemPromptInjection(value: string | null): void
export const getGitStatus: ((/* memoized */) => Promise<string | null>) & { cache: { clear?(): void } }
export const getSystemContext: ((/* memoized */) => Promise<{ [k: string]: string }>) & { cache: { clear?(): void } }
export const getUserContext: ((/* memoized */) => Promise<{ [k: string]: string }>) & { cache: { clear?(): void } }
```

Return-shape contract:

- `getGitStatus()` resolves to `string | null` (single multi-paragraph string, or `null` for non-git, test, or error).
- `getSystemContext()` resolves to a sparse map. Possible keys (and only these): `gitStatus`, `cacheBreaker`. Both are conditional; the map may be `{}`.
- `getUserContext()` resolves to a sparse map. Always includes `currentDate`. Conditionally includes `claudeMd`. No other keys.

Two helpers in `src/utils/api.ts` consume these maps:

```ts
appendSystemContext(systemPrompt: SystemPrompt, context: { [k: string]: string }): string[]
prependUserContext(messages: Message[], context: { [k: string]: string }): Message[]
```

`appendSystemContext` (`utils/api.ts:437-447`) appends a single newline-joined `key: value` block to the system prompt array (used by SDK fallback path; the main turn pipeline uses a richer per-key block — see §6.2).

`prependUserContext` (`utils/api.ts:449-474`) injects a synthetic `<system-reminder>...</system-reminder>` user message at index 0 of the message list (verbatim text in §6.2). Skipped entirely when `process.env.NODE_ENV === 'test'` or the context map is empty.

The fork-subagent contract (snapshot field):

```ts
// Tool.ts:293-299
/**
 * Parent's rendered system prompt bytes, frozen at turn start.
 * Used by fork subagents to share the parent's prompt cache — re-calling
 * getSystemPrompt() at fork-spawn time can diverge (GrowthBook cold→warm)
 * and bust the cache. See forkSubagent.ts.
 */
renderedSystemPrompt?: SystemPrompt
```

`SystemPrompt` is a branded `readonly string[]` (`src/utils/systemPromptType.ts:8-10`); `asSystemPrompt(value)` is the only constructor.

## 4. Data Model & State

Module-level mutable state in `src/context.ts`:

- `let systemPromptInjection: string | null = null` (`context.ts:23`). Setter clears the two map memoize caches.
- Three `memoize`-wrapped function objects (`getGitStatus`, `getSystemContext`, `getUserContext`) each carrying a `cache` property whose `clear()` method is invoked via optional chaining (`?.()`) at multiple sites (so test code that replaces the wrapper does not crash).

Memoization keying (lodash-es default):

- `lodash-es/memoize` keys on the **first argument**. All three functions take **no arguments**, so the cache is effectively a single-slot promise cache. The first invocation populates the slot; concurrent or subsequent callers receive the same `Promise`. This is the one-shot-per-session semantics relied on by `prependUserContext` and the cache-key prefix invariant.
- `getMemoryFiles` (in `claudemd.ts:790`) is independently memoized, but its memoize key is `forceIncludeExternal: boolean` (default `false`), so the production path (`getUserContext` → `getMemoryFiles()`) hits a separate slot from the approval-check path (`getMemoryFiles(true)`).

Adjacent state read by `getUserContext`:

- `getAdditionalDirectoriesForClaudeMd()` (`bootstrap/state.ts:1666`) — list mutated by `--add-dir` and `/add-dir`.
- `setCachedClaudeMdContent(content)` is called on every `getUserContext()` cache-miss to populate a separate one-slot cache used by the YOLO/auto-mode classifier (`utils/permissions/yoloClassifier.ts:455`). This avoids a cycle: `permissions` → `claudemd.ts` → `permissions/filesystem.ts` → `permissions`. The cached value mirrors `claudeMd || null` (intentional `||` not `??`: empty string is normalized to `null`).

State diagram (per session):

```
[unset] --(first call)--> [pending Promise] --(resolved)--> [cached value]
                                                      |
            <-- /clear (clearSessionCaches)            |
            <-- /compact main-thread (postCompactCleanup, getUserContext only)
            <-- /compact explicit clears (compact.ts:63,117,203, getUserContext only)
            <-- setSystemPromptInjection(...)  (clears BOTH getSystemContext and getUserContext)
                                                      v
                                                  [unset]
```

Note: `getGitStatus` cache is cleared by `/clear` (`caches.ts:54`) but **NOT** by `setSystemPromptInjection` (which only clears the two map caches) and **NOT** by post-compact cleanup. Git status is treated as a snapshot for the conversation lifetime irrespective of compaction.

## 5. Algorithm / Control Flow

### 5.1 `getGitStatus()` — `context.ts:36-111`

```
if process.env.NODE_ENV === 'test': return null            # avoids cycles in tests
log diag 'git_status_started'
isGit = await getIsGit()                                   # findGitRoot(getCwd()) !== null, memoized
log diag 'git_is_git_check_completed' { duration_ms, is_git }
if !isGit: log 'git_status_skipped_not_git' { duration_ms }; return null
try:
  in parallel (Promise.all):
    branch     = await getBranch()                          # cached upstream
    mainBranch = await getDefaultBranch()                   # cached upstream
    status     = (await execFileNoThrow(gitExe(), ['--no-optional-locks','status','--short'], { preserveOutputOnError: false })).stdout.trim()
    log        = (await execFileNoThrow(gitExe(), ['--no-optional-locks','log','--oneline','-n','5'], { preserveOutputOnError: false })).stdout.trim()
    userName   = (await execFileNoThrow(gitExe(), ['config','user.name'], { preserveOutputOnError: false })).stdout.trim()
  log diag 'git_commands_completed' { duration_ms, status_length: status.length }
  truncatedStatus = status.length > MAX_STATUS_CHARS
                    ? status.substring(0, MAX_STATUS_CHARS) + STATUS_TRUNCATION_SUFFIX
                    : status
  log diag 'git_status_completed' { duration_ms, truncated }
  return [
    "This is the git status at the start of the conversation. Note that this status is a snapshot in time, and will not update during the conversation.",
    "Current branch: " + branch,
    "Main branch (you will usually use this for PRs): " + mainBranch,
    *(userName ? ["Git user: " + userName] : []),
    "Status:\n" + (truncatedStatus || "(clean)"),
    "Recent commits:\n" + log,
  ].join("\n\n")
catch error:
  log diag 'git_status_failed' { duration_ms }
  logError(error)
  return null
```

Key invariants:

- The five git invocations run **in parallel**; this is load-bearing for boot latency. `getBranch`/`getDefaultBranch` are independently cached upstream, so concurrent calls do not multi-spawn.
- `--no-optional-locks` is passed to `status` and `log` (NOT to `config user.name`). This prevents git from creating optimistic index locks that would race a user's concurrent git invocation.
- `userName` is included only when truthy (empty trimmed stdout drops the line entirely; `git config user.name` returns exit 1 with empty stdout when unset, and `execFileNoThrow` swallows the non-zero exit because `preserveOutputOnError: false`).
- Empty status renders as the literal `(clean)` (after the `Status:\n` newline prefix). Truncation uses **`substring` not `slice`** (semantically equivalent for non-negative indices but bit-exact preservation matters here).
- `truncated` flag in diagnostics is `status.length > MAX_STATUS_CHARS` (not `truncatedStatus !== status`), measured pre-truncation.

### 5.2 `getSystemContext()` — `context.ts:116-150`

```
log 'system_context_started'
gitStatus = (isEnvTruthy(process.env.CLAUDE_CODE_REMOTE) || !shouldIncludeGitInstructions())
           ? null
           : await getGitStatus()
injection = feature('BREAK_CACHE_COMMAND') ? getSystemPromptInjection() : null
log 'system_context_completed' { duration_ms, has_git_status, has_injection }
return {
  ...(gitStatus ? { gitStatus } : {}),
  ...(feature('BREAK_CACHE_COMMAND') && injection
      ? { cacheBreaker: "[CACHE_BREAKER: " + injection + "]" }
      : {}),
}
```

Two independent gates — git status and cache breaker — produce zero, one, or two keys. Order in the object literal is `gitStatus` then `cacheBreaker`. JavaScript object key insertion order is observable through `Object.entries()` (used by `appendSystemContext` and `prependUserContext`), so this ordering is part of the bit-exact contract.

`shouldIncludeGitInstructions()` (`gitSettings.ts:13-18`):

```
envVal = process.env.CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS
if isEnvTruthy(envVal):       return false
if isEnvDefinedFalsy(envVal): return true     # explicit false wins over settings
return getInitialSettings().includeGitInstructions ?? true
```

`isEnvTruthy` accepts `1, true, yes, on` (lower-cased, trimmed) plus the literal boolean `true`. `isEnvDefinedFalsy` is symmetric: `0, false, no, off` (only when defined and non-empty).

Note the asymmetry between `getGitStatus` and `getSystemContext`: `getGitStatus` returns `null` in `NODE_ENV === 'test'`, but `getSystemContext` never short-circuits on test mode — it proceeds and just gets back `null`, which it then drops via the spread guard.

### 5.3 `getUserContext()` — `context.ts:155-189`

```
log 'user_context_started'
shouldDisableClaudeMd =
  isEnvTruthy(process.env.CLAUDE_CODE_DISABLE_CLAUDE_MDS)
  || (isBareMode() && getAdditionalDirectoriesForClaudeMd().length === 0)
claudeMd = shouldDisableClaudeMd
           ? null
           : getClaudeMds(filterInjectedMemoryFiles(await getMemoryFiles()))    # synchronous after the await
setCachedClaudeMdContent(claudeMd || null)                                       # populates the YOLO cycle-breaker cache
log 'user_context_completed' { duration_ms, claudemd_length: claudeMd?.length ?? 0, claudemd_disabled }
return {
  ...(claudeMd ? { claudeMd } : {}),
  currentDate: "Today's date is " + getLocalISODate() + ".",
}
```

`isBareMode()` (`envUtils.ts:60-65`): `isEnvTruthy(CLAUDE_CODE_SIMPLE) || process.argv.includes('--bare')` — checks argv directly (because keychain prefetch fires before main.tsx's action handler exports `CLAUDE_CODE_SIMPLE`).

`--bare` honors explicit `--add-dir` (the call site checks `getAdditionalDirectoriesForClaudeMd().length === 0`), but per-discovery walking is suppressed by `getMemoryFiles()` itself when bare mode is set; spec 29 owns that logic.

`currentDate` is **always** included (never gated). Date string is computed from `getLocalISODate()`:

- Returns `process.env.CLAUDE_CODE_OVERRIDE_DATE` verbatim if set (ANT-only override).
- Otherwise local-zone `YYYY-MM-DD`, hand-built (no `toLocaleString`/`toISOString`) to ensure local — not UTC — calendar.

Date is captured **once** at first cache-miss and never refreshed. After-midnight handling is delegated: `getDateChangeAttachments` (`utils/attachments.ts:1415-1444`) emits a tail `date_change` attachment instead of busting the prefix cache. The trade-off is acknowledged in `attachments.ts:1408-1412`: stale prefix-date wins over re-creating ~920K tokens of cache_creation per midnight crossing per overnight session.

### 5.4 Cache-breaker flow (ANT-only)

```
setSystemPromptInjection(value) → writes module global → clears getUserContext + getSystemContext caches
getSystemContext() (gated by feature('BREAK_CACHE_COMMAND')) reads injection and emits "cacheBreaker" key
```

The injection string is wrapped: `"[CACHE_BREAKER: " + injection + "]"`. `feature('BREAK_CACHE_COMMAND')` is build-time DCE'd via `bun:bundle`; in non-ANT bundles both the read of `getSystemPromptInjection()` at line 132 and the conditional emit at line 143 are eliminated, but `getSystemPromptInjection` / `setSystemPromptInjection` remain exported because `commands/clear/caches.ts:66` calls `setSystemPromptInjection(null)` unconditionally during `/clear`.

### 5.5 Cache-invalidation matrix

| Trigger | `getGitStatus` | `getSystemContext` | `getUserContext` | Source |
|---|---|---|---|---|
| `setSystemPromptInjection(...)` | — | clear | clear | `context.ts:32-33` |
| `/clear` (`clearSessionCaches`) | clear | clear | clear; `setSystemPromptInjection(null)`; `resetGetMemoryFilesCache('session_start')`; `getSessionStartDate.cache.clear()` | `caches.ts:52-66, 84` |
| `/compact` | — | — | clear (3 sites: pre-build, post-summary, on-fail); also `getUserContext.cache.clear()` runs at `compact.ts:63, 117, 203` and re-fetched at `:277-278` | `compact.ts` |
| auto-compact / reactive-compact main thread | — | — | clear; `resetGetMemoryFilesCache('compact')` | `postCompactCleanup.ts:51-61` |
| auto-compact / reactive-compact subagent | — | — | **NOT cleared** (see comment at `:31-39`) | `postCompactCleanup.ts:36-39` |
| `EnterWorktreeTool` / settings sync / `/memory` | — | — | (via `clearMemoryFileCaches()` only — does NOT clear `getUserContext` outer cache) | `claudemd.ts:1110-1122` |

The post-compact gotcha is documented inline (`postCompactCleanup.ts:52-58`): clearing only the inner `getMemoryFiles` cache without the outer `getUserContext` cache means the next turn hits the cached map and never re-walks memory; the centralized `runPostCompactCleanup()` clears both for main-thread compacts.

### 5.6 Fork-subagent rendered-prompt snapshot

The system prompt array (separate from this spec's two maps) is frozen onto `toolUseContext.renderedSystemPrompt` at turn start by REPL.tsx (`screens/REPL.tsx:2543` and `:2788`). Fork subagents (and resumed agents) read it directly:

```
// AgentTool.tsx:496-497 / resumeAgent.ts:118-119
if (toolUseContext.renderedSystemPrompt) {
  forkParentSystemPrompt = toolUseContext.renderedSystemPrompt
}
```

The fork agent's `getSystemPrompt` is intentionally `() => ''` (`forkSubagent.ts:70`); the override path supplies the parent's bytes. Rationale (`forkSubagent.ts:54-58`): re-calling `getSystemPrompt()` at fork-spawn time can diverge between parent and child due to GrowthBook cold→warm transitions, busting the prompt cache.

The `userContext`/`systemContext` maps are **not** themselves snapshotted onto `ToolUseContext`; instead the fork path passes the parent's already-resolved values through `override.userContext` / `override.systemContext` (`tools/AgentTool/runAgent.ts:381-382`) — the memoize cache of the parent's `getUserContext`/`getSystemContext` is shared across the same process anyway, so any subagent that calls them gets identical bytes back.

## 6. Verbatim Assets

### 6.1 Status-truncation suffix

The verbatim suffix (note the **leading newline**, NOT a leading space):

```
\n... (truncated because it exceeds 2k characters. If you need more information, run "git status" using BashTool)
```

Source: `src/context.ts:88`. Concatenation site: `status.substring(0, MAX_STATUS_CHARS) + '<suffix>'`. The leading `\n` ensures the truncation marker starts on its own line beneath the truncated status body, but it is part of the suffix string, not part of the body.

### 6.2 Verbatim system-prompt scaffolding text

Six static segments produced by `context.ts`:

1. `"This is the git status at the start of the conversation. Note that this status is a snapshot in time, and will not update during the conversation."` (`context.ts:97`)
2. `"Current branch: " + branch` (`context.ts:98`)
3. `"Main branch (you will usually use this for PRs): " + mainBranch` (`context.ts:99`)
4. `"Git user: " + userName` (`context.ts:100`, conditional)
5. `"Status:\n" + (truncatedStatus || "(clean)")` (`context.ts:101`)
6. `"Recent commits:\n" + log` (`context.ts:102`)

The six are joined by `'\n\n'` (`context.ts:103`).

`currentDate` template (`context.ts:186`):

```
Today's date is ${getLocalISODate()}.
```

`cacheBreaker` template (`context.ts:145`):

```
[CACHE_BREAKER: ${injection}]
```

Memory-instruction prompt (downstream, but emitted into the same `claudeMd` value via `getClaudeMds`; `claudemd.ts:89-90`):

```
Codebase and user instructions are shown below. Be sure to adhere to these instructions. IMPORTANT: These instructions OVERRIDE any default behavior and you MUST follow them exactly as written.
```

Per-file description suffixes (from `getClaudeMds`, `claudemd.ts:1168-1177`):

- Project: ` (project instructions, checked into the codebase)`
- Local: ` (user's private project instructions, not checked in)`
- TeamMem (when `feature('TEAMMEM')`): ` (shared team memory, synced across the organization)`
- AutoMem: ` (user's auto-memory, persists across conversations)`
- default (User/Managed): ` (user's private global instructions for all projects)`

Per-file body template (`claudemd.ts:1185`): `Contents of ${file.path}${description}:\n\n${content}` (TeamMem variant wraps content in `<team-memory-content source="shared">…</team-memory-content>`).

Aggregate (`claudemd.ts:1194`): `${MEMORY_INSTRUCTION_PROMPT}\n\n${memories.join('\n\n')}` — empty input ⇒ empty string ⇒ dropped from the map.

`prependUserContext` synthetic message body (`utils/api.ts:463-469`):

```
<system-reminder>
As you answer the user's questions, you can use the following context:
${Object.entries(context).map(([k,v]) => `# ${k}\n${v}`).join('\n')}

      IMPORTANT: this context may or may not be relevant to your tasks. You should not respond to this context unless it is highly relevant to your task.
</system-reminder>

```

Note the **six leading spaces** before `IMPORTANT:` (literal indentation in the template; preserved verbatim).

`appendSystemContext` body shape (`utils/api.ts:437-447`):

```
[
  ...systemPrompt,
  Object.entries(context).map(([key,value]) => `${key}: ${value}`).join('\n')
].filter(Boolean)
```

### 6.3 Git invocations (verbatim)

1. `git --no-optional-locks status --short`
2. `git --no-optional-locks log --oneline -n 5`
3. `git config user.name`

(`gitExe()` resolves the git binary once, memoized on first call.)

### 6.4 Constants

| Name | Value | Site |
|---|---|---|
| `MAX_STATUS_CHARS` | `2000` | `context.ts:20` |
| Status truncation suffix | see §6.1 | `context.ts:88` |
| `MAX_INCLUDE_DEPTH` (claudemd, downstream) | `5` | `claudemd.ts:537` |
| `MAX_MEMORY_CHARACTER_COUNT` (claudemd, downstream) | `40000` | `claudemd.ts:92` |

### 6.5 Memoization caches

| Cache | Default key | Cleared by |
|---|---|---|
| `getGitStatus.cache` | (no args) | `/clear` only |
| `getSystemContext.cache` | (no args) | `setSystemPromptInjection` + `/clear` |
| `getUserContext.cache` | (no args) | `setSystemPromptInjection` + `/clear` + `/compact` (3 sites) + `runPostCompactCleanup` (main-thread only) |
| `getMemoryFiles.cache` | `forceIncludeExternal: boolean` | `clearMemoryFileCaches`, `resetGetMemoryFilesCache` (with `nextEagerLoadReason`) |
| `getSessionStartDate.cache` | (no args) | `/clear` (constants/common.ts:24; not `--bare`-mode-specific in cleanup) |
| `getClaudeConfigHomeDir` (envUtils, peripheral) | `process.env.CLAUDE_CONFIG_DIR` | (auto via key change) |

### 6.6 Env / runtime gates

| Variable / call | Effect | Site |
|---|---|---|
| `NODE_ENV === 'test'` | `getGitStatus` returns `null` immediately; `prependUserContext` returns messages unchanged | `context.ts:37`, `utils/api.ts:453` |
| `CLAUDE_CODE_REMOTE` (truthy) | Skip git status block in `getSystemContext` | `context.ts:125` |
| `CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS` (truthy) | Same effect (via `shouldIncludeGitInstructions()`); explicit-falsy override | `gitSettings.ts:13-18`, `context.ts:126` |
| `CLAUDE_CODE_DISABLE_CLAUDE_MDS` (truthy) | Force `claudeMd = null` regardless of mode | `context.ts:166` |
| `CLAUDE_CODE_SIMPLE` truthy OR argv contains `--bare` AND no `--add-dir` | Force `claudeMd = null` (auto-discovery suppressed) | `context.ts:167`, `envUtils.ts:60-65` |
| `CLAUDE_CODE_OVERRIDE_DATE` | `currentDate` uses literal value (ANT-only) | `constants/common.ts:6-8` |
| `feature('BREAK_CACHE_COMMAND')` | Enables `cacheBreaker` emission and `injection` read | `context.ts:131,143` |

## 7. Side Effects & I/O

- Process spawn: `git` (3 invocations during `getGitStatus` cache miss; gated by `getIsGit()` which itself shells out only via `findGitRoot` filesystem walks, no spawn).
- Filesystem reads: `getMemoryFiles()` walks from `getOriginalCwd()` to root, plus user/managed dirs and `--add-dir` paths (full algorithm in spec 29). Triggered by `getUserContext()` cache-miss.
- No network. No keychain. No socket.
- `setCachedClaudeMdContent()` mutates the bootstrap-state classifier cache; pure in-process side effect.
- Diagnostics emission: 9 distinct `logForDiagnosticsNoPII` event names — `git_status_started`, `git_is_git_check_completed`, `git_status_skipped_not_git`, `git_commands_completed`, `git_status_completed`, `git_status_failed`, `system_context_started`, `system_context_completed`, `user_context_started`, `user_context_completed`. All payloads exclude PII (only durations, lengths, booleans).
- Trust boundary: the git status string is **fed verbatim into the system prompt** for non-bare, non-CCR sessions. Repo state (file paths, branch names, commit subjects, configured user name) becomes part of the cache-key prefix and goes to the model. Truncation cap protects against pathological repos but does not redact.

## 8. Feature Flags & Variants

| Flag / gate | On behavior | Off behavior |
|---|---|---|
| `feature('BREAK_CACHE_COMMAND')` | `getSystemContext()` reads module-level injection and emits `cacheBreaker` key wrapped as `[CACHE_BREAKER: …]`; setter still callable in either branch | Both reads at `context.ts:131,143` are DCE'd; `cacheBreaker` key never emitted; `setSystemPromptInjection(...)` still mutates module state and clears caches (so `/clear` semantics are unchanged) |
| `feature('TEAMMEM')` (downstream of `getClaudeMds`) | Adds team-memory wrapping per file | No effect at this layer |
| `CLAUDE_CODE_REMOTE` | Skip git status (`isEnvTruthy`); also affects every `prependUserContext` consumer indirectly because `gitStatus` key absent | Run git status normally |
| `CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS` | Skip git status. Three-state: truthy → off; defined-falsy → on; unset → settings.includeGitInstructions ?? true | (see left) |
| `CLAUDE_CODE_DISABLE_CLAUDE_MDS` | Hard-disable claudeMd; `setCachedClaudeMdContent(null)` | Honor walk |
| `--bare` / `CLAUDE_CODE_SIMPLE` | Skip claudeMd auto-discovery; `--add-dir` still honored (length>0 disables this gate) | Full walk |
| `NODE_ENV === 'test'` | `getGitStatus` shortcuts to `null`; `prependUserContext` returns input unchanged | Production behavior |
| `CLAUDE_CODE_OVERRIDE_DATE` (ANT-only) | Literal date string used | Local-zone YYYY-MM-DD computed |

ANT-vs-prod: `BREAK_CACHE_COMMAND` and `CLAUDE_CODE_OVERRIDE_DATE` are both ant-only paths in practice; the bun-bundle DCE preserves call sites in ANT bundles. No `USER_TYPE === 'ant'` check appears directly in `context.ts`.

## 9. Error Handling & Edge Cases

- `getGitStatus` `try/catch` wraps the entire `Promise.all` block. On any throw: `logForDiagnosticsNoPII('error','git_status_failed', {duration_ms})`, `logError(error)`, return `null`. Non-zero exit codes from individual git commands are absorbed inside `execFileNoThrow` (`preserveOutputOnError: false` discards stderr/stdout into `{ code, stdout: '', stderr: '' }` semantics; the `.then` still runs and `stdout.trim()` produces empty string — does not throw).
- `getSystemContext` does not catch; its only async work is `await getGitStatus()`, which itself never rejects.
- `getUserContext` does not catch. `getMemoryFiles()` wraps its own per-file errors (spec 29). `getClaudeMds` is synchronous and pure.
- Empty memory chain: `getClaudeMds([])` returns `''` (`claudemd.ts:1190-1192`); the spread guard `claudeMd && { claudeMd }` drops the empty string from the map, so the model sees no `claudeMd:` block — distinct from the disabled case (which is also dropped, but for a different code path). Both paths still call `setCachedClaudeMdContent(null)` because of the `||` normalization.
- Empty git status (clean tree): renders as the literal `(clean)` after `Status:\n`, distinct from "git status failed → null" (which drops the entire block).
- Truncation boundary: input of exactly 2000 chars is **not** truncated (strict `>`).
- Date midnight crossover: stale prefix wins over cache-bust by design (see §5.3 and `attachments.ts:1408-1412`).
- Race against `setSystemPromptInjection`: setter clears caches before next call returns; if a caller has already `await`-ed the previous promise, that caller sees the stale value (intentional — the injection mutates *future* sends, not in-flight ones).

User-facing strings emitted by this subsystem: none. The git-status block is a system-prompt fragment; the model receives it but the user does not see a rendered surface for it (verbose mode shows it via `appendSystemContext` debug paths owned by spec 38/01).

## 10. Telemetry & Observability

Diagnostic events (privacy-safe; via `logForDiagnosticsNoPII`):

- `git_status_started`
- `git_is_git_check_started`, `git_is_git_check_completed { duration_ms, is_git }`
- `git_status_skipped_not_git { duration_ms }`
- `git_commands_completed { duration_ms, status_length }`
- `git_status_completed { duration_ms, truncated }`
- `git_status_failed { duration_ms }` + `logError(error)`
- `system_context_started`, `system_context_completed { duration_ms, has_git_status, has_injection }`
- `user_context_started`, `user_context_completed { duration_ms, claudemd_length, claudemd_disabled }`

Analytics (downstream, in `utils/api.ts:logContextMetrics`): `tengu_context_size { git_status_size, claude_md_size, total_context_size, … }` derives sizes from the two maps for session metrics. Owned by spec 26/06 for routing.

## 11. Reimplementation Checklist

- [ ] Three module-level memoized async producers (`getGitStatus`, `getSystemContext`, `getUserContext`), each with no arguments.
- [ ] Memoize wrapper exposes a `cache.clear()` invoked via optional chaining at five external sites (`/clear`, `/compact` ×3 + post-compact, `setSystemPromptInjection`).
- [ ] `MAX_STATUS_CHARS = 2000`, strict `>` comparison; truncation suffix string identical to §6.1 including leading `\n`.
- [ ] All three git invocations pass `--no-optional-locks` for `status`/`log` only (NOT for `config user.name`).
- [ ] Six git-block segments emitted in the exact order at §6.2; joined by `\n\n`; `userName` line conditional; `(clean)` literal for empty status.
- [ ] `getSystemContext` order: `gitStatus` then `cacheBreaker` (object insertion order observable via `Object.entries`).
- [ ] `getSystemContext` skips git status iff `isEnvTruthy(CLAUDE_CODE_REMOTE) || !shouldIncludeGitInstructions()` (the latter is three-state).
- [ ] `getSystemPromptInjection` / `setSystemPromptInjection` exported regardless of `BREAK_CACHE_COMMAND`; injection emission and read both DCE'd when flag off.
- [ ] `setSystemPromptInjection(...)` clears `getUserContext.cache` and `getSystemContext.cache` (NOT `getGitStatus.cache`).
- [ ] `getUserContext` always emits `currentDate` (verbatim template `Today's date is YYYY-MM-DD.`); date captured once at first miss, refreshed only via `getDateChangeAttachments` tail.
- [ ] `getUserContext` honors three disable conditions (env hard-off, `--bare` without `--add-dir`); calls `setCachedClaudeMdContent(claudeMd || null)` on every miss (note `||`, not `??`).
- [ ] `NODE_ENV === 'test'` short-circuits `getGitStatus` and disables `prependUserContext` injection.
- [ ] `prependUserContext` synthetic body verbatim per §6.2 (six-space indent before `IMPORTANT:` preserved).
- [ ] `appendSystemContext` joins entries by `\n` and appends a single block to the `SystemPrompt` array.
- [ ] `ToolUseContext.renderedSystemPrompt?: SystemPrompt` set by REPL at turn start; consumed by fork/resume agents to skip rebuild; brand defined by `asSystemPrompt`.
- [ ] All nine diagnostic events emitted in their documented call paths.
- [ ] Cache-invalidation matrix (§5.5) preserved; in particular post-compact subagent path does NOT clear `getUserContext.cache`.

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. RESOLVED items cite where the answer landed; DEFERRED items remain genuinely unresolvable from this leak.

1. ~~**`getSessionStartDate` interaction**~~ — **NOTE Phase 9.7**: cross-check with spec 38 (output styles) confirms `getSessionStartDate` is not consumed inside output-style assembly; the cache key in `--bare` simple mode comes from `getSystemPrompt` (spec 38 §3) which builds its own session-stable cache key independent of date. `getSessionStartDate.cache` clearing on `/clear` is therefore for date-change attachment stability only (consumed by `caches.ts:55` invalidation, not as a prompt input). No behavioral change required here.
2. ~~**`getMemoryFiles` `forceIncludeExternal`**~~ — **RESOLVED Phase 9.7**: confirmed the single `forceIncludeExternal: true` consumer is `getExternalClaudeMdIncludes` (approval-check path, `claudemd.ts:1404-1417`); spec 29 (persistent memory) §4 owns this contract.
3. ~~**`isEnvTruthy(CLAUDE_CODE_REMOTE)` vs `getIsRemoteMode()`**~~ — **DEFERRED**: source preserves the env-only check at this site as observed. The asymmetry vs `cli/print.ts` is intentional per the existing comment block (system-prompt cache stability requires env-only resolution; runtime-mode toggling mid-session would invalidate cached prefix). Recorded as observed; not a defect.
4. ~~**`appendSystemContext` vs `prependUserContext` symmetry**~~ — **RESOLVED Phase 9.7**: spec 04 (turn pipeline) §5 documents the per-call-site routing: `appendSystemContext` is invoked once at session boot for the system-context map (joined with `\n` into the system prompt array tail); `prependUserContext` is invoked per-turn when the user-context map changes (rendered as `<system-reminder>`-wrapped meta user messages). They are not interchangeable.
5. ~~**`src/context/` directory**~~ — **RESOLVED Phase 9.7**: spec 37 (Ink UI shell) confirms `src/context/` contains only React Context providers (theme, transport, agent ID); no LLM-prefix content. Phase 10b coverage matrix (`PHASE10-COVERAGE.md`) marks this as 37-owned.
6. ~~**Cache-breaker user surface**~~ — **DEFERRED**: no `setSystemPromptInjection` non-null call site exists in the leaked tree. Spec 21b (ANT command catalog) confirms no `/break-cache` command is enumerated in source. The injection mechanism is either DCE'd at this build configuration or wired through an external test harness; treated as known-unfalsifiable from this leak.
7. ~~**Git userName failure mode for unconfigured user**~~ — **NOTE Phase 9.7**: `execFileNoThrow` (spec 41 / `utils/execFile.ts`) returns `{stdout: '', stderr: ..., code: 1}` on non-zero exit per spec 41 §5; the `.then(({stdout}) => stdout.trim())` therefore yields `''`, which the truthy check at `context.ts:100` treats correctly as missing. Behavior fully determined; no longer estimated.
