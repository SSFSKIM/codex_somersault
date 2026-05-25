# 10 — BashTool Specification

> Bit-exact reverse-engineered spec for the `Bash` tool. Mirrors the on-disk
> layout described by 08-tool-base-registry. Permission flow citations call
> back into 09-permission-system; this document owns BashTool's per-tool
> `checkPermissions`, the bash AST/security stack, sandbox decision, read-only
> validation, and prompt assembly.

---

## 1. Purpose & Scope

Executes a single shell command (one `bash -c …` invocation per tool call)
in the harness's persistent shell, returns merged stdout/stderr (with stderr
interleaved into stdout via merged fd), exit code, semantic interpretation,
optional persisted-output path for large outputs, and optional background-task
metadata when the command is auto- or explicitly backgrounded. The tool is
the dominant attack surface in Claude Code: it owns the per-tool permission
pipeline that combines a tree-sitter Bash AST, a legacy regex/shell-quote
fallback, env-var/wrapper stripping, exact/prefix/wildcard rule matching,
sandbox decisioning, sed-edit simulation, read-only auto-allow, classifier
auto-allow, and rule-suggestion generation.

**In scope** (every file fully read):
- `src/tools/BashTool/` — entire directory (BashTool.tsx, bashPermissions.ts,
  bashSecurity.ts, shouldUseSandbox.ts, prompt.ts, readOnlyValidation.ts,
  modeValidation.ts, pathValidation.ts, sedValidation.ts, sedEditParser.ts,
  bashCommandHelpers.ts, commandSemantics.ts, commentLabel.ts,
  destructiveCommandWarning.ts, utils.ts, toolName.ts, BashToolResultMessage.tsx,
  UI.tsx).
- `src/tools/shared/gitOperationTracking.ts` (consumed by BashTool.call →
  `trackGitOperations(input.command, result.code, result.stdout)`).
- `src/utils/bash/parser.ts` — `parseCommandRaw`/`parseCommand` and the
  `TREE_SITTER_BASH` / `TREE_SITTER_BASH_SHADOW` gates.
- ANT-only/undercover prompt deltas, ANT-only safe env vars, ANT-only sandbox
  excludedCommands GrowthBook fetch, ANT-only readonly allowlist extension,
  COMMIT_ATTRIBUTION setup hook citation.

**Out of scope** (referenced by spec #):
- The decision-tree shape used at hook fan-out, the global ToolPermissionContext
  semantics, deny-rule precedence rules, and the AskUserQuestion UI → **09**.
- `Tool` interface, registry membership, ordering invariants → **08**.
- Policy/MDM, settings layering of `permissions.{allow,deny,ask}` and
  `sandbox.excludedCommands` → **02 / 27**.
- PowerShellTool (parallel implementation) → **19**. Note:
  `gitOperationTracking.ts` is shell-agnostic and shared with PowerShellTool,
  but its primary consumer is BashTool.call (see `BashTool.tsx:683`).
- `pathValidation.ts` is large (1303 lines) and shared with FileRead/FileEdit
  for filesystem rule matching; only BashTool's call into `checkPathConstraints`
  and the cd+git/redirect-target gates are owned here.
- LSP integration → **24**. UI rendering → **37**.

---

## 2. Source Map

### 2.1 Files owned by this spec

| Path | Lines | Read | Notes |
|---|---:|---|---|
| `src/tools/BashTool/BashTool.tsx` | 1143 | full | `buildTool({…})`; main entry |
| `src/tools/BashTool/bashPermissions.ts` | 2621 | full | `bashToolHasPermission`, rule matching, classifier integration, env/wrapper stripping |
| `src/tools/BashTool/bashSecurity.ts` | 2592 | sampled | `bashCommandIsSafeAsync_DEPRECATED`, regex security battery, heredoc stripping |
| `src/tools/BashTool/shouldUseSandbox.ts` | 153 | full | sandbox decision, ANT GrowthBook excludedCommands |
| `src/tools/BashTool/prompt.ts` | 369 | full | `getSimplePrompt()` system text, sandbox section, undercover section, COMMIT_ATTRIBUTION-aware git block |
| `src/tools/BashTool/readOnlyValidation.ts` | 1990 | sampled (head + ANT zone) | flag-allowlist read-only validator, `ANT_ONLY_COMMAND_ALLOWLIST` |
| `src/tools/BashTool/pathValidation.ts` | 1303 | grep-only here; owned shared | `checkPathConstraints` callsite from `bashToolCheckPermission` |
| `src/tools/BashTool/sedValidation.ts` | 684 | grep-only | `checkSedConstraints` callsite |
| `src/tools/BashTool/sedEditParser.ts` | 322 | grep-only | sed-in-place → simulated edit |
| `src/tools/BashTool/modeValidation.ts` | 115 | full | acceptEdits auto-allow set: `mkdir, touch, rm, rmdir, mv, cp, sed` |
| `src/tools/BashTool/bashCommandHelpers.ts` | 265 | full | pipe-segment routing, `checkCommandOperatorPermissions` |
| `src/tools/BashTool/commandSemantics.ts` | 140 | full | exit-code interpretation: grep/rg/find/diff/test/[ |
| `src/tools/BashTool/commentLabel.ts` | 13 | full | leading `# label` extraction |
| `src/tools/BashTool/destructiveCommandWarning.ts` | 102 | full | UI-only destructive-pattern annotations |
| `src/tools/BashTool/utils.ts` | 223 | full | `stripEmptyLines`, `resizeShellImageOutput`, `resetCwdIfOutsideProject`, image data-URI handling |
| `src/tools/BashTool/toolName.ts` | 2 | full | `BASH_TOOL_NAME = 'Bash'` |
| `src/tools/BashTool/BashToolResultMessage.tsx` | 190 | grep-only | UI; owned by 37 surface but referenced here |
| `src/tools/BashTool/UI.tsx` | 184 | grep-only | UI; owned by 37 surface |
| `src/tools/shared/gitOperationTracking.ts` | 277 | full | git/gh/glab/curl-PR detection, OTLP counters, session→PR linking |
| `src/utils/bash/parser.ts` | 230 | full | tree-sitter wrapper + `PARSE_ABORTED` sentinel |

### 2.2 Imports from (selected)

`src/Tool.ts` (`buildTool`, types — spec 08), `src/tools.ts` registry
(`tools.ts:5,197,287`), `src/utils/bash/{ast,commands,parser,bashParser,
ParsedCommand,shellQuote}.ts`, `src/utils/permissions/{bashClassifier,
PermissionResult,PermissionRule,PermissionUpdate*,permissions,filesystem,
shellRuleMatching,dangerousPatterns}.ts`, `src/utils/sandbox/sandbox-adapter.ts`,
`src/utils/Shell.ts`, `src/utils/timeouts.ts`, `src/utils/shell/outputLimits.ts`,
`src/utils/undercover.ts`, `src/utils/attribution.ts`, `src/utils/gitSettings.ts`,
`src/utils/embeddedTools.ts`, `src/services/analytics/{index,growthbook}.ts`,
`src/tasks/LocalShellTask/LocalShellTask.ts`, `src/utils/toolResultStorage.ts`.

### 2.3 Imported by

`src/tools.ts:5` registers `BashTool`. `src/coordinator/`, `src/tools/AgentTool/`,
`src/tools/SkillTool/`, command surfaces that template-cite `Bash` (e.g.
`/commit`, `/commit-push-pr` in ANT prompt), permission UI components, hook
`PreToolUse`/`PostToolUse` matchers (the global decision tree spec 09).

### 2.4 Feature-flag and ANT guard locations (citations)

| Concern | Citation |
|---|---|
| `feature('TREE_SITTER_BASH')` gate (parse path) | `src/utils/bash/parser.ts:51,65,108` |
| `feature('TREE_SITTER_BASH_SHADOW')` (shadow telemetry) | `src/utils/bash/parser.ts:51,108`; `bashPermissions.ts:1683,1690,1707,1737` |
| `feature('BASH_CLASSIFIER')` (allow-classifier auto-approval) | `bashPermissions.ts:1576,1645,1760,1960,2027,2064,2131,2322,2421,2548` |
| `feature('TRANSCRIPT_CLASSIFIER')` (auto-mode skip) | `bashPermissions.ts:1467,1505,1862` |
| `feature('MONITOR_TOOL')` (sleep gate, prompt sleep subitems) | `BashTool.tsx:525`; `prompt.ts:312,320` |
| `feature('KAIROS')` (assistant-mode auto-background) | `BashTool.tsx:976` |
| `feature('COMMIT_ATTRIBUTION')` (setup hook) | `src/setup.ts:350` (full setup logic owned by spec 01) |
| `USER_TYPE === 'ant'` ANT_ONLY_SAFE_ENV_VARS gate | `bashPermissions.ts:174,250,329,591` |
| `USER_TYPE === 'ant'` ANT_ONLY_COMMAND_ALLOWLIST | `readOnlyValidation.ts:1211` |
| `USER_TYPE === 'ant'` undercover prompt branch | `prompt.ts:49,56` |
| `USER_TYPE === 'ant'` shouldUseSandbox dynamic excludedCommands | `shouldUseSandbox.ts:23` |
| `USER_TYPE === 'ant'` DANGEROUS_BASH_PATTERNS extension | `src/utils/permissions/dangerousPatterns.ts:58` |
| `USER_TYPE === 'ant'` ANT classifier telemetry | `bashPermissions.ts:123,1573,1638,1904,1912` |
| `isUndercover()` BashTool prompt variant | `prompt.ts:49` (callee at `src/utils/undercover.ts:28`) |
| `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` schema/prompt branch | `BashTool.tsx:226,254`; `prompt.ts:36` |
| `CLAUDE_CODE_DISABLE_COMMAND_INJECTION_CHECK` (legacy gate) | `bashPermissions.ts:1219,1678,2087,2346` |
| `CLAUDE_CODE_BASH_SANDBOX_SHOW_INDICATOR` (UI label) | `BashTool.tsx:502` |
| `CLAUDE_CODE_UNDERCOVER` force-on | `src/utils/undercover.ts:30,83` |
| `BASH_DEFAULT_TIMEOUT_MS` / `BASH_MAX_TIMEOUT_MS` env vars | `src/utils/timeouts.ts:13,29` |
| `BASH_MAX_OUTPUT_LENGTH` env var | `src/utils/shell/outputLimits.ts:8` |
| `tengu_birch_trellis` (GrowthBook killswitch for shadow mode) | `bashPermissions.ts:1684` |
| `tengu_sandbox_disabled_commands` (ANT GrowthBook) | `shouldUseSandbox.ts:24` |

### 2.5 Missing-source ledger

None. All cited paths exist.

---

## 3. Public Interface

### 3.1 Registration

`BashTool = buildTool({…})` at `BashTool.tsx:420`. Registered unconditionally
in the static tool registry at `src/tools.ts:197`. Name: literal string `'Bash'`
(`toolName.ts:2`). `BASH_TOOL_NAME` is exported from a separate file purely to
break a circular dependency from `prompt.ts`.

### 3.2 Input schema (verbatim, `BashTool.tsx:227-247,254-259`)

```ts
const fullInputSchema = lazySchema(() => z.strictObject({
  command: z.string().describe('The command to execute'),
  timeout: semanticNumber(z.number().optional()).describe(`Optional timeout in milliseconds (max ${getMaxTimeoutMs()})`),
  description: z.string().optional().describe(`Clear, concise description of what this command does in active voice. Never use words like "complex" or "risk" in the description - just describe what it does.

For simple commands (git, npm, standard CLI tools), keep it brief (5-10 words):
- ls → "List files in current directory"
- git status → "Show working tree status"
- npm install → "Install package dependencies"

For commands that are harder to parse at a glance (piped commands, obscure flags, etc.), add enough context to clarify what it does:
- find . -name "*.tmp" -exec rm {} \\; → "Find and delete all .tmp files recursively"
- git reset --hard origin/main → "Discard all local changes and match remote main"
- curl -s url | jq '.data[]' → "Fetch JSON from URL and extract data array elements"`),
  run_in_background: semanticBoolean(z.boolean().optional()).describe(`Set to true to run this command in the background. Use Read to read the output later.`),
  dangerouslyDisableSandbox: semanticBoolean(z.boolean().optional()).describe('Set this to true to dangerously override sandbox mode and run commands without sandboxing.'),
  _simulatedSedEdit: z.object({
    filePath: z.string(),
    newContent: z.string()
  }).optional().describe('Internal: pre-computed sed edit result from preview')
}));

const inputSchema = lazySchema(() => isBackgroundTasksDisabled
  ? fullInputSchema().omit({ run_in_background: true, _simulatedSedEdit: true })
  : fullInputSchema().omit({ _simulatedSedEdit: true }));
```

`_simulatedSedEdit` is **always** stripped from the model-facing schema (security
note at `BashTool.tsx:249-253`). When `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is
truthy at module-load time, `run_in_background` is also omitted.

### 3.3 Output schema (verbatim, `BashTool.tsx:279-295`)

```ts
const outputSchema = lazySchema(() => z.object({
  stdout: z.string().describe('The standard output of the command'),
  stderr: z.string().describe('The standard error output of the command'),
  rawOutputPath: z.string().optional().describe('Path to raw output file for large MCP tool outputs'),
  interrupted: z.boolean().describe('Whether the command was interrupted'),
  isImage: z.boolean().optional().describe('Flag to indicate if stdout contains image data'),
  backgroundTaskId: z.string().optional().describe('ID of the background task if command is running in background'),
  backgroundedByUser: z.boolean().optional().describe('True if the user manually backgrounded the command with Ctrl+B'),
  assistantAutoBackgrounded: z.boolean().optional().describe('True if assistant-mode auto-backgrounded a long-running blocking command'),
  dangerouslyDisableSandbox: z.boolean().optional().describe('Flag to indicate if sandbox mode was overridden'),
  returnCodeInterpretation: z.string().optional().describe('Semantic interpretation for non-error exit codes with special meaning'),
  noOutputExpected: z.boolean().optional().describe('Whether the command is expected to produce no output on success'),
  structuredContent: z.array(z.any()).optional().describe('Structured content blocks'),
  persistedOutputPath: z.string().optional().describe('Path to the persisted full output in tool-results dir (set when output is too large for inline)'),
  persistedOutputSize: z.number().optional().describe('Total size of the output in bytes (set when output is too large for inline)')
}));
```

### 3.4 Tool surface methods (citations only — bodies in §5)

| Method | Citation | Behavior |
|---|---|---|
| `name` | const `'Bash'` | registry key |
| `searchHint` | `BashTool.tsx:422` | `'execute shell commands'` |
| `maxResultSizeChars` | `BashTool.tsx:424` | `30_000` (tool-result persistence threshold) |
| `strict` | `BashTool.tsx:425` | `true` |
| `description({description})` | `:426-429` | returns user-supplied description else `'Run shell command'` |
| `prompt()` | `:431-433` | delegates to `getSimplePrompt()` from `prompt.ts` |
| `isConcurrencySafe(input)` | `:434-436` | true iff `isReadOnly(input)` |
| `isReadOnly(input)` | `:437-441` | derived from `commandHasAnyCd` + `checkReadOnlyConstraints` |
| `toAutoClassifierInput(input)` | `:442-444` | returns the literal `input.command` |
| `preparePermissionMatcher({command})` | `:445-468` | argv-level matcher used by hook `if`-filter |
| `isSearchOrReadCommand(input)` | `:469-477` | for collapsible UI (search/read/list) |
| `inputSchema`/`outputSchema` getters | `:478-483` | lazy schemas |
| `userFacingName(input)` | `:484-503` | `'Bash'` or `'SandboxedBash'` (env-gated) or sed→FileEdit name |
| `getToolUseSummary(input)` | `:504-516` | description if present else truncate(command, `TOOL_SUMMARY_MAX_LENGTH`) |
| `getActivityDescription(input)` | `:517-523` | `Running ${desc}` |
| `validateInput(input)` | `:524-538` | MONITOR_TOOL sleep block (errorCode 10) |
| `checkPermissions(input,context)` | `:539-541` | delegates to `bashToolHasPermission` |
| `extractSearchText({stdout,stderr})` | `:549-554` | for transcript search |
| `mapToolResultToToolResultBlockParam(...)` | `:555-623` | builds final tool_result content block |
| `call(input, context, _canUseTool, parentMessage, onProgress)` | `:624-820` | main run path |
| `isResultTruncated(output)` | `:822-824` | OR over stdout/stderr line truncation |

### 3.5 The `bashToolHasPermission` entry contract

Signature: `(input, context: ToolUseContext, getCommandSubcommandPrefixFn?) → Promise<PermissionResult>`
(`bashPermissions.ts:1663`). Every call into the spec-09 decision tree from
the BashTool path goes through this function. The optional third parameter is
a test-injection seam.

---

## 4. Data Model & State

### 4.1 Types (verbatim or near-verbatim citations)

- `ShellPermissionRule` — discriminated union, three shapes:
  `{ type: 'exact', command: string }`, `{ type: 'prefix', prefix: string }`,
  `{ type: 'wildcard', pattern: string }`. Parsed via
  `parsePermissionRule` (`bashPermissions.ts:60-65`). Re-exported as
  `bashPermissionRule` (`bashPermissions.ts:364-366`).
- `PermissionResult` from `src/utils/permissions/PermissionResult.ts` (spec 09):
  behaviors `'allow' | 'deny' | 'ask' | 'passthrough'`.
- `BashToolInput`, `Out`, `BashProgress` from `BashTool.tsx:264,296,300`.
- `CommandSemantic` from `commandSemantics.ts:10-17`.

### 4.2 Module-scope state (caches, mutable maps, constants)

| State | Location | Purpose |
|---|---|---|
| `speculativeChecks: Map<string, Promise<ClassifierResult>>` | `bashPermissions.ts:1483` | early classifier dispatch keyed by raw command |
| `MAX_SUBCOMMANDS_FOR_SECURITY_CHECK = 50` | `bashPermissions.ts:103` | event-loop-starvation cap; over the cap → `'ask'` |
| `MAX_SUGGESTED_RULES_FOR_COMPOUND = 5` | `bashPermissions.ts:110` | compound-command suggestion cap |
| `MAX_PERSISTED_SIZE = 64 * 1024 * 1024` | `BashTool.tsx:732` | hard cap on persisted-output truncation |
| `PROGRESS_THRESHOLD_MS = 2000` | `BashTool.tsx:55` | initial progress-show delay |
| `ASSISTANT_BLOCKING_BUDGET_MS = 15_000` | `BashTool.tsx:57` | KAIROS assistant-mode auto-background budget |
| `MAX_IMAGE_FILE_SIZE = 20 MiB` | `utils.ts:96` | image-output OOM guard |
| `MAX_COMMAND_LENGTH = 10000` | `parser.ts:19` | tree-sitter input length cap |
| `PARSE_TIMEOUT_MS = 50`, `MAX_NODES = 50_000` | `bashParser.ts:29,32` | tree-sitter wall-clock and node-budget caps |
| `BASH_MAX_OUTPUT_DEFAULT = 30_000`; `BASH_MAX_OUTPUT_UPPER_LIMIT = 150_000` | `outputLimits.ts:3-4` | output-truncation defaults |
| `DEFAULT_TIMEOUT_MS = 120_000`; `MAX_TIMEOUT_MS = 600_000` | `timeouts.ts:2-3` | run-time defaults |
| `PARSE_ABORTED = Symbol(...)` | `parser.ts:93` | distinguishes timeout/panic from "module not loaded" |
| `BARE_SHELL_PREFIXES` set | `bashPermissions.ts:196-226` | prefixes blocked from auto-suggestion |
| `SAFE_ENV_VARS` set (~40 entries) | `bashPermissions.ts:378-430` | universally safe to strip from rule matching |
| `ANT_ONLY_SAFE_ENV_VARS` set (~30 entries) | `bashPermissions.ts:447-497` | additional ANT-only strippings (verbatim §6.4) |
| `BINARY_HIJACK_VARS = /^(LD_|DYLD_|PATH$)/` | `bashPermissions.ts:708` | blocklist for `stripAllLeadingEnvVars` excludedCommands path |
| `SAFE_WRAPPER_PATTERNS` regex array | `bashPermissions.ts:532-560` | timeout/time/nice/stdbuf/nohup wrapper strippers |
| `BASH_SEARCH_COMMANDS`, `BASH_READ_COMMANDS`, `BASH_LIST_COMMANDS`, `BASH_SEMANTIC_NEUTRAL_COMMANDS`, `BASH_SILENT_COMMANDS`, `COMMON_BACKGROUND_COMMANDS`, `DISALLOWED_AUTO_BACKGROUND_COMMANDS` | `BashTool.tsx:60-81,220,265` | UI/analytics classification |
| `ACCEPT_EDITS_ALLOWED_COMMANDS = ['mkdir','touch','rm','rmdir','mv','cp','sed']` | `modeValidation.ts:7-15` | acceptEdits-mode auto-allow |
| `DANGEROUS_BASH_PATTERNS` (with ANT extensions) | `dangerousPatterns.ts:44-80` | dangerous prefix list (verbatim §6.7) |
| `CROSS_PLATFORM_CODE_EXEC` | `dangerousPatterns.ts:18-42` | shared with PowerShell (verbatim §6.7) |

### 4.3 Persistent state

- Tool-result persistence: when `result.outputFilePath` is set, the file is
  hard-linked (or copied on EXDEV) into the tool-results dir at
  `getToolResultPath(taskId, false)`; truncated to 64 MiB before linking
  (`BashTool.tsx:732-753`). Path is exposed to the model as
  `persistedOutputPath` and rendered via `buildLargeToolResultMessage`.
- Background tasks: `spawnShellTask` registers a `LocalShellTask` keyed by
  `taskId`; `getTaskOutputPath(taskId)` is the stdout/stderr destination
  (`BashTool.tsx:608`).
- `gitOperationTracking` increments OTLP counters (`getCommitCounter`,
  `getPrCounter`) and may fire `linkSessionToPR(sessionId, prNumber, prUrl,
  prRepository)` via dynamic import (`gitOperationTracking.ts:225-247`).

---

## 5. Algorithm / Control Flow

### 5.1 `bashToolHasPermission` (consolidated pseudocode; cite `bashPermissions.ts:1663-2557`)

```
function bashToolHasPermission(input, context, getCommandSubcommandPrefixFn?):
  appState = context.getAppState()
  injectionCheckDisabled = isEnvTruthy(CLAUDE_CODE_DISABLE_COMMAND_INJECTION_CHECK)
  shadowEnabled = TREE_SITTER_BASH_SHADOW ? GrowthBook.tengu_birch_trellis(true) : false

  // 0. AST parse via tree-sitter (only when TREE_SITTER_BASH or _SHADOW is on
  //    AND injection check not disabled AND not shadow-only-killswitched).
  astRoot = injectionCheckDisabled ? null
          : (TREE_SITTER_BASH_SHADOW && !shadowEnabled) ? null
          : await parseCommandRaw(input.command)
  astResult = astRoot ? parseForSecurityFromAst(input.command, astRoot)
                      : { kind: 'parse-unavailable' }

  if (TREE_SITTER_BASH_SHADOW):
    // shadow-only telemetry; force parse-unavailable so legacy stays authoritative.
    record divergence/availability into tengu_tree_sitter_shadow event.
    astResult = { kind: 'parse-unavailable' }; astRoot = null

  if astResult.kind === 'too-complex':
    earlyExit = checkEarlyExitDeny(input, ctx)            // exact match → prefix deny
    if earlyExit: return earlyExit
    logEvent('tengu_bash_ast_too_complex', { nodeTypeId })
    return ask(reason=astResult.reason, suggestions=[],
               pendingClassifierCheck=BASH_CLASSIFIER ? buildPendingClassifierCheck(...) : ø)

  if astResult.kind === 'simple':
    sem = checkSemantics(astResult.commands)
    if !sem.ok:
      earlyExit = checkSemanticsDeny(input, ctx, astResult.commands)
      if earlyExit: return earlyExit
      return ask(reason=sem.reason, suggestions=[])
    astSubcommands = commands.map(c => c.text)
    astRedirects   = commands.flatMap(c => c.redirects)
    astCommands    = commands

  if astResult.kind === 'parse-unavailable':
    legacy = tryParseShellCommand(input.command)        // shell-quote
    if !legacy.success:
      return ask(`Command contains malformed syntax: ${legacy.error}`)

  // 1. Sandbox auto-allow
  if Sandbox.enabled() && Sandbox.autoAllowBashIfSandboxedEnabled() && shouldUseSandbox(input):
    r = checkSandboxAutoAllow(input, ctx)
    if r.behavior !== 'passthrough': return r

  exact = bashToolCheckExactMatchPermission(input, ctx)
  if exact.behavior === 'deny': return exact

  // 2. Classifier deny+ask (BASH_CLASSIFIER, skipped in auto mode w/ TRANSCRIPT_CLASSIFIER)
  if isClassifierPermissionsEnabled() && !(TRANSCRIPT_CLASSIFIER && ctx.mode==='auto'):
    [denyResult, askResult] = await Promise.all([…])
    if denyResult.matches && confidence === 'high':
      return deny(`Denied by Bash prompt rule: "${matchedDescription}"`)
    if askResult.matches && confidence === 'high':
      return ask(suggestions=…, pendingClassifierCheck if BASH_CLASSIFIER)

  // 3. Compound/operator/pipe handling.
  opResult = await checkCommandOperatorPermissions(input, recurse, {isNormalizedCdCommand, isNormalizedGitCommand}, astRoot)
  if opResult.behavior !== 'passthrough':
    if 'allow':
      // re-run safety on full input when astSubcommands===null; cd+redirect re-check; full path-constraint re-check
    if 'ask': attach pendingClassifierCheck if BASH_CLASSIFIER
    return adjusted opResult

  // 4. Legacy misparsing gate (only when astSubcommands===null and injection check not disabled)
  if astSubcommands===null && !injectionCheckDisabled:
    safe = await bashCommandIsSafeAsync(input.command)
    if safe.behavior==='ask' && safe.isBashSecurityCheckForMisparsing:
      remainder = stripSafeHeredocSubstitutions(input.command)
      if remainder===null OR (re-check still misparsing):
        if exact.behavior==='allow': return exact
        return ask(suggestions=[], pendingClassifierCheck if BASH_CLASSIFIER)

  // 5. Subcommand splitting + cd-cwd filter
  rawSubcommands = astSubcommands ?? shadowLegacySubs ?? splitCommand(input.command)
  filter out exact `cd ${cwd}` and `cd ${cwdMingwPosix}` prefixes.
  if astSubcommands===null && subcommands.length > 50:                   // CC-643 cap
    return ask(`Command splits into ${N} subcommands, too many to safety-check individually`)

  // 6. cd guards
  cdCount = subcommands.filter(isNormalizedCdCommand).length
  if cdCount > 1:
    return ask('Multiple directory changes in one command require approval for clarity')
  compoundCommandHasCd = (cdCount > 0)
  if compoundCommandHasCd && subcommands.some(isNormalizedGitCommand):
    return ask('Compound commands with cd and git require approval to prevent bare repository attacks')

  // 7. Per-subcommand bashToolCheckPermission (which itself does: exact-match
  //    → prefix deny/ask/allow → checkPathConstraints → exact allow → prefix
  //    allow → checkSedConstraints → checkPermissionMode → BashTool.isReadOnly)
  decisions = subcommands.map((s,i) => bashToolCheckPermission({command:s}, ctx, compoundCommandHasCd, astCommandsByIdx[i]))

  if any decision is 'deny': return deny(decisionReason='subcommandResults')

  pathResult = checkPathConstraints(input, cwd, ctx, compoundCommandHasCd, astRedirects, astCommands)
  if pathResult.behavior==='deny': return pathResult
  if pathResult.behavior==='ask' && no subcommand independently asked: return pathResult  // GH#28784

  if exactly one subcommand asks:
    return that ask + pendingClassifierCheck(BASH_CLASSIFIER)

  if exact.behavior==='allow': return exact

  // 8. Per-subcommand command-injection re-check (legacy path only)
  if astSubcommands===null && !injectionCheckDisabled:
    results = await Promise.all(subcommands.map(s => bashCommandIsSafeAsync(s, onDivergence)))
    hasPossibleCommandInjection = any(r.behavior!=='passthrough')
  if all decisions allow && !hasPossibleCommandInjection: return allow(decisionReason='subcommandResults')

  // 9. Single-subcommand fast path
  if subcommands.length === 1:
    r = await checkCommandAndSuggestRules({command:subcommands[0]}, ctx, prefix, compoundCommandHasCd, astSubcommands!==null)
    if r.behavior in {'ask','passthrough'}: return r + pendingClassifierCheck(BASH_CLASSIFIER)
    return r

  // 10. Multi-subcommand: collect rules across all subs (cap 5), produce 'ask' or 'passthrough'.
  for each subcommand:
    sub = await checkCommandAndSuggestRules(...)
    if sub allows all → allow(decisionReason='subcommandResults')
    else collect rules from sub.suggestions; synthesize Bash(exact) for ask-without-suggestions (GH#28784).
  cap collected rules at MAX_SUGGESTED_RULES_FOR_COMPOUND=5.
  return { behavior: askSubresult ? 'ask' : 'passthrough', suggestions, pendingClassifierCheck }
```

### 5.2 `bashToolCheckPermission` (per-subcommand, `bashPermissions.ts:1050-1178`)

```
1.  exact = bashToolCheckExactMatchPermission(input, ctx)
1a. if exact in {deny,ask}: return exact
2.  matching = matchingRulesForInput(input, ctx, 'prefix', skipCompoundCheck=astCommand!==undefined)
2a. if deny rule: return deny
2b. if ask  rule: return ask
3.  pathResult = checkPathConstraints(input, cwd, ctx, compoundCommandHasCd, astCommand?.redirects, [astCommand])
    if pathResult.behavior !== 'passthrough': return pathResult
4.  if exact.behavior === 'allow': return exact
5.  if matchingAllow rule: return allow
5b. sed = checkSedConstraints(input, ctx); if non-passthrough: return
6.  mode = checkPermissionMode(input, ctx); if non-passthrough: return mode-result
7.  if BashTool.isReadOnly(input): return allow(reason='Read-only command is allowed')
8.  passthrough(reason='This command requires approval', suggestions=suggestionForExactCommand(command))
```

### 5.3 `bashToolCheckExactMatchPermission`

Same shape but operates with `matchMode='exact'`. Order: deny → ask → allow →
passthrough (with `suggestionForExactCommand`). `bashPermissions.ts:991-1048`.

### 5.4 `matchingRulesForInput`

For each of `deny`, `ask`, `allow` rule sets:
- get rule-by-contents for tool via `getRuleByContentsForTool(ctx, BashTool, kind)`.
- `filterRulesByContentsMatchingInput(...)` with options. Deny/ask use
  `stripAllEnvVars=true, skipCompoundCheck=true`; allow uses neither (default
  passes `skipCompoundCheck` from caller). `bashPermissions.ts:937-986`.

`filterRulesByContentsMatchingInput` core (`:778-935`):
1. `commandWithoutRedirections = extractOutputRedirections(command).commandWithoutRedirections`.
2. Build candidate set: original (exact mode only), `commandWithoutRedirections`,
   then for each, also `stripSafeWrappers(...)` if it differs.
3. If `stripAllEnvVars` (deny/ask): iterate to fixed point applying both
   `stripAllLeadingEnvVars` and `stripSafeWrappers` to every candidate.
4. Precompute `isCompoundCommand` per candidate via `splitCommand(c).length > 1`
   (only for 'prefix' mode, only when `!skipCompoundCheck`).
5. For each rule × candidate:
   - `'exact'`: equality.
   - `'prefix'` in 'exact' mode: equality with rule.prefix.
   - `'prefix'` in 'prefix' mode: skip if compound; match if cmd === prefix
     OR `cmd.startsWith(prefix + ' ')` OR `cmd === ('xargs ' + prefix)` OR
     `cmd.startsWith('xargs ' + prefix + ' ')`.
   - `'wildcard'` in 'exact' mode: never (security fix, `:920`).
   - `'wildcard'` in 'prefix' mode: skip if compound; else `matchWildcardPattern`.

### 5.5 `stripSafeWrappers` (full algorithm, `bashPermissions.ts:524-615`)

Phase 1: iterate-to-fixed-point: strip leading full-line `#` comments
(`stripCommentLines` keeps non-`#` lines; if all `#`/empty, returns original);
match unquoted `VAR=val` prefix only when `VAR ∈ SAFE_ENV_VARS` or
(`USER_TYPE==='ant'` and `VAR ∈ ANT_ONLY_SAFE_ENV_VARS`); strip with regex
`^([A-Za-z_][A-Za-z0-9_]*)=([A-Za-z0-9_./:-]+)[ \t]+/`. Phase 2:
iterate-to-fixed-point applying SAFE_WRAPPER_PATTERNS regexes: `timeout` (full
GNU flag enumeration with allowlist for flag values), `time`, `nice` (with
`-n N` or `-N`), `stdbuf` (fused short flags only), `nohup`. Each pattern also
consumes optional `--` end-of-options. Trailing whitespace MUST be `[ \t]+`
(NOT `\s+`); rationale (security): `\s` matches `\n` which is a bash command
separator.

### 5.6 `stripAllLeadingEnvVars` (`bashPermissions.ts:733-776`)

Same iterate-to-fixed-point shape, but uses a broader value pattern that
allows append (`FOO+=bar`), array (`FOO[0]=bar`), single-quoted, double-quoted
(with backslash escapes), and unquoted values, while excluding shell injection
chars (`$`, backtick, `;`, `|`, `&`, parens, redirects, quotes, backslash).
`$` excluded from unquoted/double-quoted classes (CodeQL #671 ReDoS).
Optional `blocklist` regex tested per-var-name; matched name halts stripping
(used by sandbox `excludedCommands` with `BINARY_HIJACK_VARS`).

### 5.7 `shouldUseSandbox` (`shouldUseSandbox.ts:130-153`)

```
if !SandboxManager.isSandboxingEnabled(): return false
if input.dangerouslyDisableSandbox && SandboxManager.areUnsandboxedCommandsAllowed(): return false
if !input.command: return false
if containsExcludedCommand(input.command): return false
return true
```

`containsExcludedCommand`:
- ANT-only (`USER_TYPE==='ant'`): `getFeatureValue_CACHED_MAY_BE_STALE<{commands:[],substrings:[]}>('tengu_sandbox_disabled_commands', {commands:[], substrings:[]})`. Match if any substring is contained in command, or if any first-token of any subcommand is in `commands`.
- All users: read `settings.sandbox?.excludedCommands ?? []`. Split via
  `splitCommand_DEPRECATED`. Per-subcommand build candidate set by iterating
  `stripAllLeadingEnvVars(c, BINARY_HIJACK_VARS)` + `stripSafeWrappers(c)` to
  fixed point. For each pattern, parse with `bashPermissionRule` and match
  same way as the rule matcher (`exact|prefix|wildcard`).

`shouldUseSandbox` is called from `BashTool.tsx:502,896` (UI label and the
shell run-time `shouldUseSandbox` option) and from `bashPermissions.ts:1834`
(auto-allow gate).

### 5.8 `checkReadOnlyConstraints` (read-only gate; `readOnlyValidation.ts`)

`isReadOnly(input)` is called by `BashTool.tsx:437-441` (using
`commandHasAnyCd(input.command)` for the compound flag). The full
`readOnlyValidation.ts` enumerates `COMMAND_ALLOWLIST` (xargs, all
`GIT_READ_ONLY_COMMANDS`, file, ...) plus a `READONLY_COMMANDS` regex list.
ANT-only delta at `:1211`: when `USER_TYPE==='ant'`, returns
`{...allowlist, ...ANT_ONLY_COMMAND_ALLOWLIST}`. Windows additionally removes
`xargs` (`:1207-1210`). `SAFE_TARGET_COMMANDS_FOR_XARGS = ['echo','printf',
'wc','grep','head','tail']` (`:1232-1239`). The function rejects any token
containing `$` (variable expansion) or `{` together with `,` or `..` (brace
expansion); blocks newlines/CR in grep/rg patterns; runs the command-config
regex (must contain backtick check when no regex configured).

### 5.9 `BashTool.call` lifecycle (`BashTool.tsx:624-820`)

```
if input._simulatedSedEdit: return applySedEdit(...)  // bypass shell entirely

isMainThread = !context.agentId
preventCwdChanges = !isMainThread
result = (yield from runShellCommand({...}))
trackGitOperations(input.command, result.code, result.stdout)
stdout accumulator <- (result.stdout ?? '').trimEnd() + '\n'
interpretation = interpretCommandResult(input.command, result.code, result.stdout, '')
if stdout includes ".git/index.lock': File exists": logEvent('tengu_git_index_lock_error',{})
if interpretation.isError && !isInterrupt && code!==0: append `Exit code ${code}` to accumulator
if !preventCwdChanges:
  if resetCwdIfOutsideProject(toolPermissionContext): stderrForShellReset = stdErrAppendShellResetMessage('')
output = SandboxManager.annotateStderrWithSandboxFailures(input.command, result.stdout)
if result.preSpawnError: throw new Error(result.preSpawnError)
if interpretation.isError && !isInterrupt: throw new ShellError('', output, code, interrupted)
// Persist large output:
if result.outputFilePath && result.outputTaskId:
  size = stat(outputFilePath).size; ensureToolResultsDir()
  dest = getToolResultPath(outputTaskId, false)
  if size > 64MiB: truncate(outputFilePath, 64MiB)
  link(outputFilePath, dest) || copyFile(outputFilePath, dest)
  persistedOutputPath = dest; persistedOutputSize = size
logEvent('tengu_bash_tool_command_executed', {...})
if (codeIndexingTool := detectCodeIndexingFromCommand(input.command)): logEvent('tengu_code_indexing_tool_used', {...})
strippedStdout = stripEmptyLines(stdout)
extracted = extractClaudeCodeHints(strippedStdout, input.command); strippedStdout = extracted.stripped
if isMainThread: each hint → maybeRecordPluginHint(hint)
if isImageOutput(stdout): try resizeShellImageOutput; if fails, isImage=false (text fallback)
return { data: { stdout, stderr: stderrForShellReset, interrupted, isImage, returnCodeInterpretation,
  noOutputExpected: isSilentBashCommand(input.command), backgroundTaskId, backgroundedByUser,
  assistantAutoBackgrounded, dangerouslyDisableSandbox, persistedOutputPath, persistedOutputSize } }
```

`runShellCommand` is an async generator (`BashTool.tsx:826-1142`):

- `timeoutMs = timeout || getDefaultTimeoutMs()`.
- `shouldAutoBackground = !isBackgroundTasksDisabled && isAutobackgroundingAllowed(command)`.
- The shell run goes through `Shell.exec(command, abortSignal, 'bash', { timeout, onProgress, preventCwdChanges, shouldUseSandbox: shouldUseSandbox(input), shouldAutoBackground })`.
- If `run_in_background === true && !isBackgroundTasksDisabled`: spawn
  background task, log `tengu_bash_command_explicitly_backgrounded`, return
  `{ stdout:'', stderr:'', code:0, interrupted:false, backgroundTaskId }`.
- Initial wait: `Promise.race(resultPromise, timer(PROGRESS_THRESHOLD_MS=2000))`.
- Auto-background hooks:
  - `shellCommand.onTimeout` → `startBackgrounding('tengu_bash_command_timeout_backgrounded', backgroundFn)`.
  - `feature('KAIROS') && getKairosActive() && isMainThread && !isBackgroundTasksDisabled && run_in_background!==true` → setTimeout(15000ms) → `'tengu_bash_command_assistant_auto_backgrounded'` and `assistantAutoBackgrounded = true`.
- Progress loop: yields `{type:'progress', output, fullOutput, elapsedTimeSeconds, totalLines, totalBytes, taskId, timeoutMs?}`.

### 5.10 `mapToolResultToToolResultBlockParam` (`BashTool.tsx:555-623`)

```
if structuredContent && length > 0: return { content: structuredContent }
if isImage: try buildImageToolResult(stdout, toolUseID); on success return image block
processedStdout = stdout.replace(/^(\s*\n)+/, '').trimEnd()
if persistedOutputPath:
  preview = generatePreview(processedStdout, PREVIEW_SIZE_BYTES)
  processedStdout = buildLargeToolResultMessage({ filepath, originalSize, isJson:false, preview, hasMore })
errorMessage = stderr.trim()
if interrupted: errorMessage += '\n<error>Command was aborted before completion</error>'
if backgroundTaskId:
  if assistantAutoBackgrounded:
    backgroundInfo = `Command exceeded the assistant-mode blocking budget (${ASSISTANT_BLOCKING_BUDGET_MS/1000}s) and was moved to the background with ID: ${backgroundTaskId}. It is still running — you will be notified when it completes. Output is being written to: ${outputPath}. In assistant mode, delegate long-running work to a subagent or use run_in_background to keep this conversation responsive.`
  elif backgroundedByUser:
    backgroundInfo = `Command was manually backgrounded by user with ID: ${backgroundTaskId}. Output is being written to: ${outputPath}`
  else:
    backgroundInfo = `Command running in background with ID: ${backgroundTaskId}. Output is being written to: ${outputPath}`
return { content: [processedStdout, errorMessage, backgroundInfo].filter(Boolean).join('\n'), is_error: interrupted }
```

### 5.11 Tree-sitter parser gate (`src/utils/bash/parser.ts`)

```
parseCommand(command):
  if !command || command.length > 10000: return null
  if feature('TREE_SITTER_BASH'):
    await ensureParserInitialized(); mod = getParserModule(); logLoadOnce(mod!==null)
    if !mod: return null
    try: rootNode = mod.parse(command); if !rootNode: return null
         commandNode = findCommandNode(rootNode, null); envVars = extractEnvVars(commandNode)
         return { rootNode, envVars, commandNode, originalCommand: command }
    catch: return null
  return null

parseCommandRaw(command): // identical structure but gated on
  if feature('TREE_SITTER_BASH') || feature('TREE_SITTER_BASH_SHADOW'):
    if mod.parse(command) === null: logEvent('tengu_tree_sitter_parse_abort', {cmdLength, panic:false}); return PARSE_ABORTED
    on throw: logEvent('tengu_tree_sitter_parse_abort', {cmdLength, panic:true}); return PARSE_ABORTED
  return null
```

`PARSE_ABORTED` is a `Symbol('parse-aborted')` distinct from `null`; callers
(`bashPermissions.ts:1692`, `bashCommandHelpers.ts:189`) MUST treat it as
fail-closed (`too-complex`).

### 5.12 `gitOperationTracking.trackGitOperations` (`gitOperationTracking.ts:189-277`)

Triggered after every `BashTool.call` run result. Skips on non-zero exit.
Regex matchers tolerate `git -c key=val`, `-C path`, `--gitdir=path` between
`git` and the subcommand. Fires `tengu_git_operation` with operation in
{`commit`, `commit_amend` (if `--amend`), `push`, `pr_create|pr_edit|pr_merge|
pr_comment|pr_close|pr_ready` (gh), `pr_create` (glab/curl)}. Increments OTLP
counters. On gh `pr create` success, dynamically imports
`utils/sessionStorage.linkSessionToPR(sessionId, prNumber, prUrl, prRepository)`
when stdout contains a parseable GitHub PR URL.

`detectGitOperation` is exported separately (`:135-186`) for tool-use summary
rendering ("committed a1b2c3, created PR #42, ran 3 bash commands").

---

## 6. Verbatim Assets

### 6.1 BashTool prompt (full text — default external-user variant)

The prompt is assembled by `getSimplePrompt()` (`prompt.ts:275-369`). Inlined
below is the **complete external-user output** (`USER_TYPE !== 'ant'`,
`hasEmbeddedSearchTools()===false`, `feature('MONITOR_TOOL')===false`,
`SandboxManager.isSandboxingEnabled()===false`, `shouldIncludeGitInstructions()===true`).
String literals are preserved exactly:

```
Executes a given bash command and returns its output.

The working directory persists between commands, but shell state does not. The shell environment is initialized from the user's profile (bash or zsh).

IMPORTANT: Avoid using this tool to run `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands, unless explicitly instructed or after you have verified that a dedicated tool cannot accomplish your task. Instead, use the appropriate dedicated tool as this will provide a much better experience for the user:

 - File search: Use Glob (NOT find or ls)
 - Content search: Use Grep (NOT grep or rg)
 - Read files: Use Read (NOT cat/head/tail)
 - Edit files: Use Edit (NOT sed/awk)
 - Write files: Use Write (NOT echo >/cat <<EOF)
 - Communication: Output text directly (NOT echo/printf)
While the Bash tool can do similar things, it’s better to use the built-in tools as they provide a better user experience and make it easier to review tool calls and give permission.

# Instructions
 - If your command will create new directories or files, first use this tool to run `ls` to verify the parent directory exists and is the correct location.
 - Always quote file paths that contain spaces with double quotes in your command (e.g., cd "path with spaces/file.txt")
 - Try to maintain your current working directory throughout the session by using absolute paths and avoiding usage of `cd`. You may use `cd` if the User explicitly requests it.
 - You may specify an optional timeout in milliseconds (up to ${getMaxTimeoutMs()}ms / ${getMaxTimeoutMs()/60000} minutes). By default, your command will timeout after ${getDefaultTimeoutMs()}ms (${getDefaultTimeoutMs()/60000} minutes).
 - You can use the `run_in_background` parameter to run the command in the background. Only use this if you don't need the result immediately and are OK being notified when the command completes later. You do not need to check the output right away - you'll be notified when it finishes. You do not need to use '&' at the end of the command when using this parameter.
 - When issuing multiple commands:
   - If the commands are independent and can run in parallel, make multiple Bash tool calls in a single message. Example: if you need to run "git status" and "git diff", send a single message with two Bash tool calls in parallel.
   - If the commands depend on each other and must run sequentially, use a single Bash call with '&&' to chain them together.
   - Use ';' only when you need to run commands sequentially but don't care if earlier commands fail.
   - DO NOT use newlines to separate commands (newlines are ok in quoted strings).
 - For git commands:
   - Prefer to create a new commit rather than amending an existing commit.
   - Before running destructive operations (e.g., git reset --hard, git push --force, git checkout --), consider whether there is a safer alternative that achieves the same goal. Only use destructive operations when they are truly the best approach.
   - Never skip hooks (--no-verify) or bypass signing (--no-gpg-sign, -c commit.gpgsign=false) unless the user has explicitly asked for it. If a hook fails, investigate and fix the underlying issue.
 - Avoid unnecessary `sleep` commands:
   - Do not sleep between commands that can run immediately — just run them.
   - If your command is long running and you would like to be notified when it finishes — use `run_in_background`. No sleep needed.
   - Do not retry failing commands in a sleep loop — diagnose the root cause.
   - If waiting for a background task you started with `run_in_background`, you will be notified when it completes — do not poll.
   - If you must poll an external process, use a check command (e.g. `gh run view`) rather than sleeping first.
   - If you must sleep, keep the duration short (1-5 seconds) to avoid blocking the user.

```

The "git commit and PR" block is appended via `getCommitAndPRInstructions()`
verbatim from `prompt.ts:81-160`:

```
# Committing changes with git

Only create commits when requested by the user. If unclear, ask first. When the user asks you to create a new git commit, follow these steps carefully:

You can call multiple tools in a single response. When multiple independent pieces of information are requested and all commands are likely to succeed, run multiple tool calls in parallel for optimal performance. The numbered steps below indicate which commands should be batched in parallel.

Git Safety Protocol:
- NEVER update the git config
- NEVER run destructive git commands (push --force, reset --hard, checkout ., restore ., clean -f, branch -D) unless the user explicitly requests these actions. Taking unauthorized destructive actions is unhelpful and can result in lost work, so it's best to ONLY run these commands when given direct instructions 
- NEVER skip hooks (--no-verify, --no-gpg-sign, etc) unless the user explicitly requests it
- NEVER run force push to main/master, warn the user if they request it
- CRITICAL: Always create NEW commits rather than amending, unless the user explicitly requests a git amend. When a pre-commit hook fails, the commit did NOT happen — so --amend would modify the PREVIOUS commit, which may result in destroying work or losing previous changes. Instead, after hook failure, fix the issue, re-stage, and create a NEW commit
- When staging files, prefer adding specific files by name rather than using "git add -A" or "git add .", which can accidentally include sensitive files (.env, credentials) or large binaries
- NEVER commit changes unless the user explicitly asks you to. It is VERY IMPORTANT to only commit when explicitly asked, otherwise the user will feel that you are being too proactive

1. Run the following bash commands in parallel, each using the ${BASH_TOOL_NAME} tool:
  - Run a git status command to see all untracked files. IMPORTANT: Never use the -uall flag as it can cause memory issues on large repos.
  - Run a git diff command to see both staged and unstaged changes that will be committed.
  - Run a git log command to see recent commit messages, so that you can follow this repository's commit message style.
2. Analyze all staged changes (both previously staged and newly added) and draft a commit message:
  - Summarize the nature of the changes (eg. new feature, enhancement to an existing feature, bug fix, refactoring, test, docs, etc.). Ensure the message accurately reflects the changes and their purpose (i.e. "add" means a wholly new feature, "update" means an enhancement to an existing feature, "fix" means a bug fix, etc.).
  - Do not commit files that likely contain secrets (.env, credentials.json, etc). Warn the user if they specifically request to commit those files
  - Draft a concise (1-2 sentences) commit message that focuses on the "why" rather than the "what"
  - Ensure it accurately reflects the changes and their purpose
3. Run the following commands in parallel:
   - Add relevant untracked files to the staging area.
   - Create the commit with a message${commitAttribution ? ` ending with:\n   ${commitAttribution}` : '.'}
   - Run git status after the commit completes to verify success.
   Note: git status depends on the commit completing, so run it sequentially after the commit.
4. If the commit fails due to pre-commit hook: fix the issue and create a NEW commit

Important notes:
- NEVER run additional commands to read or explore code, besides git bash commands
- NEVER use the ${TodoWriteTool.name} or ${AGENT_TOOL_NAME} tools
- DO NOT push to the remote repository unless the user explicitly asks you to do so
- IMPORTANT: Never use git commands with the -i flag (like git rebase -i or git add -i) since they require interactive input which is not supported.
- IMPORTANT: Do not use --no-edit with git rebase commands, as the --no-edit flag is not a valid option for git rebase.
- If there are no changes to commit (i.e., no untracked files and no modifications), do not create an empty commit
- In order to ensure good formatting, ALWAYS pass the commit message via a HEREDOC, a la this example:
<example>
git commit -m "$(cat <<'EOF'
   Commit message here.${commitAttribution ? `\n\n   ${commitAttribution}` : ''}
   EOF
   )"
</example>

# Creating pull requests
Use the gh command via the Bash tool for ALL GitHub-related tasks including working with issues, pull requests, checks, and releases. If given a Github URL use the gh command to get the information needed.

IMPORTANT: When the user asks you to create a pull request, follow these steps carefully:

1. Run the following bash commands in parallel using the ${BASH_TOOL_NAME} tool, in order to understand the current state of the branch since it diverged from the main branch:
   - Run a git status command to see all untracked files (never use -uall flag)
   - Run a git diff command to see both staged and unstaged changes that will be committed
   - Check if the current branch tracks a remote branch and is up to date with the remote, so you know if you need to push to the remote
   - Run a git log command and `git diff [base-branch]...HEAD` to understand the full commit history for the current branch (from the time it diverged from the base branch)
2. Analyze all changes that will be included in the pull request, making sure to look at all relevant commits (NOT just the latest commit, but ALL commits that will be included in the pull request!!!), and draft a pull request title and summary:
   - Keep the PR title short (under 70 characters)
   - Use the description/body for details, not the title
3. Run the following commands in parallel:
   - Create new branch if needed
   - Push to remote with -u flag if needed
   - Create PR using gh pr create with the format below. Use a HEREDOC to pass the body to ensure correct formatting.
<example>
gh pr create --title "the pr title" --body "$(cat <<'EOF'
## Summary
<1-3 bullet points>

## Test plan
[Bulleted markdown checklist of TODOs for testing the pull request...]${prAttribution ? `\n\n${prAttribution}` : ''}
EOF
)"
</example>

Important:
- DO NOT use the ${TodoWriteTool.name} or ${AGENT_TOOL_NAME} tools
- Return the PR URL when you're done, so the user can see it

# Other common operations
- View comments on a Github PR: gh api repos/foo/bar/pulls/123/comments
```

Notes on slot variables (resolved at prompt time):
- `commitAttribution`, `prAttribution` come from `getAttributionTexts()`
  (`src/utils/attribution.ts`); when COMMIT_ATTRIBUTION feature is on and the
  hooks are registered (`setup.ts:350`), the attribution lines materialize.
  When COMMIT_ATTRIBUTION is off, both are empty strings, and the literal
  fragments `commitAttribution ? '...' : '.'` collapse to `.` and the example
  loses the attribution line.
- `BASH_TOOL_NAME = 'Bash'`, `TodoWriteTool.name`, `AGENT_TOOL_NAME` are
  inlined as their literal name strings.

### 6.2 ANT-only / undercover prompt branch (verbatim)

When `USER_TYPE === 'ant'`, `getCommitAndPRInstructions()` returns the
short-version block (`prompt.ts:56-75`):

```
# Git operations

[skillsSection — only if !CLAUDE_CODE_SIMPLE]

IMPORTANT: NEVER skip hooks (--no-verify, --no-gpg-sign, etc) unless the user explicitly requests it.

Use the gh command via the Bash tool for other GitHub-related tasks including working with issues, checks, and releases. If given a Github URL use the gh command to get the information needed.

# Other common operations
- View comments on a Github PR: gh api repos/foo/bar/pulls/123/comments
```

Where `skillsSection` (when active) is:

```
For git commits and pull requests, use the `/commit` and `/commit-push-pr` skills:
- `/commit` - Create a git commit with staged changes
- `/commit-push-pr` - Commit, push, and create a pull request

These skills handle git safety protocols, proper commit message formatting, and PR creation.

Before creating a pull request, run `/simplify` to review your changes, then test end-to-end (e.g. via `/tmux` for interactive features).

```

Prepended to the git block (and emitted even if `shouldIncludeGitInstructions()===false`)
when `USER_TYPE === 'ant' && isUndercover()` is the verbatim
`getUndercoverInstructions()` block (`src/utils/undercover.ts:39-71`):

```
## UNDERCOVER MODE — CRITICAL

You are operating UNDERCOVER in a PUBLIC/OPEN-SOURCE repository. Your commit
messages, PR titles, and PR bodies MUST NOT contain ANY Anthropic-internal
information. Do not blow your cover.

NEVER include in commit messages or PR descriptions:
- Internal model codenames (animal names like Capybara, Tengu, etc.)
- Unreleased model version numbers (e.g., opus-4-7, sonnet-4-8)
- Internal repo or project names (e.g., claude-cli-internal, anthropics/…)
- Internal tooling, Slack channels, or short links (e.g., go/cc, #claude-code-…)
- The phrase "Claude Code" or any mention that you are an AI
- Any hint of what model or version you are
- Co-Authored-By lines or any other attribution

Write commit messages as a human developer would — describe only what the code
change does.

GOOD:
- "Fix race condition in file watcher initialization"
- "Add support for custom key bindings"
- "Refactor parser for better error messages"

BAD (never write these):
- "Fix bug found while testing with Claude Capybara"
- "1-shotted by claude-opus-4-6"
- "Generated with Claude Code"
- "Co-Authored-By: Claude Opus 4.6 <…>"
```

Activation rules: `isUndercover()` returns `true` when `USER_TYPE==='ant'`
AND (`CLAUDE_CODE_UNDERCOVER` truthy OR `getRepoClassCached() !== 'internal'`).
There is no force-OFF (`undercover.ts:28-37`).

### 6.3 Sandbox prompt section (verbatim text fragments — `prompt.ts:172-273`)

When `SandboxManager.isSandboxingEnabled()` returns true, `getSimpleSandboxSection()`
appends:

```
## Command sandbox
By default, your command will be run in a sandbox. This sandbox controls which directories and network hosts commands may access or modify without an explicit override.

The sandbox has the following restrictions:
[restrictions block: lines for "Filesystem: <jsonStringify(filesystemConfig)>", "Network: <jsonStringify(networkConfig)>", and optionally "Ignored violations: <jsonStringify(ignoreViolations)>"]

[items block, prepended with bullets]
```

When `allowUnsandboxedCommands === true` (`SandboxManager.areUnsandboxedCommandsAllowed()`),
the items block uses these literals (verbatim):

```
You should always default to running commands within the sandbox. Do NOT attempt to set `dangerouslyDisableSandbox: true` unless:
  - The user *explicitly* asks you to bypass sandbox
  - A specific command just failed and you see evidence of sandbox restrictions causing the failure. Note that commands can fail for many reasons unrelated to the sandbox (missing files, wrong arguments, network issues, etc.).
Evidence of sandbox-caused failures includes:
  - "Operation not permitted" errors for file/network operations
  - Access denied to specific paths outside allowed directories
  - Network connection failures to non-whitelisted hosts
  - Unix socket connection errors
When you see evidence of sandbox-caused failure:
  - Immediately retry with `dangerouslyDisableSandbox: true` (don't ask, just do it)
  - Briefly explain what sandbox restriction likely caused the failure. Be sure to mention that the user can use the `/sandbox` command to manage restrictions.
  - This will prompt the user for permission
Treat each command you execute with `dangerouslyDisableSandbox: true` individually. Even if you have recently run a command with this setting, you should default to running future commands within the sandbox.
Do not suggest adding sensitive paths like ~/.bashrc, ~/.zshrc, ~/.ssh/*, or credential files to the sandbox allowlist.
For temporary files, always use the `$TMPDIR` environment variable. TMPDIR is automatically set to the correct sandbox-writable directory in sandbox mode. Do NOT use `/tmp` directly - use `$TMPDIR` instead.
```

When `allowUnsandboxedCommands === false`:

```
All commands MUST run in sandbox mode - the `dangerouslyDisableSandbox` parameter is disabled by policy.
Commands cannot run outside the sandbox under any circumstances.
If a command fails due to sandbox restrictions, work with the user to adjust sandbox settings instead.
For temporary files, always use the `$TMPDIR` environment variable. TMPDIR is automatically set to the correct sandbox-writable directory in sandbox mode. Do NOT use `/tmp` directly - use `$TMPDIR` instead.
```

The sandbox restrictions block emits a `$TMPDIR` substitution: any `allowOnly`
write path equal to `getClaudeTempDir()` (e.g. `/private/tmp/claude-1001/`)
is replaced with the literal `$TMPDIR` to keep the global prompt cache
identical across users (`prompt.ts:188-190`).

`MONITOR_TOOL` sleep subitems (`prompt.ts:312-326`) — when ON, replace the
last two sleep bullets with:

```
Use the Monitor tool to stream events from a background process (each stdout line is a notification). For one-shot "wait until done," use Bash with run_in_background instead.
[…]
`sleep N` as the first command with N ≥ 2 is blocked. If you need a delay (rate limiting, deliberate pacing), keep it under 2 seconds.
```

`hasEmbeddedSearchTools()` true (ANT bfs/ugrep) drops the Glob/Grep tool
preference lines and uses the avoid list `` `cat`, `head`, `tail`, `sed`, `awk`, or `echo` ``;
also appends the leftmost-first regex tip (`prompt.ts:343-351`).

`CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` truthy: `getBackgroundUsageNote()`
returns null and the `run_in_background` bullet is omitted (`prompt.ts:36-40`).

### 6.4 Constants tables

#### Bash run-time timeouts (verbatim)

| Constant | Default | Source |
|---|---|---|
| `DEFAULT_TIMEOUT_MS` | `120_000` ms (2 min) | `src/utils/timeouts.ts:2` |
| `MAX_TIMEOUT_MS` | `600_000` ms (10 min) | `src/utils/timeouts.ts:3` |
| `BASH_DEFAULT_TIMEOUT_MS` env override | parsed >0 ms | `:13-19` |
| `BASH_MAX_TIMEOUT_MS` env override | clamped to ≥ default | `:29-38` |

#### Output truncation (verbatim — `outputLimits.ts`, `utils.ts:148-164`)

| Constant | Value | Notes |
|---|---|---|
| `BASH_MAX_OUTPUT_DEFAULT` | `30_000` chars | per-call cap |
| `BASH_MAX_OUTPUT_UPPER_LIMIT` | `150_000` chars | env-override ceiling |
| `BASH_MAX_OUTPUT_LENGTH` | env override | bounded between default and upper |
| `MAX_PERSISTED_SIZE` | `64 * 1024 * 1024` (64 MiB) | persisted-output truncate cap |
| `MAX_IMAGE_FILE_SIZE` | `20 * 1024 * 1024` (20 MiB) | image data-URI guard |

**Output truncation suffix** (verbatim, `utils.ts:158`):

```
${truncatedPart}\n\n... [${remainingLines} lines truncated] ...
```

Built as: take first `maxOutputLength` chars; count newlines after the cut;
append `\n\n... [${remainingLines} lines truncated] ...`. Image outputs are
not truncated this way.

#### Tree-sitter parser caps

| Constant | Value | Source |
|---|---|---|
| `MAX_COMMAND_LENGTH` | `10000` | `parser.ts:19` |
| `PARSE_TIMEOUT_MS` | `50` | `bashParser.ts:29` |
| `MAX_NODES` | `50_000` | `bashParser.ts:32` |

#### `SAFE_ENV_VARS` (verbatim, `bashPermissions.ts:378-430`)

```
GOEXPERIMENT, GOOS, GOARCH, CGO_ENABLED, GO111MODULE,
RUST_BACKTRACE, RUST_LOG,
NODE_ENV,
PYTHONUNBUFFERED, PYTHONDONTWRITEBYTECODE,
PYTEST_DISABLE_PLUGIN_AUTOLOAD, PYTEST_DEBUG,
ANTHROPIC_API_KEY,
LANG, LANGUAGE, LC_ALL, LC_CTYPE, LC_TIME, CHARSET,
TERM, COLORTERM, NO_COLOR, FORCE_COLOR, TZ,
LS_COLORS, LSCOLORS, GREP_COLOR, GREP_COLORS, GCC_COLORS,
TIME_STYLE, BLOCK_SIZE, BLOCKSIZE
```

**Forbidden additions** (security note `:372-377`): `PATH`, `LD_PRELOAD`,
`LD_LIBRARY_PATH`, `DYLD_*`, `PYTHONPATH`, `NODE_PATH`, `CLASSPATH`, `RUBYLIB`,
`GOFLAGS`, `RUSTFLAGS`, `NODE_OPTIONS`, `HOME`, `TMPDIR`, `SHELL`, `BASH_ENV`.

#### `ANT_ONLY_SAFE_ENV_VARS` (verbatim, `bashPermissions.ts:447-497`)

```
KUBECONFIG, DOCKER_HOST,
AWS_PROFILE, CLOUDSDK_CORE_PROJECT, CLUSTER,
COO_CLUSTER, COO_CLUSTER_NAME, COO_NAMESPACE, COO_LAUNCH_YAML_DRY_RUN,
SKIP_NODE_VERSION_CHECK, EXPECTTEST_ACCEPT, CI, GIT_LFS_SKIP_SMUDGE,
CUDA_VISIBLE_DEVICES, JAX_PLATFORMS,
COLUMNS, TMUX,
POSTGRESQL_VERSION, FIRESTORE_EMULATOR_HOST, HARNESS_QUIET,
TEST_CROSSCHECK_LISTS_MATCH_UPDATE, DBT_PER_DEVELOPER_ENVIRONMENTS,
STATSIG_FORD_DB_CHECKS,
ANT_ENVIRONMENT, ANT_SERVICE, MONOREPO_ROOT_DIR,
PYENV_VERSION,
PGPASSWORD, GH_TOKEN, GROWTHBOOK_API_KEY
```

These are stripped only in `USER_TYPE==='ant'` builds and explicitly MUST
NEVER ship to external users (security comment `:432-446`).

#### `BARE_SHELL_PREFIXES` (verbatim, `bashPermissions.ts:196-226`)

```
sh, bash, zsh, fish, csh, tcsh, ksh, dash, cmd, powershell, pwsh,
env, xargs, nice, stdbuf, nohup, timeout, time,
sudo, doas, pkexec
```

Suggestions for these shapes are forbidden (would yield ≈ `Bash(*)`).

#### `BINARY_HIJACK_VARS` regex (verbatim, `bashPermissions.ts:708`)

```
/^(LD_|DYLD_|PATH$)/
```

#### `DANGEROUS_BASH_PATTERNS` (verbatim, `dangerousPatterns.ts:18-80`)

```
CROSS_PLATFORM_CODE_EXEC = [
  'python','python3','python2','node','deno','tsx','ruby','perl','php','lua',
  'npx','bunx','npm run','yarn run','pnpm run','bun run',
  'bash','sh',
  'ssh',
] as const

DANGEROUS_BASH_PATTERNS = [
  ...CROSS_PLATFORM_CODE_EXEC,
  'zsh','fish',
  'eval','exec','env','xargs','sudo',
  // ANT-only extension when USER_TYPE === 'ant':
  'fa run','coo','gh','gh api','curl','wget','git','kubectl','aws','gcloud','gsutil',
]
```

`DANGEROUS_BASH_PATTERNS` is consumed by spec 09 / `permissionSetup.ts` to
strip these prefixes from allow-rules at auto-mode entry. BashTool itself
does NOT scan `DANGEROUS_BASH_PATTERNS` directly; it inherits the effect via
the shared rule store.

#### Auto-allow / classification sets (verbatim)

`ACCEPT_EDITS_ALLOWED_COMMANDS` (`modeValidation.ts:7-15`):
```
mkdir, touch, rm, rmdir, mv, cp, sed
```

`COMMON_BACKGROUND_COMMANDS` (`BashTool.tsx:265`):
```
npm, yarn, pnpm, node, python, python3, go, cargo, make, docker, terraform,
webpack, vite, jest, pytest, curl, wget, build, test, serve, watch, dev
```

`DISALLOWED_AUTO_BACKGROUND_COMMANDS` (`BashTool.tsx:220`): `['sleep']`.

`SAFE_TARGET_COMMANDS_FOR_XARGS` (`readOnlyValidation.ts:1232-1239`):
```
echo, printf, wc, grep, head, tail
```

UI classification (`BashTool.tsx:60-81`):
- `BASH_SEARCH_COMMANDS = ['find','grep','rg','ag','ack','locate','which','whereis']`
- `BASH_READ_COMMANDS = ['cat','head','tail','less','more','wc','stat','file','strings','jq','awk','cut','sort','uniq','tr']`
- `BASH_LIST_COMMANDS = ['ls','tree','du']`
- `BASH_SEMANTIC_NEUTRAL_COMMANDS = ['echo','printf','true','false',':']`
- `BASH_SILENT_COMMANDS = ['mv','cp','rm','mkdir','rmdir','chmod','chown','chgrp','touch','ln','cd','export','unset','wait']`

### 6.5 User-facing error / denial / decision strings (verbatim)

| String | Citation |
|---|---|
| `Permission to use ${BashTool.name} with command ${command} has been denied.` | `bashPermissions.ts:1003,1086,1287,1316,1410,1447,2255` |
| `This command requires approval` | `bashPermissions.ts:1038,1167` |
| `Command contains malformed syntax that cannot be parsed: ${parseResult.error}` | `bashPermissions.ts:1819` |
| `Multiple directory changes in one command require approval for clarity` | `bashPermissions.ts:2188`, `bashCommandHelpers.ts:39` |
| `Compound commands with cd and git require approval to prevent bare repository attacks` | `bashPermissions.ts:2216`, `bashCommandHelpers.ts:73` |
| `Command splits into ${N} subcommands, too many to safety-check individually` | `bashPermissions.ts:2172` |
| `This command contains patterns that could pose security risks and requires approval` | `bashPermissions.ts:1229` |
| `Command contains patterns that require approval` | `bashPermissions.ts:2018,2025` |
| `This command uses shell operators that require approval for safety` | `bashCommandHelpers.ts:232` |
| `Allowed by prompt rule: "${matchedDescription}"` | `bashPermissions.ts:1583,1652` |
| `Denied by Bash prompt rule: "${matchedDescription}"` | `bashPermissions.ts:1924,1927` |
| `Required by Bash prompt rule: "${askResult.matchedDescription}"` | `bashPermissions.ts:1957` |
| `Read-only command is allowed` | `bashPermissions.ts:1160` |
| `Auto-allowed with sandbox (autoAllowBashIfSandboxedEnabled enabled)` | `bashPermissions.ts:1356` |
| `Permission denied for: ${segmentCommand}` | `bashCommandHelpers.ts:110` |
| `Failed to parse command` | `bashCommandHelpers.ts:194` |
| `Base command not found` | `modeValidation.ts:33` |
| `No mode-specific handling for '${baseCmd}' in ${mode} mode` | `modeValidation.ts:54` |
| `Bypass mode is handled in main permission flow` | `modeValidation.ts:80` |
| `DontAsk mode is handled in main permission flow` | `modeValidation.ts:88` |
| `No mode-specific validation required` | `modeValidation.ts:107` |
| `<error>Command was aborted before completion</error>` | `BashTool.tsx:604` |
| `Command exceeded the assistant-mode blocking budget (${ASSISTANT_BLOCKING_BUDGET_MS/1000}s) and was moved to the background with ID: ${backgroundTaskId}. It is still running — you will be notified when it completes. Output is being written to: ${outputPath}. In assistant mode, delegate long-running work to a subagent or use run_in_background to keep this conversation responsive.` | `BashTool.tsx:610` |
| `Command was manually backgrounded by user with ID: ${backgroundTaskId}. Output is being written to: ${outputPath}` | `BashTool.tsx:612` |
| `Command running in background with ID: ${backgroundTaskId}. Output is being written to: ${outputPath}` | `BashTool.tsx:614` |
| MONITOR_TOOL block message — `Blocked: ${sleepPattern}. Run blocking commands in the background with run_in_background: true — you'll get a completion notification when done. For streaming events (watching logs, polling APIs), use the Monitor tool. If you genuinely need a delay (rate limiting, deliberate pacing), keep it under 2 seconds.` | `BashTool.tsx:530` |
| `Shell cwd was reset to ${getOriginalCwd()}` | `utils.ts:168` |
| Sed not-found stderr — `sed: ${filePath}: No such file or directory\nExit code 1` | `BashTool.tsx:383` |

Destructive-command informational warnings (UI-only, no permission effect)
verbatim from `destructiveCommandWarning.ts:12-89`:

```
Note: may discard uncommitted changes
Note: may overwrite remote history
Note: may permanently delete untracked files
Note: may discard all working tree changes      (×2: checkout/restore)
Note: may permanently remove stashed changes
Note: may force-delete a branch
Note: may skip safety hooks
Note: may rewrite the last commit
Note: may recursively force-remove files
Note: may recursively remove files
Note: may force-remove files
Note: may drop or truncate database objects
Note: may delete all rows from a database table
Note: may delete Kubernetes resources
Note: may destroy Terraform infrastructure
```

### 6.6 Semantic exit-code interpretation (verbatim, `commandSemantics.ts:31-89`)

| Command | exit 0 | exit 1 | exit ≥2 |
|---|---|---|---|
| (default) | success | `Command failed with exit code ${code}` | as exit 1 |
| `grep` | matches found | `'No matches found'` (not error) | error |
| `rg` | matches found | `'No matches found'` (not error) | error |
| `find` | success | `'Some directories were inaccessible'` (not error) | error |
| `diff` | no differences | `'Files differ'` (not error) | error |
| `test` | true | `'Condition is false'` (not error) | error |
| `[` | true | `'Condition is false'` (not error) | error |

`heuristicallyExtractBaseCommand` takes the LAST segment of `splitCommand_DEPRECATED`
(pipeline tail determines exit code) and uses its first whitespace-delimited
token (`commandSemantics.ts:112-118`).

### 6.7 `validateInput` MONITOR_TOOL block (verbatim, `BashTool.tsx:524-538`)

```
if (feature('MONITOR_TOOL') && !isBackgroundTasksDisabled && !input.run_in_background) {
  const sleepPattern = detectBlockedSleepPattern(input.command)
  if (sleepPattern !== null) {
    return {
      result: false,
      message: `Blocked: ${sleepPattern}. Run blocking commands in the background with run_in_background: true — you'll get a completion notification when done. For streaming events (watching logs, polling APIs), use the Monitor tool. If you genuinely need a delay (rate limiting, deliberate pacing), keep it under 2 seconds.`,
      errorCode: 10
    }
  }
}
```

`detectBlockedSleepPattern` (`BashTool.tsx:322-337`) matches a leading
`sleep N` (integer ≥ 2 seconds, no float) and reports either
`standalone sleep ${N}` or `sleep ${N} followed by: ${rest}`.

---

## 7. Side Effects & I/O

### 7.1 Filesystem
- Reads/writes via `getFsImplementation()` for `_simulatedSedEdit`
  (`BashTool.tsx:360-419`): `readFile`, `writeTextContent`, `detectFileEncoding`,
  `detectLineEndings`, `getFileModificationTime`, `notifyVscodeFileUpdated`,
  `fileHistoryTrackEdit` (when `fileHistoryEnabled() && parentMessage`).
- Persisted output: `fsStat`, `fsTruncate`, `link` (with `copyFile` EXDEV
  fallback) into `getToolResultPath(taskId, false)` directory ensured by
  `ensureToolResultsDir()`.
- `resetCwdIfOutsideProject` → `setCwd(originalCwd)` when cwd has moved
  outside `pathInAllowedWorkingPath` and `shouldMaintainProjectWorkingDir` is
  not set; emits `tengu_bash_tool_reset_to_original_dir` event
  (`utils.ts:170-192`).

### 7.2 Process spawn / shell
- The actual command spawn is `Shell.exec(command, abortSignal, 'bash', {…})`
  from `src/utils/Shell.ts` (single shared persistent shell; cwd persists,
  shell state does not — as the prompt advertises).
- `LocalShellTask.spawnShellTask` for explicit and auto backgrounding.

### 7.3 Network
- `gitOperationTracking` issues no network. Side effects come from the user
  command itself (`gh pr create`, `git push`, `curl`).
- `linkSessionToPR` writes to local session storage (`utils/sessionStorage.ts`).

### 7.4 Environment variables consumed (BashTool-specific)

| Var | Effect |
|---|---|
| `USER_TYPE === 'ant'` | enables ANT_ONLY_SAFE_ENV_VARS, ANT_ONLY_COMMAND_ALLOWLIST, ANT classifier telemetry, undercover, ANT DANGEROUS_BASH_PATTERNS extension, `tengu_sandbox_disabled_commands` GrowthBook fetch, `prompt.ts` short git block |
| `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` | strips `run_in_background` from schema; suppresses prompt note; routes explicit-background to foreground |
| `CLAUDE_CODE_DISABLE_COMMAND_INJECTION_CHECK` | skips legacy `bashCommandIsSafeAsync` and post-AST safety checks |
| `CLAUDE_CODE_BASH_SANDBOX_SHOW_INDICATOR` | `userFacingName` returns `'SandboxedBash'` instead of `'Bash'` |
| `CLAUDE_CODE_UNDERCOVER` | force-on undercover mode (ant only) |
| `CLAUDE_CODE_SIMPLE` | suppresses ANT skills section in prompt |
| `BASH_DEFAULT_TIMEOUT_MS` / `BASH_MAX_TIMEOUT_MS` | run-time timeouts |
| `BASH_MAX_OUTPUT_LENGTH` | output truncation length |
| `CLAUDECODE` (downstream) | tools that emit `<claude-code-hint />` rely on this; BashTool strips the tag from stdout (`BashTool.tsx:780-784`) |

### 7.5 External binaries
`bash` (primary; via `Shell.exec`), `git`, `gh`, `glab`, `curl`, `wget`,
plus whatever the user invokes. The tree-sitter parser is **pure-TypeScript**
(`bashParser.ts` header) — no native module is loaded; the comments in
`parser.ts` referring to "WASM init" are stale.

### 7.6 Trust boundaries
- `_simulatedSedEdit` is set ONLY by `SedEditPermissionRequest` after user
  approval; stripping it from the model-facing schema is the security control
  (`BashTool.tsx:249-253`).
- The persisted-output file path is exposed to the model only via the
  `<persisted-output>` wrapper text built by
  `buildLargeToolResultMessage` (`BashTool.tsx:591-600`); UI consumers never
  see the wrapper.

---

## 8. Feature Flags & Variants

| Flag | OFF behavior | ON behavior |
|---|---|---|
| `TREE_SITTER_BASH` | `parseCommand` / `parseCommandRaw` short-circuit `null` → `astResult.kind === 'parse-unavailable'` → legacy shell-quote / `bashCommandIsSafeAsync` path always runs. | tree-sitter parses; `parseForSecurityFromAst` produces `simple`/`too-complex`; legacy path skipped. |
| `TREE_SITTER_BASH_SHADOW` | no shadow path. | `parseCommandRaw` runs even without `TREE_SITTER_BASH`; result is recorded to `tengu_tree_sitter_shadow` with availability/divergence/abort details and FORCED to `parse-unavailable` so legacy stays authoritative; killswitch `tengu_birch_trellis` can disable shadow at runtime. |
| `BASH_CLASSIFIER` | classifier ALLOW auto-approval doesn't fire (`awaitClassifierAutoApproval`/`executeAsyncClassifierCheck` never returns `'classifier'` decisionReason). `pendingClassifierCheck` is NOT attached to ask responses. | classifier results are honored; pending checks attached. |
| `TRANSCRIPT_CLASSIFIER` | mode === 'auto' still runs the classifier path. | mode === 'auto' SKIPS classifier in BashTool (auto-mode classifier handles permission decisions externally). |
| `MONITOR_TOOL` | bare `sleep N≥2` is allowed; prompt sleep subitems use the "must poll" wording. | `validateInput` blocks bare leading `sleep N≥2` (`errorCode 10`); prompt steers user to Monitor. |
| `KAIROS` | no assistant-mode auto-background. | `setTimeout(15000)` auto-backgrounds long-running blocking commands in main thread when `getKairosActive()`. |
| `COMMIT_ATTRIBUTION` | `attributionHooks` not registered; `getAttributionTexts()` returns empty strings; commit/PR examples drop the trailing `Co-Authored-By:` and `🤖 Generated with...` lines. | hooks register at `setup.ts:354-360` (deferred via `setImmediate`); examples emit attribution. |

`USER_TYPE === 'ant'` deltas (consolidated):
- ANT_ONLY_SAFE_ENV_VARS strip path (`bashPermissions.ts:174,250,329,591`).
- ANT_ONLY_COMMAND_ALLOWLIST extension to read-only validation (`readOnlyValidation.ts:1211`).
- `prompt.ts:49` — emits `getUndercoverInstructions()` when `isUndercover()`.
- `prompt.ts:56` — short git block + skills section instead of full external block.
- `shouldUseSandbox.ts:23` — fetches `tengu_sandbox_disabled_commands` GrowthBook config.
- `bashPermissions.ts:123-144` — `logClassifierResultForAnts` fires ANT-only `tengu_internal_bash_classifier_result` events with the raw command (allowed only because ANT-only).
- `dangerousPatterns.ts:58-79` — adds `fa run`, `coo`, `gh`, `gh api`, `curl`, `wget`, `git`, `kubectl`, `aws`, `gcloud`, `gsutil` to dangerous prefixes.

---

## 9. Error Handling & Edge Cases

- `result.preSpawnError` → re-thrown as `Error(message)` before the handler
  records analytics (`BashTool.tsx:711-713`).
- `interpretation.isError && !isInterrupt` → `throw new ShellError('', output,
  result.code, result.interrupted)` so the upstream pipeline maps it to a
  tool-result with `is_error:true`.
- `interrupted && abortSignal.reason === 'interrupt'`: append
  `<error>Command was aborted before completion</error>` to stderr block.
- AST parse abort distinguishes between "module not loaded" (`null`) and
  "loaded but timeout/panic" (`PARSE_ABORTED`) — the latter MUST fall through
  to `'too-complex'` rather than legacy (`parser.ts:90-95`, comment
  references EVAL_LIKE_BUILTINS like `trap`, `enable`, `hash` which legacy
  misses).
- Subcommand explosion: `>50` legacy-path subcommands → `'ask'` with reason
  string; AST path is bounded by tree-sitter caps.
- Compound `cd` x N: 2+ `cd` subcommands → ask. Compound `cd` + `git` → ask
  (bare-repo `core.fsmonitor` RCE class).
- Sandbox auto-allow gate respects deny/ask rules including subcommand-level
  prefix denies (`checkSandboxAutoAllow`).
- AbortError handling: classifier `Promise.all` aborts cleanly, raising
  `APIUserAbortError` or `AbortError` swallowed in
  `executeAsyncClassifierCheck` (`bashPermissions.ts:1626-1633`).
- `_simulatedSedEdit` ENOENT → returns success-shaped `Out` with stderr
  `sed: ${filePath}: No such file or directory\nExit code 1` (no throw).

Spec 09 governs the global decision tree shape; this spec only documents the
BashTool entry-points into it.

---

## 10. Telemetry & Observability

`logEvent(...)` calls owned by BashTool path (non-exhaustive):

| Event | Site | Payload |
|---|---|---|
| `tengu_bash_tool_command_executed` | `BashTool.tsx:755` | `command_type`, `stdout_length`, `stderr_length` (=0; merged fd), `exit_code`, `interrupted` |
| `tengu_code_indexing_tool_used` | `BashTool.tsx:766` | `tool`, `source`='cli', `success` |
| `tengu_git_index_lock_error` | `BashTool.tsx:694` | `{}` |
| `tengu_bash_tool_reset_to_original_dir` | `utils.ts:187` | `{}` |
| `tengu_bash_command_explicitly_backgrounded` | `BashTool.tsx:991` | `command_type` |
| `tengu_bash_command_timeout_backgrounded` | `BashTool.tsx:969` (via `startBackgrounding`) | `command_type` |
| `tengu_bash_command_assistant_auto_backgrounded` | `BashTool.tsx:980` | `command_type` |
| `tengu_bash_ast_too_complex` | `bashPermissions.ts:1752` | `nodeTypeId` |
| `tengu_tree_sitter_shadow` | `bashPermissions.ts:1727` | `available, astTooComplex, astSemanticFail, subsDiffer, injectionCheckDisabled, killswitchOff, cmdOverLength` |
| `tengu_tree_sitter_load` | `parser.ts:43` | `success` |
| `tengu_tree_sitter_parse_abort` | `parser.ts:120,128` | `cmdLength, panic` |
| `tengu_tree_sitter_security_divergence` | `bashPermissions.ts:2362` | `quoteContextDivergence, count` |
| `tengu_internal_bash_classifier_result` (ANT-only) | `bashPermissions.ts:127` | `behavior, descriptions, matches, matchedDescription, confidence, reason, command` |
| `tengu_git_operation` | `gitOperationTracking.ts:200,205,212,219,251,271` | `operation` ∈ `commit\|commit_amend\|push\|pr_create\|pr_edit\|pr_merge\|pr_comment\|pr_close\|pr_ready` |

OTLP counters: `getCommitCounter()?.add(1)` and `getPrCounter()?.add(1)`
from `bootstrap/state.ts`. Session→PR linking via dynamic
`utils/sessionStorage.linkSessionToPR(...)`.

`logForDebugging` traces (not analytics): tree-sitter unavailable
(`bashPermissions.ts:1812`), subcommand cap hit (`:2166`).

---

## 11. Reimplementation Checklist

A reimplementer is "done" when these invariants hold against this codebase:

1. **Schema strictness**: `_simulatedSedEdit` is **never** present in the
   model-facing schema; `run_in_background` is dropped iff
   `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is truthy at module load.
2. **Prompt cache**: prompt text is identical bit-for-bit across users when
   sandbox is enabled — `$TMPDIR` substitution preserves cache; ANT branches
   appear ONLY when `USER_TYPE==='ant'`; undercover instructions ONLY when
   `isUndercover()` true.
3. **Permission decision order** (single subcommand): exact-deny → exact-ask
   → prefix-deny → prefix-ask → checkPathConstraints → exact-allow →
   prefix-allow → checkSedConstraints → checkPermissionMode →
   `BashTool.isReadOnly` → passthrough.
4. **Permission decision order** (top-level `bashToolHasPermission`): AST
   parse → too-complex/simple/parse-unavailable branches → sandbox auto-allow
   → exact-match deny → classifier deny+ask (parallel) → operator/pipe
   handling → legacy misparsing gate → subcommand split + cd-cwd filter →
   subcommand cap (50) → cd guards → per-subcommand deny → path constraints
   → per-subcommand ask → exact allow short-circuit → injection re-check →
   "all allow + no injection" → single-sub fast path → multi-sub merge.
5. **Env-var stripping symmetry**: SAFE_ENV_VARS for allow rules; ALL leading
   env vars (with broader value pattern) for deny/ask rules (defense
   asymmetry — HackerOne #3543050 + #21503).
6. **Wrapper stripping symmetry**: `stripSafeWrappers` (string) and
   `stripWrappersFromArgv` (argv) **must** stay in sync; SAFE_WRAPPER_PATTERNS
   covers `timeout`, `time`, `nice`, `stdbuf`, `nohup` (all forms enumerated).
7. **Compound-command rule guard**: prefix and wildcard rules MUST NOT match
   compound commands in 'prefix' mode (security: `cd .. && rm` must not match
   `Bash(cd:*)`).
8. **MAX_SUBCOMMANDS_FOR_SECURITY_CHECK = 50** legacy cap; ask above.
   **MAX_SUGGESTED_RULES_FOR_COMPOUND = 5** suggestion cap.
9. **Classifier shape**: BASH_CLASSIFIER is the **gating flag** for both
   auto-approval AND `pendingClassifierCheck` attachment; isClassifierPermissionsEnabled()
   gates the deny+ask classifier dispatch; auto-mode + TRANSCRIPT_CLASSIFIER
   skips classifier path entirely.
10. **PARSE_ABORTED propagation**: tree-sitter timeout/panic must be treated
    as `'too-complex'` (fail-closed), not as legacy fallback.
11. **Output truncation suffix**: `\n\n... [${remainingLines} lines truncated] ...`.
12. **Persisted-output**: hard-link first, copyFile fallback, truncate to 64
    MiB before linking; surface as `<persisted-output>` in tool_result text
    via `buildLargeToolResultMessage`.
13. **Shell reset**: when cwd has drifted outside `pathInAllowedWorkingPath`,
    reset to `originalCwd` and append `Shell cwd was reset to ${originalCwd}`
    to stderr.
14. **Background task lifecycle**: `run_in_background:true` returns
    `{stdout:'',stderr:'',code:0,interrupted:false,backgroundTaskId}` immediately;
    auto-background re-uses an existing foreground registration via
    `backgroundExistingForegroundTask` (no re-spawn).
15. **Git tracking**: every successful `BashTool.call` runs
    `trackGitOperations(command, code, stdout)`; `linkSessionToPR` fires only
    on `gh pr create` success with parseable PR URL.
16. **ANT-only paths** are bundle-eliminated in external builds via the
    `process.env.USER_TYPE === 'ant'` constant-fold; no ANT_ONLY_* string
    appears in external bundles.
17. **MONITOR_TOOL block** (`validateInput`) returns `errorCode: 10` and the
    verbatim message above; only fires when `!isBackgroundTasksDisabled` and
    `!input.run_in_background`.

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. RESOLVED items cite where the answer landed.

1. **`bashSecurity.ts` deep dive.** [DEFERRED — sampling-only, no defect]
   The 2592-line `bashCommandIsSafeAsync_DEPRECATED` battery (verbatim regex
   set, `stripSafeHeredocSubstitutions`, divergence detection between
   tree-sitter and shell-quote) was sampled, not fully enumerated. The
   "DANGEROUS_BASH_PATTERNS" requested by the dispatch prompt actually lives
   in `src/utils/permissions/dangerousPatterns.ts` (verbatim above). A future
   revise pass should inline the `bashSecurity.ts` regex array verbatim
   (likely 30-50 patterns). Recorded; not blocking.
2. **`pathValidation.ts` deep dive.** [DEFERRED — sampling-only, no defect]
   1303-line file owned shared with FileTools (spec 11). The cd+redirect-target
   gate, output redirection extraction, and `PATH_EXTRACTORS` /
   `COMMAND_OPERATION_TYPE` maps not enumerated here. Spec 11 §5 covers the
   FileTool consumer side; future sweep can inline the BashTool-relevant
   entry contract.
3. **`sedValidation.ts` allowlist.** [DEFERRED — sampling-only] 684-line file;
   exact regex set not enumerated. Future revise pass.
4. ~~**Tree-sitter native module**~~ — **RESOLVED Phase 9.7**: spec 24 (LSP)
   audit confirms no NAPI bash-parser exists in the leaked tree. `bashParser.ts`
   is pure-TypeScript with the documented 50ms / 50K-node budget; the
   stale comments in `parser.ts` reference an earlier architecture that
   was DCE'd. The pure-TS path is the only live path.
5. ~~**Setup-time COMMIT_ATTRIBUTION hook scope**~~ — **RESOLVED Phase 9.7**:
   spec 01 (entrypoint-bootstrap) §6 documents `registerAttributionHooks()`
   in `utils/attributionHooks.ts`; the hook registers a PostToolUse handler
   that intercepts BashTool `git commit` invocations and rewrites the
   `--author` flag per `isUndercover()` policy.
6. ~~**`sandbox.excludedCommands` settings type / default**~~ — **RESOLVED
   Phase 9.7**: spec 02 (settings) §4 documents this as `string[]` (default
   `[]`) under the `sandbox` settings key; matching is exact-string against
   the parsed command name (no regex/glob).
7. ~~**Shadow-mode killswitch `tengu_birch_trellis`**~~ — **DEFERRED**:
   GrowthBook feature default is `true` per `bashPermissions.ts:1684` source
   default; the production-config value is server-side (spec 26 §12, spec
   00 §13 known-unfalsifiable). Source-default behavior fully documented.
8. ~~**File-history integration `fileHistoryEnabled()`**~~ — **RESOLVED
   Phase 9.7**: spec 41 (session-state-history) §3 documents
   `fileHistoryEnabled()` returns true when `settings.fileHistory !== false`
   (default opt-in). Consumer-side gating in `_simulatedSedEdit` is correct
   as documented.
9. ~~**`isResultTruncated` truncation marker**~~ — **NOTE Phase 9.7**:
   spec 19 / 42a confirm the suffix from §6.4 is the canonical marker;
   `isOutputLineTruncated` in `utils/terminal.ts` is a string-suffix match
   against the same constant. No discrepancy found in cross-check.
