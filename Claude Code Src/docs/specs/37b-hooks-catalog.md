# 37b — Hooks Catalog

## §0 Scope

Catalog companion to spec 37 (Ink UI shell). Spec 37 covers the rendering loop,
overall App composition, and the high-level hook patterns; this spec enumerates
every file under `src/hooks/`.

- **Files cataloged:** 104 (`*.ts` + `*.tsx`)
- **Total LOC:** ~19,204
- **Top-level layout:**
  - `src/hooks/*.ts(x)` — flat collection of REPL-level hooks (~80 files)
  - `src/hooks/notifs/*.tsx` — startup/runtime notification surfaces (17 files)
  - `src/hooks/toolPermission/*.ts` — tool-permission engine (5 files,
    architecturally part of spec 09 permission model)

Note on terminology: this directory implements **React hooks** (the `useFoo()`
function-component primitive). It is unrelated to the Claude Code "user hooks"
config system (PreToolUse/PostToolUse/etc.) — that machinery lives in
`src/utils/hooks/` and is owned by spec 26 (services–hooks). A handful of files
in this directory **do** consume the user-hook system (`useDeferredHookMessages`,
the toolPermission/* handlers); those cross-references are called out per-entry.

## §1 Hook Categories

The 104 files fall into these clusters:

1. **Input handling & key dispatch** — `useTypeahead`, `useTextInput`,
   `useVimInput`, `useArrowKeyHistory`, `useHistorySearch`, `useSearchInput`,
   `useInputBuffer`, `usePasteHandler`, `useGlobalKeybindings`,
   `useCommandKeybindings`, `useExitOnCtrlCD*`, `useDoublePress`,
   `useBackgroundTaskNavigation`, `usePromptSuggestion`,
   `renderPlaceholder`, `useCopyOnSelect`. The largest single cluster — and
   houses `useTypeahead.tsx` (213KB), the biggest hook in the codebase.

2. **Suggestions / autocomplete** — `fileSuggestions.ts`,
   `unifiedSuggestions.ts`, `usePromptSuggestion.ts`. Provide candidate
   feeds for the `@`-mention / `/`-command dropdown above the prompt.

3. **Tool permission engine** (`toolPermission/`) — `useCanUseTool` plus the
   3-handler split (`coordinatorHandler`, `interactiveHandler`,
   `swarmWorkerHandler`) and shared `PermissionContext` + `permissionLogging`.
   These really belong to spec **09 permission model**; only their consumer
   surface (the `useCanUseTool` hook) sits in `src/hooks/`.

4. **Voice input (feature-gated)** — `useVoice`, `useVoiceIntegration`,
   `useVoiceEnabled`. Gated by `feature('VOICE_MODE')`; tied into spec 38b.

5. **Diff / IDE integration** — `useDiffData`, `useDiffInIDE`, `useTurnDiffs`,
   `useFileHistorySnapshotInit`, `useIDEIntegration`, `useIdeAtMentioned`,
   `useIdeConnectionStatus`, `useIdeLogging`, `useIdeSelection`. Multi-spec
   touchpoint with spec 13 (file edit) and spec 27 (services-ide).

6. **Session / connection lifecycle** — `useRemoteSession`, `useSSHSession`,
   `useDirectConnect`, `useReplBridge`, `useTeleportResume`,
   `useSessionBackgrounding`, `useApiKeyVerification`, `useDeferredHookMessages`,
   `useAssistantHistory`, `useLogMessages`, `useMainLoopModel`. Glue between
   the REPL and the various wire transports (specs 32–35) and spec 41
   (session/state/history).

7. **Plugin / hint recommendation** — `usePluginRecommendationBase`,
   `useLspPluginRecommendation`, `useClaudeCodeHintRecommendation`,
   `useManagePlugins`, `useChromeExtensionNotification`,
   `usePromptsFromClaudeInChrome`, `useOfficialMarketplaceNotification`.
   Cross-ref spec 28 (services-plugins).

8. **Swarm / coordinator hooks** — `useSwarmInitialization`,
   `useSwarmPermissionPoller`, `useTeammateViewAutoExit`,
   `useBackgroundTaskNavigation`, `useMailboxBridge`. Cross-ref specs 30
   (coordinator) and 14 (agent/team).

9. **Task & queue plumbing** — `useTasksV2`, `useTaskListWatcher`,
   `useScheduledTasks`, `useCommandQueue`, `useQueueProcessor`,
   `useCancelRequest`, `usePrStatus`, `useIssueFlagBanner`. Cross-ref spec 39
   (tasks).

10. **Settings / config / merging** — `useSettings`, `useSettingsChange`,
    `useSkillsChange`, `useDynamicConfig`, `useMergedClients`,
    `useMergedCommands`, `useMergedTools`. Reactive bridges between
    AppState/disk and React.

11. **Display / animation primitives** — `useTerminalSize`, `useBlink`,
    `useElapsedTime`, `useTimeout`, `useNotifyAfterTimeout`,
    `useMinDisplayTime`, `useAfterFirstRender`, `useUpdateNotification`,
    `useVirtualScroll`, `useMemoryUsage`, `useClipboardImageHint`,
    `useAwaySummary`, `useSkillImprovementSurvey`.

12. **Notifications surface (`notifs/`)** — 17 hooks, each driving one
    transient notification card. The `useStartupNotification` helper is the
    once-per-session base used by ~half the others.

## §2 Hook Entries

### §2.1 Input handling & key dispatch

#### `src/hooks/useTypeahead.tsx` (213KB, ~1,384 LOC) — by far the biggest hook
**Role:** The unified prompt-input typeahead engine. Consumes the partial input
string + cursor offset and produces (a) a suggestion list (commands, files,
agents, teammates, slack channels, shell-completion items, history matches,
ghost text), (b) a key handler that the prompt input wires through `<Box
onKeyDown>` (Tab to accept, arrows to navigate, Esc to dismiss). Internally
manages a debounced async suggestion fetcher, an overlay registration with
`overlayContext` so other modals can suspend it, command/argument parsing via
`argumentSubstitution`, and per-input-mode (slash-command vs DM vs bash) routing
via `getModeFromInput`. Heavy state: `suggestionsState` (item list, ghost text,
arg hint, cancel token), refs for in-flight requests, debounced fetch
(`useDebounceCallback`). Wires `fileSuggestions.ts`, `commandSuggestions`,
`directoryCompletion`, `shellCompletion`, `shellHistoryCompletion`,
`slackChannelSuggestions`, the agent-name registry, and team-member registry.
**Cross-spec:** spec 37 (input shell), spec 17 (skill triggers via
`getSlashCommandToolSkills`), spec 21 (command catalog suggestions), spec 14
(agent / DM @-mention), spec 31 (mailbox / member registry).
**Flag gates:** `isAgentSwarmsEnabled()` (DM teammate suggestions).

#### `src/hooks/useTextInput.ts` (17KB, 529 LOC)
**Role:** The Cursor-state controller for the prompt input. Translates raw key
events into Cursor mutations (insert, backspace, kill-line, yank, kill-ring
ops). Owns the kill-ring + yank-pop loop in `utils/Cursor.ts`. Branches into
vim mode via `useVimInput` when enabled. Also wires modifier prewarming
(`prewarmModifiers`) so Alt-key combos work cross-platform.
**Cross-spec:** spec 37; spec 38a (vim mode).

#### `src/hooks/useVimInput.ts` (~316 LOC)
**Role:** Vim-mode state machine adapter for the prompt input. Wraps
`vim/transitions.ts` and `vim/operators.ts`, exposing a key-event handler that
mutates `VimInputState` (mode, count, register, op-pending). Returns the new
Cursor + mode string for the input frame.
**Cross-spec:** spec 38a (vim mode).

#### `src/hooks/useArrowKeyHistory.tsx` (34KB, 228 LOC)
**Role:** Up/down arrow walks through `~/.claude/history.jsonl`. Does
chunked, batched disk reads (10-entry HISTORY_CHUNK_SIZE, shared `pendingLoad`
promise) so that holding the up arrow doesn't fan out into N readFile calls.
Filters by current PromptInputMode so bash-mode history doesn't bleed into
text-mode. Renders a `ConfigurableShortcutHint` for "press Esc to abandon".
**Cross-spec:** spec 41 (session/history).

#### `src/hooks/useHistorySearch.ts` (303 LOC)
**Role:** Reverse-i-search style fuzzy search over history. Same chunked
read pattern as `useArrowKeyHistory`. Distinct dialog state (cursor over a
filtered result list) registered via the keybinding context.
**Cross-spec:** spec 41.

#### `src/hooks/useSearchInput.ts` (10KB, 364 LOC)
**Role:** Generic input controller for in-app search dialogs (history,
quick-open, transcript search). Owns its own Cursor + kill-ring like
`useTextInput` but with simpler semantics — Esc abandons, Enter commits,
no vim, no paste affordances.

#### `src/hooks/useInputBuffer.ts` (132 LOC)
**Role:** Tiny ring-buffer for prompt input drafts so that a user navigating
away (e.g. opening /help) and returning lands back on what they typed.
Returns `pushToBuffer`, `peek`, `pop` callbacks; bounded by `maxBufferSize`.

#### `src/hooks/usePasteHandler.ts` (10KB, 285 LOC)
**Role:** Clipboard / bracketed-paste detector. Distinguishes between a real
"paste" (multi-char input event arriving inside `PASTE_THRESHOLD` ms) and
typed input. Auto-detects clipboard image content via `getImageFromClipboard`
and image file paths via `tryReadImageFromPath`. Debounces clipboard polling
at `CLIPBOARD_CHECK_DEBOUNCE_MS = 50`.
**Cross-spec:** spec 37 (image attach flow).

#### `src/hooks/useGlobalKeybindings.tsx` (31KB, 248 LOC)
**Role:** Registers REPL-wide keybinding handlers (Ctrl+R for transcript
toggle, screen switching, virtual-scroll exit, search bar open). Renders
nothing — pure side-effect component that must mount inside `KeybindingSetup`.
Reads from `getFeatureValue_CACHED_MAY_BE_STALE` for flag-gated bindings.
**Cross-spec:** spec 37, spec 38a (keybinding system).

#### `src/hooks/useCommandKeybindings.tsx` (107 LOC)
**Role:** Reads `command:*` actions out of the keybinding config and
registers handlers that synthesize an immediate slash-command submit. Used
so a user-defined chord like `cmd+e` can fire `/edit` directly.
**Cross-spec:** spec 21 (command catalog), spec 38a.

#### `src/hooks/useExitOnCtrlCD.ts` (~80 LOC)
**Role:** Double-press detector for Ctrl+C / Ctrl+D exit. Returns
`{ pending, keyName }` so the footer can show "Press Ctrl+C again to exit".
Uses `useDoublePress` underneath. Decoupled from the keybinding module to
avoid an import cycle.

#### `src/hooks/useExitOnCtrlCDWithKeybindings.ts` (~30 LOC)
**Role:** Thin wrapper that wires `useExitOnCtrlCD` into the keybinding
context. Exists solely to break the cycle described above.

#### `src/hooks/useDoublePress.ts` (~50 LOC)
**Role:** Generic helper: returns a function that fires `onFirstPress` on
press 1 and `onDoublePress` if pressed again within
`DOUBLE_PRESS_TIMEOUT_MS = 800`. Used by exit detection + a few other
"are-you-sure" gestures.

#### `src/hooks/useBackgroundTaskNavigation.ts` (251 LOC)
**Role:** Up/down/Tab navigation through running teammate tasks when the
REPL is in teammate-view mode. Wires keys to
`enterTeammateView`/`exitTeammateView`; sorts running teammates via
`getRunningTeammatesSorted`.
**Cross-spec:** spec 30 (coordinator), spec 14 (agent tool).

#### `src/hooks/usePromptSuggestion.ts` (177 LOC)
**Role:** Drives the speculative ghost-text suggestion shown in the prompt.
Subscribes to AppState's speculation slot, calls `abortSpeculation` when
the user accepts/rejects, logs accept/reject analytics. Pauses while the
assistant is responding.
**Cross-spec:** spec 26 (services — speculative suggestions).

#### `src/hooks/renderPlaceholder.ts` (~40 LOC)
**Role:** Pure utility (not a hook despite the directory). Renders the
prompt placeholder with cursor inversion when focused. Imported by both the
text input and search input.

#### `src/hooks/useCopyOnSelect.ts` (~80 LOC)
**Role:** Auto-copies a finished mouse selection to the clipboard
(iTerm-style). Subscribes to `useSelection.subscribe`; only fires on a real
drag-finish or multi-click (guarded by `copiedRef`). No-op outside
alt-screen mode.

### §2.2 Suggestions / autocomplete

#### `src/hooks/fileSuggestions.ts` (27KB, 811 LOC) — module, not a hook
**Role:** The file-completion engine. Maintains an in-memory `FileIndex`
(native-ts module under `src/native-ts/file-index/`) backed by ripgrep + git
ls-files for cold-start, with `.gitignore` filtering. Exports
`generateFileSuggestions`, `applyFileSuggestion`, `findLongestCommonPrefix`,
`startBackgroundCacheRefresh`, `onIndexBuildComplete`. Also resolves
`@<filename>` and CLAUDE.md config files via `markdownConfigLoader`. Yields
to the event loop in `CHUNK_MS` slices to keep typing responsive while
the index is rebuilt.
**Cross-spec:** spec 17 (skill mentions), spec 26 (services).

#### `src/hooks/unifiedSuggestions.ts` (202 LOC)
**Role:** Merges file, agent, MCP-resource, command, and slash-skill
suggestions into a single ranked Fuse.js search across heterogeneous item
types. Used by the typeahead when the user types a freeform `@`-mention
that could resolve to multiple kinds.

### §2.3 Tool permission (architecturally spec 09)

#### `src/hooks/useCanUseTool.tsx` (40KB, 203 LOC)
**Role:** The `CanUseToolFn` factory — the function the QueryEngine calls
before executing any tool. Checks `hasPermissionsToUseTool` (rule-based
auto-allow), then dispatches to one of three handlers based on context:
- coordinator mode → `coordinatorHandler`
- swarm worker → `swarmWorkerHandler`
- otherwise → `interactiveHandler` (interactive REPL prompt)
Logs every decision via `logPermissionDecision`. Returns a
`PermissionDecision` that carries optional input updates and rule
suggestions back to the engine.
**Cross-spec:** spec 09 (permission model) — primary owner; spec 04 (turn
pipeline) consumer.
**Flag gates:** classifier-checking branch is `feature`-gated.

#### `src/hooks/toolPermission/PermissionContext.ts` (12.7KB, 388 LOC)
**Role:** Builds the shared `PermissionContext` carrier — the bag of state
threaded through every handler. Includes `awaitClassifierAutoApproval`
helper (BashTool's separate ML-classifier path), the
`createPermissionQueueOps` factory that mediates concurrent permission
requests against the `setToolUseConfirmQueue` setter, and the
`createResolveOnce` idempotency wrapper used by handlers.
**Cross-spec:** spec 09.

#### `src/hooks/toolPermission/handlers/interactiveHandler.ts` (20KB, 536 LOC)
**Role:** The full interactive permission flow. Pushes a `ToolUseConfirm`
into the queue, waits on a `Promise` resolved by the React permission
dialog (`PermissionRequest`), or by a CCR bridge response
(`channelPermissions`), or by an MCP `channel/permissionRequest`
notification. Handles bash-tool's classifier as a parallel race against
user input.
**Cross-spec:** spec 09, spec 33–35 (remote/bridge/mobile permission
relay).

#### `src/hooks/toolPermission/handlers/coordinatorHandler.ts` (~140 LOC)
**Role:** Coordinator-mode flow: hooks (PreToolUse) and classifier are
awaited sequentially, then falls through to the interactive dialog if
neither produced a decision. Distinguishes coordinator from regular
interactive because hooks run on a different code path here.
**Cross-spec:** spec 30 (coordinator), spec 26 (services-hooks).

#### `src/hooks/toolPermission/handlers/swarmWorkerHandler.ts` (~159 LOC)
**Role:** Worker-side of swarm permission relay. Constructs a
`createPermissionRequest`, sends it via the leader's mailbox
(`sendPermissionRequestViaMailbox`), and registers a callback in
`useSwarmPermissionPoller` to resolve when the leader responds.
**Cross-spec:** spec 30, spec 31 (mailbox).
**Flag gates:** `isAgentSwarmsEnabled()`.

#### `src/hooks/toolPermission/permissionLogging.ts` (238 LOC)
**Role:** Centralized fan-out for permission decisions: Statsig analytics
(via `logEvent`), OTel telemetry (`logOTelEvent`), and the per-language
code-edit decision counter (`getCodeEditToolDecisionCounter`). Sandbox
attribution is read from `SandboxManager`.
**Cross-spec:** spec 09, spec 24 (services-analytics).

### §2.4 Voice input (feature-gated)

#### `src/hooks/useVoice.ts` (45.8KB, 1,144 LOC)
**Role:** Hold-to-talk voice STT engine. Captures audio via the
native-ts macOS audio module (or SoX fallback), streams to Anthropic's
voice_stream/conversation_engine endpoint, decodes incremental transcripts
into `voiceState` ({ idle, recording, finalizing }) for the UI. Handles
keypress auto-repeat to detect release: when no event arrives within
`RELEASE_TIMEOUT_MS`, the recording auto-finalizes. Internal state
includes connection ref, audio capture handle, transcript buffers, and an
analytics-batched logging path. Pulls custom keyterms from
`getVoiceKeyterms` to bias the STT model.
**Cross-spec:** spec 38b (voice mode), spec 26 (services).
**Flag gates:** entire module is `feature('VOICE_MODE')`-gated upstream.

#### `src/hooks/useVoiceIntegration.tsx` (99.5KB, 676 LOC)
**Role:** REPL-side wrapper around `useVoice`. Owns the activation key
state machine: modifier+letter combos activate immediately, bare-char
bindings (default: space) require `HOLD_THRESHOLD` rapid presses to
distinguish "held" from "typed". The first `WARMUP_THRESHOLD` chars flow
through to the input so single-press still types normally; on activation
those flow-through chars are stripped via `stripTrailing`. Returns
`{ stripTrailing, resetAnchor, handleKeyEvent, interimRange }`. Heavy
because it exhaustively documents the activation logic in JSDoc.
**Cross-spec:** spec 38b, spec 38a (keybinding).
**Flag gates:** `feature('VOICE_MODE')`. Falls back to a no-op
`useVoice` shim when disabled.

#### `src/hooks/useVoiceEnabled.ts` (~30 LOC)
**Role:** Three-way AND of user intent (`settings.voiceEnabled`), auth
(`hasVoiceAuth`), and GrowthBook kill-switch (`isVoiceGrowthBookEnabled`).
Memoized on `authVersion` because cold OAuth reads spawn `security`
(~60ms); GB lookup is cheap and stays outside the memo.
**Cross-spec:** spec 38b.

### §2.5 Diff / IDE integration

#### `src/hooks/useDiffData.ts` (110 LOC)
**Role:** Fetches and structures git diff data for the current working tree.
Caps each file at `MAX_LINES_PER_FILE = 400` and produces a `DiffFile[]`
with stats. Backed by `utils/gitDiff`.

#### `src/hooks/useDiffInIDE.ts` (379 LOC)
**Role:** Streams pending file edits to the connected IDE (VS Code /
JetBrains) for inline diff preview. Resolves edits to patches via
`getPatchForEdits`, sends a notification through the IDE MCP server, and
listens for the user's accept/reject response back through a permission
option. Deduplicates by request UUID.
**Cross-spec:** spec 13 (file edit), spec 27 (services-ide).

#### `src/hooks/useTurnDiffs.ts` (213 LOC)
**Role:** Aggregates per-turn file edits into a `TurnDiff[]` for the
`/diff` review screen. Deduplicates edits to the same file within a turn,
folds added/removed lines, snapshots the user prompt preview.
**Cross-spec:** spec 13.

#### `src/hooks/useFileHistorySnapshotInit.ts` (~50 LOC)
**Role:** One-shot effect that re-hydrates `fileHistoryState` from the
session log on resume. Guarded by an `initialized` ref + `fileHistoryEnabled`.
**Cross-spec:** spec 41 (session resume), spec 13.

#### `src/hooks/useIDEIntegration.tsx` (10.5KB, ~140 LOC)
**Role:** REPL startup orchestration for IDE detection + extension auto-install.
Sets dynamic MCP config when an IDE is detected, surfaces the onboarding
dialog, tracks installation status state.
**Cross-spec:** spec 27.

#### `src/hooks/useIdeAtMentioned.ts` (~80 LOC)
**Role:** Listens on the IDE MCP server for `at_mentioned` notifications
(IDE-side selection → "send to Claude") and enqueues them as pending
notifications. Uses a `lazySchema` Zod definition for the params.
**Cross-spec:** spec 27.

#### `src/hooks/useIdeConnectionStatus.ts` (~30 LOC)
**Role:** Pure derived state: scans `mcpClients` for the IDE client and
returns `{ status: 'connected' | 'disconnected' | 'pending' | null,
ideName }`. Used by the IDE status indicator.

#### `src/hooks/useIdeLogging.ts` (~30 LOC)
**Role:** Subscribes to IDE-emitted `log_event` notifications and forwards
them to the analytics pipeline. Lets the IDE extension fire telemetry
through the host CLI rather than directly.

#### `src/hooks/useIdeSelection.ts` (150 LOC)
**Role:** Subscribes to IDE selection-change notifications, decodes the
`{ selection: { start, end }, text? }` payload, exposes the current
selection. Used by `useIDEStatusIndicator` and the prompt input footer.

### §2.6 Session / connection lifecycle

#### `src/hooks/useReplBridge.tsx` (115KB, 722 LOC)
**Role:** REPL-side adapter for the **bridge** transport (mobile/Slack/web
clients connecting to a local CLI via the cloud relay, "CCR"). Subscribes
to `replBridgeHandle`, applies inbound SDK messages to local state,
relays outbound user input + permission requests through the bridge.
Manages the failure dismiss timer (`BRIDGE_FAILURE_DISMISS_MS = 10s`),
permission-request handler map keyed by `request_id`, the auto-mode
gating logic, the pre-flight system-init message
(`buildSystemInitMessage`), and the bypass/transition machinery in
`utils/permissions/permissionSetup`.
**Cross-spec:** spec 33 (mobile / bridge), spec 32 (mailbox), spec 09
(permissions).
**Flag gates:** `feature('BRIDGE_MODE')`.

#### `src/hooks/useRemoteSession.ts` (23KB, 605 LOC)
**Role:** REPL-side wrapper for the **remote** transport (cloud-hosted
agent sessions from the mobile app or web). Owns a
`RemoteSessionManager`, converts `SDKMessage` → local `Message`,
dispatches synthetic assistant messages for permission stubs, handles
`session_end` cleanup. Uses `BoundedUUIDSet` to dedupe replayed events.
**Cross-spec:** spec 35 (remote-server mode), spec 41.

#### `src/hooks/useSSHSession.ts` (241 LOC)
**Role:** Variant of `useRemoteSession` that drives an `ssh` child
process + auth proxy created during startup. Same external shape
(`isRemoteMode`/`sendMessage`/`cancelRequest`/`disconnect`) but different
lifecycle (process is owned by main.tsx, not by the effect).
**Cross-spec:** spec 35.

#### `src/hooks/useDirectConnect.ts` (229 LOC)
**Role:** Sibling to `useRemoteSession` but for direct WebSocket
connections (`server/directConnectManager`). Used by the home-LAN /
device-tethered direct-connect flow.
**Cross-spec:** spec 35.

#### `src/hooks/useTeleportResume.tsx` (~120 LOC)
**Role:** Resumes a "teleported" code session — picks up a session that
was migrated from another machine via `teleportResumeCodeSession`. Owns
`{ isResuming, error, selectedSession }` state and the operation-error
classification.
**Cross-spec:** spec 41 (cross-machine resume), spec 24 (analytics).

#### `src/hooks/useSessionBackgrounding.ts` (158 LOC)
**Role:** Ctrl+B handler: backgrounds the current query (creating a
background task), or foregrounds an already-backgrounded one and merges
its messages back into the visible thread.
**Cross-spec:** spec 39 (tasks), spec 41.

#### `src/hooks/useApiKeyVerification.ts` (~80 LOC)
**Role:** Verifies the active API key against the Anthropic API on
startup or after `/login`. Returns
`{ status: 'loading' | 'valid' | 'invalid' | 'missing' | 'error',
reverify }`.
**Cross-spec:** spec 02 (auth).

#### `src/hooks/useDeferredHookMessages.ts` (~50 LOC)
**Role:** Async injector for SessionStart user-hook messages. The REPL
renders immediately while `pendingHookMessages` resolves (~500ms), then
splices the resulting `HookResultMessage[]` into the message list.
Returns a callback that `onSubmit` must await before the first API call
so the model always sees hook context.
**Cross-spec:** spec 26 (services-hooks) — only `src/hooks/` consumer of
the user-hook system.

#### `src/hooks/useAssistantHistory.ts` (250 LOC)
**Role:** Pagination cursor over the cloud-stored assistant session
history (`assistant/sessionHistory`). Fetches latest events on mount,
older pages on scroll. Used by the remote-session "show history" panel.
**Cross-spec:** spec 35, spec 41.

#### `src/hooks/useLogMessages.ts` (~120 LOC)
**Role:** Records every message to the JSONL session transcript
(`recordTranscript`). Cleans messages first
(`cleanMessagesForLogging`), and in swarm mode only records messages
where the current process is a chain participant (`isChainParticipant`).
**Cross-spec:** spec 41.

#### `src/hooks/useMainLoopModel.ts` (~50 LOC)
**Role:** Selector that returns the resolved model name (e.g.
`claude-opus-4-7`), reactively re-evaluating when GrowthBook config
refreshes (so an `tengu_ant_model_override` flag flip takes effect mid-
session). Uses `useReducer` to bump on `onGrowthBookRefresh`.
**Cross-spec:** spec 24.

### §2.7 Plugin / hint recommendation

#### `src/hooks/usePluginRecommendationBase.tsx` (11.4KB, ~150 LOC)
**Role:** Shared state machine for plugin-recommendation hooks (LSP +
claude-code-hint). Centralizes the gate chain (remote-mode skip,
already-showing, in-flight async guard) and the success/failure
notification JSX. Each consumer plugs in its own `tryResolve` callback.
**Cross-spec:** spec 28 (plugins).

#### `src/hooks/useLspPluginRecommendation.tsx` (21.6KB, 193 LOC)
**Role:** Detects file edits whose extension matches a known LSP plugin,
checks the LSP binary is installed locally, and offers a one-click
install. Once-per-session via `hasShownLspRecommendationThisSession`.
**Cross-spec:** spec 28.

#### `src/hooks/useClaudeCodeHintRecommendation.tsx` (15.4KB, 128 LOC)
**Role:** Surfaces plugin-install prompts driven by `<claude-code-hint />`
tags emitted to stderr by sub-CLIs (per `docs/claude-code-hints.md`).
Show-once-ever semantics persisted to config.
**Cross-spec:** spec 28.

#### `src/hooks/useManagePlugins.ts` (11.9KB, 304 LOC)
**Role:** Reload-plugins entrypoint: re-runs `loadAllPlugins`, re-imports
agents/commands/hooks/MCP/LSP from each plugin, detects delisted
plugins, and surfaces flagged (failed-load) plugins. Triggered by the
`/reload-plugins` command and by post-install events.
**Cross-spec:** spec 28, spec 21 (commands), spec 27 (LSP), spec 26 (MCP).

#### `src/hooks/useChromeExtensionNotification.tsx` (~30 LOC)
**Role:** Once-on-startup nudge (via `useStartupNotification`) to install
the Claude-in-Chrome extension when the user is a paying subscriber and
the extension isn't installed. Honors `--chrome` / `--no-chrome` CLI flags.

#### `src/hooks/usePromptsFromClaudeInChrome.tsx` (11.6KB, ~140 LOC)
**Role:** MCP notification listener for the Claude-in-Chrome extension
sending prompts (with optional images) into the CLI from the browser.
Validates with a Zod `lazySchema` and enqueues via
`enqueuePendingNotification`.
**Cross-spec:** spec 26 (MCP).

#### `src/hooks/useOfficialMarketplaceNotification.tsx` (~30 LOC)
**Role:** Auto-installs the official plugin marketplace on first run and
shows success/config-failure notifications.
**Cross-spec:** spec 28.

### §2.8 Swarm / coordinator hooks

#### `src/hooks/useSwarmInitialization.ts` (~80 LOC)
**Role:** On REPL mount, initializes teammate hooks + dynamic team
context (from `team.json` if resuming). Conditionally loaded so dead-code
elimination strips it when swarms are disabled. Uses
`isAgentSwarmsEnabled()` as inner gate.
**Cross-spec:** spec 30, spec 31 (mailbox).

#### `src/hooks/useSwarmPermissionPoller.ts` (330 LOC)
**Role:** Worker-side: polls a leader-published permission-response file,
validates with `permissionUpdateSchema`, and invokes registered
callbacks. Counterpart of `swarmWorkerHandler`.
**Cross-spec:** spec 30, spec 09.

#### `src/hooks/useTeammateViewAutoExit.ts` (~40 LOC)
**Role:** Auto-exits teammate viewing mode when the viewed teammate is
killed or errors out. Uses a narrow AppState selector (just the viewed
task) to avoid re-rendering on every other teammate's stream tick.
**Cross-spec:** spec 30.

#### `src/hooks/useMailboxBridge.ts` (~30 LOC)
**Role:** Drains the `mailbox` (cross-process IPC) into the prompt
submitter. `useSyncExternalStore` over `mailbox.revision`; on each new
revision, polls one message and submits it as user input.
**Cross-spec:** spec 31 (mailbox), spec 30.

#### `src/hooks/useInboxPoller.ts` (34.4KB, 969 LOC)
**Role:** The big one for swarms: polls the leader's inbox for messages
addressed to teammates, dispatches them as injected user messages,
handles plan-approval responses, surfaces `tool_use_confirm` requests
from worker tasks, fires terminal-bell + system-notifier on permission
asks. Heavy: maintains pending-permission map, terminal-focus checks,
tmux backend detection. 60-line `useInterval` cycle (default 100ms).
**Cross-spec:** spec 30, spec 31, spec 09.

### §2.9 Task & queue plumbing

#### `src/hooks/useTasksV2.ts` (250 LOC)
**Role:** Reactive view of the on-disk Todo-V2 task list. `fs.watch` on
the tasks dir + `FALLBACK_POLL_MS = 5000` for missed events. `HIDE_DELAY_MS
= 5000` keeps a finished list visible briefly after completion.
**Cross-spec:** spec 39 (tasks), spec 19 (TodoTool).

#### `src/hooks/useTaskListWatcher.ts` (221 LOC)
**Role:** Lower-level watcher for a specific `taskListId` (tasks-mode).
Streams `Task[]` updates via fs.watch with `DEBOUNCE_MS = 1000` to
collapse rapid writes. Used by tasks mode + the coordinator.
**Cross-spec:** spec 39, spec 30.

#### `src/hooks/useScheduledTasks.ts` (139 LOC)
**Role:** Cron scheduler driver. Builds a `cronScheduler` from the
configured schedules, enqueues a `scheduledTaskFire` notification when a
scheduled task fires, optionally injects user messages directly into a
target teammate.
**Cross-spec:** spec 39 (KAIROS cron tools), spec 30.
**Flag gates:** `isKairosCronEnabled()`.

#### `src/hooks/useCommandQueue.ts` (~15 LOC)
**Role:** Trivial subscription to the unified command queue
(`messageQueueManager`). `useSyncExternalStore` returns the frozen
`QueuedCommand[]`.

#### `src/hooks/useQueueProcessor.ts` (~50 LOC)
**Role:** When idle (`hasActiveLocalJsxUI` = false and `queryGuard`
permits), pops and executes queued commands in priority order
(`now > next > later`). Subscribes to the same store as
`useCommandQueue`.

#### `src/hooks/useCancelRequest.ts` (276 LOC)
**Role:** Esc handler that aborts the active query, dismisses pending
permission dialogs, clears the queue depending on state. Routes
ToolUseConfirm cleanup, vim-mode awareness, spinner state. The most
complex of the "interrupt" hooks because Esc can mean five different
things by context.

#### `src/hooks/usePrStatus.ts` (106 LOC)
**Role:** Polls `gh pr status` every 60s for the working branch's PR
review state. Stops polling after 60min idle (`getLastInteractionTime`).
Detects slow `gh` (>4s) and degrades silently.
**Cross-spec:** spec 27 (gh integration).

#### `src/hooks/useIssueFlagBanner.ts` (133 LOC)
**Role:** Decides whether to show the "issues with external commands?"
banner. Scans recent bash invocations against `EXTERNAL_COMMAND_PATTERNS`
(curl/wget/ssh/kubectl/srun/docker/bq/gsutil/gcloud/aws/git push/pull/
fetch/gh pr|issue) and triggers the feedback CTA.

### §2.10 Settings / config / merging

#### `src/hooks/useSettings.ts` (~15 LOC)
**Role:** `useAppState(s => s.settings)`. AppState reactively reflects
disk changes via `settingsChangeDetector`.

#### `src/hooks/useSettingsChange.ts` (~30 LOC)
**Role:** Subscribes to `settingsChangeDetector` fan-out and invokes
`onChange(source, settings)` on disk changes. Used by surfaces that need
to re-derive on settings change but can't read AppState directly.

#### `src/hooks/useSkillsChange.ts` (~50 LOC)
**Role:** Keeps the commands list fresh under two triggers: skill file
changes (full disk re-scan) and GrowthBook init/refresh (memo-only
clear, since only `isEnabled()` predicates may have changed).
**Cross-spec:** spec 17 (skills), spec 21 (commands).

#### `src/hooks/useDynamicConfig.ts` (~25 LOC)
**Role:** `getDynamicConfig_BLOCKS_ON_INIT` shim — returns default value
synchronously then updates when GrowthBook resolves. Skipped in
NODE_ENV=test to avoid blocking.
**Cross-spec:** spec 24.

#### `src/hooks/useMergedClients.ts` (~25 LOC)
**Role:** `uniqBy('name')` merge of initial + dynamic MCP clients. Pure
useMemo.

#### `src/hooks/useMergedCommands.ts` (~15 LOC)
**Role:** `uniqBy('name')` merge of built-in + MCP-prompt commands.
Pure useMemo.

#### `src/hooks/useMergedTools.ts` (~30 LOC)
**Role:** Calls the shared `assembleToolPool` (the same pure function
used by `runAgent`) and overlays `initialTools`. Applies deny rules and
deduplicates.
**Cross-spec:** spec 03 (tool registry).

### §2.11 Display / animation primitives

#### `src/hooks/useTerminalSize.ts` (~15 LOC)
**Role:** Just `useContext(TerminalSizeContext)` with a guard error.
Wrapper exists so consumers don't import the context directly.

#### `src/hooks/useBlink.ts` (~50 LOC)
**Role:** Synchronized 600ms blink animation. Returns `[ref, isVisible]`;
all instances share a clock that pauses when no subscriber is visible
or the terminal is blurred.

#### `src/hooks/useElapsedTime.ts` (~50 LOC)
**Role:** `useSyncExternalStore`-backed live duration formatter. Pauses
on `isRunning=false`; freezes at `endTime` when set (so a 2-min task
viewed 30 min later doesn't show 32m). Subtracts `pausedMs`.

#### `src/hooks/useTimeout.ts` (~15 LOC)
**Role:** `setTimeout`-based "has N ms elapsed?" boolean. Resets on
`resetTrigger` change.

#### `src/hooks/useNotifyAfterTimeout.ts` (~50 LOC)
**Role:** Fires terminal bell + system notification when no user
interaction has occurred within the threshold (default 6s) — used to
nudge "the assistant is waiting on you".

#### `src/hooks/useMinDisplayTime.ts` (~30 LOC)
**Role:** Throttle that guarantees each distinct value gets `minMs` of
screen time before being replaced. Prevents fast-cycling progress text
from flickering past unread.

#### `src/hooks/useAfterFirstRender.ts` (~20 LOC)
**Role:** Anthropic-internal-only. When
`USER_TYPE === 'ant'` and `CLAUDE_CODE_EXIT_AFTER_FIRST_RENDER` is set,
prints startup time and exits. Used by perf benchmarks.
**Flag gates:** `USER_TYPE === 'ant'`.

#### `src/hooks/useUpdateNotification.ts` (~35 LOC)
**Role:** Compares the just-installed semver against the
last-shown-version (persisted) and returns the new version string if it's
worth notifying about. Only major.minor.patch — pre-release and build are
stripped.

#### `src/hooks/useVirtualScroll.ts` (35.1KB, 721 LOC)
**Role:** The scroll virtualizer used by the message list. Estimates
heights (default 3 rows), overscans 80 rows, mounts COLD_START_COUNT=30
items before viewport-height is known, quantizes scrollTop in the
external-store snapshot to avoid one re-render per wheel tick. Uses
`useDeferredValue` + `useSyncExternalStore` + `useLayoutEffect` for
measurement. The asymmetric estimate (low) intentionally errs toward
mounting too many rather than blank space.
**Cross-spec:** spec 37 (Ink primitives — `ScrollBox`).

#### `src/hooks/useMemoryUsage.ts` (~30 LOC)
**Role:** Polls `process.memoryUsage` every 10s, returns
`{ heapUsed, status: 'normal'|'high'|'critical' }`. Returns null while
normal so the indicator doesn't render.
**Cross-spec:** spec 40 (memory).

#### `src/hooks/useClipboardImageHint.ts` (~50 LOC)
**Role:** Watches terminal-focus regain events; if `hasImageInClipboard`,
shows a "press Ctrl+V to paste" hint with 30s cooldown.

#### `src/hooks/useAwaySummary.ts` (125 LOC)
**Role:** When the terminal is blurred for `BLUR_DELAY_MS = 5min` AND
no away-summary already exists since the last user turn, calls
`generateAwaySummary` and injects it into the message list. Helps
returning users catch up.
**Cross-spec:** spec 26 (services).
**Flag gates:** `feature(...)` via `bun:bundle`.

#### `src/hooks/useSkillImprovementSurvey.ts` (~80 LOC)
**Role:** Captures user feedback on skill suggestions. Builds a
`FeedbackSurveyResponse`, applies the resulting `SkillUpdate[]` via
`applySkillImprovement`, surfaces a system message confirming.
**Cross-spec:** spec 17.

### §2.12 Notifications surface (`notifs/`)

The `notifs/` subdirectory contains 17 hooks, each driving one
notification card. Pattern: most call `useStartupNotification(compute)`
(once-per-session, remote-mode-gated, async-safe) and return either
`null` or a `Notification` object.

#### `src/hooks/notifs/useStartupNotification.ts` (~50 LOC)
**Role:** The shared base. Fires `compute()` exactly once on mount, gated
by `getIsRemoteMode()` and a session ref. Accepts sync or async result;
async errors route to `logError`.

#### `src/hooks/notifs/useAutoModeUnavailableNotification.ts` (~40 LOC)
**Role:** Shows when the shift-tab carousel wraps past where auto mode
would have been (settings/circuit-breaker/org-allowlist).

#### `src/hooks/notifs/useCanSwitchToExistingSubscription.tsx` (~80 LOC)
**Role:** Detects "user has Console subscription but isn't logged into
it"; suggests `/login`. Caps at `MAX_SHOW_COUNT = 3`.

#### `src/hooks/notifs/useDeprecationWarningNotification.tsx` (~40 LOC)
**Role:** Reads `getModelDeprecationWarning(model)` and surfaces it; only
re-fires when the warning string changes.

#### `src/hooks/notifs/useFastModeNotification.tsx` (~160 LOC)
**Role:** Subscribes to four fastMode events (cooldown started/expired,
org-changed, overage-rejected) and fires per-event notifications.

#### `src/hooks/notifs/useIDEStatusIndicator.tsx` (20.8KB, 185 LOC)
**Role:** The dynamic IDE-status hint shown in the footer. Combines
`useIdeConnectionStatus`, IDE-detection (`detectIDEs`), JetBrains-vs-VSCode
branching, and a max-show-count cap of 5.
**Cross-spec:** spec 27.

#### `src/hooks/notifs/useInstallMessages.tsx` (~30 LOC)
**Role:** Surfaces `nativeInstaller/checkInstall` results (PATH issues,
shell aliases, errors) at startup.

#### `src/hooks/notifs/useLspInitializationNotification.tsx` (16.6KB, 142 LOC)
**Role:** 5s-interval poll of LSP manager init + per-server status; on
error adds to `appState.plugins.errors` for `/doctor`.
**Cross-spec:** spec 27.

#### `src/hooks/notifs/useMcpConnectivityStatus.tsx` (14.7KB, ~110 LOC)
**Role:** Surfaces MCP server connection failures, especially the
claude.ai cloud MCP first-connect.

#### `src/hooks/notifs/useModelMigrationNotifications.tsx` (~50 LOC)
**Role:** Table-driven post-migration nudge ("Model updated to Sonnet
4.6"). Each migration entry checks its own timestamp config field for
"writes within last 3s of launch".

#### `src/hooks/notifs/useNpmDeprecationNotification.tsx` (~30 LOC)
**Role:** "npm install of Claude Code is deprecated, run `claude
install`". Skipped in bundled mode + dev installs.

#### `src/hooks/notifs/usePluginAutoupdateNotification.tsx` (~40 LOC)
**Role:** Listens on `onPluginsAutoUpdated` event; tells user to
`/reload-plugins`.

#### `src/hooks/notifs/usePluginInstallationStatus.tsx` (12KB, 127 LOC)
**Role:** Reactive notification driven by `appState.plugins.installationStatus`
(in-progress/success/failed). Pluralizes "1 plugin" / "N plugins".

#### `src/hooks/notifs/useRateLimitWarningNotification.tsx` (12.3KB, 113 LOC)
**Role:** Pulls live rate-limit data via `useClaudeAiLimits`, calls
`getRateLimitWarning(limits, model)`, and surfaces overage-text
(`getUsingOverageText`) for paying subscribers.

#### `src/hooks/notifs/useSettingsErrors.tsx` (~50 LOC)
**Role:** Surfaces validation errors from settings JSON. Re-checks on
every settings file change via `useSettingsChange`.

#### `src/hooks/notifs/useTeammateShutdownNotification.ts` (~50 LOC)
**Role:** Aggregating notification: when N teammates shut down within
the same notification window, the `foldSpawn` function rolls the count
into a single "N teammates exited" line.
**Cross-spec:** spec 30.

## §3 Patterns

### §3.1 Naming
- `useFooBar` for hooks; lowercase-first for plain modules
  (`fileSuggestions.ts`, `unifiedSuggestions.ts`, `renderPlaceholder.ts`).
- Notification hooks live under `notifs/` and are named for what they
  notify about, not the trigger (e.g. `useFastModeNotification`, not
  `useOnFastModeChange`).
- Permission handlers live under `toolPermission/handlers/` and end in
  `Handler` (`coordinatorHandler`, `interactiveHandler`,
  `swarmWorkerHandler`).

### §3.2 React-compiler artifacts
~30% of hooks have been compiled by the React Compiler — they import
`{ c as _c } from "react/compiler-runtime"` and start every function
with `const $ = _c(N)` followed by hand-rolled cache slots
(`if ($[0] !== x || ...)`). These are the leaked compiled outputs, not
the original sources. The patterns are otherwise identical, just less
readable.

### §3.3 Feature flag gating
Voice (`feature('VOICE_MODE')`), bridge (`feature('BRIDGE_MODE')`),
swarms (`isAgentSwarmsEnabled()`), KAIROS cron (`isKairosCronEnabled()`),
and `USER_TYPE === 'ant'` are the main gates. Voice in particular uses
the conditional `require()` pattern documented in `CLAUDE.md` —
`useVoiceIntegration` captures the module namespace (not the function)
to survive `spyOn()` mutations during tests.

### §3.4 Effect-vs-store split
Three reactivity strategies coexist:
- **`useAppState(selector)`** for slices of the central Zustand-like
  store. Most state-coupled hooks use this.
- **`useSyncExternalStore(subscribe, getSnapshot)`** for module-level
  stores (`useCommandQueue`, `useQueueProcessor`,
  `useMailboxBridge`, `useVirtualScroll` scrollTop). Used when the source
  is a hand-rolled subscribable rather than an AppState slice.
- **`useEffect` + ref guard** for one-shot startup effects
  (`useAfterFirstRender`, `useFileHistorySnapshotInit`,
  `useStartupNotification`).

### §3.5 Idempotency / once-per-session refs
The pattern `const fired = useRef(false)` followed by a guard inside the
effect body recurs ~20× across notification and migration hooks. The
ref is intentionally not state — re-rendering should not re-fire the
side effect. `useStartupNotification` factored this out as a helper, but
many older hooks still hand-roll it.

### §3.6 Debounce / throttle / chunk
- `useDebounceCallback` (usehooks-ts) for clipboard polling, suggestion
  fetches, paste handling.
- `HISTORY_CHUNK_SIZE = 10` in arrow-history + history-search to amortize
  disk reads.
- `DEBOUNCE_MS = 1000` for fs.watch task-list debouncing.
- `BLUR_DELAY_MS = 5min` for away-summary trigger.
- `IDLE_STOP_MS = 60min` for PR-status polling.
- `useMinDisplayTime` is a custom throttle: guarantees min screen time
  rather than min interval.

### §3.7 Cycle breaks
Three explicit cycle breaks:
- `useExitOnCtrlCD` ↔ `useExitOnCtrlCDWithKeybindings` — the first
  doesn't import the keybindings module.
- `voiceNs = require('./useVoice.js')` capture in
  `useVoiceIntegration` — feature-flag-gated conditional require.
- `useSwarmPermissionPoller` ↔ `swarmWorkerHandler` —
  `registerPermissionCallback` is exported and imported across the
  permission handler boundary.

### §3.8 The "hook" naming overload
This directory name is unfortunate: `src/hooks/` contains React hooks,
while `src/utils/hooks/` and `src/services/hooks/` (referenced from
`useDeferredHookMessages`, the toolPermission handlers, and elsewhere)
contain Claude Code's user-hook config system (PreToolUse, PostToolUse,
SessionStart, etc.). This spec is exclusively about React hooks.

## §4 Cross-spec edges

| Spec | Files in this catalog that belong primarily there |
|---|---|
| 09 — permission model | `useCanUseTool`, `toolPermission/PermissionContext`, `toolPermission/handlers/*`, `toolPermission/permissionLogging` (6 files) |
| 13 — file-edit tool | `useDiffData`, `useDiffInIDE`, `useTurnDiffs`, `useFileHistorySnapshotInit` |
| 14 — agent/team tool | `useBackgroundTaskNavigation` (DM mention thread in `useTypeahead`) |
| 17 — skills | `useSkillsChange`, `useSkillImprovementSurvey`, `unifiedSuggestions` (skill mentions) |
| 19 — TodoTool | `useTasksV2` |
| 21 — command catalog | `useMergedCommands`, `useCommandKeybindings`, `useTypeahead` (slash command suggestions) |
| 24 — analytics | `useDynamicConfig`, `useMainLoopModel`, `permissionLogging` |
| 26 — services-hooks | `useDeferredHookMessages` (only React-hooks consumer of the user-hook system) |
| 27 — services-ide / LSP | `useIDEIntegration`, `useIdeAtMentioned`, `useIdeConnectionStatus`, `useIdeLogging`, `useIdeSelection`, `useDiffInIDE`, `notifs/useLspInitializationNotification`, `notifs/useIDEStatusIndicator`, `useLspPluginRecommendation`, `usePrStatus` |
| 28 — services-plugins | `useManagePlugins`, `useLspPluginRecommendation`, `useClaudeCodeHintRecommendation`, `usePluginRecommendationBase`, `useChromeExtensionNotification`, `usePromptsFromClaudeInChrome`, `useOfficialMarketplaceNotification`, `notifs/usePluginAutoupdateNotification`, `notifs/usePluginInstallationStatus` |
| 30 — coordinator | `useSwarmInitialization`, `useSwarmPermissionPoller`, `useTeammateViewAutoExit`, `useBackgroundTaskNavigation`, `useInboxPoller`, `useTaskListWatcher`, `notifs/useTeammateShutdownNotification`, `toolPermission/handlers/coordinatorHandler`, `toolPermission/handlers/swarmWorkerHandler` |
| 31 — mailbox | `useMailboxBridge`, `useInboxPoller` |
| 33 — bridge mode | `useReplBridge` |
| 35 — remote/server | `useRemoteSession`, `useSSHSession`, `useDirectConnect`, `useTeleportResume`, `useAssistantHistory` |
| 38a — keybindings/vim | `useGlobalKeybindings`, `useCommandKeybindings`, `useExitOnCtrlCD*`, `useDoublePress`, `useVimInput` |
| 38b — voice mode | `useVoice`, `useVoiceIntegration`, `useVoiceEnabled` |
| 39 — tasks | `useTasksV2`, `useTaskListWatcher`, `useScheduledTasks`, `useCommandQueue`, `useQueueProcessor`, `useSessionBackgrounding` |
| 40 — persistent memory | `useMemoryUsage` |
| 41 — session/state/history | `useArrowKeyHistory`, `useHistorySearch`, `useAssistantHistory`, `useLogMessages`, `useSessionBackgrounding`, `useFileHistorySnapshotInit`, `useTeleportResume` |

The remaining ~50 files (input plumbing, animation primitives, notifs/*
non-IDE/non-plugin) are owned squarely by spec **37 — Ink UI shell**
and exist nowhere else.
