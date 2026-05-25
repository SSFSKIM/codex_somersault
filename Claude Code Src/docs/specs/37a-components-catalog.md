# 37a — Components Catalog

## §0 Scope

This spec is a catalog companion to spec **37 (Ink UI shell)**. Spec 37 covers
the high-level shell, screen registry, REPL mounting, and the FullscreenLayout
rendering loop. This spec enumerates every individual component file under
`src/components/` that is not already cited in another spec — the long tail of
~315 component files from the Phase 9 coverage audit, plus the additional
components/files in `src/components/` that the audit didn't bucket separately
(e.g., `App.tsx`, `Spinner.tsx`, `Messages.tsx`, `Stats.tsx`, `Onboarding.tsx`,
`PromptInput/PromptInput.tsx`, `LogSelector.tsx`, etc.).

Coverage approach: per-file entries, 2–3 lines each, grouped by submodule
(top-level dir under `src/components/`). Sizes shown in KB are the byte sizes
of each `.ts`/`.tsx` source file. Cross-spec edges are noted only when a
component clearly belongs to a sibling subsystem (permission system → spec 09,
MCP UI → spec 23, Skills → spec 17, etc.).

**Audit deltas:**
- Coverage doc lists ~315 suspect residuals under `src/components/`.
- Filesystem inventory (`find … | wc -l`) shows **389** total component files.
- Some non-residual files (already cited by basename in spec 37) are also
  included here for completeness when they are non-trivial siblings of
  residual entries (e.g., `Message.tsx`, `Messages.tsx`, `Spinner.tsx`,
  `PromptInput/PromptInput.tsx`, `App.tsx`, `Onboarding.tsx`). They are
  flagged with **(already cited)** so spec 37's status is not regressed.

**Conventions:**
- All components use Ink primitives (`Box`, `Text`, `RawAnsi`, `NoSelect`)
  from `src/ink.js` (the barrel re-export of `src/ink/components/*`).
- All themed colors flow through `src/utils/theme.ts` keys (e.g., `keyof Theme`).
- "Flag gates" lists `feature('…')` calls observed in the file's first 30
  lines or implied by the path (e.g., `LogoV2/ChannelsNotice.tsx` is gated
  behind a flag confirmed by reading `LogoV2.tsx`).
- File-size signal: ≥80KB suggests a complex screen (probably its own
  candidate spec section); ≤2KB signals a leaf utility/wrapper.


## §1 Submodule Index

Top-level subdirectories under `src/components/` ordered by file count.

| Submodule | File count | Largest file (KB) | Purpose |
|---|---:|---:|---|
| `src/components/` (root) | 110 | 355 (PromptInput.tsx is in subdir; root max is 271 Settings/Config; flat root max is `Stats.tsx` 152KB) | Top-level screens, dialogs, pickers, message wrappers, onboarding |
| `src/components/messages/` | 40 | 79 (`SystemTextMessage.tsx`) | Per-role message renderers (assistant text/thinking/tool-use, user text/bash/image/etc.) |
| `src/components/permissions/` | 49 | 121 (`ExitPlanModePermissionRequest.tsx`) | Permission-prompt UI for each tool family + permission-rules editor |
| `src/components/PromptInput/` | 17 | 355 (`PromptInput.tsx`) | The composer — input field, footer, suggestions, hints, mode indicators |
| `src/components/agents/` | 22 | 70 (`AgentsMenu.tsx`) | Agent list/edit/create-wizard, color/model/tool selectors |
| `src/components/LogoV2/` | 14 | 75 (`LogoV2.tsx`) | Welcome-screen logo, animated Clawd, feed columns, upsell notices |
| `src/components/design-system/` | 16 | 41 (`Tabs.tsx`, `FuzzyPicker.tsx`) | Reusable primitives: themed Box/Text, Dialog, Pane, Tabs, ProgressBar, FuzzyPicker |
| `src/components/mcp/` | 13 | 179 (`ElicitationDialog.tsx`) | MCP server menus, tool views, capability section, elicitation dialog |
| `src/components/Spinner/` | 12 | 88 (`../Spinner.tsx` root + 42 `SpinnerAnimationRow.tsx`) | Spinner glyph/animation, glimmer, teammate spinner tree |
| `src/components/tasks/` | 11 | 116 (`BackgroundTasksDialog.tsx`) | Async-task UI: background tasks dialog, per-task detail dialogs, progress |
| `src/components/CustomSelect/` | 10 | 115 (`select.tsx`) | Select-input primitive used in pickers (multi/single, navigation hooks) |
| `src/components/FeedbackSurvey/` | 9 | 48 (`useFeedbackSurvey.tsx`) | Post-session feedback survey, transcript-share prompt |
| `src/components/permissions/AskUserQuestionPermissionRequest/` | 7 | 82 (`AskUserQuestionPermissionRequest.tsx`) | Multi-question permission UI (questions navigation, preview) |
| `src/components/permissions/rules/` | 8 | 119 (`PermissionRuleList.tsx`) | Permission rules editor (add/remove rules, workspace dirs, recent denials) |
| `src/components/agents/new-agent-creation/wizard-steps/` | 11 | 35 (`ConfirmStep.tsx`) | Per-step UI for create-agent wizard |
| `src/components/hooks/` | 6 | 54 (`HooksConfigMenu.tsx`) | Settings UI for the Hooks subsystem |
| `src/components/sandbox/` | 5 | 30 (`SandboxSettings.tsx`) | Sandbox config tabs (config/dependencies/overrides/doctor) |
| `src/components/diff/` | 3 | 43 (`DiffDialog.tsx`) | Multi-file diff dialog/list/detail |
| `src/components/shell/` | 4 | 14 (`OutputLine.tsx`) | Bash output line rendering, expand context, time display |
| `src/components/teams/` | 2 | 95 (`TeamsDialog.tsx`) | Multi-agent teams dialog and status |
| `src/components/wizard/` | 5 | 19 (`WizardProvider.tsx`) | Generic wizard framework (used by agent wizard) |
| `src/components/messages/UserToolResultMessage/` | 8 | 16 (`UserToolSuccessMessage.tsx`) | Tool-result variants (success, error, reject, canceled, plan-rejected) |
| `src/components/permissions/*/` (per-tool dirs) | ~15 | varies | One subdir per permission-prompted tool: BashPermissionRequest, FileEdit, FileWrite, Filesystem, NotebookEdit, PowerShell, SedEdit, Skill, WebFetch, ComputerUse, EnterPlanMode, ExitPlanMode, FilePermissionDialog |
| `src/components/memory/` | 2 | 48 (`MemoryFileSelector.tsx`) | Memory file picker, memory update notification |
| `src/components/skills/` | 1 | 27 (`SkillsMenu.tsx`) | Skills picker menu |
| `src/components/grove/` | 1 | 50 (`Grove.tsx`) | Grove (worktree) UI |
| `src/components/ui/` | 3 | 39 (`TreeSelect.tsx`) | Generic UI primitives (TreeSelect, OrderedList) |
| `src/components/Settings/` | 4 | 271 (`Config.tsx`) | Settings screen + Status/Usage/Settings entry views |
| `src/components/StructuredDiff/` | 2 | 57 (`Fallback.tsx`) | Fallback path for syntax-highlighted diffs |
| `src/components/HighlightedCode/` | 1 | 16 (`Fallback.tsx`) | Fallback for syntax-highlighted code blocks |
| `src/components/HelpV2/` | 3 | 21 (`HelpV2.tsx`) | Modern help screen (commands, general) |
| `src/components/TrustDialog/`, `ManagedSettingsSecurityDialog/`, `DesktopUpsell/`, `LspRecommendation/`, `ClaudeCodeHint/` | 2 each | varies | Single-purpose dialog clusters |

The "root" submodule (`src/components/*.tsx` directly) holds the top-level
screens that the REPL mounts as siblings under `FullscreenLayout`: `App.tsx`,
`Onboarding.tsx`, `Spinner.tsx`, `Messages.tsx`, `Message.tsx`, `MessageRow.tsx`,
`MessageSelector.tsx`, `VirtualMessageList.tsx`, `Stats.tsx`, `StatusLine.tsx`,
`ScrollKeybindingHandler.tsx`, `LogSelector.tsx`, `ResumeTask.tsx`,
`ModelPicker.tsx`, `ThemePicker.tsx`, `OutputStylePicker.tsx`, `Feedback.tsx`,
`GlobalSearchDialog.tsx`, `HistorySearchDialog.tsx`, `QuickOpenDialog.tsx`,
`TaskListV2.tsx`, plus ~40 dialogs and ~30 leaf widgets.


## §2 Component Entries

Sizes are shown after the path. All components use Ink primitives unless
otherwise noted.

### §2.1 `src/components/` (root)

#### Top-level screens / shells
- `App.tsx` (5KB) — Root component mounted by REPL; conditionally renders Onboarding vs main UI; **(already cited in spec 37)**.
- `Messages.tsx` (147KB) — The transcript renderer. Hosts the message list, scroll, ctrl+o expand state. **Cross-spec:** spec 04 (turn pipeline output). **(already cited)**.
- `Message.tsx` (79KB) — Per-message dispatcher: switches on message type and renders one of the `messages/*` components. **(already cited)**.
- `MessageRow.tsx` (48KB) — Wrapper that renders a single message with margin/dot/avatar.
- `MessageSelector.tsx` (115KB) — Interactive picker for selecting a previous message (used by `/resume`-like flows).
- `MessageResponse.tsx` (7KB) — Common shell for assistant responses (gutter dot + content layout).
- `MessageTimestamp.tsx` (5KB) — Renders relative/absolute message timestamp.
- `MessageModel.tsx` (4KB) — Tag rendering the model name next to a message.
- `messageActions.tsx` (55KB) — Actions menu attached to each message (copy, edit, retry, etc.). Owns `MessageActionsSelectedContext`.
- `VirtualMessageList.tsx` (149KB) — Virtualized scroll container for the transcript. **Cross-spec:** spec 41 (session/state).
- `FullscreenLayout.tsx` (85KB) — Top-level layout that splits header / messages / prompt-input regions. **(already cited in 37)**.
- `Onboarding.tsx` (32KB) — First-run onboarding screen; calls into trust dialog, theme/lang/model pickers. **(already cited)**.
- `Spinner.tsx` (88KB) — Activity spinner shown while a turn is in flight (root file, see §2.10 for sub-files). **(already cited)**.
- `Stats.tsx` (152KB) — Usage/cost/token statistics screen (`/status` and `/cost` rendering). **Cross-spec:** spec 06.
- `StatusLine.tsx` (49KB) — Bottom status bar (cost, model, mode flags). **(already cited)**.
- `LogSelector.tsx` (200KB) — **Largest non-bundle component.** Log/transcript browsing screen with filtering, preview, multi-select. Probably warrants its own dedicated spec section (see §3, §5).
- `ScrollKeybindingHandler.tsx` (149KB) — Page-up/down/end/home/ctrl-o keybinding handler bound to the message list.
- `ResumeTask.tsx` (38KB) — Resume-task picker UI shown by `/resume`-style commands. **Cross-spec:** spec 41.
- `Feedback.tsx` (87KB) — Standalone `/feedback` form (separate from the post-session survey under FeedbackSurvey/).
- `GlobalSearchDialog.tsx` (44KB) — Global search across history.
- `HistorySearchDialog.tsx` (20KB) — Inline history search in the prompt input.
- `QuickOpenDialog.tsx` (29KB) — Quick-open picker (file/path search). **Cross-spec:** spec 11 (file tools).
- `TaskListV2.tsx` (50KB) — Task-tracker UI. **Cross-spec:** spec 15 (task tools).

#### Pickers
- `ModelPicker.tsx` (54KB) — Model selection screen (Sonnet/Opus/Haiku, 1m flag, etc.). **Cross-spec:** spec 22.
- `ThemePicker.tsx` (36KB) — Theme picker.
- `OutputStylePicker.tsx` (13KB) — Output style picker. **Cross-spec:** spec 38.
- `LanguagePicker.tsx` (9KB) — UI language selection (intl).
- `SearchBox.tsx` (9KB) — Reusable searchable text input wrapper.

#### Dialogs (one-shot modals)
- `AutoModeOptInDialog.tsx` (13KB) — Opt-in for auto-mode (auto-accepting permissions).
- `BridgeDialog.tsx` (34KB) — IDE bridge enable/connect dialog. **Cross-spec:** spec 34.
- `BypassPermissionsModeDialog.tsx` (9KB) — Confirm switch into `--dangerously-skip-permissions` mode. **Cross-spec:** spec 09.
- `ChannelDowngradeDialog.tsx` (8KB) — Notification when version channel is downgraded.
- `ClaudeMdExternalIncludesDialog.tsx` (14KB) — Confirm import of external `CLAUDE.md` includes.
- `CostThresholdDialog.tsx` (4KB) — Cost-limit reached confirmation.
- `DesktopHandoff.tsx` (19KB) — Hand-off prompt to desktop client.
- `DevChannelsDialog.tsx` (9KB) — Internal dev-channel switcher.
- `ExportDialog.tsx` (19KB) — Conversation export dialog (md/json/png).
- `IdeAutoConnectDialog.tsx` (13KB) — Auto-connect to IDE.
- `IdeOnboardingDialog.tsx` (16KB) — IDE onboarding/install.
- `IdleReturnDialog.tsx` (10KB) — Resume after idle timeout.
- `InvalidConfigDialog.tsx` (15KB) — Settings parse errors.
- `InvalidSettingsDialog.tsx` (7KB) — Settings validation errors.
- `MCPServerApprovalDialog.tsx` (12KB) — MCP server approval. **Cross-spec:** spec 23.
- `MCPServerDesktopImportDialog.tsx` (21KB) — Import MCP servers from desktop. **Cross-spec:** spec 23.
- `MCPServerDialogCopy.tsx` (2KB) — Static copy strings for MCP dialogs.
- `MCPServerMultiselectDialog.tsx` (16KB) — Choose which MCP servers to enable. **Cross-spec:** spec 23.
- `RemoteEnvironmentDialog.tsx` (34KB) — Remote-environment selection. **Cross-spec:** spec 35.
- `TeleportRepoMismatchDialog.tsx` (13KB) — Teleport repo mismatch warning. **Cross-spec:** spec 35 (teleport ≈ remote).
- `WorkflowMultiselectDialog.tsx` (14KB) — Multi-select for workflow scripts (`feature('WORKFLOW_SCRIPTS')`).
- `WorktreeExitDialog.tsx` (35KB) — Confirm exit when on worktree.

#### Banners / hints / status
- `AgentProgressLine.tsx` (14KB) — One-line progress display for an agent (used by Task tool); shows tokens, last tool, error state.
- `ApproveApiKey.tsx` (11KB) — Inline approval of pasted API key.
- `AutoUpdater.tsx` (31KB) — Background updater UI.
- `AutoUpdaterWrapper.tsx` (12KB) — Conditional wrapper around AutoUpdater (chooses native/package-manager path).
- `AwsAuthStatusBox.tsx` (10KB) — AWS auth status display. **Cross-spec:** spec 25.
- `BashModeProgress.tsx` (6KB) — Inline progress while bash-mode (`!`) is active.
- `ClaudeCodeHint/PluginHintMenu.tsx` (9KB) — Plugin install hint menu. **Cross-spec:** spec 28.
- `ClaudeInChromeOnboarding.tsx` (12KB) — Onboarding for Chrome extension integration.
- `ClickableImageRef.tsx` (7KB) — Inline image reference (clickable to open).
- `CompactSummary.tsx` (14KB) — Compaction summary line in transcript. **Cross-spec:** spec 07.
- `ConfigurableShortcutHint.tsx` (5KB) — Renders a keybinding hint that respects user keybinding remap.
- `ConsoleOAuthFlow.tsx` (80KB) — Console OAuth login flow. **Cross-spec:** spec 25.
- `ContextSuggestions.tsx` (6KB) — Suggestions chip strip above the input. **Cross-spec:** spec 05.
- `ContextVisualization.tsx` (76KB) — Visualizes context-window usage per section. **Cross-spec:** spec 05/07.
- `CoordinatorAgentStatus.tsx` (36KB) — Coordinator/multi-agent status pane. **Cross-spec:** spec 30.
- `CtrlOToExpand.tsx` (6KB) — Inline "ctrl-o to expand" hint shown on truncated content.
- `DevBar.tsx` (5KB) — Top dev/debug bar shown when `USER_TYPE === 'ant'`.
- `DiagnosticsDisplay.tsx` (13KB) — LSP diagnostics inline display. **Cross-spec:** spec 24.
- `EffortCallout.tsx` (25KB) — Callout for thinking-effort changes.
- `EffortIndicator.ts` (1KB) — Tiny helper that returns a string label for effort level.
- `ExitFlow.tsx` (4KB) — Wraps exit handling/confirmation.
- `FallbackToolUseErrorMessage.tsx` (13KB) — Default rendering for unknown tool errors.
- `FallbackToolUseRejectedMessage.tsx` (2KB) — Default rendering for unknown rejected tool uses.
- `FastIcon.tsx` (5KB) — Lightning icon used by fast-mode UI.
- `FilePathLink.tsx` (3KB) — Renders a clickable file path with hyperlink protocol.
- `IdeStatusIndicator.tsx` (6KB) — IDE connection indicator.
- `InterruptedByUser.tsx` (2KB) — "Interrupted by user" inline marker.
- `KeybindingWarnings.tsx` (10KB) — Warning when keybindings conflict.
- `MemoryUsageIndicator.tsx` (4KB) — Memory budget bar in status. **Cross-spec:** spec 40.
- `NativeAutoUpdater.tsx` (27KB) — Native binary updater path.
- `NotebookEditToolUseRejectedMessage.tsx` (8KB) — Specific rejected message for notebook edit.
- `OffscreenFreeze.tsx` (6KB) — Wrapper that freezes a subtree's render when offscreen.
- `Passes/Passes.tsx` (27KB) — Pass-tokens UI (Anthropic billing concept).
- `PackageManagerAutoUpdater.tsx` (14KB) — npm/pnpm/yarn updater path.
- `PrBadge.tsx` (8KB) — Inline GitHub PR badge.
- `PressEnterToContinue.tsx` (2KB) — Trivial "press enter to continue" prompt.
- `RemoteCallout.tsx` (10KB) — Remote-session callout banner. **Cross-spec:** spec 35.
- `SandboxViolationExpandedView.tsx` (11KB) — Sandbox-violation detail view.
- `SentryErrorBoundary.ts` (0.5KB) — Error boundary that reports to Sentry.
- `SessionBackgroundHint.tsx` (13KB) — Hint about session backgrounding. **Cross-spec:** spec 41.
- `SessionPreview.tsx` (19KB) — Preview card for a session in resume picker. **Cross-spec:** spec 41.
- `ShowInIDEPrompt.tsx` (17KB) — "Show diff in IDE" prompt.
- `SkillImprovementSurvey.tsx` (15KB) — Inline survey after a skill runs. **Cross-spec:** spec 17.
- `StatusNotices.tsx` (6KB) — Collection of small status banners.
- `StructuredDiff.tsx` (25KB) — Structured (line-by-line) diff renderer. Has `StructuredDiff/Fallback.tsx` (57KB) for non-NAPI fallback path and `StructuredDiff/colorDiff.ts` (1KB) for color helpers.
- `StructuredDiffList.tsx` (4KB) — Wrapper rendering a list of `StructuredDiff` hunks.
- `TagTabs.tsx` (21KB) — Tabs navigated by tag; used in Settings.
- `TeammateViewHeader.tsx` (7KB) — Header shown in teammate (subagent) view. **Cross-spec:** spec 30.
- `TeleportError.tsx` (19KB) — Teleport error display. **Cross-spec:** spec 35.
- `TeleportProgress.tsx` (16KB) — Teleport progress display.
- `TeleportResumeWrapper.tsx` (15KB) — Wraps resume flow when a teleport session is detected.
- `TeleportStash.tsx` (16KB) — Teleport stash UI.
- `TextInput.tsx` (21KB) — Standard one-line text input (wraps BaseTextInput).
- `BaseTextInput.tsx` (19KB) — Lower-level controlled-cursor text input used by TextInput and VimTextInput.
- `VimTextInput.tsx` (16KB) — Vim-mode wrapper around BaseTextInput. **Cross-spec:** spec 39.
- `ThinkingToggle.tsx` (18KB) — Toggle for assistant-thinking visibility.
- `TokenWarning.tsx` (21KB) — Context-window warning banner.
- `ToolUseLoader.tsx` (5KB) — Generic spinner row for in-flight tool uses.
- `ValidationErrorsList.tsx` (20KB) — Renders a list of zod validation errors.
- `Markdown.tsx` (28KB) — Markdown renderer (uses marked + Ink). **(already cited in spec 37 cousins).**
- `MarkdownTable.tsx` (48KB) — Markdown table renderer.
- `HighlightedCode.tsx` (18KB) — Syntax-highlighted code block (calls into NAPI, falls back to `HighlightedCode/Fallback.tsx`).
- `Settings/Settings.tsx` (19KB) — Top-level settings screen entry.
- `Settings/Status.tsx` (26KB) — Settings → Status sub-tab.
- `Settings/Usage.tsx` (40KB) — Settings → Usage sub-tab. **Cross-spec:** spec 06.
- `Settings/Config.tsx` (271KB) — Settings → Config sub-tab. **Largest dialog file.** Probably warrants its own dedicated spec section (see §3, §5).
- `TrustDialog/TrustDialog.tsx` (32KB) + `utils.ts` (7KB) — Workspace-trust dialog (per-folder trust prompt).

#### Trivial barrels & re-exports omitted
- `CustomSelect/index.ts` (≤120 bytes), `Spinner/index.ts` (~600 bytes), `wizard/index.ts` (~330 bytes), `mcp/index.ts` (~520 bytes), `agents/types.ts` (~915 bytes), `agents/utils.ts` (~530 bytes), `design-system/color.ts` (~850 bytes), `permissions/utils.ts` (~660 bytes), `Spinner/teammateSelectHint.ts` (64 bytes; classified as tiny-stub by audit).


### §2.2 `src/components/messages/`

Per-role/per-event message renderers. All consumed by `Message.tsx`'s switch.

- `AdvisorMessage.tsx` (14KB) — "Advisor" content blocks (used by review/advisor flows).
- `AssistantRedactedThinkingMessage.tsx` (3KB) — Renders redacted_thinking blocks.
- `AssistantTextMessage.tsx` (30KB) — Assistant text block, with API-error mapping (rate limits, invalid keys, OAuth, keychain locked).
- `AssistantThinkingMessage.tsx` (8KB) — Plain extended-thinking block.
- `AssistantToolUseMessage.tsx` (45KB) — Assistant `tool_use` block dispatcher (renders the tool-specific component for each call).
- `AttachmentMessage.tsx` (71KB) — Renders pasted/attached content (text, image, file). **Cross-spec:** spec 11.
- `CollapsedReadSearchContent.tsx` (78KB) — Collapsed view for grouped Read/Grep/Glob tool runs.
- `CompactBoundaryMessage.tsx` (2KB) — Inline marker between compaction boundaries. **Cross-spec:** spec 07.
- `GroupedToolUseContent.tsx` (8KB) — Wrapper for grouped tool uses.
- `HighlightedThinkingText.tsx` (15KB) — Highlighting helper for thinking text.
- `HookProgressMessage.tsx` (11KB) — Hook execution progress line. **Cross-spec:** spec 04.
- `PlanApprovalMessage.tsx` (25KB) — Plan-mode approval message. **Cross-spec:** spec 18.
- `RateLimitMessage.tsx` (17KB) — Rate-limit banner shown inline.
- `ShutdownMessage.tsx` (14KB) — Teammate-shutdown / process-exit message. **Cross-spec:** spec 30.
- `SystemAPIErrorMessage.tsx` (12KB) — Generic system-level API error.
- `SystemTextMessage.tsx` (79KB) — System-text rendering (largest in this submodule); handles many subtypes via discriminator.
- `TaskAssignmentMessage.tsx` (8KB) — Inline "task assigned to <agent>" message. **Cross-spec:** spec 30.
- `UserAgentNotificationMessage.tsx` (6KB) — Notification from a sub-agent to user. **Cross-spec:** spec 30.
- `UserBashInputMessage.tsx` (5KB) — User's bash-mode input echo.
- `UserBashOutputMessage.tsx` (4KB) — User-provided bash output (paste).
- `UserChannelMessage.tsx` (11KB) — Channel/pubsub message.
- `UserCommandMessage.tsx` (9KB) — Slash-command echo.
- `UserImageMessage.tsx` (6KB) — Inline user-supplied image.
- `UserLocalCommandOutputMessage.tsx` (15KB) — Local command output captured for context.
- `UserMemoryInputMessage.tsx` (7KB) — Memory-update input echo. **Cross-spec:** spec 40.
- `UserPlanMessage.tsx` (4KB) — Plan-mode user input. **Cross-spec:** spec 18.
- `UserPromptMessage.tsx` (15KB) — Standard user prompt rendering.
- `UserResourceUpdateMessage.tsx` (12KB) — MCP resource-update notification. **Cross-spec:** spec 23.
- `UserTeammateMessage.tsx` (24KB) — Message routed from another teammate. **Cross-spec:** spec 30.
- `UserTextMessage.tsx` (29KB) — Plain user text rendering.
- `nullRenderingAttachments.ts` (2KB) — Helper that suppresses rendering of certain attachment types.
- `teamMemCollapsed.tsx` (14KB) — Collapsed team-memory message.
- `teamMemSaved.ts` (0.7KB) — Helper for team-memory saved indicator.

#### `src/components/messages/UserToolResultMessage/`
- `UserToolResultMessage.tsx` (14KB) — Dispatcher for tool-result content.
- `UserToolSuccessMessage.tsx` (16KB) — Generic success rendering.
- `UserToolErrorMessage.tsx` (12KB) — Generic error rendering.
- `UserToolRejectMessage.tsx` (9KB) — Permission-rejected tool result.
- `RejectedToolUseMessage.tsx` (2KB) — Inline rejected tool-use marker.
- `RejectedPlanMessage.tsx` (3KB) — Plan-mode rejection marker.
- `UserToolCanceledMessage.tsx` (2KB) — User-canceled tool marker.
- `utils.tsx` (4KB) — Shared formatting helpers.


### §2.3 `src/components/permissions/`

Permission UI lives close to the permission system (spec 09) but is mounted
by spec 37's REPL.

- `PermissionDialog.tsx` (7KB) — Outer themed wrapper used by all permission requests (title + content).
- `PermissionPrompt.tsx` (37KB) — Top-level permission prompt entry (chooses the per-tool component). **Cross-spec:** spec 09.
- `PermissionRequest.tsx` (34KB) — Generic permission-request shell.
- `PermissionRequestTitle.tsx` (6KB) — Title bar for a permission request (icon + tool name).
- `PermissionExplanation.tsx` (24KB) — Body that explains the permission decision.
- `PermissionRuleExplanation.tsx` (15KB) — Explains which rule matched.
- `PermissionDecisionDebugInfo.tsx` (53KB) — `USER_TYPE === 'ant'` debug overlay.
- `FallbackPermissionRequest.tsx` (31KB) — Default UI when no per-tool component is registered.
- `SandboxPermissionRequest.tsx` (14KB) — Sandbox permission request.
- `WorkerBadge.tsx` (4KB) — Badge on permission requests showing worker context. **Cross-spec:** spec 30.
- `WorkerPendingPermission.tsx` (9KB) — Pending-permission UI for worker requests.
- `hooks.ts` (8KB) — Shared hooks for permission components (e.g., focus, key handling).
- `shellPermissionHelpers.tsx` (23KB) — Helpers for shell permission options. **Cross-spec:** spec 10.
- `useShellPermissionFeedback.ts` (5KB) — Feedback emission for shell permission decisions.
- `utils.ts` (0.7KB) — Tiny helpers.

#### Per-tool subdirs
- `BashPermissionRequest/BashPermissionRequest.tsx` (76KB) + `bashToolUseOptions.tsx` (21KB) — Bash permission UI with "always allow" matchers. **Cross-spec:** spec 10.
- `ComputerUseApproval/ComputerUseApproval.tsx` (45KB) — Computer-use approval (screenshot, mouse, keyboard).
- `EnterPlanModePermissionRequest/EnterPlanModePermissionRequest.tsx` (13KB) — Plan-mode entry approval. **Cross-spec:** spec 18.
- `ExitPlanModePermissionRequest/ExitPlanModePermissionRequest.tsx` (122KB) — Plan-mode exit approval (largest in module). Probably warrants its own spec section (see §3).
- `FileEditPermissionRequest/FileEditPermissionRequest.tsx` (16KB) — File edit approval. **Cross-spec:** spec 11.
- `FilePermissionDialog/FilePermissionDialog.tsx` (30KB) + `permissionOptions.tsx` (22KB) + `useFilePermissionDialog.ts` (7KB) + `usePermissionHandler.ts` (5KB) + `ideDiffConfig.ts` (0.9KB) — Generic file-permission dialog, IDE-diff config, hooks. **Cross-spec:** spec 11.
- `FileWritePermissionRequest/FileWritePermissionRequest.tsx` (17KB) + `FileWriteToolDiff.tsx` (10KB) — Write approval with inline diff. **Cross-spec:** spec 11.
- `FilesystemPermissionRequest/FilesystemPermissionRequest.tsx` (13KB) — Bare filesystem permission. **Cross-spec:** spec 11.
- `NotebookEditPermissionRequest/NotebookEditPermissionRequest.tsx` (16KB) + `NotebookEditToolDiff.tsx` (25KB) — Jupyter-notebook edit approval. **Cross-spec:** spec 11.
- `PowerShellPermissionRequest/PowerShellPermissionRequest.tsx` (39KB) + `powershellToolUseOptions.tsx` (12KB) — PowerShell approval. **Cross-spec:** spec 10.
- `SedEditPermissionRequest/SedEditPermissionRequest.tsx` (21KB) — Sed-edit approval. **Cross-spec:** spec 11.
- `SkillPermissionRequest/SkillPermissionRequest.tsx` (37KB) — Skill execution approval. **Cross-spec:** spec 17.
- `WebFetchPermissionRequest/WebFetchPermissionRequest.tsx` (23KB) — WebFetch approval. **Cross-spec:** spec 13.

#### `src/components/permissions/AskUserQuestionPermissionRequest/`
- `AskUserQuestionPermissionRequest.tsx` (82KB) — Top-level "ask user question" approval (the AskUserQuestion tool).
- `PreviewBox.tsx` (26KB) — Preview of pending question.
- `PreviewQuestionView.tsx` (53KB) — Full preview view.
- `QuestionView.tsx` (59KB) — One-question rendering.
- `QuestionNavigationBar.tsx` (23KB) — Navigation between questions.
- `SubmitQuestionsView.tsx` (17KB) — Submit-all view.
- `use-multiple-choice-state.ts` (4KB) — State hook for multiple-choice questions.

#### `src/components/permissions/rules/`
- `PermissionRuleList.tsx` (119KB) — Rules editor's main list (largest in subdir; warrants its own §). **Cross-spec:** spec 09.
- `AddPermissionRules.tsx` (22KB) — Add-rule form.
- `AddWorkspaceDirectory.tsx` (38KB) — Add a workspace directory rule.
- `RemoveWorkspaceDirectory.tsx` (10KB) — Remove a workspace directory.
- `WorkspaceTab.tsx` (15KB) — Workspace-rules tab.
- `RecentDenialsTab.tsx` (19KB) — Recent denials tab.
- `PermissionRuleInput.tsx` (16KB) — Input for a single rule.
- `PermissionRuleDescription.tsx` (7KB) — Description renderer.


### §2.4 `src/components/PromptInput/`

The composer. `PromptInput.tsx` (355KB) is the single largest component file
in the entire repo and is implicit in spec 37; it is repeated here for
catalog completeness.

- `PromptInput.tsx` (355KB) — Main composer screen (mode switching, history, paste, suggestions). Probably warrants its own spec section (see §3, §5). **(already cited in spec 37)**.
- `PromptInputFooter.tsx` (33KB) — Footer (token usage, cost, hints).
- `PromptInputFooterLeftSide.tsx` (87KB) — Footer left widget cluster (mode/model/effort indicators).
- `PromptInputFooterSuggestions.tsx` (34KB) — Suggestion chips below the input.
- `PromptInputHelpMenu.tsx` (33KB) — `?` help overlay for input.
- `PromptInputModeIndicator.tsx` (11KB) — Mode pill (bash/normal/etc.).
- `PromptInputQueuedCommands.tsx` (20KB) — Queued-commands display.
- `PromptInputStashNotice.tsx` (2KB) — Stash notice under input.
- `Notifications.tsx` (48KB) — Stack of inline notifications shown above the prompt input.
- `HistorySearchInput.tsx` (5KB) — Reverse-search input (Ctrl-R style).
- `IssueFlagBanner.tsx` (2KB) — Banner shown when an issue is flagged.
- `SandboxPromptFooterHint.tsx` (8KB) — Footer hint when sandbox is enabled.
- `ShimmeredInput.tsx` (17KB) — Input with shimmer animation (used during state transitions).
- `VoiceIndicator.tsx` (11KB) — Voice-mode indicator. **Flag:** `feature('VOICE_MODE')`.
- `inputModes.ts` (0.7KB) — Helpers for `!`-bash-mode prefix.
- `inputPaste.ts` (3KB) — Paste handler.
- `useMaybeTruncateInput.ts` (1.5KB) — Truncation hook.
- `usePromptInputPlaceholder.ts` (2KB) — Placeholder text hook.
- `useShowFastIconHint.ts` (0.7KB) — Fast-mode icon visibility hook.
- `useSwarmBanner.ts` (5KB) — Swarm-mode banner hook. **Cross-spec:** spec 30.
- `utils.ts` (2KB) — Misc helpers.


### §2.5 `src/components/agents/`

Agent management UI (`/agents` and create-agent wizard).

- `AgentDetail.tsx` (24KB) — One-agent detail view.
- `AgentEditor.tsx` (26KB) — Edit-agent form.
- `AgentNavigationFooter.tsx` (3KB) — Navigation footer for agent screens.
- `AgentsList.tsx` (52KB) — List of agents.
- `AgentsMenu.tsx` (71KB) — Top-level agents menu (entry to detail/edit/create).
- `ColorPicker.tsx` (14KB) — Agent color picker.
- `ModelSelector.tsx` (7KB) — Per-agent model picker.
- `ToolSelector.tsx` (65KB) — Per-agent tool-permissions picker.
- `agentFileUtils.ts` (7KB) — Read/write `.claude/agents/*.md` files.
- `generateAgent.ts` (10KB) — LLM-driven agent generator (`/agent`).
- `validateAgent.ts` (3KB) — Validation rules.

#### `src/components/agents/new-agent-creation/`
- `CreateAgentWizard.tsx` (10KB) — Wizard wrapper (uses `wizard/` framework).
- `wizard-steps/ColorStep.tsx` (10KB)
- `wizard-steps/ConfirmStep.tsx` (35KB) + `ConfirmStepWrapper.tsx` (14KB)
- `wizard-steps/DescriptionStep.tsx` (14KB)
- `wizard-steps/GenerateStep.tsx` (22KB)
- `wizard-steps/LocationStep.tsx` (8KB)
- `wizard-steps/MemoryStep.tsx` (14KB) — **Cross-spec:** spec 40.
- `wizard-steps/MethodStep.tsx` (9KB)
- `wizard-steps/ModelStep.tsx` (6KB)
- `wizard-steps/PromptStep.tsx` (15KB)
- `wizard-steps/ToolsStep.tsx` (7KB)
- `wizard-steps/TypeStep.tsx` (12KB)


### §2.6 `src/components/LogoV2/`

Welcome-screen / logo / startup-feed UI. All `LogoV2.tsx` is feature-gated
by combinations like `feature('PROACTIVE')` — see imports in `LogoV2.tsx`.

- `LogoV2.tsx` (75KB) — Layout coordinator (Clawd + feed columns + emergency tip + version notices).
- `WelcomeV2.tsx` (58KB) — Welcome screen variant.
- `Clawd.tsx` (19KB) — Static Clawd ASCII logo.
- `AnimatedClawd.tsx` (14KB) — Animated Clawd.
- `AnimatedAsterisk.tsx` (8KB) — Spinning asterisk.
- `CondensedLogo.tsx` (19KB) — Compact logo for narrow terminals.
- `Feed.tsx` (14KB) — Feed-area wrapper.
- `FeedColumn.tsx` (5KB) — One column in startup feed.
- `feedConfigs.tsx` (12KB) — Recent-activity / what's-new / project-onboarding / guest-passes feed configs.
- `EmergencyTip.tsx` (7KB) — Emergency tip banner.
- `GuestPassesUpsell.tsx` (9KB) — Guest passes upsell card.
- `OverageCreditUpsell.tsx` (18KB) — Overage credit upsell card.
- `Opus1mMergeNotice.tsx` (6KB) — Opus 1m migration notice.
- `ChannelsNotice.tsx` (30KB) — Notice for the Channels feature (flag-gated).
- `VoiceModeNotice.tsx` (8KB) — Voice-mode rollout notice. **Flag:** `feature('VOICE_MODE')`.


### §2.7 `src/components/design-system/`

Reusable themed primitives. `Box`/`Text` raw primitives live under `src/ink/`
(spec 37); these layer theme-keyed colors and convenience props on top.

- `ThemedBox.tsx` (18KB) — Theme-aware Box.
- `ThemedText.tsx` (14KB) — Theme-aware Text.
- `ThemeProvider.tsx` (19KB) — React context for the active theme.
- `Dialog.tsx` (14KB) — Base modal dialog with cancel/confirm keybindings.
- `Pane.tsx` (7KB) — Bordered pane.
- `Divider.tsx` (11KB) — Horizontal/vertical rule.
- `Tabs.tsx` (41KB) — Tab navigation.
- `ListItem.tsx` (20KB) — Standardized list item.
- `LoadingState.tsx` (6KB) — Inline loading indicator.
- `ProgressBar.tsx` (7KB) — Themed progress bar.
- `Ratchet.tsx` (7KB) — Animated counter.
- `StatusIcon.tsx` (8KB) — Success/error/warning/info icon.
- `Byline.tsx` (6KB) — Small byline text below a title.
- `KeyboardShortcutHint.tsx` (7KB) — Renders a key-sequence hint.
- `FuzzyPicker.tsx` (41KB) — Generic fuzzy-search picker.
- `color.ts` (0.9KB) — Color helpers.


### §2.8 `src/components/CustomSelect/`

Lower-level select primitives. Used by all pickers (Theme, Model, etc.).

- `select.tsx` (115KB) — Single-select input.
- `SelectMulti.tsx` (30KB) — Multi-select input.
- `select-input-option.tsx` (58KB) — Per-option renderer when select has an embedded text input.
- `select-option.tsx` (6KB) — Per-option renderer.
- `option-map.ts` (1KB) — Option index helpers.
- `use-select-state.ts` (3KB) — Single-select state.
- `use-multi-select-state.ts` (11KB) — Multi-select state.
- `use-select-input.ts` (9KB) — Embedded-input handling.
- `use-select-navigation.ts` (16KB) — Arrow-key navigation.
- `index.ts` (≤120 bytes) — barrel.


### §2.9 `src/components/mcp/`

MCP server management UI. **Cross-spec:** spec 23.

- `MCPListPanel.tsx` (58KB) — Main MCP servers list panel.
- `MCPSettings.tsx` (40KB) — MCP settings entry.
- `MCPAgentServerMenu.tsx` (27KB) — Agent-MCP server menu.
- `MCPRemoteServerMenu.tsx` (102KB) — Remote MCP server config (largest in module). **Probably warrants its own spec section (see §3).**
- `MCPStdioServerMenu.tsx` (28KB) — Stdio MCP server config.
- `MCPToolDetailView.tsx` (23KB) — Per-tool detail view.
- `MCPToolListView.tsx` (16KB) — Tool list within a server.
- `MCPReconnect.tsx` (16KB) — Reconnect prompt.
- `McpParsingWarnings.tsx` (22KB) — Parsing-warning display.
- `CapabilitiesSection.tsx` (5KB) — Server capabilities listing.
- `ElicitationDialog.tsx` (180KB) — Elicitation (server-driven) dialog. **Largest in spec 23 surface; warrants its own spec section.**
- `utils/reconnectHelpers.tsx` (5KB) — Helpers for reconnect logic.
- `index.ts` (≤520 bytes) — barrel.


### §2.10 `src/components/Spinner/`

Sub-components of the root `Spinner.tsx`.

- `SpinnerAnimationRow.tsx` (43KB) — Animated single-row spinner with thinking shimmer.
- `SpinnerGlyph.tsx` (10KB) — The glyph wheel.
- `TeammateSpinnerLine.tsx` (39KB) — Per-teammate spinner line. **Cross-spec:** spec 30.
- `TeammateSpinnerTree.tsx` (28KB) — Tree of teammate spinners. **Cross-spec:** spec 30.
- `GlimmerMessage.tsx` (27KB) — Animated "thinking" / "doing X" message.
- `FlashingChar.tsx` (6KB) — One flashing character.
- `ShimmerChar.tsx` (3KB) — One shimmer character.
- `useShimmerAnimation.ts` (1KB) — Shimmer animation hook.
- `useStalledAnimation.ts` (2.5KB) — Stalled-state animation hook.
- `utils.ts` (2KB) — Color interpolation helpers.
- `index.ts` (≤600 bytes) — barrel.
- `teammateSelectHint.ts` (64 bytes) — tiny stub.


### §2.11 `src/components/tasks/`

Async / background-task UI. **Cross-spec:** spec 15 (task tools), spec 30
(coordinator), spec 35 (remote sessions).

- `BackgroundTasksDialog.tsx` (116KB) — Dialog listing all background tasks.
- `BackgroundTask.tsx` (31KB) — Single-task row.
- `BackgroundTaskStatus.tsx` (43KB) — Status pill for a task.
- `RemoteSessionDetailDialog.tsx` (96KB) — Remote-session detail.
- `RemoteSessionProgress.tsx` (28KB) — Remote-session progress.
- `AsyncAgentDetailDialog.tsx` (30KB) — Async-agent detail.
- `InProcessTeammateDetailDialog.tsx` (31KB) — In-process teammate detail.
- `DreamDetailDialog.tsx` (26KB) — "Dream" task detail (Anthropic-internal experiment).
- `ShellDetailDialog.tsx` (39KB) — Shell-task detail.
- `ShellProgress.tsx` (7KB) — Shell-task progress.
- `renderToolActivity.tsx` (4KB) — Helper to render tool activity.
- `taskStatusUtils.tsx` (14KB) — Status mapping helpers.


### §2.12 `src/components/FeedbackSurvey/`

Post-session survey + transcript-share. **Cross-spec:** spec 26 (analytics).

- `FeedbackSurvey.tsx` (19KB) — Survey screen.
- `FeedbackSurveyView.tsx` (11KB) — Inner view.
- `useFeedbackSurvey.tsx` (48KB) — Survey state machine hook.
- `useMemorySurvey.tsx` (30KB) — Memory-specific survey hook.
- `usePostCompactSurvey.tsx` (24KB) — Post-compaction survey hook.
- `useSurveyState.tsx` (15KB) — Generic survey state.
- `useDebouncedDigitInput.ts` (3KB) — Helper for digit-only inputs.
- `TranscriptSharePrompt.tsx` (10KB) — Transcript-share dialog.
- `submitTranscriptShare.ts` (3KB) — Submit helper.


### §2.13 `src/components/hooks/`

Settings UI for the Hooks subsystem. **Cross-spec:** spec 04 (hooks runtime).

- `HooksConfigMenu.tsx` (54KB) — Main hooks config menu.
- `PromptDialog.tsx` (7KB) — Prompt-hook config dialog.
- `SelectEventMode.tsx` (14KB) — Pick which event a hook fires on.
- `SelectHookMode.tsx` (13KB) — Pick hook execution mode.
- `SelectMatcherMode.tsx` (15KB) — Pick matcher type.
- `ViewHookMode.tsx` (18KB) — View an existing hook.


### §2.14 `src/components/sandbox/`

- `SandboxSettings.tsx` (30KB) — Sandbox settings entry.
- `SandboxConfigTab.tsx` (17KB) — Config tab.
- `SandboxDependenciesTab.tsx` (17KB) — Dependencies tab.
- `SandboxOverridesTab.tsx` (20KB) — Overrides tab.
- `SandboxDoctorSection.tsx` (6KB) — Doctor diagnostics.


### §2.15 `src/components/diff/`

- `DiffDialog.tsx` (43KB) — Full diff dialog.
- `DiffDetailView.tsx` (23KB) — One file's diff detail.
- `DiffFileList.tsx` (25KB) — File list within the diff.


### §2.16 `src/components/shell/`

- `OutputLine.tsx` (14KB) — One line of bash output.
- `ShellProgressMessage.tsx` (14KB) — Bash progress line.
- `ShellTimeDisplay.tsx` (5KB) — Elapsed-time display.
- `ExpandShellOutputContext.tsx` (4KB) — Context for ctrl-o expansion of shell output.


### §2.17 `src/components/teams/` (`feature('COORDINATOR_MODE')`)

- `TeamsDialog.tsx` (95KB) — Team management dialog.
- `TeamStatus.tsx` (7KB) — Team status display.


### §2.18 `src/components/wizard/`

Generic wizard framework used by agent creation and IDE bridge.

- `WizardProvider.tsx` (19KB) — Context provider with steps/data state.
- `WizardDialogLayout.tsx` (6KB) — Dialog layout for wizards.
- `WizardNavigationFooter.tsx` (4KB) — Next/back/finish buttons.
- `useWizard.ts` (0.5KB) — Convenience hook.
- `index.ts` (≤330 bytes) — barrel.


### §2.19 `src/components/memory/`

- `MemoryFileSelector.tsx` (48KB) — File picker for memory updates. **Cross-spec:** spec 40.
- `MemoryUpdateNotification.tsx` (5KB) — "Memory updated" inline banner.


### §2.20 `src/components/skills/`

- `SkillsMenu.tsx` (27KB) — Skill picker. **Cross-spec:** spec 17.


### §2.21 `src/components/grove/`

- `Grove.tsx` (50KB) — Grove (worktree) screen. Likely flag-gated.


### §2.22 `src/components/ui/`

- `TreeSelect.tsx` (39KB) — Tree-structured select.
- `OrderedList.tsx` (7KB) — Numbered list.
- `OrderedListItem.tsx` (3KB) — One item.


### §2.23 `src/components/Settings/`

(Listed in §2.1 root section above; included here for index completeness.)

- `Settings.tsx` (19KB) — Top-level settings entry.
- `Status.tsx` (26KB), `Usage.tsx` (40KB), `Config.tsx` (271KB).


### §2.24 Single-purpose dialog clusters

- `TrustDialog/TrustDialog.tsx` (32KB) + `utils.ts` (7KB) — Workspace trust.
- `ManagedSettingsSecurityDialog/ManagedSettingsSecurityDialog.tsx` (14KB) + `utils.ts` (4KB) — Managed-settings security warning.
- `DesktopUpsell/DesktopUpsellStartup.tsx` (16KB) — Startup upsell for desktop client.
- `LspRecommendation/LspRecommendationMenu.tsx` (10KB) — LSP plugin recommendation. **Cross-spec:** spec 24.
- `ClaudeCodeHint/PluginHintMenu.tsx` (9KB) — Plugin hint menu. **Cross-spec:** spec 28.
- `Passes/Passes.tsx` (27KB) — Pass tokens UI.
- `HelpV2/HelpV2.tsx` (21KB) + `Commands.tsx` (10KB) + `General.tsx` (3KB) — Help screen.


## §3 Patterns Observed

**1. React Compiler runtime everywhere.** Almost every `.tsx` starts with
`import { c as _c } from "react/compiler-runtime";` followed by `const $ = _c(N);`
on first render. This is the React Forget / React Compiler memoization
runtime — components are pre-compiled with manual memo slots. It means the
public source has been through a build step and is **not the original
hand-written source**: prop destructuring is often hoisted into numbered
slots and bare `useMemo`s are gone. Treat reads carefully.

**2. Three-layer Ink stack.** From bottom to top:
- `src/ink/components/` — raw Ink primitives (`Box`, `Text`, `RawAnsi`,
  `NoSelect`, `Newline`, `Spacer`, `Link`, `Button`, `ScrollBox`, `AlternateScreen`).
  Plus `src/ink.js` barrel that re-exports `Box, Text, ...` plus `useTheme`,
  `useAnimationFrame`, etc.
- `src/components/design-system/` — themed wrappers (`ThemedBox`, `ThemedText`,
  `Dialog`, `Pane`, `Divider`, `Tabs`, `FuzzyPicker`, `KeyboardShortcutHint`).
- `src/components/<feature>/` — feature-specific components that compose the
  themed primitives.

**3. One subdir per major modal/screen + per-tool permission.** Permission
prompts split into one directory per tool family, each holding the screen
plus tool-specific options/diff helpers. Same pattern for agents wizard
steps, MCP server menus (Stdio/Remote/Agent), Sandbox settings tabs, and
Hook-config modes. This mirrors the `src/tools/<Tool>/` layout in spec 8.

**4. Survey/wizard hooks live next to UI.** State machines like
`useFeedbackSurvey.tsx` (48KB), `useMemorySurvey.tsx` (30KB),
`usePostCompactSurvey.tsx` (24KB), `WizardProvider.tsx` (19KB) are
component-adjacent rather than living under `src/hooks/` — making them
catalog-relevant here even though they don't render UI directly.

**5. `feature(...)` gating is concentrated at top-level mount points.**
Most leaf components are unconditional; flags are checked once in the
parent (`LogoV2.tsx`, `App.tsx`, etc.) and the flag-only subtree is
imported with a conditional `require()`.


## §4 Cross-Spec Dependency Edges

Components in this catalog form the "view layer" for many sibling
subsystems. The most-frequent edges, by count:

- **spec 09 (permissions)** → entire `src/components/permissions/` tree
  (49 files, including `PermissionRuleList.tsx` 119KB and
  `ExitPlanModePermissionRequest.tsx` 122KB). The view layer for spec 09
  is materially under spec 37a.
- **spec 23 (MCP)** → `src/components/mcp/` (13 files, including the 180KB
  `ElicitationDialog.tsx`) plus `MCPServerApprovalDialog.tsx`,
  `MCPServerDesktopImportDialog.tsx`, `MCPServerMultiselectDialog.tsx`,
  `UserResourceUpdateMessage.tsx`.
- **spec 30 (coordinator / multi-agent)** → `CoordinatorAgentStatus.tsx`,
  `Spinner/TeammateSpinnerLine.tsx`, `Spinner/TeammateSpinnerTree.tsx`,
  `messages/UserTeammateMessage.tsx`, `messages/TaskAssignmentMessage.tsx`,
  `messages/UserAgentNotificationMessage.tsx`, `messages/ShutdownMessage.tsx`,
  `permissions/WorkerBadge.tsx`, `permissions/WorkerPendingPermission.tsx`,
  `teams/*`, `TeammateViewHeader.tsx`. **~13 components.**
- **spec 11 (file tools)** → `FileEditToolDiff.tsx`, `FileEditToolUpdatedMessage.tsx`,
  `FileEditToolUseRejectedMessage.tsx`, `NotebookEditToolUseRejectedMessage.tsx`,
  `messages/AttachmentMessage.tsx`, all per-file permission subdirs.
  **~10 components.**
- **spec 35 (remote / teleport / server mode)** → `RemoteCallout.tsx`,
  `RemoteEnvironmentDialog.tsx`, `Teleport*.tsx` (5 files),
  `tasks/RemoteSessionDetailDialog.tsx`, `tasks/RemoteSessionProgress.tsx`.
  **~9 components.**
- **spec 18 (plan mode)** → `messages/PlanApprovalMessage.tsx`,
  `messages/UserPlanMessage.tsx`,
  `permissions/EnterPlanModePermissionRequest/`,
  `permissions/ExitPlanModePermissionRequest/`,
  `messages/UserToolResultMessage/RejectedPlanMessage.tsx`. **5 components.**
- **spec 41 (session/state/history)** → `SessionPreview.tsx`,
  `SessionBackgroundHint.tsx`, `ResumeTask.tsx`, `VirtualMessageList.tsx`,
  `TeleportResumeWrapper.tsx`. **~5 components.**
- **spec 25 (OAuth/auth)** → `ConsoleOAuthFlow.tsx`, `AwsAuthStatusBox.tsx`,
  `ApproveApiKey.tsx`. **3 components.**
- **spec 17 (skills)** → `skills/SkillsMenu.tsx`,
  `permissions/SkillPermissionRequest/SkillPermissionRequest.tsx`,
  `SkillImprovementSurvey.tsx`. **3 components.**
- **spec 24 (LSP)** → `LspRecommendation/LspRecommendationMenu.tsx`,
  `DiagnosticsDisplay.tsx`. **2 components.**
- **spec 40 (persistent memory)** → `memory/*`,
  `agents/new-agent-creation/wizard-steps/MemoryStep.tsx`,
  `messages/UserMemoryInputMessage.tsx`, `MemoryUsageIndicator.tsx`. **5 components.**
- **spec 39 (vim)** → `VimTextInput.tsx`. **1.**
- **spec 28 (plugins)** → `ClaudeCodeHint/PluginHintMenu.tsx`. **1.**
- **spec 34 (bridge)** → `BridgeDialog.tsx`. **1.**
- **spec 38 (output styles)** → `OutputStylePicker.tsx`. **1.**
- **spec 06 (cost/tokens)** → `Stats.tsx`, `Settings/Usage.tsx`,
  `TokenWarning.tsx`, `MemoryUsageIndicator.tsx`. **4.**
- **spec 07 (compaction)** → `CompactSummary.tsx`,
  `messages/CompactBoundaryMessage.tsx`,
  `FeedbackSurvey/usePostCompactSurvey.tsx`. **3.**
- **spec 05 (context)** → `ContextSuggestions.tsx`, `ContextVisualization.tsx`. **2.**
- **spec 22 (service api)** → `ModelPicker.tsx` (calls into model registry),
  `messages/AssistantTextMessage.tsx` (knows API error prefixes from
  `services/api/errors.ts`).

**Surprise:** The largest spec-09 surface (`PermissionRuleList.tsx` 119KB,
`ExitPlanModePermissionRequest.tsx` 122KB) and the largest spec-23 surface
(`ElicitationDialog.tsx` 180KB) actually live entirely under
`src/components/`, not under `src/services/`. Spec 37 historically
treats components as "the view layer" but the bulk of permission and
MCP UX logic — including state machines, validation, and rule editing —
resides in components, not services. **Conclusion:** specs 09 and 23
should each get a "see also: 37a §2.3 / §2.9" pointer, and the
~22 components shared between specs 09/23 and 37a are effectively
co-owned.

**Surprise:** ~13 components belong to spec 30 (coordinator) — the
audit only flagged 1 (`CoordinatorAgentStatus.tsx`). The coordinator
system is much more deeply tied to UI than its single explicit
component would suggest.

