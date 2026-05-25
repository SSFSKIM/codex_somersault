# 21d — Command Catalog: Plugin & Misc Long Tail

> Catalog companion to specs 21a / 21b / 21c. Picks up the **73 command files**
> the original public/ant/flagged split missed: the entire `src/commands/plugin/`
> marketplace UI cluster (5 huge files plus 12 shared cells), the
> `install-github-app/` step machine (10 step components), and the long tail of
> public commands (`add-dir`, `chrome`, `context`, `copy`, `desktop`, `diff`,
> `doctor`, `effort`, `export`, `extra-usage`, `fast`, `feedback`, `heapdump`,
> `help`, `ide`, `install`, `install-slack-app`, `keybindings`, `login`,
> `logout`, `memory`, `mobile`, `passes`, `permissions`, `plan`,
> `privacy-settings`, `rate-limit-options`, `release-notes`, `remote-env`,
> `resume`, `review/ultrareviewCommand`, `rewind`, `sandbox-toggle`, `session`,
> `skills`, `status`, `stickers`, `tag`, `tasks`, `terminalSetup`, `theme`,
> `thinkback`, `thinkback-play`, `upgrade`, `usage`).
>
> **Read 20-command-system.md and 21a-command-catalog-public.md first.** This
> file inherits its terminology and the `Command` type union from spec 20. Most
> entries here are `local-jsx` shims that delegate to a component under
> `src/components/` or `src/screens/` — those component files are cited here and
> belong to spec 37 (Ink UI shell).
>
> All `src/commands/<name>/index.ts` registry entries are catalogued in
> 21a §3 already. Where 21a stopped at the registry citation, this file
> documents the **implementation file** (`<name>.tsx` or `<name>.ts`) — its
> imports, state model (if any), and the side effects the registry stub
> can't show.

---

## §0 Scope

### IN scope
- All 18 files under `src/commands/plugin/` (5 marketplace UIs ~785KB, 7 shared
  cells/dialogs/helpers, 4 entry/parsing files, 1 trust banner, 1 validator).
- All 14 step files under `src/commands/install-github-app/` (the OAuth + repo
  + workflow setup wizard).
- 4-file `extra-usage/` cluster (`extra-usage.tsx`,
  `extra-usage-noninteractive.ts`, `extra-usage-core.ts`, `index.ts`).
- 3-file `context/` cluster (`context.tsx`, `context-noninteractive.ts`,
  `index.ts`).
- 3-file `add-dir/` cluster (`add-dir.tsx`, `validation.ts`, `index.ts`).
- All other public-tier `<name>.tsx` / `<name>.ts` implementation files
  enumerated in PHASE9-COVERAGE.md's `→ 21-command-catalog (a/b/c)` bucket.

### OUT of scope
- Registry-level metadata for these commands → already in 21a §3 (one entry
  per `<name>/index.ts`).
- Feature-flag–gated commands → 21c.
- ANT-only commands → 21b.
- The plugin **service** layer (`src/services/plugins/*`,
  `src/utils/plugins/*`) → spec **28-service-plugins.md**. This file describes
  the **slash-command UI** that drives the service.
- Component implementations the commands delegate to (e.g. `HelpV2`,
  `Doctor`, `Settings`, `BackgroundTasksDialog`, `ExportDialog`,
  `RemoteEnvironmentDialog`) → spec **37-ink-ui-shell.md**.

---

## §1 Plugin Command Family

### §1.1 Overview

`/plugin` (aliases `/plugins`, `/marketplace`) is the user-facing surface for
Claude Code's plugin marketplace system. Its registry stub
(`src/commands/plugin/index.tsx`, 11 lines) declares
`type: 'local-jsx'`, `name: 'plugin'`, `aliases: ['plugins', 'marketplace']`,
`immediate: true`, and lazy-loads `./plugin.js`. The two files
`index.tsx` and `plugin.tsx` together total ~22 lines — every byte of the
plugin slash command's behaviour lives in the **18 sibling files** in
`src/commands/plugin/`, totaling **~960KB** uncompressed source.

The wiring is:
```
/plugin <args>
  → commands/plugin/index.tsx       (registry stub)
  → commands/plugin/plugin.tsx       (3-line call() returning <PluginSettings/>)
  → commands/plugin/PluginSettings.tsx  (top-level tab orchestrator)
       → ManagePlugins.tsx       (Installed tab)
       → DiscoverPlugins.tsx     (Discover tab)
       → ManageMarketplaces.tsx  (Marketplaces tab)
       → PluginErrors.tsx        (Errors tab — formatErrorMessage helper)
       → BrowseMarketplace.tsx   (drilled into from a marketplace row)
       → AddMarketplace.tsx      (drilled into from "Add" action)
       → ValidatePlugin.tsx      (subcommand-only, never tabbed)
```

The cluster is conceptually one application with four tabs (Discover /
Installed / Marketplaces / Errors), heavy keyboard navigation, scoped install
prompts, MCP server detail drills, and post-install configuration flows. It
is the largest single command in the entire codebase.

`parseArgs.ts` defines the `ParsedCommand` discriminated union the
slash-command surface exposes:

```typescript
type ParsedCommand =
  | { type: 'menu' }
  | { type: 'help' }
  | { type: 'install';   marketplace?: string; plugin?: string }
  | { type: 'manage' }
  | { type: 'uninstall'; plugin?: string }
  | { type: 'enable';    plugin?: string }
  | { type: 'disable';   plugin?: string }
  | { type: 'validate';  path?: string }
  | { type: 'marketplace'; action?: 'add'|'remove'|'update'|'list'; target?: string }
```

`parsePluginArgs(args)` (`parseArgs.ts:17-103`) implements the dispatch.
Notable: `install plugin@marketplace` syntax is split on `@` (line 39); a bare
URL/path argument to `install` is treated as a marketplace, not a plugin
(lines 44-55); both `marketplace` and `market` are accepted as the verb
(line 79); `rm` is an alias for `remove` (line 87); unknown verbs fall
through to `{ type: 'menu' }` (line 100).

### §1.2 Files

#### `src/commands/plugin/index.tsx` (11 lines, 1.3KB)
**Role:** Registry stub. Declares the `Command` object exported as default.
- `type: 'local-jsx'`, `name: 'plugin'`, `aliases: ['plugins', 'marketplace']`.
- `description: 'Manage Claude Code plugins'`, `immediate: true`.
- `load: () => import('./plugin.js')`.
- No `availability`, no `isEnabled`, no `isHidden`.

#### `src/commands/plugin/plugin.tsx` (6 lines, 1.6KB)
**Role:** `local-jsx` `call()`. Returns `<PluginSettings onComplete={onDone} args={args} />`.
The implementation is delegated 100% to `PluginSettings`.

#### `src/commands/plugin/parseArgs.ts` (103 lines, 2.8KB)
**Role:** Argument parser. Exports `ParsedCommand` discriminated union and
`parsePluginArgs(args?: string)`. Pure function; no I/O.
- Used by `PluginSettings.tsx` to dispatch to the right tab on first render.
- `install` argument disambiguation (plugin vs marketplace) is the only
  non-trivial logic.

#### `src/commands/plugin/usePagination.ts` (171 lines, 5.0KB)
**Role:** Custom hook for continuous-scroll list virtualization with
keyboard-driven cursor. `DEFAULT_MAX_VISIBLE = 5` (line 3). Internally tracks a
`scrollOffsetRef` and recomputes the visible window via `useMemo` based on
`selectedIndex`. Exposes a backwards-compatible page-based API
(`currentPage`, `totalPages`, `goToPage`, `nextPage`, `prevPage`) but those
are no-ops — actual scrolling is driven by `selectedIndex` updates. The
returned `scrollPosition.canScrollUp` / `canScrollDown` drive the
"more above ↑" / "more below ↓" affordances.
- Used by: `ManagePlugins`, `DiscoverPlugins`, `BrowseMarketplace`.

#### `src/commands/plugin/PluginTrustWarning.tsx` (~30 lines, 3.9KB)
**Role:** Memoized warning banner shown above install actions. Yellow figures.warning
+ italic dim text reading: *"Make sure you trust a plugin before installing,
updating, or using it. Anthropic does not control what MCP servers, files, or
other software are included in plugins and cannot verify that they will work
as intended or that they won't change. See each plugin's homepage for more
information."* Optionally appends a custom enterprise-policy message via
`getPluginTrustMessage()` (cited from `utils/plugins/marketplaceHelpers`).
- Used by: `BrowseMarketplace`, `DiscoverPlugins`.
- Heavily React-Compiler-cached (`_c(3)` cache slots) — the entire JSX tree
  is hoisted because it's almost-pure.

#### `src/commands/plugin/PluginSettings.tsx` (~1900 lines from 128KB source)
**Role:** Top-level tab orchestrator and the `local-jsx` call's actual root.
- Owns the `ViewState` type (`type: 'discover' | 'installed' | 'marketplaces' | 'errors'`,
  plus drill-in states for plugin/marketplace details).
- Renders `Pane` + `Tabs` from `components/design-system`, with one
  `<Tab>` per tab id.
- Dispatches first-render based on `parsePluginArgs(args)`:
  - `{ type: 'install', marketplace, plugin }` → opens BrowseMarketplace
    pre-targeted to that marketplace/plugin.
  - `{ type: 'manage' }` → Installed tab.
  - `{ type: 'marketplace', action }` → Marketplaces tab with that action.
  - `{ type: 'validate', path }` → ValidatePlugin component (no tab).
  - `{ type: 'menu' }` → Installed tab default.
- Owns `error` and `result` state (lifted so children can post status).
- Owns `MarketplaceList` helper (lines 31-60) for the list-only `marketplace list`
  subcommand path that bypasses the UI entirely.
- Imports from `services/plugins/pluginOperations` (cross-ref §28),
  `utils/plugins/marketplaceManager`, `utils/plugins/marketplaceHelpers`,
  and `state/AppState` (for app-state cache invalidation after install).
- Uses `useExitOnCtrlCDWithKeybindings` for graceful Ctrl-C/D exit.

#### `src/commands/plugin/ManagePlugins.tsx` (2,214 lines, **322KB — largest single command file in the codebase**)
**Role:** "Installed" tab. Lists every installed plugin grouped by scope
(Flagged → Project → Local → User → Enterprise → Managed → Built-in →
Dynamic). Cell renderer is `UnifiedInstalledCell`.

**State machine.** The internal `ViewState` is a discriminated union with
**12 states** (lines 78-105):
```typescript
type ViewState =
  | 'plugin-list'                          // default tab content
  | 'plugin-details'                       // selected plugin detail page
  | 'configuring'                          // post-install option dialog
  | { type: 'plugin-options'; ... }        // reconfigure already-installed plugin
  | { type: 'configuring-options'; schema: PluginOptionSchema }
  | 'confirm-project-uninstall'            // confirm uninstalling project-scoped
  | { type: 'confirm-data-cleanup'; size: { bytes: number; human: string } }
  | { type: 'flagged-detail';        plugin: FlaggedPluginInfo }
  | { type: 'failed-plugin-details'; plugin: FailedPluginInfo }
  | { type: 'mcp-detail';            client: MCPServerConnection }
  | { type: 'mcp-tools';             client: MCPServerConnection }
  | { type: 'mcp-tool-detail';       client: MCPServerConnection; tool: Tool }
```

**Subviews** the file renders, in order of how a user reaches them:
1. **List view** with scope headers + `UnifiedInstalledCell` rows + scroll
   indicators + a help-text byline showing `type to search /
   plugin:toggle / select:accept / confirm:no` shortcuts.
2. **Plugin details** when Enter on a row.
3. **Plugin-options reconfigure dialog** when reconfiguring a plugin that has
   `configSchema`.
4. **Confirm-data-cleanup** when uninstalling a plugin with cached data —
   shows formatted byte size from `getPluginDataDirSize`.
5. **Flagged-plugin detail** for plugins the policy engine flagged
   (`getFlaggedPlugins` from `utils/plugins/pluginFlagging`).
6. **Failed-plugin details** showing `PluginError[]` from `formatErrorMessage`
   (helper imported from `./PluginErrors.js`).
7. **MCP server detail / tool list / tool detail** drilled in from a plugin
   that exposes MCP servers — reuses `MCPToolListView`,
   `MCPToolDetailView` from `components/mcp/`.

**Operations dispatched.** Imports and uses these op functions from
`services/plugins/pluginOperations` (cross-ref spec **28**):
- `enablePluginOp`, `disablePluginOp`
- `uninstallPluginOp`
- `updatePluginOp`
- `getPluginInstallationFromV2`, `isInstallableScope`
- `isPluginEnabledAtProjectScope`

**Pending state model.** `pendingToggles: Set<string>` is rendered as
"will enable" / "will disable" sentinels in the cell (line 31 of
`UnifiedInstalledCell.tsx`); the bottom-of-pane footer reads
*"Run /reload-plugins to apply changes"* whenever the set is non-empty
(`ManagePlugins.tsx:2208-2212`). Plugin enable/disable does **not** apply
immediately to the running session — `/reload-plugins` is required.

**Search mode.** Uses `useSearchInput` + `SearchBox`. When search is active,
the parent (PluginSettings) is notified via `onSearchModeChange` so it can
suppress its own keybindings.

**Scope-grouping logic** (lines 2153-2179): builds a `getScopeLabel` lookup
that maps internal scope identifiers to user-facing labels. `dynamic` is
relabelled as `Built-in`, `flagged` is rendered bold + warning color.

**Helpers defined in-file:**
- `getBaseFileNames(dirPath)` (lines 129-148) — lists `.md` files in a directory,
  strips extension. Used to enumerate a plugin's commands.
- `getSkillDirNames(dirPath)` (lines 160-?) — scans a skills directory for
  subdirectories containing `SKILL.md`. Used to enumerate a plugin's skills.

**Flag gates:** none. ManagePlugins is unconditionally registered.

**Cross-spec edges:** spec 28 (plugin service & operations), spec 23
(MCP service for the MCP drill-ins), spec 27 (policy engine for flagged
plugins), spec 18 (mode/effort — none, just listing here for completeness).

#### `src/commands/plugin/DiscoverPlugins.tsx` (~1500 lines, 107KB)
**Role:** "Discover" tab. Aggregates all plugins from all configured
marketplaces into a single searchable list and lets the user install with a
scope picker.

**State machine** (lines 38-46):
```typescript
type ViewState =
  | 'plugin-list'
  | 'plugin-details'
  | { type: 'plugin-options'; plugin: LoadedPlugin; pluginId: string }
```

- Loads marketplaces via `loadMarketplacesWithGracefulDegradation` (so a
  single broken marketplace doesn't block the others).
- Filters out plugins blocked by enterprise policy
  (`isPluginBlockedByPolicy`).
- Surfaces install counts via `getInstallCounts` + `formatInstallCount`.
- Detects if a plugin is already installed
  (`isPluginGloballyInstalled`).
- Calls `installPluginFromMarketplace` from
  `utils/plugins/pluginInstallationHelpers` to install — which then triggers
  `findPluginOptionsTarget` to decide whether to open a config dialog post-install.
- Uses shared helpers from `pluginDetailsHelpers.tsx`
  (`buildPluginDetailsMenuOptions`, `extractGitHubRepo`).

#### `src/commands/plugin/BrowseMarketplace.tsx` (~1700 lines, 119KB)
**Role:** Marketplace-detail view. Drill from a single marketplace row in the
Marketplaces tab. Shows that marketplace's plugin list with install affordance.

**State machine** (lines 38-42):
```typescript
type ViewState =
  | 'marketplace-list'   // first level (drilling from an "install" arg)
  | 'plugin-list'
  | 'plugin-details'
  | { type: 'plugin-options'; plugin: LoadedPlugin; pluginId: string }
```

- Distinguished from DiscoverPlugins by being scoped to a single
  marketplace (so users can browse a curated list rather than the union).
- Same install machinery (`installPluginFromMarketplace`,
  `findPluginOptionsTarget`, `PluginOptionsFlow`).
- Re-uses `pluginDetailsHelpers` for menu options.
- `targetMarketplace` / `targetPlugin` props let the parent jump straight to
  a specific marketplace + plugin row when invoked via `/plugin install
  plugin@marketplace`.

#### `src/commands/plugin/ManageMarketplaces.tsx` (~1700 lines, 119KB)
**Role:** "Marketplaces" tab. Lists configured marketplaces with their
sources, last-updated time, plugin count, installed plugin count, and
auto-update toggle.

**State machine** (line 48):
```typescript
type InternalViewState = 'list' | 'details' | 'confirm-remove'
```

**Per-row pending state** (lines 38-47):
```typescript
type MarketplaceState = {
  name: string
  source: string
  lastUpdated?: string
  pluginCount?: number
  installedPlugins?: LoadedPlugin[]
  pendingUpdate?: boolean
  pendingRemove?: boolean
  autoUpdate?: boolean
}
```

- Uses raw `useInput` (eslint-disabled at line 9) for `u` (update),
  `r` (remove), and `y/n` confirmation shortcuts that aren't in the
  keybinding schema.
- Operations: `refreshMarketplace`, `removeMarketplaceSource`,
  `setMarketplaceAutoUpdate`, `updatePluginsForMarketplaces`.
- Reaches into `services/analytics` to log `tengu_marketplace_*` events.

#### `src/commands/plugin/AddMarketplace.tsx` (~500 lines, 22KB)
**Role:** "Add a marketplace" form. Single text input + add button. Auto-runs
when `cliMode=true` (i.e., when invoked via `/plugin marketplace add <url>`
with the URL pre-filled).

- Parses input via `parseMarketplaceInput` (URL, GitHub repo, file path).
- Calls `addMarketplaceSource`, then `saveMarketplaceToSettings`.
- Shows progress messages via `progressMessage` state during the
  potentially-slow git clone or HTTP fetch.
- Tracks `hasAttemptedAutoAdd: useRef(false)` to prevent re-triggering on
  re-renders.

#### `src/commands/plugin/PluginErrors.tsx` (~330 lines, 23KB)
**Role:** Two purposes —
1. The "Errors" tab in the main settings UI.
2. The exported helpers `formatErrorMessage` and `getErrorGuidance` used by
   ManagePlugins, DiscoverPlugins, and BrowseMarketplace whenever they need
   to render a `PluginError` from `types/plugin`.

**Error taxonomy** (~25 error types switched in `formatErrorMessage`,
lines 2-?):
- `path-not-found`, `git-auth-failed`, `git-timeout`, `network-error`
- `manifest-parse-error`, `manifest-validation-error`
- `plugin-not-found`, `marketplace-not-found`, `marketplace-load-failed`
- `mcp-config-invalid`, `mcp-server-suppressed-duplicate`
- `hook-load-failed`, `component-load-failed`
- `mcpb-download-failed`, `mcpb-extract-failed`, `mcpb-invalid-manifest`
- `marketplace-blocked-by-policy`
- `dependency-unsatisfied` (with `not-enabled` vs `not-installed` sub-reason)
- `lsp-config-invalid`, `lsp-server-start-failed`, `lsp-server-crashed`,
  `lsp-request-timeout`

`getErrorGuidance(error)` returns user-actionable next-step text per type
(documented in source as `// suggest: re-clone` etc.).

#### `src/commands/plugin/PluginOptionsDialog.tsx` (~600 lines, 35KB)
**Role:** Generic config-form dialog driven by a `PluginOptionSchema`. Walks
fields one at a time (or shows them all in a list), collecting strings into
`Record<string, string>`.

**Key feature: secret preservation.** The exported `buildFinalValues(fields,
collected, configSchema, initialValues)` (lines 24-45) is called when the
user hits Save. For sensitive fields (`schema.sensitive === true`), if the
collected buffer is empty AND `initialValues` had a value, the key is
**omitted** from the payload — `savePluginOptions` then leaves the existing
secret in place. The comment block at lines 12-22 explicitly notes this is
a security measure to prevent silent secret-wipes on reconfigure.

Type coercion in the same function:
- `type: 'number'`: `Number('') → 0` is treated as omitted (so `required`
  validation can fire); else `Number(value)`, falling back to the raw string
  if `NaN`.
- `type: 'boolean'`: `isEnvTruthy(value)` from `utils/envUtils`.
- Default: pass-through string.

Uses raw `useInput` (eslint-disabled at line 6) for text input handling.

#### `src/commands/plugin/PluginOptionsFlow.tsx` (~430 lines, 19KB)
**Role:** Multi-step orchestrator for post-install config dialogs.

- Exports `findPluginOptionsTarget(pluginId)` (lines 27-33) — used by
  Discover/Browse to resolve the just-installed plugin so the dialog can
  open against fresh data.
- Walks both top-level `manifest.userConfig` and channel-specific
  `userConfig` blocks (the latter from `getUnconfiguredChannels` /
  `mcpPluginIntegration`). Each becomes a `ConfigStep`:

```typescript
type ConfigStep = {
  key: string
  title: string
  subtitle: string
  schema: PluginOptionSchema
  load: () => PluginOptionValues | undefined
  save: (values: PluginOptionValues) => void
}
```

- Calls `onDone('skipped')` immediately when nothing needs configuring
  (lines 1-8 docstring).
- Save dispatches per-step: `savePluginOptions` for top-level options,
  `saveMcpServerUserConfig` for MCP channels.

#### `src/commands/plugin/UnifiedInstalledCell.tsx` (~1200 lines, 44KB)
**Role:** Single-row renderer for the Installed tab. The "unified" name
reflects that it renders both **plugins** and **MCP servers** with the same
card layout — `item.type === 'plugin' | 'mcp-server'` discriminator.

Status icon + text logic (sampled lines 18-50):
- `pendingToggle === 'will-enable' | 'will-disable'` → suggestion arrow +
  "will enable / will disable"
- `errorCount > 0` → red cross + `N error(s)`
- (else cases continue: `disabled`, `enabled`, `outdated`, `built-in`, etc.)

Heavily React-Compiler-cached (`_c(142)` cache slots) — single largest
useMemo cache count in the plugin family, reflecting the cost of repeatedly
rendering this cell during scroll.

#### `src/commands/plugin/ValidatePlugin.tsx` (~250 lines, 12KB)
**Role:** Output for `/plugin validate <path>`. Calls `validateManifest(path)`
from `utils/plugins/validatePlugin`, formats `result.errors` and
`result.warnings` arrays into a single text payload, calls `onComplete`.

When `path` is missing, returns the embedded usage block:
```
Usage: /plugin validate <path>
Validate a plugin or marketplace manifest file or directory.
Examples:
  /plugin validate .claude-plugin/plugin.json
  /plugin validate /path/to/plugin-directory
  /plugin validate .
When given a directory, automatically validates .claude-plugin/marketplace.json
or .claude-plugin/plugin.json (prefers marketplace if both exist).
Or from the command line: claude plugin validate <path>
```
(Lines 26-?, verbatim.)

This is also exposed at the CLI top level — `claude plugin validate` (see
spec 01). Same code path.

#### `src/commands/plugin/pluginDetailsHelpers.tsx` (~340 lines, 12KB)
**Role:** Pure-ish helpers shared by Discover/Browse plugin-details views.
- `extractGitHubRepo(plugin)` — discriminates `plugin.entry.source.source ===
  'github'` and returns `entry.source.repo`, else `null`.
- `buildPluginDetailsMenuOptions(hasHomepage, githubRepo)` — returns the
  ordered list of action options (`install-user`, `install-project`, `install-local`,
  …, `view-homepage`, `view-github`, `back`) shown on the details screen.
- Also exports `PluginSelectionKeyHint` (a small JSX helper for the keybinding
  byline) and the `InstallablePlugin` and `PluginDetailsMenuOption` types.

### §1.3 Plugin command interaction with `/plugins` slash

All three name-aliases (`/plugin`, `/plugins`, `/marketplace`) hit
`commands/plugin/index.tsx`'s default export and therefore go through the
same `<PluginSettings/>` root. Args are not pre-parsed by the registry;
PluginSettings calls `parsePluginArgs(args)` itself on first render to decide
which tab and which drill-in to open.

`/reload-plugins` is a **separate** command (`commands/reload-plugins/`,
catalogued in 21a). After enabling/disabling plugins via this UI, the
pending toggles persist to settings but the running session's plugin list is
**not** refreshed — `/reload-plugins` must be invoked to apply.

The `claude plugin <subcommand>` CLI top-level invocation (spec 01) also
hits this code path: the entrypoint forwards to the slash command with
`cliMode: true` propagated to AddMarketplace and others so they auto-run
without prompting.

### §1.4 Cross-spec edges

| Subsystem | Spec | What ManagePlugins/family imports |
|---|---|---|
| Plugin service & ops | **28** | `enablePluginOp`, `disablePluginOp`, `uninstallPluginOp`, `updatePluginOp`, `getPluginInstallationFromV2`, `isInstallableScope` |
| Plugin storage & paths | **28** | `loadInstalledPluginsV2`, `pluginDataDirPath`, `getPluginDataDirSize` |
| Marketplace manager | **28** | `getMarketplace`, `loadKnownMarketplacesConfig`, `addMarketplaceSource`, `removeMarketplaceSource`, `refreshMarketplace`, `setMarketplaceAutoUpdate` |
| Marketplace helpers | **28** | `loadMarketplacesWithGracefulDegradation`, `createPluginId`, `formatFailureDetails`, `formatMarketplaceLoadingErrors`, `getMarketplaceSourceDisplay`, `getPluginTrustMessage` |
| Plugin loader | **28** | `loadAllPlugins`, `pluginStartupCheck.getPluginEditableScopes` |
| Plugin policy | **27** | `isPluginBlockedByPolicy` |
| Plugin flagging | **28** | `getFlaggedPlugins`, `markFlaggedPluginsSeen`, `removeFlaggedPlugin`, `pluginFlagging` |
| Plugin options storage | **28** | `loadPluginOptions`, `savePluginOptions`, `getUnconfiguredOptions` |
| MCPB plugins | **28** | `mcpbHandler` (`isMcpbSource`, `loadMcpbFile`, `loadMcpServerUserConfig`, `saveMcpServerUserConfig`) |
| MCP service | **23** | `useMcpToggleEnabled`, `MCPConnectionManager`, `filterToolsByServer`, types |
| MCP UI | **37** | `MCPToolListView`, `MCPToolDetailView`, `MCPRemoteServerMenu`, `MCPStdioServerMenu` |
| Built-in plugins | **28** | `getBuiltinPluginDefinition` |
| Settings | **02** | `getSettings_DEPRECATED`, `getSettingsForSource`, `updateSettingsForSource` |
| Analytics | **26** | `logEvent('tengu_marketplace_*')` |
| Keybindings | **39** | `useKeybinding`, `useKeybindings`, raw `useInput` for text-mode |
| Search hook | **37** | `useSearchInput`, `SearchBox` |
| Terminal sizing | **37** | `useTerminalSize`, `useTerminalFocus` |
| App state | **41** | `useAppState`, `useSetAppState`, `setAppState` invalidation |

---

## §2 GitHub-App Install Wizard

The `/install-github-app` command (registered in 21a §3.27) is implemented as
a 14-file step machine under `src/commands/install-github-app/`. The
registry stub (`index.ts`) lazy-loads `install-github-app.tsx`, which is the
top-level `<InstallGitHubApp/>` component owning the step state.

### §2.1 State machine

`install-github-app.tsx` (lines 28-?) defines `INITIAL_STATE: State` with the
following step machine (sampled at lines 28-40):

```typescript
{
  step: 'check-gh',
  selectedRepoName: '',
  currentRepo: '',
  useCurrentRepo: false,
  apiKeyOrOAuthToken: '',
  useExistingKey: true,
  currentWorkflowInstallStep: 0,
  warnings: [],
  secretExists: false,
  secretName: 'ANTHROPIC_API_KEY',
  useExistingSecret: true,
  /* ... */
}
```

The `step` field cycles through these values, each implemented as one
component file:

| Step | File | Role |
|---|---|---|
| `check-gh` | `CheckGitHubStep.tsx` | Verify `gh` CLI installed + authenticated |
| `choose-repo` | `ChooseRepoStep.tsx` | Pick a target repo (default: current `git remote`) |
| `install-app` | `InstallAppStep.tsx` | Open the GitHub App install URL in browser |
| `oauth-flow` | `OAuthFlowStep.tsx` | Anthropic OAuth (when no API key configured) |
| `api-key` | `ApiKeyStep.tsx` | Collect or use existing `ANTHROPIC_API_KEY` |
| `check-existing-secret` | `CheckExistingSecretStep.tsx` | `gh secret list` to detect existing secrets |
| `existing-workflow` | `ExistingWorkflowStep.tsx` | Detect existing `.github/workflows/claude.yml` |
| `creating` | `CreatingStep.tsx` | Push the commit / create the workflow |
| `warnings` | `WarningsStep.tsx` | Show non-fatal warnings before success |
| `success` | `SuccessStep.tsx` | Final confirmation + URL to view runs |
| `error` | `ErrorStep.tsx` | Terminal failure with retry option |

Helper module `setupGitHubActions.ts` contains the `gh` shell invocations
(`execa`-based) that actually install the workflow.

### §2.2 Cross-spec edges
- spec **25-service-oauth-auth**: OAuthFlowStep delegates to the same OAuth
  flow as `/login` (cross-ref 21a §3.34).
- spec **42-misc** (or wherever git utilities live): `getGithubRepo` from
  `utils/git`, `execFileNoThrow`.
- spec **26-service-analytics-flags**: `logEvent('tengu_github_app_*')` at
  each step transition.

`WorkflowMultiselectDialog` (imported from `components/`) is one of the two
points where install-github-app reaches into shared UI; it should be
catalogued under spec 37.

---

## §3 Long-tail Public Commands

### §3.1 Group by purpose

**A. UI-thin shims (returning a single `<Component/>`).**

| Command | File | Component delegated to | Component spec |
|---|---|---|---|
| `/copy` | `copy/copy.tsx` | inline `Select`/markdown extractor | this file |
| `/desktop` | `desktop/desktop.tsx` | `DesktopHandoff` | 37 |
| `/diff` | `diff/diff.tsx` | `DiffDialog` (lazy-imported) | 37 |
| `/doctor` | `doctor/doctor.tsx` | `Doctor` (from `screens/`) | 37 |
| `/export` | `export/export.tsx` | `ExportDialog` | 37 |
| `/feedback` | `feedback/feedback.tsx` | `Feedback` | 37 |
| `/help` | `help/help.tsx` | `HelpV2` | 37 |
| `/memory` | `memory/memory.tsx` | `MemoryFileSelector` | 37 / 40 |
| `/passes` | `passes/passes.tsx` | `Passes` | 37 |
| `/permissions` | `permissions/permissions.tsx` | `PermissionRuleList` | 09 / 37 |
| `/privacy-settings` | `privacy-settings/privacy-settings.tsx` | `GroveDialog` / `PrivacySettingsDialog` | 37 |
| `/release-notes` | `release-notes/release-notes.ts` | (text payload) | this file |
| `/remote-env` | `remote-env/remote-env.tsx` | `RemoteEnvironmentDialog` | 35 / 37 |
| `/sandbox-toggle` | `sandbox-toggle/sandbox-toggle.tsx` | `SandboxSettings` | 37 |
| `/skills` | `skills/skills.tsx` | `SkillsMenu` | 17 / 37 |
| `/status` | `status/status.tsx` | `<Settings defaultTab="Status"/>` | 37 |
| `/stickers` | `stickers/stickers.ts` | (browser open) | this file |
| `/tasks` | `tasks/tasks.tsx` | `BackgroundTasksDialog` | 37 |
| `/usage` | `usage/usage.tsx` | `<Settings defaultTab="Usage"/>` | 37 |

**B. Multi-step or stateful commands** (substantive logic in the command file
itself).

| Command | File | Description |
|---|---|---|
| `/add-dir` | `add-dir/add-dir.tsx` (+ `validation.ts`) | Validate path, persist via `applyPermissionUpdate` + `persistPermissionUpdate`. Inline `AddDirError` component for failure paths. |
| `/chrome` | `chrome/chrome.tsx` | Menu with `install-extension`, `reconnect`, `manage-permissions`, `toggle-default` actions; opens browser to `claude.ai/chrome` etc. |
| `/context` | `context/context.tsx` (+ `context-noninteractive.ts`) | Renders `<ContextVisualization/>` with API-view-transformed messages. The `toApiView` helper applies the same compact-boundary + `projectView` (CONTEXT_COLLAPSE) + `microcompactMessages` transforms `query.ts` does, so the displayed token count matches what the model sees. |
| `/effort` | `effort/effort.tsx` | Manage `effortLevel` setting. Help args `help / -h / --help`. Validates value via `isEffortLevel`, persists via `updateSettingsForSource`. |
| `/extra-usage` | `extra-usage/extra-usage.tsx` (+ `-noninteractive.ts` + `-core.ts`) | Browser handoff (default flow); on auth failure falls through to `<Login/>`. The `-core.ts` is shared between interactive and non-interactive variants. |
| `/fast` | `fast/fast.tsx` | Toggle `fastMode` setting. Switches `mainLoopModel` if current model doesn't support fast mode (via `isFastModeSupportedByModel`). Sub-flow: cooldown clearance, model pricing display. |
| `/heapdump` | `heapdump/heapdump.ts` | Wraps `performHeapDump()` from `utils/heapDumpService`; returns `{type:'text', value: heapPath\\ndiagPath}` on success. |
| `/ide` | `ide/ide.tsx` | Detect IDEs via `detectIDEs` / `detectRunningIDEs`, show `IDEScreen` with available + unavailable lists, persist selection. Includes `IdeAutoConnectDialog` for auto-connect prompting. |
| `/install` | `install.tsx` (39KB, single-file form) | Native installer flow. State machine: `checking → cleaning-npm → installing → setting-up → set-up → success` with `error` branch. Calls `checkInstall`, `cleanupNpmInstallations`, `cleanupShellAliases`, `installLatest` from `utils/nativeInstaller/`. Render-able both as command and as standalone `render()` (top-level). |
| `/install-github-app` | (see §2 above) | 14-file step machine for GitHub Action wizard. |
| `/install-slack-app` | `install-slack-app/install-slack-app.ts` | One-shot: `logEvent('tengu_install_slack_app_clicked')`, increment `slackAppInstallCount`, open browser to `slack.com/marketplace/A08SF47R6P4-claude`. |
| `/keybindings` | `keybindings/keybindings.ts` | Open keybindings JSONC in `$EDITOR`. Uses exclusive-create (`wx`) when writing the template to avoid TOCTOU; gated on `isKeybindingCustomizationEnabled()`. |
| `/login` | `login/login.tsx` | OAuth flow + post-login refresh: resets cost state, refreshes managed settings / policy limits / GrowthBook, strips signature blocks (signature-bearing thinking/connector_text blocks bind to the API key, so a key change invalidates them). |
| `/logout` | `logout/logout.tsx` | Calls `performLogout({clearOnboarding})` which (in this critical order): flushes telemetry **before** clearing credentials (to prevent org leakage across accounts), then `removeApiKey`, clears OAuth tokens, clears caches (betas, tool schema, remote managed settings, policy limits), invalidates user cache, calls `gracefulShutdownSync`. |
| `/mobile` | `mobile/mobile.tsx` | QR code render (via `qrcode` lib) for iOS / Android app store URLs. Inline `MobileQRCode` component. |
| `/plan` | `plan/plan.tsx` | View / edit current plan file. `PlanDisplay` component (in-file) shows the current plan. Triggers `handlePlanModeTransition` and applies plan-mode permission updates via `applyPermissionUpdate` + `prepareContextForPlanMode`. Supports opening plan in `$EDITOR` via `editFileInEditor`. |
| `/rate-limit-options` | `rate-limit-options/rate-limit-options.tsx` | Conditional menu: shows `upgrade`, `extra-usage`, `cancel`. Uses GrowthBook flag value (`getFeatureValue_CACHED_MAY_BE_STALE`) and rate-limit tier from `getRateLimitTier`. Delegates to `extraUsageCall` and `upgradeCall` from sibling commands. |
| `/resume` | `resume/resume.tsx` | Multi-modal: list-and-pick UI via `LogSelector`, plus search via `agenticSessionSearch`, custom-title search via `searchSessionsByCustomTitle`, cross-project resume via `checkCrossProjectResume`. Result type `ResumeResult` with `'sessionNotFound'` and other variants. Uses `useIsInsideModal` to render differently when called from within a dialog. |
| `/review` (ultra) | `review/ultrareviewCommand.tsx` | Spawn a remote review session via `launchRemoteReview` (CCR/teleport). On success returns content blocks as user message with `shouldQuery: true`. Handles abort signal — if aborted during the ~5s launch window, suppresses `onDone` to avoid writing to a dead transcript slot. Includes `UltrareviewOverageDialog` flow for overage confirmation via `checkOverageGate` / `confirmOverage`. |
| `/rewind` | `rewind/rewind.ts` | Triggers `context.openMessageSelector()` if available, returns `{type:'skip'}` to suppress message append. |
| `/session` | `session/session.tsx` | Show session info (`remoteSessionUrl` from app state) as QR code via `qrcode`. Bare-bones `<SessionInfo/>` component. |
| `/tag` | `tag/tag.tsx` | Tag the current session. Tag identified by `getCurrentSessionTag`; persisted via `saveTag`. Supports `help`/`info` standard args. Includes inline `ConfirmRemoveTag` dialog. Uses `recursivelySanitizeUnicode` on user-supplied tag name. |
| `/terminalSetup` | `terminalSetup/terminalSetup.tsx` (large) | Interactive backup-and-modify flow for terminal preferences (Apple Terminal `.plist`, iTerm2 prefs). Calls `backupTerminalPreferences`, `setupShellCompletion`, `markTerminalSetupComplete`. Supports `supportsHyperlinks` detection. ANSI / shell config edits via `addItemToJSONCArray` + `safeParseJSONC`. |
| `/theme` | `theme/theme.tsx` | `<ThemePicker/>` wrapper. On select: `setTheme(setting)` + `onDone('Theme set to ${setting}')`. |
| `/thinkback` | `thinkback/thinkback.tsx` | Plays an animation. Triggers `enablePluginOp` if the `thinkback` plugin isn't enabled (silent enable). Deals with the marketplace difference: `claude-code-marketplace` (ANT) vs `OFFICIAL_MARKETPLACE_NAME` (public). Exports `playAnimation` for `/thinkback-play` to reuse. |
| `/thinkback-play` | `thinkback-play/thinkback-play.ts` | Looks up the `thinkback` plugin's installed skills directory from V2 plugin config, then invokes `playAnimation` (re-exported from `/thinkback`). |
| `/upgrade` | `upgrade/upgrade.tsx` | Plan-upgrade flow. Detects if user is already on Max 20x (highest tier) — uses `getOauthProfileFromOauthToken` if subscription metadata is missing from the cached OAuth tokens. Falls through to `<Login/>` flow. |

### §3.2 Detailed entries (≥5KB)

#### `/install` (`install.tsx`, 39KB)

State union (lines 21-39):
```typescript
type InstallState =
  | { type: 'checking' }
  | { type: 'cleaning-npm' }
  | { type: 'installing'; version: string }
  | { type: 'setting-up' }
  | { type: 'set-up'; messages: string[] }
  | { type: 'success'; version: string; setupMessages?: string[] }
  | { type: 'error'; message: string; warnings?: string[] /* ... */ }
```

Native installer entry point. Can be invoked both as a slash command and as a
top-level `claude install` CLI subcommand (spec 01). Calls `render()` directly
when used as the standalone CLI.

Side effects:
- `cleanupNpmInstallations()` removes prior npm-installed copies.
- `cleanupShellAliases()` cleans up `~/.zshrc` / `~/.bashrc` aliases.
- `installLatest({force, target})` downloads + installs the native binary.
- `updateSettingsForSource('userSettings', {...})` to record install state.

#### `/login` (`login.tsx`, ~10KB)

After successful login (`success === true`), this exact post-login sequence
runs (sourced from comments in lines 25-?):
1. `resetCostState()` — switching accounts means starting fresh on cost tracking.
2. `void refreshRemoteManagedSettings()` (non-blocking).
3. `void refreshGrowthBookAfterAuthChange()` (non-blocking) — feature flags
   may differ per account.
4. `void refreshPolicyLimits()` (non-blocking).
5. `checkAndDisableAutoModeIfNeeded()` + `checkAndDisableBypassPermissionsIfNeeded()`
   — if the new account's policy disallows auto-mode or
   `--dangerously-skip-permissions`, those settings are forcibly disabled.
6. `resetUserCache()`.
7. `enrollTrustedDevice()` — adds the new account's device fingerprint to the
   bridge's trusted-device set (spec 34).

`stripSignatureBlocks(messages)` runs synchronously **before** the async
refresh chain — signature-bearing thinking/connector_text blocks are bound
to the previous API key and would be rejected by Anthropic's signature
verifier on the next request.

#### `/logout` (`logout.tsx`, ~8KB)

The exported helper `performLogout({clearOnboarding})` runs in this exact
order (per source comments and lines 19-?):
1. **Flush telemetry first.** `await import('../../utils/telemetry/instrumentation.js'); await flushTelemetry();` — comment at line 18-19: *"Flush telemetry BEFORE clearing credentials to prevent org data leakage"*.
2. `removeApiKey()` — deletes the keychain entry / config key.
3. Clear OAuth tokens (`getClaudeAIOAuthTokens` first to know what to clear).
4. `clearTrustedDeviceTokenCache()` — bridge trust state (spec 34).
5. `clearBetasCaches()`, `clearToolSchemaCache()`, `clearRemoteManagedSettingsCache()`,
   `clearPolicyLimitsCache()`.
6. `refreshGrowthBookAfterAuthChange()` — flush new (logged-out) feature
   values.
7. `getSecureStorage()` clear (keychain fallback).
8. `resetUserCache()`.
9. `gracefulShutdownSync()`.

#### `/install-slack-app` (`install-slack-app.ts`, ~30 lines)
Trivial. `logEvent('tengu_install_slack_app_clicked', {})`, bumps
`slackAppInstallCount` in global config, opens
`https://slack.com/marketplace/A08SF47R6P4-claude` in the browser. Returns
text payload: either *"Opening Slack app installation page in browser…"* or
*"Couldn't open browser. Visit: <url>"*.

#### `/keybindings` (`keybindings.ts`, ~70 lines)
Gated on `isKeybindingCustomizationEnabled()` from
`keybindings/loadUserBindings`. If gated off: returns *"Keybinding
customization is not enabled. This feature is currently in preview."*

Otherwise: writes `generateKeybindingsTemplate()` to `getKeybindingsPath()`
using the `wx` (exclusive create) flag — fails with `EEXIST` if the file
already exists, which is what the code wants (so we don't clobber a user's
edits). Then opens the file in `$EDITOR` via `editFileInEditor`. Cross-ref
spec 39 (vim-keybindings) and spec 02 (settings/migrations).

#### `/release-notes` (`release-notes.ts`)
Tries to fetch the latest changelog from `CHANGELOG_URL` with a **500ms
timeout** (line 24). On timeout or error, falls back to `getStoredChangelog()`
which reads the cached copy. Pure text command; result is the formatted
output of `formatReleaseNotes`.

#### `/effort` (`effort.tsx`)
Help args: `['help', '-h', '--help']` (line 9, `COMMON_HELP_ARGS`). Calls
`getDisplayedEffortLevel()`, `getEffortEnvOverride()`,
`getEffortValueDescription()` to build the status output. On set: validates
via `isEffortLevel`, persists via `updateSettingsForSource('userSettings',
{effortLevel: persistable})`. Logs `tengu_effort_command` event.

#### `/fast` (`fast.tsx`)
Toggle for `fastMode` user setting. The interesting bit is the auto
model-switch (lines 22-30):
```typescript
const needsModelSwitch = !isFastModeSupportedByModel(prev.mainLoopModel)
return {
  ...prev,
  ...(needsModelSwitch
    ? { mainLoopModel: getFastModeModel(), mainLoopModelForSession: null }
    : {}),
  /* ... */
}
```
Status output uses `formatModelPricing` + `getOpus46CostTier` to display the
Fast-mode model's pricing. Cooldown handling via `clearFastModeCooldown` /
`getFastModeRuntimeState`. `prefetchFastModeStatus` is called on mount to
warm the status cache.

#### `/context` (`context.tsx` + `context-noninteractive.ts`)

`context.tsx`'s `toApiView(messages)` (lines 18-29) replicates `query.ts`'s
pre-API transforms so the displayed token count matches what the model
sees:
1. `getMessagesAfterCompactBoundary(messages)` — strip pre-compact-boundary
   messages.
2. If `feature('CONTEXT_COLLAPSE')` is on: lazy-require
   `services/contextCollapse/operations.js`'s `projectView()` and apply.
3. (Note: microcompact is handled by `microcompactMessages` import; the
   non-interactive variant explicitly applies it, the interactive one
   reads it via the visualization component.)

The non-interactive variant `context-noninteractive.ts` mirrors the same
collection logic for the SDK `get_context_usage` control request, returning
typed `ContextData` from `analyzeContextUsage` and formatted-tokens
`formatTokens(...)`. Cross-ref spec 03 (QueryEngine) for the API-side
transforms and spec 07 (compaction) for compact boundary semantics.

#### `/copy` (`copy.tsx`)

Extracts code blocks from the most recent assistant message via
`marked.lexer(stripPromptXMLTags(...))` (line 31), filters
`token.type === 'code'`, and presents them in a `<Select>` for the user to
pick. Selected block is written to a tmpdir file (`COPY_DIR =
join(tmpdir(), 'claude')`, `RESPONSE_FILENAME = 'response.md'`), and the path
is set to clipboard via `setClipboard` (the OSC-52 escape from
`ink/termio/osc.js`). Looks back at most `MAX_LOOKBACK = 20` (line 25)
assistant messages.

#### `/passes` (`passes.tsx`)
First-visit detection: if `!config.hasVisitedPasses`, persists
`hasVisitedPasses: true` AND `passesLastSeenRemaining: getCachedRemainingPasses()`
(so subsequent upsells can detect if the count changed). Always logs
`tengu_guest_passes_visited` with `is_first_visit` boolean.

#### `/privacy-settings` (`privacy-settings.tsx`)
Two-stage:
1. `isQualifiedForGrove()` gate — non-qualified users get the fallback message
   *"Review and manage your privacy settings at
   https://claude.ai/settings/data-privacy-controls"* and `onDone` returns.
2. Qualified: parallel-fetches `getGroveSettings()` + `getGroveNoticeConfig()`,
   shows `<GroveDialog/>` + `<PrivacySettingsDialog/>` with the user's
   decision (`'accept' | 'decline' | 'escape' | 'defer'`) routed back via
   `onDoneWithDecision`. `'escape'` and `'defer'` both produce the
   *"Privacy settings dialog dismissed"* message with `display: 'system'`.

#### `/rate-limit-options` (`rate-limit-options.tsx`)
Conditional menu of plan-management actions. Displayed options depend on:
- `getRateLimitTier()` and `getSubscriptionType()` from auth.
- `hasClaudeAiBillingAccess()` — gates the upgrade path.
- `getFeatureValue_CACHED_MAY_BE_STALE()` GrowthBook value — gates which
  plan ladder rungs to show.
- `useClaudeAiLimits()` hook for live limit display.

Selecting an option dispatches to `extraUsageCall` (from sibling
`/extra-usage`) or `upgradeCall` (from sibling `/upgrade`) — these are
imported and called directly rather than re-routing through the slash
command system.

#### `/resume` (`resume.tsx`)
Multi-shape result type:
```typescript
type ResumeResult =
  | { resultType: 'sessionNotFound'; arg: string }
  | /* ... other shapes ... */
```
Behaviors based on args:
- No args → list-and-pick UI (`LogSelector` over
  `loadAllProjectsMessageLogs` or `loadSameRepoMessageLogs`).
- UUID → `validateUuid(arg)` then `getSessionIdFromLog`.
- Custom title query → `searchSessionsByCustomTitle` (gated on
  `isCustomTitleEnabled()`).
- Cross-project resume → `checkCrossProjectResume` for sessions outside the
  current worktree (`getWorktreePaths`).
- "lite" log path: `isLiteLog` → `loadFullLog` to expand.

`useIsInsideModal()` lets it render differently when launched from another
modal (it suppresses its own outer chrome).

#### `/sandbox-toggle` (`sandbox-toggle.tsx`)
Platform-gating logic:
- `SandboxManager.isSupportedPlatform()` → false on WSL1; emits *"Error:
  Sandboxing requires WSL2. WSL1 is not supported."* (or generic
  macOS/Linux/WSL2 message on other platforms).
- `SandboxManager.checkDependencies()` returns structured
  `{errors, warnings}` for missing deps (e.g., `sandbox-exec` on macOS,
  `bwrap` on Linux).
- Undocumented `enabledPlatforms` enterprise setting in
  `getSettings_DEPRECATED()` further restricts platform list (referenced at
  line ~25).

UI is `<SandboxSettings/>`; cross-ref spec 09 (permission-system) for how
sandbox decisions feed permission checks.

#### `/terminalSetup` (`terminalSetup.tsx`, large)
Backup-and-modify flow over `~/Library/Preferences/com.apple.Terminal.plist`
(via `getTerminalPlistPath()`) and similar iTerm2 prefs. Flow:
1. `backupTerminalPreferences()` to a side file.
2. Detect if terminal natively supports CSI u / Kitty keyboard protocol.
3. Edit the prefs (binary plist via `plutil` shell), patch shortcut
   bindings.
4. `setupShellCompletion()` adds shell-completion source line to user's
   `~/.zshrc` / `~/.bashrc`.
5. `markTerminalSetupComplete()` records completion in global config.
6. Provide rollback option via `checkAndRestoreTerminalBackup`.
Uses `addItemToJSONCArray` + `safeParseJSONC` for JSONC config edits. Logs
errors via `isFsInaccessible` to differentiate permission errors from
genuine missing files.

#### `/thinkback` (`thinkback.tsx`) and `/thinkback-play` (`thinkback-play.ts`)
**`/thinkback`** is the configuration entry — it ensures the `thinkback`
plugin is enabled (silent `enablePluginOp` if not) and plays an animation.
The marketplace differs by user type: ANT users get `claude-code-marketplace`,
public users get `OFFICIAL_MARKETPLACE_NAME`. Exports `playAnimation` for reuse.

**`/thinkback-play`** is the runtime trigger. It looks up the installed
plugin's skill directory in V2 plugin config (`loadInstalledPluginsV2()`),
identifies the install via `getPluginId()` (which is just the `thinkback@<marketplace>`
string), and calls the re-exported `playAnimation`.

If the plugin is not installed in V2 config (`installations` is empty), the
command returns an error to the user (line 24-?).

---

## §4 Reconciliation with 21a

Inspecting 21a §3 against this catalogue, the following entries in 21a are
**registry-only** stubs (each documents the `index.ts` but has no
implementation entry):

- §3.1 `/add-dir` — 21a covers index, `add-dir.tsx` is here.
- §3.4 `/branch` — 21a may cover this, double-check during HANDOFF; **NOT** in
  PHASE9-COVERAGE residuals → likely already covered.
- The full set listed in PHASE9-COVERAGE.md's `→ 21-command-catalog` bucket
  (72 files) — accepted as-is in 21d.

**Recommendation:** accept-as-is. 21a's discipline is "registry-citation
level," and its prose is already long. Forking the implementation files
into 21d is the cleanest split. No edits to 21a are required for the files
in scope here — the 73 residuals are genuinely missing rather than
mis-categorized. (One exception worth noting in HANDOFF: §3.31 `/insights` in
21a says the heavy body is "summarized" elsewhere. That summary is in spec
21 / 21a §3.31 itself, not residual.)

**Citation correctness audit performed:** none of the 73 residual files
listed in PHASE9-COVERAGE.md appear under cited paths in 21a/21b/21c (cross-checked
by grepping for each basename). All 73 are genuine omissions.

---

## §5 Cross-spec edges

| 21d §/file | Cross-spec dependency |
|---|---|
| §1 plugin family | spec **28** (plugin service & all `utils/plugins/*` and `services/plugins/*`); spec **23** (MCP service for the MCP-server detail drill-ins inside ManagePlugins); spec **27** (`isPluginBlockedByPolicy` policy check); spec **37** (every `Pane`/`Tabs`/`Dialog`/`Byline`/`SearchBox` import); spec **39** (`useKeybinding`, raw `useInput`); spec **41** (`useAppState` invalidation); spec **02** (settings persistence); spec **26** (analytics events) |
| §2 install-github-app | spec **25** (OAuthFlowStep); spec **26** (`tengu_github_app_*` events); spec **42** (`utils/git`, `execFileNoThrow`) |
| §3.A UI-thin shims | spec **37** (every component delegated to) |
| §3.B `/context` | spec **03** (QueryEngine pre-API transforms it mirrors); spec **07** (compact boundary); CONTEXT_COLLAPSE feature flag (spec **00** Appendix A) |
| §3.B `/login` | spec **25** (OAuth, console-OAuth flow); spec **26** (GrowthBook); spec **27** (policy limits + bypass-permissions killswitch); spec **34** (trusted-device enrollment) |
| §3.B `/logout` | spec **25** (token clearing); spec **42** (telemetry flush ordering); spec **34** (trusted-device cache) |
| §3.B `/install` | spec **01** (top-level CLI variant); `utils/nativeInstaller` (covered by spec 01 / 42) |
| §3.B `/install-github-app` | (see §2) |
| §3.B `/keybindings` | spec **39** (vim/keybindings) |
| §3.B `/sandbox-toggle` | spec **09** (sandbox feeds permission-system); `utils/sandbox/sandbox-adapter` is covered by spec 09 |
| §3.B `/resume` | spec **41** (session storage); spec **42** (`utils/agenticSessionSearch`, worktree paths) |
| §3.B `/review` (ultra) | CCR/teleport (spec **35-mode-remote-server**); `UltrareviewOverageDialog` is in the same `commands/review/` dir |
| §3.B `/rewind` | spec **41** (message selector / rewind UI is part of session-state-history) |
| §3.B `/thinkback` + `/thinkback-play` | spec **28** (plugin enable op + V2 plugin config); spec **17** (skill directory enumeration) |
| §3.B `/upgrade` | spec **25** (`getOauthProfileFromOauthToken`, OAuth profile lookup) |

---

## §6 Files cataloged here (full list)

### Plugin family (18 files)
1. `src/commands/plugin/index.tsx`
2. `src/commands/plugin/plugin.tsx`
3. `src/commands/plugin/parseArgs.ts`
4. `src/commands/plugin/usePagination.ts`
5. `src/commands/plugin/PluginTrustWarning.tsx`
6. `src/commands/plugin/PluginSettings.tsx` (128KB — top-level tab orchestrator)
7. `src/commands/plugin/ManagePlugins.tsx` (322KB — Installed tab; largest file)
8. `src/commands/plugin/DiscoverPlugins.tsx` (107KB)
9. `src/commands/plugin/BrowseMarketplace.tsx` (119KB)
10. `src/commands/plugin/ManageMarketplaces.tsx` (119KB)
11. `src/commands/plugin/AddMarketplace.tsx` (22KB)
12. `src/commands/plugin/PluginErrors.tsx` (23KB)
13. `src/commands/plugin/PluginOptionsDialog.tsx` (35KB)
14. `src/commands/plugin/PluginOptionsFlow.tsx` (19KB)
15. `src/commands/plugin/UnifiedInstalledCell.tsx` (44KB)
16. `src/commands/plugin/ValidatePlugin.tsx` (12KB)
17. `src/commands/plugin/pluginDetailsHelpers.tsx` (12KB)
18. (also referenced: `./types.js` — internal `ViewState` type re-export, not separately cited.)

Total plugin source: **~960KB across 18 files**, of which the 5 huge tab UIs
(ManagePlugins / BrowseMarketplace / DiscoverPlugins / ManageMarketplaces /
PluginSettings) together total ~785KB.

### install-github-app step machine (14 files)
1. `src/commands/install-github-app/index.ts` (registry, in 21a)
2. `src/commands/install-github-app/install-github-app.tsx` (root)
3. `src/commands/install-github-app/setupGitHubActions.ts` (helper)
4. `src/commands/install-github-app/CheckGitHubStep.tsx`
5. `src/commands/install-github-app/ChooseRepoStep.tsx`
6. `src/commands/install-github-app/InstallAppStep.tsx`
7. `src/commands/install-github-app/OAuthFlowStep.tsx`
8. `src/commands/install-github-app/ApiKeyStep.tsx`
9. `src/commands/install-github-app/CheckExistingSecretStep.tsx`
10. `src/commands/install-github-app/ExistingWorkflowStep.tsx`
11. `src/commands/install-github-app/CreatingStep.tsx`
12. `src/commands/install-github-app/WarningsStep.tsx`
13. `src/commands/install-github-app/SuccessStep.tsx`
14. `src/commands/install-github-app/ErrorStep.tsx`

### Long-tail public command files (44 files)
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
- `src/commands/extra-usage/extra-usage-core.ts` (shared core; not in
  PHASE9 list but cited transitively)
- `src/commands/fast/fast.tsx`
- `src/commands/feedback/feedback.tsx`
- `src/commands/heapdump/heapdump.ts`
- `src/commands/help/help.tsx`
- `src/commands/ide/ide.tsx`
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
- `src/commands/privacy-settings/privacy-settings.tsx`
- `src/commands/rate-limit-options/rate-limit-options.tsx`
- `src/commands/release-notes/release-notes.ts`
- `src/commands/remote-env/remote-env.tsx`
- `src/commands/resume/resume.tsx`
- `src/commands/review/ultrareviewCommand.tsx`
- `src/commands/rewind/rewind.ts`
- `src/commands/sandbox-toggle/sandbox-toggle.tsx`
- `src/commands/session/session.tsx`
- `src/commands/skills/skills.tsx`
- `src/commands/status/status.tsx`
- `src/commands/stickers/stickers.ts`
- `src/commands/tag/tag.tsx`
- `src/commands/tasks/tasks.tsx`
- `src/commands/terminalSetup/terminalSetup.tsx`
- `src/commands/theme/theme.tsx`
- `src/commands/thinkback/thinkback.tsx`
- `src/commands/thinkback-play/thinkback-play.ts`
- `src/commands/upgrade/upgrade.tsx`
- `src/commands/usage/usage.tsx`
- `src/commands/add-dir/validation.ts` (also covered transitively)

**Total cataloged in 21d: 76 files** (18 plugin + 13 install-github-app step
files counted as residuals + 45 long-tail). PHASE9 listed 72 commands as
residuals; the extra files here (`extra-usage-core.ts`,
`add-dir/validation.ts`, the registry stubs `index.ts` for plugin and
install-github-app) are transitively cited.
