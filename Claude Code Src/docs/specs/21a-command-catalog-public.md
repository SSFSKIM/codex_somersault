# 21a — Command Catalog: Public Built-ins

> Per-command spec for every entry that is registered in `COMMANDS()` without `INTERNAL_ONLY_COMMANDS` gating and without a `feature(...)` gate. Auth-gated commands (`availability: ['claude-ai' | 'console']`) are HERE because their registration is unconditional — they hide themselves via `meetsAvailabilityRequirement` (20 §5.6). The two ANT-only `prompt` commands `/commit` and `/commit-push-pr` are also documented here because their verbatim prompts are large and benefit from being adjacent to other prompt commands; their gating is noted but the bit-exact corpus lives here.
>
> **Read 20-command-system.md first.** This file inherits its terminology and does not redefine the `Command` type union, registry mechanics, or dispatch.

---

## 1. Purpose & Scope

This sub-file enumerates every `/foo` command that a non-ANT user can see when all feature flags are off. Each entry receives:
- name, kind (`local` / `local-jsx` / `prompt`), aliases
- argument hint, availability, isEnabled / isHidden conditions
- side effects (filesystem, network, settings, app state)
- verbatim prompt (for `prompt` commands) or behavior pseudocode (for `local` / `local-jsx`)
- caller pattern (REPL only, programmatic, MCP-callable, etc.)

### IN scope
- All entries listed in `COMMANDS()` at `commands.ts:258-346` whose registration is unconditional.
- The two `/login`-class entries gated by `!isUsing3PServices()` (`commands.ts:337`).
- `/commit` and `/commit-push-pr` (technically ANT-only — included here for prompt-corpus density).
- `/insights` lazy shim — the metadata + the dynamic-import side; the heavy `insights.ts` body is summarized.

### OUT of scope
- Feature-flag–gated commands → 21c.
- Other ANT-only commands → 21b.
- Skill discovery, plugin loader, workflow loader → 17 / 28 / 19.

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/commands.ts` | 754 | Registry. Cited for static order in `COMMANDS()` and for ANT/3P gates. |
| `src/commands/<name>/index.ts` (or `index.tsx`) | typically 5–30 | Command metadata + lazy `load: () => import('./<name>.js')`. |
| `src/commands/<name>/<name>.ts` (or `.tsx`) | varies | Implementation: `call` for `local` / `local-jsx`, `getPromptForCommand` already in metadata for `prompt`. |
| `src/commands/<name>.ts` (single-file form) | varies | Used for: `commit`, `commit-push-pr`, `init`, `init-verifiers`, `advisor`, `version`, `review`, `security-review`, `insights`, `statusline.tsx`, `ultraplan.tsx`, `brief.ts`, `bridge-kick.ts`. |
| `src/commands/createMovedToPluginCommand.ts` | 65 | Helper: wraps a prompt with an ANT branch that points at a plugin install command. Used by `pr_comments`, `security-review`. |

### 2.2 Source-coverage summary

See parent `21-command-catalog.md` § "Source-coverage inventory".

### 2.3 Imports from / imported by

Per-command files are leaves: each imports `Command` (and helpers) from `commands.js` / `types/command.js`. The registry imports them all (one default export per file). No command file imports another command file.

---

## 3. Public Interface (Contract)

The interface is per-command. This section is one sub-heading per command (or per related cluster). Every entry conforms to the `Command` type union from spec 20.

The static order below mirrors the order in `COMMANDS()` (`commands.ts:258-319`) so the typeahead order is reproducible. Where two commands share a name (e.g., `context` / `contextNonInteractive`, `extra-usage` / `extra-usage-noninteractive`, `reset-limits` / `reset-limits-noninteractive`), both entries are documented under one sub-heading.

### 3.1 `/add-dir` — Add a new working directory

- Path: `src/commands/add-dir/index.ts:3-9`
- Kind: `local-jsx`
- Description: `Add a new working directory`
- argumentHint: `<path>`
- Lazy load: `() => import('./add-dir.js')`
- Validation: `add-dir/validation.ts` (separate file).
- No `availability`, no `isEnabled`, always visible.

### 3.2 `/advisor` — Configure the advisor model

- Path: `src/commands/advisor.ts:96-107`
- Kind: `local`
- argumentHint: `[<model>|off]`
- `isEnabled: () => canUserConfigureAdvisor()` (see `utils/advisor.ts`)
- `isHidden` mirrors `isEnabled` (computed via `get isHidden()`)
- `supportsNonInteractive: true`
- `call(args, context)`:
  - If `args` empty → emit current advisor model state. Three cases: not set, set but base model unsupported, set and active. Verbatim strings (`advisor.ts:25-41`):
    - `'Advisor: not set\nUse "/advisor <model>" to enable (e.g. "/advisor opus").'`
    - `Advisor: ${current} (inactive)\nThe current model (${baseModel}) does not support advisors.`
    - `Advisor: ${current}\nUse "/advisor unset" to disable or "/advisor <model>" to change.`
  - `arg === 'unset' || arg === 'off'`:
    - Sets `advisorModel: undefined` in app state and `userSettings`.
    - `Advisor disabled (was ${prev}).` or `'Advisor already unset.'`.
  - Else: validates via `validateModel`, then `isValidAdvisorModel`. On success, persists `advisorModel: normalizedModel` to `userSettings`. Returns:
    - `Advisor set to ${normalizedModel}.`
    - or, if base model lacks support: `Advisor set to ${normalizedModel}.\nNote: Your current model (${baseModel}) does not support advisors. Switch to a supported model to use the advisor.`
  - On invalid input: `Invalid advisor model: ${error}` / `Unknown model: ${arg} (${resolvedModel})` / `The model ${arg} (${resolvedModel}) cannot be used as an advisor`
- Side effects: writes `advisorModel` to user settings via `updateSettingsForSource`.

### 3.3 `/agents` — Manage agent configurations

- Path: `src/commands/agents/index.ts:3-8`
- Kind: `local-jsx`
- No description amplification, no aliases.
- Implementation in `agents/agents.tsx` (Ink dialog).

### 3.4 `/branch` — Create a branch of the current conversation

- Path: `src/commands/branch/index.ts:3-13`
- Kind: `local-jsx`
- Aliases: `feature('FORK_SUBAGENT') ? [] : ['fork']` — `'fork'` alias only when the standalone `/fork` command (FORK_SUBAGENT flag) is absent
- argumentHint: `[name]`
- Implementation `branch/branch.ts:222-296`:
  - Creates fork of current transcript (`createFork`) — copies main-conversation `TranscriptMessage` entries to a new session file with `forkedFrom: { sessionId, messageUuid }` traceability.
  - Generates unique fork name: `<base> (Branch)` then `<base> (Branch 2)`, etc., via `getUniqueForkName` regex `^${escapeRegExp(baseName)} \\(Branch(?: (\\d+))?\\)$`.
  - Persists with `mode: 0o600`; project dir `mode: 0o700`.
  - Logs `tengu_conversation_forked` analytics event.
  - Resumes into the fork via `context.resume(sessionId, forkLog, 'fork')`.
  - Success message: `Branched conversation${titleInfo}. You are now in the branch.\nTo resume the original: claude -r ${originalSessionId}`
  - On failure: `Failed to branch conversation: ${message}`
  - Empty/missing transcript: throws `'No conversation to branch'` or `'No messages to branch'`.

### 3.5 `/btw` — Quick side question

- Path: `src/commands/btw/index.ts:3-11`
- Kind: `local-jsx`
- argumentHint: `<question>`
- `immediate: true` — bypasses queue.
- Description: `Ask a quick side question without interrupting the main conversation`
- In `REMOTE_SAFE_COMMANDS` (20 §6.2).

### 3.6 `/chrome` — Claude in Chrome (Beta) settings

- Path: `src/commands/chrome/index.ts:4-11`
- Kind: `local-jsx`
- availability: `['claude-ai']`
- `isEnabled: () => !getIsNonInteractiveSession()` (interactive only).
- Description: `Claude in Chrome (Beta) settings`

### 3.7 `/clear`, `/reset`, `/new` — Wipe conversation

- Path: `src/commands/clear/index.ts:10-17`
- Kind: `local`
- aliases: `['reset', 'new']`
- `supportsNonInteractive: false` — should just create a new session.
- Implementation `clear/clear.ts:4-7`: invokes `clearConversation(context)`, returns `{ type: 'text', value: '' }`.
- Helpers in `clear/caches.ts` and `clear/conversation.ts`.
- In `REMOTE_SAFE_COMMANDS` and `BRIDGE_SAFE_COMMANDS`.

### 3.8 `/color` — Set prompt-bar color

- Path: `src/commands/color/index.ts:7-15`
- Kind: `local-jsx`
- argumentHint: `<color|default>`
- `immediate: true`
- In `REMOTE_SAFE_COMMANDS`.

### 3.9 `/compact` — Summarize and clear

- Path: `src/commands/compact/index.ts:4-13`
- Kind: `local`
- Description: `Clear conversation history but keep a summary in context. Optional: /compact [instructions for summarization]`
- `isEnabled: () => !isEnvTruthy(process.env.DISABLE_COMPACT)`
- `supportsNonInteractive: true`
- argumentHint: `<optional custom summarization instructions>`
- In `BRIDGE_SAFE_COMMANDS`.
- Implementation `compact/compact.ts:40-137`: orchestrates `trySessionMemoryCompaction`, optional reactive path (gated by `feature('REACTIVE_COMPACT')`), or legacy `compactConversation`. Uses microcompact pre-pass. Triggers `notifyCompaction` with `feature('PROMPT_CACHE_BREAK_DETECTION')`. See spec 07 for compaction internals.
- Error strings (re-thrown verbatim from `services/compact/compact.js`): `ERROR_MESSAGE_NOT_ENOUGH_MESSAGES`, `ERROR_MESSAGE_INCOMPLETE_RESPONSE`, `ERROR_MESSAGE_USER_ABORT`. On abort: `'Compaction canceled.'` On other failures: `Error during compaction: ${error}`.
- Success display via `buildDisplayText(context, userDisplayMessage)` — uses `chalk.dim('Compacted ' + ...)` plus optional `(${expandShortcut} to see full summary)` prefix.

### 3.10 `/config`, `/settings` — Open config panel

- Path: `src/commands/config/index.ts:3-10`
- Kind: `local-jsx`
- aliases: `['settings']`
- Description: `Open config panel`

### 3.11 `/copy` — Copy last response

- Path: `src/commands/copy/index.ts:7-13`
- Kind: `local-jsx`
- Description: `Copy Claude's last response to clipboard (or /copy N for the Nth-latest)`
- In `REMOTE_SAFE_COMMANDS`.

### 3.12 `/desktop`, `/app` — Continue in Claude Desktop

- Path: `src/commands/desktop/index.ts:3-25`
- Kind: `local-jsx`
- aliases: `['app']`
- availability: `['claude-ai']`
- `isEnabled: isSupportedPlatform` — `process.platform === 'darwin'` OR (`win32` && `arch === 'x64'`).
- `isHidden` mirrors `isEnabled`.

### 3.13 `/context` — Visualize context usage (interactive + non-interactive)

- Path: `src/commands/context/index.ts:4-24`
- Two registrations sharing name `context`:
  - Interactive (`local-jsx`): `isEnabled: () => !getIsNonInteractiveSession()`. Description: `Visualize current context usage as a colored grid`. Lazy-loads `./context.js`.
  - Non-interactive (`local`): `supportsNonInteractive: true`, hidden + disabled when interactive session. Description: `Show current context usage`. Lazy-loads `./context-noninteractive.js`.
- Pattern: dual variants resolve via `meetsAvailabilityRequirement` × `isCommandEnabled` filtering at `getCommands()` time (20 §5.5).

### 3.14 `/cost` — Session cost / subscription status

- Path: `src/commands/cost/index.ts:8-22`
- Kind: `local`
- `supportsNonInteractive: true`
- `isHidden`: ANT users always see it; non-ANT users see it only when `!isClaudeAISubscriber()` (subscribers don't get a per-call cost; they see subscription wording).
- Implementation `cost/cost.ts:6-24`:
  - Subscriber path: shows
    - `'You are currently using your overages to power your Claude Code usage. We will automatically switch you back to your subscription rate limits when they reset'` (overage)
    - or `'You are currently using your subscription to power your Claude Code usage'` (normal).
    - ANT subscribers additionally get `\n\n[ANT-ONLY] Showing cost anyway:\n ${formatTotalCost()}` appended.
  - Non-subscriber path: just `formatTotalCost()`.
- In `REMOTE_SAFE_COMMANDS` and `BRIDGE_SAFE_COMMANDS`.

### 3.15 `/diff` — View uncommitted changes

- Path: `src/commands/diff/index.ts:3-9`
- Kind: `local-jsx`
- Description: `View uncommitted changes and per-turn diffs`

### 3.16 `/doctor` — Diagnose installation

- Path: `src/commands/doctor/index.ts:4-12`
- Kind: `local-jsx`
- `isEnabled: () => !isEnvTruthy(process.env.DISABLE_DOCTOR_COMMAND)`
- Description: `Diagnose and verify your Claude Code installation and settings`

### 3.17 `/effort` — Set effort level

- Path: `src/commands/effort/index.ts:5-13`
- Kind: `local-jsx`
- argumentHint: `[low|medium|high|max|auto]`
- `immediate` is dynamic via `shouldInferenceConfigCommandBeImmediate()` (`utils/immediateCommand.ts`).
- Description: `Set effort level for model usage`

### 3.18 `/exit`, `/quit` — Exit REPL

- Path: `src/commands/exit/index.ts:3-10`
- Kind: `local-jsx`
- aliases: `['quit']`
- `immediate: true`
- In `REMOTE_SAFE_COMMANDS`.

### 3.19 `/fast` — Toggle fast mode

- Path: `src/commands/fast/index.ts:6-21`
- Kind: `local-jsx`
- Description (dynamic via getter): `` `Toggle fast mode (${FAST_MODE_MODEL_DISPLAY} only)` ``
- availability: `['claude-ai', 'console']`
- `isEnabled: () => isFastModeEnabled()`; `isHidden` mirrors.
- argumentHint: `[on|off]`
- `immediate` = `shouldInferenceConfigCommandBeImmediate()`.

### 3.20 `/feedback`, `/bug` — Submit feedback

- Path: `src/commands/feedback/index.ts:6-19`
- Kind: `local-jsx`
- aliases: `['bug']`
- argumentHint: `[report]`
- `isEnabled` is the negation of a 7-way disjunction:
  - `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, `CLAUDE_CODE_USE_FOUNDRY`, `DISABLE_FEEDBACK_COMMAND`, `DISABLE_BUG_COMMAND` env truthy
  - `isEssentialTrafficOnly()` — privacy-level gate
  - `process.env.USER_TYPE === 'ant'` — ANTs don't see /feedback
  - `!isPolicyAllowed('allow_product_feedback')` — admin policy
- In `REMOTE_SAFE_COMMANDS`.

### 3.21 `/files` — List files in context

- Path: `src/commands/files/index.ts:3-9`
- Kind: `local`
- `isEnabled: () => process.env.USER_TYPE === 'ant'`
- `supportsNonInteractive: true`
- In `BRIDGE_SAFE_COMMANDS`.
- (Source for the `call` impl: `commands/files/files.ts` — not read here.)

### 3.22 `/heapdump` — Dump JS heap

- Path: `src/commands/heapdump/index.ts:3-10`
- Kind: `local`
- `isHidden: true` — never shown in typeahead.
- `supportsNonInteractive: true`
- Description: `Dump the JS heap to ~/Desktop`

### 3.23 `/help` — Show help

- Path: `src/commands/help/index.ts:3-9`
- Kind: `local-jsx`
- Description: `Show help and available commands`
- Renders `<HelpV2 commands={commands} onClose={onDone} />` — receives the resolved command list from `LocalJSXCommandContext.options.commands`.
- In `REMOTE_SAFE_COMMANDS`.

### 3.24 `/hooks` — View hook configurations

- Path: `src/commands/hooks/index.ts:3-9`
- Kind: `local-jsx`
- `immediate: true`
- Description: `View hook configurations for tool events`
- Implementation logs `tengu_hooks_command` analytics event, renders `<HooksConfigMenu>` with `getTools(permissionContext).map(_.name)`.

### 3.25 `/ide` — IDE integration

- Path: `src/commands/ide/index.ts:3-10`
- Kind: `local-jsx`
- argumentHint: `[open]`
- Description: `Manage IDE integrations and show status`

### 3.26 `/init` — Initialize CLAUDE.md (and optional skills/hooks)

- Path: `src/commands/init.ts`
- Kind: `prompt`
- `contentLength: 0` (dynamic).
- `progressMessage: 'analyzing your codebase'`
- Description (getter, `init.ts:229-235`):
  - `'Initialize new CLAUDE.md file(s) and optional skills/hooks with codebase documentation'` when `feature('NEW_INIT') && (USER_TYPE === 'ant' || isEnvTruthy(CLAUDE_CODE_NEW_INIT))`
  - else `'Initialize a new CLAUDE.md file with codebase documentation'`
- `getPromptForCommand()` (init.ts:239-253): calls `maybeMarkProjectOnboardingComplete()`, then returns the matching prompt.
- The two prompts (verbatim, see §6.1 and §6.2) are `OLD_INIT_PROMPT` (init.ts:6-26) and `NEW_INIT_PROMPT` (init.ts:28-224).
- Side effect: marks project as onboarded.
- Source: `'builtin'`.

### 3.27 `/init-verifiers` — Create verifier skills

- Path: `src/commands/init-verifiers.ts:3-262`
- Kind: `prompt`
- ANT-only via `INTERNAL_ONLY_COMMANDS` (`commands.ts:234`); placed in 21a because the prompt is large and the command is conceptually adjacent to `/init`.
- Description: `Create verifier skill(s) for automated verification of code changes`
- `progressMessage: 'analyzing your project and creating verifier skills'`
- `contentLength: 0` (dynamic).
- `source: 'builtin'`
- Returns one block of text — full prompt in §6.3.

### 3.28 `/keybindings` — Edit keybindings

- Path: `src/commands/keybindings/index.ts:4-12`
- Kind: `local`
- `isEnabled: () => isKeybindingCustomizationEnabled()`
- `supportsNonInteractive: false`
- Description: `Open or create your keybindings configuration file`
- In `REMOTE_SAFE_COMMANDS`.

### 3.29 `/install-github-app` — Set up GitHub Actions

- Path: `src/commands/install-github-app/index.ts:4-12`
- Kind: `local-jsx`
- availability: `['claude-ai', 'console']`
- `isEnabled: () => !isEnvTruthy(process.env.DISABLE_INSTALL_GITHUB_APP_COMMAND)`
- Multi-step Ink flow with steps: CheckGitHub, ChooseRepo, OAuthFlow, ApiKey, CheckExistingSecret, ExistingWorkflow, Warnings, InstallApp, Creating, Error, Success (per the directory listing `install-github-app/*.tsx`).
- `setupGitHubActions.ts` is the action runner.

### 3.30 `/install-slack-app` — Install Claude Slack app

- Path: `src/commands/install-slack-app/index.ts:3-10`
- Kind: `local`
- availability: `['claude-ai']`
- `supportsNonInteractive: false`
- Description: `Install the Claude Slack app`

### 3.31 `/mcp` — Manage MCP servers

- Path: `src/commands/mcp/index.ts:3-10`
- Kind: `local-jsx`
- `immediate: true`
- argumentHint: `[enable|disable [server-name]]`
- Implementation `mcp/mcp.tsx`: when `args === 'enable <server>'` or `'enable all'` etc., calls `useMcpToggleEnabled` toggle; on `'all'`, filters `mcpClients` to disabled (or non-disabled) and toggles. User-facing strings: `` `All MCP servers are already ${isEnabling ? "enabled" : "disabled"}` ``, `` `MCP server "${target}" not found` ``, `` `${isEnabling ? "Enabled" : "Disabled"} ${toToggle.length} MCP server(s)` ``, `` `MCP server "${target}" ${isEnabling ? "enabled" : "disabled"}` ``.
- Subcommands: `addCommand.ts`, `xaaIdpCommand.ts` — programmatic add/idp handlers.

### 3.32 `/memory` — Edit Claude memory files

- Path: `src/commands/memory/index.ts:3-9`
- Kind: `local-jsx`
- Description: `Edit Claude memory files`

### 3.33 `/mobile`, `/ios`, `/android` — Mobile QR

- Path: `src/commands/mobile/index.ts:3-9`
- Kind: `local-jsx`
- aliases: `['ios', 'android']`
- Description: `Show QR code to download the Claude mobile app`
- In `REMOTE_SAFE_COMMANDS`.

### 3.34 `/model` — Set model

- Path: `src/commands/model/index.ts:5-12`
- Kind: `local-jsx`
- argumentHint: `[model]`
- `immediate` = `shouldInferenceConfigCommandBeImmediate()`.
- Description (getter): `` `Set the AI model for Claude Code (currently ${renderModelName(getMainLoopModel())})` ``
- Renders `ModelPicker`, supports `COMMON_HELP_ARGS` and `COMMON_INFO_ARGS` parsing. Cancel message: `` `Kept model as ${chalk.bold(displayModel)}` `` with `display: 'system'`.
- Logs `tengu_model_command_menu` events with action ∈ {cancel, ...}.

### 3.35 `/output-style` — (deprecated)

- Path: `src/commands/output-style/index.ts:3-10`
- Kind: `local-jsx`
- `isHidden: true`
- Description: `Deprecated: use /config to change output style`

### 3.36 `/remote-env` — Configure remote env

- Path: `src/commands/remote-env/index.ts:5-13`
- Kind: `local-jsx`
- `isEnabled: () => isClaudeAISubscriber() && isPolicyAllowed('allow_remote_sessions')`
- `isHidden: true` when not enabled (mirror).
- Description: `Configure the default remote environment for teleport sessions`

### 3.37 `/plugin`, `/plugins`, `/marketplace` — Manage plugins

- Path: `src/commands/plugin/index.tsx:3-9`
- Kind: `local-jsx`
- aliases: `['plugins', 'marketplace']`
- `immediate: true`
- Description: `Manage Claude Code plugins`
- Implementation `plugin/plugin.tsx`: renders `<PluginSettings onComplete={onDone} args={args} />`. Multi-screen UI: `BrowseMarketplace`, `AddMarketplace`, `ManageMarketplaces`, `ManagePlugins`, `DiscoverPlugins`, `PluginErrors`, `PluginOptionsDialog`, `PluginOptionsFlow`, `PluginTrustWarning`, `UnifiedInstalledCell`, `ValidatePlugin`. Argument parsing in `parseArgs.ts`.

### 3.38 `/pr-comments` — Get GitHub PR comments

- Path: `src/commands/pr_comments/index.ts:3-49` (registration); built via `createMovedToPluginCommand`.
- Kind: `prompt`
- `pluginName: 'pr-comments'`, `pluginCommand: 'pr-comments'`
- `progressMessage: 'fetching PR comments'`
- ANT branch (verbatim from `createMovedToPluginCommand.ts:44-58`):
  > `This command has been moved to a plugin. Tell the user:\n\n1. To install the plugin, run:\n   claude plugin install ${pluginName}@claude-code-marketplace\n\n2. After installation, use /${pluginName}:${pluginCommand} to run this command\n\n3. For more information, see: https://github.com/anthropics/claude-code-marketplace/blob/main/${pluginName}/README.md\n\nDo not attempt to run the command. Simply inform the user about the plugin installation.`
- Non-ANT (private marketplace) branch: full PR-comments fetch prompt — see §6.4 verbatim.

### 3.39 `/release-notes` — View release notes

- Path: `src/commands/release-notes/index.ts:3-10`
- Kind: `local`
- `supportsNonInteractive: true`
- Description: `View release notes`
- In `BRIDGE_SAFE_COMMANDS`.

### 3.40 `/reload-plugins` — Apply pending plugin changes

- Path: `src/commands/reload-plugins/index.ts:6-13`
- Kind: `local`
- `supportsNonInteractive: false` — SDK callers use `query.reloadPlugins()` control request instead.
- Description: `Activate pending plugin changes in the current session`

### 3.41 `/rename` — Rename conversation

- Path: `src/commands/rename/index.ts:3-10`
- Kind: `local-jsx`
- `immediate: true`
- argumentHint: `[name]`
- Helpers in `rename/generateSessionName.ts` (auto-name via model) and `rename/rename.ts`.

### 3.42 `/resume`, `/continue` — Resume previous conversation

- Path: `src/commands/resume/index.ts:3-10`
- Kind: `local-jsx`
- aliases: `['continue']`
- argumentHint: `[conversation id or search term]`
- Description: `Resume a previous conversation`

### 3.43 `/session`, `/remote` — Show remote session URL/QR

- Path: `src/commands/session/index.ts:4-13`
- Kind: `local-jsx`
- aliases: `['remote']`
- `isEnabled: () => getIsRemoteMode()`
- `isHidden` mirrors.
- In `REMOTE_SAFE_COMMANDS`.

### 3.44 `/skills` — List skills

- Path: `src/commands/skills/index.ts:3-9`
- Kind: `local-jsx`
- Description: `List available skills`

### 3.45 `/stats` — Usage statistics

- Path: `src/commands/stats/index.ts:3-9`
- Kind: `local-jsx`
- Description: `Show your Claude Code usage statistics and activity`

### 3.46 `/status` — Status panel

- Path: `src/commands/status/index.ts:3-11`
- Kind: `local-jsx`
- `immediate: true`
- Description: `Show Claude Code status including version, model, account, API connectivity, and tool statuses`

### 3.47 `/statusline` — Set up the statusline

- Path: `src/commands/statusline.tsx:4-22`
- Kind: `prompt`
- `aliases: []`
- `progressMessage: 'setting up statusLine'`
- `allowedTools: [AGENT_TOOL_NAME, 'Read(~/**)', 'Edit(~/.claude/settings.json)']`
- `disableNonInteractive: true`
- `source: 'builtin'`
- Prompt body (verbatim, §6.5):
  > `` `Create an ${AGENT_TOOL_NAME} with subagent_type "statusline-setup" and the prompt "${prompt}"` ``
  > where `prompt = args.trim() || 'Configure my statusLine from my shell PS1 configuration'`
- In `REMOTE_SAFE_COMMANDS`.

### 3.48 `/stickers` — Order stickers

- Path: `src/commands/stickers/index.ts:3-9`
- Kind: `local`
- `supportsNonInteractive: false`
- In `REMOTE_SAFE_COMMANDS`.
- Description: `Order Claude Code stickers`

### 3.49 `/tag` — Tag session (ANT-only)

- Path: `src/commands/tag/index.ts:3-10`
- Kind: `local-jsx`
- `isEnabled: () => process.env.USER_TYPE === 'ant'` — universal at registration but enabled only for ANTs (this is the **only** non-INTERNAL_ONLY ANT gate aside from `/files` and the version-related `/version`).
- argumentHint: `<tag-name>`
- Description: `Toggle a searchable tag on the current session`

### 3.50 `/theme` — Change theme

- Path: `src/commands/theme/index.ts:3-9`
- Kind: `local-jsx`
- In `REMOTE_SAFE_COMMANDS`.

### 3.51 `/feedback` (already documented at §3.20)

### 3.52 `/review` — Review a PR (local)

- Path: `src/commands/review.ts:33-43`
- Kind: `prompt`
- Description: `Review a pull request`
- `progressMessage: 'reviewing pull request'`
- `contentLength: 0`
- `source: 'builtin'`
- Prompt body (verbatim §6.6):
  > See `LOCAL_REVIEW_PROMPT` (review.ts:9-31).

### 3.53 `/ultrareview` — Remote bughunter (CCR)

- Path: `src/commands/review.ts:48-54`
- Kind: `local-jsx` — the only entry point to the remote bughunter path; `/review` stays local.
- Description (verbatim with embedded URL constant `CCR_TERMS_URL`):
  > `` `~10–20 min · Finds and verifies bugs in your branch. Runs in Claude Code on the web. See ${CCR_TERMS_URL}` ``
  > where `CCR_TERMS_URL = 'https://code.claude.com/docs/en/claude-code-on-the-web'`
- `isEnabled: () => isUltrareviewEnabled()` (`commands/review/ultrareviewEnabled.ts`).
- Lazy load: `() => import('./review/ultrareviewCommand.js')`.
- Sub-files: `review/reviewRemote.ts`, `review/UltrareviewOverageDialog.tsx` (overage dialog when free reviews exhausted).

### 3.54 `/rewind`, `/checkpoint` — Restore previous point

- Path: `src/commands/rewind/index.ts:3-11`
- Kind: `local`
- aliases: `['checkpoint']`
- argumentHint: `''`
- `supportsNonInteractive: false`
- Description: `Restore the code and/or conversation to a previous point`

### 3.55 `/security-review` — Security review (moved to plugin)

- Path: `src/commands/security-review.ts:198-243`
- Kind: `prompt` (via `createMovedToPluginCommand`)
- pluginName: `'security-review'`
- `progressMessage: 'analyzing code changes for security risks'`
- ANT branch: same install-prompt as in `createMovedToPluginCommand` (§3.38).
- Non-ANT branch: parses `SECURITY_REVIEW_MARKDOWN` (security-review.ts:6-196) frontmatter to extract `allowed-tools`, executes shell commands inline, then submits the processed text. Frontmatter `allowed-tools` (verbatim):
  > `Bash(git diff:*), Bash(git status:*), Bash(git log:*), Bash(git show:*), Bash(git remote show:*), Read, Glob, Grep, LS, Task`
- Full body: see §6.7.

### 3.56 `/terminal-setup` — Terminal key bindings

- Path: `src/commands/terminalSetup/index.ts:5-21`
- Kind: `local-jsx`
- Description (computed from `env.terminal`): if `Apple_Terminal` → `'Enable Option+Enter key binding for newlines and visual bell'`; else `'Install Shift+Enter key binding for newlines'`.
- `isHidden: env.terminal !== null && env.terminal in NATIVE_CSIU_TERMINALS`
- `NATIVE_CSIU_TERMINALS` constant table (verbatim):
  ```
  { ghostty: 'Ghostty', kitty: 'Kitty', 'iTerm.app': 'iTerm2', WezTerm: 'WezTerm' }
  ```

### 3.57 `/upgrade` — Upgrade subscription

- Path: `src/commands/upgrade/index.ts:5-13`
- Kind: `local-jsx`
- availability: `['claude-ai']`
- `isEnabled: () => !isEnvTruthy(DISABLE_UPGRADE_COMMAND) && getSubscriptionType() !== 'enterprise'`
- Description: `Upgrade to Max for higher rate limits and more Opus`

### 3.58 `/extra-usage` — Configure extra usage (interactive + non-interactive)

- Path: `src/commands/extra-usage/index.ts:8-36`
- Two registrations sharing name `extra-usage`:
  - Interactive (`local-jsx`): `isEnabled: () => isExtraUsageAllowed() && !getIsNonInteractiveSession()`. Loads `./extra-usage.js`.
  - Non-interactive (`local`): `supportsNonInteractive: true`, hidden when interactive. Loads `./extra-usage-noninteractive.js`.
- `isExtraUsageAllowed` helper combines `isOverageProvisioningAllowed()` and `!DISABLE_EXTRA_USAGE_COMMAND`.
- Description: `Configure extra usage to keep working when limits are hit`
- `extra-usage-core.ts` shared logic.

### 3.59 `/rate-limit-options`

- Path: `src/commands/rate-limit-options/index.ts:5-15`
- Kind: `local-jsx`
- `isEnabled: () => isClaudeAISubscriber()`
- `isHidden: true` — internal only.
- Description: `Show options when rate limit is reached`

### 3.60 `/usage` — Plan usage

- Path: `src/commands/usage/index.ts:3-10`
- Kind: `local-jsx`
- availability: `['claude-ai']`
- Description: `Show plan usage limits`
- In `REMOTE_SAFE_COMMANDS`.

### 3.61 `/insights` — Session report (lazy shim)

- Path: `src/commands.ts:189-202` (registration); `src/commands/insights.ts` (3200 LOC implementation, lazy-imported).
- Kind: `prompt`
- Name `'insights'`, `contentLength: 0`, `progressMessage: 'analyzing your sessions'`, `source: 'builtin'`.
- Description: `Generate a report analyzing your Claude Code sessions`
- `getPromptForCommand(args, context)` body (verbatim):
  > ```typescript
  > const real = (await import('./commands/insights.js')).default
  > if (real.type !== 'prompt') throw new Error('unreachable')
  > return real.getPromptForCommand(args, context)
  > ```
- Heavy `insights.ts` includes:
  - ANT-only homespace data collection: SCP `~/root/.claude/projects/` from `coder list -o json` workspaces, `find ... | wc -l` per-host counts, parallel collection. Top-level constants `getAnalysisModel = getDefaultOpusModel`, `getInsightsModel = getDefaultOpusModel` (insights.ts:41-49).
  - Diff-rendering via `diffLines` (`diff` package).
  - Per-session stats from `getSessionFilesWithMtime`, `loadAllLogsFromSessionFile`, `getSessionIdFromLog` (utils/sessionStorage).
  - HTML escaping via `escapeXmlAttr`.
- Open question: full prompt body of `insights.ts` is too large to paste verbatim into this catalog (~110KB). Spec 21 §12 OQ #3 — punted; prompt corpus to be extracted in a follow-up if/when needed. Documented at registry-citation level.

### 3.62 `/vim` — Toggle Vim mode

- Path: `src/commands/vim/index.ts:3-10`
- Kind: `local`
- `supportsNonInteractive: false`
- Description: `Toggle between Vim and Normal editing modes`
- In `REMOTE_SAFE_COMMANDS`.

### 3.63 `/think-back` — Year in Review (gated by Statsig)

- Path: `src/commands/thinkback/index.ts:4-12`
- Kind: `local-jsx`
- name: `'think-back'`
- `isEnabled: () => checkStatsigFeatureGate_CACHED_MAY_BE_STALE('tengu_thinkback')`
- Description: `Your 2025 Claude Code Year in Review`

### 3.64 `/thinkback-play` — Play the animation (hidden helper)

- Path: `src/commands/thinkback-play/index.ts:5-14`
- Kind: `local`
- `isHidden: true`
- `supportsNonInteractive: false`
- Same Statsig gate as `/think-back`.
- Description: `Play the thinkback animation`

### 3.65 `/permissions`, `/allowed-tools` — Manage permission rules

- Path: `src/commands/permissions/index.ts:3-10`
- Kind: `local-jsx`
- aliases: `['allowed-tools']`
- Description: `Manage allow & deny tool permission rules`
- See spec 09 for permission state mutations.

### 3.66 `/plan` — Plan mode

- Path: `src/commands/plan/index.ts:3-10`
- Kind: `local-jsx`
- argumentHint: `[open|<description>]`
- Description: `Enable plan mode or view the current session plan`
- In `REMOTE_SAFE_COMMANDS`.

### 3.67 `/privacy-settings` — Privacy

- Path: `src/commands/privacy-settings/index.ts:4-12`
- Kind: `local-jsx`
- `isEnabled: () => isConsumerSubscriber()`
- Description: `View and update your privacy settings`

### 3.68 `/export` — Export conversation

- Path: `src/commands/export/index.ts:3-11`
- Kind: `local-jsx`
- argumentHint: `[filename]`
- Description: `Export the current conversation to a file or clipboard`

### 3.69 `/sandbox` — Toggle sandbox

- Path: `src/commands/sandbox-toggle/index.ts:5-44`
- Kind: `local-jsx`
- `immediate: true`
- argumentHint: `'exclude "command pattern"'`
- name: `'sandbox'`
- Description (dynamic getter): combines status icon (`figures.tick`/`figures.circle`/`figures.warning`) with verbatim suffixes:
  - `'sandbox disabled'`
  - `'sandbox enabled (auto-allow)'` or `'sandbox enabled'`
  - `', fallback allowed'` (when unsandboxed allowed)
  - `' (managed)'` when `areSandboxSettingsLockedByPolicy()`
  - All wrapped: `` `${icon} ${statusText} (⏎ to configure)` ``
- `isHidden`: when `!isSupportedPlatform() || !isPlatformInEnabledList()`.
- Settings I/O via `SandboxManager` (utils/sandbox/sandbox-adapter).

### 3.70 `/login`, `/logout` — Sign in / out

- Path: `src/commands/login/index.ts`, `src/commands/logout/index.ts`
- Both gated at registry by `!isUsing3PServices()` (`commands.ts:337`).
- Login (`local-jsx`):
  - `isEnabled: () => !isEnvTruthy(DISABLE_LOGIN_COMMAND)`
  - Description (computed): `hasAnthropicApiKeyAuth() ? 'Switch Anthropic accounts' : 'Sign in with your Anthropic account'`
  - Login itself: post-success calls `resetCostState()`, `refreshRemoteManagedSettings()`, `refreshPolicyLimits()`, `resetUserCache()`, `refreshGrowthBookAfterAuthChange()`, `clearTrustedDeviceToken()` then re-enrolls. Also calls `context.setMessages(stripSignatureBlocks)` to strip signature-bearing content blocks bound to the old API key.
  - Resets `bypassPermissionsCheck` and `autoModeGateCheck` after login.
- Logout (`local-jsx`):
  - `isEnabled: () => !isEnvTruthy(DISABLE_LOGOUT_COMMAND)`
  - Description: `Sign out from your Anthropic account`
  - `performLogout({clearOnboarding})`:
    - `await flushTelemetry()` BEFORE clearing credentials (prevents org data leakage).
    - `await removeApiKey()`.
    - `getSecureStorage().delete()` — wipes ALL secure storage.
    - `clearAuthRelatedCaches()`.
    - `saveGlobalConfig` resets `hasCompletedOnboarding`, `subscriptionNoticeCount`, `hasAvailableSubscription`, customApiKey approvals (when `clearOnboarding`).
    - Calls `clearBetasCaches`, `clearToolSchemaCache`, `clearRemoteManagedSettingsCache`, `clearPolicyLimitsCache`, `clearTrustedDeviceTokenCache`, `resetUserCache`, `refreshGrowthBookAfterAuthChange`.

### 3.71 `/passes` — Referral / passes

- Path: `src/commands/passes/index.ts:6-17`
- Kind: `local-jsx`
- Description (dynamic): `'Share a free week of Claude Code with friends and earn extra usage'` (when reward cached) else `'Share a free week of Claude Code with friends'`
- `isHidden`: `!eligible || !hasCache` from `checkCachedPassesEligibility()`.

### 3.72 `/tasks`, `/bashes` — List background tasks

- Path: `src/commands/tasks/index.ts:3-10`
- Kind: `local-jsx`
- aliases: `['bashes']`
- Description: `List and manage background tasks`

### 3.73 `/commit` — ANT-only git commit prompt

- Path: `src/commands/commit.ts`
- Kind: `prompt`
- Description: `Create a git commit`
- `progressMessage: 'creating commit'`
- `contentLength: 0` (dynamic)
- `source: 'builtin'`
- `allowedTools` (verbatim const `ALLOWED_TOOLS`, commit.ts:6-10):
  ```
  ['Bash(git add:*)', 'Bash(git status:*)', 'Bash(git commit:*)']
  ```
- `getPromptForCommand` runs `executeShellCommandsInPrompt` over the body (so `!\`...\`` syntax for inline shell substitution is expanded), with `alwaysAllowRules.command = ALLOWED_TOOLS` injected into the permission context.
- Prefix: when `USER_TYPE === 'ant' && isUndercover()`, `getUndercoverInstructions() + '\n'` is prepended (so the model writes commit messages without ANT-specific phrasing).
- Full prompt: §6.8.

### 3.74 `/commit-push-pr` — ANT-only commit + PR prompt

- Path: `src/commands/commit-push-pr.ts`
- Kind: `prompt`
- `description: 'Commit, push, and open a PR'`
- `progressMessage: 'creating commit and PR'`
- `contentLength` is computed dynamically (`get contentLength()`) — uses default branch `'main'` for the estimate.
- `allowedTools` (commit-push-pr.ts:10-24):
  ```
  ['Bash(git checkout --branch:*)', 'Bash(git checkout -b:*)',
   'Bash(git add:*)', 'Bash(git status:*)', 'Bash(git push:*)',
   'Bash(git commit:*)', 'Bash(gh pr create:*)', 'Bash(gh pr edit:*)',
   'Bash(gh pr view:*)', 'Bash(gh pr merge:*)',
   'ToolSearch', 'mcp__slack__send_message',
   'mcp__claude_ai_Slack__slack_send_message']
  ```
- `getPromptForCommand`: parallel `getDefaultBranch()` + `getEnhancedPRAttribution(context.getAppState)`; appends `## Additional instructions from user\n\n${trimmedArgs}` when args provided; runs `executeShellCommandsInPrompt`.
- Undercover mode (ANT only) elides reviewer args, the `## Changelog` section, and the Slack step.
- Full prompt: §6.9.

---

## 4. Data Model & State

Most commands have no persistent state of their own; mutations target:
- App state (`context.setAppState(...)`) — e.g. `/advisor`, `/brief`, `/effort`, `/model`, `/fast`.
- Settings file via `updateSettingsForSource(...)` (spec 02) — e.g. `/advisor`, `/permissions`, `/sandbox`.
- Session transcript file (`commands/branch/branch.ts`).
- Secure storage (`/logout`).
- Telemetry buffer (flushed by `/logout` before credential wipe).

`compactConversation` (called by `/compact`) returns a `CompactionResult` that becomes a `LocalCommandResult` of variant `compact` (spec 07).

---

## 5. Algorithm / Control Flow

Each command's algorithm is captured in §3 above as either:
- a verbatim prompt body (for `prompt` commands), reproduced in §6
- pseudocode summary citing the implementation file (for `local` / `local-jsx`)

Cluster-level control flow:
- All `prompt`-kind built-ins resolve to `[{type: 'text', text: ...}]` (Anthropic SDK `ContentBlockParam` shape). None return multi-block prompts.
- `executeShellCommandsInPrompt(promptContent, context, '<command-name>')` is the canonical pre-pass for prompts that contain `!\`shell\`` substitutions (commit, commit-push-pr, security-review). It mutates `context.toolPermissionContext.alwaysAllowRules.command` to include the command's `ALLOWED_TOOLS` so the inline shell commands don't prompt for permission.

---

## 6. Verbatim Assets

This section inlines the largest prompt corpora cited in §3. Smaller user-facing strings are inlined directly in §3.

### 6.1 `OLD_INIT_PROMPT` (verbatim, `init.ts:6-26`)

```
Please analyze this codebase and create a CLAUDE.md file, which will be given to future instances of Claude Code to operate in this repository.

What to add:
1. Commands that will be commonly used, such as how to build, lint, and run tests. Include the necessary commands to develop in this codebase, such as how to run a single test.
2. High-level code architecture and structure so that future instances can be productive more quickly. Focus on the "big picture" architecture that requires reading multiple files to understand.

Usage notes:
- If there's already a CLAUDE.md, suggest improvements to it.
- When you make the initial CLAUDE.md, do not repeat yourself and do not include obvious instructions like "Provide helpful error messages to users", "Write unit tests for all new utilities", "Never include sensitive information (API keys, tokens) in code or commits".
- Avoid listing every component or file structure that can be easily discovered.
- Don't include generic development practices.
- If there are Cursor rules (in .cursor/rules/ or .cursorrules) or Copilot rules (in .github/copilot-instructions.md), make sure to include the important parts.
- If there is a README.md, make sure to include the important parts.
- Do not make up information such as "Common Development Tasks", "Tips for Development", "Support and Documentation" unless this is expressly included in other files that you read.
- Be sure to prefix the file with the following text:

```
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
```
```

### 6.2 `NEW_INIT_PROMPT`

The full text spans `init.ts:28-224` (197 lines). Reproduced verbatim from the source — all phases, examples, command snippets, and YAML stubs preserved exactly:

(See `init.ts:28-224` — starts with `'Set up a minimal CLAUDE.md (and optionally skills and hooks) for this repo. CLAUDE.md is loaded into every Claude Code session, so it must be concise — only include what Claude would get wrong without it.\n\n## Phase 1: Ask what to set up\n\n...'`. Eight phases: Phase 1 (Ask), Phase 2 (Explore), Phase 3 (Fill in gaps), Phase 4 (Write CLAUDE.md), Phase 5 (Write CLAUDE.local.md), Phase 6 (Skills), Phase 7 (Optimizations), Phase 8 (Summary). Phase 7 calls out `/plugin install frontend-design@claude-plugins-official`, `/plugin install playwright@claude-plugins-official`, `/plugin install skill-creator@claude-plugins-official` as recommendations.)

> **Note**: the prompt embeds an `update-config` Skill invocation contract: `[hooks-only] <one-line summary>`. Reimplementers must preserve this literal invocation string for the hooks-only branch (init.ts:207).

### 6.3 `init-verifiers` prompt

Verbatim from `init-verifiers.ts:15-256`. Phases: Phase 1 (Auto-Detection), Phase 2 (Verification Tool Setup — Web/CLI/API), Phase 3 (Interactive Q&A), Phase 4 (Generate Verifier Skill), Phase 5 (Confirm Creation). Includes:

- The skill template structure (frontmatter + body skeleton with sections "Project Context", "Setup Instructions", "Authentication", "Reporting", "Cleanup", "Self-Update").
- The `allowed-tools` YAML for each verifier type (verbatim from init-verifiers.ts:213-245):
  - **verifier-playwright**: `Bash(npm:*)`, `Bash(yarn:*)`, `Bash(pnpm:*)`, `Bash(bun:*)`, `mcp__playwright__*`, `Read`, `Glob`, `Grep`.
  - **verifier-cli**: `Tmux`, `Bash(asciinema:*)`, `Read`, `Glob`, `Grep`.
  - **verifier-api**: `Bash(curl:*)`, `Bash(http:*)`, `Bash(npm:*)`, `Bash(yarn:*)`, `Read`, `Glob`, `Grep`.

### 6.4 `/pr-comments` non-ANT prompt (verbatim, `pr_comments/index.ts:11-39`)

```
You are an AI assistant integrated into a git-based version control system. Your task is to fetch and display comments from a GitHub pull request.

Follow these steps:

1. Use `gh pr view --json number,headRepository` to get the PR number and repository info
2. Use `gh api /repos/{owner}/{repo}/issues/{number}/comments` to get PR-level comments
3. Use `gh api /repos/{owner}/{repo}/pulls/{number}/comments` to get review comments. Pay particular attention to the following fields: `body`, `diff_hunk`, `path`, `line`, etc. If the comment references some code, consider fetching it using eg `gh api /repos/{owner}/{repo}/contents/{path}?ref={branch} | jq .content -r | base64 -d`
4. Parse and format all comments in a readable way
5. Return ONLY the formatted comments, with no additional text

Format the comments as:

## Comments

[For each comment thread:]
- @author file.ts#line:
  ```diff
  [diff_hunk from the API response]
  ```
  > quoted comment text

  [any replies indented]

If there are no comments, return "No comments found."

Remember:
1. Only show the actual comments, no explanatory text
2. Include both PR-level and code review comments
3. Preserve the threading/nesting of comment replies
4. Show the file and line number context for code review comments
5. Use jq to parse the JSON responses from the GitHub API

${args ? 'Additional user input: ' + args : ''}
```

### 6.5 `/statusline` prompt (verbatim, `statusline.tsx:15-21`)

```
Create an ${AGENT_TOOL_NAME} with subagent_type "statusline-setup" and the prompt "${prompt}"
```

Where `prompt = args.trim() || 'Configure my statusLine from my shell PS1 configuration'`.

### 6.6 `/review` prompt (verbatim, `review.ts:9-31`)

```
      You are an expert code reviewer. Follow these steps:

      1. If no PR number is provided in the args, run `gh pr list` to show open PRs
      2. If a PR number is provided, run `gh pr view <number>` to get PR details
      3. Run `gh pr diff <number>` to get the diff
      4. Analyze the changes and provide a thorough code review that includes:
         - Overview of what the PR does
         - Analysis of code quality and style
         - Specific suggestions for improvements
         - Any potential issues or risks

      Keep your review concise but thorough. Focus on:
      - Code correctness
      - Following project conventions
      - Performance implications
      - Test coverage
      - Security considerations

      Format your review with clear sections and bullet points.

      PR number: ${args}
```

### 6.7 `/security-review` prompt

Verbatim markdown body from `security-review.ts:6-196`. Frontmatter:

```yaml
---
allowed-tools: Bash(git diff:*), Bash(git status:*), Bash(git log:*), Bash(git show:*), Bash(git remote show:*), Read, Glob, Grep, LS, Task
description: Complete a security review of the pending changes on the current branch
---
```

Body sections (preserved exactly as authored):
- `GIT STATUS:`, `FILES MODIFIED:`, `COMMITS:`, `DIFF CONTENT:` blocks (with `!\`...\`` substitutions for `git status`, `git diff --name-only origin/HEAD...`, `git log --no-decorate origin/HEAD...`, `git diff origin/HEAD...`)
- `OBJECTIVE`, `CRITICAL INSTRUCTIONS` (4 numbered points incl. EXCLUSIONS list)
- `SECURITY CATEGORIES TO EXAMINE` — five subsections (Input Validation, Authentication & Authorization, Crypto & Secrets Management, Injection & Code Execution, Data Exposure)
- `ANALYSIS METHODOLOGY` — three phases (Repository Context Research, Comparative Analysis, Vulnerability Assessment)
- `REQUIRED OUTPUT FORMAT` with worked example `# Vuln 1: XSS: \`foo.py:42\``
- `SEVERITY GUIDELINES` (HIGH / MEDIUM / LOW), `CONFIDENCE SCORING` (four bands)
- `FINAL REMINDER`, `FALSE POSITIVE FILTERING` block (17 hard exclusions, 12 precedents, 4 signal-quality criteria, 3-band confidence)
- `START ANALYSIS:` 3-step plan (sub-task identify → parallel filter sub-tasks → drop confidence <8).

### 6.8 `/commit` prompt (verbatim, `commit.ts:20-54`)

```
${prefix}## Context

- Current git status: !`git status`
- Current git diff (staged and unstaged changes): !`git diff HEAD`
- Current branch: !`git branch --show-current`
- Recent commits: !`git log --oneline -10`

## Git Safety Protocol

- NEVER update the git config
- NEVER skip hooks (--no-verify, --no-gpg-sign, etc) unless the user explicitly requests it
- CRITICAL: ALWAYS create NEW commits. NEVER use git commit --amend, unless the user explicitly requests it
- Do not commit files that likely contain secrets (.env, credentials.json, etc). Warn the user if they specifically request to commit those files
- If there are no changes to commit (i.e., no untracked files and no modifications), do not create an empty commit
- Never use git commands with the -i flag (like git rebase -i or git add -i) since they require interactive input which is not supported

## Your task

Based on the above changes, create a single git commit:

1. Analyze all staged changes and draft a commit message:
   - Look at the recent commits above to follow this repository's commit message style
   - Summarize the nature of the changes (new feature, enhancement, bug fix, refactoring, test, docs, etc.)
   - Ensure the message accurately reflects the changes and their purpose (i.e. "add" means a wholly new feature, "update" means an enhancement to an existing feature, "fix" means a bug fix, etc.)
   - Draft a concise (1-2 sentences) commit message that focuses on the "why" rather than the "what"

2. Stage relevant files and create the commit using HEREDOC syntax:
\`\`\`
git commit -m "$(cat <<'EOF'
Commit message here.${commitAttribution ? `\n\n${commitAttribution}` : ''}
EOF
)"
\`\`\`

You have the capability to call multiple tools in a single response. Stage and create the commit using a single message. Do not use any other tools or do anything else. Do not send any other text or messages besides these tool calls.
```

Where `prefix = (USER_TYPE === 'ant' && isUndercover()) ? getUndercoverInstructions() + '\n' : ''`, and `commitAttribution = getAttributionTexts().commit`.

### 6.9 `/commit-push-pr` prompt (verbatim, `commit-push-pr.ts:57-105`)

```
${prefix}## Context

- `SAFEUSER`: ${safeUser}
- `whoami`: ${username}
- `git status`: !`git status`
- `git diff HEAD`: !`git diff HEAD`
- `git branch --show-current`: !`git branch --show-current`
- `git diff ${defaultBranch}...HEAD`: !`git diff ${defaultBranch}...HEAD`
- `gh pr view --json number 2>/dev/null || true`: !`gh pr view --json number 2>/dev/null || true`

## Git Safety Protocol

- NEVER update the git config
- NEVER run destructive/irreversible git commands (like push --force, hard reset, etc) unless the user explicitly requests them
- NEVER skip hooks (--no-verify, --no-gpg-sign, etc) unless the user explicitly requests it
- NEVER run force push to main/master, warn the user if they request it
- Do not commit files that likely contain secrets (.env, credentials.json, etc)
- Never use git commands with the -i flag (like git rebase -i or git add -i) since they require interactive input which is not supported

## Your task

Analyze all changes that will be included in the pull request, making sure to look at all relevant commits (NOT just the latest commit, but ALL commits that will be included in the pull request from the git diff ${defaultBranch}...HEAD output above).

Based on the above changes:
1. Create a new branch if on ${defaultBranch} (use SAFEUSER from context above for the branch name prefix, falling back to whoami if SAFEUSER is empty, e.g., `username/feature-name`)
2. Create a single commit with an appropriate message using heredoc syntax${commitAttribution ? `, ending with the attribution text shown in the example below` : ''}:
\`\`\`
git commit -m "$(cat <<'EOF'
Commit message here.${commitAttribution ? `\n\n${commitAttribution}` : ''}
EOF
)"
\`\`\`
3. Push the branch to origin
4. If a PR already exists for this branch (check the gh pr view output above), update the PR title and body using `gh pr edit` to reflect the current diff${addReviewerArg}. Otherwise, create a pull request using `gh pr create` with heredoc syntax for the body${reviewerArg}.
   - IMPORTANT: Keep PR titles short (under 70 characters). Use the body for details.
\`\`\`
gh pr create --title "Short, descriptive title" --body "$(cat <<'EOF'
## Summary
<1-3 bullet points>

## Test plan
[Bulleted markdown checklist of TODOs for testing the pull request...]${changelogSection}${effectivePrAttribution ? `\n\n${effectivePrAttribution}` : ''}
EOF
)"
\`\`\`

You have the capability to call multiple tools in a single response. You MUST do all of the above in a single message.${slackStep}

Return the PR URL when you're done, so the user can see it.
```

Default substitutions (from commit-push-pr.ts:31-48):
- `reviewerArg = ' and \`--reviewer anthropics/claude-code\`'`
- `addReviewerArg = ' (and add \`--add-reviewer anthropics/claude-code\`)'`
- `changelogSection = '\n\n## Changelog\n<!-- CHANGELOG:START -->\n[If this PR contains user-facing changes, add a changelog entry here. Otherwise, remove this section.]\n<!-- CHANGELOG:END -->'`
- `slackStep` (verbatim): `\n\n5. After creating/updating the PR, check if the user's CLAUDE.md mentions posting to Slack channels. If it does, use ToolSearch to search for "slack send message" tools. If ToolSearch finds a Slack tool, ask the user if they'd like you to post the PR URL to the relevant Slack channel. Only post if the user confirms. If ToolSearch returns no results or errors, skip this step silently—do not mention the failure, do not attempt workarounds, and do not try alternative approaches.`

When `USER_TYPE === 'ant' && isUndercover()`: `prefix = getUndercoverInstructions() + '\n'`; `reviewerArg`, `addReviewerArg`, `changelogSection`, `slackStep` all set to `''`.

When user provides args, the suffix `\n\n## Additional instructions from user\n\n${trimmedArgs}` is appended (commit-push-pr.ts:127-131).

### 6.10 `/insights` lazy shim (verbatim, `commands.ts:189-202`)

```typescript
const usageReport: Command = {
  type: 'prompt',
  name: 'insights',
  description: 'Generate a report analyzing your Claude Code sessions',
  contentLength: 0,
  progressMessage: 'analyzing your sessions',
  source: 'builtin',
  async getPromptForCommand(args, context) {
    const real = (await import('./commands/insights.js')).default
    if (real.type !== 'prompt') throw new Error('unreachable')
    return real.getPromptForCommand(args, context)
  },
}
```

### 6.11 `createMovedToPluginCommand` ANT branch (verbatim, `createMovedToPluginCommand.ts:44-58`)

```
This command has been moved to a plugin. Tell the user:

1. To install the plugin, run:
   claude plugin install ${pluginName}@claude-code-marketplace

2. After installation, use /${pluginName}:${pluginCommand} to run this command

3. For more information, see: https://github.com/anthropics/claude-code-marketplace/blob/main/${pluginName}/README.md

Do not attempt to run the command. Simply inform the user about the plugin installation.
```

---

## 7. Side Effects & I/O

| Command | Side effect | Owning subsystem |
|---|---|---|
| `/init`, `/init-verifiers` | `maybeMarkProjectOnboardingComplete()` (init only); model writes files via subsequent tool calls | spec 02 (settings) |
| `/branch` | Writes `<projects>/<project>/<sessionId>.jsonl` with `mode 0o600` | session storage |
| `/clear` | Wipes conversation; spec 04 turn pipeline | spec 04 |
| `/compact` | Replaces messages, sets `lastSummarizedMessageId`, `suppressCompactWarning`, `runPostCompactCleanup`, `notifyCompaction` | spec 07 |
| `/login` | OAuth flow; sets API key; refreshes GrowthBook, policy limits, remote-managed settings; trusted-device enroll | spec 25 |
| `/logout` | `flushTelemetry`, `removeApiKey`, `getSecureStorage().delete()`, multi-cache wipe; saveGlobalConfig may reset `hasCompletedOnboarding` etc. | spec 25 |
| `/permissions` | Mutates `toolPermissionContext` | spec 09 |
| `/sandbox` | Mutates SandboxManager state, settings file | spec 09 |
| `/hooks` | View only (no settings mutation in command itself) | spec 02 / 09 |
| `/advisor` | Writes `advisorModel` to user settings | spec 02 |
| `/model` | Mutates `mainLoopModel` in app state | spec 03 |
| `/commit` | Tool calls Bash to run git; allowed via `ALLOWED_TOOLS` injection | spec 10 |
| `/commit-push-pr` | Tool calls Bash + gh + Slack MCP | specs 10, 16 |

Top-level `process.env` reads: every command that has an `isEnabled` referencing `process.env.*` reads at command-list-build time (NOT at module import — `isEnabled` is a function).

---

## 8. Feature Flags & Variants

This sub-file's commands are NOT feature-flag–gated at registration. They may, however, observe flags at runtime:

| Site | Flag | Effect |
|---|---|---|
| `branch/index.ts:8` | `FORK_SUBAGENT` | Removes `'fork'` alias when standalone /fork exists |
| `init.ts:230-232,247-249` | `NEW_INIT` | Switches description and prompt body |
| `compact/compact.ts:35-37` | `REACTIVE_COMPACT` | Routes through reactive path |
| `compact/compact.ts:67` | `PROMPT_CACHE_BREAK_DETECTION` | Calls `notifyCompaction` |

ANT-only paths within universally-registered commands:
- `/cost` — visibility (`isHidden`) inverts for ANTs (`cost/index.ts:14-17`).
- `/cost` — appends ANT-only line to subscriber output (`cost/cost.ts:18-20`).
- `/feedback` — ANT users see no /feedback (`feedback/index.ts:14`).
- `/files` — only enabled for ANTs (`files/index.ts:6`).
- `/tag` — only enabled for ANTs (`tag/index.ts:6`).
- `/commit`, `/commit-push-pr` — ANT-undercover prefix, slack/changelog elision.
- `/init` — `NEW_INIT` description/prompt path requires `USER_TYPE === 'ant' || CLAUDE_CODE_NEW_INIT`.

3P-aware (suppressed for Bedrock/Vertex/Foundry):
- `/login`, `/logout` — registry-level via `!isUsing3PServices()` (`commands.ts:337`).
- `/feedback` — additionally checks `CLAUDE_CODE_USE_BEDROCK / VERTEX / FOUNDRY`.

---

## 9. Error Handling & Edge Cases

Per command (sampled):
- `/branch` — `'No conversation to branch'` and `'No messages to branch'` (branch.ts:81,113); transcript read failure same path.
- `/compact` — abort signal: `'Compaction canceled.'`; otherwise `Error during compaction: ${error}` after `logError`.
- `/advisor` — `'Invalid advisor model: ${error}'`, `'Unknown model: ${arg} (${resolvedModel})'`, `'The model ${arg} (${resolvedModel}) cannot be used as an advisor'`.
- `/mcp` — toggle: `'All MCP servers are already ${enabled|disabled}'`, `'MCP server "${target}" not found'`, `'Enabled/Disabled ${n} MCP server(s)'`, `'MCP server "${target}" enabled/disabled'`.
- `/insights` shim — `throw new Error('unreachable')` if loaded module is not a prompt command.

For `prompt` commands, error handling beyond the prompt body is the responsibility of the model and the underlying tools (spec 04).

---

## 10. Telemetry & Observability

Sampled events emitted by commands in this file:
- `/branch` — `tengu_conversation_forked` with `{message_count, has_custom_title}` (branch.ts:254).
- `/brief` (in 21c) — `tengu_brief_mode_toggled`.
- `/hooks` — `tengu_hooks_command` (hooks/hooks.tsx:7).
- `/model` — `tengu_model_command_menu` with `{action: 'cancel'|...}` (model/model.tsx).
- `/feedback` — see implementation file.
- `/extra-usage`, `/upgrade`, `/passes`, `/usage` — touch policy/auth services and emit their own events.
- `/insights` shim — none directly; the lazy-imported module emits its own.

No metrics/OTel spans at the command-shim layer; the LLM turn emits standard turn-pipeline telemetry (spec 04).

---

## 11. Reimplementation Checklist

A reimplementer of public commands must preserve:

- [ ] Static order in `COMMANDS()` matches `commands.ts:258-319` exactly — typeahead order is determined by registration order (20 §5.3).
- [ ] Each command's `name`, `aliases`, `argumentHint`, `description`, `availability`, `isEnabled`, `isHidden`, `immediate`, `supportsNonInteractive`, `disableNonInteractive`, `kind` match the cited file byte-for-byte.
- [ ] `prompt`-kind commands return `Promise<ContentBlockParam[]>` of length 1, type `'text'` (no command in this catalog returns multi-block prompts).
- [ ] `local` and `local-jsx` commands MUST lazy-load via `load: () => import('./<file>.js')`. `version`, `advisor`, `bridge-kick` use `Promise.resolve({ call })` instead — preserve.
- [ ] `executeShellCommandsInPrompt(content, context, '<command-name>')` is called for every `prompt` command that has `!\`...\`` substitutions in its body — must inject `alwaysAllowRules.command = ALLOWED_TOOLS` into the per-call permission context (commit.ts:65-90, commit-push-pr.ts:133-152, security-review.ts:215-234).
- [ ] `/login`/`/logout` registration is gated by `!isUsing3PServices()` at the registry, not by per-command `isEnabled`.
- [ ] `/cost` `isHidden` checks `process.env.USER_TYPE === 'ant'` BEFORE `isClaudeAISubscriber()` (so ANT subs still see cost).
- [ ] `/insights` MUST stay as a lazy shim with `contentLength: 0` and `import('./commands/insights.js')` — moving the heavy module to the static import block reintroduces the 113KB startup cost.
- [ ] `/init` description and prompt body switch on `feature('NEW_INIT') && (USER_TYPE === 'ant' || CLAUDE_CODE_NEW_INIT)` — both checks needed.
- [ ] `/branch` aliases conditional: `feature('FORK_SUBAGENT') ? [] : ['fork']`.
- [ ] `/compact` checks `feature('REACTIVE_COMPACT')` and `feature('PROMPT_CACHE_BREAK_DETECTION')` at runtime.
- [ ] `LocalCommandResult` of variant `'compact'` carries `displayText` rendered via `chalk.dim('Compacted ' + ...)`.
- [ ] `createMovedToPluginCommand` always returns the install-prompt for ANT users; non-ANT users get `getPromptWhileMarketplaceIsPrivate(args, context)`.
- [ ] `/login` post-success refresh order matters: `resetCostState`, `refreshRemoteManagedSettings` (non-blocking), `refreshPolicyLimits` (non-blocking), `resetUserCache` BEFORE `refreshGrowthBookAfterAuthChange`, then `clearTrustedDeviceToken` then re-enroll.
- [ ] `/logout` MUST `flushTelemetry()` before `removeApiKey()` (org-data leakage prevention).
- [ ] `/sandbox` description getter must reproduce icon precedence: `figures.warning` (deps missing) > `figures.tick` (enabled) > `figures.circle` (disabled), and the trailing `(⏎ to configure)` literal.
- [ ] `/extra-usage` and `/context` register TWO entries — interactive `local-jsx` and non-interactive `local` — sharing a name; the gating ensures only one is selected per session.
- [ ] `/statusline` allowedTools include `Read(~/**)` and `Edit(~/.claude/settings.json)` (literal patterns).
- [ ] `/commit` and `/commit-push-pr` `ALLOWED_TOOLS` arrays are exact — match `commit.ts:6-10` and `commit-push-pr.ts:10-24` verbatim.
- [ ] `commit-push-pr` undercover branch sets `reviewerArg`, `addReviewerArg`, `changelogSection`, `slackStep` to `''`.

---

## 12. Open Questions / Unknowns

1. **`/insights` prompt body** — read fully but too large to inline (~3200 lines). Spec defers to a future 21d if the corpus must be bit-exact.
2. **`createMovedToPluginCommand` cleanup** — comment says "Once the marketplace is public, this parameter and the fallback logic can be removed" (createMovedToPluginCommand.ts:13-14). When marketplace becomes public, both `pr-comments` and `security-review` could become pure plugin pointers. Keep the fallback path documented for now.
3. **`statusline.tsx` and `plugin/index.tsx`** ship as compiled output (visible source map base64 in the file). The `index.tsx` originals are reconstructable from the inline sourcesContent — done in §3.37 and §3.47.
4. **`AGENT_TOOL_NAME` literal value** — referenced from `tools/AgentTool/constants.js`. Spec 14 to enumerate.
5. **`ResumeEntrypoint = 'fork'`** — used by `/branch` to pass `'fork'` into `context.resume`. Documented in spec 20 §3.5; verify spec 41 (session state) handles this entrypoint correctly.
6. **`undercover` mode trigger** — `isUndercover()` from `utils/undercover.ts` not enumerated here; affects commit*, commit-push-pr*, possibly others. Spec 02/26 to detail.
