# 19 — Miscellaneous Tools (catch-all)

> **Scope.** This spec is the **catch-all** for every tool registered in
> `src/tools.ts` that is NOT covered by 10 (Bash), 11 (files), 12 (search),
> 13 (web), 14 (agent/team), 15 (tasks), 16 (mcp/lsp), 17 (skill), or
> 18 (modes/plan/worktree). Tools whose source is **missing from the leaked
> tree** are documented at registry-citation level only and listed in §12.
>
> **Out of scope (refer by spec).** Permission engine -> 09. Tool base /
> registry assembly -> 08. Slash commands -> 21. Proactive idle behavior
> -> 31. Kairos mode -> 32. Bridge -> 34. Remote/UDS inbox -> 35. Voice
> -> 36.

---

## §0. Source-coverage inventory

| Tool | Registry citation | Gate | Source present? |
|---|---|---|---|
| `SleepTool` | `tools.ts:25-28` | `feature('PROACTIVE') \|\| feature('KAIROS')` | partial - only `prompt.ts` |
| `SyntheticOutputTool` | imported `tools.ts:97`; not in `getAllBaseTools()` | runtime: `isSyntheticOutputToolEnabled({isNonInteractiveSession})` | yes |
| `BriefTool` | `tools.ts:13`, included `tools.ts:238` | `feature('KAIROS') \|\| feature('KAIROS_BRIEF')` + opt-in | yes |
| `AskUserQuestionTool` | `tools.ts:73`, included `tools.ts:211` | always-on, disabled when `--channels` active | yes |
| `ConfigTool` | `tools.ts:81`, included `tools.ts:214` | `process.env.USER_TYPE === 'ant'` | yes |
| `REPLTool` | `tools.ts:16-19`, included `tools.ts:232` | `process.env.USER_TYPE === 'ant'` (top-level conditional `require`) | partial - `constants.ts` + `primitiveTools.ts` only |
| `PowerShellTool` | lazy require `tools.ts:150-155`, included `tools.ts:242` | `isPowerShellToolEnabled()` (Windows + env) | yes |
| `RemoteTriggerTool` | `tools.ts:36-38`, included `tools.ts:236` | `feature('AGENT_TRIGGERS_REMOTE')` + GB `tengu_surreal_dali` + policy | yes |
| `TestingPermissionTool` | `tools.ts:58`, included `tools.ts:244` | `process.env.NODE_ENV === 'test'` | yes |
| `MonitorTool` | `tools.ts:39-41`, included `tools.ts:237` | `feature('MONITOR_TOOL')` | **missing** |
| `WorkflowTool` (incl. `bundled/`) | `tools.ts:129-134`, `commands.ts:401-405`, included `tools.ts:233` | `feature('WORKFLOW_SCRIPTS')` | **missing** |
| `WebBrowserTool` | `tools.ts:117-119`, included `tools.ts:217` | `feature('WEB_BROWSER_TOOL')` | **missing** |
| `SnipTool` | `tools.ts:123-125`, included `tools.ts:243` | `feature('HISTORY_SNIP')` | **missing** |
| `TungstenTool` | `tools.ts:60`, included `tools.ts:215` | `process.env.USER_TYPE === 'ant'` (ANT-only) | **missing** |
| `VerifyPlanExecutionTool` | `tools.ts:91-95`, included `tools.ts:231` | `process.env.CLAUDE_CODE_VERIFY_PLAN === 'true'` | **missing** |
| `PushNotificationTool` | `tools.ts:45-49`, included `tools.ts:240` | `feature('KAIROS') \|\| feature('KAIROS_PUSH_NOTIFICATION')` | **missing** |
| `SendUserFileTool` | `tools.ts:42-44`, included `tools.ts:239` | `feature('KAIROS')` | **missing** |
| `SubscribePRTool` | `tools.ts:50-52`, included `tools.ts:241` | `feature('KAIROS_GITHUB_WEBHOOKS')` | **missing** |
| `SuggestBackgroundPRTool` | `tools.ts:20-24`, included `tools.ts:216` | ANT-only top-level conditional `require` | **missing** |
| `OverflowTestTool` | `tools.ts:107-109`, included `tools.ts:221` | `feature('OVERFLOW_TEST_TOOL')` | **missing** |
| `CtxInspectTool` | `tools.ts:110-112`, included `tools.ts:222` | `feature('CONTEXT_COLLAPSE')` | **missing** |
| `TerminalCaptureTool` | `tools.ts:113-116`, included `tools.ts:223` | `feature('TERMINAL_PANEL')` | **missing** |
| `ListPeersTool` | `tools.ts:126-128`, included `tools.ts:227` | `feature('UDS_INBOX')` | **missing** |
| `ScheduleCronTool / CronCreateTool / CronDeleteTool / CronListTool` | `tools.ts:29-35`, spread `tools.ts:235` | Build-time: `feature('AGENT_TRIGGERS')` (DCE); Runtime: `isKairosCronEnabled()` per-tool `isEnabled()` (`CronCreateTool.ts:67-69`, `CronDeleteTool.ts:46-48`, `CronListTool.ts:48-50`) | yes |
<!-- Naming note: `ScheduleCronTool` is the *directory* name (`src/tools/ScheduleCronTool/`); the three exported tool symbols are `CronCreateTool`, `CronDeleteTool`, `CronListTool`. There is no symbol named `ScheduleCronTool`. References in other specs that say "ScheduleCronTool" mean the directory / 3-tool group. -->
<!-- Two-layer gate (mirrors §3.3 BriefTool): the `feature('AGENT_TRIGGERS')` ternary at `tools.ts:29-35` is build-time DCE only. Each tool's runtime `isEnabled()` calls `isKairosCronEnabled()` from `prompt.ts`; `isDurableCronEnabled()` is a separate, narrower predicate that controls whether `durable:true` persists across sessions (see §3.10 / §13.3). -->
| `ReviewArtifactTool` | `components/permissions/PermissionRequest.tsx:36` (NOT in `tools.ts`) | `feature('REVIEW_ARTIFACT')` | **missing** |

> Bit-exact rule: this spec preserves verbatim prompts and Zod schemas only
> for tools whose source is in the leak. Missing-source tools are
> documented to registry-citation depth - no invented prompts, schemas, or
> behavior.

---

## §1. Purpose

The miscellaneous bucket gathers every built-in tool that does not fit a
larger functional theme. It has three sub-purposes:

1. **Idle / pacing primitives** - `SleepTool` lets the model wait without
   holding a shell; relied on by Proactive (31) and Kairos (32) modes
   (`tools.ts:25-28`).
2. **Out-of-band user channels** - `BriefTool` (Kairos's user-facing
   message channel) and `AskUserQuestionTool` (multiple-choice clarifier)
   move information between model and human outside the normal
   assistant text stream (`tools.ts:13`, `:73`; `:211`, `:238`).
3. **Internal / meta tools** - `ConfigTool` mutates settings via tool
   call (ANT-only), `REPLTool` wraps a JS VM that the model uses to
   script built-in primitives (ANT-only), `PowerShellTool` is the
   Windows shell peer of `BashTool`, `RemoteTriggerTool` calls the
   claude.ai remote-agents API, `SyntheticOutputTool` enforces
   structured output for non-interactive runs, and
   `TestingPermissionTool` is a test-only stub.

In addition this spec records **15 missing-source registry entries**
(see §0 inventory and §12) so that the registry-derived feature flag
matrix from `00-overview.md` §8 is fully accounted for in the spec set.
The Cron triplet (`CronCreateTool` / `CronDeleteTool` / `CronListTool`) is
**not** missing — full source is in the leak (`src/tools/ScheduleCronTool/`),
documented in §3.10 / §6.10 / §13.3 below; this aligns with spec 00 §2.5
post-Phase-9.6 B-mini.

---

## §2. Inputs and outputs

For each PRESENT tool, §6 carries the verbatim Zod input/output schema and
prompt. For each MISSING tool, only the registry name and gate are known -
no input/output surface can be reconstructed without invention.

Cross-tool inputs:

- **`SleepTool`** - schema and prompt assets are missing (the leak ships
  only `tools/SleepTool/prompt.ts`). The exported prompt is verbatim in §6.
  The tool is included via a lazy `require('./tools/SleepTool/SleepTool.js')`
  at `tools.ts:27` and conditionally added at `tools.ts:234`.
- **`SyntheticOutputTool`** - schema is `z.object({}).passthrough()`
  augmented at runtime by an Ajv-compiled JSON Schema supplied by the
  caller via `createSyntheticOutputTool(jsonSchema)`
  (`SyntheticOutputTool.ts:11,116-163`). Output is the original input
  echoed under `structured_output`.
- **`BriefTool`** - strict object: `{message, attachments?, status}`.
  Output: `{message, attachments?, sentAt?}`. Both schemas live in
  `BriefTool.ts:20-65` (verbatim in §6).
- **`AskUserQuestionTool`** - strict object: `{questions[1..4],
  answers?, annotations?, metadata?}` plus a `.refine` for
  question-text/option-label uniqueness (`AskUserQuestionTool.tsx:14-67`).
- **`ConfigTool`** - strict object `{setting, value?}` (verbatim in §6).
- **`PowerShellTool`** - strict object `{command, timeout?,
  description?, run_in_background?, dangerouslyDisableSandbox?}`;
  `run_in_background` is omitted when
  `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is truthy
  (`PowerShellTool.tsx:228-239`).
- **`RemoteTriggerTool`** - strict object `{action: 'list'|'get'|
  'create'|'update'|'run', trigger_id?, body?}` (`RemoteTriggerTool.ts:18-31`).
- **`TestingPermissionTool`** - empty strict object
  (`testing/TestingPermissionTool.tsx:10`).
- **`REPLTool`** - schema not present in leak; `constants.ts` exports
  the name (`'REPL'`) and the `REPL_ONLY_TOOLS` set hidden when REPL
  mode is active.

---

## §3. Public interface (per-tool)

### 3.1 SleepTool - `'Sleep'`

- **Citation.** `tools.ts:25-28` (gate), `tools.ts:234` (inclusion).
- **Gate.** `feature('PROACTIVE') || feature('KAIROS')`. Bun DCE strips
  the conditional require when neither is set.
- **Asset coverage.** Only `tools/SleepTool/prompt.ts` exists in the leak.
  `SleepTool.ts` is missing - the tool object, input schema, and `call()`
  body cannot be reproduced.
- **Known surface.** `SLEEP_TOOL_NAME = 'Sleep'`; `DESCRIPTION = 'Wait
  for a specified duration'`; `SLEEP_TOOL_PROMPT` (verbatim in §6)
  references `<${TICK_TAG}>` periodic check-in prompts and explicitly
  tells the model to prefer this tool over `Bash(sleep ...)`.
- **Adjacent.** Spec 31 (proactive idle ticks); spec 32 (Kairos uses
  sleep to wait between brief cycles).

### 3.2 SyntheticOutputTool - `'StructuredOutput'`

- **Citation.** Imported `tools.ts:97`, name added to `specialTools` set
  `tools.ts:301-305` (filtered out of `getTools()` because it is created
  per-request rather than registered).
- **Gate.** `isSyntheticOutputToolEnabled({isNonInteractiveSession})`
  (`SyntheticOutputTool.ts:22-26`) - true iff non-interactive.
- **Behavior.** `createSyntheticOutputTool(jsonSchema)` returns either
  `{tool}` or `{error}`. On success a fresh tool clone is built whose
  `inputJSONSchema` is the supplied JSON Schema and whose `call()` runs
  an Ajv-compiled validator against the input; mismatches throw a
  telemetry-safe error. A `WeakMap<object, CreateResult>` caches results
  keyed by `jsonSchema` identity (`:109-125`). The static
  `SyntheticOutputTool` exists for typing - the per-request tool is what
  reaches the model.
- **Permission.** `checkPermissions` always returns `{behavior:'allow'}`
  (`:66-72`).
- **Adjacent.** Spec 03 (structured-output enforcement during query loop).

### 3.3 BriefTool - `'SendUserMessage'` (legacy alias `'Brief'`)

- **Citation.** Static import `tools.ts:13`; included unconditionally at
  `tools.ts:238` but `isEnabled()` is the real gate.
- **Gates.**
  - **Build-time entitlement**: `feature('KAIROS') ||
    feature('KAIROS_BRIEF')` (`BriefTool.ts:88-100`). Composite top-level
    guard is load-bearing for Bun DCE - in external builds the ternary
    constant-folds to `false` and the BriefTool object is dead-coded.
  - **Runtime entitlement**: `getKairosActive() ||
    isEnvTruthy(process.env.CLAUDE_CODE_BRIEF) ||
    getFeatureValue_CACHED_WITH_REFRESH('tengu_kairos_brief', false,
    KAIROS_BRIEF_REFRESH_MS)` where `KAIROS_BRIEF_REFRESH_MS = 5 * 60 *
    1000` (`:67`, `:88-100`).
  - **Activation**: `(getKairosActive() || getUserMsgOptIn()) &&
    isBriefEntitled()` (`:126-134`). Opt-in is set by `--brief`,
    `defaultView: 'chat'`, `/brief` command, `/config defaultView`
    picker, `SendUserMessage` in `--tools` / SDK `tools` option, or
    `CLAUDE_CODE_BRIEF` env (see :103-114 commentary).
- **Surface.** `aliases: [LEGACY_BRIEF_TOOL_NAME]` (`'Brief'`),
  `searchHint = 'send a message to the user - your primary visible
  output channel'`, `maxResultSizeChars: 100_000`, `userFacingName:
  () => ''`, `isReadOnly: () => true`, `isConcurrencySafe: () => true`,
  `toAutoClassifierInput: input => input.message`.
- **`validateInput`.** When `attachments` is present and non-empty,
  calls `validateAttachmentPaths(attachments)` from `./attachments.js`;
  otherwise short-circuits `{result: true}` (`:163-168`).
- **`call()`.** Always logs `tengu_brief_send` with `{proactive: status
  === 'proactive', attachment_count}`. Captures `sentAt = new
  Date().toISOString()`. With no attachments, returns `{message,
  sentAt}`. Otherwise calls `resolveAttachments(attachments,
  {replBridgeEnabled: context.getAppState().replBridgeEnabled, signal:
  context.abortController.signal})` and returns `{message, attachments:
  resolved, sentAt}` (`:186-203`).
- **Result block.** `'Message delivered to user.'` plus suffix
  `' (n attachments included)'` when `n > 0` (`:175-183`).
- **Prompt.** See §6.3. The `BRIEF_PROACTIVE_SECTION` constant is the
  system-prompt fragment used by Kairos - referenced from spec 32.
- **Adjacent.** Spec 32 (Kairos system prompt mandates
  `SendUserMessage`), spec 21 (`/brief` command activates opt-in).

### 3.4 AskUserQuestionTool - `'AskUserQuestion'`

- **Citation.** Static import `tools.ts:73`; included unconditionally at
  `tools.ts:211`. Real gate is `isEnabled()`.
- **Surface.** `searchHint = 'prompt the user with a multiple-choice
  question'`, `maxResultSizeChars: 100_000`, `shouldDefer: true`,
  `userFacingName: () => ''`, `isConcurrencySafe: () => true`,
  `isReadOnly: () => true`, `requiresUserInteraction: () => true`,
  `toAutoClassifierInput: input => input.questions.map(q =>
  q.question).join(' | ')` (`AskUserQuestionTool.tsx:109-153`).
- **Disable rule.** Returns `false` when `(feature('KAIROS') ||
  feature('KAIROS_CHANNELS')) && getAllowedChannels().length > 0`
  (`:135-145`) - the multiple-choice dialog hangs when the user is on a
  remote channel.
- **Permission.** Always `{behavior:'ask', message:'Answer questions?',
  updatedInput: input}` (`:182-188`).
- **Prompt selection.** `prompt()` reads
  `getQuestionPreviewFormat()`. If `undefined`, returns the bare
  `ASK_USER_QUESTION_TOOL_PROMPT` (no preview guidance for SDK
  consumers who haven't opted in). Otherwise concatenates
  `PREVIEW_FEATURE_PROMPT[format]` where `format in {'markdown', 'html'}`
  (`:117-125`).
- **`validateInput`.** When `getQuestionPreviewFormat() === 'html'`,
  every option's `preview` must pass `validateHtmlPreview` - failures
  return `{result:false, message:'Option "{label}" in question
  "{question}": {err}', errorCode:1}` (`:158-181`).
- **Schema constraints.** 1-4 questions, 2-4 options each; multiSelect
  default false; `header` chip max length 12 chars
  (`ASK_USER_QUESTION_TOOL_CHIP_WIDTH = 12`); `.refine` enforces
  question-text uniqueness across the request and option-label
  uniqueness within each question (`:14-67`).
- **`call()`.** Pure passthrough - returns `{questions, answers,
  ...(annotations && {annotations})}`; the actual UI dialog is wired
  through `shouldDefer: true` + the permission engine (see 09).
- **Result block.** Concatenates `"{question}"="{answer}"` per Q with
  optional `selected preview:\n{preview}` and `user notes: {notes}`
  suffixes; final string prefixed by `'User has answered your
  questions: '` and suffixed by `". You can now continue with the
  user's answers in mind."` (`:224-244`).

### 3.5 ConfigTool - `'Config'` (ANT-only)

- **Citation.** Static import `tools.ts:81`; included only when
  `process.env.USER_TYPE === 'ant'` at `tools.ts:214`. ANT-only
  inclusion preserved by the `biome-ignore-all` import-order header at
  `tools.ts:1`.
- **Surface.** `searchHint = 'get or set Claude Code settings (theme,
  model)'`, `maxResultSizeChars: 100_000`, `userFacingName: () =>
  'Config'`, `shouldDefer: true`, `isConcurrencySafe: () => true`,
  `isReadOnly: input => input.value === undefined`,
  `toAutoClassifierInput: input => input.value === undefined ?
  input.setting : "${setting} = ${value}"` (`ConfigTool.ts:67-97`).
- **Permission.** `value === undefined` -> `{behavior:'allow'}`;
  otherwise `{behavior:'ask', message:'Set ${setting} to
  ${jsonStringify(value)}'}` (`:98-107`).
- **Read path.** `getValue(config.source, path)` walks settings via path
  array; if `formatOnRead` is set the value is transformed for display
  (`:436-453`, `:135-144`).
- **Write path.** Stepwise:
  1. Voice-mode runtime kill-switch - when `feature('VOICE_MODE') &&
     setting === 'voiceEnabled'` and `!isVoiceGrowthBookEnabled()`,
     return `{success:false, error:'Unknown setting: "voiceEnabled"'}`
     (`:116-125`).
  2. `isSupported(setting)` check (`:126-130`).
  3. **Reset**: `setting === 'remoteControlAtStartup'` and value is the
     literal string `'default'` (case-insensitive, trimmed) - deletes
     the key from global config, recomputes via
     `getRemoteControlAtStartup()`, syncs `replBridgeEnabled` in
     AppState (`:148-180`).
  4. **Boolean coercion**: `'true'`/`'false'` -> boolean; else error
     `'${setting} requires true or false.'` (`:185-201`).
  5. **Options check**: `'Invalid value "${value}". Options:
     ${options.join(", ")}'` (`:203-214`).
  6. **Async validate**: `validateOnWrite` (used by `model`) - error
     surfaced verbatim (`:217-229`).
  7. **Voice pre-flight** (only when `feature('VOICE_MODE') && setting
     === 'voiceEnabled' && finalValue === true`): runs
     `isVoiceModeEnabled` / `isAnthropicAuthEnabled` /
     `isVoiceStreamAvailable` / `checkRecordingAvailability` /
     `checkVoiceDependencies` / `requestMicrophonePermission` and
     emits platform-specific guidance text
     (`'Settings -> Privacy -> Microphone'` on win32, `"your system's
     audio settings"` on linux, otherwise `'System Settings -> Privacy
     & Security -> Microphone'`) (`:231-308`).
  8. **Storage write**: `source === 'global'` -> `saveGlobalConfig`;
     else `updateSettingsForSource('userSettings',
     buildNestedObject(path, finalValue))` (`:312-343`).
  9. **AppState sync**: `notifyChange('userSettings')` for voice
     (`:347-353`); generic `appStateKey` setter (`:355-362`); special
     `remoteControlAtStartup` -> re-derive + push to
     `replBridgeEnabled` (`:364-381`).
  10. Logs `tengu_config_tool_changed` with stringified `setting` and
     `value` (`:383-389`).
- **Result block.** GET -> `'${setting} = ${jsonStringify(value)}'`. SET
  -> `'Set ${setting} to ${jsonStringify(newValue)}'`. Failure ->
  `'Error: ${error}'` with `is_error: true` (`:412-433`).
- **Prompt.** `generatePrompt()` builds a Markdown listing of every
  registered setting, partitioned into "Global Settings (stored in
  ~/.claude.json)" and "Project Settings (stored in settings.json)",
  followed by a dynamic "## Model" section. `model`/`voiceEnabled` are
  excluded from the listing under specific conditions (see §6.5).
- **Supported-settings map.** See §6.6 for the verbatim
  `SUPPORTED_SETTINGS` registry. The `process.env.USER_TYPE === 'ant'`
  spread at `supportedSettings.ts:134-143` adds
  `classifierPermissionsEnabled`. The `feature('AUTO_THEME')` ternary
  at `:34` swaps `THEME_NAMES` for `THEME_SETTINGS` in the `theme`
  options list.

### 3.6 REPLTool - `'REPL'` (ANT-only)

- **Citation.** Top-level conditional require `tools.ts:16-19`
  (`process.env.USER_TYPE === 'ant'`); included `tools.ts:232` only when
  also ant. The implementation file `tools/REPLTool/REPLTool.ts` is
  **missing** from the leak; only `constants.ts` and `primitiveTools.ts`
  are present.
- **Mode gate.** `isReplModeEnabled()` returns:
  - `false` if `CLAUDE_CODE_REPL` is defined-falsy.
  - `true` if `CLAUDE_REPL_MODE` is truthy.
  - else `process.env.USER_TYPE === 'ant' &&
    process.env.CLAUDE_CODE_ENTRYPOINT === 'cli'` (`constants.ts:23-30`).
- **Hidden primitives.** When REPL mode is on, `getTools()` filters out
  every name in `REPL_ONLY_TOOLS = { FileRead, FileWrite, FileEdit,
  Glob, Grep, Bash, NotebookEdit, Agent }` after confirming REPL is in
  the allowed list (`tools.ts:312-323`, `constants.ts:37-46`). The same
  set is exposed via `getReplPrimitiveTools()` for display-side
  classifiers in `primitiveTools.ts:11-39` (lazy getter to avoid the TDZ
  caused by the cycle `collapseReadSearch.ts -> primitiveTools.ts ->
  FileReadTool -> tool registry`).
- **Simple-mode interaction.** Under `CLAUDE_CODE_SIMPLE` truthy:
  - if REPL mode is on AND `REPLTool` is non-null, the tool list
    becomes `[REPLTool]` (with `TaskStopTool, getSendMessageTool()`
    appended when `feature('COORDINATOR_MODE') &&
    coordinatorModeModule?.isCoordinatorMode()`) (`tools.ts:277-286`).
  - otherwise simple mode falls back to `[BashTool, FileReadTool,
    FileEditTool]` (`:287-298`).
- **Surface beyond constants.** Unknown - see §12.

### 3.7 PowerShellTool - `'PowerShell'`

- **Citation.** Lazy require via `getPowerShellTool()` at
  `tools.ts:150-155`; included at `tools.ts:242`. Lazy because the
  require triggers Windows-only path-permission code paths.
- **Runtime gate.** `isPowerShellToolEnabled()` returns `false` unless
  `getPlatform() === 'windows'`. On Windows: ant defaults on
  (`!isEnvDefinedFalsy(CLAUDE_CODE_USE_POWERSHELL_TOOL)`); external
  defaults off (`isEnvTruthy(CLAUDE_CODE_USE_POWERSHELL_TOOL)`)
  (`shellToolUtils.ts:17-22`).
- **Surface.** `name = POWERSHELL_TOOL_NAME = 'PowerShell'`
  (`toolName.ts:2`), `searchHint = 'execute Windows PowerShell
  commands'`, `maxResultSizeChars: 30_000`, `strict: true`, `isReadOnly`
  calls `isReadOnlyCommand(input.command)` (sync, no AST),
  `isConcurrencySafe` mirrors `isReadOnly` - no parallel write
  (`PowerShellTool.tsx:272-316`).
- **Schema branch.** When
  `process.env.CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is truthy at module
  load, the input schema is `fullInputSchema().omit({run_in_background:
  true})`; otherwise the full schema is used (`:225-240`).
- **Sandbox refusal.** `isWindowsSandboxPolicyViolation()` checks
  `getPlatform() === 'windows' &&
  SandboxManager.isSandboxEnabledInSettings() &&
  !SandboxManager.areUnsandboxedCommandsAllowed()` and refuses with
  `WINDOWS_SANDBOX_POLICY_REFUSAL` (verbatim in §6.7). Checked in both
  `validateInput` and `call()` (`:218-222`).
- **Sleep guard.** `detectBlockedSleepPattern(command)` parses the FIRST
  statement only (split on `[;|&\r\n]`), matches
  `/^(?:start-sleep|sleep)(?:\s+-s(?:econds)?)?\s+(\d+)\s*$/i`, and
  returns a description string when `secs >= 2`; sub-2s sleeps are
  allowed (`:189-205`).
- **Auto-background.** Allowed except for
  `DISALLOWED_AUTO_BACKGROUND_COMMANDS = ['start-sleep', 'sleep']`
  (`:167-181`). Assistant-mode budget is
  `ASSISTANT_BLOCKING_BUDGET_MS = 15_000` (`:162`).
- **Search/read collapse classifier.** `PS_SEARCH_COMMANDS`,
  `PS_READ_COMMANDS`, `PS_SEMANTIC_NEUTRAL_COMMANDS` sets at `:54-95`
  drive `isSearchOrReadPowerShellCommand` for the UI collapsing logic.
- **Prompt.** Generated by `getPrompt()` in `prompt.ts:73-145`;
  branches on `getPowerShellEdition()` in `{'desktop','core',null}` and
  on `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS`. Verbatim assets in §6.7
  (edition sections, background note, sleep guidance).
- **Permission delegation.** Defers to `powershellToolHasPermission` and
  the readOnly/destructive classifiers under `tools/PowerShellTool/`
  (mirror of the Bash flow, see spec 10 for shell-command permission
  semantics).

### 3.8 RemoteTriggerTool - `'RemoteTrigger'`

- **Citation.** `tools.ts:36-38` (gate), `:236` (inclusion).
- **Gate.** `feature('AGENT_TRIGGERS_REMOTE')` (compile-time) plus
  runtime `getFeatureValue_CACHED_MAY_BE_STALE('tengu_surreal_dali',
  false) && isPolicyAllowed('allow_remote_sessions')`
  (`RemoteTriggerTool.ts:57-62`).
- **Surface.** `searchHint = 'manage scheduled remote agent triggers'`,
  `maxResultSizeChars: 100_000`, `shouldDefer: true`,
  `isConcurrencySafe: () => true`, `isReadOnly: input => input.action
  === 'list' || input.action === 'get'`, `toAutoClassifierInput: input
  => 'RemoteTrigger ${action}${trigger_id ? " ${trigger_id}" : ""}'`
  (`:46-72`).
- **Network.** `axios.request({method, url, headers, data, timeout:
  20_000, signal: context.abortController.signal, validateStatus: () =>
  true})`. Base URL = `${getOauthConfig().BASE_API_URL}/v1/code/triggers`.
  Headers: `Authorization: 'Bearer ${accessToken}'`, `Content-Type:
  'application/json'`, `'anthropic-version': '2023-06-01'`,
  `'anthropic-beta': 'ccr-triggers-2026-01-30'` (`TRIGGERS_BETA`),
  `'x-organization-uuid': orgUUID` (`:78-143`).
- **Action routing.**
  - `list` -> `GET base`.
  - `get` -> `GET base/{trigger_id}`; throws `'get requires
    trigger_id'` if missing.
  - `create` -> `POST base` with `body`; throws on missing body.
  - `update` -> `POST base/{trigger_id}` with `body` (partial); throws
    `'update requires trigger_id'` / `'update requires body'`.
  - `run` -> `POST base/{trigger_id}/run` with `data = {}`; throws
    `'run requires trigger_id'`.
- **Auth pre-check.** `await checkAndRefreshOAuthTokenIfNeeded()`;
  missing access token -> `'Not authenticated with a claude.ai account.
  Run /login and try again.'` (`:79-85`). Missing org UUID -> `'Unable
  to resolve organization UUID.'` (`:86-89`).
- **Output.** `{status: res.status, json: jsonStringify(res.data)}`.
  Result block content: `'HTTP ${status}\n${json}'` (`:152-158`).
- **Adjacent.** Spec 32 (Kairos triggers cross-references), spec 25
  (OAuth client).

### 3.9 TestingPermissionTool - `'TestingPermission'`

- **Citation.** Static import `tools.ts:58`; included at `tools.ts:244`
  only when `process.env.NODE_ENV === 'test'`.
- **Surface.** Empty strict-object schema; `userFacingName: () =>
  'TestingPermission'`; `maxResultSizeChars: 100_000`; `isReadOnly: ()
  => true`; `isConcurrencySafe: () => true`.
- **Behavior.** `checkPermissions()` always returns `{behavior:'ask',
  message:'Run test?'}`. `call()` returns `'TestingPermission executed
  successfully'`. All renderers return `null`.
- **Note.** The leaked artifact contains the literal compiled string
  `"production" === 'test'` in `isEnabled()`, which always evaluates
  `false` - the bundled runtime build has the comparison short-circuited
  away (`testing/TestingPermissionTool.tsx:28-29`).

### 3.10 ScheduleCron triplet — `CronCreate` / `CronDelete` / `CronList`

- **Citation.** `cronTools = feature('AGENT_TRIGGERS') ? [CronCreateTool,
  CronDeleteTool, CronListTool] : []` (`tools.ts:29-35`); spread once at
  the inclusion site (`tools.ts:235`, single spread `...cronTools` — there
  is one inclusion line, not three).
- **Two-layer gate** (mirrors §3.3 BriefTool):
  - **Build-time entitlement.** The triplet array literal is wrapped in a
    `feature('AGENT_TRIGGERS')` ternary so Bun DCE can strip the
    `require()`s when the flag is off. With the flag off, `cronTools`
    is the empty array and the inclusion site is a no-op.
  - **Runtime entitlement.** Each tool's `isEnabled()` returns
    `isKairosCronEnabled()` (`CronCreateTool.ts:67-69`,
    `CronDeleteTool.ts:46-48`, `CronListTool.ts:48-50`). A separate
    `isDurableCronEnabled()` predicate (`prompt.ts`) gates whether the
    `durable:true` schema field actually persists across sessions —
    `CronCreateTool.call()` computes
    `effectiveDurable = durable && isDurableCronEnabled()` (`:120`)
    so the kill-switch can flip mid-session without producing schema
    validation errors.
- **CronCreate input schema** (`z.strictObject` from `inputSchema()` at
  `CronCreateTool.ts:27-43`):
  - `cron: z.string()` — 5-field cron expression in local time.
  - `prompt: z.string()` — prompt to enqueue at each fire time.
  - `recurring: semanticBoolean(z.boolean().optional())` — default
    `true`; `false` = one-shot then auto-delete.
  - `durable: semanticBoolean(z.boolean().optional())` — default
    `false`; `true` writes to `.claude/scheduled_tasks.json`.
- **CronCreate output schema.** `{id, humanSchedule, recurring,
  durable?}` (`:45-53`).
- **CronCreate `validateInput`** (`:82-115`) — four error codes, in order:
  - `errorCode:1` invalid cron expression (5-field parse failure).
  - `errorCode:2` cron does not match any calendar date in next year
    (`nextCronRunMs(...) === null`).
  - `errorCode:3` `MAX_JOBS = 50` (`:25`); refuses when `tasks.length >=
    50`.
  - `errorCode:4` `durable && getTeammateContext()` — durable crons
    refused for teammates because teammates do not persist across
    sessions (would orphan `agentId`).
- **CronCreate `call()`** (`:117-141`): `addCronTask(cron, prompt,
  recurring, effectiveDurable, getTeammateContext()?.agentId)` →
  `setScheduledTasksEnabled(true)` to start the polling tick.
- **CronDelete** (`CronDeleteTool.ts`): input
  `z.strictObject({id: z.string()})`; output `{id}`. `validateInput`
  errors: `errorCode:1` no such job; `errorCode:2` teammate trying to
  delete another agent's cron (compares `task.agentId` to
  `getTeammateContext()?.agentId`). `call()` is `await
  removeCronTasks([id])` from `utils/cronTasks.ts`.
- **CronList** (`CronListTool.ts`): empty input (`z.strictObject({})`);
  output `{jobs: [{id, cron, humanSchedule, prompt, recurring?,
  durable?}]}`. `isReadOnly()` and `isConcurrencySafe()` both `true`.
  Teammates only see crons whose `agentId` matches their own; team-lead
  context (`ctx === undefined`) sees all. Renders human-readable schedule
  via `cronToHuman` from `utils/cron.ts`.
- **Cross-spec.** Slash command `/cron` lives in spec 21c. Related
  durable-task persistence (`scheduled_tasks.json`, scheduler tick) is
  shared with spec 32 (Kairos) and spec 35 (UDS inbox / remote
  triggers — `RemoteTriggerTool` shares the trigger registry). Sub-file
  catalog in §13.3 (purpose-only); deeper helper files
  (`utils/cronTasks.ts`, `utils/cron.ts`, `prompt.ts`, `UI.tsx`) are
  not re-documented here to avoid duplication.

### 3.11-3.25 Missing-source tools (registry-citation level only)

> See §12 for the consolidated missing-source ledger. Each tool's
> registry-citation, gate, and inclusion site is listed there. Public
> interface for these tools cannot be reproduced from the leak - no
> prompt, schema, or `call()` body is documented to avoid invention.

---

## §4. Behavior - algorithmic notes

### 4.1 Inclusion order in `getAllBaseTools()`

The relevant tail of the registry array (built tools omitted) is, with
exact gates:

```
AskUserQuestionTool                        // tools.ts:211 (always; see §3.4)
SkillTool                                  // 17
EnterPlanModeTool                          // 18
USER_TYPE==='ant' ? [ConfigTool] : []      // tools.ts:214
USER_TYPE==='ant' ? [TungstenTool] : []    // tools.ts:215  - missing source
SuggestBackgroundPRTool ? [...] : []       // tools.ts:216  - missing source
WebBrowserTool ? [...] : []                // tools.ts:217  - missing source
isTodoV2Enabled() ? [TaskCreate..List]:[]  // 15
OverflowTestTool ? [...] : []              // tools.ts:221  - missing source
CtxInspectTool ? [...] : []                // tools.ts:222  - missing source
TerminalCaptureTool ? [...] : []           // tools.ts:223  - missing source
isEnvTruthy(ENABLE_LSP_TOOL) ? [LSPTool]:[] // 16
isWorktreeModeEnabled() ? [...] : []       // 18
getSendMessageTool()                       // 14 (always-included)
ListPeersTool ? [...] : []                 // tools.ts:227  - missing source
isAgentSwarmsEnabled() ? [Team*]:[]        // 14
VerifyPlanExecutionTool ? [...] : []       // tools.ts:231  - missing source
USER_TYPE==='ant' && REPLTool ? [...]:[]   // tools.ts:232
WorkflowTool ? [...] : []                  // tools.ts:233  - missing source
SleepTool ? [...] : []                     // tools.ts:234  - partial
...cronTools                               // tools.ts:235  - present (CronCreate/Delete/List, see §3.10)
RemoteTriggerTool ? [...] : []             // tools.ts:236
MonitorTool ? [...] : []                   // tools.ts:237  - missing source
BriefTool                                  // tools.ts:238 (gated by isEnabled)
SendUserFileTool ? [...] : []              // tools.ts:239  - missing source
PushNotificationTool ? [...] : []          // tools.ts:240  - missing source
SubscribePRTool ? [...] : []               // tools.ts:241  - missing source
getPowerShellTool() ? [...] : []           // tools.ts:242
SnipTool ? [...] : []                      // tools.ts:243  - missing source
NODE_ENV==='test' ? [TestingPermissionTool]:[] // tools.ts:244
ListMcpResourcesTool, ReadMcpResourceTool      // 16
isToolSearchEnabledOptimistic() ? [...] : []   // 08
```

Order is load-bearing for prompt cache stability - see comment at
`tools.ts:191` ("MUST stay in sync with
...claude_code_global_system_caching"). The
`biome-ignore-all assist/source/organizeImports` header at `:1` exists
because the ANT-only conditional `require`s and `feature(...)` requires
must remain at module-top before the registry array.

### 4.2 ConfigTool special cases - pseudocode

```
function ConfigTool.call({setting, value}, ctx):
  if VOICE_MODE && setting == 'voiceEnabled' && !isVoiceGrowthBookEnabled():
    return {success:false, error:`Unknown setting: "${setting}"`}
  if !isSupported(setting):
    return {success:false, error:`Unknown setting: "${setting}"`}

  config := getConfig(setting); path := getPath(setting)
  if value === undefined:
    cur  := getValue(config.source, path)
    disp := config.formatOnRead ? config.formatOnRead(cur) : cur
    return {success:true, operation:'get', setting, value: disp}

  if setting == 'remoteControlAtStartup' &&
     value.toString().toLowerCase().trim() == 'default':
    saveGlobalConfig(prev => delete prev.remoteControlAtStartup)
    resolved := getRemoteControlAtStartup()
    syncReplBridgeEnabled(resolved); return success(resolved)

  if config.type == 'boolean': coerce ('true'/'false') or fail
  if options := getOptionsForSetting(setting); !options.includes(String(value)):
    return invalid-options error
  if config.validateOnWrite: r := await config.validateOnWrite(v); fail-if-invalid
  if VOICE_MODE && setting=='voiceEnabled' && v===true:
    voicePreflight()  # see §3.5 step 7
  prev := getValue(config.source, path)
  write(config.source, path, finalValue)
  if VOICE_MODE && setting=='voiceEnabled':
    settingsChangeDetector.notifyChange('userSettings')
  if config.appStateKey: setAppState(prev[appStateKey] = finalValue)
  if setting=='remoteControlAtStartup': resync replBridgeEnabled
  logEvent('tengu_config_tool_changed', {setting, value: String(v)})
  return {success:true, operation:'set', setting, previousValue:prev, newValue:finalValue}
```

### 4.3 RemoteTrigger action routing - pseudocode

```
function RemoteTriggerTool.call({action, trigger_id, body}, ctx):
  await checkAndRefreshOAuthTokenIfNeeded()
  accessToken := getClaudeAIOAuthTokens()?.accessToken
  if !accessToken: throw 'Not authenticated with a claude.ai account. Run /login and try again.'
  orgUUID := await getOrganizationUUID()
  if !orgUUID: throw 'Unable to resolve organization UUID.'
  base := `${getOauthConfig().BASE_API_URL}/v1/code/triggers`
  headers := {Authorization, Content-Type, anthropic-version, anthropic-beta=ccr-triggers-2026-01-30, x-organization-uuid}
  switch action:
    list   -> GET base
    get    -> require trigger_id ; GET base/${trigger_id}
    create -> require body       ; POST base body
    update -> require trigger_id and body ; POST base/${trigger_id} body
    run    -> require trigger_id ; POST base/${trigger_id}/run with data={}
  res := axios.request({..., timeout:20_000, signal, validateStatus:()=>true})
  return {status: res.status, json: jsonStringify(res.data)}
```

### 4.4 PowerShell sleep-pattern detector - pseudocode

```
function detectBlockedSleepPattern(command):
  first := command.trim().split(/[;|&\r\n]/)[0]?.trim() ?? ''
  m := /^(?:start-sleep|sleep)(?:\s+-s(?:econds)?)?\s+(\d+)\s*$/i.exec(first)
  if !m: return null
  secs := parseInt(m[1], 10)
  if secs < 2: return null     # sub-2s allowed
  rest := command.trim().slice(first.length).replace(/^[\s;|&]+/, '')
  return rest
    ? `Start-Sleep ${secs} followed by: ${rest}`
    : `standalone Start-Sleep ${secs}`
```

### 4.5 SyntheticOutputTool factory - pseudocode

```
toolCache := WeakMap<object, {tool}|{error}>
function createSyntheticOutputTool(jsonSchema):
  if cached := toolCache.get(jsonSchema): return cached
  result := buildSyntheticOutputTool(jsonSchema)
  toolCache.set(jsonSchema, result); return result

function buildSyntheticOutputTool(jsonSchema):
  ajv := new Ajv({allErrors:true})
  if !ajv.validateSchema(jsonSchema): return {error: ajv.errorsText(ajv.errors)}
  validate := ajv.compile(jsonSchema)
  return {tool: {...SyntheticOutputTool, inputJSONSchema: jsonSchema,
    call: input -> if !validate(input): throw TelemetrySafeError(`Output does not match required schema: ${formatErrors(validate.errors)}`)
                   return {data: 'Structured output provided successfully', structured_output: input}}}
```

---

## §5. Side effects, dependencies, errors

- **Filesystem.** `BriefTool` opens attachment files via
  `resolveAttachments` (`BriefTool.ts:195-199`). `ConfigTool` writes to
  `~/.claude.json` (global) or the project user-settings store. No
  other present tool here writes the filesystem.
- **Network.** `RemoteTriggerTool` calls the claude.ai CCR API
  (`v1/code/triggers`) with a 20s axios timeout and abort signal.
- **Process state.** `PowerShellTool` spawns processes via
  `tasks/LocalShellTask`, registers/unregisters foreground tasks, and
  may back-fork into a background task; sandbox routing follows
  BashTool's shouldUseSandbox flow (see spec 10).
- **AppState.** `ConfigTool` mutates `replBridgeEnabled`,
  `replBridgeOutboundOnly`, generic `appStateKey` slots
  (`verbose|mainLoopModel|thinkingEnabled`).
- **Errors.**
  - SyntheticOutputTool throws
    `TelemetrySafeError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` with
    public + telemetry-safe messages on schema mismatch
    (`SyntheticOutputTool.ts:148-152`).
  - RemoteTriggerTool throws plain `Error` for missing-args, OAuth, and
    org-UUID failures.
  - PowerShellTool throws `WINDOWS_SANDBOX_POLICY_REFUSAL` (verbatim
    §6.7) on policy violation.
  - ConfigTool returns `{success:false, error}` for every failure path
    rather than throwing; `mapToolResultToToolResultBlockParam` sets
    `is_error: true` only for the failure branch.

---

## §6. Verbatim assets

> Assets are reproduced bit-exact. Any divergence from source files
> cited at the head of each block is a bug in this spec.

### §6.1 SleepTool - `tools/SleepTool/prompt.ts`

```ts
import { TICK_TAG } from '../../constants/xml.js'

export const SLEEP_TOOL_NAME = 'Sleep'

export const DESCRIPTION = 'Wait for a specified duration'

export const SLEEP_TOOL_PROMPT = `Wait for a specified duration. The user can interrupt the sleep at any time.

Use this when the user tells you to sleep or rest, when you have nothing to do, or when you're waiting for something.

You may receive <${TICK_TAG}> prompts - these are periodic check-ins. Look for useful work to do before sleeping.

You can call this concurrently with other tools - it won't interfere with them.

Prefer this over \`Bash(sleep ...)\` - it doesn't hold a shell process.

Each wake-up costs an API call, but the prompt cache expires after 5 minutes of inactivity - balance accordingly.`
```

(Note: source uses Unicode em-dashes; the literal source dash is `—`.)

### §6.2 SyntheticOutputTool - `SyntheticOutputTool.ts:11-21,44-51`

```ts
const inputSchema = lazySchema(() => z.object({}).passthrough())
const outputSchema = lazySchema(() =>
  z.string().describe('Structured output tool result'),
)

export const SYNTHETIC_OUTPUT_TOOL_NAME = 'StructuredOutput'

// description():
//   'Return structured output in the requested format'
// prompt():
//   'Use this tool to return your final response in the requested structured format. You MUST call this tool exactly once at the end of your response to provide the structured output.'
```

### §6.3 BriefTool - `BriefTool.ts:20-65` and `prompt.ts`

Input schema (verbatim):

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    message: z
      .string()
      .describe('The message for the user. Supports markdown formatting.'),
    attachments: z
      .array(z.string())
      .optional()
      .describe(
        'Optional file paths (absolute or relative to cwd) to attach. Use for photos, screenshots, diffs, logs, or any file the user should see alongside your message.',
      ),
    status: z
      .enum(['normal', 'proactive'])
      .describe(
        "Use 'proactive' when you're surfacing something the user hasn't asked for and needs to see now - task completion while they're away, a blocker you hit, an unsolicited status update. Use 'normal' when replying to something the user just said.",
      ),
  }),
)
```

Output schema (verbatim):

```ts
const outputSchema = lazySchema(() =>
  z.object({
    message: z.string().describe('The message'),
    attachments: z
      .array(
        z.object({
          path: z.string(),
          size: z.number(),
          isImage: z.boolean(),
          file_uuid: z.string().optional(),
        }),
      )
      .optional()
      .describe('Resolved attachment metadata'),
    sentAt: z
      .string()
      .optional()
      .describe(
        'ISO timestamp captured at tool execution on the emitting process. Optional - resumed sessions replay pre-sentAt outputs verbatim.',
      ),
  }),
)
```

Prompt assets (`tools/BriefTool/prompt.ts`):

```ts
export const BRIEF_TOOL_NAME = 'SendUserMessage'
export const LEGACY_BRIEF_TOOL_NAME = 'Brief'
export const DESCRIPTION = 'Send a message to the user'

export const BRIEF_TOOL_PROMPT = `Send a message the user will read. Text outside this tool is visible in the detail view, but most won't open it - the answer lives here.

\`message\` supports markdown. \`attachments\` takes file paths (absolute or cwd-relative) for images, diffs, logs.

\`status\` labels intent: 'normal' when replying to what they just asked; 'proactive' when you're initiating - a scheduled task finished, a blocker surfaced during background work, you need input on something they haven't asked about. Set it honestly; downstream routing uses it.`

export const BRIEF_PROACTIVE_SECTION = `## Talking to the user

${BRIEF_TOOL_NAME} is where your replies go. Text outside it is visible if the user expands the detail view, but most won't - assume unread. Anything you want them to actually see goes through ${BRIEF_TOOL_NAME}. The failure mode: the real answer lives in plain text while ${BRIEF_TOOL_NAME} just says "done!" - they see "done!" and miss everything.

So: every time the user says something, the reply they actually read comes through ${BRIEF_TOOL_NAME}. Even for "hi". Even for "thanks".

If you can answer right away, send the answer. If you need to go look - run a command, read files, check something - ack first in one line ("On it - checking the test output"), then work, then send the result. Without the ack they're staring at a spinner.

For longer work: ack -> work -> result. Between those, send a checkpoint when something useful happened - a decision you made, a surprise you hit, a phase boundary. Skip the filler ("running tests..."), a checkpoint earns its place by carrying information.

Keep messages tight - the decision, the file:line, the PR number. Second person always ("your config"), never third.`
```

(Note: source uses Unicode em-dashes throughout BRIEF_PROACTIVE_SECTION;
the literal characters are `—`.)

### §6.4 AskUserQuestionTool - schemas and prompts

`tools/AskUserQuestionTool/prompt.ts` (verbatim):

```ts
export const ASK_USER_QUESTION_TOOL_NAME = 'AskUserQuestion'
export const ASK_USER_QUESTION_TOOL_CHIP_WIDTH = 12

export const DESCRIPTION =
  'Asks the user multiple choice questions to gather information, clarify ambiguity, understand preferences, make decisions or offer them choices.'

export const PREVIEW_FEATURE_PROMPT = {
  markdown: `
Preview feature:
Use the optional \`preview\` field on options when presenting concrete artifacts that users need to visually compare:
- ASCII mockups of UI layouts or components
- Code snippets showing different implementations
- Diagram variations
- Configuration examples

Preview content is rendered as markdown in a monospace box. Multi-line text with newlines is supported. When any option has a preview, the UI switches to a side-by-side layout with a vertical option list on the left and preview on the right. Do not use previews for simple preference questions where labels and descriptions suffice. Note: previews are only supported for single-select questions (not multiSelect).
`,
  html: `
Preview feature:
Use the optional \`preview\` field on options when presenting concrete artifacts that users need to visually compare:
- HTML mockups of UI layouts or components
- Formatted code snippets showing different implementations
- Visual comparisons or diagrams

Preview content must be a self-contained HTML fragment (no <html>/<body> wrapper, no <script> or <style> tags - use inline style attributes instead). Do not use previews for simple preference questions where labels and descriptions suffice. Note: previews are only supported for single-select questions (not multiSelect).
`,
} as const

export const ASK_USER_QUESTION_TOOL_PROMPT = `Use this tool when you need to ask the user questions during execution. This allows you to:
1. Gather user preferences or requirements
2. Clarify ambiguous instructions
3. Get decisions on implementation choices as you work
4. Offer choices to the user about what direction to take.

Usage notes:
- Users will always be able to select "Other" to provide custom text input
- Use multiSelect: true to allow multiple answers to be selected for a question
- If you recommend a specific option, make that the first option in the list and add "(Recommended)" at the end of the label

Plan mode note: In plan mode, use this tool to clarify requirements or choose between approaches BEFORE finalizing your plan. Do NOT use this tool to ask "Is my plan ready?" or "Should I proceed?" - use ${EXIT_PLAN_MODE_TOOL_NAME} for plan approval. IMPORTANT: Do not reference "the plan" in your questions (e.g., "Do you have feedback about the plan?", "Does the plan look good?") because the user cannot see the plan in the UI until you call ${EXIT_PLAN_MODE_TOOL_NAME}. If you need plan approval, use ${EXIT_PLAN_MODE_TOOL_NAME} instead.
`
```

Question option schema (`AskUserQuestionTool.tsx:14-17`):

```ts
const questionOptionSchema = lazySchema(() => z.object({
  label: z.string().describe('The display text for this option that the user will see and select. Should be concise (1-5 words) and clearly describe the choice.'),
  description: z.string().describe('Explanation of what this option means or what will happen if chosen. Useful for providing context about trade-offs or implications.'),
  preview: z.string().optional().describe('Optional preview content rendered when this option is focused. Use for mockups, code snippets, or visual comparisons that help users compare options. See the tool description for the expected content format.')
}));
```

Question schema (`:19-24`):

```ts
const questionSchema = lazySchema(() => z.object({
  question: z.string().describe('The complete question to ask the user. Should be clear, specific, and end with a question mark. Example: "Which library should we use for date formatting?" If multiSelect is true, phrase it accordingly, e.g. "Which features do you want to enable?"'),
  header: z.string().describe(`Very short label displayed as a chip/tag (max ${ASK_USER_QUESTION_TOOL_CHIP_WIDTH} chars). Examples: "Auth method", "Library", "Approach".`),
  options: z.array(questionOptionSchema()).min(2).max(4).describe(`The available choices for this question. Must have 2-4 options. Each option should be a distinct, mutually exclusive choice (unless multiSelect is enabled). There should be no 'Other' option, that will be provided automatically.`),
  multiSelect: z.boolean().default(false).describe('Set to true to allow the user to select multiple options instead of just one. Use when choices are not mutually exclusive.')
}));
```

Annotation + common-fields + input schema (`:25-67`):

```ts
const annotationsSchema = lazySchema(() => {
  const annotationSchema = z.object({
    preview: z.string().optional().describe('The preview content of the selected option, if the question used previews.'),
    notes: z.string().optional().describe('Free-text notes the user added to their selection.')
  });
  return z.record(z.string(), annotationSchema).optional().describe('Optional per-question annotations from the user (e.g., notes on preview selections). Keyed by question text.');
});
const UNIQUENESS_REFINE = {
  check: (data) => { /* questions and option labels both globally/locally unique */ },
  message: 'Question texts must be unique, option labels must be unique within each question'
} as const;
const commonFields = lazySchema(() => ({
  answers: z.record(z.string(), z.string()).optional().describe('User answers collected by the permission component'),
  annotations: annotationsSchema(),
  metadata: z.object({
    source: z.string().optional().describe('Optional identifier for the source of this question (e.g., "remember" for /remember command). Used for analytics tracking.')
  }).optional().describe('Optional metadata for tracking and analytics purposes. Not displayed to user.')
}));
const inputSchema = lazySchema(() => z.strictObject({
  questions: z.array(questionSchema()).min(1).max(4).describe('Questions to ask the user (1-4 questions)'),
  ...commonFields()
}).refine(UNIQUENESS_REFINE.check, { message: UNIQUENESS_REFINE.message }));
```

Output schema (`:69-73`):

```ts
const outputSchema = lazySchema(() => z.object({
  questions: z.array(questionSchema()).describe('The questions that were asked'),
  answers: z.record(z.string(), z.string()).describe('The answers provided by the user (question text -> answer string; multi-select answers are comma-separated)'),
  annotations: annotationsSchema()
}));
```

User-facing strings (`:91`, `:198-204`, `:241-243`):

- `"User answered Claude's questions:"` - header before the answer list.
- Each row: `"· ${questionText} -> ${answer}"` (color: `inactive`).
- Reject path: `'User declined to answer questions'`.
- Tool-result block content:
  `"User has answered your questions: ${answersText}. You can now
  continue with the user's answers in mind."`.

### §6.5 ConfigTool - input/output schemas and prompt scaffold

Input + output (`ConfigTool.ts:36-62`):

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    setting: z
      .string()
      .describe(
        'The setting key (e.g., "theme", "model", "permissions.defaultMode")',
      ),
    value: z
      .union([z.string(), z.boolean(), z.number()])
      .optional()
      .describe('The new value. Omit to get current value.'),
  }),
)

const outputSchema = lazySchema(() =>
  z.object({
    success: z.boolean(),
    operation: z.enum(['get', 'set']).optional(),
    setting: z.string().optional(),
    value: z.unknown().optional(),
    previousValue: z.unknown().optional(),
    newValue: z.unknown().optional(),
    error: z.string().optional(),
  }),
)
```

Prompt scaffolding (`tools/ConfigTool/prompt.ts:9-77`, verbatim with
dynamic substitutions denoted by `${...}`):

```
DESCRIPTION = 'Get or set Claude Code configuration settings.'

Generated prompt body:

`Get or set Claude Code configuration settings.

  View or change Claude Code settings. Use when the user requests configuration changes, asks about current settings, or when adjusting a setting would benefit them.


## Usage
- **Get current value:** Omit the "value" parameter
- **Set new value:** Include the "value" parameter

## Configurable settings list
The following settings are available for you to change:

### Global Settings (stored in ~/.claude.json)
${globalSettings.join('\n')}

### Project Settings (stored in settings.json)
${projectSettings.join('\n')}

${modelSection}
## Examples
- Get theme: { "setting": "theme" }
- Set dark theme: { "setting": "theme", "value": "dark" }
- Enable vim mode: { "setting": "editorMode", "value": "vim" }
- Enable verbose: { "setting": "verbose", "value": true }
- Change model: { "setting": "model", "value": "opus" }
- Change permission mode: { "setting": "permissions.defaultMode", "value": "plan" }
`
```

Model section (`prompt.ts:79-93`):

```
`## Model
- model - Override the default model. Available options:
${options.map(o => '  - ' + (o.value === null ? 'null/"default"' : '"' + o.value + '"') + ': ' + (o.descriptionForModel ?? o.description)).join('\n')}`
```

Fallback if `getModelOptions()` throws:

```
`## Model
- model - Override the default model (sonnet, opus, haiku, best, or full model ID)`
```

Voice-mode hide rule (`prompt.ts:23-28`): when
`feature('VOICE_MODE') && key === 'voiceEnabled' &&
!isVoiceGrowthBookEnabled()`, the entry is omitted from the listing.

### §6.6 ConfigTool - `SUPPORTED_SETTINGS` registry (verbatim)

`tools/ConfigTool/supportedSettings.ts:29-186`:

```ts
export const SUPPORTED_SETTINGS: Record<string, SettingConfig> = {
  theme: {
    source: 'global',
    type: 'string',
    description: 'Color theme for the UI',
    options: feature('AUTO_THEME') ? THEME_SETTINGS : THEME_NAMES,
  },
  editorMode: {
    source: 'global',
    type: 'string',
    description: 'Key binding mode',
    options: EDITOR_MODES,
  },
  verbose: {
    source: 'global',
    type: 'boolean',
    description: 'Show detailed debug output',
    appStateKey: 'verbose',
  },
  preferredNotifChannel: {
    source: 'global',
    type: 'string',
    description: 'Preferred notification channel',
    options: NOTIFICATION_CHANNELS,
  },
  autoCompactEnabled: {
    source: 'global',
    type: 'boolean',
    description: 'Auto-compact when context is full',
  },
  autoMemoryEnabled: {
    source: 'settings',
    type: 'boolean',
    description: 'Enable auto-memory',
  },
  autoDreamEnabled: {
    source: 'settings',
    type: 'boolean',
    description: 'Enable background memory consolidation',
  },
  fileCheckpointingEnabled: {
    source: 'global',
    type: 'boolean',
    description: 'Enable file checkpointing for code rewind',
  },
  showTurnDuration: {
    source: 'global',
    type: 'boolean',
    description:
      'Show turn duration message after responses (e.g., "Cooked for 1m 6s")',
  },
  terminalProgressBarEnabled: {
    source: 'global',
    type: 'boolean',
    description: 'Show OSC 9;4 progress indicator in supported terminals',
  },
  todoFeatureEnabled: {
    source: 'global',
    type: 'boolean',
    description: 'Enable todo/task tracking',
  },
  model: {
    source: 'settings',
    type: 'string',
    description: 'Override the default model',
    appStateKey: 'mainLoopModel',
    getOptions: () => {
      try {
        return getModelOptions()
          .filter(o => o.value !== null)
          .map(o => o.value as string)
      } catch {
        return ['sonnet', 'opus', 'haiku']
      }
    },
    validateOnWrite: v => validateModel(String(v)),
    formatOnRead: v => (v === null ? 'default' : v),
  },
  alwaysThinkingEnabled: {
    source: 'settings',
    type: 'boolean',
    description: 'Enable extended thinking (false to disable)',
    appStateKey: 'thinkingEnabled',
  },
  'permissions.defaultMode': {
    source: 'settings',
    type: 'string',
    description: 'Default permission mode for tool usage',
    options: feature('TRANSCRIPT_CLASSIFIER')
      ? ['default', 'plan', 'acceptEdits', 'dontAsk', 'auto']
      : ['default', 'plan', 'acceptEdits', 'dontAsk'],
  },
  language: {
    source: 'settings',
    type: 'string',
    description:
      'Preferred language for Claude responses and voice dictation (e.g., "japanese", "spanish")',
  },
  teammateMode: {
    source: 'global',
    type: 'string',
    description:
      'How to spawn teammates: "tmux" for traditional tmux, "in-process" for same process, "auto" to choose automatically',
    options: TEAMMATE_MODES,
  },
  ...(process.env.USER_TYPE === 'ant'
    ? {
        classifierPermissionsEnabled: {
          source: 'settings' as const,
          type: 'boolean' as const,
          description:
            'Enable AI-based classification for Bash(prompt:...) permission rules',
        },
      }
    : {}),
  ...(feature('VOICE_MODE')
    ? {
        voiceEnabled: {
          source: 'settings' as const,
          type: 'boolean' as const,
          description: 'Enable voice dictation (hold-to-talk)',
        },
      }
    : {}),
  ...(feature('BRIDGE_MODE')
    ? {
        remoteControlAtStartup: {
          source: 'global' as const,
          type: 'boolean' as const,
          description:
            'Enable Remote Control for all sessions (true | false | default)',
          formatOnRead: () => getRemoteControlAtStartup(),
        },
      }
    : {}),
  ...(feature('KAIROS') || feature('KAIROS_PUSH_NOTIFICATION')
    ? {
        taskCompleteNotifEnabled: {
          source: 'global' as const,
          type: 'boolean' as const,
          description:
            'Push to your mobile device when idle after Claude finishes (requires Remote Control)',
        },
        inputNeededNotifEnabled: {
          source: 'global' as const,
          type: 'boolean' as const,
          description:
            'Push to your mobile device when a permission prompt or question is waiting (requires Remote Control)',
        },
        agentPushNotifEnabled: {
          source: 'global' as const,
          type: 'boolean' as const,
          description:
            'Allow Claude to push to your mobile device when it deems it appropriate (requires Remote Control)',
        },
      }
    : {}),
}
```

### §6.7 PowerShellTool - verbatim assets

Tool name (`tools/PowerShellTool/toolName.ts:2`):

```ts
export const POWERSHELL_TOOL_NAME = 'PowerShell' as const
```

Full input schema (`PowerShellTool.tsx:228-239`):

```ts
const fullInputSchema = lazySchema(() => z.strictObject({
  command: z.string().describe('The PowerShell command to execute'),
  timeout: semanticNumber(z.number().optional()).describe(`Optional timeout in milliseconds (max ${getMaxTimeoutMs()})`),
  description: z.string().optional().describe('Clear, concise description of what this command does in active voice.'),
  run_in_background: semanticBoolean(z.boolean().optional()).describe(`Set to true to run this command in the background. Use Read to read the output later.`),
  dangerouslyDisableSandbox: semanticBoolean(z.boolean().optional()).describe('Set this to true to dangerously override sandbox mode and run commands without sandboxing.')
}));

// When CLAUDE_CODE_DISABLE_BACKGROUND_TASKS is truthy at module load:
const inputSchema = lazySchema(() => isBackgroundTasksDisabled
  ? fullInputSchema().omit({run_in_background: true})
  : fullInputSchema());
```

Output schema (`:245-256`):

```ts
const outputSchema = lazySchema(() => z.object({
  stdout: z.string().describe('The standard output of the command'),
  stderr: z.string().describe('The standard error output of the command'),
  interrupted: z.boolean().describe('Whether the command was interrupted'),
  returnCodeInterpretation: z.string().optional().describe('Semantic interpretation for non-error exit codes with special meaning'),
  isImage: z.boolean().optional().describe('Flag to indicate if stdout contains image data'),
  persistedOutputPath: z.string().optional().describe('Path to persisted full output when too large for inline'),
  persistedOutputSize: z.number().optional().describe('Total output size in bytes when persisted'),
  backgroundTaskId: z.string().optional().describe('ID of the background task if command is running in background'),
  backgroundedByUser: z.boolean().optional().describe('True if the user manually backgrounded the command with Ctrl+B'),
  assistantAutoBackgrounded: z.boolean().optional().describe('True if the command was auto-backgrounded by the assistant-mode blocking budget')
}));
```

Sandbox refusal message (`:219`):

```
'Enterprise policy requires sandboxing, but sandboxing is not available on native Windows. Shell command execution is blocked on this platform by policy.'
```

Prompt template (`tools/PowerShellTool/prompt.ts:73-145`) is reproduced
character-for-character at the cited lines and is referenced by source
citation rather than inline duplication to avoid copy-drift. The prompt
is assembled by `getPrompt()` from:

- the base body (`prompt.ts:78-144`),
- the edition section returned by `getEditionSection(edition)` for
  `edition` in `{'desktop', 'core', null}` (`:51-71`),
- the optional background note returned by `getBackgroundUsageNote()`
  (`:26-31`) when `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is not truthy,
- the optional sleep guidance returned by `getSleepGuidance()`
  (`:33-44`) on the same condition,

with `getMaxOutputLength()`, `getMaxTimeoutMs()`,
`getDefaultTimeoutMs()`, `POWERSHELL_TOOL_NAME`, `GLOB_TOOL_NAME`,
`GREP_TOOL_NAME`, `FILE_READ_TOOL_NAME`, `FILE_WRITE_TOOL_NAME`,
`FILE_EDIT_TOOL_NAME` substituted into the template at call time.

### §6.8 RemoteTriggerTool - verbatim assets

`tools/RemoteTriggerTool/prompt.ts`:

```ts
export const REMOTE_TRIGGER_TOOL_NAME = 'RemoteTrigger'

export const DESCRIPTION =
  'Manage scheduled remote Claude Code agents (triggers) via the claude.ai CCR API. Auth is handled in-process - the token never reaches the shell.'

export const PROMPT = `Call the claude.ai remote-trigger API. Use this instead of curl - the OAuth token is added automatically in-process and never exposed.

Actions:
- list: GET /v1/code/triggers
- get: GET /v1/code/triggers/{trigger_id}
- create: POST /v1/code/triggers (requires body)
- update: POST /v1/code/triggers/{trigger_id} (requires body, partial update)
- run: POST /v1/code/triggers/{trigger_id}/run

The response is the raw JSON from the API.`
```

Input schema (`RemoteTriggerTool.ts:18-31`):

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    action: z.enum(['list', 'get', 'create', 'update', 'run']),
    trigger_id: z
      .string()
      .regex(/^[\w-]+$/)
      .optional()
      .describe('Required for get, update, and run'),
    body: z
      .record(z.string(), z.unknown())
      .optional()
      .describe('JSON body for create and update'),
  }),
)
```

Output schema (`:35-40`):

```ts
const outputSchema = lazySchema(() =>
  z.object({
    status: z.number(),
    json: z.string(),
  }),
)
```

Constants (`:44`):

```ts
const TRIGGERS_BETA = 'ccr-triggers-2026-01-30'
```

User-facing error strings:

- `'Not authenticated with a claude.ai account. Run /login and try again.'`
- `'Unable to resolve organization UUID.'`
- `'get requires trigger_id'`, `'update requires trigger_id'`,
  `'run requires trigger_id'`, `'create requires body'`,
  `'update requires body'`.
- Result block content template: `"HTTP ${output.status}\n${output.json}"`.

### §6.9 TestingPermissionTool - verbatim assets

`tools/testing/TestingPermissionTool.tsx` (key fragments):

```ts
const NAME = 'TestingPermission'
const inputSchema = lazySchema(() => z.strictObject({}))

// description():
//   'Test tool that always asks for permission'
// prompt():
//   'Test tool that always asks for permission before executing. Used for end-to-end testing.'
// userFacingName():
//   'TestingPermission'
// checkPermissions():
//   { behavior: 'ask', message: 'Run test?' }
// call():
//   data: '${NAME} executed successfully'
```

Note the production-bundled `isEnabled()` literal:
`return "production" === 'test'`. Source comment
(`TestingPermissionTool.tsx:1-3`):

```
This testing-only tool will always pop up a permission dialog when called by
the model.
```

### §6.10 REPLTool - partial verbatim assets

`tools/REPLTool/constants.ts:11-46` (verbatim already shown in §3.6) is
the complete public asset surface in the leak. There is no schema,
prompt, or `call()` body to reproduce.

---

## §7. Concurrency, idempotency, ordering

- **Concurrency-safe** (declared `isConcurrencySafe: () => true`):
  `SyntheticOutputTool`, `BriefTool`, `AskUserQuestionTool`,
  `ConfigTool`, `RemoteTriggerTool`, `TestingPermissionTool`.
  `PowerShellTool` is conditionally concurrency-safe -
  `isConcurrencySafe(input)` returns `this.isReadOnly?.(input) ?? false`
  (`PowerShellTool.tsx:285-287`).
- **Read-only** (declared `isReadOnly: () => true`):
  `SyntheticOutputTool`, `BriefTool`, `AskUserQuestionTool`,
  `TestingPermissionTool`. `ConfigTool` is read-only iff `value ===
  undefined`. `RemoteTriggerTool` is read-only iff `action === 'list' ||
  action === 'get'`. PowerShell delegates to
  `isReadOnlyCommand(input.command)` (sync, no AST - known limitation,
  `:309-315`).
- **`shouldDefer: true`** (deferred-tool surface, see 03):
  `ConfigTool`, `AskUserQuestionTool`, `RemoteTriggerTool`. None of the
  other present tools opt into deferral.
- **Aliases.** `BriefTool` sets `aliases: [LEGACY_BRIEF_TOOL_NAME]` (=
  `'Brief'`) - `toolMatchesName()` honors aliases for permission rules
  and tool-search.
- **Caching.** `SyntheticOutputTool`'s per-schema toolCache is identity
  (`WeakMap<jsonSchema, CreateResult>`). `tengu_kairos_brief` is read
  with a 5-minute refresh window (`KAIROS_BRIEF_REFRESH_MS = 5*60*1000`).
- **Cache invariant.** The order of `getAllBaseTools()` is load-bearing
  for the `claude_code_global_system_caching` policy (`tools.ts:191`):
  conditional spreads must keep their position even when the inner
  array is empty.

---

## §8. Telemetry

- `tengu_brief_send` - `{proactive: status === 'proactive',
  attachment_count: attachments?.length ?? 0}` (`BriefTool.ts:188-191`).
- `tengu_config_tool_changed` - `{setting, value: String(finalValue)}`
  (`ConfigTool.ts:383-389`). Both fields are typed as
  `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS`.

No other present tool in this spec emits telemetry inside its `call()`.

---

## §9. Settings touched

- `theme`, `editorMode`, `verbose`, `preferredNotifChannel`,
  `autoCompactEnabled`, `autoMemoryEnabled`, `autoDreamEnabled`,
  `fileCheckpointingEnabled`, `showTurnDuration`,
  `terminalProgressBarEnabled`, `todoFeatureEnabled`, `model`,
  `alwaysThinkingEnabled`, `permissions.defaultMode`, `language`,
  `teammateMode` - written via `ConfigTool` (verbatim list in §6.6).
- `classifierPermissionsEnabled` - ANT-only.
- `voiceEnabled` - `feature('VOICE_MODE')`.
- `remoteControlAtStartup` - `feature('BRIDGE_MODE')`.
- `taskCompleteNotifEnabled`, `inputNeededNotifEnabled`,
  `agentPushNotifEnabled` - `feature('KAIROS') ||
  feature('KAIROS_PUSH_NOTIFICATION')`.

`AppState` keys mutated by `ConfigTool`: `verbose`, `mainLoopModel`,
`thinkingEnabled` (via `appStateKey`), plus `replBridgeEnabled` and
`replBridgeOutboundOnly` (special-case for `remoteControlAtStartup`).

---

## §10. Permission notes (delegate to 09)

Not duplicated - see spec 09. Spec-19-specific deltas:

- `BriefTool`, `SyntheticOutputTool`, `TestingPermissionTool`
  (effectively disabled in production builds) do not register custom
  permission UI.
- `AskUserQuestionTool` uses `requiresUserInteraction: () => true` and
  `shouldDefer: true`; the deferred-question dialog is rendered by the
  permission engine (09). When `--channels` is active the tool is
  `isEnabled() === false`, and channel permission relay's
  `interactiveHandler.ts` skips `requiresUserInteraction()` tools -
  there is no alternate approval path.
- `ConfigTool` sets `shouldDefer: true` and short-circuits permission to
  `'allow'` for reads; writes always `'ask'`.
- `RemoteTriggerTool` sets `shouldDefer: true` (no custom permission)
  and relies on the standard ask flow.
- `ReviewArtifactTool` (missing source) has a custom permission renderer
  registered at `components/permissions/PermissionRequest.tsx:36-37,57`
  and `:136`. This is the only spec-19 tool with a dedicated
  PermissionRequest component, and the only registry reference outside
  `tools.ts`. See §12.

---

## §11. Open questions / verification required

- **REPLTool primitive surface** - `REPLTool.ts` is missing. The
  schema, prompt, and the VM bridging code that maps REPL primitives to
  the tools listed in `getReplPrimitiveTools()` cannot be verified from
  the leak. Mark for adversarial review against an installed CLI.
- **SleepTool implementation** - only `prompt.ts` is in the leak; the
  Zod schema, default duration, and the wake-up tick handler reference
  `<${TICK_TAG}>` from `constants/xml.ts` but the listener wiring lives
  outside this directory (likely in `query.ts` / proactive subsystem,
  spec 31).
- **Workflow scripts integration** - `WorkflowTool/bundled/index.js`
  exports `initBundledWorkflows()` (`tools.ts:131`) and
  `WorkflowTool/createWorkflowCommand.js` exports `getWorkflowCommands`
  (`commands.ts:401-405`). Source is missing; the actual
  workflow-script schema, runner, and bundled script set cannot be
  documented.
- **ANT vs feature-flag overlap** - `TungstenTool` is gated **both** by
  the `USER_TYPE === 'ant'` import at `tools.ts:60` (top-level static
  import - present in the leak even when ANT-stripped) AND by the
  `:215` inclusion guard. The static import means an external build
  must still bundle the file unless the bundler can dead-code import
  references; verify in spec 26 / build-config docs.

---

## §12. Missing-source ledger (spec-19 catch-all)

Each entry below is a tool referenced in the registry whose
implementation source is **absent from the leak**. We document only:
the tool's expected name (where derivable from registry/comments), the
exact registry citation, the gate, and the inclusion site. Public
interface, prompt, schema, and `call()` semantics are deliberately not
filled in - see §11 verification list.

| # | Tool | Registry citation | Gate | Inclusion site |
|---|---|---|---|---|
| 12.1 | `MonitorTool` | `tools.ts:39-41` | `feature('MONITOR_TOOL')` | `tools.ts:237` |
| 12.2 | `WorkflowTool` (incl. `bundled/index.js::initBundledWorkflows`, `WorkflowTool.js`, `createWorkflowCommand.js::getWorkflowCommands`) | `tools.ts:129-134`; `commands.ts:401-405` | `feature('WORKFLOW_SCRIPTS')` | `tools.ts:233`; `commands.ts` workflow command merge |
| 12.3 | `WebBrowserTool` | `tools.ts:117-119` | `feature('WEB_BROWSER_TOOL')` | `tools.ts:217` |
| 12.4 | `SnipTool` | `tools.ts:123-125` | `feature('HISTORY_SNIP')` | `tools.ts:243` |
| 12.5 | `TungstenTool` | top-level static import `tools.ts:60`; inclusion guard `tools.ts:215` | `process.env.USER_TYPE === 'ant'` (ANT-only) | `tools.ts:215` |
| 12.6 | `VerifyPlanExecutionTool` | `tools.ts:91-95` | `process.env.CLAUDE_CODE_VERIFY_PLAN === 'true'` | `tools.ts:231` |
| 12.7 | `PushNotificationTool` | `tools.ts:45-49` | `feature('KAIROS') \|\| feature('KAIROS_PUSH_NOTIFICATION')` | `tools.ts:240` |
| 12.8 | `SendUserFileTool` | `tools.ts:42-44` | `feature('KAIROS')` | `tools.ts:239` |
| 12.9 | `SubscribePRTool` | `tools.ts:50-52` | `feature('KAIROS_GITHUB_WEBHOOKS')` | `tools.ts:241` |
| 12.10 | `SuggestBackgroundPRTool` | top-level conditional require `tools.ts:20-24`; inclusion guard `tools.ts:216` | `process.env.USER_TYPE === 'ant'` (ANT-only) | `tools.ts:216` |
| 12.11 | `OverflowTestTool` | `tools.ts:107-109` | `feature('OVERFLOW_TEST_TOOL')` | `tools.ts:221` |
| 12.12 | `CtxInspectTool` | `tools.ts:110-112` | `feature('CONTEXT_COLLAPSE')` | `tools.ts:222` |
| 12.13 | `TerminalCaptureTool` | `tools.ts:113-116` | `feature('TERMINAL_PANEL')` | `tools.ts:223` |
| 12.14 | `ListPeersTool` | `tools.ts:126-128` | `feature('UDS_INBOX')` | `tools.ts:227` |
| 12.15 | `ReviewArtifactTool` (+ `ReviewArtifactPermissionRequest`) | `components/permissions/PermissionRequest.tsx:36-37`, switch case `:57`, body branch `:136` (NOT registered in `tools.ts`) | `feature('REVIEW_ARTIFACT')` | (registered indirectly through permission renderer; `tools.ts` does not list it) |

<!-- Phase 9.6c: rows for `CronCreateTool` / `CronDeleteTool` / `CronListTool` removed from this ledger — source IS present in `src/tools/ScheduleCronTool/`. See §3.10 for the canonical schemas/behavior and §13.3 for the sub-file catalog. The triplet has a single inclusion site (`tools.ts:235`, `...cronTools` spread), not three; documenting it as three rows here misrepresented the registry topology. Renumbering: old 12.18 (ReviewArtifactTool) is now 12.15. -->

Notes:

- Items 12.5 (Tungsten) and 12.10 (SuggestBackgroundPR) are the only
  ANT-only catch-all members. Both use ternary-with-non-null assertion
  at the inclusion site (`tools.ts:215-216`); the `TungstenTool` import
  is unconditional at module top, while `SuggestBackgroundPRTool` uses
  a top-level conditional require so external builds can DCE the
  reference.
- Item 12.15 (`ReviewArtifactTool`) is the only spec-19 reference
  outside `tools.ts` — it's surfaced via the permission engine's
  renderer dispatch, not via the tools registry.
- The `cronTools` triplet was previously listed here (Phase 9.5
  numbering 12.15-12.17) but is now documented in §3.10 with full
  source-derived behavior. Master plan §8 still lists
  `AGENT_TRIGGERS -> ScheduleCronTool/{CronCreate, CronDelete,
  CronList}Tool`; the `/cron` slash commands live in spec 21c. The
  triplet has a single inclusion site (one `...cronTools` spread at
  `tools.ts:235`), not three.

## §13. Tool sub-file catalogs (Phase 10 cleanup)

Several tool directories have helper files that the main spec body discusses
behaviorally but does not enumerate by filename. Listed here for coverage.

### §13.1 `src/tools/PowerShellTool/` helpers

| File | Purpose |
|---|---|
| `src/tools/PowerShellTool/clmTypes.ts` | `CLM_ALLOWED_TYPES` set — Microsoft Constrained Language Mode allowlist (type accelerators + full names, lowercase). Inverted check: any AST type literal not in this set escalates to `ask`. ADSI/ADSISearcher deliberately **removed** to block LDAP-bind sandbox-escape vector. |
| `src/tools/PowerShellTool/commonParameters.ts` | `COMMON_SWITCHES` (`-verbose`, `-debug`) and `COMMON_VALUE_PARAMS` (`-erroraction`, `-warningaction`, ..., `-pipelinevariable`) — `[CmdletBinding()]` common parameters. Shared between `pathValidation.ts` and `readOnlyValidation.ts` to avoid an import cycle; stored lowercase with leading dash. |
| `src/tools/PowerShellTool/gitSafety.ts` | Git-specific sandbox-escape mitigations: bare-repo attack (cwd containing `HEAD`+`objects/`+`refs/`) and the git-internal-write + git compound. `resolveCwdReentry` normalizes `../<cwd-basename>/` paths so the validator and PS runtime agree on `hooks/` matches. |
| `src/tools/PowerShellTool/powershellPermissions.ts` | PowerShell adaptation of `bashPermissions.ts` — case-insensitive cmdlet matching against `ShellPermissionRule`s, `createPermissionRequestMessage` integration, bare-git-repo guard via `isCurrentDirectoryBareGitRepo`. |
| `src/tools/PowerShellTool/powershellSecurity.ts` | AST-based dangerous-pattern detection: code injection, download cradles, privilege escalation, dynamic command names, COM objects, module loading. Returns `'ask'` when AST parse fails (safe default). Consumes `DANGEROUS_SCRIPT_BLOCK_CMDLETS`, `FILEPATH_EXECUTION_CMDLETS`, `MODULE_LOADING_CMDLETS` from `utils/powershell/dangerousCmdlets.ts`. |

### §13.2 `src/tools/BriefTool/` helpers

| File | Purpose |
|---|---|
| `src/tools/BriefTool/upload.ts` | Best-effort attachment upload to `/api/oauth/file_upload` (private API) when the REPL bridge is active, so web viewers can preview attachments. Returns `file_uuid` for web rendering; `{path, size, isImage}` always preserved so local-terminal/desktop paths render even on failure. Imports gated via `bun:bundle` `feature('BRIDGE_MODE')`. |

### §13.3 `src/tools/ScheduleCronTool/` sub-files

The triplet is registered as a single feature-gated array in `tools.ts:29-35`
(`cronTools = feature('AGENT_TRIGGERS') ? [...] : []`) and spread once at
`tools.ts:235`. Canonical schemas/behavior are in §3.10. Sub-files:

| File | Purpose |
|---|---|
| `src/tools/ScheduleCronTool/CronCreateTool.ts` | `CronCreate` tool (158 lines). `MAX_JOBS = 50`; `validateInput` errorCodes 1–4 (cron parse / no-match-in-year / max-jobs / durable+teammate refusal); `call()` invokes `addCronTask` then `setScheduledTasksEnabled(true)`. Runtime gate `isKairosCronEnabled()`; `effectiveDurable = durable && isDurableCronEnabled()`. |
| `src/tools/ScheduleCronTool/CronDeleteTool.ts` | `CronDelete` tool — removes one cron job by `id`. Uses `removeCronTasks` from `utils/cronTasks.ts`; runtime gate `isKairosCronEnabled()`; `validateInput` errorCode 2 enforces `task.agentId === getTeammateContext()?.agentId` so teammates may only delete their own crons. |
| `src/tools/ScheduleCronTool/CronListTool.ts` | `CronList` tool — empty input schema, returns `{jobs: [{id, cron, humanSchedule, prompt, recurring?, durable?}]}` via `listAllCronTasks`; `isReadOnly() === true`, `isConcurrencySafe() === true`. Teammates filter to own `agentId`; team-lead sees all. Renders human-readable schedule via `cronToHuman` (`utils/cron.ts`). |
| `src/tools/ScheduleCronTool/prompt.ts` | Exports `CRON_*_TOOL_NAME`, `CRON_*_DESCRIPTION`, `buildCron{Create,Delete,List}Prompt(durable: boolean)` builders, `DEFAULT_MAX_AGE_DAYS`, and the runtime predicates `isKairosCronEnabled()` / `isDurableCronEnabled()` (the two-layer gate's runtime layer). |
| `src/tools/ScheduleCronTool/UI.tsx` | Ink renderers: `renderCreate{ToolUse,Result}Message`, `renderDelete{ToolUse,Result}Message`, `renderList{ToolUse,Result}Message`. |

Helper modules `utils/cron.ts` (`cronToHuman`, `parseCronExpression`,
`nextCronRunMs`) and `utils/cronTasks.ts` (`addCronTask`, `removeCronTasks`,
`listAllCronTasks`, `getCronFilePath`) are referenced but not re-documented
here; spec 32 (Kairos) and spec 35 (durable scheduling) are the canonical
homes for the on-disk file format and tick-loop semantics.

End of spec.
