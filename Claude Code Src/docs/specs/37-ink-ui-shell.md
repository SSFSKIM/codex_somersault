# 37 — Ink UI Shell Specification

> **Status**: done · **Owner**: sub-H7 · **Last updated**: 2026-05-10
> **Companion catalogs (Phase 9)**: [37a — components catalog](./37a-components-catalog.md),
> [37b — hooks catalog](./37b-hooks-catalog.md),
> [37c — ink primitives catalog](./37c-ink-primitives-catalog.md). Per-component,
> per-hook, and per-ink-primitive enumeration lives there; this spec owns the
> contract surface and reimplementer-level behavior.
> **Adjacent (do not redocument)**: 09 (permission protocol), 16 (MCP rich
> output payloads), 34 (bridge IPC), 35 (remote IPC), 36 (voice STT),
> 38 (output styles), 39 (vim & keybindings), 41 (session/history state).

## 1. Purpose & Scope

Spec 37 owns the **Ink-based terminal UI shell** of Claude Code: the
React-for-the-terminal renderer fork (`src/ink/`), the public `ink.ts`
re-export façade, the design system, the ~140-component widget library
(`src/components/`), the screen surfaces (`src/screens/`), the dialog
launcher utilities (`src/dialogLaunchers.tsx`,
`src/interactiveHelpers.tsx`, `src/replLauncher.tsx`), the theme system
(`src/utils/theme.ts`), spinner glyphs and animation, the prompt-input
footer/status bar, the trust dialog, the MCP-server-approval dialog,
the permission-dialog rendering chrome, voice indicators, the
LogoV2/welcome screen, and the agent-color manager UI consumption.

**In scope**: Ink renderer + reconciler + layout pipeline (consumption
documented at the level required for reimplementation; the renderer is a
local fork, not the upstream `ink` package); the `ThemeProvider`
wrapper; the dialog/REPL launcher pattern; dialog string surfaces
verbatim; spinner frame sequences and reduced-motion fallback; UI
deltas for `HISTORY_PICKER`, `HOOK_PROMPTS`, `MESSAGE_ACTIONS`,
`LODESTONE`, `STREAMLINED_OUTPUT`, `MCP_RICH_OUTPUT`, `AUTO_THEME`,
`BUDDY` (companion sprite), `VOICE_MODE` indicators.

**Out of scope** (refer by spec):
- Permission decision tree, `PermissionResult` schema, deny-rule
  matching → spec 09 (this spec owns the **renderer** of those
  decisions; it does not own the **logic**).
- MCP rich-output payload classification (image/audio/resource link
  branching) → spec 16. Spec 37 only owns the React tree.
- Vim keybindings dispatch + chord state machine → spec 39. Spec 37
  consumes `useKeybinding`/`useKeybindings` and renders
  `KeyboardShortcutHint`.
- Output styles (system-prompt deltas) → spec 38. Spec 37 owns the
  `OutputStylePicker` UI but not the prompt mutation.
- Voice STT engine, key-term extraction, server connection → spec 36.
  Spec 37 owns `VoiceIndicator` and `VoiceModeNotice`.
- Bridge / remote IPC → spec 34/35. Spec 37 owns `BridgeStatusIndicator`,
  `BridgeDialog`, `RemoteCallout`.
- Session state / history persistence → spec 41. Spec 37 owns
  `HistorySearchDialog` UI and `MessageSelector` cursor; not the
  history file format.

---

## 2. Source Map

### 2.1 Source-coverage inventory

| Path | Status |
|---|---|
| `src/ink.ts` (85 lines) | **fully read** |
| `src/ink/constants.ts` (3 lines, `FRAME_INTERVAL_MS = 16`) | **fully read** |
| `src/ink/ink.tsx` (1722 lines) | **grep-inspected** (renderer/onFrame/patchConsole) |
| `src/ink/components/`, `src/ink/hooks/`, `src/ink/events/`, `src/ink/layout/`, `src/ink/termio/` | **listed**; representative files referenced |
| `src/ink/Ansi.tsx`, `colorize.ts`, `styles.ts`, `wrap-text.ts`, `tabstops.ts`, `bidi.ts`, `searchHighlight.ts`, `selection.ts`, `optimizer.ts`, `reconciler.ts`, `renderer.ts`, `root.ts`, `screen.ts`, `terminal.ts`, `terminal-querier.ts`, `terminal-focus-state.ts` | **grep-inspected** |
| `src/components/design-system/color.ts` | **fully read** |
| `src/components/design-system/ThemeProvider.tsx` | **fully read** |
| `src/components/design-system/{ThemedBox,ThemedText,Dialog,Divider,Pane,Tabs,FuzzyPicker,KeyboardShortcutHint,ListItem,LoadingState,ProgressBar,Ratchet,StatusIcon,Byline}.tsx` | **listed** |
| `src/components/Spinner/{SpinnerGlyph,utils,index}.tsx` | **fully read** |
| `src/components/Spinner/{FlashingChar,GlimmerMessage,ShimmerChar,SpinnerAnimationRow,TeammateSpinnerLine,TeammateSpinnerTree,useShimmerAnimation,useStalledAnimation}.tsx` | **listed** |
| `src/components/Spinner.tsx` (38KB) | **grep-inspected** |
| `src/components/PromptInput/PromptInput.tsx` (2338 lines) | **grep-inspected** for `feature('HISTORY_PICKER')` sites |
| `src/components/PromptInput/PromptInputFooter.tsx` (190 lines) | **fully read** |
| `src/components/PromptInput/{PromptInputFooterLeftSide,PromptInputFooterSuggestions,PromptInputHelpMenu,PromptInputModeIndicator,PromptInputQueuedCommands,PromptInputStashNotice,SandboxPromptFooterHint,ShimmeredInput,VoiceIndicator,Notifications}.tsx` | **listed** + grep |
| `src/components/permissions/{PermissionDialog,PermissionRequest,PermissionPrompt,FallbackPermissionRequest}.tsx` | **fully read** |
| `src/components/permissions/{PermissionRequestTitle,PermissionRuleExplanation,PermissionExplanation,PermissionDecisionDebugInfo,SandboxPermissionRequest,WorkerBadge,WorkerPendingPermission,shellPermissionHelpers}.tsx` | **listed** |
| `src/components/permissions/{AskUserQuestion,Bash,EnterPlanMode,ExitPlanMode,FileEdit,Filesystem,FileWrite,NotebookEdit,PowerShell,Skill,WebFetch,SedEdit,FilePermissionDialog,ComputerUseApproval,rules}/` | **listed** |
| `src/components/TrustDialog/TrustDialog.tsx` (289 lines) | **fully read** |
| `src/components/TrustDialog/utils.ts` | **listed** |
| `src/components/MCPServerApprovalDialog.tsx`, `MCPServerDialogCopy.tsx` | **fully read** |
| `src/components/MCPServerDesktopImportDialog.tsx`, `MCPServerMultiselectDialog.tsx`, `mcp/` | **listed** |
| `src/components/StatusLine.tsx` (323 lines), `StatusNotices.tsx` (54 lines) | **grep-inspected** |
| `src/components/Messages.tsx` (833 lines), `MessageSelector.tsx` (830 lines), `Message.tsx`, `MessageRow.tsx`, `MessageModel.tsx`, `MessageResponse.tsx`, `MessageTimestamp.tsx`, `messageActions.tsx`, `messages/` | **listed** |
| `src/components/LogoV2/{LogoV2,WelcomeV2,Clawd,AnimatedClawd,AnimatedAsterisk,CondensedLogo,Feed,FeedColumn,feedConfigs,ChannelsNotice,EmergencyTip,GuestPassesUpsell,Opus1mMergeNotice,OverageCreditUpsell,VoiceModeNotice}.tsx` | **listed** |
| `src/components/Onboarding.tsx`, `App.tsx`, `BridgeDialog.tsx`, `BypassPermissionsModeDialog.tsx`, `AutoModeOptInDialog.tsx`, `ClaudeMdExternalIncludesDialog.tsx`, `DevChannelsDialog.tsx`, `ChannelDowngradeDialog.tsx`, `ApproveApiKey.tsx`, `ExitFlow.tsx`, `IdleReturnDialog.tsx`, `Feedback.tsx`, `Stats.tsx`, etc. | **listed** |
| `src/components/{HelpV2/,HighlightedCode/,StructuredDiff*,Settings/,wizard/,FeedbackSurvey/,ClaudeCodeHint/,DesktopUpsell/,grove/,sandbox/,shell/,skills/,tasks/,teams/,ui/,Passes/,memory/,agents/,diff/,hooks/,CustomSelect/}` | **listed** |
| `src/screens/REPL.tsx` (5005 lines) | **grep-inspected** for animation intervals + flag sites |
| `src/screens/Doctor.tsx` (574 lines), `ResumeConversation.tsx` (398 lines) | **listed** |
| `src/dialogLaunchers.tsx` (132 lines) | **fully read** |
| `src/interactiveHelpers.tsx` (365 lines) | **fully read** |
| `src/replLauncher.tsx` (22 lines) | **fully read** |
| `src/utils/theme.ts` (639 lines) | **fully read** |
| `src/utils/renderOptions.ts` (77 lines) | **grep-inspected** |
| `src/keybindings/defaultBindings.ts` | **grep-inspected** for `MESSAGE_ACTIONS`/`QUICK_SEARCH` flag rows |
| `src/cli/print.ts:857` (`STREAMLINED_OUTPUT` site) | **grep-inspected** |

### 2.2 Imports from / imported by (high level)

`ink.ts` → `./components/design-system/ThemeProvider`,
`./ink/root`, `./ink/colorize`, `./ink/Ansi`, ink/components/{App,Box,
Button,Link,Newline,NoSelect,RawAnsi,Spacer,StdinContext,Text},
ink/events/{click,emitter,event,input,terminal-focus},
ink/{focus,frame,measure-element,wrap-text,termio/osc},
ink/hooks/{use-animation-frame,use-app,use-input,use-interval,
use-selection,use-stdin,use-tab-status,use-terminal-focus,
use-terminal-title,use-terminal-viewport}.

Imported by: `interactiveHelpers.tsx`, `dialogLaunchers.tsx`,
`replLauncher.tsx`, the entire `src/components/` tree (via the public
`Box`/`Text`/`Link`/`Button`/`useTheme` re-exports), `screens/`,
`tools/<*>Tool/UI.tsx`, top-level `main.tsx`.

### 2.3 Feature-flag and ANT guard locations (UI-side)

| Flag | UI site | Spec citation |
|---|---|---|
| `HISTORY_PICKER` | `components/PromptInput/PromptInput.tsx:1721,1727,2144`; `hooks/useHistorySearch.ts:240` | this spec |
| `HOOK_PROMPTS` | `screens/REPL.tsx:2520` (`requestPrompt: feature('HOOK_PROMPTS') ? requestPrompt : undefined`) | this spec + 09 |
| `MESSAGE_ACTIONS` | `keybindings/defaultBindings.ts:88,268`; `screens/REPL.tsx:606,4562,4905` | this spec + 39 |
| `LODESTONE` | `interactiveHelpers.tsx:176` (`updateDeepLinkTerminalPreference()`); `main.tsx:647,3781`; `utils/backgroundHousekeeping.ts:10,39`; `utils/settings/types.ts:808` | this spec |
| `STREAMLINED_OUTPUT` | `cli/print.ts:857` | this spec + 38 |
| `MCP_RICH_OUTPUT` | `tools/MCPTool/UI.tsx:51` | spec 16 (rendering owner; classification owner) |
| `AUTO_THEME` | `components/design-system/ThemeProvider.tsx:65` (`watchSystemTheme` import gate) | this spec |
| `VOICE_MODE` | `screens/REPL.tsx:4905` (`insertTextRef`); `components/PromptInput/VoiceIndicator.tsx`, `LogoV2/VoiceModeNotice.tsx` | spec 36 (engine); 37 (UI) |
| `BRIDGE_MODE` | `components/PromptInput/PromptInputFooter.tsx:160` (`BridgeStatusIndicator`) | spec 34 (logic); 37 (renderer) |
| `BUDDY` | `screens/REPL.tsx:4590` (companion sprite layout) | this spec |
| `KAIROS_CHANNELS` | `interactiveHelpers.tsx:241` | spec 32 (logic); 37 (DevChannelsDialog) |
| `TRANSCRIPT_CLASSIFIER` | `interactiveHelpers.tsx:224` (`AutoModeOptInDialog`) | spec 09 (mode); 37 (dialog) |
| ANT (`USER_TYPE === 'ant'`) | `ink/termio/osc.ts:468` (terminal OSC variants); `interactiveHelpers.tsx:146` literal `"external" === 'ant'` (DCE marker); `components/PromptInput/PromptInputFooter.tsx:146,150` likewise | this spec |

`"external" === 'ant'` literal evaluates statically to `false` in
external builds; the bundler strips the branch. The leaked tree
contains these as the post-DCE-marker form (the original was
`process.env.USER_TYPE === 'ant'`).

### 2.4 Source-gap notes

- `src/components/Messages.tsx` and `MessageSelector.tsx` (~830 LOC each)
  exceed the read window; structure inferred from grep + adjacent
  imports. Detail-level claims about message rendering pipelines that
  are not grep-confirmed are flagged in §12.
- `src/screens/REPL.tsx` (5005 LOC) was grep-inspected only. All
  REPL-specific behavioral claims in this spec cite a grep-confirmed
  line or are flagged in §12.
- `src/components/Spinner.tsx` (~38KB) was grep-inspected; spinner
  glyph + reduced-motion semantics are bit-exact from
  `Spinner/SpinnerGlyph.tsx` + `Spinner/utils.ts`. The teammate-spinner
  layout details are in `Spinner/TeammateSpinnerTree.tsx` and not
  redocumented here.
- **Hooks giants not enumerated here.** `src/hooks/useTypeahead.tsx`
  (~207KB / 1384 LOC) and `src/hooks/useReplBridge.tsx` (~113KB / 722
  LOC) are the two largest UI-shell hooks; they exceed the read window
  and are deferred to spec 37b. This spec cites them only at the
  contract surface (e.g. `HISTORY_PICKER` flag site at
  `useHistorySearch.ts:240`); per-hook state machines live in 37b.

---

## 3. Public Interface (Contract)

### 3.1 `ink.ts` exports (verbatim signature surface)

```ts
export async function render(node: ReactNode,
  options?: NodeJS.WriteStream | RenderOptions): Promise<Instance>
export async function createRoot(options?: RenderOptions): Promise<Root>
export { color }                                  // theme-aware
export type { Props as BoxProps }; export { default as Box }
export type { Props as TextProps }; export { default as Text }
export { ThemeProvider, usePreviewTheme, useTheme, useThemeSetting }
export { Ansi }
export type { Props as AppProps }
export type { Props as BaseBoxProps }; export { default as BaseBox }
export type { ButtonState, Props as ButtonProps }
export { default as Button }
export type { Props as LinkProps };   export { default as Link }
export type { Props as NewlineProps };export { default as Newline }
export { NoSelect }; export { RawAnsi }; export { default as Spacer }
export type { Props as StdinProps }
export type { Props as BaseTextProps }; export { default as BaseText }
export type { DOMElement }
export { ClickEvent }; export { EventEmitter }; export { Event }
export type { Key }; export { InputEvent }
export type { TerminalFocusEventType }; export { TerminalFocusEvent }
export { FocusManager }
export type { FlickerReason }
export { useAnimationFrame, useApp, useInput,
         useAnimationTimer, useInterval,
         useSelection, useStdin, useTabStatus,
         useTerminalFocus, useTerminalTitle, useTerminalViewport }
export { default as measureElement }
export { supportsTabStatus }
export { default as wrapText }
```
(Source: `src/ink.ts:18-85`.)

The shell wraps every render call with the `ThemeProvider` so that
`ThemedBox`/`ThemedText` resolve their colors without each call site
mounting a provider (`src/ink.ts:14-31`):

```ts
function withTheme(node: ReactNode): ReactNode {
  return createElement(ThemeProvider, null, node)
}
export async function render(node, options) {
  return inkRender(withTheme(node), options)
}
export async function createRoot(options) {
  const root = await inkCreateRoot(options)
  return { ...root, render: node => root.render(withTheme(node)) }
}
```

### 3.2 Dialog launcher contract (`interactiveHelpers.tsx`)

```ts
export function showDialog<T = void>(
  root: Root,
  renderer: (done: (result: T) => void) => React.ReactNode
): Promise<T>

export async function exitWithError(
  root: Root, message: string, beforeExit?: () => Promise<void>
): Promise<never>

export async function exitWithMessage(
  root: Root, message: string,
  options?: { color?: TextProps['color']; exitCode?: number;
              beforeExit?: () => Promise<void> }
): Promise<never>

export function showSetupDialog<T = void>(
  root: Root,
  renderer: (done: (result: T) => void) => React.ReactNode,
  options?: { onChangeAppState?: typeof onChangeAppState }
): Promise<T>

export async function renderAndRun(
  root: Root, element: React.ReactNode
): Promise<void>

export async function showSetupScreens(
  root: Root, permissionMode: PermissionMode,
  allowDangerouslySkipPermissions: boolean,
  commands?: Command[], claudeInChrome?: boolean,
  devChannels?: ChannelEntry[]
): Promise<boolean>

export function getRenderContext(exitOnCtrlC: boolean): {
  renderOptions: RenderOptions
  getFpsMetrics: () => FpsMetrics | undefined
  stats: StatsStore
}

export function completeOnboarding(): void
```
(Source: `src/interactiveHelpers.tsx:32-365`.)

### 3.3 `dialogLaunchers.tsx` — extracted dialog launchers

Eight launchers, each dynamically `import()`ing its component and
wiring a `done`-style callback via `showSetupDialog` /
`renderAndRun`. Contract preserved verbatim from inline call sites in
`main.tsx` (the file's docstring, `src/dialogLaunchers.tsx:1-8`,
states "Zero behavior change"):

| Launcher | Component | Result |
|---|---|---|
| `launchSnapshotUpdateDialog` (`:29`) | `agents/SnapshotUpdateDialog` | `'merge' \| 'keep' \| 'replace'` (cancel → `'keep'`) |
| `launchInvalidSettingsDialog` (`:44`) | `InvalidSettingsDialog` | `void` |
| `launchAssistantSessionChooser` (`:58`) | `assistant/AssistantSessionChooser` | `string \| null` |
| `launchAssistantInstallWizard` (`:73`) | `commands/assistant/assistant.NewInstallWizard` | `string \| null`; rejects on installer error via `Promise.race` |
| `launchTeleportResumeWrapper` (`:91`) | `TeleportResumeWrapper` (`source="cliArg"`) | `TeleportRemoteResponse \| null` |
| `launchTeleportRepoMismatchDialog` (`:102`) | `TeleportRepoMismatchDialog` | `string \| null` |
| `launchResumeChooser` (`:117`) | `screens/ResumeConversation` (wrapped in `<App><KeybindingSetup>`) | `void` (uses `renderAndRun`, parallel `Promise.all` with `getWorktreePaths`) |

`replLauncher.launchRepl(root, appProps, replProps, renderAndRun)`
dynamically imports `screens/REPL` and `components/App`, then renders
`<App {...appProps}><REPL {...replProps} /></App>` via the
caller-supplied `renderAndRun` (`src/replLauncher.tsx:12-22`).

### 3.4 ThemeProvider contract (`design-system/ThemeProvider.tsx`)

```ts
type ThemeContextValue = {
  themeSetting: ThemeSetting        // saved (may be 'auto')
  setThemeSetting: (s: ThemeSetting) => void
  setPreviewTheme: (s: ThemeSetting) => void
  savePreview: () => void
  cancelPreview: () => void
  currentTheme: ThemeName            // resolved (never 'auto')
}
const DEFAULT_THEME: ThemeName = 'dark'           // L20
export function ThemeProvider({children, initialState, onThemeSave})
export function useTheme(): [ThemeName, (s: ThemeSetting) => void]
export function useThemeSetting(): ThemeSetting
export function usePreviewTheme(): {
  setPreviewTheme; savePreview; cancelPreview }
```

Resolution rule (`:81`):
`currentTheme = activeSetting === 'auto' ? systemTheme : activeSetting`
where `activeSetting = previewTheme ?? themeSetting`.

`feature('AUTO_THEME')` gates dynamic import of
`utils/systemThemeWatcher.js` and the OSC 11 watcher install
(`ThemeProvider.tsx:64-80`); `systemTheme` is seeded by
`getSystemThemeName()` at provider creation, then corrected on first
poll. Switching to `'auto'` re-seeds from the cache so the OSC
round-trip "doesn't flash the wrong palette" (`:90-92`).

### 3.5 `color()` resolver (`design-system/color.ts`)

```ts
export function color(
  c: keyof Theme | Color | undefined,
  theme: ThemeName,
  type: ColorType = 'foreground'
): (text: string) => string
```
Bypasses theme lookup if `c` starts with `rgb(`, `#`, `ansi256(`, or
`ansi:`; otherwise resolves through `getTheme(theme)[c]`
(`color.ts:9-30`).

---

## 4. Data Model & State

### 4.1 `Theme` shape (`utils/theme.ts:4-89`)

`Theme` is a flat string-valued record with these keys (verbatim,
order preserved):

```
autoAccept, bashBorder, claude, claudeShimmer,
claudeBlue_FOR_SYSTEM_SPINNER, claudeBlueShimmer_FOR_SYSTEM_SPINNER,
permission, permissionShimmer, planMode, ide,
promptBorder, promptBorderShimmer, text, inverseText,
inactive, inactiveShimmer, subtle, suggestion, remember,
background, success, error, warning, merged, warningShimmer,
diffAdded, diffRemoved, diffAddedDimmed, diffRemovedDimmed,
diffAddedWord, diffRemovedWord,
red_FOR_SUBAGENTS_ONLY, blue_FOR_SUBAGENTS_ONLY,
green_FOR_SUBAGENTS_ONLY, yellow_FOR_SUBAGENTS_ONLY,
purple_FOR_SUBAGENTS_ONLY, orange_FOR_SUBAGENTS_ONLY,
pink_FOR_SUBAGENTS_ONLY, cyan_FOR_SUBAGENTS_ONLY,
professionalBlue, chromeYellow,
clawd_body, clawd_background,
userMessageBackground, userMessageBackgroundHover,
messageActionsBackground, selectionBg,
bashMessageBackgroundColor, memoryBackgroundColor,
rate_limit_fill, rate_limit_empty, fastMode, fastModeShimmer,
briefLabelYou, briefLabelClaude,
rainbow_red, rainbow_orange, rainbow_yellow, rainbow_green,
rainbow_blue, rainbow_indigo, rainbow_violet,
rainbow_red_shimmer, rainbow_orange_shimmer, rainbow_yellow_shimmer,
rainbow_green_shimmer, rainbow_blue_shimmer, rainbow_indigo_shimmer,
rainbow_violet_shimmer
```

`THEME_NAMES = ['dark','light','light-daltonized','dark-daltonized',
'light-ansi','dark-ansi'] as const` (`:91-98`).
`THEME_SETTINGS = ['auto', ...THEME_NAMES] as const` (`:103`).

### 4.2 Render lifecycle (high level)

```
boot (interactiveHelpers.getRenderContext)
  ↓
fpsTracker = new FpsTracker()
stats = createStatsStore()                       (interactiveHelpers.tsx:311-313)
  ↓
RenderOptions = baseRenderOptions ∪ {
  onFrame: (event) => {
    fpsTracker.record(event.durationMs)
    stats.observe('frame_duration_ms', event.durationMs)
    if (CLAUDE_CODE_FRAME_TIMING_LOG) appendFileSync(JSONL { total,
       ...phases, rss, cpu })
    if (isSynchronizedOutputSupported()) return        // DEC 2026
    for flicker in event.flickers (≠ 'resize'):
      if Date.now() - lastFlickerTime < 1000:
        logEvent('tengu_flicker', { desiredHeight, actualHeight, reason })
      lastFlickerTime = now
  }
}                                              (interactiveHelpers.tsx:319-363)
  ↓
ink.createRoot(options)  (ink.ts wraps with ThemeProvider)
  ↓
showSetupScreens(...)    → onboarding / trust / mcp.json approval /
                            external-includes / Grove / approve-api-key /
                            bypass-mode / auto-mode opt-in /
                            dev-channels / claude-in-chrome onboarding
  ↓
renderAndRun(root, <App><REPL .../></App>) {
  root.render(element)
  startDeferredPrefetches()                    (interactiveHelpers.tsx:99-103)
  await root.waitUntilExit()
  await gracefulShutdown(0)
}
```

`patchConsole` (default `true`, `ink/ink.tsx:182-183` + `:1571,:1594`)
hijacks `console.*` so third-party logs do not corrupt the alt-screen
buffer. Errors after Ink mount must use `exitWithError`/`exitWithMessage`
which render through the React tree before unmount
(`interactiveHelpers.tsx:46-80`).

### 4.3 Spinner state

`SpinnerGlyph` props (`Spinner/SpinnerGlyph.tsx:15-21`):

```ts
type Props = { frame: number; messageColor: keyof Theme;
  stalledIntensity?: number;     // 0..1
  reducedMotion?: boolean;
  time?: number }                // ms, only for reducedMotion
```

Frame buffer construction (`SpinnerGlyph.tsx:6-7`):

```ts
const DEFAULT_CHARACTERS = getDefaultCharacters()
const SPINNER_FRAMES = [...DEFAULT_CHARACTERS,
                        ...[...DEFAULT_CHARACTERS].reverse()]
```

State for stalled animation ramps in `useStalledAnimation`; shimmer
modulation in `useShimmerAnimation`; reduced-motion is a 2-second
duty cycle (1s on, 1s dim) of a single `●` glyph
(`SpinnerGlyph.tsx:8-9`).

---

## 5. Algorithm / Control Flow

### 5.1 `showSetupScreens` (`interactiveHelpers.tsx:104-298`)

Pseudocode (decision-preserving, branch order verbatim):

```
if production==='test' || isEnvTruthy(false) || IS_DEMO: return false
config = getGlobalConfig()
onboardingShown = false
if !config.theme || !config.hasCompletedOnboarding:
  onboardingShown = true
  showSetupDialog(<Onboarding onDone={()=>{completeOnboarding();done()}}>,
                   { onChangeAppState })
if !isEnvTruthy(CLAUBBIT):
  if !checkHasTrustDialogAccepted():
    showSetupDialog(<TrustDialog commands onDone={done}/>)
  setSessionTrustAccepted(true)
  resetGrowthBook(); void initializeGrowthBook()
  void getSystemContext()
  if getSettingsWithAllErrors().errors.length === 0:
    await handleMcpjsonServerApprovals(root)
  if await shouldShowClaudeMdExternalIncludesWarning():
    showSetupDialog(<ClaudeMdExternalIncludesDialog isStandaloneDialog
                     externalIncludes onDone/>)
void updateGithubRepoPathMapping()
if feature('LODESTONE'): updateDeepLinkTerminalPreference()
applyConfigEnvironmentVariables()
setImmediate(() => initializeTelemetryAfterTrust())
if await isQualifiedForGrove():
  decision = await showSetupDialog(<GroveDialog showIfAlreadyViewed=false
              location={onboardingShown?'onboarding':'policy_update_modal'}
              onDone={done}/>)
  if decision === 'escape':
    logEvent('tengu_grove_policy_exited',{}); gracefulShutdownSync(0); return false
if process.env.ANTHROPIC_API_KEY && !isRunningOnHomespace():
  customApiKeyTruncated = normalizeApiKeyForConfig(env.ANTHROPIC_API_KEY)
  if getCustomApiKeyStatus(...) === 'new':
    showSetupDialog(<ApproveApiKey ...onDone>, { onChangeAppState })
if (permissionMode==='bypassPermissions' || allowDangerouslySkipPermissions)
   && !hasSkipDangerousModePermissionPrompt():
  showSetupDialog(<BypassPermissionsModeDialog onAccept={done}/>)
if feature('TRANSCRIPT_CLASSIFIER'):
  if permissionMode==='auto' && !hasAutoModeOptIn():
    showSetupDialog(<AutoModeOptInDialog onAccept onDecline=
                     {()=>gracefulShutdownSync(1)} declineExits/>)
if feature('KAIROS') || feature('KAIROS_CHANNELS'):
  if getAllowedChannels().length > 0 || (devChannels?.length ?? 0) > 0:
    await checkGate_CACHED_OR_BLOCKING('tengu_harbor')
  if devChannels && devChannels.length > 0:
    [{isChannelsEnabled},{getClaudeAIOAuthTokens}] = await Promise.all([
      import('./services/mcp/channelAllowlist.js'),
      import('./utils/auth.js')])
    if !isChannelsEnabled() || !getClaudeAIOAuthTokens()?.accessToken:
      setAllowedChannels([...current, ...devChannels.map(c=>({...c,dev:true}))])
      setHasDevChannels(true)
    else:
      showSetupDialog(<DevChannelsDialog channels onAccept=
        { ()=>{ setAllowedChannels([...]); setHasDevChannels(true); done() } }/>)
if claudeInChrome && !getGlobalConfig().hasCompletedClaudeInChromeOnboarding:
  showSetupDialog(<ClaudeInChromeOnboarding onDone={done}/>)
return onboardingShown
```

### 5.2 Permission UI dispatch (`PermissionRequest.permissionComponentForTool`)

```
switch (tool):
  FileEditTool        → FileEditPermissionRequest
  FileWriteTool       → FileWritePermissionRequest
  BashTool            → BashPermissionRequest
  PowerShellTool      → PowerShellPermissionRequest
  ReviewArtifactTool  → ReviewArtifactPermissionRequest ?? Fallback
                        (gated by feature('REVIEW_ARTIFACT'))
  WebFetchTool        → WebFetchPermissionRequest
  NotebookEditTool    → NotebookEditPermissionRequest
  ExitPlanModeV2Tool  → ExitPlanModePermissionRequest
  EnterPlanModeTool   → EnterPlanModePermissionRequest
  SkillTool           → SkillPermissionRequest
  AskUserQuestionTool → AskUserQuestionPermissionRequest
  WorkflowTool        → WorkflowPermissionRequest ?? Fallback
                        (gated by feature('WORKFLOW_SCRIPTS'))
  MonitorTool         → MonitorPermissionRequest ?? Fallback
                        (gated by feature('MONITOR_TOOL'))
  GlobTool, GrepTool, FileReadTool → FilesystemPermissionRequest
  default             → FallbackPermissionRequest
```
(Source `PermissionRequest.tsx:47-82`.)

`PermissionRequest` renders `<PermissionComponent>` with props
`{toolUseContext, toolUseConfirm, onDone, onReject, verbose,
workerBadge, setStickyFooter}` and surrounds with
`useKeybinding('app:interrupt', cancelHandler, {context:
'Confirmation'})` and `useNotifyAfterTimeout(notificationMessage,
'permission_prompt')` (`PermissionRequest.tsx:146-216`).

`getNotificationMessage(toolUseConfirm)` (verbatim,
`PermissionRequest.tsx:128-143`):
- `tool === ExitPlanModeV2Tool` → `"Claude Code needs your approval for the plan"`
- `tool === EnterPlanModeTool` → `"Claude Code wants to enter plan mode"`
- `feature('REVIEW_ARTIFACT') && tool === ReviewArtifactTool` →
  `"Claude needs your approval for a review artifact"`
- `!toolName || toolName.trim() === ''` → `"Claude Code needs your attention"`
- otherwise → `` `Claude needs your permission to use ${toolName}` ``

### 5.3 `PermissionPrompt` flow (`PermissionPrompt.tsx`)

```
question default = "Do you want to proceed?"             (:54)
DEFAULT_PLACEHOLDERS = {
  accept: 'tell Claude what to do next',
  reject: 'tell Claude what to do differently'
}                                                          (:30-33)
focusedOption = options.find(o => o.value === focusedValue)
showTabHint = (focusedFeedbackType === 'accept' && !acceptInputMode)
            || (focusedFeedbackType === 'reject' && !rejectInputMode)
selectOptions = options.map(opt =>
  feedbackConfig
    ? (isInputMode → { type:'input', label, value,
         placeholder ?? DEFAULT_PLACEHOLDERS[type], onChange, allowEmptySubmitToCancel:true }
                    : { label, value })
    : { label, value })
on Tab toggle: log 'tengu_(accept|reject)_feedback_mode_(entered|collapsed)'
on submit: feedback = trimmed(rawFeedback) || undefined
           log analytics with has_instructions, instructions_length
```

### 5.4 ThemeProvider auto-watch

```
useEffect on [activeSetting, internal_querier]:
  if !feature('AUTO_THEME'): return                       (:65)
  if activeSetting !== 'auto' || !internal_querier: return
  cleanup, cancelled = undefined, false
  void import('utils/systemThemeWatcher').then(({watchSystemTheme}) => {
    if cancelled: return
    cleanup = watchSystemTheme(internal_querier, setSystemTheme)
  })
  return () => { cancelled = true; cleanup?.() }
```

### 5.5 Frame timing → flicker analytics

`onFrame` is called per render frame; flicker reasons that are not
`'resize'` are logged at most once per second
(`interactiveHelpers.tsx:325-362`):

```
if isSynchronizedOutputSupported(): skip flicker analytics  // DEC 2026 BSU/ESU
for flicker in event.flickers:
  if flicker.reason === 'resize': continue
  now = Date.now()
  if now - lastFlickerTime < 1000:
    logEvent('tengu_flicker', { desiredHeight, actualHeight, reason })
  lastFlickerTime = now
```

### 5.6 `STREAMLINED_OUTPUT` print-mode delta (`cli/print.ts:857`)

`feature('STREAMLINED_OUTPUT')` toggles a CLI print-output variant
when running headless (`-p` / `--print`); detail of the output shape
is owned by spec 38. This spec only records the flag site.

---

## 6. Verbatim Assets

### 6.1 Theme palette — `dark` (default; `theme.ts:440-515`)

```
autoAccept                          rgb(175,135,255)
bashBorder                          rgb(253,93,177)
claude                              rgb(215,119,87)
claudeShimmer                       rgb(235,159,127)
claudeBlue_FOR_SYSTEM_SPINNER       rgb(147,165,255)
claudeBlueShimmer_FOR_SYSTEM_SPINNER rgb(177,195,255)
permission                          rgb(177,185,249)
permissionShimmer                   rgb(207,215,255)
planMode                            rgb(72,150,140)
ide                                 rgb(71,130,200)
promptBorder                        rgb(136,136,136)
promptBorderShimmer                 rgb(166,166,166)
text                                rgb(255,255,255)
inverseText                         rgb(0,0,0)
inactive                            rgb(153,153,153)
inactiveShimmer                     rgb(193,193,193)
subtle                              rgb(80,80,80)
suggestion                          rgb(177,185,249)
remember                            rgb(177,185,249)
background                          rgb(0,204,204)
success                             rgb(78,186,101)
error                               rgb(255,107,128)
warning                             rgb(255,193,7)
merged                              rgb(175,135,255)
warningShimmer                      rgb(255,223,57)
diffAdded                           rgb(34,92,43)
diffRemoved                         rgb(122,41,54)
diffAddedDimmed                     rgb(71,88,74)
diffRemovedDimmed                   rgb(105,72,77)
diffAddedWord                       rgb(56,166,96)
diffRemovedWord                     rgb(179,89,107)
red_FOR_SUBAGENTS_ONLY              rgb(220,38,38)
blue_FOR_SUBAGENTS_ONLY             rgb(37,99,235)
green_FOR_SUBAGENTS_ONLY            rgb(22,163,74)
yellow_FOR_SUBAGENTS_ONLY           rgb(202,138,4)
purple_FOR_SUBAGENTS_ONLY           rgb(147,51,234)
orange_FOR_SUBAGENTS_ONLY           rgb(234,88,12)
pink_FOR_SUBAGENTS_ONLY             rgb(219,39,119)
cyan_FOR_SUBAGENTS_ONLY             rgb(8,145,178)
professionalBlue                    rgb(106,155,204)
chromeYellow                        rgb(251,188,4)
clawd_body                          rgb(215,119,87)
clawd_background                    rgb(0,0,0)
userMessageBackground               rgb(55, 55, 55)
userMessageBackgroundHover          rgb(70, 70, 70)
messageActionsBackground            rgb(44, 50, 62)
selectionBg                         rgb(38, 79, 120)
bashMessageBackgroundColor          rgb(65, 60, 65)
memoryBackgroundColor               rgb(55, 65, 70)
rate_limit_fill                     rgb(177,185,249)
rate_limit_empty                    rgb(80,83,112)
fastMode                            rgb(255,120,20)
fastModeShimmer                     rgb(255,165,70)
briefLabelYou                       rgb(122,180,232)
briefLabelClaude                    rgb(215,119,87)
rainbow_red                         rgb(235,95,87)
rainbow_orange                      rgb(245,139,87)
rainbow_yellow                      rgb(250,195,95)
rainbow_green                       rgb(145,200,130)
rainbow_blue                        rgb(130,170,220)
rainbow_indigo                      rgb(155,130,200)
rainbow_violet                      rgb(200,130,180)
rainbow_red_shimmer                 rgb(250,155,147)
rainbow_orange_shimmer              rgb(255,185,137)
rainbow_yellow_shimmer              rgb(255,225,155)
rainbow_green_shimmer               rgb(185,230,180)
rainbow_blue_shimmer                rgb(180,205,240)
rainbow_indigo_shimmer              rgb(195,180,230)
rainbow_violet_shimmer              rgb(230,180,210)
```

`light` differs only in tonal direction (text=`rgb(0,0,0)`,
inverseText=`rgb(255,255,255)`, subtle=`rgb(175,175,175)`,
inactive=`rgb(102,102,102)`, plus muted RGBs); full palette at
`theme.ts:115-191`. `dark-daltonized` at `:521-596`. `light-ansi`,
`dark-ansi`, `light-daltonized` use `'ansi:<name>'` strings (theme
keys preserved); cited at `:197-279`, `:278-358`, `:359-439`.

`getTheme(themeName)` switch (`:598-613`) returns `lightTheme`,
`lightAnsiTheme`, `darkAnsiTheme`, `lightDaltonizedTheme`,
`darkDaltonizedTheme`, **default `darkTheme`**.

### 6.2 Spinner glyph sequences (`Spinner/utils.ts:4-11`)

```ts
export function getDefaultCharacters(): string[] {
  if (process.env.TERM === 'xterm-ghostty') {
    return ['·', '✢', '✳', '✶', '✻', '*']
  }
  return process.platform === 'darwin'
    ? ['·', '✢', '✳', '✶', '✻', '✽']
    : ['·', '✢', '*', '✶', '✻', '✽']
}
```

`SPINNER_FRAMES = [...DEFAULT, ...[...DEFAULT].reverse()]` — yields
the 12-frame palindrome cycle (`SpinnerGlyph.tsx:7`).
`REDUCED_MOTION_DOT = '●'` (U+25CF), `REDUCED_MOTION_CYCLE_MS = 2000`,
`ERROR_RED = { r: 171, g: 43, b: 63 }` (`SpinnerGlyph.tsx:8-14`).

Reduced-motion dim flip:
`isDim = Math.floor(time / (REDUCED_MOTION_CYCLE_MS / 2)) % 2 === 1`
(`:37`).

### 6.3 Trust dialog strings (`TrustDialog/TrustDialog.tsx`)

Title (`:257`): `"Accessing workspace:"` (rendered by
`PermissionDialog title="Accessing workspace:" color="warning"
titleColor="warning"`).

Body lines (rendered as separate `<Text>` inside a
`flexDirection="column" gap={1} paddingTop={1}` box; `:206-217`):

1. `<Text bold={true}>{getFsImplementation().cwd()}</Text>`  *(verbatim cwd)*
2. `<Text>Quick safety check: Is this a project you created or one you trust? (Like your own code, a well-known open source project, or work from your team). If not, take a moment to review what's in this folder first.</Text>`
3. `<Text>Claude Code'll be able to read, edit, and execute files here.</Text>`
4. `<Text dimColor={true}><Link url="https://code.claude.com/docs/en/security">Security guide</Link></Text>`

Select options (`:227-233`):

```
[ { label: "Yes, I trust this folder", value: "enable_all" },
  { label: "No, exit",                 value: "exit"        } ]
```

Footer hint (`:248`): `pending` →
`<>Press {exitState.keyName} again to exit</>`; otherwise
`<>Enter to confirm · Esc to cancel</>`.

Telemetry events (`:134-144,163-173`): `tengu_trust_dialog_shown`,
`tengu_trust_dialog_accept` with metadata `{isHomeDir, hasMcpServers,
hasHooks, hasBashExecution, hasApiKeyHelper, hasAwsCommands,
hasGcpCommands, hasOtelHeadersHelper, hasDangerousEnvVars}`.

Persistence (`:174-178`): if `homedir() === getCwd()`,
`setSessionTrustAccepted(true)` (no on-disk write); else
`saveCurrentProjectConfig(c => ({...c, hasTrustDialogAccepted: true}))`.

### 6.4 MCP-server-approval dialog strings
(`MCPServerApprovalDialog.tsx`, `MCPServerDialogCopy.tsx`)

Title (`MCPServerApprovalDialog.tsx:63`):
```
`New MCP server found in .mcp.json: ${serverName}`
```
Color: `"warning"`.

Body (`MCPServerDialogCopy.tsx:8`, single `<Text>` block):
```
MCP servers may execute code or access system resources. All tool calls
require approval. Learn more in the <Link
url="https://code.claude.com/docs/en/mcp">MCP documentation</Link>.
```

Select options (`:81-90`):
```
[ { label: "Use this and all future MCP servers in this project",
    value: "yes_all" },
  { label: "Use this MCP server",                   value: "yes" },
  { label: "Continue without using this MCP server", value: "no"  } ]
```

Cancel handler equivalent to selecting `"no"` (`:97`). Telemetry:
`logEvent('tengu_mcp_dialog_choice', { choice: value })`.

Persistence:
- `yes` / `yes_all` → append to `enabledMcpjsonServers` in
  `localSettings`; if `yes_all` also set
  `enableAllProjectMcpServers: true`.
- `no` → append to `disabledMcpjsonServers` in `localSettings`.

### 6.5 Permission dialog chrome (`PermissionDialog.tsx`)

Component skeleton (verbatim `:62`):

```tsx
<Box flexDirection="column" borderStyle="round" borderColor={color}
     borderLeft={false} borderRight={false} borderBottom={false}
     marginTop={1}>
  <Box paddingX={1} flexDirection="column">
    <Box justifyContent="space-between">
      <PermissionRequestTitle title={title} subtitle={subtitle}
        color={titleColor} workerBadge={workerBadge}/>
      {titleRight}
    </Box>
  </Box>
  <Box flexDirection="column" paddingX={innerPaddingX}>{children}</Box>
</Box>
```

Defaults: `color = 'permission'`, `innerPaddingX = 1` (`:29-30`).

### 6.6 PermissionPrompt strings (`PermissionPrompt.tsx`)

Question default (`:54`): `"Do you want to proceed?"`

Feedback placeholders (`:30-33`):

```ts
const DEFAULT_PLACEHOLDERS: Record<FeedbackType, string> = {
  accept: 'tell Claude what to do next',
  reject: 'tell Claude what to do differently',
}
```

Analytics events: `tengu_accept_feedback_mode_entered`,
`tengu_accept_feedback_mode_collapsed`,
`tengu_reject_feedback_mode_entered`,
`tengu_reject_feedback_mode_collapsed`
(`PermissionPrompt.tsx:153-167`).

### 6.7 FallbackPermissionRequest (`FallbackPermissionRequest.tsx`)

Title: `"Tool use"` (`:323`).

Options sequence built as
`[ Yes, optional Yes-don't-ask-again, No ]` (`:158-208`):

```ts
{ label: "Yes",  value: "yes",  feedbackConfig: { type: "accept" } }
// only when shouldShowAlwaysAllowOptions():
{ label: <Text>Yes, and don't ask again for {<Text bold>{userFacingName}</Text>}{" "}commands in {<Text bold>{originalCwd}</Text>}</Text>,
  value: "yes-dont-ask-again" }
{ label: "No",   value: "no",   feedbackConfig: { type: "reject" } }
```

Tool description rendering (`:269-292`): `truncateToLines(description, 3)`
inside a dim Text wrapper, padded `paddingX={2} paddingY={1}`. MCP
suffix: `originalUserFacingName.endsWith(" (MCP)") ? <Text dimColor>
" (MCP)"</Text> : ""`.

### 6.8 PromptInputFooter layout (`PromptInputFooter.tsx`)

Constants (file-local) — see §6.9.

Narrow threshold: `isNarrow = columns < 80` (`:105`).
`isShort = isFullscreen && rows < 24` (`:110`).

Top-level row (paddings `paddingX={2}`, gap `1` else `0`):

```tsx
<Box flexDirection={isNarrow ? 'column' : 'row'}
     justifyContent={isNarrow ? 'flex-start' : 'space-between'}
     paddingX={2} gap={isNarrow ? 0 : 1}>
  <Box flexDirection="column" flexShrink={isNarrow ? 0 : 1}>
    {mode==='prompt' && !isShort && !exitMessage.show && !isPasting
       && statusLineShouldDisplay(settings) &&
        <StatusLine messagesRef={messagesRef}
                    lastAssistantMessageId={lastAssistantMessageId}
                    vimMode={vimMode}/> }
    <PromptInputFooterLeftSide ... />
  </Box>
  <Box flexShrink={1} gap={1}>
    {isFullscreen ? null : <Notifications .../>}
    {"external" === 'ant' && isUndercover() && <Text dimColor>undercover</Text>}
    <BridgeStatusIndicator bridgeSelected={bridgeSelected}/>
  </Box>
</Box>
{"external" === 'ant' && <CoordinatorTaskPanel/>}
```

Suggestions branch (non-fullscreen): renders
`<PromptInputFooterSuggestions>` inside `paddingX={2} paddingY={0}`.
Help-open branch: renders `<PromptInputHelpMenu dimColor fixedWidth
paddingX={2}/>`. Bridge pill text:

```tsx
<Text color={bridgeSelected ? 'background' : status.color}
      inverse={bridgeSelected} wrap="truncate">
  {status.label}
  {bridgeSelected && <Text dimColor> · Enter to view</Text>}
</Text>
```

Implicit (config-driven) bridge sessions hide unless
`status.label === 'Remote Control reconnecting'`
(`PromptInputFooter.tsx:182-185`).

`PromptInputFooterLeftSide` keyboard hints (verbatim string fragments
from `PromptInputFooterLeftSide.tsx:452,507`):

- `<KeyboardShortcutHint shortcut="Enter" action="view tasks"/>` /
  `<KeyboardShortcutHint shortcut="↓" action="manage"/>`
- `<KeyboardShortcutHint shortcut={escShortcut} action="interrupt"/>`

Suppress-hint rule (`PromptInputFooter.tsx:121-122`):
`suppressHint = suppressHintFromProps || statusLineShouldDisplay(settings) || isSearching`.

### 6.9 Constants table

| Name | Value | Source |
|---|---|---|
| `FRAME_INTERVAL_MS` | `16` | `ink/constants.ts:2` |
| frame onFrame quarter-interval | `~250 fps setTimeout floor` (comment) | `ink/ink.tsx:755` |
| `RECENT_SCROLL_REPIN_WINDOW_MS` | `3000` | `screens/REPL.tsx:305` |
| `TITLE_ANIMATION_INTERVAL_MS` | `960` | `screens/REPL.tsx:475,501` |
| `PROMPT_SUPPRESSION_MS` | `1500` | `screens/REPL.tsx:979` |
| reconciler yield priority | `5ms` (comment) | `screens/REPL.tsx:1316` |
| `PROMPT_FOOTER_LINES` | `5` | `PromptInput/PromptInput.tsx:192` |
| `MIN_INPUT_VIEWPORT_LINES` | `3` | `PromptInput/PromptInput.tsx:193` |
| `DEFAULT_THEME` (renderer fallback) | `'dark'` | `design-system/ThemeProvider.tsx:20` |
| `THEME_NAMES` | `['dark','light','light-daltonized','dark-daltonized','light-ansi','dark-ansi']` | `utils/theme.ts:91-98` |
| `THEME_SETTINGS` | `['auto', ...THEME_NAMES]` | `utils/theme.ts:103` |
| `REDUCED_MOTION_DOT` | `'●'` | `Spinner/SpinnerGlyph.tsx:8` |
| `REDUCED_MOTION_CYCLE_MS` | `2000` | `Spinner/SpinnerGlyph.tsx:9` |
| `ERROR_RED` (spinner stalled-end) | `{r:171,g:43,b:63}` | `Spinner/SpinnerGlyph.tsx:10-14` |
| `DEFAULT_PLACEHOLDERS` | `{accept:'tell Claude what to do next', reject:'tell Claude what to do differently'}` | `permissions/PermissionPrompt.tsx:30-33` |
| flicker dedupe window | `1000ms` | `interactiveHelpers.tsx:353` |
| isNarrow column threshold | `< 80` | `PromptInputFooter.tsx:105` |
| isShort row threshold | fullscreen `&& rows < 24` | `PromptInputFooter.tsx:110` |
| chalk `level` for Apple Terminal asciichart | `2` (256 color) | `utils/theme.ts:617-620` |
| themeColorToAnsi fallback | `'\x1b[35m'` (magenta) | `utils/theme.ts:638` |

### 6.10 Critical regex

`parseRGB` (`Spinner/utils.ts:74`):
```
/rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)/
```
`themeColorToAnsi` (`utils/theme.ts:627`):
```
/rgb\(\s?(\d+),\s?(\d+),\s?(\d+)\s?\)/
```
`color()` raw-value bypass (`design-system/color.ts:19-25`): prefix
test against `'rgb('`, `'#'`, `'ansi256('`, `'ansi:'`.

---

## 7. Side Effects & I/O

- **stdin/stdout/stderr**: `RenderOptions.stdin/stdout/stderr` from
  `getBaseRenderOptions(exitOnCtrlC)`. `patchConsole` (default `true`)
  redirects all `console.*` writes (`ink/ink.tsx:182`).
- **OSC sequences**: terminal-querier polls OSC 11 (background color) +
  OSC 4 to seed `systemTheme`; `feature('AUTO_THEME')` enables a
  watcher that reuses `internal_querier` from `useStdin()`.
  Terminal capability detection in `ink/terminal.ts`,
  `ink/termio/osc.ts` (ANT delta at `:468`).
- **DEC 2026 BSU/ESU**: `isSynchronizedOutputSupported()` short-circuits
  flicker telemetry when the terminal buffers redraw atomically
  (`interactiveHelpers.tsx:344-347`).
- **Filesystem**:
  - `getGlobalConfig()`/`saveGlobalConfig()` for theme + onboarding
    flags (`design-system/ThemeProvider.tsx:36-42`).
  - `saveCurrentProjectConfig({...,hasTrustDialogAccepted:true})` when
    cwd ≠ home (`TrustDialog.tsx:177`).
  - `updateSettingsForSource('localSettings', ...)` for
    enabled/disabled MCP server lists
    (`MCPServerApprovalDialog.tsx:31,36,48`).
  - `appendFileSync(CLAUDE_CODE_FRAME_TIMING_LOG, ...)` JSONL bench
    output (`interactiveHelpers.tsx:341`).
- **Process / shutdown**: `gracefulShutdown(0)` at end of
  `renderAndRun`; `gracefulShutdownSync(1)` on `Ctrl-C` exit + Trust
  dialog "exit" + `AutoModeOptInDialog` decline; `process.exit(exitCode)`
  inside `exitWithMessage` after Ink unmount.
- **Environment variables consumed**:
  `TERM` (`Spinner/utils.ts:5` → ghostty glyph variant);
  `process.platform` (`Spinner/utils.ts:8`);
  `IS_DEMO`, `CLAUBBIT`, `ANTHROPIC_API_KEY`,
  `CLAUDE_CODE_FRAME_TIMING_LOG` (`interactiveHelpers.tsx`);
  `COLORFGBG` (initial system-theme seed via `getSystemThemeName`).
- **Trust boundary**: `applyConfigEnvironmentVariables()` is called
  AFTER trust dialog acceptance (or in bypass mode). OTel telemetry
  initializer is `setImmediate`-scheduled so dynamic OTel imports
  resolve after first render (`interactiveHelpers.tsx:184-190`).
- **Module-import-time chalk-level mutation** (`ink/colorize.ts:21-26`,
  `:52-54`). On import, `colorize.ts` mutates the `chalk` singleton:
  - `boostChalkLevelForXtermJs()`: if `process.env.TERM_PROGRAM ===
    'vscode'` AND `chalk.level === 2`, sets `chalk.level = 3`. Rationale:
    code-server / Coder containers don't set `COLORTERM=truecolor`;
    chalk's supports-color falls through to 256-color and downgrades
    `rgb(215,119,87)` (Claude orange) to a washed salmon. Gated on
    `level === 2` (not `< 3`) to respect `NO_COLOR` / `FORCE_COLOR=0`
    (level 0).
  - `clampChalkLevelForTmux()`: if `process.env.TMUX` is set AND
    `chalk.level > 2` AND `CLAUDE_CODE_TMUX_TRUECOLOR` is not set,
    clamps `chalk.level = 2`. Rationale: tmux's truecolor passthrough
    requires `terminal-overrides ,*:Tc`; default tmux drops bg
    sequences. 256-color (`grey93`) renders identically.
  - **Order matters**: boost runs before clamp, so tmux-inside-VSCode
    correctly ends at level 2.
  - **Reimplementer hazard**: this is a global singleton mutation at
    module import time, computed once. `chalk` is shared across the
    entire app; this affects all fg/bg/hex output everywhere, not just
    Ink rendering. Skipping these patches produces visibly washed-out
    or black-on-dark output in code-server and tmux.

---

## 8. Feature Flags & Variants

| Flag / gate | Effect on Spec 37 surface | Site |
|---|---|---|
| `HISTORY_PICKER` | Replaces inline history-search overlay with a picker; gates `useKeybindings` activation matrix in `PromptInput`. | `PromptInput.tsx:1721,1727,2144`; `useHistorySearch.ts:240` |
| `HOOK_PROMPTS` | Wires REPL `requestPrompt` callback into `ToolUseContext`; PreToolUse hooks may render a permission dialog. Default off → tools skip programmatic permission prompting. | `screens/REPL.tsx:2520` |
| `MESSAGE_ACTIONS` | Adds per-message action keybindings + `MessageActionsKeybindings` overlay; only active in `isFullscreenEnvEnabled() && !disableMessageActions`. | `keybindings/defaultBindings.ts:88,268`; `screens/REPL.tsx:606,4562,4905` |
| `LODESTONE` | Calls `updateDeepLinkTerminalPreference()` post-trust; enables a deeplink protocol module + a settings key. | `interactiveHelpers.tsx:176`; `main.tsx:647,3781`; `utils/backgroundHousekeeping.ts:10,39`; `utils/settings/types.ts:808` |
| `STREAMLINED_OUTPUT` | Headless `-p` CLI prints in streamlined form. Behavior owner: spec 38. | `cli/print.ts:857` |
| `MCP_RICH_OUTPUT` | Enables image/audio/resource-link rendering in MCP tool UI. Owner: spec 16. Spec 37 owns the React tree only. | `tools/MCPTool/UI.tsx:51` |
| `AUTO_THEME` | Dynamic import of `systemThemeWatcher`, OSC-11 live-watch wired through `useStdin().internal_querier`. Off → 'auto' resolves once and never updates. | `design-system/ThemeProvider.tsx:65` |
| `BUDDY` | Companion sprite changes REPL bottom-row layout (`flexDirection: 'column'` when `companionNarrow`). | `screens/REPL.tsx:4590` |
| `TRANSCRIPT_CLASSIFIER` | Enables `auto` permission mode + `AutoModeOptInDialog`. | `interactiveHelpers.tsx:224-235` |
| `KAIROS` / `KAIROS_CHANNELS` | Enables `tengu_harbor` gate poll, `DevChannelsDialog`, channel-allowlist mutations. | `interactiveHelpers.tsx:241-288` |
| `BRIDGE_MODE` | Enables `BridgeStatusIndicator` + bridge-related state subscriptions in footer. Owner: spec 34. | `PromptInputFooter.tsx:160-189` |
| `VOICE_MODE` | Enables `VoiceIndicator` in `PromptInputFooterLeftSide`, `VoiceModeNotice` in `LogoV2/`, `insertTextRef` plumbing in REPL. Owner: spec 36. | `screens/REPL.tsx:4905`; `LogoV2/VoiceModeNotice.tsx`; `PromptInput/VoiceIndicator.tsx` |
| ANT (`USER_TYPE === 'ant'`) | Footer renders `<Text dimColor>undercover</Text>` when `isUndercover()`; ANT-only `CoordinatorTaskPanel`; `ink/termio/osc.ts:468` terminal-OSC variant. | `PromptInputFooter.tsx:146,150`; `interactiveHelpers.tsx` literal `"external" === 'ant'` post-DCE marker |
| `IS_DEMO`, `CLAUBBIT`, `production==='test'` | `showSetupScreens` returns early (`false`). | `interactiveHelpers.tsx:105-107,131` |

---

## 9. Error Handling & Edge Cases

- **Console-during-Ink corruption**: `patchConsole` redirects writes to
  the Ink log buffer; fatal errors must use `exitWithError` so the
  React tree renders the message before unmount
  (`interactiveHelpers.tsx:46-80`).
- **Async installer rejection**: `launchAssistantInstallWizard`
  attaches a `Promise.race` against an explicit `rejectWithError`
  channel so installer failures throw `Installation failed: ${msg}`
  instead of resolving to `null` (which means cancel)
  (`dialogLaunchers.tsx:73-85`).
- **Trust-already-accepted fast path**: if
  `checkHasTrustDialogAccepted()` returns true, `<TrustDialog>` is
  not even imported (skip dynamic import + render cycle); the dialog
  itself short-circuits on mount via `setTimeout(onDone)` if state
  is already set (`TrustDialog.tsx:199-201`).
- **Cold StatSig disk cache (`tengu_harbor`)**: a fresh install with
  channels passed via `--channels` may silently drop notifications;
  `interactiveHelpers.tsx:241-252` blocks on
  `checkGate_CACHED_OR_BLOCKING('tengu_harbor')` only when channels
  exist. Documented as gh#37026.
- **Settings invalid → no MCP approval**: `handleMcpjsonServerApprovals`
  is skipped when `getSettingsWithAllErrors().errors.length > 0`
  (`interactiveHelpers.tsx:159-161`).
- **Bridge "Failed" state**: surfaced via notification, not the footer
  pill (`PromptInputFooter.tsx:173`).
- **ANSI-theme RGB fallback**: `themeColorToAnsi` returns `'\x1b[35m'`
  (magenta) if the regex fails to match; Apple Terminal forced into
  256-color level 2 to avoid 24-bit corruption (`utils/theme.ts:617-638`).

---

## 10. Telemetry & Observability

Events emitted by Spec 37 surfaces:

| Event | Payload | Site |
|---|---|---|
| `tengu_trust_dialog_shown` | `{isHomeDir, hasMcpServers, hasHooks, hasBashExecution, hasApiKeyHelper, hasAwsCommands, hasGcpCommands, hasOtelHeadersHelper, hasDangerousEnvVars}` | `TrustDialog.tsx:134-144` |
| `tengu_trust_dialog_accept` | same shape | `TrustDialog.tsx:163-173` |
| `tengu_mcp_dialog_choice` | `{choice}` (`'yes' \| 'yes_all' \| 'no'`) | `MCPServerApprovalDialog.tsx:21-23` |
| `tengu_grove_policy_exited` | `{}` | `interactiveHelpers.tsx:197` |
| `tengu_stdin_interactive` | `{}` (when `baseOptions.stdin` set) | `interactiveHelpers.tsx:308` |
| `tengu_flicker` | `{desiredHeight, actualHeight, reason}` 1-second-debounced, skipped under DEC 2026 sync output | `interactiveHelpers.tsx:354-358` |
| `tengu_accept_feedback_mode_entered` / `_collapsed` | `{toolName, isMcp}` | `permissions/PermissionPrompt.tsx:153-158` |
| `tengu_reject_feedback_mode_entered` / `_collapsed` | same shape | `permissions/PermissionPrompt.tsx:163-167` |
| Permission accept (with feedback) | `{toolName, isMcp, has_instructions, instructions_length, ...}` | `permissions/PermissionPrompt.tsx:196-200` |

Stats: `frame_duration_ms` is observed on every frame; FPS metrics
exposed via `getFpsMetrics()` for the App context
(`interactiveHelpers.tsx:311-327`). Bench output: JSONL appended to
`CLAUDE_CODE_FRAME_TIMING_LOG` if set.

---

## 10.5. React Compiler Runtime (build-time memo cache)

UI-shell `.tsx` source files are pre-compiled by the React Compiler.
The compiler injects a memoization cache via the symbol `_c` imported
from `react/compiler-runtime`, then rewrites each component/hook to
allocate a per-render slot array (`_c(N)`) and gate prop/JSX recomputes
behind cache slots. Confirmed sites:

- `src/components/Spinner.tsx:1` — `import { c as _c } from "react/compiler-runtime";`
- `src/screens/REPL.tsx:1` — same import.
- `src/hooks/useTypeahead.tsx` — `useMemoCache`/`_c(N)` invocations
  in the compiled output (input-side hooks the compiler memoizes
  aggressively due to large render bodies).

Implications for reimplementers:

- The compiler is part of the **build contract**, not just an
  optimization. Hand-written equivalents that drop `_c` slots will
  re-render at every parent tick — Spinner glyph timing, REPL
  fullscreen layout math, and typeahead candidate filtering all rely
  on compiler-inserted memo slots to stay within Ink's frame budget.
- The `// biome-ignore-all assist/source/organizeImports: ANT-ONLY
  import markers must not be reordered` header sits directly under
  the `_c` import in compiled files; reordering imports breaks both
  the compiler-runtime contract and the ANT bundle-strip pattern (§8).
- `react/compiler-runtime` is not a runtime dependency the consumer
  picks: it's bundled with React 19+. A reimplementation targeting
  React 18 cannot use the same emitted form and must hand-memo via
  `useMemo` / `React.memo` / `useCallback` at every site.
- Companion catalogs (37a / 37b) enumerate per-file `_c(N)` slot
  counts where they diverge from compiler defaults.

---

## 11. Reimplementation Checklist

- [ ] `ink.ts` re-exports the exact set in §3.1; `render`/`createRoot`
      wrap children in `<ThemeProvider>` so consumers do not need to.
- [ ] `ThemeProvider` defaults the resolved theme to `'dark'` when no
      provider is mounted (tests, tooling).
- [ ] `getTheme(themeName)` returns `dark` for any unknown name.
- [ ] Theme keys are the exact 67-key set in §4.1; key names like
      `claudeBlue_FOR_SYSTEM_SPINNER`, `red_FOR_SUBAGENTS_ONLY`, and
      `clawd_body` are part of the contract — renames break consumers.
- [ ] `THEME_SETTINGS` includes `'auto'`, `THEME_NAMES` does not.
- [ ] `feature('AUTO_THEME')` gates dynamic import of
      `utils/systemThemeWatcher`; without it, `'auto'` is resolved
      once at provider creation.
- [ ] Spinner glyph sequence varies by `TERM` (Ghostty) and platform
      (Darwin vs other) per §6.2; frame buffer is the
      `[forwards, ...reversed]` palindrome.
- [ ] Reduced motion uses `'●'` with a 2000ms duty cycle (1s on, 1s
      dim) and `Math.floor(time/1000) % 2 === 1` for the dim flip.
- [ ] Stalled spinner interpolates from theme color to `rgb(171,43,63)`
      using `interpolateColor`; ANSI fallback uses `'error'` keyword
      when `stalledIntensity > 0.5`, else `messageColor`.
- [ ] `showSetupScreens` evaluates onboarding → trust → MCP approvals
      → CLAUDE.md external includes → repo-path mapping → Lodestone
      deep-link → applyConfigEnvironmentVariables → setImmediate(OTel
      init) → Grove policy → custom-API-key approval → bypass-mode
      dialog → auto-mode opt-in → Kairos channels → Claude-in-Chrome,
      in that order.
- [ ] Trust dialog persists via `setSessionTrustAccepted(true)` for
      `homedir() === cwd()` and `saveCurrentProjectConfig` otherwise.
- [ ] Trust dialog body, options, hint, MCP-approval body and options,
      and PermissionPrompt placeholders are bit-exact from §6.3, §6.4,
      §6.6.
- [ ] `PermissionDialog` chrome: `borderStyle="round"`, top-only
      border (`borderLeft/Right/Bottom={false}`), `marginTop={1}`,
      title row `paddingX={1} flexDirection="column"` with
      `justifyContent="space-between"` row containing
      `<PermissionRequestTitle>` + optional `titleRight`, then the
      `flexDirection="column" paddingX={innerPaddingX}` body.
- [ ] `PermissionRequest.permissionComponentForTool` switch order is a
      contract; default is `FallbackPermissionRequest`.
- [ ] `getNotificationMessage` notification strings are bit-exact
      (§5.2).
- [ ] `STREAMLINED_OUTPUT` is consumed at `cli/print.ts:857`; spec 38
      owns the print delta.
- [ ] `HOOK_PROMPTS` gates `requestPrompt` injection into
      `ToolUseContext` at `screens/REPL.tsx:2520`.
- [ ] `MESSAGE_ACTIONS` only renders the keybinding overlay when
      fullscreen is enabled and `disableMessageActions` is false.
- [ ] `HISTORY_PICKER` flips `useHistorySearch.ts:240` to inactive and
      enables the picker hook in `PromptInput.tsx:1721`.
- [ ] `BUDDY` companion-narrow layout flips
      `flexDirection: 'column'` at `screens/REPL.tsx:4590`.
- [ ] `LODESTONE` triggers `updateDeepLinkTerminalPreference()` after
      trust, registers a protocol module via
      `utils/backgroundHousekeeping.ts:10`, and adds a settings key.
- [ ] ANT-only footer additions (`undercover` text and
      `CoordinatorTaskPanel`) are bundled out via the
      `"external" === 'ant'` literal pattern post-DCE.
- [ ] `patchConsole` is on by default; rendering messages after Ink
      mount goes through `exitWithError`/`exitWithMessage`.
- [ ] `onFrame` flicker analytics: skip under
      `isSynchronizedOutputSupported()`; debounce non-`resize`
      flickers at 1000ms; payload `{desiredHeight, actualHeight,
      reason}`.
- [ ] `isNarrow = columns < 80`; `isShort = isFullscreen && rows < 24`.
- [ ] `dialogLaunchers.tsx`: every dialog uses dynamic `import()` to
      keep main.tsx bundle slim; `launchResumeChooser` parallelizes
      `getWorktreePaths` with the imports via `Promise.all`.
- [ ] **React Compiler runtime** (§10.5): `import { c as _c } from
      "react/compiler-runtime"` is present at the top of compiled
      `.tsx` files (verified at `Spinner.tsx:1`, `REPL.tsx:1`,
      `useTypeahead.tsx`); reimplementations must either ship the
      same compiler emit or hand-memoize every render path.
- [ ] **chalk-level patches** (§7): `ink/colorize.ts:21-26` boosts
      `chalk.level` 2→3 when `TERM_PROGRAM === 'vscode'`; `:52-54`
      clamps level >2 → 2 when `process.env.TMUX` is set (unless
      `CLAUDE_CODE_TMUX_TRUECOLOR` overrides). Both run at module
      import, mutate the `chalk` singleton globally, and order is
      load-bearing (boost before clamp).
- [ ] **`src/screens/` is exactly 3 files** — `REPL.tsx`,
      `Doctor.tsx`, `ResumeConversation.tsx`. There is **no central
      screen registry**; `main.tsx` and `interactiveHelpers.tsx`
      directly import each screen by path. Reimplementers should not
      introduce a router/registry layer for parity.

---

## 12. Open Questions / Unknowns

1. **Spinner.tsx top-level (38KB)**. Beyond `SpinnerGlyph`, the file
   exposes message-ramp orchestration, teammate-tree spinner sync,
   and stalled/shimmer animation hooks. A Phase-9 deep-read pass
   should enumerate each exported component and its frame timing.
2. **Messages.tsx + MessageSelector.tsx + Message.tsx pipeline**.
   Message rendering, virtualization (`VirtualMessageList.tsx`),
   message-actions cursor model, and message highlighting
   (`searchHighlight.ts`) interactions need explicit pseudocode. Listed
   as adjacent files only.
3. **`screens/REPL.tsx` (5005 LOC)**. Contains the canonical
   composition of every flag-gated UI surface above; only the
   constants and flag sites cited in §6.9 + §8 were grep-confirmed.
   Remaining surfaces (Coordinator panel layout math, RECENT_SCROLL
   state machine, fullscreen `setStickyFooter` lifecycle) need a
   focused expansion.
4. **`ink/ink.tsx` Ink-fork delta vs. upstream**. The leak appears to
   carry an Anthropic-internal fork of `ink` with bespoke
   `onFrame`/`flickers`/`patchConsole`/`internal_querier`/
   `useTerminalViewport` extensions. A diff against upstream `ink`
   would clarify which behaviors are forked vs. inherited.
5. **`isSynchronizedOutputSupported()`** detection mechanism (DEC 2026
   BSU/ESU) — referenced via `ink/terminal.ts` but the actual probe
   was not read.
6. **`KAIROS`/`KAIROS_CHANNELS`** dialog is in scope for spec 32 logic
   but the rendered `DevChannelsDialog` component lives under
   `src/components/`. Boundary: spec 37 owns rendering, spec 32 owns
   what gets rendered.
7. **`PermissionRequestTitle.tsx`**, `PermissionRuleExplanation.tsx`,
   `PermissionExplanation.tsx`, `PermissionDecisionDebugInfo.tsx`,
   `WorkerBadge.tsx`, `WorkerPendingPermission.tsx` — used by every
   `PermissionDialog`, but only the type signatures + props were
   grep-confirmed. Title/subtitle truncation rules are unverified.
8. **HelpV2/General.tsx + Commands.tsx** content (the `?` overlay) is
   referenced by `PromptInputHelpMenu` and not surveyed.
9. **`TeleportRepoMismatchDialog`**, `TeleportResumeWrapper`,
   `TeleportError`, `TeleportProgress`, `TeleportStash`: full UX
   strings not surveyed; spec 37 owns the launcher contract only.
10. **Custom-Select** component family (`CustomSelect/`) is the
    underlying primitive for the trust + MCP-approval + bypass-mode
    dialogs; cursor/keyboard semantics not surveyed in this pass.
11. **`design-system/FuzzyPicker.tsx`** (used by `QuickOpenDialog`,
    `GlobalSearchDialog`, `LogSelector`, `LanguagePicker`,
    `ModelPicker`, `ThemePicker`, `OutputStylePicker`,
    `WorkflowMultiselectDialog`, `MCPServerMultiselectDialog`):
    listed only.

---

## Source citations summary

Every cited line belongs to the leaked tree at
`/Users/new/Downloads/claude-code-main/`. Files exceeding the read
window (`Messages.tsx`, `MessageSelector.tsx`, `Spinner.tsx`,
`screens/REPL.tsx`, `PromptInput.tsx`, `StatusLine.tsx`) are cited
only for grep-confirmed lines; broader claims are flagged in §12.
