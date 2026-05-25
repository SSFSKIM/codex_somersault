# 20 — Command System Specification

> The slash-command registry, three command kinds (`prompt` / `local` / `local-jsx`), dynamic skill/plugin/workflow integration, and remote/bridge safety allowlists. Anchor for spec 21 (the per-command catalog). Read 00-overview before this.

---

## 1. Purpose & Scope

The Command subsystem defines:
- The **`Command` type union** — three discriminated kinds covering every user-visible `/foo` action (`types/command.ts:175-206`).
- The **command registry** in `commands.ts` — static + conditional imports, ANT-only set, feature-flag gates, plus dynamic skill/plugin/workflow integration.
- **Lazy loading**: `LocalCommand` / `LocalJSXCommand` defer their implementations via `import()`; the heavy `insights` command additionally lazy-loads the prompt builder.
- **Availability gating**: separate from `isEnabled()`. `availability: 'claude-ai' | 'console'` filters by auth/provider mid-session.
- **Remote-safe / bridge-safe allowlists**: explicit allowlists for which commands are permitted to execute over remote sessions (CCR) and the IDE bridge.
- **Dynamic skill insertion**: skills discovered after initial load (e.g., during file operations) are injected into the command list at the right place.

### IN scope
- `src/commands.ts` (754 lines) — registry, gating, allowlists, helpers.
- `src/types/command.ts` (217 lines) — type definitions for `Command`, `PromptCommand`, `LocalCommand`, `LocalJSXCommand`, `CommandAvailability`, `LocalCommandResult`, `LocalJSXCommandContext`, `LocalJSXCommandOnDone`, `ResumeEntrypoint`, `CommandResultDisplay`.
- The integration surface with `skills/`, `plugins/`, `tools/WorkflowTool/`.
- Cross-references to: 21 (per-command catalog), 17 (skill system), 28 (plugin loader), 32 (workflow flag), 34 (bridge safe set).

### OUT of scope
- Per-command behavior, prompts, args → 21.
- Skill discovery internals → 17.
- Plugin manifest format → 28.
- Workflow tool internals → 19.
- Bridge transport → 34.

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/commands.ts` | 754 | Static + conditional command imports; `COMMANDS()` memoized; `getCommands(cwd)`; `loadAllCommands(cwd)` memoized; `getSkills(cwd)`; `getMcpSkillCommands`; `getSkillToolCommands`; `getSlashCommandToolSkills`; `meetsAvailabilityRequirement`; `isBridgeSafeCommand`; `filterCommandsForRemoteMode`; `findCommand` / `hasCommand` / `getCommand`; `formatDescriptionWithSource`; `clearCommandsCache`; `clearCommandMemoizationCaches`; `INTERNAL_ONLY_COMMANDS`, `REMOTE_SAFE_COMMANDS`, `BRIDGE_SAFE_COMMANDS`, `builtInCommandNames` |
| `src/types/command.ts` | 217 | `Command` discriminated union; `PromptCommand`, `LocalCommand`, `LocalJSXCommand`; `CommandBase`; `CommandAvailability`; `LocalCommandResult`, `LocalJSXCommandContext`, `LocalJSXCommandOnDone`; `ResumeEntrypoint`; `CommandResultDisplay`; `getCommandName`, `isCommandEnabled` |

### 2.2 Source coverage

| Source | Read fully | Sampled | Grep-inspected |
|---|---|---|---|
| `src/commands.ts` | ✅ | | |
| `src/types/command.ts` | ✅ | | |
| `src/commands/commit.ts` (PromptCommand sample) | | ✅ (top 40) | |
| `src/commands/init.ts` (feature-gated PromptCommand) | | ✅ (top 40) | |
| `src/commands/cost/index.ts` (LocalCommand sample) | | ✅ (full, ~25 lines) | |
| `src/commands/config/index.ts` (LocalJSXCommand sample) | | ✅ (full, ~10 lines) | |
| Other ~95 command files | | | grep only |

### 2.3 Imports from

`commands.ts` imports:
- 60+ command modules statically (one per built-in command).
- `bun:bundle` — `feature` for build-time DCE.
- `lodash-es/memoize` — for `COMMANDS()`, `loadAllCommands`, `getSkillToolCommands`, `getSlashCommandToolSkills`, `builtInCommandNames`.
- `./skills/loadSkillsDir.js` — `getSkillDirCommands`, `clearSkillCaches`, `getDynamicSkills`.
- `./skills/bundledSkills.js` — `getBundledSkills`.
- `./plugins/builtinPlugins.js` — `getBuiltinPluginSkillCommands`.
- `./utils/plugins/loadPluginCommands.js` — `getPluginCommands`, `clearPluginCommandCache`, `getPluginSkills`, `clearPluginSkillsCache`.
- `./utils/auth.js` — `isUsing3PServices`, `isClaudeAISubscriber`.
- `./utils/model/providers.js` — `isFirstPartyAnthropicBaseUrl`.
- `./utils/log.js` / `./utils/errors.js` / `./utils/debug.js` — `logError`, `toError`, `logForDebugging`.
- `./utils/settings/constants.js` — `getSettingSourceName`.
- `./types/command.js` — `Command`, `getCommandName`, `isCommandEnabled`.

`types/command.ts` imports types from:
- `@anthropic-ai/sdk/resources/index.mjs` — `ContentBlockParam`.
- `crypto` — `UUID`.
- `../hooks/useCanUseTool.js`, `../services/compact/compact.js`, `../services/mcp/types.js`, `../Tool.js`, `../utils/effort.js`, `../utils/ide.js`, `../utils/settings/constants.js`, `../utils/settings/types.js`, `../utils/theme.js`, `./logs.js`, `./message.js`, `./plugin.js`.

### 2.4 Imported by (downstream consumers)

`commands.ts` is imported by:
- `screens/REPL.tsx` (typeahead, command picker).
- `query.ts` (slash-command parsing in user messages — spec 04).
- `tools/SkillTool/` (skill enumeration via `getSkillToolCommands` — spec 17).
- `bridge/` (BRIDGE_SAFE_COMMANDS check — spec 34).
- `remote/` (REMOTE_SAFE_COMMANDS filtering — spec 35).
- `tools.ts` (re-exports `Command` type).

`types/command.ts` is the canonical type source; imported by `Tool.ts`, `commands.ts`, every command module, plus the REPL/UI components that surface commands.

### 2.5 On-Disk Pattern (representative command shapes)

| Pattern | Example | Files |
|---|---|---|
| Single-file PromptCommand | `commands/commit.ts` | one `.ts` exporting a `Command` object with `type: 'prompt'`, `getPromptForCommand` |
| Two-file LocalCommand (lazy load) | `commands/cost/{index.ts, cost.ts}` | `index.ts` exports the metadata + `load: () => import('./cost.js')`; `cost.ts` exports `call` |
| Two-file LocalJSXCommand (lazy load) | `commands/config/{index.ts, config.tsx}` | `index.ts` metadata + `load`; `config.tsx` exports `call` returning `React.ReactNode` |
| Feature-gated PromptCommand | `commands/init.ts` | `feature('NEW_INIT')` toggles between `OLD_INIT_PROMPT` and `NEW_INIT_PROMPT` (verbatim strings inside the file) |
| Single-file LocalCommand (no lazy) | `commands/version.ts`, `commands/advisor.ts` | one `.ts` with `type: 'local'` and inline `load` |
| Multi-file complex command | `commands/agents/`, `commands/mcp/`, `commands/skills/` | dir with `index.ts` metadata + multiple supporting files |
| Dynamic-imported via `feature()` gate | `commands/proactive.ts`, `commands/brief.ts`, `commands/voice/`, `commands/buddy/`, etc. | top-level `const X = feature('FOO') ? require('./X.js').default : null` |

---

## 3. Public Interface (Contract)

### 3.1 Command type union (verbatim from `types/command.ts:205-206`)

```typescript
export type Command = CommandBase &
  (PromptCommand | LocalCommand | LocalJSXCommand)
```

### 3.2 `CommandBase` (verbatim from `types/command.ts:175-203`)

```typescript
export type CommandBase = {
  availability?: CommandAvailability[]
  description: string
  hasUserSpecifiedDescription?: boolean
  /** Defaults to true. Only set when the command has conditional enablement
   * (feature flags, env checks, etc). */
  isEnabled?: () => boolean
  /** Defaults to false. Only set when the command should be hidden from
   * typeahead/help. */
  isHidden?: boolean
  name: string
  aliases?: string[]
  isMcp?: boolean
  argumentHint?: string                         // Hint text for command arguments (gray after command)
  whenToUse?: string                            // From the "Skill" spec. Detailed usage scenarios
  version?: string
  disableModelInvocation?: boolean              // Whether to hide from model-invocable list
  userInvocable?: boolean                       // Whether users can invoke /skill-name
  loadedFrom?:
    | 'commands_DEPRECATED'
    | 'skills'
    | 'plugin'
    | 'managed'
    | 'bundled'
    | 'mcp'
  kind?: 'workflow'                             // Distinguishes workflow-backed commands
  immediate?: boolean                           // Bypasses queue; runs without waiting for stop point
  isSensitive?: boolean                         // Args redacted from conversation history
  /** Defaults to `name`. Only override when displayed name differs (e.g. plugin prefix stripping). */
  userFacingName?: () => string
}
```

### 3.3 `PromptCommand` (verbatim from `types/command.ts:25-57`)

```typescript
export type PromptCommand = {
  type: 'prompt'
  progressMessage: string
  contentLength: number                          // command content length (chars; for token estimation)
  argNames?: string[]
  allowedTools?: string[]
  model?: string
  source: SettingSource | 'builtin' | 'mcp' | 'plugin' | 'bundled'
  pluginInfo?: {
    pluginManifest: PluginManifest
    repository: string
  }
  disableNonInteractive?: boolean
  hooks?: HooksSettings                          // Hooks registered when the skill is invoked
  skillRoot?: string                             // Base dir for skill resources (CLAUDE_PLUGIN_ROOT)
  context?: 'inline' | 'fork'                    // 'inline' (default) or 'fork' (sub-agent)
  agent?: string                                 // Agent type when forked
  effort?: EffortValue
  paths?: string[]                               // Glob patterns; skill visible only after model touches matches
  getPromptForCommand(
    args: string,
    context: ToolUseContext,
  ): Promise<ContentBlockParam[]>
}
```

### 3.4 `LocalCommand` (verbatim from `types/command.ts:62-78`)

```typescript
export type LocalCommandResult =
  | { type: 'text'; value: string }
  | {
      type: 'compact'
      compactionResult: CompactionResult
      displayText?: string
    }
  | { type: 'skip' }                            // Skip messages

export type LocalCommandCall = (
  args: string,
  context: LocalJSXCommandContext,
) => Promise<LocalCommandResult>

export type LocalCommandModule = {
  call: LocalCommandCall
}

type LocalCommand = {
  type: 'local'
  supportsNonInteractive: boolean
  load: () => Promise<LocalCommandModule>       // lazy-load implementation
}
```

### 3.5 `LocalJSXCommand` (verbatim from `types/command.ts:80-152`)

```typescript
export type LocalJSXCommandContext = ToolUseContext & {
  canUseTool?: CanUseToolFn
  setMessages: (updater: (prev: Message[]) => Message[]) => void
  options: {
    dynamicMcpConfig?: Record<string, ScopedMcpServerConfig>
    ideInstallationStatus: IDEExtensionInstallationStatus | null
    theme: ThemeName
  }
  onChangeAPIKey: () => void
  onChangeDynamicMcpConfig?: (
    config: Record<string, ScopedMcpServerConfig>,
  ) => void
  onInstallIDEExtension?: (ide: IdeType) => void
  resume?: (
    sessionId: UUID,
    log: LogOption,
    entrypoint: ResumeEntrypoint,
  ) => Promise<void>
}

export type ResumeEntrypoint =
  | 'cli_flag'
  | 'slash_command_picker'
  | 'slash_command_session_id'
  | 'slash_command_title'
  | 'fork'

export type CommandResultDisplay = 'skip' | 'system' | 'user'

export type LocalJSXCommandOnDone = (
  result?: string,
  options?: {
    display?: CommandResultDisplay
    shouldQuery?: boolean
    metaMessages?: string[]
    nextInput?: string
    submitNextInput?: boolean
  },
) => void

export type LocalJSXCommandCall = (
  onDone: LocalJSXCommandOnDone,
  context: ToolUseContext & LocalJSXCommandContext,
  args: string,
) => Promise<React.ReactNode>

export type LocalJSXCommandModule = {
  call: LocalJSXCommandCall
}

type LocalJSXCommand = {
  type: 'local-jsx'
  load: () => Promise<LocalJSXCommandModule>
}
```

### 3.6 `CommandAvailability` (verbatim from `types/command.ts:155-173`)

```typescript
export type CommandAvailability =
  | 'claude-ai'                                 // claude.ai OAuth subscriber (Pro/Max/Team/Enterprise)
  | 'console'                                   // Direct api.anthropic.com Console API key user
```

### 3.7 Registry functions (signatures from `commands.ts`)

```typescript
const COMMANDS: () => Command[]                  // memoized, builds the static + feature-gated array

export const builtInCommandNames: () => Set<string>  // memoized, names + aliases of all COMMANDS()

export async function getCommands(cwd: string): Promise<Command[]>
// Returns commands available to the current user. Filters by
// meetsAvailabilityRequirement() and isCommandEnabled(); inserts
// dynamic skills between plugin skills and built-ins; runs auth checks
// fresh each call so /login takes effect immediately.

const loadAllCommands: (cwd: string) => Promise<Command[]>     // memoized by cwd

async function getSkills(cwd: string): Promise<{
  skillDirCommands: Command[]
  pluginSkills: Command[]
  bundledSkills: Command[]
  builtinPluginSkills: Command[]
}>
// Loads all four skill sources in parallel; per-source try/catch; never
// throws — failed sources return [].

export function meetsAvailabilityRequirement(cmd: Command): boolean
// Returns true if cmd has no `availability` OR matches at least one of
// the listed auth types. NOT memoized — re-evaluated every getCommands()
// call so /login takes effect mid-session.

export function isBridgeSafeCommand(cmd: Command): boolean
// 'local-jsx' → false (Ink UI); 'prompt' → true (skills are safe by
// construction); 'local' → BRIDGE_SAFE_COMMANDS membership check.

export function filterCommandsForRemoteMode(commands: Command[]): Command[]
// Returns commands ∈ REMOTE_SAFE_COMMANDS only. Pre-filter for --remote
// mode REPL render before CCR init message arrives.

export function findCommand(
  commandName: string,
  commands: Command[],
): Command | undefined
// Match against name OR getCommandName(_) OR aliases.

export function hasCommand(commandName: string, commands: Command[]): boolean

export function getCommand(commandName: string, commands: Command[]): Command
// Throws ReferenceError with sorted available list if not found.

export function formatDescriptionWithSource(cmd: Command): string
// Adds source annotation suffix for typeahead UI. Different format per
// source: workflow → "(workflow)", plugin → "(<plugin-name>) <desc>" or
// "<desc> (plugin)", bundled → "<desc> (bundled)", others → uses
// getSettingSourceName().

export function clearCommandsCache(): void
export function clearCommandMemoizationCaches(): void

export function getMcpSkillCommands(
  mcpCommands: readonly Command[],
): readonly Command[]
// Filters AppState.mcp.commands to MCP-provided model-invocable prompts.
// Gated by feature('MCP_SKILLS'); returns [] when off.

export const getSkillToolCommands: (cwd: string) => Promise<Command[]>
// memoized. Returns ALL prompt-based commands the model can invoke
// (skills + commands), excluding 'builtin' source. Used by SkillTool's
// listing.

export const getSlashCommandToolSkills: (cwd: string) => Promise<Command[]>
// memoized. Filters to skill-typed commands (loadedFrom in
// {skills, plugin, bundled} OR disableModelInvocation set). Wraps in
// try/catch returning [] on failure.

export const REMOTE_SAFE_COMMANDS: Set<Command>      // ~17 entries
export const BRIDGE_SAFE_COMMANDS: Set<Command>      // ~6 entries
export const INTERNAL_ONLY_COMMANDS: Command[]       // ANT-only set
```

Re-exported from `types/command.js`:
```typescript
export type { Command, CommandBase, CommandResultDisplay, LocalCommandResult,
              LocalJSXCommandContext, PromptCommand, ResumeEntrypoint }
export { getCommandName, isCommandEnabled }
```

---

## 4. Data Model & State

### 4.1 Helper functions (verbatim from `types/command.ts:209-216`)

```typescript
export function getCommandName(cmd: CommandBase): string {
  return cmd.userFacingName?.() ?? cmd.name
}

export function isCommandEnabled(cmd: CommandBase): boolean {
  return cmd.isEnabled?.() ?? true
}
```

### 4.2 Memoization caches

`commands.ts` keeps four `lodash-es/memoize` caches:

| Memoized | Key | Purpose | Cleared by |
|---|---|---|---|
| `COMMANDS` | none (single value) | Static + feature-gated command array | `loadAllCommands.cache?.clear?.()` is independent; this one is rebuilt on module reload only |
| `builtInCommandNames` | none | Set of names + aliases | not cleared individually |
| `loadAllCommands` | `cwd` | Skills/plugins/workflows loaded from disk + built-ins | `clearCommandMemoizationCaches()` |
| `getSkillToolCommands` | `cwd` | Filtered subset for SkillTool | `clearCommandMemoizationCaches()` |
| `getSlashCommandToolSkills` | `cwd` | Filtered subset for slash-command skills | `clearCommandMemoizationCaches()` |

A separate cache, `getSkillIndex` in `services/skillSearch/localSearch.ts`, is built ON TOP of these. `clearSkillIndexCache?.()` (gated by `feature('EXPERIMENTAL_SKILL_SEARCH')`) must be called explicitly when the inner caches are cleared (`commands.ts:527-531`).

### 4.3 Persistent state

The Command subsystem is otherwise **stateless**. Skill discovery state lives in `skills/loadSkillsDir.ts` (spec 17); plugin command state in `utils/plugins/loadPluginCommands.ts` (spec 28); workflow command state in `tools/WorkflowTool/createWorkflowCommand.ts` (spec 19).

---

## 5. Algorithm / Control Flow

### 5.1 `COMMANDS()` build (memoized, `commands.ts:258-346`)

```
function COMMANDS() -> Command[]:
  return [
    addDir, advisor, agents, branch, btw, chrome, clear, color,
    compact, config, copy, desktop, context, contextNonInteractive,
    cost, diff, doctor, effort, exit, fast, files, heapDump, help,
    ide, init, keybindings, installGitHubApp, installSlackApp, mcp,
    memory, mobile, model, outputStyle, remoteEnv, plugin, pr_comments,
    releaseNotes, reloadPlugins, rename, resume, session, skills, stats,
    status, statusline, stickers, tag, theme, feedback, review,
    ultrareview, rewind, securityReview, terminalSetup, upgrade,
    extraUsage, extraUsageNonInteractive, rateLimitOptions, usage,
    usageReport, vim,

    if webCmd:                  webCmd                 // feature('CCR_REMOTE_SETUP')
    if forkCmd:                 forkCmd                // feature('FORK_SUBAGENT')
    if buddy:                   buddy                  // feature('BUDDY')
    if proactive:               proactive              // feature('PROACTIVE') || feature('KAIROS')
    if briefCommand:            briefCommand           // feature('KAIROS') || feature('KAIROS_BRIEF')
    if assistantCommand:        assistantCommand       // feature('KAIROS')
    if bridge:                  bridge                 // feature('BRIDGE_MODE')
    if remoteControlServerCmd:  remoteControlServerCmd // feature('DAEMON') && feature('BRIDGE_MODE')
    if voiceCommand:            voiceCommand           // feature('VOICE_MODE')

    thinkback, thinkbackPlay, permissions, plan, privacySettings, hooks,
    exportCommand, sandboxToggle,

    if !isUsing3PServices():    logout, login()        // hide /login,/logout for 3P providers

    passes,
    if peersCmd:                peersCmd               // feature('UDS_INBOX')
    tasks,
    if workflowsCmd:            workflowsCmd           // feature('WORKFLOW_SCRIPTS')
    if torch:                   torch                  // feature('TORCH')

    if USER_TYPE === 'ant' && !IS_DEMO:
                                ...INTERNAL_ONLY_COMMANDS
  ]
```

### 5.2 `INTERNAL_ONLY_COMMANDS` (verbatim list from `commands.ts:225-254`)

```typescript
export const INTERNAL_ONLY_COMMANDS = [
  backfillSessions,
  breakCache,
  bughunter,
  commit,
  commitPushPr,
  ctx_viz,
  goodClaude,
  issue,
  initVerifiers,
  ...(forceSnip ? [forceSnip] : []),         // feature('HISTORY_SNIP')
  mockLimits,
  bridgeKick,
  version,
  ...(ultraplan ? [ultraplan] : []),         // feature('ULTRAPLAN')
  ...(subscribePr ? [subscribePr] : []),     // feature('KAIROS_GITHUB_WEBHOOKS')
  resetLimits,
  resetLimitsNonInteractive,
  onboarding,
  share,
  summary,
  teleport,
  antTrace,
  perfIssue,
  env,
  oauthRefresh,
  debugToolCall,
  agentsPlatform,                            // ANT-only, top-level require
  autofixPr,
].filter(Boolean)
```

These commands are added to `COMMANDS()` only when `process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO`.

### 5.3 `loadAllCommands(cwd)` (memoized, `commands.ts:449-469`)

```
loadAllCommands(cwd) -> Command[]:
  parallel:
    [skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills] = getSkills(cwd)
    pluginCommands                                  = getPluginCommands()
    workflowCommands                                = getWorkflowCommands ? getWorkflowCommands(cwd) : []

  return [
    ...bundledSkills,
    ...builtinPluginSkills,
    ...skillDirCommands,
    ...workflowCommands,
    ...pluginCommands,
    ...pluginSkills,
    ...COMMANDS(),
  ]
```

The order is **bundled → built-in plugin → user skill dirs → workflows → plugins → plugin skills → built-in commands**. This determines which command "wins" when names collide (insertion order).

### 5.4 `getSkills(cwd)` (defensive parallel load, `commands.ts:353-398`)

```
getSkills(cwd) -> {skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills}:
  try:
    parallel:
      skillDirCommands = getSkillDirCommands(cwd) catch err -> { logError; return [] }
      pluginSkills     = getPluginSkills() catch err -> { logError; return [] }
    bundledSkills        = getBundledSkills()                  // sync, registered at startup
    builtinPluginSkills  = getBuiltinPluginSkillCommands()
    return { skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills }
  catch err:
    logError(err)
    return { skillDirCommands: [], pluginSkills: [], bundledSkills: [], builtinPluginSkills: [] }
```

Key invariants:
- **Per-source try/catch** so one failed source does not break the others.
- **Outer try/catch** as defense-in-depth (the inner `Promise.all` already catches per-promise).
- Failure is **swallowed and logged**; the function returns empty arrays. Skill loading is non-critical to the harness.

### 5.5 `getCommands(cwd)` (`commands.ts:476-517`)

```
getCommands(cwd) -> Command[]:
  allCommands  = loadAllCommands(cwd)             // memoized
  dynamicSkills = getDynamicSkills()              // skills discovered during file ops

  baseCommands = allCommands.filter(_ ->
    meetsAvailabilityRequirement(_) && isCommandEnabled(_))

  if dynamicSkills.length === 0:
    return baseCommands

  baseCommandNames = Set(baseCommands.map(c -> c.name))
  uniqueDynamicSkills = dynamicSkills.filter(s ->
    !baseCommandNames.has(s.name) &&
    meetsAvailabilityRequirement(s) &&
    isCommandEnabled(s))

  if uniqueDynamicSkills.length === 0:
    return baseCommands

  builtInNames = Set(COMMANDS().map(c -> c.name))
  insertIndex  = baseCommands.findIndex(c -> builtInNames.has(c.name))

  if insertIndex === -1:
    return [...baseCommands, ...uniqueDynamicSkills]

  return [
    ...baseCommands.slice(0, insertIndex),
    ...uniqueDynamicSkills,
    ...baseCommands.slice(insertIndex),
  ]
```

Key invariants:
- **Availability re-evaluated every call** (`meetsAvailabilityRequirement` is NOT memoized) so that auth changes (`/login`) take effect immediately.
- **Dynamic skills are deduped by name** against the base set.
- **Dynamic skills inserted before the first built-in command** so they appear after plugin skills but before built-ins (preserving the insertion-order priority from §5.3).

### 5.6 `meetsAvailabilityRequirement` (`commands.ts:417-443`)

```
meetsAvailabilityRequirement(cmd) -> boolean:
  if !cmd.availability: return true              // universal
  for a in cmd.availability:
    switch a:
      case 'claude-ai':
        if isClaudeAISubscriber(): return true
      case 'console':
        // Console API key user = direct 1P API customer (not 3P, not claude.ai).
        // Excludes Bedrock/Vertex/Foundry (3P) and gateway users.
        if !isClaudeAISubscriber() &&
           !isUsing3PServices() &&
           isFirstPartyAnthropicBaseUrl():
          return true
  return false
```

### 5.7 Remote / bridge safety logic

```
isBridgeSafeCommand(cmd) -> boolean:
  if cmd.type === 'local-jsx': return false      // Ink UI; cannot stream over bridge
  if cmd.type === 'prompt':    return true       // skills expand to text; safe by construction
  return BRIDGE_SAFE_COMMANDS.has(cmd)            // 'local' commands need explicit opt-in

filterCommandsForRemoteMode(commands) -> Command[]:
  return commands.filter(cmd -> REMOTE_SAFE_COMMANDS.has(cmd))
```

`REMOTE_SAFE_COMMANDS` and `BRIDGE_SAFE_COMMANDS` are explicit allowlists; the default for both is **deny**.

### 5.8 Lazy `usageReport` (heavy command, `commands.ts:189-202`)

```typescript
// insights.ts is 113KB (3200 lines, includes diffLines/html rendering). Lazy
// shim defers the heavy module until /insights is actually invoked.
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

This pattern (declaring command metadata in `commands.ts` while the implementation is dynamically imported) is reserved for genuinely heavy modules. Standard `LocalCommand` / `LocalJSXCommand` already lazy-load via the `load: () => import(...)` field.

### 5.9 `findCommand` / `getCommand` (lookup, `commands.ts:688-719`)

```
findCommand(commandName, commands) -> Command | undefined:
  return commands.find(c ->
    c.name === commandName ||
    getCommandName(c) === commandName ||             // userFacingName override
    c.aliases?.includes(commandName)
  )

getCommand(commandName, commands) -> Command:        // throws if not found
  c = findCommand(commandName, commands)
  if !c:
    throw ReferenceError(`Command ${commandName} not found. Available commands: ${
      commands.map(c -> {
        name = getCommandName(c)
        return c.aliases ? `${name} (aliases: ${c.aliases.join(', ')})` : name
      }).sort((a,b) -> a.localeCompare(b)).join(', ')
    }`)
  return c
```

The error includes the full sorted available-command list — useful for debugging typeahead misses.

### 5.10 `formatDescriptionWithSource` (`commands.ts:728-754`)

Display annotation rules for typeahead and help screens:

```
formatDescriptionWithSource(cmd) -> string:
  if cmd.type !== 'prompt':       return cmd.description
  if cmd.kind === 'workflow':     return `${cmd.description} (workflow)`
  if cmd.source === 'plugin':
    pluginName = cmd.pluginInfo?.pluginManifest.name
    if pluginName:                return `(${pluginName}) ${cmd.description}`
    return                        `${cmd.description} (plugin)`
  if cmd.source ∈ ['builtin', 'mcp']: return cmd.description
  if cmd.source === 'bundled':    return `${cmd.description} (bundled)`
  return                          `${cmd.description} (${getSettingSourceName(cmd.source)})`
```

For model-facing prompts (e.g., `SkillTool`), use `cmd.description` directly without the source suffix.

---

## 6. Verbatim Assets

### 6.1 ANT import-order banner (`commands.ts:1`)

```typescript
// biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered
```

### 6.2 `REMOTE_SAFE_COMMANDS` (verbatim from `commands.ts:619-637`)

```typescript
export const REMOTE_SAFE_COMMANDS: Set<Command> = new Set([
  session,    // Shows QR code / URL for remote session
  exit,       // Exit the TUI
  clear,      // Clear screen
  help,       // Show help
  theme,      // Change terminal theme
  color,      // Change agent color
  vim,        // Toggle vim mode
  cost,       // Show session cost (local cost tracking)
  usage,      // Show usage info
  copy,       // Copy last message
  btw,        // Quick note
  feedback,   // Send feedback
  plan,       // Plan mode toggle
  keybindings,// Keybinding management
  statusline, // Status line toggle
  stickers,   // Stickers
  mobile,     // Mobile QR code
])
```

### 6.3 `BRIDGE_SAFE_COMMANDS` (verbatim from `commands.ts:651-660`)

```typescript
export const BRIDGE_SAFE_COMMANDS: Set<Command> = new Set(
  [
    compact,       // Shrink context — useful mid-session from a phone
    clear,         // Wipe transcript
    cost,          // Show session cost
    summary,       // Summarize conversation
    releaseNotes,  // Show changelog
    files,         // List tracked files
  ].filter((c): c is Command => c !== null),
)
```

The `.filter((c): c is Command => c !== null)` guard handles null entries that would arise if any of the listed commands were feature-gated to `null`. Currently `summary` is in `INTERNAL_ONLY_COMMANDS` so could be null in non-ANT builds; the filter keeps the set well-typed.

### 6.4 Lazy `usageReport` shim (verbatim from `commands.ts:189-202`)

See §5.8.

### 6.5 ESLint disable comments

Around feature-gated `require()` blocks (`commands.ts:60-63`, `:123`):
```typescript
/* eslint-disable @typescript-eslint/no-require-imports */
```

Around the ANT-only `agentsPlatform` require (`commands.ts:47`):
```typescript
/* eslint-disable @typescript-eslint/no-require-imports */
```

Note: `commands.ts` does **not** use `custom-rules/no-process-env-top-level` because `commands.ts:49` reads `process.env.USER_TYPE` inside an immediately-evaluated `require()` ternary, which the codebase's lint rule does not flag (see overview Appendix A #4).

### 6.6 Type-only re-exports (verbatim from `commands.ts:213-222`)

```typescript
export type {
  Command,
  CommandBase,
  CommandResultDisplay,
  LocalCommandResult,
  LocalJSXCommandContext,
  PromptCommand,
  ResumeEntrypoint,
} from './types/command.js'
export { getCommandName, isCommandEnabled } from './types/command.js'
```

These keep `commands.ts` as the public face of the registry while the canonical type source remains `types/command.ts`.

---

## 7. Side Effects & I/O

The Command subsystem performs the following I/O (mostly during skill/plugin discovery):

| Source | Effect | Owning subsystem |
|---|---|---|
| Skill directory walks | filesystem read of `.claude/skills/` and parent CLAUDE.md chain | spec 17 (skills) |
| Plugin manifest reads | filesystem read of `.claude/plugins/` configured paths | spec 28 (plugins) |
| Workflow scripts | filesystem read of workflow `.json` files | spec 19 (workflow tool) |
| Bundled skills | in-process synchronous registration (no I/O) | spec 17 |
| MCP-provided skills | network/IPC to MCP servers | spec 23 (MCP) |
| Lazy command modules | dynamic `import()` of compiled JS chunks | runtime per command |

Top-level reads of `process.env`:
- `USER_TYPE === 'ant'` and `!IS_DEMO` for `INTERNAL_ONLY_COMMANDS` inclusion (`commands.ts:343`).
- Inline ANT check inside `agentsPlatform` (`commands.ts:49`).

No top-level network or filesystem activity occurs at module-evaluation time; all expensive work is deferred to `loadAllCommands()` on first call.

---

## 8. Feature Flags & Variants

### 8.1 `feature()` gates affecting the registry

| Flag | Effect | Site |
|---|---|---|
| `PROACTIVE` ∨ `KAIROS` | adds `/proactive` | `commands.ts:62-65` |
| `KAIROS` ∨ `KAIROS_BRIEF` | adds `/brief` | `commands.ts:66-69` |
| `KAIROS` | adds `/assistant`, `INTERNAL_ONLY_COMMANDS` includes `subscribePr` only when `KAIROS_GITHUB_WEBHOOKS` | `commands.ts:70-72` |
| `BRIDGE_MODE` | adds `/bridge` | `commands.ts:73-75` |
| `DAEMON` ∧ `BRIDGE_MODE` | adds `/remoteControlServer` | `commands.ts:76-79` |
| `VOICE_MODE` | adds `/voice` | `commands.ts:80-82` |
| `HISTORY_SNIP` | adds `/force-snip` to `INTERNAL_ONLY_COMMANDS` | `commands.ts:83-85`, `:235` |
| `WORKFLOW_SCRIPTS` | adds `/workflows` and the `getWorkflowCommands` loader for dynamic workflow commands | `commands.ts:86-90`, `:401-405` |
| `CCR_REMOTE_SETUP` | adds `/remote-setup` | `commands.ts:91-95` |
| `EXPERIMENTAL_SKILL_SEARCH` | wires `clearSkillIndexCache` for the skill search index | `commands.ts:96-100`, `:531` |
| `KAIROS_GITHUB_WEBHOOKS` | adds `/subscribe-pr` to `INTERNAL_ONLY_COMMANDS` | `commands.ts:101-103`, `:240` |
| `ULTRAPLAN` | adds `/ultraplan` to `INTERNAL_ONLY_COMMANDS` | `commands.ts:104-106`, `:239` |
| `TORCH` | adds `/torch` | `commands.ts:107` |
| `UDS_INBOX` | adds `/peers` | `commands.ts:108-112` |
| `FORK_SUBAGENT` | adds `/fork` | `commands.ts:113-117` |
| `BUDDY` | adds `/buddy` | `commands.ts:118-122` |
| `WORKFLOW_SCRIPTS` (in catalog loader) | enables workflow command dynamic load | `commands.ts:401-405` |
| `MCP_SKILLS` | enables MCP-provided model-invocable prompts as skills | `commands.ts:550` |

### 8.2 Non-`feature()` runtime gates

| Gate | Effect | Site |
|---|---|---|
| `USER_TYPE === 'ant'` (top-level require) | `agentsPlatform` command | `commands.ts:48-51` |
| `USER_TYPE === 'ant' && !IS_DEMO` | `INTERNAL_ONLY_COMMANDS` spread into `COMMANDS()` | `commands.ts:343-345` |
| `isUsing3PServices()` | hides `/login` and `/logout` for 3P (Bedrock/Vertex/Foundry) | `commands.ts:337` |
| `isClaudeAISubscriber()` | gates `availability: 'claude-ai'` | `commands.ts:421-422` |
| `!isClaudeAISubscriber() && !isUsing3PServices() && isFirstPartyAnthropicBaseUrl()` | gates `availability: 'console'` | `commands.ts:425-432` |

### 8.3 ANT-only paths affecting commands

| Concern | Site | Behavior |
|---|---|---|
| `agentsPlatform` top-level require | `commands.ts:47-51` | Only ANT |
| `INTERNAL_ONLY_COMMANDS` spread into list | `commands.ts:343` | ANT && !IS_DEMO |
| Per-command ANT logic | (varies) | Documented in spec 21 catalog per command |

### 8.4 Variants table

| Variant | Command-list delta vs production default |
|---|---|
| `USER_TYPE === 'ant'` | + `INTERNAL_ONLY_COMMANDS` (29 entries when all flags on) |
| `IS_DEMO=1` | suppresses the ANT spread (used for demo/screenshot workflows) |
| 3P provider (Bedrock/Vertex/Foundry) | − `/login`, − `/logout` |
| `claude-ai` subscriber | + commands with `availability: ['claude-ai']` |
| Console API customer (1P) | + commands with `availability: ['console']` |
| Custom base URL gateway | excluded from `'console'` availability |

---

## 9. Error Handling & Edge Cases

| Failure mode | Behavior |
|---|---|
| Skill source throws (per-source `Promise.all` catch in `getSkills`) | `logError`; `logForDebugging` "<source> failed to load, continuing without them"; returns `[]` for that source |
| Outer skill load throws (defense in depth in `getSkills`) | `logError`; `logForDebugging` "Unexpected error in getSkills, returning empty"; returns all-empty struct |
| `getSlashCommandToolSkills` throws | `logError`; `logForDebugging` "Returning empty skills array due to load failure"; returns `[]` |
| Command not found (`getCommand`) | Throws `ReferenceError(\`Command ${name} not found. Available commands: <sorted list>\`)` |
| Duplicate names between dynamic skills and base | Dynamic skill DROPPED (filtered by `!baseCommandNames.has(s.name)`); base wins |
| Plugin without `pluginManifest.name` in `formatDescriptionWithSource` | Falls back to `${desc} (plugin)` instead of `(<name>) <desc>` |

User-facing error strings owned by individual commands (spec 21).

---

## 10. Telemetry & Observability

| Point | Site |
|---|---|
| `logError(toError(err))` on per-source skill load failure | `commands.ts:362, 369` |
| `logForDebugging('Skill directory commands failed to load, continuing without them')` | `commands.ts:363-365` |
| `logForDebugging('Plugin skills failed to load, continuing without them')` | `commands.ts:370` |
| `logForDebugging(\`getSkills returning: ${...counts}\`)` | `commands.ts:378-380` |
| `logForDebugging('Returning empty skills array due to load failure')` | `commands.ts:604` |

No metrics / OTel spans at this layer; per-command emission is owned by individual commands (spec 21).

---

## 11. Reimplementation Checklist

A reimplementer of `commands.ts` + `types/command.ts` must preserve:

- [ ] `Command` is a discriminated union: `CommandBase & (PromptCommand | LocalCommand | LocalJSXCommand)`. The discriminator field is `type: 'prompt' | 'local' | 'local-jsx'`.
- [ ] `LocalCommand` and `LocalJSXCommand` lazy-load via `load: () => import('...')` returning a module with `call`. The metadata stays in the small `index.ts` so module evaluation cost is paid only when invoked.
- [ ] `PromptCommand.getPromptForCommand` returns `Promise<ContentBlockParam[]>` (Anthropic SDK shape) — not just a string.
- [ ] `PromptCommand.contentLength` is hand-set to the prompt's character count for token estimation. Heavy commands (e.g., insights) use the lazy shim pattern with `contentLength: 0` because the real prompt is loaded on demand.
- [ ] `PromptCommand.allowedTools` is an array of permission-rule patterns (e.g., `'Bash(git add:*)'`); commit/diff-style commands use this to pre-allow tool calls without prompting.
- [ ] `PromptCommand.context: 'inline' | 'fork'` controls whether the skill expands inline or forks a sub-agent.
- [ ] `PromptCommand.paths` (glob patterns) makes a skill conditionally visible after the model touches matching files (dynamic skill activation — spec 17).
- [ ] `LocalCommandResult` is a discriminated union: `text | compact | skip`. The `compact` type carries a `CompactionResult` (spec 07).
- [ ] `LocalJSXCommandOnDone(result?, options?)` callback supports `display: 'skip'|'system'|'user'`, `shouldQuery`, `metaMessages`, `nextInput`, `submitNextInput`. These shape how the result re-enters the conversation.
- [ ] `CommandAvailability = 'claude-ai' | 'console'`. `'console'` excludes 3P providers AND custom base URLs (gateway users) — must include `isFirstPartyAnthropicBaseUrl()` check.
- [ ] `meetsAvailabilityRequirement` is NOT memoized so `/login` takes effect mid-session.
- [ ] Loading order in `loadAllCommands(cwd)`: bundled skills, builtin plugin skills, user skill dirs, workflow commands, plugin commands, plugin skills, built-in `COMMANDS()`. Insertion order resolves name collisions.
- [ ] Dynamic skills (`getDynamicSkills()`) are inserted **before the first built-in command** so they appear after plugin skills but before built-ins. Deduped against `baseCommandNames` by name.
- [ ] Per-source skill loading uses inner `Promise.all` with per-source `.catch(err -> logError; return [])` AND an outer `try/catch` returning all-empty. One source failing does NOT prevent others.
- [ ] `INTERNAL_ONLY_COMMANDS` is spread into `COMMANDS()` only when `USER_TYPE === 'ant' && !IS_DEMO`. The `IS_DEMO` exclusion is intentional for demos.
- [ ] `REMOTE_SAFE_COMMANDS` and `BRIDGE_SAFE_COMMANDS` are explicit allowlists; default is **deny**. `local-jsx` commands are blocked from bridge by type (Ink UI cannot stream); `prompt` commands are bridge-safe by construction; `local` commands need explicit opt-in.
- [ ] `BRIDGE_SAFE_COMMANDS` filter clause `.filter((c): c is Command => c !== null)` keeps the set well-typed when entries come from feature-gated nullable consts.
- [ ] Heavy command pattern: large modules (e.g., `insights.ts` ~113KB / 3200 lines) define a thin `Command` shim in `commands.ts` whose `getPromptForCommand` does a dynamic `import()` of the real module on first use.
- [ ] `getCommand` throws `ReferenceError` with full sorted available-command list — NOT a generic "not found" error.
- [ ] Memoization caches use `lodash-es/memoize`; clears via `cache.clear?.()`. `clearCommandsCache()` clears in/skill/plugin/workflow/skillSearch index in coordinated order: `loadAllCommands`, `getSkillToolCommands`, `getSlashCommandToolSkills`, `clearSkillIndexCache?.()`, `clearPluginCommandCache()`, `clearPluginSkillsCache()`, `clearSkillCaches()`.
- [ ] `getSkillIndex` (in `services/skillSearch/localSearch.ts`) is a separate memoization layer ON TOP of these inner caches. Clearing inner alone is insufficient — must call `clearSkillIndexCache?.()` (gated by `feature('EXPERIMENTAL_SKILL_SEARCH')`).
- [ ] `formatDescriptionWithSource` is for **user-facing UI only**. Model-facing prompts (e.g., SkillTool listing) MUST use `cmd.description` directly to avoid leaking source annotations into the model's context.
- [ ] `findCommand` matches against `name`, `getCommandName(cmd)` (userFacingName override), and `aliases` — three checks per candidate.
- [ ] `getMcpSkillCommands` filters by `cmd.type === 'prompt' && cmd.loadedFrom === 'mcp' && !cmd.disableModelInvocation`; gated by `feature('MCP_SKILLS')` (returns `[]` when off).
- [ ] `getSkillToolCommands` includes `cmd.type === 'prompt' && !cmd.disableModelInvocation && cmd.source !== 'builtin'`, plus the loadedFrom acceptance set: `bundled`, `skills`, `commands_DEPRECATED`, OR (`hasUserSpecifiedDescription` || `whenToUse`) for plugin/MCP.
- [ ] `getSlashCommandToolSkills` is a stricter filter: `loadedFrom ∈ {skills, plugin, bundled}` OR `disableModelInvocation` set; with try/catch returning `[]` on failure.
- [ ] `.js` import suffix; ANT import-order banner at `commands.ts:1`; conditional `require()` for build-time DCE; ESLint disables only for `@typescript-eslint/no-require-imports` in this file (no `custom-rules/no-process-env-top-level`).
- [ ] Re-export of types from `commands.ts` (kept for back-compat with consumers who import `Command` from the registry rather than the type module).

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited.

1. ~~**`insights.ts` size claim**~~ — **RESOLVED Phase 9.7**: spec 21b (ANT command catalog) confirms `insights.ts` size; lazy-shim comment is consistent. No drift detected.
2. ~~**`SettingSource` value set**~~ — **RESOLVED Phase 9.7**: spec 02 (settings) §3 enumerates the union as `'managed' | 'user' | 'project' | 'local' | 'session'` (from `utils/settings/constants.ts`). Combined with the `'builtin' | 'mcp' | 'plugin' | 'bundled'` literal additions in `PromptCommand.source`.
3. ~~**`LogOption`**~~ — **RESOLVED Phase 9.7**: spec 26 (analytics) §6 enumerates `LogOption` enum from `services/analytics/logs.ts`; consumer-side defined.
4. ~~**`CompactionResult` shape**~~ — **RESOLVED Phase 9.7**: spec 07 (compaction) §4 documents `CompactionResult { messages, originalTokens, finalTokens, summary }` from `services/compact/`.
5. ~~**`HooksSettings`**~~ — **RESOLVED Phase 9.7**: spec 02 §3 and spec 09 §4 jointly own; `HooksSettings` is `Record<HookEvent, HookCommand[]>` — see spec 02's §3.6 schema.
6. ~~**Dynamic-skill discovery trigger**~~ — **RESOLVED Phase 9.7**: spec 17 §5 documents the `dynamicSkillDirTriggers` Set populated by file-op tools (Read/Edit) when matching SKILL.md is in path. Trigger evaluation runs at turn boundary in `query()` (spec 03).
7. ~~**`pluginManifest` shape**~~ — **RESOLVED Phase 9.7**: spec 28 (plugins) §3 enumerates `PluginManifest` from `types/plugin.ts`.
8. ~~**`workflowCommands` cycle / freshness**~~ — **RESOLVED Phase 9.7**: spec 28 §5 confirms `/reload-plugins` invalidates the memoized `loadAllCommands` cache (via `clearAllCaches`); workflows reload correctly.
9. ~~**`isMcp` field redundancy**~~ — **NOTE Phase 9.7**: confirmed back-compat field; `loadedFrom: 'mcp'` is the canonical source-of-truth. Spec 23 (service-mcp) recommends new code key off `loadedFrom`.
10. ~~**Internal-only vs universal command split**~~ — **RESOLVED Phase 9.7**: spec 21b §1 documents the split rationale — `INTERNAL_ONLY_COMMANDS` are commands gated by `USER_TYPE === 'ant'` at registration time; commands in universal `COMMANDS()` graduated to public via deliberate review. `commit` remains internal-only (uses `isUndercover()`-aware attribution); `pr_comments` graduated. No further unknown.
