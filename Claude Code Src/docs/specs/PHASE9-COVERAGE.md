# Phase 9 Coverage Audit

Mechanical diff between `src/` files and paths cited in specs under `docs/specs/`.

Generated 2026-05-09 against 46 spec files (00-overview.md..42-misc.md plus 21a/21b/21c).


## Summary

- **Total src/ files:** 1902 (excluding `src/main.tsx` bundled artifact: 1901)
- **Citations in specs (strict, `src/`-prefixed):** 868 unique paths
- **Strict residuals (no `src/`-prefixed citation):** 1161
- **Permissive residuals (also matching basename/suffix tokens):** 918
- **Bundled artifact:** 1 (`src/main.tsx`, 803KB)
- **Trivial residuals (tiny stubs / index barrels / .d.ts / json):** 10
- **Suspect residuals (real source needing coverage):** 908

The permissive count is the more honest figure: 918 of 1901 non-bundled source files are not cited even by basename in any spec.


## Per-Subdir Residual Table

Suspect-residual count per top-level `src/` subdir (sorted by residual count):

| Subdir | Total Files | Suspect Residual | % Uncovered |
|---|---:|---:|---:|
| `src/utils` | 564 | 327 | 58% |
| `src/components` | 389 | 315 | 81% |
| `src/hooks` | 104 | 86 | 83% |
| `src/commands` | 207 | 73 | 35% |
| `src/ink` | 96 | 69 | 72% |
| `src/skills` | 20 | 13 | 65% |
| `src/constants` | 21 | 9 | 43% |
| `src/tools` | 184 | 9 | 5% |
| `src/cli` | 19 | 6 | 32% |
| `src/types` | 11 | 1 | 9% |
| `src/ROOT` | 18 | 0 | 0% |
| `src/assistant` | 1 | 0 | 0% |
| `src/bootstrap` | 1 | 0 | 0% |
| `src/bridge` | 31 | 0 | 0% |
| `src/buddy` | 6 | 0 | 0% |
| `src/context` | 9 | 0 | 0% |
| `src/coordinator` | 1 | 0 | 0% |
| `src/entrypoints` | 8 | 0 | 0% |
| `src/keybindings` | 14 | 0 | 0% |
| `src/memdir` | 8 | 0 | 0% |
| `src/migrations` | 11 | 0 | 0% |
| `src/moreright` | 1 | 0 | 0% |
| `src/native-ts` | 4 | 0 | 0% |
| `src/outputStyles` | 1 | 0 | 0% |
| `src/plugins` | 2 | 0 | 0% |
| `src/query` | 4 | 0 | 0% |
| `src/remote` | 4 | 0 | 0% |
| `src/schemas` | 1 | 0 | 0% |
| `src/screens` | 3 | 0 | 0% |
| `src/server` | 3 | 0 | 0% |
| `src/services` | 130 | 0 | 0% |
| `src/state` | 6 | 0 | 0% |
| `src/tasks` | 12 | 0 | 0% |
| `src/upstreamproxy` | 2 | 0 | 0% |
| `src/vim` | 5 | 0 | 0% |
| `src/voice` | 1 | 0 | 0% |

**Surprising patterns:**
- `src/components/` (389 files, 81% uncovered) and `src/hooks/` (104 files, 83% uncovered) are catastrophically under-cited. Spec **37-ink-ui-shell.md** is responsible for these — it appears to enumerate only the major shells/screens, not individual components.
- `src/utils/` (564 files, 58% uncovered) — spec **42-misc.md** picks up some but the long tail is largely unmentioned.
- `src/ink/` (96 files, 72% uncovered) — Ink primitives wrappers not enumerated by 37.
- Strong coverage (0% suspect residuals) in: `services/`, `bridge/`, `coordinator/`, `tasks/`, `schemas/`, `migrations/`, `memdir/`, `query/`, `vim/`, `voice/`, `plugins/`, `outputStyles/`, `state/`, `keybindings/`, `entrypoints/`, `remote/`, `server/`, `screens/`, `buddy/`, `context/`, `native-ts/`, `bootstrap/`, and the `ROOT` (top-level src/*.ts) — these specs were thorough at the file level.
- `src/tools/` is 95% covered (only 9 suspects out of 184) — tool-specific specs (10-19) are working well.
- `src/commands/` is 65% covered — the 21a/21b/21c split missed roughly 73 command files.


## Suspect Residuals by Proposed Spec Assignment


### → `37-ink-ui-shell` (372 files)

- `src/components/AgentProgressLine.tsx`
- `src/components/AutoUpdater.tsx`
- `src/components/AutoUpdaterWrapper.tsx`
- `src/components/AwsAuthStatusBox.tsx`
- `src/components/BaseTextInput.tsx`
- `src/components/BashModeProgress.tsx`
- `src/components/ClaudeCodeHint/PluginHintMenu.tsx`
- `src/components/ClaudeInChromeOnboarding.tsx`
- `src/components/ClickableImageRef.tsx`
- `src/components/CompactSummary.tsx`
- `src/components/ConfigurableShortcutHint.tsx`
- `src/components/ConsoleOAuthFlow.tsx`
- `src/components/ContextSuggestions.tsx`
- `src/components/ContextVisualization.tsx`
- `src/components/CostThresholdDialog.tsx`
- `src/components/CtrlOToExpand.tsx`
- `src/components/CustomSelect/SelectMulti.tsx`
- `src/components/CustomSelect/option-map.ts`
- `src/components/CustomSelect/select-input-option.tsx`
- `src/components/CustomSelect/select-option.tsx`
- `src/components/CustomSelect/select.tsx`
- `src/components/CustomSelect/use-multi-select-state.ts`
- `src/components/CustomSelect/use-select-input.ts`
- `src/components/CustomSelect/use-select-navigation.ts`
- `src/components/CustomSelect/use-select-state.ts`
- `src/components/DesktopUpsell/DesktopUpsellStartup.tsx`
- `src/components/DevBar.tsx`
- `src/components/DiagnosticsDisplay.tsx`
- `src/components/EffortCallout.tsx`
- `src/components/EffortIndicator.ts`
- `src/components/ExportDialog.tsx`
- `src/components/FallbackToolUseErrorMessage.tsx`
- `src/components/FallbackToolUseRejectedMessage.tsx`
- `src/components/FastIcon.tsx`
- `src/components/FeedbackSurvey/FeedbackSurvey.tsx`
- `src/components/FeedbackSurvey/FeedbackSurveyView.tsx`
- `src/components/FeedbackSurvey/TranscriptSharePrompt.tsx`
- `src/components/FeedbackSurvey/submitTranscriptShare.ts`
- `src/components/FeedbackSurvey/useDebouncedDigitInput.ts`
- `src/components/FeedbackSurvey/usePostCompactSurvey.tsx`
- `src/components/FeedbackSurvey/useSurveyState.tsx`
- `src/components/FileEditToolDiff.tsx`
- `src/components/FileEditToolUpdatedMessage.tsx`
- `src/components/FileEditToolUseRejectedMessage.tsx`
- `src/components/FilePathLink.tsx`
- `src/components/FullscreenLayout.tsx`
- `src/components/GlobalSearchDialog.tsx`
- `src/components/HelpV2/HelpV2.tsx`
- `src/components/HighlightedCode.tsx`
- `src/components/HighlightedCode/Fallback.tsx`
- `src/components/HistorySearchDialog.tsx`
- `src/components/IdeAutoConnectDialog.tsx`
- `src/components/IdeOnboardingDialog.tsx`
- `src/components/IdeStatusIndicator.tsx`
- `src/components/InterruptedByUser.tsx`
- `src/components/InvalidConfigDialog.tsx`
- `src/components/InvalidSettingsDialog.tsx`
- `src/components/KeybindingWarnings.tsx`
- `src/components/LanguagePicker.tsx`
- `src/components/LogSelector.tsx`
- `src/components/LogoV2/AnimatedAsterisk.tsx`
- `src/components/LogoV2/AnimatedClawd.tsx`
- `src/components/LogoV2/Clawd.tsx`
- `src/components/LogoV2/CondensedLogo.tsx`
- `src/components/LogoV2/EmergencyTip.tsx`
- `src/components/LogoV2/Feed.tsx`
- `src/components/LogoV2/FeedColumn.tsx`
- `src/components/LogoV2/GuestPassesUpsell.tsx`
- `src/components/LogoV2/LogoV2.tsx`
- `src/components/LogoV2/Opus1mMergeNotice.tsx`
- `src/components/LogoV2/OverageCreditUpsell.tsx`
- `src/components/LogoV2/WelcomeV2.tsx`
- `src/components/LogoV2/feedConfigs.tsx`
- `src/components/LspRecommendation/LspRecommendationMenu.tsx`
- `src/components/ManagedSettingsSecurityDialog/ManagedSettingsSecurityDialog.tsx`
- `src/components/Markdown.tsx`
- `src/components/MarkdownTable.tsx`
- `src/components/ModelPicker.tsx`
- `src/components/NativeAutoUpdater.tsx`
- `src/components/NotebookEditToolUseRejectedMessage.tsx`
- `src/components/OffscreenFreeze.tsx`
- `src/components/PackageManagerAutoUpdater.tsx`
- `src/components/Passes/Passes.tsx`
- `src/components/PrBadge.tsx`
- `src/components/PressEnterToContinue.tsx`
- `src/components/PromptInput/HistorySearchInput.tsx`
- `src/components/PromptInput/IssueFlagBanner.tsx`
- `src/components/PromptInput/PromptInputFooterSuggestions.tsx`
- `src/components/PromptInput/PromptInputHelpMenu.tsx`
- `src/components/PromptInput/PromptInputModeIndicator.tsx`
- `src/components/PromptInput/PromptInputQueuedCommands.tsx`
- `src/components/PromptInput/PromptInputStashNotice.tsx`
- `src/components/PromptInput/SandboxPromptFooterHint.tsx`
- `src/components/PromptInput/ShimmeredInput.tsx`
- `src/components/PromptInput/inputModes.ts`
- `src/components/PromptInput/inputPaste.ts`
- `src/components/PromptInput/useMaybeTruncateInput.ts`
- `src/components/PromptInput/usePromptInputPlaceholder.ts`
- `src/components/PromptInput/useShowFastIconHint.ts`
- `src/components/PromptInput/useSwarmBanner.ts`
- `src/components/QuickOpenDialog.tsx`
- `src/components/ResumeTask.tsx`
- `src/components/SandboxViolationExpandedView.tsx`
- `src/components/ScrollKeybindingHandler.tsx`
- `src/components/SearchBox.tsx`
- `src/components/SentryErrorBoundary.ts`
- `src/components/SessionBackgroundHint.tsx`
- `src/components/SessionPreview.tsx`
- `src/components/Settings/Settings.tsx`
- `src/components/Settings/Status.tsx`
- `src/components/Settings/Usage.tsx`
- `src/components/ShowInIDEPrompt.tsx`
- `src/components/SkillImprovementSurvey.tsx`
- `src/components/Spinner/FlashingChar.tsx`
- `src/components/Spinner/GlimmerMessage.tsx`
- `src/components/Spinner/ShimmerChar.tsx`
- `src/components/Spinner/SpinnerAnimationRow.tsx`
- `src/components/Spinner/TeammateSpinnerLine.tsx`
- `src/components/Spinner/useShimmerAnimation.ts`
- `src/components/Spinner/useStalledAnimation.ts`
- `src/components/StructuredDiff.tsx`
- `src/components/StructuredDiff/Fallback.tsx`
- `src/components/StructuredDiff/colorDiff.ts`
- `src/components/StructuredDiffList.tsx`
- `src/components/TagTabs.tsx`
- `src/components/TaskListV2.tsx`
- `src/components/TeammateViewHeader.tsx`
- `src/components/TeleportError.tsx`
- `src/components/TeleportProgress.tsx`
- `src/components/TeleportRepoMismatchDialog.tsx`
- `src/components/TeleportResumeWrapper.tsx`
- `src/components/TeleportStash.tsx`
- `src/components/TextInput.tsx`
- `src/components/ThemePicker.tsx`
- `src/components/ThinkingToggle.tsx`
- `src/components/ToolUseLoader.tsx`
- `src/components/ValidationErrorsList.tsx`
- `src/components/VimTextInput.tsx`
- `src/components/WorkflowMultiselectDialog.tsx`
- `src/components/WorktreeExitDialog.tsx`
- `src/components/agents/AgentDetail.tsx`
- `src/components/agents/AgentEditor.tsx`
- `src/components/agents/AgentNavigationFooter.tsx`
- `src/components/agents/AgentsList.tsx`
- `src/components/agents/AgentsMenu.tsx`
- `src/components/agents/ColorPicker.tsx`
- `src/components/agents/ModelSelector.tsx`
- `src/components/agents/ToolSelector.tsx`
- `src/components/agents/agentFileUtils.ts`
- `src/components/agents/new-agent-creation/CreateAgentWizard.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/ColorStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/ConfirmStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/ConfirmStepWrapper.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/DescriptionStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/GenerateStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/LocationStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/MethodStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/ModelStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/PromptStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/ToolsStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/TypeStep.tsx`
- `src/components/agents/validateAgent.ts`
- `src/components/design-system/Byline.tsx`
- `src/components/design-system/Dialog.tsx`
- `src/components/design-system/Divider.tsx`
- `src/components/design-system/KeyboardShortcutHint.tsx`
- `src/components/design-system/ListItem.tsx`
- `src/components/design-system/LoadingState.tsx`
- `src/components/design-system/Pane.tsx`
- `src/components/design-system/ProgressBar.tsx`
- `src/components/design-system/Ratchet.tsx`
- `src/components/design-system/StatusIcon.tsx`
- `src/components/design-system/Tabs.tsx`
- `src/components/design-system/ThemedBox.tsx`
- `src/components/design-system/ThemedText.tsx`
- `src/components/diff/DiffDetailView.tsx`
- `src/components/diff/DiffDialog.tsx`
- `src/components/diff/DiffFileList.tsx`
- `src/components/grove/Grove.tsx`
- `src/components/hooks/HooksConfigMenu.tsx`
- `src/components/hooks/PromptDialog.tsx`
- `src/components/hooks/SelectEventMode.tsx`
- `src/components/hooks/SelectHookMode.tsx`
- `src/components/hooks/SelectMatcherMode.tsx`
- `src/components/hooks/ViewHookMode.tsx`
- `src/components/mcp/CapabilitiesSection.tsx`
- `src/components/mcp/ElicitationDialog.tsx`
- `src/components/mcp/MCPAgentServerMenu.tsx`
- `src/components/mcp/MCPListPanel.tsx`
- `src/components/mcp/MCPReconnect.tsx`
- `src/components/mcp/MCPRemoteServerMenu.tsx`
- `src/components/mcp/MCPSettings.tsx`
- `src/components/mcp/MCPStdioServerMenu.tsx`
- `src/components/mcp/MCPToolDetailView.tsx`
- `src/components/mcp/MCPToolListView.tsx`
- `src/components/mcp/McpParsingWarnings.tsx`
- `src/components/mcp/utils/reconnectHelpers.tsx`
- `src/components/messages/AdvisorMessage.tsx`
- `src/components/messages/AssistantRedactedThinkingMessage.tsx`
- `src/components/messages/AssistantTextMessage.tsx`
- `src/components/messages/AssistantThinkingMessage.tsx`
- `src/components/messages/AssistantToolUseMessage.tsx`
- `src/components/messages/AttachmentMessage.tsx`
- `src/components/messages/CollapsedReadSearchContent.tsx`
- `src/components/messages/CompactBoundaryMessage.tsx`
- `src/components/messages/GroupedToolUseContent.tsx`
- `src/components/messages/HighlightedThinkingText.tsx`
- `src/components/messages/HookProgressMessage.tsx`
- `src/components/messages/PlanApprovalMessage.tsx`
- `src/components/messages/RateLimitMessage.tsx`
- `src/components/messages/ShutdownMessage.tsx`
- `src/components/messages/SystemAPIErrorMessage.tsx`
- `src/components/messages/SystemTextMessage.tsx`
- `src/components/messages/TaskAssignmentMessage.tsx`
- `src/components/messages/UserAgentNotificationMessage.tsx`
- `src/components/messages/UserBashInputMessage.tsx`
- `src/components/messages/UserBashOutputMessage.tsx`
- `src/components/messages/UserChannelMessage.tsx`
- `src/components/messages/UserCommandMessage.tsx`
- `src/components/messages/UserImageMessage.tsx`
- `src/components/messages/UserLocalCommandOutputMessage.tsx`
- `src/components/messages/UserPlanMessage.tsx`
- `src/components/messages/UserPromptMessage.tsx`
- `src/components/messages/UserResourceUpdateMessage.tsx`
- `src/components/messages/UserTeammateMessage.tsx`
- `src/components/messages/UserTextMessage.tsx`
- `src/components/messages/UserToolResultMessage/RejectedPlanMessage.tsx`
- `src/components/messages/UserToolResultMessage/RejectedToolUseMessage.tsx`
- `src/components/messages/UserToolResultMessage/UserToolCanceledMessage.tsx`
- `src/components/messages/UserToolResultMessage/UserToolErrorMessage.tsx`
- `src/components/messages/UserToolResultMessage/UserToolRejectMessage.tsx`
- `src/components/messages/UserToolResultMessage/UserToolResultMessage.tsx`
- `src/components/messages/UserToolResultMessage/UserToolSuccessMessage.tsx`
- `src/components/messages/UserToolResultMessage/utils.tsx`
- `src/components/messages/nullRenderingAttachments.ts`
- `src/components/messages/teamMemCollapsed.tsx`
- `src/components/messages/teamMemSaved.ts`
- `src/components/permissions/AskUserQuestionPermissionRequest/AskUserQuestionPermissionRequest.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/PreviewBox.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/PreviewQuestionView.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/QuestionNavigationBar.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/QuestionView.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/SubmitQuestionsView.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/use-multiple-choice-state.ts`
- `src/components/permissions/BashPermissionRequest/BashPermissionRequest.tsx`
- `src/components/permissions/ComputerUseApproval/ComputerUseApproval.tsx`
- `src/components/permissions/EnterPlanModePermissionRequest/EnterPlanModePermissionRequest.tsx`
- `src/components/permissions/ExitPlanModePermissionRequest/ExitPlanModePermissionRequest.tsx`
- `src/components/permissions/FileEditPermissionRequest/FileEditPermissionRequest.tsx`
- `src/components/permissions/FilePermissionDialog/FilePermissionDialog.tsx`
- `src/components/permissions/FilePermissionDialog/ideDiffConfig.ts`
- `src/components/permissions/FilePermissionDialog/permissionOptions.tsx`
- `src/components/permissions/FilePermissionDialog/useFilePermissionDialog.ts`
- `src/components/permissions/FilePermissionDialog/usePermissionHandler.ts`
- `src/components/permissions/FileWritePermissionRequest/FileWritePermissionRequest.tsx`
- `src/components/permissions/FileWritePermissionRequest/FileWriteToolDiff.tsx`
- `src/components/permissions/FilesystemPermissionRequest/FilesystemPermissionRequest.tsx`
- `src/components/permissions/NotebookEditPermissionRequest/NotebookEditPermissionRequest.tsx`
- `src/components/permissions/NotebookEditPermissionRequest/NotebookEditToolDiff.tsx`
- `src/components/permissions/PowerShellPermissionRequest/PowerShellPermissionRequest.tsx`
- `src/components/permissions/PowerShellPermissionRequest/powershellToolUseOptions.tsx`
- `src/components/permissions/SandboxPermissionRequest.tsx`
- `src/components/permissions/SedEditPermissionRequest/SedEditPermissionRequest.tsx`
- `src/components/permissions/SkillPermissionRequest/SkillPermissionRequest.tsx`
- `src/components/permissions/WebFetchPermissionRequest/WebFetchPermissionRequest.tsx`
- `src/components/permissions/rules/AddPermissionRules.tsx`
- `src/components/permissions/rules/AddWorkspaceDirectory.tsx`
- `src/components/permissions/rules/PermissionRuleDescription.tsx`
- `src/components/permissions/rules/PermissionRuleInput.tsx`
- `src/components/permissions/rules/PermissionRuleList.tsx`
- `src/components/permissions/rules/RecentDenialsTab.tsx`
- `src/components/permissions/rules/RemoveWorkspaceDirectory.tsx`
- `src/components/permissions/rules/WorkspaceTab.tsx`
- `src/components/permissions/shellPermissionHelpers.tsx`
- `src/components/permissions/useShellPermissionFeedback.ts`
- `src/components/sandbox/SandboxConfigTab.tsx`
- `src/components/sandbox/SandboxDependenciesTab.tsx`
- `src/components/sandbox/SandboxDoctorSection.tsx`
- `src/components/sandbox/SandboxOverridesTab.tsx`
- `src/components/sandbox/SandboxSettings.tsx`
- `src/components/shell/ExpandShellOutputContext.tsx`
- `src/components/shell/OutputLine.tsx`
- `src/components/shell/ShellProgressMessage.tsx`
- `src/components/shell/ShellTimeDisplay.tsx`
- `src/components/tasks/AsyncAgentDetailDialog.tsx`
- `src/components/tasks/BackgroundTask.tsx`
- `src/components/tasks/BackgroundTaskStatus.tsx`
- `src/components/tasks/BackgroundTasksDialog.tsx`
- `src/components/tasks/DreamDetailDialog.tsx`
- `src/components/tasks/InProcessTeammateDetailDialog.tsx`
- `src/components/tasks/ShellDetailDialog.tsx`
- `src/components/tasks/ShellProgress.tsx`
- `src/components/tasks/renderToolActivity.tsx`
- `src/components/tasks/taskStatusUtils.tsx`
- `src/components/teams/TeamStatus.tsx`
- `src/components/teams/TeamsDialog.tsx`
- `src/components/ui/OrderedList.tsx`
- `src/components/ui/OrderedListItem.tsx`
- `src/components/ui/TreeSelect.tsx`
- `src/components/wizard/WizardDialogLayout.tsx`
- `src/components/wizard/WizardNavigationFooter.tsx`
- `src/components/wizard/WizardProvider.tsx`
- `src/components/wizard/useWizard.ts`
- `src/ink/clearTerminal.ts`
- `src/ink/components/AlternateScreen.tsx`
- `src/ink/components/AppContext.ts`
- `src/ink/components/Box.tsx`
- `src/ink/components/Button.tsx`
- `src/ink/components/ClockContext.tsx`
- `src/ink/components/CursorDeclarationContext.ts`
- `src/ink/components/ErrorOverview.tsx`
- `src/ink/components/Link.tsx`
- `src/ink/components/Newline.tsx`
- `src/ink/components/NoSelect.tsx`
- `src/ink/components/RawAnsi.tsx`
- `src/ink/components/ScrollBox.tsx`
- `src/ink/components/Spacer.tsx`
- `src/ink/components/StdinContext.ts`
- `src/ink/components/TerminalFocusContext.tsx`
- `src/ink/components/TerminalSizeContext.tsx`
- `src/ink/components/Text.tsx`
- `src/ink/dom.ts`
- `src/ink/events/click-event.ts`
- `src/ink/events/dispatcher.ts`
- `src/ink/events/emitter.ts`
- `src/ink/events/event-handlers.ts`
- `src/ink/events/focus-event.ts`
- `src/ink/events/input-event.ts`
- `src/ink/events/keyboard-event.ts`
- `src/ink/events/terminal-event.ts`
- `src/ink/events/terminal-focus-event.ts`
- `src/ink/focus.ts`
- `src/ink/frame.ts`
- `src/ink/get-max-width.ts`
- `src/ink/hit-test.ts`
- `src/ink/hooks/use-animation-frame.ts`
- `src/ink/hooks/use-declared-cursor.ts`
- `src/ink/hooks/use-input.ts`
- `src/ink/hooks/use-interval.ts`
- `src/ink/hooks/use-search-highlight.ts`
- `src/ink/hooks/use-selection.ts`
- `src/ink/hooks/use-tab-status.ts`
- `src/ink/hooks/use-terminal-focus.ts`
- `src/ink/hooks/use-terminal-title.ts`
- `src/ink/hooks/use-terminal-viewport.ts`
- `src/ink/instances.ts`
- `src/ink/layout/geometry.ts`
- `src/ink/layout/node.ts`
- `src/ink/layout/yoga.ts`
- `src/ink/line-width-cache.ts`
- `src/ink/log-update.ts`
- `src/ink/measure-element.ts`
- `src/ink/measure-text.ts`
- `src/ink/node-cache.ts`
- `src/ink/output.ts`
- `src/ink/parse-keypress.ts`
- `src/ink/render-border.ts`
- `src/ink/render-node-to-output.ts`
- `src/ink/render-to-screen.ts`
- `src/ink/squash-text-nodes.ts`
- `src/ink/stringWidth.ts`
- `src/ink/supports-hyperlinks.ts`
- `src/ink/termio.ts`
- `src/ink/termio/ansi.ts`
- `src/ink/termio/csi.ts`
- `src/ink/termio/dec.ts`
- `src/ink/termio/esc.ts`
- `src/ink/termio/sgr.ts`
- `src/ink/termio/tokenize.ts`
- `src/ink/useTerminalNotification.ts`
- `src/ink/widest-line.ts`
- `src/ink/wrapAnsi.ts`

### → `42-misc` (302 files)

- `src/constants/cyberRiskInstruction.ts`
- `src/constants/errorIds.ts`
- `src/constants/figures.ts`
- `src/constants/files.ts`
- `src/constants/github-app.ts`
- `src/constants/product.ts`
- `src/constants/spinnerVerbs.ts`
- `src/constants/systemPromptSections.ts`
- `src/constants/toolLimits.ts`
- `src/types/generated/events_mono/growthbook/v1/growthbook_experiment_event.ts`
- `src/utils/CircularBuffer.ts`
- `src/utils/Cursor.ts`
- `src/utils/QueryGuard.ts`
- `src/utils/ShellCommand.ts`
- `src/utils/abortController.ts`
- `src/utils/activityManager.ts`
- `src/utils/agentContext.ts`
- `src/utils/ansiToPng.ts`
- `src/utils/ansiToSvg.ts`
- `src/utils/apiPreconnect.ts`
- `src/utils/appleTerminalBackup.ts`
- `src/utils/argumentSubstitution.ts`
- `src/utils/array.ts`
- `src/utils/asciicast.ts`
- `src/utils/authPortable.ts`
- `src/utils/autoModeDenials.ts`
- `src/utils/autoRunIssue.tsx`
- `src/utils/autoUpdater.ts`
- `src/utils/aws.ts`
- `src/utils/awsAuthStatusManager.ts`
- `src/utils/bash/ParsedCommand.ts`
- `src/utils/bash/ShellSnapshot.ts`
- `src/utils/bash/ast.ts`
- `src/utils/bash/bashPipeCommand.ts`
- `src/utils/bash/heredoc.ts`
- `src/utils/bash/prefix.ts`
- `src/utils/bash/registry.ts`
- `src/utils/bash/shellCompletion.ts`
- `src/utils/bash/shellPrefix.ts`
- `src/utils/bash/shellQuote.ts`
- `src/utils/bash/shellQuoting.ts`
- `src/utils/bash/specs/alias.ts`
- `src/utils/bash/specs/pyright.ts`
- `src/utils/bash/specs/sleep.ts`
- `src/utils/bash/specs/srun.ts`
- `src/utils/bash/specs/timeout.ts`
- `src/utils/bash/treeSitterAnalysis.ts`
- `src/utils/binaryCheck.ts`
- `src/utils/browser.ts`
- `src/utils/bufferedWriter.ts`
- `src/utils/caCertsConfig.ts`
- `src/utils/cachePaths.ts`
- `src/utils/classifierApprovals.ts`
- `src/utils/classifierApprovalsHook.ts`
- `src/utils/claudeCodeHints.ts`
- `src/utils/claudeDesktop.ts`
- `src/utils/claudeInChrome/chromeNativeHost.ts`
- `src/utils/claudeInChrome/common.ts`
- `src/utils/claudeInChrome/mcpServer.ts`
- `src/utils/claudeInChrome/setupPortable.ts`
- `src/utils/claudeInChrome/toolRendering.tsx`
- `src/utils/cleanup.ts`
- `src/utils/cliArgs.ts`
- `src/utils/cliHighlight.ts`
- `src/utils/codeIndexing.ts`
- `src/utils/collapseBackgroundBashNotifications.ts`
- `src/utils/collapseHookSummaries.ts`
- `src/utils/collapseTeammateShutdowns.ts`
- `src/utils/combinedAbortSignal.ts`
- `src/utils/commandLifecycle.ts`
- `src/utils/completionCache.ts`
- `src/utils/computerUse/appNames.ts`
- `src/utils/computerUse/cleanup.ts`
- `src/utils/computerUse/common.ts`
- `src/utils/computerUse/computerUseLock.ts`
- `src/utils/computerUse/drainRunLoop.ts`
- `src/utils/computerUse/escHotkey.ts`
- `src/utils/computerUse/executor.ts`
- `src/utils/computerUse/gates.ts`
- `src/utils/computerUse/hostAdapter.ts`
- `src/utils/computerUse/inputLoader.ts`
- `src/utils/computerUse/mcpServer.ts`
- `src/utils/computerUse/swiftLoader.ts`
- `src/utils/computerUse/toolRendering.tsx`
- `src/utils/computerUse/wrapper.tsx`
- `src/utils/contentArray.ts`
- `src/utils/contextAnalysis.ts`
- `src/utils/contextSuggestions.ts`
- `src/utils/controlMessageCompat.ts`
- `src/utils/cron.ts`
- `src/utils/cronTasksLock.ts`
- `src/utils/crossProjectResume.ts`
- `src/utils/debugFilter.ts`
- `src/utils/deepLink/banner.ts`
- `src/utils/deepLink/parseDeepLink.ts`
- `src/utils/deepLink/protocolHandler.ts`
- `src/utils/deepLink/registerProtocol.ts`
- `src/utils/deepLink/terminalLauncher.ts`
- `src/utils/deepLink/terminalPreference.ts`
- `src/utils/desktopDeepLink.ts`
- `src/utils/detectRepository.ts`
- `src/utils/directMemberMessage.ts`
- `src/utils/displayTags.ts`
- `src/utils/doctorDiagnostic.ts`
- `src/utils/dxt/helpers.ts`
- `src/utils/dxt/zip.ts`
- `src/utils/earlyInput.ts`
- `src/utils/editor.ts`
- `src/utils/env.ts`
- `src/utils/envValidation.ts`
- `src/utils/errorLogSink.ts`
- `src/utils/exampleCommands.ts`
- `src/utils/execFileNoThrowPortable.ts`
- `src/utils/execSyncWrapper.ts`
- `src/utils/exportRenderer.tsx`
- `src/utils/extraUsage.ts`
- `src/utils/filePersistence/outputsScanner.ts`
- `src/utils/fileReadCache.ts`
- `src/utils/findExecutable.ts`
- `src/utils/fingerprint.ts`
- `src/utils/format.ts`
- `src/utils/formatBriefTimestamp.ts`
- `src/utils/fpsTracker.ts`
- `src/utils/fullscreen.ts`
- `src/utils/generatedFiles.ts`
- `src/utils/generators.ts`
- `src/utils/genericProcessUtils.ts`
- `src/utils/getWorktreePaths.ts`
- `src/utils/getWorktreePathsPortable.ts`
- `src/utils/ghPrStatus.ts`
- `src/utils/git/gitConfigParser.ts`
- `src/utils/git/gitFilesystem.ts`
- `src/utils/git/gitignore.ts`
- `src/utils/gitDiff.ts`
- `src/utils/github/ghAuthStatus.ts`
- `src/utils/githubRepoPathMapping.ts`
- `src/utils/gracefulShutdown.ts`
- `src/utils/groupToolUses.ts`
- `src/utils/hash.ts`
- `src/utils/headlessProfiler.ts`
- `src/utils/heapDumpService.ts`
- `src/utils/heatmap.ts`
- `src/utils/highlightMatch.tsx`
- `src/utils/horizontalScroll.ts`
- `src/utils/hyperlink.ts`
- `src/utils/iTermBackup.ts`
- `src/utils/ide.ts`
- `src/utils/idePathConversion.ts`
- `src/utils/idleTimeout.ts`
- `src/utils/imageStore.ts`
- `src/utils/imageValidation.ts`
- `src/utils/inProcessTeammateHelpers.ts`
- `src/utils/intl.ts`
- `src/utils/jetbrains.ts`
- `src/utils/json.ts`
- `src/utils/jsonRead.ts`
- `src/utils/keyboardShortcuts.ts`
- `src/utils/localInstaller.ts`
- `src/utils/lockfile.ts`
- `src/utils/logoV2Utils.ts`
- `src/utils/mailbox.ts`
- `src/utils/managedEnv.ts`
- `src/utils/managedEnvConstants.ts`
- `src/utils/markdown.ts`
- `src/utils/markdownConfigLoader.ts`
- `src/utils/mcp/dateTimeParser.ts`
- `src/utils/mcp/elicitationValidation.ts`
- `src/utils/mcpInstructionsDelta.ts`
- `src/utils/mcpOutputStorage.ts`
- `src/utils/mcpValidation.ts`
- `src/utils/mcpWebSocketTransport.ts`
- `src/utils/memoize.ts`
- `src/utils/messagePredicates.ts`
- `src/utils/messages/mappers.ts`
- `src/utils/model/agent.ts`
- `src/utils/model/aliases.ts`
- `src/utils/model/antModels.ts`
- `src/utils/model/bedrock.ts`
- `src/utils/model/check1mAccess.ts`
- `src/utils/model/configs.ts`
- `src/utils/model/contextWindowUpgradeCheck.ts`
- `src/utils/model/deprecation.ts`
- `src/utils/model/model.ts`
- `src/utils/model/modelAllowlist.ts`
- `src/utils/model/modelCapabilities.ts`
- `src/utils/model/modelOptions.ts`
- `src/utils/model/modelStrings.ts`
- `src/utils/model/modelSupportOverrides.ts`
- `src/utils/model/providers.ts`
- `src/utils/model/validateModel.ts`
- `src/utils/modifiers.ts`
- `src/utils/nativeInstaller/installer.ts`
- `src/utils/nativeInstaller/packageManagers.ts`
- `src/utils/nativeInstaller/pidLock.ts`
- `src/utils/objectGroupBy.ts`
- `src/utils/pasteStore.ts`
- `src/utils/peerAddress.ts`
- `src/utils/platform.ts`
- `src/utils/powershell/dangerousCmdlets.ts`
- `src/utils/powershell/staticPrefix.ts`
- `src/utils/preflightChecks.tsx`
- `src/utils/process.ts`
- `src/utils/processUserInput/processBashCommand.tsx`
- `src/utils/processUserInput/processTextPrompt.ts`
- `src/utils/processUserInput/processUserInput.ts`
- `src/utils/profilerBase.ts`
- `src/utils/promptEditor.ts`
- `src/utils/promptShellExecution.ts`
- `src/utils/queryProfiler.ts`
- `src/utils/queueProcessor.ts`
- `src/utils/readEditContext.ts`
- `src/utils/releaseNotes.ts`
- `src/utils/sandbox/sandbox-ui-utils.ts`
- `src/utils/sanitization.ts`
- `src/utils/screenshotClipboard.ts`
- `src/utils/sdkEventQueue.ts`
- `src/utils/semver.ts`
- `src/utils/sequential.ts`
- `src/utils/set.ts`
- `src/utils/shell/bashProvider.ts`
- `src/utils/shell/powershellDetection.ts`
- `src/utils/shell/powershellProvider.ts`
- `src/utils/shell/prefix.ts`
- `src/utils/shell/readOnlyCommandValidation.ts`
- `src/utils/shell/resolveDefaultShell.ts`
- `src/utils/shell/specPrefix.ts`
- `src/utils/shellConfig.ts`
- `src/utils/sideQuery.ts`
- `src/utils/sideQuestion.ts`
- `src/utils/signal.ts`
- `src/utils/slashCommandParsing.ts`
- `src/utils/sleep.ts`
- `src/utils/sliceAnsi.ts`
- `src/utils/standaloneAgent.ts`
- `src/utils/startupProfiler.ts`
- `src/utils/staticRender.tsx`
- `src/utils/statsCache.ts`
- `src/utils/statusNoticeDefinitions.tsx`
- `src/utils/statusNoticeHelpers.ts`
- `src/utils/stream.ts`
- `src/utils/streamJsonStdoutGuard.ts`
- `src/utils/streamlinedTransform.ts`
- `src/utils/stringUtils.ts`
- `src/utils/subprocessEnv.ts`
- `src/utils/suggestions/directoryCompletion.ts`
- `src/utils/suggestions/slackChannelSuggestions.ts`
- `src/utils/swarm/It2SetupPrompt.tsx`
- `src/utils/swarm/backends/ITermBackend.ts`
- `src/utils/swarm/backends/InProcessBackend.ts`
- `src/utils/swarm/backends/PaneBackendExecutor.ts`
- `src/utils/swarm/backends/TmuxBackend.ts`
- `src/utils/swarm/backends/detection.ts`
- `src/utils/swarm/backends/it2Setup.ts`
- `src/utils/swarm/backends/registry.ts`
- `src/utils/swarm/backends/teammateModeSnapshot.ts`
- `src/utils/swarm/inProcessRunner.ts`
- `src/utils/swarm/permissionSync.ts`
- `src/utils/swarm/reconnection.ts`
- `src/utils/swarm/spawnInProcess.ts`
- `src/utils/swarm/teamHelpers.ts`
- `src/utils/swarm/teammateInit.ts`
- `src/utils/swarm/teammateLayoutManager.ts`
- `src/utils/swarm/teammateModel.ts`
- `src/utils/swarm/teammatePromptAddendum.ts`
- `src/utils/systemDirectories.ts`
- `src/utils/systemTheme.ts`
- `src/utils/taggedId.ts`
- `src/utils/task/TaskOutput.ts`
- `src/utils/task/diskOutput.ts`
- `src/utils/task/outputFormatting.ts`
- `src/utils/task/sdkProgress.ts`
- `src/utils/teamDiscovery.ts`
- `src/utils/teammate.ts`
- `src/utils/teammateContext.ts`
- `src/utils/telemetryAttributes.ts`
- `src/utils/teleport/environmentSelection.ts`
- `src/utils/teleport/environments.ts`
- `src/utils/teleport/gitBundle.ts`
- `src/utils/tempfile.ts`
- `src/utils/terminalPanel.ts`
- `src/utils/textHighlighting.ts`
- `src/utils/tmuxSocket.ts`
- `src/utils/tokens.ts`
- `src/utils/toolErrors.ts`
- `src/utils/toolPool.ts`
- `src/utils/toolSchemaCache.ts`
- `src/utils/transcriptSearch.ts`
- `src/utils/treeify.ts`
- `src/utils/truncate.ts`
- `src/utils/ultraplan/keyword.ts`
- `src/utils/unaryLogging.ts`
- `src/utils/userPromptKeywords.ts`
- `src/utils/uuid.ts`
- `src/utils/warningHandler.ts`
- `src/utils/which.ts`
- `src/utils/windowsPaths.ts`
- `src/utils/withResolvers.ts`
- `src/utils/words.ts`
- `src/utils/workloadContext.ts`
- `src/utils/xdg.ts`
- `src/utils/yaml.ts`
- `src/utils/zodToJsonSchema.ts`

### → `37-ink-ui-shell or 04-turn-pipeline` (92 files)

- `src/hooks/fileSuggestions.ts`
- `src/hooks/notifs/useAutoModeUnavailableNotification.ts`
- `src/hooks/notifs/useCanSwitchToExistingSubscription.tsx`
- `src/hooks/notifs/useDeprecationWarningNotification.tsx`
- `src/hooks/notifs/useFastModeNotification.tsx`
- `src/hooks/notifs/useIDEStatusIndicator.tsx`
- `src/hooks/notifs/useInstallMessages.tsx`
- `src/hooks/notifs/useMcpConnectivityStatus.tsx`
- `src/hooks/notifs/useModelMigrationNotifications.tsx`
- `src/hooks/notifs/useNpmDeprecationNotification.tsx`
- `src/hooks/notifs/usePluginAutoupdateNotification.tsx`
- `src/hooks/notifs/useRateLimitWarningNotification.tsx`
- `src/hooks/notifs/useSettingsErrors.tsx`
- `src/hooks/notifs/useStartupNotification.ts`
- `src/hooks/notifs/useTeammateShutdownNotification.ts`
- `src/hooks/renderPlaceholder.ts`
- `src/hooks/unifiedSuggestions.ts`
- `src/hooks/useAfterFirstRender.ts`
- `src/hooks/useApiKeyVerification.ts`
- `src/hooks/useBackgroundTaskNavigation.ts`
- `src/hooks/useBlink.ts`
- `src/hooks/useCancelRequest.ts`
- `src/hooks/useChromeExtensionNotification.tsx`
- `src/hooks/useClaudeCodeHintRecommendation.tsx`
- `src/hooks/useClipboardImageHint.ts`
- `src/hooks/useCommandKeybindings.tsx`
- `src/hooks/useCommandQueue.ts`
- `src/hooks/useCopyOnSelect.ts`
- `src/hooks/useDeferredHookMessages.ts`
- `src/hooks/useDiffData.ts`
- `src/hooks/useDiffInIDE.ts`
- `src/hooks/useDirectConnect.ts`
- `src/hooks/useDoublePress.ts`
- `src/hooks/useDynamicConfig.ts`
- `src/hooks/useElapsedTime.ts`
- `src/hooks/useExitOnCtrlCD.ts`
- `src/hooks/useExitOnCtrlCDWithKeybindings.ts`
- `src/hooks/useGlobalKeybindings.tsx`
- `src/hooks/useIDEIntegration.tsx`
- `src/hooks/useIdeAtMentioned.ts`
- `src/hooks/useIdeConnectionStatus.ts`
- `src/hooks/useIdeLogging.ts`
- `src/hooks/useIdeSelection.ts`
- `src/hooks/useInboxPoller.ts`
- `src/hooks/useInputBuffer.ts`
- `src/hooks/useIssueFlagBanner.ts`
- `src/hooks/useLspPluginRecommendation.tsx`
- `src/hooks/useMainLoopModel.ts`
- `src/hooks/useMergedClients.ts`
- `src/hooks/useMergedCommands.ts`
- `src/hooks/useMinDisplayTime.ts`
- `src/hooks/useNotifyAfterTimeout.ts`
- `src/hooks/useOfficialMarketplaceNotification.tsx`
- `src/hooks/usePasteHandler.ts`
- `src/hooks/usePluginRecommendationBase.tsx`
- `src/hooks/usePrStatus.ts`
- `src/hooks/usePromptSuggestion.ts`
- `src/hooks/usePromptsFromClaudeInChrome.tsx`
- `src/hooks/useQueueProcessor.ts`
- `src/hooks/useScheduledTasks.ts`
- `src/hooks/useSearchInput.ts`
- `src/hooks/useSettings.ts`
- `src/hooks/useSettingsChange.ts`
- `src/hooks/useSkillImprovementSurvey.ts`
- `src/hooks/useSkillsChange.ts`
- `src/hooks/useSwarmInitialization.ts`
- `src/hooks/useSwarmPermissionPoller.ts`
- `src/hooks/useTaskListWatcher.ts`
- `src/hooks/useTasksV2.ts`
- `src/hooks/useTeammateViewAutoExit.ts`
- `src/hooks/useTeleportResume.tsx`
- `src/hooks/useTerminalSize.ts`
- `src/hooks/useTextInput.ts`
- `src/hooks/useTimeout.ts`
- `src/hooks/useTurnDiffs.ts`
- `src/hooks/useTypeahead.tsx`
- `src/hooks/useUpdateNotification.ts`
- `src/hooks/useVirtualScroll.ts`
- `src/utils/hooks/AsyncHookRegistry.ts`
- `src/utils/hooks/apiQueryHookHelper.ts`
- `src/utils/hooks/execAgentHook.ts`
- `src/utils/hooks/execHttpHook.ts`
- `src/utils/hooks/execPromptHook.ts`
- `src/utils/hooks/fileChangedWatcher.ts`
- `src/utils/hooks/hookEvents.ts`
- `src/utils/hooks/hookHelpers.ts`
- `src/utils/hooks/hooksConfigManager.ts`
- `src/utils/hooks/hooksConfigSnapshot.ts`
- `src/utils/hooks/hooksSettings.ts`
- `src/utils/hooks/registerFrontmatterHooks.ts`
- `src/utils/hooks/registerSkillHooks.ts`
- `src/utils/hooks/ssrfGuard.ts`

### → `21-command-catalog (a/b/c)` (72 files)

- `src/commands/add-dir/add-dir.tsx`
- `src/commands/chrome/chrome.tsx`
- `src/commands/context/context-noninteractive.ts`
- `src/commands/context/context.tsx`
- `src/commands/copy/copy.tsx`
- `src/commands/desktop/desktop.tsx`
- `src/commands/diff/diff.tsx`
- `src/commands/doctor/doctor.tsx`
- `src/commands/effort/effort.tsx`
- `src/commands/export/export.tsx`
- `src/commands/extra-usage/extra-usage-noninteractive.ts`
- `src/commands/extra-usage/extra-usage.tsx`
- `src/commands/fast/fast.tsx`
- `src/commands/feedback/feedback.tsx`
- `src/commands/heapdump/heapdump.ts`
- `src/commands/help/help.tsx`
- `src/commands/ide/ide.tsx`
- `src/commands/install-github-app/ApiKeyStep.tsx`
- `src/commands/install-github-app/CheckExistingSecretStep.tsx`
- `src/commands/install-github-app/CheckGitHubStep.tsx`
- `src/commands/install-github-app/ChooseRepoStep.tsx`
- `src/commands/install-github-app/CreatingStep.tsx`
- `src/commands/install-github-app/ErrorStep.tsx`
- `src/commands/install-github-app/ExistingWorkflowStep.tsx`
- `src/commands/install-github-app/InstallAppStep.tsx`
- `src/commands/install-github-app/OAuthFlowStep.tsx`
- `src/commands/install-github-app/SuccessStep.tsx`
- `src/commands/install-github-app/WarningsStep.tsx`
- `src/commands/install-github-app/install-github-app.tsx`
- `src/commands/install-slack-app/install-slack-app.ts`
- `src/commands/install.tsx`
- `src/commands/keybindings/keybindings.ts`
- `src/commands/login/login.tsx`
- `src/commands/logout/logout.tsx`
- `src/commands/memory/memory.tsx`
- `src/commands/mobile/mobile.tsx`
- `src/commands/passes/passes.tsx`
- `src/commands/permissions/permissions.tsx`
- `src/commands/plan/plan.tsx`
- `src/commands/plugin/AddMarketplace.tsx`
- `src/commands/plugin/BrowseMarketplace.tsx`
- `src/commands/plugin/DiscoverPlugins.tsx`
- `src/commands/plugin/ManageMarketplaces.tsx`
- `src/commands/plugin/ManagePlugins.tsx`
- `src/commands/plugin/PluginErrors.tsx`
- `src/commands/plugin/PluginOptionsDialog.tsx`
- `src/commands/plugin/PluginOptionsFlow.tsx`
- `src/commands/plugin/PluginSettings.tsx`
- `src/commands/plugin/PluginTrustWarning.tsx`
- `src/commands/plugin/UnifiedInstalledCell.tsx`
- `src/commands/plugin/ValidatePlugin.tsx`
- `src/commands/plugin/pluginDetailsHelpers.tsx`
- `src/commands/plugin/usePagination.ts`
- `src/commands/privacy-settings/privacy-settings.tsx`
- `src/commands/rate-limit-options/rate-limit-options.tsx`
- `src/commands/release-notes/release-notes.ts`
- `src/commands/remote-env/remote-env.tsx`
- `src/commands/resume/resume.tsx`
- `src/commands/review/ultrareviewCommand.tsx`
- `src/commands/rewind/rewind.ts`
- `src/commands/sandbox-toggle/sandbox-toggle.tsx`
- `src/commands/session/session.tsx`
- `src/commands/status/status.tsx`
- `src/commands/stickers/stickers.ts`
- `src/commands/tag/tag.tsx`
- `src/commands/tasks/tasks.tsx`
- `src/commands/terminalSetup/terminalSetup.tsx`
- `src/commands/theme/theme.tsx`
- `src/commands/thinkback-play/thinkback-play.ts`
- `src/commands/thinkback/thinkback.tsx`
- `src/commands/upgrade/upgrade.tsx`
- `src/commands/usage/usage.tsx`

### → `17-tool-skill` (16 files)

- `src/commands/skills/skills.tsx`
- `src/components/skills/SkillsMenu.tsx`
- `src/skills/bundled/batch.ts`
- `src/skills/bundled/claudeApi.ts`
- `src/skills/bundled/claudeApiContent.ts`
- `src/skills/bundled/claudeInChrome.ts`
- `src/skills/bundled/debug.ts`
- `src/skills/bundled/keybindings.ts`
- `src/skills/bundled/loremIpsum.ts`
- `src/skills/bundled/remember.ts`
- `src/skills/bundled/simplify.ts`
- `src/skills/bundled/stuck.ts`
- `src/skills/bundled/updateConfig.ts`
- `src/skills/bundled/verify.ts`
- `src/skills/bundled/verifyContent.ts`
- `src/utils/skills/skillChangeDetector.ts`

### → `41-session-state-history` (16 files)

- `src/hooks/useArrowKeyHistory.tsx`
- `src/hooks/useAssistantHistory.ts`
- `src/hooks/useFileHistorySnapshotInit.ts`
- `src/hooks/useRemoteSession.ts`
- `src/hooks/useSSHSession.ts`
- `src/hooks/useSessionBackgrounding.ts`
- `src/utils/agenticSessionSearch.ts`
- `src/utils/hooks/sessionHooks.ts`
- `src/utils/listSessionsImpl.ts`
- `src/utils/sessionEnvVars.ts`
- `src/utils/sessionEnvironment.ts`
- `src/utils/sessionIngressAuth.ts`
- `src/utils/sessionTitle.ts`
- `src/utils/sessionUrl.ts`
- `src/utils/suggestions/shellHistoryCompletion.ts`
- `src/utils/ultraplan/ccrSession.ts`

### → `11-tool-files` (9 files)

- `src/tools/BriefTool/upload.ts`
- `src/tools/PowerShellTool/clmTypes.ts`
- `src/tools/PowerShellTool/commonParameters.ts`
- `src/tools/PowerShellTool/gitSafety.ts`
- `src/tools/PowerShellTool/powershellPermissions.ts`
- `src/tools/PowerShellTool/powershellSecurity.ts`
- `src/tools/ScheduleCronTool/CronCreateTool.ts`
- `src/tools/ScheduleCronTool/CronDeleteTool.ts`
- `src/tools/ScheduleCronTool/CronListTool.ts`

### → `29-service-memory or 40-persistent-memory` (8 files)

- `src/components/MemoryUsageIndicator.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/MemoryStep.tsx`
- `src/components/memory/MemoryFileSelector.tsx`
- `src/components/memory/MemoryUpdateNotification.tsx`
- `src/components/messages/UserMemoryInputMessage.tsx`
- `src/hooks/useMemoryUsage.ts`
- `src/utils/memory/versions.ts`
- `src/utils/teamMemoryOps.ts`

### → `01-entrypoint-bootstrap` (6 files)

- `src/cli/transports/HybridTransport.ts`
- `src/cli/transports/SSETransport.ts`
- `src/cli/transports/SerialBatchEventUploader.ts`
- `src/cli/transports/WorkerStateUploader.ts`
- `src/cli/transports/ccrClient.ts`
- `src/cli/transports/transportUtils.ts`

### → `28-service-plugins` (6 files)

- `src/utils/plugins/gitAvailability.ts`
- `src/utils/plugins/hintRecommendation.ts`
- `src/utils/plugins/lspRecommendation.ts`
- `src/utils/plugins/performStartupChecks.tsx`
- `src/utils/plugins/pluginFlagging.ts`
- `src/utils/plugins/validatePlugin.ts`

### → `35-mode-remote-server` (5 files)

- `src/components/RemoteCallout.tsx`
- `src/components/RemoteEnvironmentDialog.tsx`
- `src/components/tasks/RemoteSessionDetailDialog.tsx`
- `src/components/tasks/RemoteSessionProgress.tsx`
- `src/utils/background/remote/preconditions.ts`

### → `34-mode-bridge` (2 files)

- `src/hooks/useMailboxBridge.ts`
- `src/utils/swarm/leaderPermissionBridge.ts`

### → `14-tool-agent-team or 30-coordinator` (1 files)

- `src/components/CoordinatorAgentStatus.tsx`

### → `10-tool-bash` (1 files)

- `src/components/permissions/BashPermissionRequest/bashToolUseOptions.tsx`

## Trivial Residual Categories

- **tiny-stub** (10 files):
  - `src/components/Spinner/teammateSelectHint.ts`
  - `src/constants/turnCompletionVerbs.ts`
  - `src/ink/events/event.ts`
  - `src/ink/hooks/use-app.ts`
  - `src/ink/hooks/use-stdin.ts`

## Bundled Artifacts

- `src/main.tsx` (803,924 bytes) — primary bundle, intentionally excluded from residual count.

Other large files (>100KB) sampled inline are real source (e.g., `src/commands/plugin/ManagePlugins.tsx` 322KB, `src/hooks/useTypeahead.tsx` 213KB), not bundles. They appear in residuals because their host specs (21c, 37) did not cite them by `src/`-prefixed path.


## Methodology Notes

- Strict mode: matched only `src/[A-Za-z0-9_/.-]+\.(ts|tsx|js|json)` regex against spec text.
- Permissive mode: also matched any path-like token (without `src/` prefix) against the suffix of any source file.
- Specs scanned: all `docs/specs/*.md` except `INDEX.md` and `HANDOFF.md` (46 files total).
- Bundled artifact filter: only `src/main.tsx` was hard-excluded; large non-bundled `.tsx` files remain in residuals because they truly are uncited source.