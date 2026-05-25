# 42a — Utils Long Tail

## §0 Scope

Catalog companion to spec **42-misc**. Spec 42 covers cross-cutting utilities
at architectural level (process lifecycle, common formatting, etc.); this
spec **enumerates every individual utility module** under `src/utils/` not
already cited elsewhere by strict path.

Authoritative residual list: `PHASE9-COVERAGE.md` → `42-misc` bucket
(`src/utils/...` rows only). 327 files; total ~3.93 MB on disk; ~80K LOC of
TypeScript out of the 180 K-LOC `src/utils/` directory.

The bulk of the directory is single-purpose helpers, but a meaningful tail
(23 files >20KB) implements substantial subsystems. Those get prose entries
in §2; mid-tier (5–20KB, 126 files) get one-paragraph table entries in §3;
trivial (<5KB, 178 files) get one-line table entries in §4 grouped by
category.

Cross-cutting note: many "utils" files are actually subsystem entry points
that sit under `src/utils/` for historical reasons. They are still cited
here, but **§5 reassignment map** flags those that ought to belong to a
sibling spec (modes, services, tools).

---

## §1 Category Index

327 residual utils bucketed by purpose. Counts include `.ts` and `.tsx`.

| Category | Files | Notable members |
|---|---:|---|
| Bash / shell parsing & permissions (`utils/bash/`, `utils/shell/`, `utils/powershell/`) | 28 | `bash/ast.ts` (112KB), `shell/readOnlyCommandValidation.ts` (68KB), `bash/heredoc.ts`, `bash/ShellSnapshot.ts`, `bash/bashPipeCommand.ts`, `powershell/staticPrefix.ts` |
| Swarm / multi-agent coordination (`utils/swarm/`) | 21 | `swarm/inProcessRunner.ts` (53KB), `swarm/It2SetupPrompt.tsx`, `swarm/permissionSync.ts`, `swarm/backends/*` (8 files) |
| Computer-use (`utils/computerUse/`) | 16 | `computerUse/wrapper.tsx` (49KB), `executor.ts` (23KB), `toolRendering.tsx`, `appNames.ts`, `computerUseLock.ts` |
| Model selection / routing (`utils/model/`) | 17 | `model/model.ts` (21KB), `modelOptions.ts`, `modelAllowlist.ts`, `bedrock.ts`, `aliases.ts`, `validateModel.ts` |
| Hooks runtime (`utils/hooks/`) | 13 | `hooks/hooksConfigManager.ts`, `execAgentHook.ts`, `execHttpHook.ts`, `execPromptHook.ts`, `ssrfGuard.ts`, `AsyncHookRegistry.ts` |
| Plugins (`utils/plugins/`) | 6 | `plugins/validatePlugin.ts` (28KB), `lspRecommendation.ts`, `hintRecommendation.ts`, `performStartupChecks.tsx`, `pluginFlagging.ts`, `gitAvailability.ts` |
| Claude-in-Chrome (`utils/claudeInChrome/`) | 5 | `claudeInChrome/toolRendering.tsx` (34KB), `chromeNativeHost.ts`, `mcpServer.ts`, `common.ts`, `setupPortable.ts` |
| Native installer / autoupdate (`utils/nativeInstaller/`, top-level) | 5 | `nativeInstaller/installer.ts` (55KB), `pidLock.ts`, `packageManagers.ts`, `autoUpdater.ts`, `localInstaller.ts` |
| Deep link / protocol handler (`utils/deepLink/`) | 6 | `deepLink/terminalLauncher.ts`, `registerProtocol.ts`, `parseDeepLink.ts`, `protocolHandler.ts`, `terminalPreference.ts`, `banner.ts` |
| Status & UI rendering (`utils/statusNotice*`, `*Renderer.tsx`) | 8 | `statusNoticeDefinitions.tsx`, `statusNoticeHelpers.ts`, `staticRender.tsx`, `exportRenderer.tsx`, `autoRunIssue.tsx`, `preflightChecks.tsx`, `markdown.ts`, `cliHighlight.ts` |
| ANSI / terminal rendering | 6 | `ansiToPng.ts` (215KB!), `ansiToSvg.ts`, `asciicast.ts`, `sliceAnsi.ts`, `hyperlink.ts`, `horizontalScroll.ts` |
| Git / GitHub | 7 | `git/gitFilesystem.ts`, `git/gitConfigParser.ts`, `git/gitignore.ts`, `gitDiff.ts`, `github/ghAuthStatus.ts`, `ghPrStatus.ts`, `githubRepoPathMapping.ts`, `detectRepository.ts` |
| MCP utility | 6 | `mcp/elicitationValidation.ts`, `mcp/dateTimeParser.ts`, `mcpValidation.ts`, `mcpInstructionsDelta.ts`, `mcpOutputStorage.ts`, `mcpWebSocketTransport.ts` |
| Settings / env / sandbox | 8 | `managedEnv.ts`, `managedEnvConstants.ts`, `env.ts`, `envValidation.ts`, `subprocessEnv.ts`, `sandbox/sandbox-ui-utils.ts`, `caCertsConfig.ts`, `xdg.ts` |
| Process / spawn / cleanup | 12 | `cleanup.ts`, `gracefulShutdown.ts`, `genericProcessUtils.ts`, `execFileNoThrowPortable.ts`, `execSyncWrapper.ts`, `process.ts`, `lockfile.ts`, `cronTasksLock.ts`, `combinedAbortSignal.ts`, `abortController.ts`, `which.ts`, `findExecutable.ts` |
| Profiling / debug / telemetry | 9 | `queryProfiler.ts`, `startupProfiler.ts`, `headlessProfiler.ts`, `profilerBase.ts`, `fpsTracker.ts`, `heapDumpService.ts`, `telemetryAttributes.ts`, `unaryLogging.ts`, `errorLogSink.ts`, `debugFilter.ts` |
| Suggestions / completion | 6 | `suggestions/directoryCompletion.ts`, `suggestions/slackChannelSuggestions.ts`, `suggestions/shellHistoryCompletion.ts`, `bash/shellCompletion.ts`, `completionCache.ts`, `exampleCommands.ts` |
| Session / agentic search | 6 | `agenticSessionSearch.ts`, `listSessionsImpl.ts`, `transcriptSearch.ts`, `sessionEnvironment.ts`, `sessionEnvVars.ts`, `sessionTitle.ts`, `sessionUrl.ts`, `sessionIngressAuth.ts`, `crossProjectResume.ts` |
| Task / output / I/O | 9 | `task/TaskOutput.ts`, `task/diskOutput.ts`, `task/outputFormatting.ts`, `task/sdkProgress.ts`, `bufferedWriter.ts`, `stream.ts`, `streamJsonStdoutGuard.ts`, `streamlinedTransform.ts`, `toolResultStorage.ts` |
| Teleport / remote bundle | 4 | `teleport/gitBundle.ts`, `teleport/environments.ts`, `teleport/environmentSelection.ts`, `background/remote/preconditions.ts` |
| Ultraplan / CCR / sideQuery | 4 | `ultraplan/ccrSession.ts`, `ultraplan/keyword.ts`, `sideQuery.ts`, `sideQuestion.ts` |
| Image / clipboard / paste | 6 | `imageStore.ts`, `imageValidation.ts`, `screenshotClipboard.ts`, `pasteStore.ts`, `appleTerminalBackup.ts`, `iTermBackup.ts` |
| Cron / scheduling | 2 | `cron.ts`, `cronTasksLock.ts` |
| IDE integration | 4 | `ide.ts` (47KB), `idePathConversion.ts`, `jetbrains.ts`, `editor.ts` |
| Crypto / hash / id | 5 | `hash.ts`, `uuid.ts`, `taggedId.ts`, `fingerprint.ts`, `peerAddress.ts` |
| Time / date / interval | 4 | `cron.ts`, `formatBriefTimestamp.ts`, `idleTimeout.ts`, `sleep.ts` |
| Data structures | 6 | `CircularBuffer.ts`, `Cursor.ts` (47KB), `set.ts`, `array.ts`, `contentArray.ts`, `objectGroupBy.ts` |
| Format / string / markdown | 8 | `format.ts`, `stringUtils.ts`, `truncate.ts`, `markdown.ts`, `markdownConfigLoader.ts`, `treeify.ts`, `displayTags.ts`, `textHighlighting.ts`, `highlightMatch.tsx`, `cliHighlight.ts` |
| Generators / async | 4 | `generators.ts`, `withResolvers.ts`, `sequential.ts`, `signal.ts` |
| Misc / one-offs | ~70 | many small helpers — see §4 |

(Some files appear in multiple buckets; counts above are the dominant
classification. Total reconciles to 327.)

---

## §2 Substantial utilities (>20KB) — 23 entries

Each entry: role · public surface · cross-spec dependents.

### `src/utils/ansiToPng.ts` — 215 KB

**Role.** Render ANSI-escaped terminal output directly to a PNG image,
**bypassing SVG entirely**. Replaces a previous `ansiToSvg → @resvg/resvg-wasm`
pipeline (2.36 MB embedded WASM, 2.1 MB runtime font, ~224 ms per render,
broken on systems without the hardcoded font path). The new path blits a
bundled 24×48 Fira Code bitmap font (8-bit AA alpha) into an RGBA Uint8Array,
then encodes via `node:zlib`'s `deflateSync`. Identical output across
mac/linux/windows, ~5–15 ms per render, zero external deps.

**Why so large.** The rasterized bitmap glyph atlas covering printable ASCII
plus the unicode characters used by `/stats` output is embedded as a
TypedArray literal in the source. Regenerable via
`bun scripts/generate-bitmap-font.ts`.
**Public API.** `ansiToPng(text, options): Buffer`. Imports `parseAnsi`,
`DEFAULT_BG`, `ParsedLine`, `AnsiColor` from `./ansiToSvg.ts` (still kept for
its parser).
**Cross-spec.** spec 19 (`/export` command produces screenshots),
spec 38 (output styles), spec 37 (terminal rendering). The screenshot
clipboard path (`screenshotClipboard.ts`) consumes its output.

### `src/utils/bash/ast.ts` — 112 KB

**Role.** AST-based bash command analysis using **tree-sitter-bash**, the
permission engine's safety-critical core. Replaces an earlier
`shell-quote` + char-walker that detected parser differentials one by one.
Walks the parse tree against an **explicit allowlist of node types**;
anything off-list classifies the command as `'too-complex'` and falls back
to manual permission prompting. Documented as **fail-closed**: never
interpret structure not understood. Not a sandbox — answers exactly "can
we trustworthily extract argv[] for each simple command in this string?"
**Public API.** `parseForSecurity(cmd) → { kind: 'simple', commands } |
{ kind: 'too-complex' } | { kind: 'parse-unavailable' }`; types
`SimpleCommand`, `Redirect`, `ParseForSecurityResult`. Uses parser indirected
through `./parser.js` so the native NAPI binding can be swapped.
**Cross-spec.** spec 10 (BashTool permission check), spec 9 (permission
system base), spec 12 (search tools that share the parser). This file is
**THE** trust boundary between user input and shell exec.

### `src/utils/shell/readOnlyCommandValidation.ts` — 68 KB

**Role.** Shared command validation maps used by **both** BashTool and
PowerShellTool. Encodes for every well-known external CLI (git, gh, npm,
docker, kubectl, etc.) the set of read-only subcommands plus their safe
flags. The bulk of file size is encyclopedic flag tables.
**Public API.** `GIT_READ_ONLY_COMMANDS`, `GH_READ_ONLY_COMMANDS`
(ant-only), `EXTERNAL_READONLY_COMMANDS` (cross-shell), `containsVulnerable
UncPath()`, types `FlagArgType`, `ExternalCommandConfig`. Each entry can
declare `additionalCommandIsDangerousCallback` for cases that need code
beyond flag matching.
**Cross-spec.** spec 10 (BashTool), spec 19/Bash family (PowerShellTool),
spec 9 (default-allowlist auto-approval).

### `src/utils/nativeInstaller/installer.ts` — 55 KB

**Role.** File-based **native installer system** with directory-tree
managed by symlinks. Owns version installation, activation, multi-process
locking via `lockfile.ts`, mtime-based simple fallback, and supports both
JS and native (Bun-bundled) builds. Co-resident with `pidLock.ts`,
`packageManagers.ts`, and `localInstaller.ts`.
**Public API.** `installVersion`, `activateVersion`, `cleanupOldVersions`,
`getInstalledVersions`, `setActiveVersion`. Reads/writes `~/.claude/
versions/` and `~/.claude/cache/`.
**Cross-spec.** spec 1 (entrypoint bootstrap), spec 42 (autoupdate). Tightly
coupled to `autoUpdater.ts`.

### `src/utils/swarm/inProcessRunner.ts` — 54 KB

**Role.** **In-process teammate runner** — wraps `runAgent()` for teammates
that share the leader's Node process (vs the tmux/iTerm2 separate-process
backends). Provides AsyncLocalStorage context isolation
(`runWithTeammateContext`), progress tracking with AppState updates, idle
notification on completion, plan-mode approval flow, abort/cleanup,
auto-compact handling. Largest single piece of swarm orchestration logic.
**Public API.** `runInProcessTeammate(state, ctx)`, plus internal
`appendCappedMessage`, idle-notification routing, permission callback
register/unregister.
**Cross-spec.** spec 14 (Agent/Team tool), spec 30 (coordinator), spec 7
(compact), spec 9 (permissions).

### `src/utils/computerUse/wrapper.tsx` — 49 KB

**Role.** The `.call()` override adapter between Tool's `ToolUseContext`
and `@ant/computer-use-mcp`'s `bindSessionContext`. Spread into the MCP
tool object in `client.ts` (same pattern as Claude-in-Chrome rendering
overrides). Per-process binding cache + per-call ref for context that
varies (`abortController`, `setToolJSX`, `sendOSNotification`). Gated on
`feature('CHICAGO_MCP')` and runtime GrowthBook flag `tengu_malort_pedway`.
**Public API.** `getComputerUseCallOverride(tool)` returning
`Pick<Tool, 'call'>`.
**Cross-spec.** spec 16 (MCP tool wiring), spec 36 (voice mode shares
some MCP plumbing). Module-level `let binding` is a deliberate exception
to the no-module-state rule.

### `src/utils/Cursor.ts` — 47 KB

**Role.** Text cursor / kill-ring / word-segmentation logic for the
prompt input — Emacs-style line editing primitives. Maintains a **global
kill ring** (`KILL_RING_MAX_SIZE = 10`) shared across all input fields,
yank/yank-pop state, grapheme-aware cursor movement using Intl.Segmenter
(via `intl.ts`), wrap-aware width via `ink/wrapAnsi`, `stringWidth`.
**Public API.** `Cursor` class + free functions `pushToKillRing`,
`popFromKillRing`, `cycleKillRing`, plus word-boundary navigation helpers.
**Cross-spec.** spec 37 (TextInput, VimTextInput, BaseTextInput),
spec 39 (Vim mode), spec 21 (commands using PromptInput).

### `src/utils/ide.ts` — 47 KB

**Role.** **IDE integration core** — detection, JetBrains plugin install
state, MCP-via-IDE-RPC bridging, IdeOnboardingDialog lazy require,
WSL distro matching. Memoized. Ties together `idePathConversion.ts`
(WSL ↔ Windows) and `jetbrains.ts` (plugin presence) into a single API
the rest of the app can call. Uses `axios` + `execa` + raw TCP
(`createConnection`) for various IDE protocols.
**Public API.** `getIdeType`, `isJetBrainsPluginInstalledCached`,
`isSupportedJetBrainsTerminal`, `getTerminalIdeType`, `toIDEDisplayName`,
plus the dialog-firing flow.
**Cross-spec.** spec 24 (LSP service), spec 16 (MCP), spec 25 (auth flows
that surface IDE-related dialogs), spec 34 (bridge mode).

### `src/utils/swarm/It2SetupPrompt.tsx` — 43 KB

**Role.** Multi-step iTerm2 setup wizard component (Python package manager
detection → `pip install iterm2` → API verification → done/fail), spawned
when the swarm needs the iTerm2 Python API. Driven by Ink + custom
`Select`. Compiled by react/compiler-runtime — file is large because
of compiler-emitted memoization scaffolding (`useMemo` cache slots).
**Public API.** `<It2SetupPrompt onDone={…} tmuxAvailable={…} />`.
**Cross-spec.** spec 30 (coordinator), spec 14 (team setup), spec 37 (Ink).

### `src/utils/claudeInChrome/toolRendering.tsx` — 34 KB

**Role.** Per-tool message renderers for the Claude-in-Chrome MCP server's
17 browser tools (`navigate`, `read_page`, `find`, `form_input`,
`computer`, `javascript_tool`, etc.). Each tool has its own input and
result rendering branch using design-system `MessageResponse`,
`Link` (with hyperlink-supports detection), `truncateToWidth`. Tracks
last-used Chrome tab id for follow-up rendering.
**Public API.** Re-exports `ChromeToolName` union; provides
`getClaudeInChromeMCPRenderingOverrides()` consumed by MCP client.ts.
**Cross-spec.** spec 16 (MCP tool wiring), spec 37 (Ink rendering).

### `src/utils/bash/heredoc.ts` — 31 KB

**Role.** **Heredoc extraction and restoration** to work around
`shell-quote`'s misparse of `<<` as two `<` operators. Pre-extracts heredocs
into placeholder tokens, parses, then re-substitutes. Supports `<<WORD`,
`<<'WORD'`, `<<"WORD"`, `<<-WORD`, combined dash+quoted variants.
Documented limitations: heredocs inside backtick command substitution may
not extract; very complex multi-heredoc may not extract. Failures pass
through unchanged (safe — caller falls back to manual approval).
**Public API.** `extractHeredocs`, `restoreHeredocs`,
`HEREDOC_PLACEHOLDER_PREFIX/SUFFIX`.
**Cross-spec.** spec 10 (BashTool), spec 9 (permission engine).

### `src/utils/statusNoticeDefinitions.tsx` — 31 KB

**Role.** Catalog of status-notice banners shown in the prompt input
header (memory file too large, agent-descriptions over budget, JetBrains
terminal mismatched, etc.). Pure data declarations: each
`StatusNoticeDefinition` has `{ id, type: 'warning'|'info', isActive(ctx),
render(ctx) }`. Notices are evaluated against a `StatusNoticeContext`
containing config, agent definitions, memory file info.
**Public API.** `STATUS_NOTICE_DEFINITIONS` array, types `StatusNoticeType`
/ `StatusNoticeContext` / `StatusNoticeDefinition`.
**Cross-spec.** spec 37 (UI shell), spec 40 (memory), spec 41 (session).

### `src/utils/plugins/validatePlugin.ts` — 28 KB

**Role.** Validates `plugin.json` and `marketplace.json` files using zod
schemas. Surfaces warnings for marketplace-only fields accidentally placed
in plugin manifests (`category`, `source`, `tags`, `strict`, `id`) — known
plugin-author confusion point. Crawls plugin directories (commands, hooks,
skills, agents) checking frontmatter and JSON shape.
**Public API.** `validatePlugin(rootPath)`,
`validatePluginManifestText(text)`, supporting `MARKETPLACE_ONLY_MANIFEST_
FIELDS`. Produces structured warnings vs errors.
**Cross-spec.** spec 28 (plugin service), spec 21 (plugin commands).

### `src/utils/swarm/permissionSync.ts` — 26 KB

**Role.** Synchronized permission prompts for agent swarms. Workers
forward permission requests to the leader's mailbox; leader prompts the
user; response sent back to worker's mailbox. Filesystem-based message
passing using `lockfile`. Schema-validated with zod. Pull from
`PermissionUpdate`. Designed so workers can poll for responses without
needing leader IPC.
**Public API.** `sendPermissionRequest`, `pollPermissionResponses`,
`processPermissionMailbox`, `cleanupStalePermissions`.
**Cross-spec.** spec 30 (coordinator), spec 9 (permissions), spec 14 (team
tool).

### `src/utils/computerUse/executor.ts` — 24 KB

**Role.** CLI `ComputerExecutor` — wraps two native modules:
`@ant/computer-use-input` (Rust/enigo for mouse+keyboard+frontmost-app)
and `@ant/computer-use-swift` (SCContentFilter screenshots, NSWorkspace
apps, TCC). Maintains "CLI deltas from Cowork" (Anthropic desktop
counterpart): no `withClickThrough` (no overlay window), terminal as
surrogate host, clipboard via `pbcopy`/`pbpaste` instead of Electron's
`clipboard` module.
**Public API.** Implements the `ComputerExecutor` contract from
`packages/desktop/computer-use-mcp/src/executor.ts`.
**Cross-spec.** spec 16 (MCP), spec 36 (voice mode neighbor),
spec 18 (modes).

### `src/utils/git/gitFilesystem.ts` — 22 KB

**Role.** **Filesystem-based git state reading** — avoids spawning git
subprocesses. Resolves `.git` directories (including worktrees and
submodules), parses HEAD (ref or raw SHA), resolves refs via loose files
and packed-refs, owns `GitHeadWatcher` that caches branch/SHA via
`fs.watchFile`. Correctness notes verified against git source files
(`refs/files-backend.c`, `packed-backend.c`, `setup.c`, `shallow.c`).
**Public API.** `resolveGitDir`, `getCurrentBranch`, `getRefHash`,
`isShallow`, `GitHeadWatcher`. The performant alternative to shelling
`git rev-parse`.
**Cross-spec.** spec 11 (file tools), spec 42 (misc).

### `src/utils/processUserInput/processBashCommand.tsx` — 22 KB

**Role.** Routes `!`-prefixed user input (bash mode) through BashTool with
shell selection (default → bash, isPowerShellToolEnabled() override),
synthetic-message wrapping for the assistant transcript, attachment
handling, ShellError translation. Renders `BashModeProgress` UI.
**Public API.** `processBashCommand(inputString, precedingInputBlocks,
attachmentMessages, context, setToolJSX)` returning `{messages, shouldQuery}`.
**Cross-spec.** spec 4 (turn pipeline), spec 10 (BashTool), spec 18 (modes).

### `src/utils/bash/ShellSnapshot.ts` — 22 KB

**Role.** Captures and writes shell environment snapshots (env, alias, fn)
to a temp file consumed by spawned bash subshells via `BASH_ENV`. Uses
embedded ripgrep for file searches. 10 s creation timeout. Quotes via
shellQuote. Output is shell-source-able.
**Public API.** `createAndSaveSnapshot(cwd, options)`, `getSnapshotPath`.
**Cross-spec.** spec 10 (BashTool — every bash invocation reads it),
spec 42 (misc).

### `src/utils/swarm/backends/TmuxBackend.ts` — 21 KB

**Role.** tmux pane backend for multi-agent swarm. Tracks first-pane
external session, caches leader window target, `isInsideTmux` detection,
isolated socket via `getSwarmSocketName()`. Implements `PaneBackend`
interface (counterpart in `ITermBackend.ts` and `InProcessBackend.ts`).
**Public API.** `registerTmuxBackend()` (auto-call on import),
`createTmuxPane`, `attachTmuxPane`, `killTmuxPane`.
**Cross-spec.** spec 30 (coordinator), spec 14 (team tool).

### `src/utils/model/model.ts` — 21 KB

**Role.** **Model selection orchestration.** Resolves the active "main loop"
model from override flags, subscription tier, 1m-context eligibility,
`getMainLoopModelOverride`, GrowthBook gates. Subscriber-aware:
`isClaudeAISubscriber`, `isMaxSubscriber`, `isProSubscriber`,
`isTeamPremiumSubscriber`. ANT-ONLY codename strings are wrapped in
`USER_TYPE === 'ant'` so Bun strips them at build time (header comment
explains the leak avoidance). Adds `scripts/excluded-strings.txt`
co-maintenance contract.
**Public API.** `getRuntimeMainLoopModel`, `getCanonicalName`,
`parseUserSpecifiedModel`, `getSmallFastModel`.
**Cross-spec.** spec 22 (API service), spec 21 (commands like `/model`),
spec 6 (cost tracking).

### `src/utils/swarm/teamHelpers.ts` — 21 KB

**Role.** Team file operations and `spawnTeam`/`cleanup` ops. Reads
`~/.claude/teams/<id>/team.json`, knows `TEAM_LEAD_NAME`, isPaneBackend,
notifies `tasks.ts`. Zod-validated input schema.
**Public API.** `inputSchema` (zod), `runTeamOperation`,
`getTeamConfig(teamId)`, `cleanupTeam(teamId)`.
**Cross-spec.** spec 14 (Team tool), spec 30 (coordinator), spec 15 (tasks).

### `src/utils/markdownConfigLoader.ts` — 21 KB

**Role.** **CLAUDE.md / AGENTS.md / .claude/*.md hierarchical loader.**
Walks from project root through ancestors honoring git boundaries, parses
frontmatter, dedupes by canonical git root, evaluates settings sources
(`isSettingSourceEnabled`), invokes ripgrep for content searches.
Bun-feature-gated. Memoized.
**Public API.** `loadMarkdownConfig(opts)`, `findApplicableMarkdownFiles`.
**Cross-spec.** spec 5 (context assembly), spec 40 (memory), spec 29
(memory service).

### `src/utils/doctorDiagnostic.ts` — 21 KB

**Role.** Backs `claude doctor`. Detects shell type, install method (apk,
asdf, deb, brew, npm, native-bundled), checks for global-install perms,
runs autoupdater eligibility test, reports installation health. Cross-
references `localInstaller`, `autoUpdater`, `bundledMode`. Returns a
structured diagnostic report consumed by the doctor command UI.
**Public API.** `getCurrentInstallationType`, `runDoctorDiagnostics`.
**Cross-spec.** spec 21 (`/doctor` command catalog), spec 1 (bootstrap).

---

## §3 Mid-size utilities (5–20 KB) — 126 entries

Format: `path` · role · cross-spec.

| Path | Role | Cross-spec |
|---|---|---|
| `src/utils/gracefulShutdown.ts` | onExit signal-handler chain (chalk-coloured banner + persistence flag) | 1, 41 |
| `src/utils/processUserInput/processUserInput.ts` | Top-level user-input dispatcher (bash, slash command, text prompt) | 4 |
| `src/utils/preflightChecks.tsx` | Pre-API connectivity check Ink screen with axios SSL hint | 1, 25 |
| `src/utils/model/modelOptions.ts` | Builds model picker option list keyed by subscription tier | 21, 22 |
| `src/utils/autoUpdater.ts` | Periodic auto-updater w/ growthbook-gated dynamic config | 1, 42 |
| `src/utils/deepLink/terminalLauncher.ts` | Launch Claude inside the user's preferred terminal emulator (mac/linux/win) | 42 |
| `src/utils/cleanup.ts` | Periodic cache cleanup (image cache, old native-installer versions) | 42 |
| `src/utils/computerUse/toolRendering.tsx` | Per-tool input/result renderers for computer-use MCP | 16 |
| `src/utils/bash/treeSitterAnalysis.ts` | Tree-sitter AST analyzers for command security (counterpart to `ast.ts`) | 10, 9 |
| `src/utils/hooks/hooksConfigManager.ts` | Memoized hook lookup by event/matcher with priority sort | 42 |
| `src/utils/exportRenderer.tsx` | `/export` command rendered to ANSI string for file output | 21 |
| `src/utils/gitDiff.ts` | Diff-library wrapper that resolves repo root and calls git diff or in-memory patches | 11 |
| `src/utils/listSessionsImpl.ts` | Standalone listSessions for Agent SDK (no bootstrap deps) | 41 |
| `src/utils/swarm/backends/registry.ts` | Backend selection: prefers tmux/iterm2/in-process based on env detection | 30 |
| `src/utils/claudeInChrome/common.ts` | Chrome MCP server name + helpers (extension dir scanning, bundle id) | 16 |
| `src/utils/ShellCommand.ts` | TaskOutput-backed ShellCommand wrapper around ChildProcess; tree-kill termination | 10 |
| `src/utils/statsCache.ts` | On-disk stats cache (open()/randomBytes for atomic writes) | 6 |
| `src/utils/claudeInChrome/chromeNativeHost.ts` | Pure-TS Chrome native messaging host (replaces Rust NAPI binding) | 16 |
| `src/utils/tmuxSocket.ts` | Isolated `claude-<PID>` tmux socket so Claude doesn't kill user's tmux | 30 |
| `src/utils/task/diskOutput.ts` | Disk-backed task output ring with symlinks, fs constants, mkdir | 15 |
| `src/utils/ultraplan/ccrSession.ts` | CCR session polling for `/ultraplan` (ExitPlanMode tool_result extractor) | 14, 30 |
| `src/utils/swarm/backends/ITermBackend.ts` | iTerm2 pane backend (tracks session ids, first-pane reuse) | 30 |
| `src/utils/hooks/execAgentHook.ts` | Spawn an Agent-event hook as a child query() with disabled tools | 42 |
| `src/utils/task/TaskOutput.ts` | In-memory CircularBuffer over disk fallback (8 MB default, 1 s poll) | 15 |
| `src/utils/powershell/staticPrefix.ts` | PowerShell prefix extractor mirroring bash/prefix.ts via PS AST | 18, 9 |
| `src/utils/staticRender.tsx` | Render Ink subtree to a string (ink doesn't support nested `<Static>`) | 37 |
| `src/utils/hooks/sessionHooks.ts` | Session-event hook dispatch (SessionStart, etc.) | 42, 41 |
| `src/utils/nativeInstaller/pidLock.ts` | PID-based lock (vs mtime) — detects crashed owners immediately | 42 |
| `src/utils/deepLink/registerProtocol.ts` | Register `claude-cli://` per-platform (.app trampoline / .desktop / Win registry) | 42 |
| `src/utils/markdown.ts` | marked → ANSI renderer with theme color, hyperlinks, blockquote bar | 37, 38 |
| `src/utils/releaseNotes.ts` | GitHub releases fetcher + cached changelog parser; privacy-gated | 21 |
| `src/utils/shell/prefix.ts` | Haiku-LLM-driven command prefix extractor factory (shared bash/PS) | 9, 10 |
| `src/utils/claudeInChrome/mcpServer.ts` | stdio MCP server bootstrap for `@ant/claude-for-chrome-mcp` | 16, 23 |
| `src/utils/shell/bashProvider.ts` | Bash invocation provider (rearrange-pipe + snapshot + quote rules) | 10 |
| `src/utils/words.ts` | Random word slug list for plan ids (whimsical adjectives + nouns) | 42 |
| `src/utils/env.ts` | Memoized environment lookup (homedir, OAuth file suffix, exec discovery) | 1 |
| `src/utils/swarm/backends/PaneBackendExecutor.ts` | Common pane-backend executor — flag inheritance, env merge, mailbox writes | 30 |
| `src/utils/bash/shellQuote.ts` | Safe wrappers around shell-quote (parse error → log+pass-through) | 10 |
| `src/utils/bash/bashPipeCommand.ts` | Rearranges piped commands so stdin redirect attaches to first command, not eval | 10 |
| `src/utils/plugins/lspRecommendation.ts` | LSP plugin recommendation when binary present + file extension matches | 24, 28 |
| `src/utils/autoRunIssue.tsx` | "Auto-run blocked" Ink dialog (run/cancel keybindings, reason text) | 37 |
| `src/utils/swarm/backends/InProcessBackend.ts` | In-process backend (uses InProcessTeammateTask + mailbox) | 30, 14 |
| `src/utils/swarm/spawnInProcess.ts` | TeammateContext + AbortController + AppState registration | 30 |
| `src/utils/agenticSessionSearch.ts` | Side-query-driven semantic search across session transcripts | 41, 22 |
| `src/utils/skills/skillChangeDetector.ts` | Chokidar watch over skill dirs → invalidate command cache | 17 |
| `src/utils/heapDumpService.ts` | `/heapdump` backing — V8 heap snapshot stream, space stats | 21 |
| `src/utils/logoV2Utils.ts` | Logo / startup banner helpers — subscription, cwd, changelog excerpt | 37 |
| `src/utils/teleport/gitBundle.ts` | `git stash create`+`git bundle` → upload to /v1/files for CCR seed | 35 |
| `src/utils/tokens.ts` | Token usage extractor from message; rough token estimation glue | 6 |
| `src/utils/plugins/performStartupChecks.tsx` | Background plugin install + marketplace cache reset on startup | 28 |
| `src/utils/cron.ts` | Minimal 5-field cron expression parser (no L/W/?, local TZ) | 42 |
| `src/utils/mcp/elicitationValidation.ts` | MCP elicitation primitive-schema validation (enum, multi-select, string) | 16, 23 |
| `src/utils/format.ts` | Pure leaf-safe formatters: file size, relative time, plurals | 37 |
| `src/utils/bash/ParsedCommand.ts` | Memoized ParsedCommand wrapper combining commands.ts + treeSitterAnalysis | 10 |
| `src/utils/teammate.ts` | Teammate identity resolution (ALS → CLI flags → none) | 30, 14 |
| `src/utils/model/bedrock.ts` | AWS Bedrock inference profiles (memoized list, region prefix) | 22 |
| `src/utils/json.ts` | jsonc-parser wrapper with stat-cached parse, applyEdits, modify | 42 |
| `src/utils/messages/mappers.ts` | SDK message ↔ internal Message mappers (SDKAssistantMessage, etc.) | 4 |
| `src/utils/nativeInstaller/packageManagers.ts` | Detects homebrew, npm, yarn, pnpm, deb, apk, asdf | 42 |
| `src/utils/hooks/AsyncHookRegistry.ts` | Tracks pending async hooks, emits hook progress for SDK | 42 |
| `src/utils/queryProfiler.ts` | Per-query profiling via perf_hooks (CLAUDE_CODE_PROFILE_QUERY=1) | 42 |
| `src/utils/hooks/execHttpHook.ts` | Axios-based HTTP hook executor with SSRF guard, 10-min timeout | 42 |
| `src/utils/hooks/ssrfGuard.ts` | DNS lookup wrapper blocking RFC1918/link-local but allowing loopback | 42 |
| `src/utils/fullscreen.ts` | Tmux client_control_mode probe + alt-screen entry/exit | 37 |
| `src/utils/memoize.ts` | LRU memoizer with refresh-state + JSON-stringify keys | 42 |
| `src/utils/hooks/hooksSettings.ts` | hooks.json schema, sortMatchersByPriority, getAllHooks across sources | 42 |
| `src/utils/ansiToSvg.ts` | ANSI parser + SVG renderer (legacy; retained for ansiToPng's parser import) | 38 |
| `src/utils/sideQuery.ts` | Forked-context query for side questions (cache-friendly) | 22, 4 |
| `src/utils/transcriptSearch.ts` | In-memory transcript search avoiding rendered-as-sentinel false positives | 41 |
| `src/utils/managedEnv.ts` | Managed-env application: clears caches, configures global agents | 26, 27 |
| `src/utils/shell/specPrefix.ts` | Fig-spec-driven prefix extractor (shared bash/PS) | 9 |
| `src/utils/bash/shellCompletion.ts` | Shell-tab-completion suggestion provider for the prompt input | 37 |
| `src/utils/asciicast.ts` | Asciinema-format session recording (CLAUDE_CODE_ASCIICAST=1) | 41, 42 |
| `src/utils/contextAnalysis.ts` | Token-count analytics over normalized messages | 7, 5 |
| `src/utils/dxt/zip.ts` | DXT/zip extractor with traversal guard + 50:1 ratio limit | 28 |
| `src/utils/background/remote/preconditions.ts` | Subscription/repo gates before launching remote-session | 35 |
| `src/utils/contextSuggestions.ts` | Suggests when to use Bash/Read/Grep/WebFetch tools given context | 5 |
| `src/utils/readEditContext.ts` | Snippet extractor for edit-tool error context (8 KB chunks) | 11 |
| `src/utils/computerUse/computerUseLock.ts` | Single-instance lock for computer-use to prevent concurrent host control | 16 |
| `src/utils/desktopDeepLink.ts` | Open-Claude-Desktop check (min version 1.1.2396) | 42 |
| `src/utils/suggestions/directoryCompletion.ts` | LRU-cached `@`-mention directory completion | 37 |
| `src/utils/mcpOutputStorage.ts` | Spill-to-disk for oversized MCP tool results | 23 |
| `src/utils/promptShellExecution.ts` | Frontmatter `shell:` field execution for skill/command prompts | 17, 21 |
| `src/utils/swarm/backends/it2Setup.ts` | Detect Python pkg manager, install iterm2 module, verify API | 30 |
| `src/utils/hooks/execPromptHook.ts` | LLM-prompt-style hook executor (queryModelWithoutStreaming) | 42 |
| `src/utils/managedEnvConstants.ts` | Inference-routing env var allowlist; per-model launch maintenance note | 22, 26 |
| `src/utils/claudeInChrome/setupPortable.ts` | Chrome-extension install detection (prod + dev extension ids) | 16 |
| `src/utils/agentContext.ts` | AsyncLocalStorage agent context for analytics attribution (subagent vs teammate) | 14, 30 |
| `src/utils/git/gitConfigParser.ts` | Pure parser for `.git/config` (case-insensitive sections, quoted subsections) | 42 |
| `src/utils/editor.ts` | $EDITOR / GUI-editor classification + spawn (Memoized) | 21 |
| `src/utils/stringUtils.ts` | escapeRegExp, plural, safeJoinLines, capitalize | 42 |
| `src/utils/computerUse/appNames.ts` | Filter Spotlight app list (noise + prompt-injection hardening) | 16 |
| `src/utils/errorLogSink.ts` | File-based error log sink (separate from log.ts to avoid cycles) | 42 |
| `src/utils/claudeCodeHints.ts` | `<claude-code-hint />` stderr tag parser + module store | 28 |
| `src/utils/genericProcessUtils.ts` | Cross-platform `ps`/getAncestorPids (handles win32+cygwin+wsl + bsd-vs-unix) | 42 |
| `src/utils/suggestions/slackChannelSuggestions.ts` | Slack MCP-search-channels suggestion provider | 16 |
| `src/utils/mcpValidation.ts` | MCP tool result content-block validation (text/image limits, token estimate) | 23, 16 |
| `src/utils/cronTasksLock.ts` | Lease lock for `.claude/scheduled_tasks.json` (only one session = scheduler) | 42 |
| `src/utils/bash/prefix.ts` | bash-side prefix extractor: WRAPPER_COMMANDS list + spec walk via specPrefix.ts | 9 |
| `src/utils/exampleCommands.ts` | Stable per-cwd example prompt rotation (sample, save in project config) | 21 |
| `src/utils/powershell/dangerousCmdlets.ts` | Shared dangerous PS cmdlet list (Invoke-Expression, etc.) | 9, 10 |
| `src/utils/sideQuestion.ts` | `/btw` side-question runner (cache-friendly forked agent) | 22 |
| `src/utils/codeIndexing.ts` | Detection of Sourcegraph/Cody/etc. via CLI + MCP for analytics | 26 |
| `src/utils/startupProfiler.ts` | Startup phase profiling (sampled to Statsig + verbose mode) | 1, 26 |
| `src/utils/detectRepository.ts` | Parse remote URL into `{host, owner, name}` (ParsedRepository) | 42 |
| `src/utils/headlessProfiler.ts` | Per-turn TTFT profiler for `-p` print mode | 22 |
| `src/utils/mcpWebSocketTransport.ts` | WebSocket Transport implementation for MCP | 23 |
| `src/utils/model/modelAllowlist.ts` | Subscription/family allowlist filter for model picker | 22 |
| `src/utils/terminalPanel.ts` | Meta+J built-in terminal panel via per-instance tmux socket | 37 |
| `src/utils/windowsPaths.ts` | Win32 path conversion + cygpath wrapping; memoized | 42 |
| `src/utils/streamlinedTransform.ts` | "Distillation-resistant" SDK message format (text + cumulative tool counts) | 22 |
| `src/utils/jetbrains.ts` | JetBrains plugin install detection per IDE name | 24 |
| `src/utils/shell/powershellProvider.ts` | PS invocation flags + command (shared with hooks.ts) | 18 |
| `src/utils/truncate.ts` | Width-aware ellipsis truncation (CJK/emoji-correct via stringWidth) | 37 |
| `src/utils/deepLink/parseDeepLink.ts` | Parse `claude-cli://open?q=…&cwd=…&repo=…` URIs | 42 |
| `src/utils/promptEditor.ts` | $EDITOR-launched prompt editing with paste-ref expansion | 21 |
| `src/utils/completionCache.ts` | Shell completion script cache (chalk + hyperlinks for hint UI) | 42 |
| `src/utils/plugins/pluginFlagging.ts` | Tracks delisted/auto-removed plugins (~/.claude/plugins/flagged-plugins.json) | 28 |
| `src/utils/model/agent.ts` | Subagent/agent model resolution incl. Bedrock prefix logic | 22, 14 |
| `src/utils/groupToolUses.ts` | Group consecutive same-tool uses into a single rendered message | 4, 37 |
| `src/utils/plugins/hintRecommendation.ts` | Plugin hint recommendations (companion to lspRecommendation) | 28 |
| `src/utils/earlyInput.ts` | Capture stdin during startup before REPL ready | 1, 37 |
| `src/utils/hooks/fileChangedWatcher.ts` | Chokidar-driven file/cwd-changed hook dispatch | 42 |
| `src/utils/heatmap.ts` | Heatmap renderer for `/usage` activity grid (chalk colours, percentiles) | 21 |
| `src/utils/model/modelStrings.ts` | Bedrock inference-profile resolution + model string overrides | 22 |
| `src/utils/githubRepoPathMapping.ts` | `<owner>/<name>` → local-path config mapping (for deep links) | 42 |

---

## §4 Trivial utilities (<5 KB) — 178 entries

Single-line entries grouped by category. Inferred from filename + minimal
imports; full prose would not add information.

### Argument / parsing helpers
- `src/utils/argumentSubstitution.ts` — `{1}`/`{2}`/`{ARGS}` substitution in command/skill prompts.
- `src/utils/slashCommandParsing.ts` — Parses `/cmd arg1 arg2` syntax; aware of quoted args.
- `src/utils/cliArgs.ts` — Argv normalization + flag parsing helpers.
- `src/utils/processUserInput/processTextPrompt.ts` — Text-prompt sub-branch of `processUserInput`.

### Bash specs (fig-style)
- `src/utils/bash/specs/alias.ts` — Spec for `alias` builtin.
- `src/utils/bash/specs/pyright.ts` — Spec for `pyright` CLI.
- `src/utils/bash/specs/sleep.ts` — Spec for `sleep` (numeric arg).
- `src/utils/bash/specs/srun.ts` — Spec for SLURM `srun`.
- `src/utils/bash/specs/timeout.ts` — Spec for `timeout`.
- `src/utils/bash/registry.ts` — Registers all bash specs into a lookup.
- `src/utils/bash/shellPrefix.ts` — `formatShellPrefixCommand` — formats env-var prefixes.
- `src/utils/bash/shellQuoting.ts` — Cross-shell quoting (bash vs PowerShell).
- `src/utils/shell/powershellDetection.ts` — Detects pwsh vs powershell.exe.
- `src/utils/shell/resolveDefaultShell.ts` — Choose default shell from setting/env.

### Hooks (small)
- `src/utils/hooks/hooksConfigSnapshot.ts` — Snapshot of hooks settings for re-render comparison.
- `src/utils/hooks/hookEvents.ts` — Hook event emit / progress interval.
- `src/utils/hooks/apiQueryHookHelper.ts` — Helper for API-side hook injection.
- `src/utils/hooks/hookHelpers.ts` — Shared hook execution helpers.
- `src/utils/hooks/registerFrontmatterHooks.ts` — Register hooks declared in slash-command frontmatter.
- `src/utils/hooks/registerSkillHooks.ts` — Register hooks declared in skill frontmatter.

### Sessions
- `src/utils/sessionEnvironment.ts` — Env-snapshot caching for session.
- `src/utils/sessionEnvVars.ts` — Resolves `CLAUDE_CODE_*` env vars.
- `src/utils/sessionTitle.ts` — Generate session display title.
- `src/utils/sessionUrl.ts` — Build session deep-link URLs.
- `src/utils/sessionIngressAuth.ts` — Auth check for inbound session ingress.
- `src/utils/crossProjectResume.ts` — Resume sessions from a different project's transcript.
- `src/utils/suggestions/shellHistoryCompletion.ts` — Completion from `~/.bash_history` etc.
- `src/utils/ultraplan/keyword.ts` — Keyword extraction for ultraplan branching.

### Deep link
- `src/utils/deepLink/protocolHandler.ts` — Top-level dispatch on `claude-cli://` URI.
- `src/utils/deepLink/banner.ts` — One-time deep-link onboarding banner.
- `src/utils/deepLink/terminalPreference.ts` — Persist user's preferred terminal.

### Models (small)
- `src/utils/model/validateModel.ts` — Validate user-supplied model name.
- `src/utils/model/configs.ts` — Per-model static config map.
- `src/utils/model/modelCapabilities.ts` — Per-model capability bits (vision, 1m, etc.).
- `src/utils/model/aliases.ts` — Model alias table (`opus`, `sonnet`, etc.).
- `src/utils/model/antModels.ts` — ANT-only model codename strings.
- `src/utils/model/deprecation.ts` — Deprecation banners for retired models.
- `src/utils/model/modelSupportOverrides.ts` — Per-model override of capability bits.
- `src/utils/model/providers.ts` — `getAPIProvider(model) → 'anthropic' | 'bedrock' | 'vertex' | …`.
- `src/utils/model/check1mAccess.ts` — Check 1m-context eligibility per subscription.
- `src/utils/model/contextWindowUpgradeCheck.ts` — Periodic check for newly-eligible 1m context.

### Computer use (small)
- `src/utils/computerUse/cleanup.ts` — Disposer for CU resources.
- `src/utils/computerUse/common.ts` — Shared CU constants/utility.
- `src/utils/computerUse/drainRunLoop.ts` — Drain pending events when CU exits.
- `src/utils/computerUse/escHotkey.ts` — Esc-hotkey global registration to abort CU.
- `src/utils/computerUse/gates.ts` — GrowthBook gates (`tengu_malort_pedway`, etc.).
- `src/utils/computerUse/hostAdapter.ts` — Per-host CU adapter selection.
- `src/utils/computerUse/inputLoader.ts` — Lazy require for `@ant/computer-use-input`.
- `src/utils/computerUse/swiftLoader.ts` — Lazy require for `@ant/computer-use-swift`.
- `src/utils/computerUse/mcpServer.ts` — Stdio MCP server for CU.

### Swarm (small)
- `src/utils/swarm/backends/detection.ts` — `isInsideTmux/isInITerm2/...` probes.
- `src/utils/swarm/backends/teammateModeSnapshot.ts` — Snapshot for teammate-mode UI.
- `src/utils/swarm/teammateInit.ts` — Teammate initialization helpers.
- `src/utils/swarm/teammateLayoutManager.ts` — Pane layout manager (rows/cols).
- `src/utils/swarm/teammateModel.ts` — Per-teammate model selection.
- `src/utils/swarm/teammatePromptAddendum.ts` — Common prompt suffix for teammates.
- `src/utils/swarm/reconnection.ts` — Reconnect to existing swarm panes.
- `src/utils/swarm/leaderPermissionBridge.ts` — Marshal leader permission decisions to mailbox.

### MCP (small)
- `src/utils/mcp/dateTimeParser.ts` — Natural-language datetime → ISO 8601 for elicitation.
- `src/utils/mcpInstructionsDelta.ts` — Compute diff of MCP server instructions for re-prompt.

### Plugins (small)
- `src/utils/plugins/gitAvailability.ts` — Check git presence (gate plugin git-clone install).

### Sandbox / classifier
- `src/utils/sandbox/sandbox-ui-utils.ts` — Sandbox status display helpers.
- `src/utils/classifierApprovals.ts` — Auto-approval classifier interface.
- `src/utils/classifierApprovalsHook.ts` — Hook to feed classifier approvals.
- `src/utils/autoModeDenials.ts` — Track classifier denials for auto-mode.

### Process / lifecycle (small)
- `src/utils/abortController.ts` — `createAbortController()` ergonomics wrapper.
- `src/utils/combinedAbortSignal.ts` — Compose multiple abort signals.
- `src/utils/queueProcessor.ts` — Generic queue processor (n at a time).
- `src/utils/sequential.ts` — `sequential` async serializer.
- `src/utils/idleTimeout.ts` — Run callback after idle period of no activity.
- `src/utils/sleep.ts` — `sleep(ms)` Promise.
- `src/utils/withResolvers.ts` — Promise.withResolvers polyfill.
- `src/utils/signal.ts` — Lightweight signal/event primitive.
- `src/utils/lockfile.ts` — Filesystem lockfile (used by installer/sched lock).
- `src/utils/execFileNoThrowPortable.ts` — Portable execFile (no-throw) for SDK without bootstrap deps.
- `src/utils/execSyncWrapper.ts` — Deprecated execSync wrapper kept for legacy callers.
- `src/utils/process.ts` — Cross-platform process helpers.
- `src/utils/genericProcessUtils.ts` — see §3 (>5KB).
- `src/utils/which.ts` — Cross-platform `which`.
- `src/utils/findExecutable.ts` — Find an executable in PATH with fallbacks.
- `src/utils/standaloneAgent.ts` — Spawn agent as standalone subprocess.

### Profiling / debug (small)
- `src/utils/profilerBase.ts` — Shared base for query/startup/headless profilers.
- `src/utils/fpsTracker.ts` — Render-frame FPS tracker for Ink shell.
- `src/utils/debugFilter.ts` — Filter debug log entries by namespace.
- `src/utils/unaryLogging.ts` — Log a single statement with deduplication.
- `src/utils/telemetryAttributes.ts` — Common analytics attributes.
- `src/utils/warningHandler.ts` — Process `warning` event filter.

### Format / string (small)
- `src/utils/formatBriefTimestamp.ts` — "5m ago" style timestamps.
- `src/utils/treeify.ts` — Tree-print nested data.
- `src/utils/displayTags.ts` — Render `<system-reminder>` etc. tags as labels.
- `src/utils/textHighlighting.ts` — Match-highlight runs in a string.
- `src/utils/highlightMatch.tsx` — React variant for Ink rendering.
- `src/utils/cliHighlight.ts` — Syntax highlighting for code blocks (chalk based).
- `src/utils/sliceAnsi.ts` — Slice ANSI-aware substring.
- `src/utils/horizontalScroll.ts` — Horizontal-scroll line of long content.
- `src/utils/hyperlink.ts` — OSC-8 hyperlink wrapping.

### Data structures (small)
- `src/utils/CircularBuffer.ts` — Fixed-size ring buffer.
- `src/utils/array.ts` — `count`, `chunk`, `unique` helpers.
- `src/utils/contentArray.ts` — Helpers for `ContentBlockParam[]` arrays.
- `src/utils/objectGroupBy.ts` — Polyfill for `Object.groupBy`.
- `src/utils/set.ts` — Set helpers (intersect/union).
- `src/utils/generators.ts` — Async-generator helpers.

### Crypto / id (small)
- `src/utils/hash.ts` — Stable string hash (FNV/xxhash-like).
- `src/utils/uuid.ts` — UUID v4 wrapper.
- `src/utils/taggedId.ts` — Branded-string id factory (`type X = string & {__tag}`).
- `src/utils/fingerprint.ts` — Stable client fingerprint for analytics.
- `src/utils/peerAddress.ts` — Resolve peer address for IPC.

### Image / clipboard
- `src/utils/imageStore.ts` — On-disk image cache for paste/screenshot.
- `src/utils/imageValidation.ts` — Image format/size validation.
- `src/utils/screenshotClipboard.ts` — Copy a Buffer/PNG to system clipboard.
- `src/utils/pasteStore.ts` — Pasted-content store with refs (`#1`, `#2`).
- `src/utils/appleTerminalBackup.ts` — Backup Apple Terminal preferences.
- `src/utils/iTermBackup.ts` — Backup iTerm2 preferences.

### IDE / editor (small)
- `src/utils/idePathConversion.ts` — WSL distro/Win path conversion.
- `src/utils/jetbrains.ts` — see §3.

### Env / settings (small)
- `src/utils/envValidation.ts` — Validate user-set env vars.
- `src/utils/subprocessEnv.ts` — Compute env to pass to spawned subprocesses.
- `src/utils/caCertsConfig.ts` — Resolve user-supplied CA certs.
- `src/utils/xdg.ts` — XDG_CONFIG_HOME / XDG_DATA_HOME helpers.
- `src/utils/cachePaths.ts` — Per-cache-domain path constants.
- `src/utils/systemDirectories.ts` — Cross-platform standard dirs.
- `src/utils/systemTheme.ts` — Detect system light/dark mode.
- `src/utils/platform.ts` — `getPlatform() → 'darwin'|'linux'|'win32'`.
- `src/utils/aws.ts` — AWS region/account helpers.
- `src/utils/awsAuthStatusManager.ts` — AWS auth status banner state.
- `src/utils/authPortable.ts` — Auth functions for SDK without bootstrap.
- `src/utils/browser.ts` — Open URL in default browser cross-platform.
- `src/utils/intl.ts` — Cached `Intl.Segmenter` / `Intl.RelativeTimeFormat`.

### Output / IO (small)
- `src/utils/bufferedWriter.ts` — Coalesced async file writer.
- `src/utils/stream.ts` — Async iterator helpers over streams.
- `src/utils/streamJsonStdoutGuard.ts` — Guard against stdout pollution in stream-json mode.
- `src/utils/sdkEventQueue.ts` — Event queue for SDK callbacks.
- `src/utils/sanitization.ts` — String sanitization for log output.
- `src/utils/task/outputFormatting.ts` — Format task output for transcript.
- `src/utils/task/sdkProgress.ts` — SDK progress event mapping.

### Teleport / remote
- `src/utils/teleport/environments.ts` — Known teleport environment list.
- `src/utils/teleport/environmentSelection.ts` — Pick environment based on subscription/region.

### Network
- `src/utils/apiPreconnect.ts` — TCP preconnect to api.anthropic.com to warm DNS.

### Plugins / collapsing
- `src/utils/collapseBackgroundBashNotifications.ts` — Collapse N "background bash done" into one.
- `src/utils/collapseHookSummaries.ts` — Collapse N hook summaries.
- `src/utils/collapseTeammateShutdowns.ts` — Collapse N teammate-shutdown lines.

### Generated artifact
- `src/utils/generatedFiles.ts` — Markers for build-time generated files.
- `src/utils/zodToJsonSchema.ts` — Convert Zod schema → JSON Schema for tool/MCP definitions.
- `src/utils/yaml.ts` — Tiny YAML wrapper (parseYaml).

### Tool result / message
- `src/utils/toolPool.ts` — Pool/registry of tools.
- `src/utils/toolErrors.ts` — Standardized tool error shapes.
- `src/utils/toolSchemaCache.ts` — Cached schema shape per tool name.
- `src/utils/messagePredicates.ts` — `isAssistantMessage`, `isToolResultMessage`, etc.
- `src/utils/userPromptKeywords.ts` — Keyword detection for routing/UI hints.
- `src/utils/modifiers.ts` — Cross-platform modifier-key normalization.
- `src/utils/keyboardShortcuts.ts` — Keyboard shortcut display strings.

### File / filesystem (small)
- `src/utils/fileReadCache.ts` — TTL cache for file reads in tool dispatch.
- `src/utils/filePersistence/outputsScanner.ts` — Scan outputs dir for stale files.
- `src/utils/getWorktreePaths.ts` — Compute worktree path mapping.
- `src/utils/getWorktreePathsPortable.ts` — Portable variant for SDK.
- `src/utils/git/gitignore.ts` — Parse .gitignore patterns.
- `src/utils/tempfile.ts` — Tempfile creation helper.
- `src/utils/binaryCheck.ts` — Detect if a file is binary (heuristic).

### MCP / settings (smaller)
- `src/utils/QueryGuard.ts` — Lightweight guard for query-level operations.
- `src/utils/extraUsage.ts` — Extra-usage feature gating.
- `src/utils/inProcessTeammateHelpers.ts` — Helper functions for in-process teammates.
- `src/utils/teammateContext.ts` — AsyncLocalStorage context for teammates.
- `src/utils/teamDiscovery.ts` — Discover existing teams in current cwd.
- `src/utils/teamMemoryOps.ts` — Team memory file operations.
- `src/utils/memory/versions.ts` — MEMORY.md format versioning.
- `src/utils/jsonRead.ts` — `stripBOM` + JSON read helper.
- `src/utils/directMemberMessage.ts` — Send direct message to specific teammate.
- `src/utils/mailbox.ts` — Tiny mailbox primitive (used by team comms).
- `src/utils/workloadContext.ts` — Workload (worker-thread) AsyncLocalStorage.
- `src/utils/controlMessageCompat.ts` — Backwards-compatible control message shapes.
- `src/utils/commandLifecycle.ts` — Pre/post hooks around command run.
- `src/utils/statusNoticeHelpers.ts` — Helpers used by `statusNoticeDefinitions.tsx`.
- `src/utils/github/ghAuthStatus.ts` — `gh auth status` parsing.
- `src/utils/ghPrStatus.ts` — `gh pr status` parsing.
- `src/utils/semver.ts` — `gt`/`lt`/`gte` semver helpers.

---

## §5 Cross-spec dependency map / reassignment notes

Some files were placed under `src/utils/` but architecturally belong with
sibling subsystems. They remain cataloged here for completeness but should
be cross-referenced from their natural spec.

| Suggested home spec | Files |
|---|---|
| 9 / 10 — permissions & BashTool | `bash/ast.ts`, `bash/treeSitterAnalysis.ts`, `bash/heredoc.ts`, `bash/ParsedCommand.ts`, `bash/prefix.ts`, `bash/ShellSnapshot.ts`, `bash/shellQuote.ts`, `bash/shellQuoting.ts`, `bash/bashPipeCommand.ts`, `bash/shellCompletion.ts`, `bash/registry.ts`, `bash/shellPrefix.ts`, `bash/specs/*`, `shell/readOnlyCommandValidation.ts`, `shell/specPrefix.ts`, `shell/prefix.ts`, `shell/bashProvider.ts`, `shell/powershellProvider.ts`, `shell/powershellDetection.ts`, `shell/resolveDefaultShell.ts`, `powershell/dangerousCmdlets.ts`, `powershell/staticPrefix.ts` (~22 files) |
| 14 / 30 — agent/team/coordinator | all `swarm/*` (21 files), `teammate.ts`, `teammateContext.ts`, `teamDiscovery.ts`, `teamMemoryOps.ts`, `directMemberMessage.ts`, `inProcessTeammateHelpers.ts`, `agentContext.ts`, `agentId.ts` (excluded — already cited), `model/agent.ts` (~28 files) |
| 16 — MCP/computer-use | all `computerUse/*` (16 files), all `claudeInChrome/*` (5 files), `mcpValidation.ts`, `mcpOutputStorage.ts`, `mcpInstructionsDelta.ts`, `mcpWebSocketTransport.ts`, `mcp/elicitationValidation.ts`, `mcp/dateTimeParser.ts` (~28 files) |
| 22 — API service / model | all `model/*` (17 files), `streamlinedTransform.ts`, `tokens.ts`, `headlessProfiler.ts`, `queryProfiler.ts`, `sideQuery.ts`, `sideQuestion.ts` (~22 files) |
| 28 — plugins | all `plugins/*` (6 files), `claudeCodeHints.ts`, `dxt/*` (2 files) |
| 35 — remote/teleport | `teleport/*` (3 files), `background/remote/preconditions.ts`, `desktopDeepLink.ts` |
| 41 — session/history | `agenticSessionSearch.ts`, `transcriptSearch.ts`, `listSessionsImpl.ts`, `sessionEnvironment.ts`, `sessionEnvVars.ts`, `sessionTitle.ts`, `sessionUrl.ts`, `sessionIngressAuth.ts`, `crossProjectResume.ts`, `ultraplan/ccrSession.ts` |
| 42 — genuinely cross-cutting | the remainder (process/lifecycle, env, format, data structures, IDs, etc.) |

**Reassignment surprise.** Roughly **75 of the 327 residuals (23 %) belong
under another existing spec** — they're under `src/utils/` only because the
codebase predates the spec partitioning. The reassignment table should
inform any future re-shuffle of the catalog; for now, this spec serves as
the "of record" enumeration and other specs cite into it.

---

## §6 LOC histogram

| Bucket | Files | Cumulative size |
|---|---:|---:|
| Large (>20 KB) | 23 | ~1.40 MB |
| Mid (5–20 KB) | 126 | ~1.43 MB |
| Trivial (<5 KB) | 178 | ~0.40 MB |
| **Total** | **327** | **~3.23 MB** |

Single 215 KB outlier (`ansiToPng.ts`) skews the large-bucket distribution
heavily — it accounts for ~15 % of the entire residual catalog by size and
is essentially an embedded font asset, not handwritten code.

---

## §7 Phase 10 cleanup additions

Files in `src/utils/` (and `src/utils/bash/specs/`, `src/utils/dxt/`) that
weren't covered by §2's per-file walk. One-line catalog so the basename
appears in the spec corpus.

| File | Purpose |
|---|---|
| `src/utils/activityManager.ts` | `ActivityManager` class — generic activity tracking that deduplicates overlapping operations and reports separate user vs CLI active-time metrics. `USER_ACTIVITY_TIMEOUT_MS=5s` window for user activity. Backed by `getActiveTimeCounter` from `bootstrap/state.ts`. |
| `src/utils/bash/specs/nohup.ts` | `CommandSpec` for `nohup` — declares `name`, `description`, and a single `isCommand: true` arg so the bash parser treats the trailing tokens as a nested command (immune to hangups). |
| `src/utils/bash/specs/time.ts` | `CommandSpec` for `time` — same shape as `nohup`: a single `isCommand: true` arg so the bash parser nests the wrapped command (timing). |
| `src/utils/claudeDesktop.ts` | Claude Desktop integration helpers — locate `claude_desktop_config.json` per-platform (macOS via `~/Library/Application Support/Claude/`, WSL via Windows `%APPDATA%`), parse and validate `McpStdioServerConfigSchema` entries. Throws on unsupported platforms. |
| `src/utils/dxt/helpers.ts` | DXT/MCPB manifest validation — lazy-imports `@anthropic-ai/mcpb` (zod v3, ~700 KB of bound closures) so the cost is only paid when validating `.dxt`/`.mcpb` packages. Returns flattened error messages on parse failure. |
| `src/utils/shellConfig.ts` | Shell config-file (`.bashrc`/`.zshrc`) management for installer alias + PATH entries. `CLAUDE_ALIAS_REGEX` matches `alias claude=...` lines. Respects `ZDOTDIR` for zsh users. Used by `localInstaller.ts`. |
| `src/utils/userAgent.ts` | `getClaudeCodeUserAgent()` — returns `claude-code/${MACRO.VERSION}`. Kept dependency-free so SDK-bundled code (bridge, `cli/transports/*`) can import without dragging in `auth.ts` and its transitive dependency tree. |

