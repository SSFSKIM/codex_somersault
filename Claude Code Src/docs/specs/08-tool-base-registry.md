# 08 — Tool Base & Registry Specification

> The Tool interface, the registry, and the on-disk pattern that every concrete tool follows. Anchor for specs 10-19. Read 00-overview before this.

---

## 1. Purpose & Scope

The Tool subsystem defines:
- The **`Tool<Input, Output, P>` interface** — the single contract every callable capability implements (`Tool.ts:362-695`).
- The **`buildTool()` factory** with `TOOL_DEFAULTS` for safe defaults (`Tool.ts:783-792`).
- The **`ToolUseContext`** god object threaded into every tool call (`Tool.ts:158-300`, ~74 leaf fields).
- The **registry assembly pipeline** in `tools.ts`: `getAllBaseTools` → `getTools` → `assembleToolPool` → `getMergedTools`.
- The **on-disk pattern** for tool directories under `src/tools/<Name>/`.

### IN scope
- `src/Tool.ts` (792 lines), `src/tools.ts` (389 lines), `src/tools/utils.ts`.
- The Tool interface, ToolDef, BuiltTool, TOOL_DEFAULTS, registry functions, helper utilities.
- The on-disk convention shared by all `src/tools/<Name>/` directories.
- Cross-references to: 09 (permission system), 04 (turn pipeline calling tools), 03 (query engine constructing context).

### OUT of scope
- Permission decision tree → 09.
- Per-tool implementations → 10..19.
- Tool result rendering details for the UI shell → 37.
- MCP tool dynamic registration → 23.
- Skill-as-prompt-command surface → 17, 20.

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/Tool.ts` | 792 | Type definitions: `Tool`, `ToolDef`, `BuiltTool`, `ToolUseContext`, `ToolPermissionContext`, `ValidationResult`, `ToolResult`, `Progress`; `TOOL_DEFAULTS` constant; `buildTool` factory; `findToolByName` / `toolMatchesName` helpers |
| `src/tools.ts` | 389 | Static + conditional tool imports; `getAllBaseTools()`; `getTools(permissionContext)`; `assembleToolPool(permissionContext, mcpTools)`; `getMergedTools(permissionContext, mcpTools)`; `filterToolsByDenyRules`; `parseToolPreset`; `TOOL_PRESETS`; `getToolsForDefaultPreset` |
| `src/tools/utils.ts` | ~30 | Shared utilities (small) |
| `src/types/permissions.ts` | ~100+ | `PermissionMode`, `PermissionRule*`, `PermissionResult` (re-exported by Tool.ts; primary owner is 09) |

### 2.2 Source coverage

| Source | Read fully | Sampled | Grep-inspected | Notes |
|---|---|---|---|---|
| `src/Tool.ts` | ✅ | | | All 792 lines read in Phase 0 |
| `src/tools.ts` | ✅ | | | All 389 lines read in Phase 0 |
| `src/tools/utils.ts` | | ✅ (size) | | Tiny; deferred to surface scan during sub-spec drafting if relied on |
| `src/types/permissions.ts` | | ✅ (top 120) | | Phase 0 covered the mode/rule definitions; rest owned by 09 |

### 2.3 Imports from (top-level external + internal)

`Tool.ts` imports types from:
- `@anthropic-ai/sdk/resources/index.mjs` — `ToolResultBlockParam`, `ToolUseBlockParam`
- `@modelcontextprotocol/sdk/types.js` — `ElicitRequestURLParams`, `ElicitResult`
- `crypto` — `UUID`
- `zod/v4` — `z`
- `./commands.js` — `Command` (re-export back from registry; circular-safe via type-only)
- `./hooks/useCanUseTool.js` — `CanUseToolFn`
- `./utils/thinking.js` — `ThinkingConfig`
- `./context/notifications.js` — `Notification`
- `./services/mcp/types.js` — `MCPServerConnection`, `ServerResource`
- `./tools/AgentTool/loadAgentsDir.js` — `AgentDefinition`, `AgentDefinitionsResult`
- `./types/message.js` — `Message`, `UserMessage`, `AssistantMessage`, `AttachmentMessage`, `ProgressMessage`, `SystemMessage`, `SystemLocalCommandMessage` ⚠️ **file not present in `src/types/`** — see §12 missing-source
- `./types/permissions.js` — `AdditionalWorkingDirectory`, `PermissionMode`, `PermissionResult`, `ToolPermissionRulesBySource`
- `./types/tools.js` — `AgentToolProgress`, `BashProgress`, `MCPProgress`, `REPLToolProgress`, `SkillToolProgress`, `TaskOutputProgress`, `ToolProgressData`, `WebSearchProgress` ⚠️ **file not present** — see §12 missing-source
- `./utils/fileStateCache.js` — `FileStateCache`
- `./utils/permissions/denialTracking.js` — `DenialTrackingState`
- `./utils/systemPromptType.js` — `SystemPrompt`
- `./utils/toolResultStorage.js` — `ContentReplacementState`
- `./components/Spinner.js` — `SpinnerMode`
- `./constants/querySource.js` — `QuerySource`
- `./entrypoints/agentSdkTypes.js` — `SDKStatus`
- `./state/AppState.js` — `AppState`
- `./types/hooks.js` — `HookProgress`, `PromptRequest`, `PromptResponse`
- `./types/ids.js` — `AgentId`
- `./types/utils.js` — `DeepImmutable`
- `./utils/commitAttribution.js` — `AttributionState`
- `./utils/fileHistory.js` — `FileHistoryState`
- `./utils/theme.js` — `Theme`, `ThemeName`

`tools.ts` imports each tool module statically OR via `require()` behind a feature gate. Lazy `require()` getters break circular deps for AgentTool's coordinator loop:
- `getTeamCreateTool` (line 63), `getTeamDeleteTool` (line 66), `getSendMessageTool` (line 69)

Imports `feature` from `bun:bundle` (line 104) — the canonical DCE entry point.

### 2.4 Imported by (downstream consumers)

`Tool.ts` is imported by every tool dir's main file (e.g., `BashTool.tsx`, `FileReadTool.ts`, `AgentTool.tsx`) plus:
- `query.ts` (turn pipeline — spec 04)
- `QueryEngine.ts` (spec 03)
- `hooks/toolPermission/` (spec 09)
- `hooks/useCanUseTool.ts`
- `services/mcp/` (MCP tool wrapping)
- `coordinator/` (multi-agent — spec 30)

`tools.ts` is imported by:
- `query.ts` and `QueryEngine.ts` (to assemble the toolset for an API call)
- `hooks/useMergedTools` (REPL hook)
- `tools/AgentTool/runAgent.ts` (coordinator workers receive a per-agent toolset)
- `screens/REPL.tsx` (initial render setup)

### 2.5 On-Disk Pattern (representative tool directories)

Every tool lives in `src/tools/<Name>/`. Sampled patterns (Phase 1):

#### `src/tools/BashTool/` (most complex; 18 files)
```
BashTool.tsx                    main: tool definition via buildTool()
BashToolResultMessage.tsx       result rendering
UI.tsx                          tool-use rendering
prompt.ts                       system prompt + getDefaultTimeoutMs / getMaxTimeoutMs / getSimplePrompt
toolName.ts                     BASH_TOOL_NAME constant
bashPermissions.ts              permission predicates (bashToolHasPermission, etc.)
bashSecurity.ts                 security guards
commandSemantics.ts             interpretCommandResult
commentLabel.ts                 comment-pattern handling
destructiveCommandWarning.ts    destructive operation warnings
modeValidation.ts               mode-aware validation
pathValidation.ts               path validation
readOnlyValidation.ts           checkReadOnlyConstraints (for plan/read-only modes)
sedEditParser.ts                parseSedEditCommand (sed-based edits)
sedValidation.ts                sed validation
shouldUseSandbox.ts             sandbox decision
bashCommandHelpers.ts           helpers
utils.ts                        misc utils (resetCwdIfOutsideProject, image output, etc.)
```

#### `src/tools/FileReadTool/` (compact; 5 files)
```
FileReadTool.ts                 main
prompt.ts                       system prompt
UI.tsx                          rendering
imageProcessor.ts               image resize/format/compression
limits.ts                       size/page caps
```

#### `src/tools/AgentTool/` (mid-complexity; 15 files)
```
AgentTool.tsx                   main
UI.tsx                          rendering
prompt.ts                       system prompt
runAgent.ts                     subagent execution loop
forkSubagent.ts                 fork-mode spawn (for parent-cache sharing)
resumeAgent.ts                  resume from sidechain records
loadAgentsDir.ts                discover AgentDefinition from disk
builtInAgents.ts                explore/plan/etc. built-ins (gated by BUILTIN_EXPLORE_PLAN_AGENTS)
built-in/                       individual built-in agent files (e.g., exploreAgent.ts)
agentColorManager.ts            color assignment for transcript display
agentDisplay.ts                 display utilities
agentMemory.ts                  per-agent memory
agentMemorySnapshot.ts          memory snapshot (gated by AGENT_MEMORY_SNAPSHOT)
agentToolUtils.ts               misc
constants.ts                    constants
```

### 2.6 Convention summary (derived from sample)

For any `src/tools/<Name>/` dir, the sub-agent writing 10..19 should expect:
- **Always**: `<Name>Tool.tsx` or `<Name>Tool.ts` — main tool definition via `buildTool({...})`.
- **Almost always**: `prompt.ts` — system prompt content; `UI.tsx` — render functions.
- **Frequently**: `utils.ts`, `constants.ts`, validation/parser files for tool-specific logic.
- **Sometimes**: subdirs (`built-in/`, `bundled/`) for inlined assets or sub-modules.

---

## 3. Public Interface (Contract)

### 3.1 The `Tool` Type (verbatim from `Tool.ts:362-695`)

The Tool interface is the contract every callable capability implements. Below is the complete shape, abridged to the field signatures (full docstrings preserved in source). Sub-agents in 10..19 inline the full annotated form for their own subsystem.

```typescript
export type Tool<
  Input extends AnyObject = AnyObject,
  Output = unknown,
  P extends ToolProgressData = ToolProgressData,
> = {
  // Identity
  readonly name: string
  aliases?: string[]
  searchHint?: string                                    // for ToolSearch keyword matching
  userFacingName(input: Partial<z.infer<Input>> | undefined): string

  // Schema
  readonly inputSchema: Input                            // Zod schema (or)
  readonly inputJSONSchema?: ToolInputJSONSchema         // raw JSONSchema (MCP path)
  outputSchema?: z.ZodType<unknown>

  // Lifecycle predicates
  isEnabled(): boolean
  isConcurrencySafe(input: z.infer<Input>): boolean
  isReadOnly(input: z.infer<Input>): boolean
  isDestructive?(input: z.infer<Input>): boolean         // defaults false
  isOpenWorld?(input: z.infer<Input>): boolean
  requiresUserInteraction?(): boolean
  isMcp?: boolean
  isLsp?: boolean
  inputsEquivalent?(a: z.infer<Input>, b: z.infer<Input>): boolean
  interruptBehavior?(): 'cancel' | 'block'               // defaults 'block'

  // Deferred loading & cache hints
  readonly shouldDefer?: boolean                          // ToolSearch path
  readonly alwaysLoad?: boolean                           // forces inclusion in initial prompt
  mcpInfo?: { serverName: string; toolName: string }      // for MCP tools
  readonly maxResultSizeChars: number                     // disk-overflow threshold
  readonly strict?: boolean                               // tengu_tool_pear strict mode

  // Validation & permission
  validateInput?(
    input: z.infer<Input>,
    context: ToolUseContext,
  ): Promise<ValidationResult>
  checkPermissions(
    input: z.infer<Input>,
    context: ToolUseContext,
  ): Promise<PermissionResult>
  preparePermissionMatcher?(
    input: z.infer<Input>,
  ): Promise<(pattern: string) => boolean>
  getPath?(input: z.infer<Input>): string

  // Observability hooks
  backfillObservableInput?(input: Record<string, unknown>): void
  toAutoClassifierInput(input: z.infer<Input>): unknown

  // Prompt + description
  prompt(options: {
    getToolPermissionContext: () => Promise<ToolPermissionContext>
    tools: Tools
    agents: AgentDefinition[]
    allowedAgentTypes?: string[]
  }): Promise<string>
  description(
    input: z.infer<Input>,
    options: {
      isNonInteractiveSession: boolean
      toolPermissionContext: ToolPermissionContext
      tools: Tools
    },
  ): Promise<string>

  // Execution
  call(
    args: z.infer<Input>,
    context: ToolUseContext,
    canUseTool: CanUseToolFn,
    parentMessage: AssistantMessage,
    onProgress?: ToolCallProgress<P>,
  ): Promise<ToolResult<Output>>

  // Visual classification
  isSearchOrReadCommand?(input: z.infer<Input>): {
    isSearch: boolean
    isRead: boolean
    isList?: boolean
  }
  isTransparentWrapper?(): boolean
  userFacingNameBackgroundColor?(
    input: Partial<z.infer<Input>> | undefined,
  ): keyof Theme | undefined
  getToolUseSummary?(input: Partial<z.infer<Input>> | undefined): string | null
  getActivityDescription?(
    input: Partial<z.infer<Input>> | undefined,
  ): string | null
  isResultTruncated?(output: Output): boolean

  // Result mapping
  mapToolResultToToolResultBlockParam(
    content: Output,
    toolUseID: string,
  ): ToolResultBlockParam
  extractSearchText?(out: Output): string                  // transcript search index

  // Rendering (Ink/React) — most are optional
  renderToolUseMessage(
    input: Partial<z.infer<Input>>,
    options: { theme: ThemeName; verbose: boolean; commands?: Command[] },
  ): React.ReactNode
  renderToolUseTag?(input: Partial<z.infer<Input>>): React.ReactNode
  renderToolUseProgressMessage?(
    progressMessagesForMessage: ProgressMessage<P>[],
    options: {
      tools: Tools
      verbose: boolean
      terminalSize?: { columns: number; rows: number }
      inProgressToolCallCount?: number
      isTranscriptMode?: boolean
    },
  ): React.ReactNode
  renderToolUseQueuedMessage?(): React.ReactNode
  renderToolUseRejectedMessage?(input, options): React.ReactNode
  renderToolUseErrorMessage?(result, options): React.ReactNode
  renderToolResultMessage?(content, progressMessages, options): React.ReactNode
  renderGroupedToolUse?(toolUses, options): React.ReactNode | null
}

export type Tools = readonly Tool[]
```

### 3.2 `ToolDef` and `BuiltTool<D>` (factory input/output types)

```typescript
type DefaultableToolKeys =
  | 'isEnabled' | 'isConcurrencySafe' | 'isReadOnly'
  | 'isDestructive' | 'checkPermissions'
  | 'toAutoClassifierInput' | 'userFacingName'

export type ToolDef<Input, Output, P> =
  Omit<Tool<Input, Output, P>, DefaultableToolKeys> &
  Partial<Pick<Tool<Input, Output, P>, DefaultableToolKeys>>

type BuiltTool<D> = Omit<D, DefaultableToolKeys> & {
  [K in DefaultableToolKeys]-?: K extends keyof D
    ? undefined extends D[K] ? ToolDefaults[K] : D[K]
    : ToolDefaults[K]
}

export function buildTool<D extends AnyToolDef>(def: D): BuiltTool<D>
```

`buildTool()` is the single canonical entry. Every tool is constructed via this factory; direct object literals would lose the safe defaults.

### 3.3 Registry functions (verbatim signatures from `tools.ts`)

```typescript
export function getAllBaseTools(): Tools
// Exhaustive list. Order is invariant — must match StatSig
// claude_code_global_system_caching config (tools.ts:191).

export function filterToolsByDenyRules<T extends {
  name: string
  mcpInfo?: { serverName: string; toolName: string }
}>(tools: readonly T[], permissionContext: ToolPermissionContext): T[]

export const getTools: (permissionContext: ToolPermissionContext) => Tools
// = base list filtered by deny rules + REPL gate + isEnabled().
// Special-cases CLAUDE_CODE_SIMPLE → [BashTool, FileReadTool, FileEditTool] (+ COORDINATOR_MODE adds AgentTool/TaskStop/SendMessage).

export function assembleToolPool(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools,
): Tools
// Built-ins (sorted) + MCP tools (sorted) → uniqBy 'name' (built-in wins).
// Partition stability is required by claude_code_system_cache_policy server config.

export function getMergedTools(
  permissionContext: ToolPermissionContext,
  mcpTools: Tools,
): Tools
// Flat concat without sort or dedupe; for token counting and tool-search
// thresholds where MCP tools must be visible.

export function parseToolPreset(preset: string): ToolPreset | null
export function getToolsForDefaultPreset(): string[]
export const TOOL_PRESETS = ['default'] as const
```

Re-exports from `constants/tools.js`: `ALL_AGENT_DISALLOWED_TOOLS`, `CUSTOM_AGENT_DISALLOWED_TOOLS`, `ASYNC_AGENT_ALLOWED_TOOLS`, `COORDINATOR_MODE_ALLOWED_TOOLS`, `REPL_ONLY_TOOLS` (re-exported from `tools/REPLTool/constants.js`).

### 3.4 Helper functions in `Tool.ts`

```typescript
export function toolMatchesName(
  tool: { name: string; aliases?: string[] },
  name: string,
): boolean
// True if name matches primary or any alias.

export function findToolByName(tools: Tools, name: string): Tool | undefined

export function filterToolProgressMessages(
  progressMessagesForMessage: ProgressMessage[],
): ProgressMessage<ToolProgressData>[]

export const getEmptyToolPermissionContext: () => ToolPermissionContext
```

---

## 4. Data Model & State

### 4.1 `ToolUseContext` (verbatim from `Tool.ts:158-300`, the god-object)

74 leaf fields. Reproduced compactly; see `Tool.ts:158` for full annotations.

```typescript
export type ToolUseContext = {
  options: {
    commands: Command[]
    debug: boolean
    mainLoopModel: string
    tools: Tools
    verbose: boolean
    thinkingConfig: ThinkingConfig
    mcpClients: MCPServerConnection[]
    mcpResources: Record<string, ServerResource[]>
    isNonInteractiveSession: boolean
    agentDefinitions: AgentDefinitionsResult
    maxBudgetUsd?: number
    customSystemPrompt?: string
    appendSystemPrompt?: string
    querySource?: QuerySource
    refreshTools?: () => Tools
  }
  abortController: AbortController
  readFileState: FileStateCache
  getAppState(): AppState
  setAppState(f: (prev: AppState) => AppState): void
  setAppStateForTasks?: (f: (prev: AppState) => AppState) => void  // session-scoped infra
  handleElicitation?: (                                              // MCP -32042 elicitations
    serverName: string,
    params: ElicitRequestURLParams,
    signal: AbortSignal,
  ) => Promise<ElicitResult>
  setToolJSX?: SetToolJSXFn
  addNotification?: (notif: Notification) => void
  appendSystemMessage?: (msg: Exclude<SystemMessage, SystemLocalCommandMessage>) => void
  sendOSNotification?: (opts: { message: string; notificationType: string }) => void
  nestedMemoryAttachmentTriggers?: Set<string>
  loadedNestedMemoryPaths?: Set<string>                              // CLAUDE.md re-injection dedup
  dynamicSkillDirTriggers?: Set<string>
  discoveredSkillNames?: Set<string>                                 // skill_discovery telemetry
  userModified?: boolean
  setInProgressToolUseIDs: (f: (prev: Set<string>) => Set<string>) => void
  setHasInterruptibleToolInProgress?: (v: boolean) => void           // REPL only
  setResponseLength: (f: (prev: number) => number) => void
  pushApiMetricsEntry?: (ttftMs: number) => void                     // ANT-only OTPS
  setStreamMode?: (mode: SpinnerMode) => void
  onCompactProgress?: (event: CompactProgressEvent) => void
  setSDKStatus?: (status: SDKStatus) => void
  openMessageSelector?: () => void
  updateFileHistoryState: (updater) => void
  updateAttributionState: (updater) => void
  setConversationId?: (id: UUID) => void
  agentId?: AgentId                                                  // subagent identity
  agentType?: string
  requireCanUseTool?: boolean                                        // speculation overlay
  messages: Message[]
  fileReadingLimits?: { maxTokens?: number; maxSizeBytes?: number }
  globLimits?: { maxResults?: number }
  toolDecisions?: Map<string, {
    source: string
    decision: 'accept' | 'reject'
    timestamp: number
  }>
  queryTracking?: QueryChainTracking
  requestPrompt?: (
    sourceName: string,
    toolInputSummary?: string | null,
  ) => (request: PromptRequest) => Promise<PromptResponse>
  toolUseId?: string
  criticalSystemReminder_EXPERIMENTAL?: string
  preserveToolUseResults?: boolean                                   // in-process teammate transcripts
  localDenialTracking?: DenialTrackingState                          // async subagent denial counter
  contentReplacementState?: ContentReplacementState                  // tool result budget
  renderedSystemPrompt?: SystemPrompt                                // fork subagent cache sharing
}
```

### 4.2 `ToolPermissionContext` (verbatim from `Tool.ts:123-138`)

> **Canonical owner**: `src/Tool.ts:123-138` (this file). A no-runtime-deps **mirror** exists at `src/types/permissions.ts:427-441` for breaking import cycles and intentionally omits `isAutoModeAvailable`. Permission-system semantics (rule matching, dialog, classifier) belong to spec 09; this spec owns the type's exported shape.

```typescript
export type ToolPermissionContext = DeepImmutable<{
  mode: PermissionMode
  additionalWorkingDirectories: Map<string, AdditionalWorkingDirectory>
  alwaysAllowRules: ToolPermissionRulesBySource
  alwaysDenyRules: ToolPermissionRulesBySource
  alwaysAskRules: ToolPermissionRulesBySource
  isBypassPermissionsModeAvailable: boolean
  isAutoModeAvailable?: boolean                                      // set at permissionSetup.ts:987 when TC enabled
  strippedDangerousRules?: ToolPermissionRulesBySource
  shouldAvoidPermissionPrompts?: boolean                             // background agents
  awaitAutomatedChecksBeforeDialog?: boolean                         // coordinator workers
  prePlanMode?: PermissionMode                                       // restored on plan exit
}>
```

`getEmptyToolPermissionContext()` returns the canonical empty value:
```typescript
{
  mode: 'default',
  additionalWorkingDirectories: new Map(),
  alwaysAllowRules: {},
  alwaysDenyRules: {},
  alwaysAskRules: {},
  isBypassPermissionsModeAvailable: false,
}
```

Detailed semantics in spec 09.

### 4.3 `ValidationResult` and `ToolResult<T>`

```typescript
export type ValidationResult =
  | { result: true }
  | { result: false; message: string; errorCode: number }

export type ToolResult<T> = {
  data: T
  newMessages?: (
    | UserMessage
    | AssistantMessage
    | AttachmentMessage
    | SystemMessage
  )[]
  contextModifier?: (context: ToolUseContext) => ToolUseContext      // non-concurrency-safe only
  mcpMeta?: {
    _meta?: Record<string, unknown>
    structuredContent?: Record<string, unknown>
  }
}
```

### 4.4 Progress types

```typescript
export type Progress = ToolProgressData | HookProgress
export type ToolProgress<P extends ToolProgressData> = {
  toolUseID: string
  data: P
}
export type ToolCallProgress<P extends ToolProgressData = ToolProgressData> =
  (progress: ToolProgress<P>) => void
```

`ToolProgressData` is a union from `types/tools.js` containing per-tool progress shapes (`AgentToolProgress`, `BashProgress`, `MCPProgress`, `REPLToolProgress`, `SkillToolProgress`, `TaskOutputProgress`, `WebSearchProgress`). The `types/tools.ts` source file is **not present at the expected path** — see §12.

### 4.5 `CompactProgressEvent`

```typescript
export type CompactProgressEvent =
  | { type: 'hooks_start'; hookType: 'pre_compact' | 'post_compact' | 'session_start' }
  | { type: 'compact_start' }
  | { type: 'compact_end' }
```

### 4.6 `QueryChainTracking`

```typescript
export type QueryChainTracking = {
  chainId: string
  depth: number
}
```

### 4.7 `SetToolJSXFn`

```typescript
export type SetToolJSXFn = (args: {
  jsx: React.ReactNode | null
  shouldHidePromptInput: boolean
  shouldContinueAnimation?: true
  showSpinner?: boolean
  isLocalJSXCommand?: boolean
  isImmediate?: boolean
  clearLocalJSX?: boolean
} | null) => void
```

### 4.8 In-memory state

The Tool subsystem is itself **stateless** — `Tool` instances are immutable after construction. Per-call state lives in the `ToolUseContext` and the `AppState` Zustand store (spec 41). Permission state is in `ToolPermissionContext` (immutable, replaced on update).

There is no persistent on-disk state owned by spec 08 directly; tool overflow files (`maxResultSizeChars`) are written by `utils/toolResultStorage.js` and owned by spec 04.

---

## 5. Algorithm / Control Flow

### 5.1 Registry assembly pipeline

Pseudocode of the full pipeline, faithful to `tools.ts:179-389`:

```
function getToolsForDefaultPreset() -> string[]:
  tools = getAllBaseTools()
  enabled = tools.map(t -> t.isEnabled())
  return tools.filter((_, i) -> enabled[i]).map(t -> t.name)

function getAllBaseTools() -> Tools:
  return [
    AgentTool,
    TaskOutputTool,
    BashTool,
    if !hasEmbeddedSearchTools(): GlobTool, GrepTool   // ANT bun bundle has bfs/ugrep
    ExitPlanModeV2Tool,
    FileReadTool, FileEditTool, FileWriteTool, NotebookEditTool,
    WebFetchTool,
    TodoWriteTool, WebSearchTool, TaskStopTool,
    AskUserQuestionTool, SkillTool, EnterPlanModeTool,
    if USER_TYPE === 'ant': ConfigTool, TungstenTool
    if SuggestBackgroundPRTool: SuggestBackgroundPRTool   // ANT-only via top-level require
    if WebBrowserTool: WebBrowserTool                     // feature('WEB_BROWSER_TOOL')
    if isTodoV2Enabled(): TaskCreateTool, TaskGetTool, TaskUpdateTool, TaskListTool
    if OverflowTestTool: OverflowTestTool                 // feature('OVERFLOW_TEST_TOOL')
    if CtxInspectTool: CtxInspectTool                     // feature('CONTEXT_COLLAPSE')
    if TerminalCaptureTool: TerminalCaptureTool           // feature('TERMINAL_PANEL')
    if envTruthy(ENABLE_LSP_TOOL): LSPTool
    if isWorktreeModeEnabled(): EnterWorktreeTool, ExitWorktreeTool
    getSendMessageTool(),                                  // lazy require
    if ListPeersTool: ListPeersTool                       // feature('UDS_INBOX')
    if isAgentSwarmsEnabled(): getTeamCreateTool(), getTeamDeleteTool()  // lazy require
    if VerifyPlanExecutionTool: VerifyPlanExecutionTool   // env CLAUDE_CODE_VERIFY_PLAN === 'true'
    if USER_TYPE === 'ant' && REPLTool: REPLTool          // ANT-only
    if WorkflowTool: WorkflowTool                         // feature('WORKFLOW_SCRIPTS')
    if SleepTool: SleepTool                               // feature('PROACTIVE') || feature('KAIROS')
    ...cronTools                                           // feature('AGENT_TRIGGERS') → 3 tools
    if RemoteTriggerTool: RemoteTriggerTool               // feature('AGENT_TRIGGERS_REMOTE')
    if MonitorTool: MonitorTool                           // feature('MONITOR_TOOL')
    BriefTool,
    if SendUserFileTool: SendUserFileTool                 // feature('KAIROS')
    if PushNotificationTool: PushNotificationTool         // feature('KAIROS') || feature('KAIROS_PUSH_NOTIFICATION')
    if SubscribePRTool: SubscribePRTool                   // feature('KAIROS_GITHUB_WEBHOOKS')
    if getPowerShellTool(): getPowerShellTool()           // isPowerShellToolEnabled()
    if SnipTool: SnipTool                                 // feature('HISTORY_SNIP')
    if NODE_ENV === 'test': TestingPermissionTool
    ListMcpResourcesTool, ReadMcpResourceTool,
    if isToolSearchEnabledOptimistic(): ToolSearchTool
  ]
```

The order is hand-maintained and **must match the upstream StatSig `claude_code_global_system_caching` config** (`tools.ts:191-192`). Reordering invalidates the global system prompt cache for every user.

**Tools NOT in the flat `getAllBaseTools()` array but still real:**
- `AgentOutputTool`, `BashOutputTool` — name **aliases** declared on `TaskOutputTool` via `aliases: ['AgentOutputTool','BashOutputTool']` (see spec 15 §3 / `TaskOutputTool.ts:184`); the registry resolves them to the same tool object.
- `McpAuthTool` — created **per server on demand** by `createMcpAuthTool(serverName, config)` (spec 16 §3); never statically listed.
- `ReviewArtifactTool` — flag-gated tool (`feature('REVIEW_ARTIFACT')`) registered through the **permission-UI surface** (`src/components/permissions/PermissionRequest.tsx:36`), NOT through `tools.ts`. See spec 19 §0 inventory and the `REVIEW_ARTIFACT` row in spec 00 §8.1.B.

### 5.2 `getTools(permissionContext)`

```
function getTools(permissionContext) -> Tools:
  if envTruthy(CLAUDE_CODE_SIMPLE):
    if isReplModeEnabled() && REPLTool:
      simple = [REPLTool]
      if feature('COORDINATOR_MODE') && coordinatorModeModule.isCoordinatorMode():
        simple.push(TaskStopTool, getSendMessageTool())
      return filterToolsByDenyRules(simple, permissionContext)
    simple = [BashTool, FileReadTool, FileEditTool]
    if feature('COORDINATOR_MODE') && coordinatorModeModule.isCoordinatorMode():
      simple.push(AgentTool, TaskStopTool, getSendMessageTool())
    return filterToolsByDenyRules(simple, permissionContext)

  specialTools = { ListMcpResourcesTool.name, ReadMcpResourceTool.name, SYNTHETIC_OUTPUT_TOOL_NAME }
  tools = getAllBaseTools().filter(t -> !specialTools.has(t.name))
  allowed = filterToolsByDenyRules(tools, permissionContext)

  if isReplModeEnabled():
    if allowed contains REPL_TOOL_NAME:
      allowed = allowed.filter(t -> !REPL_ONLY_TOOLS.has(t.name))

  return allowed.filter(t -> t.isEnabled())
```

Notes:
- **Special tools removed**: `ListMcpResourcesTool`, `ReadMcpResourceTool`, `SYNTHETIC_OUTPUT_TOOL_NAME`. These are added back via different paths (MCP resource tools are conditionally added by callers; `SyntheticOutputTool` is only the name reference here).
- **REPL mode strips primitives**: When `REPLTool` is present, primitives in `REPL_ONLY_TOOLS` are removed from direct exposure (still callable from inside the REPL VM).

### 5.3 `assembleToolPool(permissionContext, mcpTools)`

```
function assembleToolPool(permissionContext, mcpTools) -> Tools:
  builtIn = getTools(permissionContext)
  allowedMcp = filterToolsByDenyRules(mcpTools, permissionContext)
  byName = (a, b) -> a.name.localeCompare(b.name)
  return uniqBy(
    [...builtIn].sort(byName).concat(allowedMcp.sort(byName)),
    'name'
  )
```

**Why partition-then-sort, not flat-sort**: The server-side `claude_code_system_cache_policy` places the global cache breakpoint after the last prefix-matched built-in tool. A flat sort would interleave MCP tools into built-ins and invalidate all downstream cache keys whenever an MCP tool sorted between existing built-ins. `uniqBy` preserves insertion order so name conflicts resolve to built-in (`tools.ts:354-365`).

`Array.toSorted` (Node 20+) is intentionally avoided because the codebase supports Node 18 (`tools.ts:360-361`). `[...builtIn].sort(byName)` is the canonical pattern; `allowedMcp.sort(byName)` mutates the fresh `.filter()` result, which is acceptable.

### 5.4 `getMergedTools` and `filterToolsByDenyRules`

```
function getMergedTools(permissionContext, mcpTools) -> Tools:
  return [...getTools(permissionContext), ...mcpTools]
// Flat concat, no sort, no dedupe. Used for token counting and ToolSearch
// thresholds where MCP tools must be counted but cache stability is not the
// concern.

function filterToolsByDenyRules(tools, permissionContext) -> Tools:
  return tools.filter(t -> !getDenyRuleForTool(permissionContext, t))
// Uses the same matcher as the runtime permission check (step 1a in spec 09),
// so MCP server-prefix rules like `mcp__server` strip all tools from that
// server BEFORE the model sees them, not just at call time.
```

### 5.5 `buildTool` factory algorithm

```
TOOL_DEFAULTS = {
  isEnabled: () -> true,
  isConcurrencySafe: (_input?) -> false,         // assume not safe
  isReadOnly: (_input?) -> false,                // assume writes
  isDestructive: (_input?) -> false,
  checkPermissions: (input, _ctx?) ->
    Promise.resolve({ behavior: 'allow', updatedInput: input }),
  toAutoClassifierInput: (_input?) -> '',         // skip classifier
  userFacingName: (_input?) -> '',                // overridden below to def.name
}

function buildTool(def) -> Tool:
  return { ...TOOL_DEFAULTS, userFacingName: () -> def.name, ...def }
```

Defaults are **fail-closed where it matters**: `isConcurrencySafe = false`, `isReadOnly = false`, `checkPermissions = allow` (the general permission system in `utils/permissions/permissions.ts` is the authoritative gate; tool-specific logic only adds restrictions).

### 5.6 Lazy `require()` for circular-dep breaking

`tools.ts` imports `TeamCreateTool`, `TeamDeleteTool`, `SendMessageTool` lazily because each transitively imports `tools.ts` again (via `coordinator/`). The pattern (`tools.ts:62-72`):

```typescript
const getTeamCreateTool = () =>
  require('./tools/TeamCreateTool/TeamCreateTool.js')
    .TeamCreateTool as typeof import('./tools/TeamCreateTool/TeamCreateTool.js').TeamCreateTool
```

The `as typeof import(...)` cast preserves type information without actually evaluating the module at import time. ESLint disables: `@typescript-eslint/no-require-imports`.

### 5.7 Conditional require for build-time strip

The pattern (used ~15× in `tools.ts`):

```typescript
const SleepTool = feature('PROACTIVE') || feature('KAIROS')
  ? require('./tools/SleepTool/SleepTool.js').SleepTool
  : null
```

The `bun:bundle` `feature()` resolver evaluates the condition at build time. The `require` is only emitted in builds where the feature is on; in builds where it is off, the entire branch (including the `require`) is statically eliminated. **Equivalent dynamic `import()` does NOT strip** — must not be substituted.

### 5.8 ToolSearch / `defer_loading` discipline

Tools with `shouldDefer === true` are sent to the API with `defer_loading: true` so their full schema does not appear in the initial prompt; the model must call `ToolSearchTool` first to discover them. Tools with `alwaysLoad === true` override `shouldDefer` and always appear initially.

For MCP tools, `_meta['anthropic/alwaysLoad']` on the tool's MCP definition sets `alwaysLoad`.

`ToolSearchTool` itself is conditionally added at the tail of `getAllBaseTools()` only when `isToolSearchEnabledOptimistic()` returns true. The actual `defer_loading` decision is made at request time in `claude.ts` (spec 22).

---

## 6. Verbatim Assets

### 6.1 `TOOL_DEFAULTS` (verbatim from `Tool.ts:757-769`)

```typescript
const TOOL_DEFAULTS = {
  isEnabled: () => true,
  isConcurrencySafe: (_input?: unknown) => false,
  isReadOnly: (_input?: unknown) => false,
  isDestructive: (_input?: unknown) => false,
  checkPermissions: (
    input: { [key: string]: unknown },
    _ctx?: ToolUseContext,
  ): Promise<PermissionResult> =>
    Promise.resolve({ behavior: 'allow', updatedInput: input }),
  toAutoClassifierInput: (_input?: unknown) => '',
  userFacingName: (_input?: unknown) => '',
}
```

### 6.2 ANT import-order rule banner (verbatim from `tools.ts:1`)

```typescript
// biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered
```

This banner must appear at the head of files where conditional `require()` blocks are interleaved with static imports. The bundler relies on textual position to strip ANT-only imports correctly. **Do not remove or reorder.**

### 6.3 `TOOL_PRESETS` (verbatim from `tools.ts:161-170`)

```typescript
export const TOOL_PRESETS = ['default'] as const
export type ToolPreset = (typeof TOOL_PRESETS)[number]

export function parseToolPreset(preset: string): ToolPreset | null {
  const presetString = preset.toLowerCase()
  if (!TOOL_PRESETS.includes(presetString as ToolPreset)) {
    return null
  }
  return presetString as ToolPreset
}
```

Currently only `'default'`; the type is open for future expansion.

### 6.4 Special tool name sets

- `REPL_ONLY_TOOLS` — re-exported from `tools/REPLTool/constants.js`. Tools hidden from direct exposure when REPL mode is on.
- `ALL_AGENT_DISALLOWED_TOOLS`, `CUSTOM_AGENT_DISALLOWED_TOOLS`, `ASYNC_AGENT_ALLOWED_TOOLS`, `COORDINATOR_MODE_ALLOWED_TOOLS` — from `constants/tools.js`. Govern which tools are available in different agent contexts. Spec 14 (Agent/Team) consumes these.

### 6.5 Cache invariant comments (verbatim — these are load-bearing)

`tools.ts:191-192`:
```
NOTE: This MUST stay in sync with https://console.statsig.com/4aF3Ewatb6xPVpCwxb5nA3/dynamic_configs/claude_code_global_system_caching, in order to cache the system prompt across users.
```

`tools.ts:354-365`:
```
// Sort each partition for prompt-cache stability, keeping built-ins as a
// contiguous prefix. The server's claude_code_system_cache_policy places a
// global cache breakpoint after the last prefix-matched built-in tool; a flat
// sort would interleave MCP tools into built-ins and invalidate all downstream
// cache keys whenever an MCP tool sorts between existing built-ins. uniqBy
// preserves insertion order, so built-ins win on name conflict.
// Avoid Array.toSorted (Node 20+) — we support Node 18. builtInTools is
// readonly so copy-then-sort; allowedMcpTools is a fresh .filter() result.
```

### 6.6 ESLint disable patterns (file-shape-dependent)

- For top-level ANT env reads + conditional `require()` blocks (e.g., `tools.ts:15-16`):
  ```typescript
  /* eslint-disable custom-rules/no-process-env-top-level, @typescript-eslint/no-require-imports */
  ```
- For lazy-getter `require()` only (e.g., `tools.ts:62`):
  ```typescript
  /* eslint-disable @typescript-eslint/no-require-imports */
  ```
- For `feature()`-gated `require()` only (e.g., `tools.ts:106`):
  ```typescript
  /* eslint-disable custom-rules/no-process-env-top-level, @typescript-eslint/no-require-imports */
  ```

The pattern is: include only the disables actually needed for the file's specific shape (see overview Appendix A #4).

---

## 7. Side Effects & I/O

The Tool subsystem itself performs **no I/O** — it is pure data and types. Side effects are carried out by individual tool implementations (specs 10..19) and the call site (spec 04).

The only direct effects in this spec's scope:
- Build-time DCE evaluation of `feature(...)` calls (Bun bundler).
- Top-level reads of `process.env.USER_TYPE`, `process.env.NODE_ENV`, `process.env.CLAUDE_CODE_VERIFY_PLAN`, `process.env.ENABLE_LSP_TOOL`, `process.env.CLAUDE_CODE_SIMPLE` — at module-evaluation time, evaluated by `bun:bundle` for DCE where the variable is build-time known.
- Lazy `require()` evaluation at first call site.

---

## 8. Feature Flags & Variants

### 8.1 `feature()` gates affecting the registry (in `tools.ts`)

| Flag | Effect | Site |
|---|---|---|
| `MONITOR_TOOL` | adds `MonitorTool` (currently absent dir) | `tools.ts:39-41`, `:237` |
| `KAIROS` | enables `SendUserFileTool` and contributes to `PushNotificationTool` | `tools.ts:42-44`, `:46-49` |
| `KAIROS_PUSH_NOTIFICATION` | enables `PushNotificationTool` independently | `tools.ts:46-49` |
| `KAIROS_GITHUB_WEBHOOKS` | enables `SubscribePRTool` | `tools.ts:50-52` |
| `PROACTIVE` ∨ `KAIROS` | enables `SleepTool` | `tools.ts:25-28`, `:234` |
| `AGENT_TRIGGERS` | enables `CronCreateTool`, `CronDeleteTool`, `CronListTool` | `tools.ts:29-35`, `:235` |
| `AGENT_TRIGGERS_REMOTE` | enables `RemoteTriggerTool` | `tools.ts:36-38`, `:236` |
| `OVERFLOW_TEST_TOOL` | enables `OverflowTestTool` | `tools.ts:107-109`, `:221` |
| `CONTEXT_COLLAPSE` | enables `CtxInspectTool` | `tools.ts:110-112`, `:222` |
| `TERMINAL_PANEL` | enables `TerminalCaptureTool` | `tools.ts:113-116`, `:223` |
| `WEB_BROWSER_TOOL` | enables `WebBrowserTool` | `tools.ts:117-119`, `:217` |
| `COORDINATOR_MODE` | activates coordinator mode tool overlays in simple mode | `tools.ts:120-122`, `:280-285`, `:292-297` |
| `HISTORY_SNIP` | enables `SnipTool` | `tools.ts:123-125`, `:243` |
| `UDS_INBOX` | enables `ListPeersTool` | `tools.ts:126-128`, `:227` |
| `WORKFLOW_SCRIPTS` | enables `WorkflowTool` (initBundledWorkflows on first eval) | `tools.ts:129-134`, `:233` |

### 8.2 Non-`feature()` runtime gates

| Gate | Effect | Site |
|---|---|---|
| `process.env.USER_TYPE === 'ant'` (top-level require) | `REPLTool`, `SuggestBackgroundPRTool` | `tools.ts:16-19`, `:20-24` |
| `process.env.USER_TYPE === 'ant'` (in array) | `ConfigTool`, `TungstenTool`, `REPLTool` inclusion | `tools.ts:214`, `:215`, `:232` |
| `process.env.CLAUDE_CODE_VERIFY_PLAN === 'true'` | `VerifyPlanExecutionTool` | `tools.ts:91-95`, `:231` |
| `process.env.NODE_ENV === 'test'` | `TestingPermissionTool` | `tools.ts:244` |
| `process.env.CLAUDE_CODE_SIMPLE` truthy | reduces toolset to bash/read/edit (+REPL/coordinator delta) | `tools.ts:273-298` |
| `process.env.ENABLE_LSP_TOOL` truthy | enables `LSPTool` | `tools.ts:224` |
| `hasEmbeddedSearchTools()` | drops `GlobTool`, `GrepTool` (ANT bun bundle has bfs/ugrep) | `tools.ts:201` |
| `isTodoV2Enabled()` | adds `TaskCreateTool`, `TaskGetTool`, `TaskUpdateTool`, `TaskListTool` | `tools.ts:218-220` |
| `isToolSearchEnabledOptimistic()` | adds `ToolSearchTool` at tail | `tools.ts:249` |
| `isPowerShellToolEnabled()` | adds `PowerShellTool` | `tools.ts:151-155`, `:242` |
| `isReplModeEnabled()` | strips `REPL_ONLY_TOOLS` after wrapping | `tools.ts:314-323` |
| `isWorktreeModeEnabled()` | adds `EnterWorktreeTool`, `ExitWorktreeTool` | `tools.ts:225` |
| `isAgentSwarmsEnabled()` | adds `TeamCreateTool`, `TeamDeleteTool` | `tools.ts:228-230` |

### 8.3 Variants table

| Variant | Tool delta vs production default |
|---|---|
| `USER_TYPE === 'ant'` | + `ConfigTool`, `TungstenTool`, `REPLTool`, `SuggestBackgroundPRTool` |
| `CLAUDE_CODE_SIMPLE=1` | toolset reduced to `[BashTool, FileReadTool, FileEditTool]` (+ delta if coordinator/REPL active) |
| `ENABLE_LSP_TOOL=1` | + `LSPTool` |
| `NODE_ENV=test` | + `TestingPermissionTool` |
| `CLAUDE_CODE_VERIFY_PLAN=true` | + `VerifyPlanExecutionTool` |
| Embedded search bundle | − `GlobTool`, − `GrepTool` |
| TodoV2 enabled | + `TaskCreate/Get/Update/List` (otherwise `TaskUpdateTool`-only via Todo path) |
| ToolSearch enabled | + `ToolSearchTool` (and tools with `shouldDefer=true` are deferred) |
| REPL enabled | tools in `REPL_ONLY_TOOLS` hidden from direct surface |
| Coordinator mode | adds Agent/TaskStop/SendMessage to simple toolset |
| Worktree mode | + `EnterWorktreeTool`, `ExitWorktreeTool` |
| Agent swarms | + `TeamCreateTool`, `TeamDeleteTool` |

---

## 9. Error Handling & Edge Cases

The registry layer itself does not throw. Failure modes:

- **Missing tool source file at registry-resolved path** (e.g., `MonitorTool`, `WorkflowTool` etc.): `require()` would throw `MODULE_NOT_FOUND` at first evaluation. In the leaked tree these are guarded by `feature()` flags that are off in the leaked build, so the require line is DCE-stripped. **In a reimplementation, all required modules must exist OR the gate must be off.**
- **Duplicate tool name**: `assembleToolPool` resolves via `uniqBy('name')` keeping the first occurrence (built-in wins); `getMergedTools` does NOT dedupe and would deliver duplicates. Callers must understand which they are calling.
- **Invalid input schema**: each tool's `validateInput()` returns `ValidationResult` which the call site checks before `checkPermissions()` (see spec 04 turn pipeline).
- **MCP tools with prefix-rule deny**: `filterToolsByDenyRules` strips entire MCP server's tools before the model sees them, preventing the model from attempting to call them.

User-facing error messages are owned by individual tool implementations (specs 10..19) and the permission system (spec 09).

---

## 10. Telemetry & Observability

The base registry layer emits no telemetry directly. Per-tool telemetry (e.g., `BashTool` `logEvent` calls) is owned by individual tool specs. ToolSearch / deferred-loading metrics (was_discovered, etc.) are tracked via `discoveredSkillNames` on `ToolUseContext` (spec 04 / spec 17).

---

## 11. Reimplementation Checklist

A reimplementer of `Tool.ts` + `tools.ts` must preserve:

- [ ] Every Tool implements at minimum: `name`, `inputSchema`, `prompt`, `call`, `description`, `userFacingName`, `isReadOnly`, `isConcurrencySafe`, `isEnabled`, `checkPermissions`, `mapToolResultToToolResultBlockParam`, `renderToolUseMessage`, `maxResultSizeChars`, `toAutoClassifierInput`. `buildTool()` provides safe defaults for the seven keys in `DefaultableToolKeys` (`isEnabled`, `isConcurrencySafe`, `isReadOnly`, `isDestructive`, `checkPermissions`, `toAutoClassifierInput`, `userFacingName`).
- [ ] `TOOL_DEFAULTS` are fail-closed: `isReadOnly=false`, `isConcurrencySafe=false`, `checkPermissions={behavior:'allow',updatedInput}` — the *general* permission system is the gate, not the tool-specific check.
- [ ] `userFacingName` default in `buildTool` overrides `TOOL_DEFAULTS` to `() -> def.name`, then `def.userFacingName` (if any) overrides that. Spread order matters: `{ ...TOOL_DEFAULTS, userFacingName: () -> def.name, ...def }`.
- [ ] `getAllBaseTools()` order matches the upstream `claude_code_global_system_caching` StatSig config. Reordering invalidates global cache.
- [ ] `assembleToolPool()` keeps built-ins as a sorted contiguous prefix, then sorted MCP tools, with `uniqBy('name')` preserving insertion order. Cache breakpoint location is set by `claude_code_system_cache_policy`.
- [ ] `getMergedTools()` is the flat-concat alternative for token counting and ToolSearch thresholds (NOT for prompt assembly).
- [ ] `filterToolsByDenyRules` uses the same matcher as the runtime check (`getDenyRuleForTool`), so MCP server-prefix denies (`mcp__server`) strip whole servers before the model sees them.
- [ ] CLAUDE_CODE_SIMPLE branch: `[BashTool, FileReadTool, FileEditTool]` + coordinator delta; if REPL mode also on, replace primitives with `[REPLTool]` plus the same coordinator delta.
- [ ] REPL mode strips `REPL_ONLY_TOOLS` only when `REPLTool` is present in the resulting set.
- [ ] Special tools list `{ ListMcpResourcesTool.name, ReadMcpResourceTool.name, SYNTHETIC_OUTPUT_TOOL_NAME }` is removed from `getTools()` output; callers add MCP-resource tools via different paths.
- [ ] `Tool.shouldDefer === true` triggers `defer_loading: true` in the API request; `Tool.alwaysLoad === true` overrides that. MCP tools set `alwaysLoad` via `_meta['anthropic/alwaysLoad']`.
- [ ] `Tool.maxResultSizeChars`: triggers disk persistence with path-only preview returned to Claude. `FileReadTool` overrides to `Infinity` to avoid Read→file→Read loops.
- [ ] `ToolUseContext` has the full ~74 fields documented in §4.1; subagent context construction (`createSubagentContext`, `forkSubagent`) clones the parent's `contentReplacementState` by default for cache-sharing forks.
- [ ] Lazy `require()` getters used for circular deps with `as typeof import(...)` to preserve types: `getTeamCreateTool`, `getTeamDeleteTool`, `getSendMessageTool`.
- [ ] Conditional `require()` for build-time DCE: `feature() ? require(...).X : null`. Equivalent dynamic `import()` is forbidden (does not strip at build time).
- [ ] ANT import-order banner (`biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered`) preserved at file head where required.
- [ ] On-disk pattern preserved for new tools: `<Name>Tool.tsx` (or `.ts`) main, `prompt.ts` for system prompt, `UI.tsx` for rendering, additional helpers as needed.
- [ ] `.js` import suffix for `.ts` source files (NodeNext/ESM resolution).
- [ ] Zod v4: `import { z } from 'zod/v4'` (not `'zod'`).

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. RESOLVED items cite where the answer landed.

1. ~~**`src/types/tools.ts` and `src/types/message.ts` location**~~ — **DEFERRED (missing-leaked-source)**: `src/types/generated/` exists but contains only `events_mono/` and `google/` proto-binding subdirectories. `src/types/hooks.ts:15` still imports `from 'src/types/message.js'`. Confirmed missing. Recorded in spec 00 §13 missing-source ledger and 00 §12 Q1 (RESOLVED there with same finding).
2. ~~**`SYNTHETIC_OUTPUT_TOOL_NAME`**~~ — **RESOLVED Phase 9.7**: spec 19 (tool-misc) and spec 30 (coordinator) confirm the SyntheticOutput tool is the swarm-worker-facing surface used to emit teammate completion summaries. The name-only reference in `tools.ts:97` is for the `specialTools` filter (excludes it from auto-permission prompting). The tool is registered separately via swarm-init (per `tools.ts:228` referenced in spec 30 §2.1).
3. ~~**`MCPTool` referenced as type symbol but not in `getAllBaseTools()`**~~ — **RESOLVED Phase 9.7**: spec 16 (tool-mcp-lsp) and spec 23 (service-mcp) confirm MCP tools enter the pool via the `mcpTools` parameter of `assembleToolPool`, populated from `appState.mcp.tools` after MCP server connection completes. The `MCPTool` type symbol is a TypeScript discriminator, not an array entry.
4. ~~**StatSig dynamic config `claude_code_global_system_caching`**~~ — **DEFERRED**: server-side StatSig artifact (UI-only at console.statsig.com per spec 26 §12). Recorded in spec 00 §13 as known-unfalsifiable.
5. ~~**StatSig dynamic config `claude_code_system_cache_policy`**~~ — **DEFERRED**: same as Q4. Spec 00 §13 row 1 covers both configs.
6. ~~**`tengu_tool_pear`**~~ — **NOTE Phase 9.7**: GrowthBook flag controlling per-tool `strict: true` in API request. Default and full enumeration are GrowthBook-server artifacts (not source-derivable, per spec 26 §12). Consumer side (spec 22) inspects via `getFeatureValue_CACHED_MAY_BE_STALE`.
7. ~~**`TungstenTool` / `MergedTool` outputSchema requirement**~~ — **NOTE Phase 9.7**: in-source TODO preserved at `Tool.ts:398-400`. Not a defect; recorded for future reimplementer.
8. ~~**`ANT_ONLY_SAFE_ENV_VARS`**~~ — **RESOLVED Phase 9.7**: enumerated verbatim in spec 10 §6 (Bash tool spec); ownership confirmed there.
9. ~~**REPL VM tool delegation**~~ — **RESOLVED Phase 9.7**: spec 19 (tool-misc) §3 documents the REPLTool wrapper protocol (postMessage between host process and the VM-isolated worker). Mechanism is consumer-side; the registry layer here only wires the name.
10. ~~**`CLAUDE_AGENT_SDK_MCP_NO_PREFIX`**~~ — **RESOLVED Phase 9.7**: live at `services/mcp/client.ts:1763` (`isEnvTruthy(process.env.CLAUDE_AGENT_SDK_MCP_NO_PREFIX)` controls MCP tool name prefixing). Spec 23 §3 documents. Cross-ref also in spec 00 §12 Q6.
