# 30 — Coordinator & Multi-Agent Spawn

> **Owner**: sub-G0 · **Status**: done · **Last updated**: 2026-05-08
> Adjacent: 03 (QueryEngine), 14 (agent/team tool surface), 15 (task tools), 31 (proactive), 41 (resume).

---

## 1. Purpose & Scope

This spec documents the engine that turns an `Agent` / `TeamCreate` / `SendMessage` tool_use call (or a `feature('FORK_SUBAGENT')` implicit fork) into a running sub-process: the **spawn algorithm** (`tools/shared/spawnMultiAgent.ts`), the **worker turn loop** (`tools/AgentTool/runAgent.ts` → spec 03 `query()`), the **coordinator orchestrator** (`coordinator/coordinatorMode.ts` + `tools/AgentTool/builtInAgents.ts`'s `getCoordinatorAgents()` lazy require), the **built-in agent definitions** (`tools/AgentTool/built-in/*.ts`), the **mailbox transport** (`utils/teammateMailbox.ts`), the **fork-subagent prefix builder** (`tools/AgentTool/forkSubagent.ts`), and the **periodic summarization service** (`services/AgentSummary/agentSummary.ts`). It also owns the `services/awaySummary.ts`, `services/toolUseSummary/`, and `services/autoDream/` services per 00 §2.3.

**IN scope**: `src/coordinator/`, `src/tools/shared/spawnMultiAgent.ts`, `src/tools/AgentTool/{runAgent,forkSubagent,resumeAgent,agentMemory,agentMemorySnapshot,agentColorManager,agentDisplay,agentToolUtils,builtInAgents}.ts`, `src/tools/AgentTool/built-in/*.ts`, `src/services/{AgentSummary,autoDream,toolUseSummary}/`, `src/services/awaySummary.ts`, `src/hooks/useAwaySummary.ts`, `src/utils/teammateMailbox.ts`, **`src/utils/swarm/` (the entire 22-file swarm subsystem — backends, in-process runner, permission sync, teamHelpers, layout, init, reconnection, etc.; ~7,548 LOC)**, **`src/utils/agentSwarmsEnabled.ts` (`isAgentSwarmsEnabled` gate)**, **`src/utils/teammateContext.ts` (AsyncLocalStorage teammate identity isolation)**, **`src/hooks/useSwarmPermissionPoller.ts` and `src/hooks/toolPermission/handlers/swarmWorkerHandler.ts`** (worker→leader permission bridge), the `BG_SESSIONS`/`AGENT_MEMORY_SNAPSHOT`/`AWAY_SUMMARY`/`COORDINATOR_MODE`/`BUILTIN_EXPLORE_PLAN_AGENTS`/`FORK_SUBAGENT`/`PROMPT_CACHE_BREAK_DETECTION`/`VERIFICATION_AGENT`/`KAIROS` deltas reachable here, the ANT-only `'remote'` isolation option, and prompt-cache break detection inside agent runs.

**Two distinct gates** — readers must not conflate them:
- `isCoordinatorMode()` (`coordinator/coordinatorMode.ts:36-41`) — gates the **coordinator orchestrator** persona. Requires `feature('COORDINATOR_MODE')` + env-truthy `CLAUDE_CODE_COORDINATOR_MODE`. Mutually exclusive with `isForkSubagentEnabled()` (fork gate explicitly checks `!isCoordinatorMode()`, `forkSubagent.ts:34`).
- `isAgentSwarmsEnabled()` (`utils/agentSwarmsEnabled.ts:24-44`) — gates the **multi-process teammate / swarm-worker** persona (CLI: `--agent-teams`, env: `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS`, killswitch: GrowthBook `tengu_amber_flint`; ant always-on). Drives `tools.ts:228` registration of TeamCreate/TeamDelete/SendMessage/SyntheticOutput, `setup.ts:105/117` swarm-init, `main.tsx:1187/1385/2141` resume-as-teammate paths, and the worker-side permission bridge.

A single process is **either** a coordinator (orchestrating) **or** a swarm worker (teammate spawned by another claude) **or** neither — never both. The two gates are independent feature flags with independent kill-switches; documents that conflate "coordinator" and "swarm" are wrong.

**Delegation to spec 29 (memory service)**: persistent memory writes (`extractMemories/`, `SessionMemory/`, `teamMemorySync/`, MEMORY.md content lifecycle) belong to spec 29; this spec owns only the *summary* services (`AgentSummary/`, `awaySummary.ts`, `toolUseSummary/`, `autoDream/`) which are scratch summarization runs, not memory writes.

**Handoff with spec 14**: spec 14 owns the **AgentTool tool surface** (input schema, prompt assembly, `checkPermissions`, registry wiring); this spec owns the **runner** (`runAgent.ts`, `forkSubagent.ts` execution, coordinator orchestration, `builtInAgents.ts`). Both specs cite `forkSubagent.ts` — spec 14 references the *gate predicate* (`isForkSubagentEnabled`), this spec owns the *fork-message construction and execution* path.

**OUT of scope** (cross-references):
- `AgentTool` / `TeamCreateTool` / `TeamDeleteTool` / `SendMessageTool` schema, prompt assembly and registry wiring → spec **14**.
- `Task*` tools and `TodoWriteTool` → spec **15**.
- The streaming `query()` and `QueryEngine` API loop the worker drives into → spec **03**, **04**.
- `PROACTIVE` auto-wakeup and `SleepTool` mechanics → spec **31** (we cite the `proactiveModule?.isProactiveActive()` callsite).
- Bridge / remote / CCR transports — `teleportToRemote`, `RemoteAgentTask` → spec **34** / **35**.
- UI rendering of agent activity (status pill, agent dialog) → spec **37**.
- Session-persistence of agents (sidechain transcripts, agent metadata) → spec **41** (we cite the `recordSidechainTranscript` / `writeAgentMetadata` calls).

---

## 2. Source-coverage inventory

| Path | Lines | Role |
|---|---|---|
| `src/coordinator/coordinatorMode.ts` | 369 | `isCoordinatorMode`, `matchSessionMode`, `getCoordinatorUserContext`, `getCoordinatorSystemPrompt` (full coordinator prompt verbatim) |
| `src/tools/shared/spawnMultiAgent.ts` | 1093 | tmux/iTerm2/in-process spawn handlers; `spawnTeammate` (sole export used by `AgentTool`); `resolveTeammateModel`; `generateUniqueTeammateName` |
| `src/tools/AgentTool/runAgent.ts` | 973 | The worker turn-loop body — fork context, MCP merge, system-prompt assembly, frontmatter hooks, query() driver, prompt-cache-break cleanup. |
| `src/tools/AgentTool/AgentTool.tsx` | 1397 | Tool surface (spec 14) — but call() also hosts the foreground/background lifecycle, worktree setup, fork routing, remote isolation, name registry write — algorithm details surfaced here. |
| `src/tools/AgentTool/forkSubagent.ts` | 210 | `isForkSubagentEnabled`, `FORK_AGENT`, `isInForkChild`, `buildForkedMessages`, `buildChildMessage`, `buildWorktreeNotice` |
| `src/tools/AgentTool/agentToolUtils.ts` | 686 | `filterToolsForAgent`, `resolveAgentTools`, `agentToolResultSchema`, `finalizeAgentTool`, `runAsyncAgentLifecycle`, `classifyHandoffIfNeeded`, `extractPartialResult` |
| `src/tools/AgentTool/builtInAgents.ts` | 72 | `getBuiltInAgents` registry — coordinator branch, Explore/Plan A/B, Verification gate, SDK kill-switch |
| `src/tools/AgentTool/built-in/{generalPurposeAgent,exploreAgent,planAgent,statuslineSetup,verificationAgent,claudeCodeGuideAgent}.ts` | 35 / 84 / 93 / 145 / 153 / 206 | Built-in agent definitions; full system prompts |
| `src/tools/AgentTool/agentMemory.ts` / `agentMemorySnapshot.ts` | 177 / 197 | Persistent agent-memory dirs (user/project/local); snapshot sync state |
| `src/tools/AgentTool/agentColorManager.ts` / `agentDisplay.ts` | 66 / 104 | Color assignment table; source-grouping for `/agents` UI |
| `src/tools/AgentTool/resumeAgent.ts` | 265 | `resumeAgentBackground` — replays sidechain transcript, fork resume path |
| `src/utils/teammateMailbox.ts` | 1184 | Inbox file transport, all envelope schemas, idle/permission/shutdown protocol |
| `src/services/AgentSummary/agentSummary.ts` | 179 | 30s-tick fork-based progress summary |
| `src/services/awaySummary.ts` + `src/hooks/useAwaySummary.ts` | 74 + 125 | "While you were away" 5-minute blur recap |
| `src/services/toolUseSummary/toolUseSummaryGenerator.ts` | 112 | SDK Haiku tool-batch label generator |
| `src/services/autoDream/{autoDream,config,consolidationLock,consolidationPrompt}.ts` | 324 / 21 / 140 / 65 | Background memory consolidation fork |
| `src/constants/tools.ts` | 113 | `ASYNC_AGENT_ALLOWED_TOOLS`, `ALL_AGENT_DISALLOWED_TOOLS`, `CUSTOM_AGENT_DISALLOWED_TOOLS`, `IN_PROCESS_TEAMMATE_ALLOWED_TOOLS`, `COORDINATOR_MODE_ALLOWED_TOOLS` |
| `src/constants/xml.ts:52,63,66` | — | `TEAMMATE_MESSAGE_TAG`, `FORK_BOILERPLATE_TAG`, `FORK_DIRECTIVE_PREFIX` |

#### 2.1 Swarm subsystem (`src/utils/swarm/`, 22 files, ~7,548 LOC)

The swarm subsystem implements the **multi-process / multi-pane teammate** model — spawning, identity, transport, permission relaying, and reconnection — that sits below `spawnMultiAgent.ts`. Architecture covered here; per-file roles (signature inventory) are catalogued in spec **42a §`utils/swarm/`** and not re-listed here. Architectural roles by layer:

| Layer | File(s) | LOC | Role |
|---|---|---|---|
| In-process runner | `swarm/inProcessRunner.ts` | 1552 | Wraps `runAgent()` for in-process teammates. Drives the worker turn-loop with mailbox-side-channel permission requests, idle/shutdown protocol, plan-mode handshake, and auto-compact handling. The largest single piece of swarm orchestration logic. Imports `useSwarmPermissionPoller` callback registration and `sendPermissionRequestViaMailbox` for permission round-trips. |
| Permission bridge | `swarm/permissionSync.ts` (928), `swarm/leaderPermissionBridge.ts` (54), `hooks/useSwarmPermissionPoller.ts`, `hooks/toolPermission/handlers/swarmWorkerHandler.ts` | ~1100 | The worker→leader permission RPC. `isSwarmWorker()` (`permissionSync.ts:596`) returns true when the current process was spawned with `--agent-id`+`--team-name`. Worker-side `swarmWorkerHandler.ts:43` short-circuits `useCanUseTool` when `isAgentSwarmsEnabled() && isSwarmWorker()`, calls `sendPermissionRequestViaMailbox` (`permissionSync.ts:676`), and awaits a `permission_response` envelope via `registerPermissionCallback` (`useSwarmPermissionPoller`). Leader-side `useSwarmPermissionPoller` polls the leader's inbox, surfaces the dialog, then `leaderPermissionBridge.ts` writes the decision back. Sandbox-permission RPC (`sandbox_permission_request`/`_response`) uses the parallel `registerSandboxPermissionCallback` path (`REPL.tsx:40-41`). |
| Spawn-process glue | `swarm/spawnInProcess.ts` (328), `swarm/spawnUtils.ts` (146), `swarm/teammateInit.ts` (129) | ~600 | `spawnInProcessTeammate` sets up the AsyncLocalStorage teammate context (via `utils/teammateContext.ts`) + AbortController + AppState registration before `startInProcessTeammate` fires. `teammateInit.ts` runs once when the teammate process starts (claude was launched with `--agent-id`+`--team-name`+`--agent-name`+`--parent-session-id`). |
| Backend abstraction | `swarm/backends/registry.ts` (464), `backends/types.ts` (311), `backends/detection.ts` (128), `backends/teammateModeSnapshot.ts` (87), `backends/PaneBackendExecutor.ts` (354), `backends/TmuxBackend.ts` (764), `backends/ITermBackend.ts` (370), `backends/InProcessBackend.ts` (339), `backends/it2Setup.ts` (245) | ~3060 | Three concrete pane backends (tmux, iTerm2-it2, in-process) behind a common executor. `detection.ts` probes which is available; `teammateModeSnapshot.ts` caches user choice (`'auto'` vs explicit). `PaneBackendExecutor.ts` is the shared spawn driver (flag inheritance, env merge, mailbox writes). `it2Setup.ts` runs `pip install iterm2` and verifies the Python-API path; `It2SetupPrompt.tsx` (379) renders the Ink dialog when `setupResult` is needed. |
| Reconnection / layout | `swarm/reconnection.ts` (119), `swarm/teammateLayoutManager.ts` (107) | ~226 | Reconnect to existing swarm panes after a crash; manage rows/cols of the swarm view. |
| Identity / model / prompt | `swarm/teammateModel.ts` (10), `swarm/teammatePromptAddendum.ts` (18), `swarm/teamHelpers.ts` (683), `swarm/constants.ts` (33) | ~744 | Per-teammate model resolution, common prompt suffix injected into teammate system prompts, team-file CRUD helpers (member list, leader registration). |
| It2-setup UI | `swarm/It2SetupPrompt.tsx` | 379 | Ink+JSX dialog driven by `swarm/backends/it2Setup.ts`, surfaced from `spawnMultiAgent.ts:362`. |

**`isAgentSwarmsEnabled()` checkpoints touched in this spec's surface area** (non-exhaustive): `tools.ts:228` (registration of swarm-only tools), `setup.ts:105,117` (init), `main.tsx:1187,1385,2141` (resume teammate identity), `main.tsx:2911` (force plan-mode for swarm worker if `isPlanModeRequired`), `tools/ExitPlanModeTool/ExitPlanModeV2Tool.ts:406`, `tools/TaskUpdateTool/TaskUpdateTool.ts:189,277,390`, `tools/TaskCreateTool/prompt.ts:6,10`, `screens/REPL.tsx:2218`, `hooks/toolPermission/handlers/swarmWorkerHandler.ts:43`. None of these are gated by `feature('COORDINATOR_MODE')`.

**AsyncLocalStorage isolation guarantee** (M6): `utils/teammateContext.ts` exposes `isInProcessTeammate()` (imported at `AgentTool.tsx:38`) and the ALS store keyed by `{teammateId, teamName, agentName, parentSessionId}`. `spawnInProcessTeammate` (`swarm/spawnInProcess.ts`) wraps `startInProcessTeammate` in `runWith(ctx, fn)` so every async hop inside the in-process worker — including `runAgent`, MCP calls, and mailbox writes — sees the correct teammate identity even when multiple in-process workers share the leader's heap. The guarantee is **per-async-context**, not per-thread; concurrent teammates do not share state because each is isolated by ALS. Loss of context (`getStore() === undefined`) means the code is running on the leader, never an arbitrary teammate.

`src/coordinator/workerAgent.ts` is referenced by lazy `require()` from `builtInAgents.ts:38-40` but is **not present** in the leaked tree (recorded in §12).

---

## 3. Public Interface

Single primary export shape (callers in spec 14):

```ts
// src/tools/shared/spawnMultiAgent.ts:1088-1093
export async function spawnTeammate(
  config: SpawnTeammateConfig,
  context: ToolUseContext,
): Promise<{ data: SpawnOutput }>
```

`SpawnTeammateConfig` and `SpawnOutput` are verbatim in §6.4. The teammate path is the in-process / tmux / iTerm2 split-pane spawn used by `AgentTool` when both `team_name` and `name` are supplied (`AgentTool.tsx:284-316`).

The async-agent path uses `runAgent({...})` (`runAgent.ts:248`) directly, driven through `runAsyncAgentLifecycle({...})` (`agentToolUtils.ts:508`). The fork path uses `buildForkedMessages(directive, assistantMessage)` (`forkSubagent.ts:107`) plus `runAgent` with `useExactTools: true`.

Coordinator-mode toggles (`coordinatorMode.ts:36-78`):

```ts
isCoordinatorMode(): boolean
matchSessionMode(sessionMode: 'coordinator' | 'normal' | undefined): string | undefined
getCoordinatorUserContext(mcpClients, scratchpadDir?): { [k: string]: string }
getCoordinatorSystemPrompt(): string
```

Mailbox surface (`utils/teammateMailbox.ts`): `writeToMailbox`, `readMailbox`, `readUnreadMessages`, `markMessageAsReadByIndex`, `markMessagesAsRead`, `clearMailbox`, plus envelope constructors/checkers (`createIdleNotification` / `isIdleNotification`, `createPermissionRequestMessage` / `isPermissionRequest`, etc., enumerated §6.5).

Periodic summarization: `startAgentSummarization(taskId, agentId, cacheSafeParams, setAppState) → { stop }` (`agentSummary.ts:46`).

Away summary: `useAwaySummary(messages, setMessages, isLoading)` React hook (`useAwaySummary.ts:32`).

Auto-dream: `initAutoDream()`, `executeAutoDream(context, appendSystemMessage?)` (`autoDream.ts:122,319`).

Tool-use summary (SDK): `generateToolUseSummary({tools, signal, isNonInteractiveSession, lastAssistantText?})` (`toolUseSummaryGenerator.ts:45`).

**Coordinator tool-pool filter — canonical owner: spec 30.** `applyCoordinatorToolFilter` lives at `src/utils/toolPool.ts:35` and is invoked from two surfaces that must remain in lockstep:
1. **Headless / non-REPL path** — `main.tsx:1872-1879` performs a dynamic `await import('./utils/toolPool.js')` and applies the filter to the merged tool list before the QueryEngine first turn.
2. **REPL path** — `hooks/useMergedTools.ts` applies the same filter as part of the React-driven tool merge.

Spec 30 owns the **filter's call-site invariants** (which tools are removed in coordinator mode, why `INTERNAL_WORKER_TOOLS` exists, the contract that both surfaces produce identical filtered lists). Any change to the coordinator tool whitelist must update `INTERNAL_WORKER_TOOLS` (`coordinatorMode.ts:29-34`), `COORDINATOR_MODE_ALLOWED_TOOLS` (`constants/tools.ts:107-112`), **and** verify both call sites still apply the filter. Spec 14 owns the AgentTool surface but **not** this filter — it sees a pre-filtered tool list.

---

## 4. Data Model & State

### 4.1 Spawn input/output

Verbatim from `spawnMultiAgent.ts:107-150`:

```ts
export type SpawnOutput = {
  teammate_id: string
  agent_id: string
  agent_type?: string
  model?: string
  name: string
  color?: string
  tmux_session_name: string
  tmux_window_name: string
  tmux_pane_id: string
  team_name?: string
  is_splitpane?: boolean
  plan_mode_required?: boolean
}

export type SpawnTeammateConfig = {
  name: string
  prompt: string
  team_name?: string
  cwd?: string
  use_splitpane?: boolean
  plan_mode_required?: boolean
  model?: string
  agent_type?: string
  description?: string
  invokingRequestId?: string
}
```

### 4.2 Agent memory scope

`tools/AgentTool/agentMemory.ts:13`:
```ts
export type AgentMemoryScope = 'user' | 'project' | 'local'
```
Resolves to (`agentMemory.ts:52-65`):
- `user`: `<getMemoryBaseDir()>/agent-memory/<agentType>/`
- `project`: `<cwd>/.claude/agent-memory/<agentType>/`
- `local`: `<cwd>/.claude/agent-memory-local/<agentType>/` (or `$CLAUDE_CODE_REMOTE_MEMORY_DIR/projects/<gitRoot>/agent-memory-local/<agentType>/` when set, `agentMemory.ts:29-44`).

Colon (`:`) in plugin-namespaced types is replaced with `-` (`agentMemory.ts:20-22`).

Snapshot sync state (`agentMemorySnapshot.ts:14-25`):
- `<cwd>/.claude/agent-memory-snapshots/<agentType>/snapshot.json` carries `{ updatedAt: string }`.
- `<scopeDir>/.snapshot-synced.json` carries `{ syncedFrom: string }`. Action enum is `'none' | 'initialize' | 'prompt-update'`.

### 4.3 Color palette

`agentColorManager.ts:4-23`:
```ts
type AgentColorName = 'red'|'blue'|'green'|'yellow'|'purple'|'orange'|'pink'|'cyan'
const AGENT_COLORS: readonly AgentColorName[] = ['red','blue','green','yellow','purple','orange','pink','cyan']
```
`general-purpose` always returns `undefined` (`agentColorManager.ts:37-39`); palette mapping is a hard-coded table to `*_FOR_SUBAGENTS_ONLY` theme keys.

### 4.4 Agent result envelope

Zod schema verbatim (`agentToolUtils.ts:227-258`) — see §6.6.

### 4.5 Mailbox message types

Wire shape (`teammateMailbox.ts:43-50`):
```ts
type TeammateMessage = {
  from: string
  text: string
  timestamp: string
  read: boolean
  color?: string
  summary?: string
}
```
File path `<getTeamsDir()>/<sanitize(team)>/inboxes/<sanitize(agent)>.json` (`teammateMailbox.ts:56-66`).

Structured protocol envelopes nested in `text` (each carries its own `type` discriminant): `idle_notification`, `permission_request`, `permission_response`, `sandbox_permission_request`, `sandbox_permission_response`, `shutdown_request`, `shutdown_approved`, `shutdown_rejected`, `team_permission_update`, `mode_set_request`, `plan_approval_request`, `plan_approval_response`, `task_assignment` — see §6.5 for full Zod schemas.

`isStructuredProtocolMessage` (`teammateMailbox.ts:1073-1095`) is the dispatch oracle: messages whose `type` equals one of `permission_request | permission_response | sandbox_permission_request | sandbox_permission_response | shutdown_request | shutdown_approved | team_permission_update | mode_set_request | plan_approval_request | plan_approval_response` are routed by the inbox poller and **not** consumed as raw LLM context.

### 4.6 Coordinator state surface

A coordinator session is identified by `feature('COORDINATOR_MODE') && isEnvTruthy(process.env.CLAUDE_CODE_COORDINATOR_MODE)` (`coordinatorMode.ts:37-40`). When entering / exiting a resumed session, `matchSessionMode` mutates `process.env.CLAUDE_CODE_COORDINATOR_MODE` and emits `tengu_coordinator_mode_switched` (`coordinatorMode.ts:49-78`).

`CLAUDE_CODE_SIMPLE` env-truthy switches the coordinator's worker-tool capability list to just `Bash, Read, Edit` plus MCP (`coordinatorMode.ts:88-96, 112-114`).

### 4.7 Tool-availability sets

Verbatim (`constants/tools.ts:36-112`) — see §6.7.

---

## 5. Algorithm / Control Flow

The lifecycle has eight distinct entry points. They share `runAgent()` for the actual conversation loop.

```
                    ┌── AgentTool.call (spec 14)
                    │
   coordinator?─Y──>│ assemble coordinator system prompt + worker tool whitelist
   COORDINATOR_MODE │
                    │
   teammate?────Y──>│ spawnTeammate → handleSpawn → tmux | iTerm2 | in-process
   (team_name+name) │
                    │
   fork?────────Y──>│ FORK_AGENT, useExactTools, buildForkedMessages
   (no subagent_type│
    + FORK_SUBAGENT)│
                    │
   regular subagent>│ runAgent({agentDefinition, ...})
                    │
                    └── always async if any of:
                          run_in_background=true, selectedAgent.background=true,
                          isCoordinator, isForkSubagentEnabled,
                          (KAIROS && kairosEnabled), proactiveModule.isProactiveActive
                          AND !CLAUDE_CODE_DISABLE_BACKGROUND_TASKS
```

### 5.1 `spawnMultiAgent` end-to-end pseudocode

Verbatim algorithm body lives in §6.1; here is the dispatch flow (`spawnMultiAgent.ts:1040-1078`):

```
handleSpawn(input, ctx):
  if isInProcessEnabled(): return handleSpawnInProcess(input, ctx)
  try:
    await detectAndGetBackend()           // tmux / iTerm2-it2 / iTerm2-native
  catch e:
    if getTeammateModeFromSnapshot() != 'auto': throw
    markInProcessFallback()
    return handleSpawnInProcess(input, ctx)
  if input.use_splitpane != false:
    return handleSpawnSplitPane(input, ctx)
  return handleSpawnSeparateWindow(input, ctx)
```

**Split-pane handler** (`spawnMultiAgent.ts:305-539`):
1. Resolve model: `resolveTeammateModel(input.model, appState.mainLoopModel)`. `'inherit'` → leader's model; undefined → `getDefaultTeammateModel(leader)` → `getGlobalConfig().teammateDefaultModel` ?? `getHardcodedTeammateModelFallback()` (`spawnMultiAgent.ts:72-101`).
2. Resolve teamName from `input.team_name ?? appState.teamContext?.teamName`. Throw if absent.
3. `generateUniqueTeammateName`: scan `teamFile.members`; if base name collides, append numeric suffix `-2`, `-3`, … (`spawnMultiAgent.ts:267-294`).
4. `sanitizeAgentName(name)` (strips `@`).
5. `teammateId = formatAgentId(sanitized, teamName)` — deterministic.
6. Backend detection: if `needsIt2Setup`, render `<It2SetupPrompt>` JSX and await user choice `'installed' | 'use-tmux' | 'cancelled'`. On `'installed'`/`'use-tmux'`: `resetBackendDetection()` then re-detect.
7. `assignTeammateColor(teammateId)` from `agentColorManager`'s palette.
8. `createTeammatePaneInSwarmView(name, color)` returns `{paneId, isFirstTeammate}`. If `isFirstTeammate && insideTmux`: `enablePaneBorderStatus()`.
9. Build CLI args: `--agent-id`, `--agent-name`, `--team-name`, `--agent-color`, `--parent-session-id <getSessionId()>`, optional `--plan-mode-required`, optional `--agent-type`. Quote each via `quote([])`.
10. `buildInheritedCliFlags({planModeRequired, permissionMode})` (`spawnMultiAgent.ts:208-260`):
    - permissionMode `bypassPermissions` ⇒ `--dangerously-skip-permissions` *unless* `planModeRequired` (plan mode wins).
    - `acceptEdits` / `auto` propagate as `--permission-mode <mode>`.
    - Propagate `--model` (from `getMainLoopModelOverride()`), `--settings`, `--plugin-dir` (each inline plugin), `--chrome` / `--no-chrome`.
11. If `model` set: strip any inherited `--model` flag, append `--model <quote([model])>`.
12. `envStr = buildInheritedEnvVars()` (CLAUDECODE, CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS, API provider vars).
13. Spawn cmd: `cd <dir> && env <envs> <binary> <teammateArgs> <flags>`. Binary = `process.env[TEAMMATE_COMMAND_ENV_VAR]` ?? (`isInBundledMode()` ? `process.execPath` : `process.argv[1]`) (`spawnMultiAgent.ts:193-198`).
14. `sendCommandToPane(paneId, spawnCommand, !insideTmux)`.
15. setAppState: register teammate in `teamContext.teammates[teammateId]`.
16. `registerOutOfProcessTeammateTask` — creates `InProcessTeammateTaskState`, registers via `registerTask`. Abort handler kills the pane via the originating backend (`spawnMultiAgent.ts:825-833`).
17. Append to team file (`<teamsDir>/<team>/team.json` member list) via `readTeamFileAsync`/`writeTeamFileAsync`.
18. **Initial prompt delivery**: `writeToMailbox(sanitizedName, {from: TEAM_LEAD_NAME, text: prompt, timestamp}, teamName)`. The teammate's inbox poller reads it on its first turn.

**Separate-window handler** (`spawnMultiAgent.ts:545-753`): identical except (a) `ensureSession(SWARM_SESSION_NAME)` first, (b) creates a tmux *window* rather than a pane via `tmux new-window -t SWARM_SESSION_NAME -n <windowName> -P -F #{pane_id}`, (c) sends spawn command via `tmux send-keys -t <session>:<window>`, (d) `backendType` is hard-coded `'tmux'`.

**In-process handler** (`spawnMultiAgent.ts:840-1032`):
1. Resolve model + teamName + sanitizedName + teammateId + color (same as split-pane).
2. Look up `agent_type` against `context.options.agentDefinitions.activeAgents`; if `isCustomAgent(found)`, attach as `agentDefinition` to spawn config.
3. `await spawnInProcessTeammate(config, context)` — sets up `AsyncLocalStorage` teammate context; returns `{taskId, teammateContext, abortController}` on success.
4. `startInProcessTeammate({identity, taskId, prompt, description, model, agentDefinition, teammateContext, toolUseContext: {...context, messages: []}, abortController, invokingRequestId})` — fire-and-forget. The strip of `messages: []` is intentional: pinned parent conversation would survive `/clear` and auto-compact.
5. setAppState: register teammate (and auto-register lead via `formatAgentId(TEAM_LEAD_NAME, teamName)` when `!prev.teamContext?.leadAgentId`).
6. Append to team file with `backendType: 'in-process'`.
7. **Skip mailbox write** — in-process teammates receive the prompt directly through `startInProcessTeammate`. Sending via both paths would double-deliver.

### 5.2 `runAgent` worker turn-loop pseudocode

`runAgent.ts:248-860`:

```
runAgent({agentDefinition, promptMessages, toolUseContext, canUseTool, isAsync,
          forkContextMessages, querySource, override, model, maxTurns,
          preserveToolUseResults, availableTools, allowedTools,
          onCacheSafeParams, contentReplacementState, useExactTools,
          worktreePath, description, transcriptSubdir, onQueryProgress}):

  appState = ctx.getAppState()
  permissionMode = appState.toolPermissionContext.mode
  rootSetAppState = ctx.setAppStateForTasks ?? ctx.setAppState
  resolvedAgentModel = getAgentModel(agentDefinition.model, ctx.mainLoopModel, model, permissionMode)
  agentId = override?.agentId ?? createAgentId()

  if transcriptSubdir: setAgentTranscriptSubdir(agentId, transcriptSubdir)
  if isPerfettoTracingEnabled(): registerPerfettoAgent(agentId, agentDefinition.agentType, parentId)
  if USER_TYPE=='ant': log dump-prompts path

  contextMessages = forkContextMessages ? filterIncompleteToolCalls(forkContextMessages) : []
  initialMessages = [...contextMessages, ...promptMessages]
  agentReadFileState = forkContextMessages ? cloneFileStateCache(parent) : createFileStateCacheWithSizeLimit(READ_FILE_STATE_CACHE_SIZE)

  [baseUserContext, baseSystemContext] = await Promise.all([
    override?.userContext ?? getUserContext(),
    override?.systemContext ?? getSystemContext(),
  ])

  // CLAUDE.md drop for read-only built-ins, gated by tengu_slim_subagent_claudemd (default true)
  if agentDefinition.omitClaudeMd && !override?.userContext && getFeatureValue_CACHED('tengu_slim_subagent_claudemd', true):
    delete resolvedUserContext.claudeMd

  // gitStatus drop for Explore/Plan
  if agentType in {'Explore','Plan'}: delete resolvedSystemContext.gitStatus

  agentGetAppState = () => {
    state = ctx.getAppState()
    tpc = state.toolPermissionContext
    // permission-mode override: agent's own mode wins UNLESS parent is bypassPermissions/acceptEdits/auto
    if agentDefinition.permissionMode && tpc.mode != 'bypassPermissions' && tpc.mode != 'acceptEdits' && !(TRANSCRIPT_CLASSIFIER && tpc.mode=='auto'):
      tpc = {...tpc, mode: agentDefinition.permissionMode}
    // shouldAvoidPermissionPrompts
    shouldAvoidPrompts = canShowPermissionPrompts !== undefined
        ? !canShowPermissionPrompts
        : agentDefinition.permissionMode == 'bubble' ? false : isAsync
    if shouldAvoidPrompts: tpc.shouldAvoidPermissionPrompts = true
    if isAsync && !shouldAvoidPrompts: tpc.awaitAutomatedChecksBeforeDialog = true
    // session permissions: replace ONLY session rules, preserve cliArg
    if allowedTools !== undefined:
      tpc.alwaysAllowRules = {cliArg: tpc.alwaysAllowRules.cliArg, session: [...allowedTools]}
    return {...state, toolPermissionContext: tpc, effortValue: agentDefinition.effort ?? state.effortValue}
  }

  resolvedTools = useExactTools ? availableTools : resolveAgentTools(agentDefinition, availableTools, isAsync).resolvedTools
  agentSystemPrompt = override?.systemPrompt ?? asSystemPrompt(getAgentSystemPrompt(agentDefinition, ctx, resolvedAgentModel, additionalWorkingDirs, resolvedTools))
  agentAbortController = override?.abortController ?? (isAsync ? new AbortController() : ctx.abortController)

  // SubagentStart hooks → collect additionalContexts → push as user attachment if any
  for await (hr of executeSubagentStartHooks(agentId, agentType, signal)): collect

  // Frontmatter hooks scoped to agent lifetime, gated by isRestrictedToPluginOnly('hooks') vs admin-trusted
  if agentDefinition.hooks && hooksAllowed: registerFrontmatterHooks(rootSetAppState, agentId, hooks, ..., isAgent=true)

  // Skill preloading — tries exact name, plugin-prefixed name, suffix-:name match (resolveSkillName, runAgent.ts:945-973)
  for skillName in agentDefinition.skills: resolveSkillName → load → push as isMeta user message

  // Agent-specific MCP servers (initializeAgentMcpServers, runAgent.ts:95-218)
  // - admin-trusted gate when isRestrictedToPluginOnly('mcp')
  // - inline {[name]: config} entries are newly-created (cleaned up); string refs are shared (not cleaned up)
  {clients, tools, cleanup} = await initializeAgentMcpServers(agentDefinition, ctx.options.mcpClients)
  allTools = uniqBy([...resolvedTools, ...agentMcpTools], 'name')

  agentOptions = {
    isNonInteractiveSession: useExactTools ? parent : (isAsync ? true : (parent ?? false)),
    appendSystemPrompt: parent.appendSystemPrompt,
    tools: allTools, commands: [],
    debug, verbose,
    mainLoopModel: resolvedAgentModel,
    thinkingConfig: useExactTools ? parent.thinkingConfig : {type: 'disabled'},
    mcpClients, mcpResources, agentDefinitions,
    ...(useExactTools && {querySource}),  // survives autocompact
  }

  agentToolUseContext = createSubagentContext(ctx, {options: agentOptions, agentId, agentType, messages: initialMessages, readFileState, abortController, getAppState: agentGetAppState, shareSetAppState: !isAsync, shareSetResponseLength: true, criticalSystemReminder_EXPERIMENTAL, contentReplacementState})
  if preserveToolUseResults: agentToolUseContext.preserveToolUseResults = true

  if onCacheSafeParams:
    onCacheSafeParams({systemPrompt: agentSystemPrompt, userContext, systemContext, toolUseContext: agentToolUseContext, forkContextMessages: initialMessages})

  // Persist BEFORE first turn (fire-and-forget): sidechain transcript + agent metadata
  void recordSidechainTranscript(initialMessages, agentId).catch(...)
  void writeAgentMetadata(agentId, {agentType, ...(worktreePath && {worktreePath}), ...(description && {description})}).catch(...)
  lastRecordedUuid = initialMessages.at(-1)?.uuid ?? null

  try:
    for await message of query({messages, systemPrompt: agentSystemPrompt, userContext, systemContext, canUseTool, toolUseContext: agentToolUseContext, querySource, maxTurns: maxTurns ?? agentDefinition.maxTurns}):
      onQueryProgress?.()
      // forward TTFT/OTPS metrics
      if message.type=='stream_event' && message.event.type=='message_start' && message.ttftMs != null:
        ctx.pushApiMetricsEntry?.(message.ttftMs); continue
      if message.type=='attachment':
        if message.attachment.type=='max_turns_reached': log; break
        yield message; continue
      if isRecordableMessage(message):
        await recordSidechainTranscript([message], agentId, lastRecordedUuid).catch(...)
        if message.type != 'progress': lastRecordedUuid = message.uuid
        yield message
    if agentAbortController.signal.aborted: throw new AbortError()
    if isBuiltInAgent(agentDefinition) && agentDefinition.callback: agentDefinition.callback()
  finally:
    await mcpCleanup()
    if agentDefinition.hooks: clearSessionHooks(rootSetAppState, agentId)
    if feature('PROMPT_CACHE_BREAK_DETECTION'): cleanupAgentTracking(agentId)
    agentToolUseContext.readFileState.clear()
    initialMessages.length = 0
    unregisterPerfettoAgent(agentId)
    clearAgentTranscriptSubdir(agentId)
    rootSetAppState(prev => purge prev.todos[agentId])
    killShellTasksForAgent(agentId, ctx.getAppState, rootSetAppState)
    if feature('MONITOR_TOOL'): killMonitorMcpTasksForAgent(agentId, ctx.getAppState, rootSetAppState)
```

`isRecordableMessage` accepts: assistant, user, progress, or `system` with `subtype === 'compact_boundary'` (`runAgent.ts:231-246`).

`filterIncompleteToolCalls` (`runAgent.ts:866-904`) collects all `tool_use_id`s that have a matching `tool_result` block and drops any assistant message with a `tool_use` block whose id isn't in that set. Required for fork context to avoid orphaned-tool-use 400 errors.

### 5.3 Async-agent lifecycle (`runAsyncAgentLifecycle`, `agentToolUtils.ts:508-686`)

```
runAsyncAgentLifecycle({taskId, abortController, makeStream, metadata, description, ctx, rootSetAppState, agentIdForCleanup, enableSummarization, getWorktreeResult}):
  agentMessages = []
  try:
    tracker = createProgressTracker()
    resolveActivity = createActivityDescriptionResolver(ctx.options.tools)
    onCacheSafeParams = enableSummarization ? params => stopSummarization = startAgentSummarization(taskId, asAgentId(taskId), params, rootSetAppState).stop : undefined
    for await msg of makeStream(onCacheSafeParams):
      agentMessages.push(msg)
      // append-to-task only when UI holds (retain=true)
      rootSetAppState(prev => isLocalAgentTask && t.retain ? {...prev, tasks: {...prev.tasks, [taskId]: {...t, messages: [...t.messages, msg]}}} : prev)
      updateProgressFromMessage(tracker, msg, resolveActivity, ctx.options.tools)
      updateAsyncAgentProgress(taskId, getProgressUpdate(tracker), rootSetAppState)
      if lastToolName = getLastToolUseName(msg): emitTaskProgress(...)
    stopSummarization?.()
    agentResult = finalizeAgentTool(agentMessages, taskId, metadata)
    completeAsyncAgent(agentResult, rootSetAppState)   // FIRST — TaskOutput(block=true) unblocks before classifier/git
    finalMessage = extractTextContent(agentResult.content, '\n')
    if feature('TRANSCRIPT_CLASSIFIER'):
      handoffWarning = await classifyHandoffIfNeeded({...})  // §6.3
      if handoffWarning: finalMessage = `${handoffWarning}\n\n${finalMessage}`
    worktreeResult = await getWorktreeResult()
    enqueueAgentNotification({taskId, description, status: 'completed', setAppState, finalMessage, usage, toolUseId, ...worktreeResult})
  catch error:
    stopSummarization?.()
    if error instanceof AbortError:
      killAsyncAgent(taskId, rootSetAppState)
      log tengu_agent_tool_terminated reason='user_kill_async'
      enqueueAgentNotification({status:'killed', finalMessage: extractPartialResult(agentMessages), ...worktreeResult})
      return
    failAsyncAgent(taskId, errorMessage(error), rootSetAppState)
    enqueueAgentNotification({status:'failed', error: errorMessage(error), ...})
  finally:
    clearInvokedSkillsForAgent(agentIdForCleanup)
    clearDumpState(agentIdForCleanup)
```

**Critical ordering** (`agentToolUtils.ts:599-604`, `:642-646`): status-transition writes (`completeAsyncAgent` / `killAsyncAgent`) MUST precede `classifyHandoffIfNeeded` and `getWorktreeResult` so `TaskOutput(block=true)` callers unblock immediately even if git or classifier hangs (gh-20236).

### 5.4 Sync-agent lifecycle (`AgentTool.tsx:686-1265`)

The sync path runs the agent inline via `agentIterator = runAgent(...)[Symbol.asyncIterator]()` and races `agentIterator.next()` against `backgroundPromise = registration.backgroundSignal.then(() => ({type:'background'}))` (`AgentTool.tsx:885-892`). Background trigger is either user-initiated (`/bg` style) or **auto-background after** `getAutoBackgroundMs()` ms — `120_000` if `CLAUDE_AUTO_BACKGROUND_TASKS` env-truthy or `tengu_auto_background_agents=true` (`AgentTool.tsx:72-77`).

When backgrounded mid-run: `agentIterator.return(undefined).catch(()=>{})` raced against `sleep(1000)` to release MCP/hooks; then a fresh `runAgent` pass starts with `isAsync: true` reusing the same `agentMessages` for progress continuity (`AgentTool.tsx:912-1037`).

`PROGRESS_THRESHOLD_MS = 2000` (`AgentTool.tsx:63`) — after 2 seconds, render `<BackgroundHint />` JSX.

`isBackgroundTasksDisabled = isEnvTruthy(process.env.CLAUDE_CODE_DISABLE_BACKGROUND_TASKS)` evaluated at module load (`AgentTool.tsx:66-68`).

`shouldRunAsync` (`AgentTool.tsx:567`) is the OR of: explicit `run_in_background`, `selectedAgent.background === true`, `isCoordinator`, `isForkSubagentEnabled()`, `(KAIROS && appState.kairosEnabled)`, `proactiveModule?.isProactiveActive()` — all gated by `!isBackgroundTasksDisabled`.

### 5.5 Fork-subagent path (`forkSubagent.ts`)

Gate: `feature('FORK_SUBAGENT') && !isCoordinatorMode() && !getIsNonInteractiveSession()` (`forkSubagent.ts:32-39`). Mutually exclusive with coordinator.

Effects when on:
- `subagent_type` becomes optional on the Agent schema (`AgentTool.tsx:110-125`).
- Omitting `subagent_type` triggers an implicit fork (`AgentTool.tsx:322`); `selectedAgent = FORK_AGENT`.
- All agent spawns run async for unified `<task-notification>` re-entry (`AgentTool.tsx:557`).
- `/fork <directive>` slash command becomes available (slash-command spec 21).

Recursive-fork guard (`AgentTool.tsx:325-334`):
1. Primary: `toolUseContext.options.querySource === 'agent:builtin:fork'` (compaction-resistant — set on context.options at spawn time, survives autocompact's message rewrite).
2. Fallback: `isInForkChild(messages)` scans for `<fork-boilerplate>` text-block opener.

`FORK_AGENT` definition verbatim — see §6.2 (synthetic, not registered in `getBuiltInAgents()`).

`buildForkedMessages` algorithm (`forkSubagent.ts:107-169`) — see §6.8.

`buildWorktreeNotice(parentCwd, worktreeCwd)` — see §6.8 — appended to fork prompt when worktree isolation is also active (`AgentTool.tsx:598-602`).

### 5.6 Coordinator mode integration

The coordinator does NOT itself spawn workers; it is a system-prompt + tool-whitelist preset for the main thread that *forces* every Agent invocation async (`AgentTool.tsx:567`) and rewrites the registered built-in agent list. Lookup chain:

1. `getBuiltInAgents()` (`builtInAgents.ts:22-72`) checks `feature('COORDINATOR_MODE') && CLAUDE_CODE_COORDINATOR_MODE`. If true: lazy-require `coordinator/workerAgent.js` (**absent in leak**) and return `getCoordinatorAgents()`.
2. Otherwise return `[GENERAL_PURPOSE_AGENT, STATUSLINE_SETUP_AGENT, ...(EXPLORE+PLAN if enabled), ...(CLAUDE_CODE_GUIDE if non-SDK), ...(VERIFICATION_AGENT if feature+gate)]`.

Coordinator filters tool pool to `COORDINATOR_MODE_ALLOWED_TOOLS = {Agent, TaskStop, SendMessage, SyntheticOutput}` (`constants/tools.ts:107-112`); REPL path applies it via `useMergedTools.ts`, headless path via `applyCoordinatorToolFilter` import in `main.tsx:1872-1879`.

Coordinator user-context augmentation (`coordinatorMode.ts:80-109`): a `workerToolsContext` user-context entry listing the worker's tool whitelist (Bash/Read/Edit triple in `CLAUDE_CODE_SIMPLE`, else `ASYNC_AGENT_ALLOWED_TOOLS \\ INTERNAL_WORKER_TOOLS` — `INTERNAL_WORKER_TOOLS = {TeamCreate, TeamDelete, SendMessage, SyntheticOutput}`), MCP server names if any, and the scratchpad path when `tengu_scratch` gate is on (`coordinatorMode.ts:25-27,104-106`).

### 5.7 Resume path (`resumeAgent.ts:42-265`)

`resumeAgentBackground({agentId, prompt, toolUseContext, canUseTool, invokingRequestId})`:
1. Read `[transcript, meta] = await Promise.all([getAgentTranscript(asAgentId(agentId)), readAgentMetadata(asAgentId(agentId))])`. Throw if no transcript.
2. Filter: `filterWhitespaceOnlyAssistantMessages(filterOrphanedThinkingOnlyMessages(filterUnresolvedToolUses(transcript.messages)))`.
3. `reconstructForSubagentResume(ctx.contentReplacementState, resumedMessages, transcript.contentReplacements)` — re-replays content replacement so the prompt cache lines up.
4. If `meta.worktreePath` exists and is a directory: bump mtime via `utimes(now, now)` to defeat stale-worktree cleanup (#22355). Otherwise undefined.
5. Select agent: `meta.agentType === FORK_AGENT.agentType` ⇒ `FORK_AGENT, isResumedFork=true`; else `activeAgents.find(...)` ?? `GENERAL_PURPOSE_AGENT`.
6. Fork-resume: pull system prompt from `ctx.renderedSystemPrompt` if present, else reconstruct via `buildEffectiveSystemPrompt({mainThreadAgentDefinition, ctx, customSystemPrompt, defaultSystemPrompt: getSystemPrompt(...), appendSystemPrompt})`.
7. `workerTools = isResumedFork ? ctx.options.tools : assembleToolPool({...ctx.toolPermissionContext, mode: selectedAgent.permissionMode ?? 'acceptEdits'}, appState.mcp.tools)`.
8. `runAgent` params: `promptMessages = [...resumedMessages, createUserMessage({content: prompt})]`, `forkContextMessages: undefined` (already in resumedMessages — re-supplying duplicates tool_use ids), `useExactTools: isResumedFork`.
9. Skip `filterDeniedAgents` re-gating (original spawn passed). Skip name-registry write (original entry persists).
10. Drive through `runAsyncAgentLifecycle` with `enableSummarization = isCoordinatorMode() || isForkSubagentEnabled() || getSdkAgentProgressSummariesEnabled()` (`resumeAgent.ts:250-253`).
11. `wrapWithCwd` ensures `runAgent` runs under `runWithCwdOverride(resumedWorktreePath, ...)` when worktree was rehydrated.

### 5.8 Periodic summarization (`agentSummary.ts`)

`SUMMARY_INTERVAL_MS = 30_000` (`agentSummary.ts:26`).

```
startAgentSummarization(taskId, agentId, cacheSafeParams, setAppState):
  drop forkContextMessages from baseParams (rebuild each tick from disk)
  scheduleNext()
  return {stop: () => stopped=true; clearTimeout; abortController.abort()}

runSummary():
  if stopped: return
  transcript = await getAgentTranscript(agentId)
  if !transcript || transcript.messages.length < 3: scheduleNext(); return
  cleanMessages = filterIncompleteToolCalls(transcript.messages)
  forkParams = {...baseParams, forkContextMessages: cleanMessages}
  canUseTool = async () => ({behavior: 'deny', message: 'No tools needed for summary', decisionReason: {type:'other', reason:'summary only'}})
  // DO NOT set maxOutputTokens (clamps budget_tokens → thinking config mismatch → cache miss)
  result = await runForkedAgent({promptMessages: [createUserMessage(buildSummaryPrompt(previousSummary))], cacheSafeParams: forkParams, canUseTool, querySource: 'agent_summary', forkLabel: 'agent_summary', overrides: {abortController}, skipTranscript: true})
  for msg of result.messages where assistant && !isApiErrorMessage:
    text = first text block; if non-empty: previousSummary = text; updateAgentSummary(taskId, text, setAppState); break
  finally: scheduleNext()  // reset on completion to avoid overlap
```

Summary prompt verbatim — see §6.10.

### 5.9 Away-summary hook (`useAwaySummary.ts`)

`BLUR_DELAY_MS = 5 * 60_000` (`useAwaySummary.ts:13`).

Gate: `feature('AWAY_SUMMARY') && getFeatureValue_CACHED('tengu_sedge_lantern', false)` (`useAwaySummary.ts:48-54`).

Subscribes to `subscribeTerminalFocus` → on `'blurred'` start a 5-min timer. On fire: if `isLoading`, set `pendingRef=true` (fire when turn ends); else `generate()`. On `'focused'`: clear timer + abort in-flight + reset pending. `'unknown'` (terminal lacks DECSET 1004): no-op.

Idempotency: `hasSummarySinceLastUserTurn` (`useAwaySummary.ts:16-23`) walks back to find a `system` `away_summary` message before the most recent non-meta user turn.

`generateAwaySummary` (`awaySummary.ts:29-74`): truncates to `RECENT_MESSAGE_WINDOW = 30` messages (`awaySummary.ts:16`), includes session memory if present, `queryModelWithoutStreaming` with `getSmallFastModel()` and `tools: []`, `querySource: 'away_summary'`, `skipCacheWrite: true`. Prompt verbatim — §6.11.

### 5.10 Auto-dream (`autoDream.ts`)

Three gates evaluated cheapest-first (`autoDream.ts:1-12`): time → sessions → lock.

```
runAutoDream(context, appendSystemMessage):
  if !force && !isGateOpen(): return  // KAIROS off, !RemoteMode, autoMemoryEnabled, isAutoDreamEnabled
  cfg = getConfig()  // tengu_onyx_plover (defensive); minHours=24, minSessions=5
  lastAt = await readLastConsolidatedAt()  // mtime of <autoMem>/.consolidate-lock
  hoursSince = (now - lastAt) / 3_600_000
  if hoursSince < cfg.minHours: return
  // Scan throttle 10 min
  if (now - lastSessionScanAt) < SESSION_SCAN_INTERVAL_MS: return
  lastSessionScanAt = now
  sessionIds = (await listSessionsTouchedSince(lastAt)).filter(id => id != getSessionId())
  if sessionIds.length < cfg.minSessions: return
  priorMtime = force ? lastAt : await tryAcquireConsolidationLock()
  if priorMtime === null: return
  taskId = registerDreamTask(setAppState, {sessionsReviewing, priorMtime, abortController})
  try:
    prompt = buildConsolidationPrompt(memoryRoot, transcriptDir, extra)
    result = await runForkedAgent({promptMessages, cacheSafeParams: createCacheSafeParams(ctx), canUseTool: createAutoMemCanUseTool(memoryRoot), querySource: 'auto_dream', forkLabel: 'auto_dream', skipTranscript: true, overrides: {abortController}, onMessage: makeDreamProgressWatcher(taskId, setAppState)})
    completeDreamTask(taskId, setAppState)
    if filesTouched.length > 0: appendSystemMessage(createMemorySavedMessage(filesTouched))
  catch e:
    if abortController.signal.aborted: return
    failDreamTask(taskId, setAppState)
    await rollbackConsolidationLock(priorMtime)  // rewind mtime so time-gate passes again
```

Lock semantics (`consolidationLock.ts:46-84`):
- File: `<autoMemPath>/.consolidate-lock`. Body = holder PID. mtime = `lastConsolidatedAt`.
- `HOLDER_STALE_MS = 60 * 60 * 1000` (`consolidationLock.ts:20`). Past that, even live PID is reclaimed.
- Race resolution: both reclaimers write; `readFile` re-verify; loser bails when PID mismatch.
- `recordConsolidation()` writes from manual `/dream` runs (`consolidationLock.ts:130-140`).

Consolidation prompt verbatim — see §6.12.

### 5.11 Tool-use-summary generator (`toolUseSummaryGenerator.ts`)

SDK helper, not in the LLM loop. `generateToolUseSummary({tools, signal, isNonInteractiveSession, lastAssistantText?})` ⇒ ≤ ~30-char Haiku label using `queryHaiku`. Truncates each tool input/output to 300 chars via `jsonStringify` (`toolUseSummaryGenerator.ts:102-111`). Caps gracefully: returns `null` if `tools.length === 0` or on any error (errors logged with `errorId = E_TOOL_USE_SUMMARY_GENERATION_FAILED`). Prompt verbatim — see §6.13.

### 5.12 Prompt-cache break detection in agent runs

`feature('PROMPT_CACHE_BREAK_DETECTION')` is gated at agent-finally (`runAgent.ts:823-826`) — calls `cleanupAgentTracking(agentId)` to delete the per-source `previousStateBySource` entry (`promptCacheBreakDetection.ts:700-702`). The detection itself (`MAX_TRACKED_SOURCES`, diff writing, `notifyCacheDeletion`, `notifyCompaction`) is owned by spec 22 (`services/api/`); from this spec's perspective each agentId is a bounded tracking-key whose lifetime ends at agent finally.

### 5.13 BG_SESSIONS task summaries

`feature('BG_SESSIONS')` lives in two places relevant here:
- `main.tsx:1115-1118` — when the gate is on AND `--agent <name>` was passed, `process.env.CLAUDE_CODE_AGENT = agentCli`.
- `query.ts:118-120,1685-1701` — `taskSummaryModule = require('./utils/taskSummary.js')` is lazy-imported when the gate is on; mid-turn, top-level conversations (`!toolUseContext.agentId`) call `taskSummaryModule.maybeGenerateTaskSummary({...})` if `shouldGenerateTaskSummary()` returns true. Subagents/forks are explicitly excluded so per-agent summaries are not generated; the user-facing `claude ps` command consumes the resulting summaries. The forked summary fork shares the cache prefix via `forkContextMessages` (full message tail).

### 5.14 ANT-only `'remote'` isolation

`AgentTool.tsx:99,431-482`. The schema enum is `("external" === 'ant' ? z.enum(['worktree','remote']) : z.enum(['worktree']))` (a literal-vs-literal compile-time `'external'` check, replaced at build time per the bundler convention). When `effectiveIsolation === 'remote'`:

1. `await checkRemoteAgentEligibility()` — bail with formatted preconditions on failure.
2. `teleportToRemote({initialMessage: prompt, description, signal, onBundleFail})` returns the CCR session (or null).
3. `registerRemoteAgentTask({remoteTaskType: 'remote-agent', session, command: prompt, context, toolUseId})` returns `{taskId, sessionId}`.
4. Emit `tengu_agent_tool_remote_launched`.
5. Return `RemoteLaunchedOutput` (`AgentTool.tsx:183-190`) — `'remote_launched'` status with `outputFile = getTaskOutputPath(taskId)`. Always async.

CCR transport details (registration, bundle failure handling, session URL formation) are owned by spec 35 — this spec just routes into them.

### 5.15 Verification agent

`feature('VERIFICATION_AGENT')` AND `getFeatureValue_CACHED('tengu_hive_evidence', false)` ⇒ append `VERIFICATION_AGENT` to the registry (`builtInAgents.ts:64-69`). The agent type `'verification'` is also referenced from `TaskUpdateTool.ts:335` — those callers spawn a verification subagent when a task is moved to a verifying state (spec 15).

`VERIFICATION_AGENT` is `background: true` (`built-in/verificationAgent.ts:138`) — i.e. **always async** even when `run_in_background` is omitted. Carries `criticalSystemReminder_EXPERIMENTAL` (verbatim §6.2).

---

## 6. Verbatim Assets

### 6.1 `spawnMultiAgent` algorithm pseudocode (collapsed)

```
spawnTeammate(config, ctx) ≡ handleSpawn(config, ctx)

handleSpawn(input, ctx):
  if isInProcessEnabled(): return handleSpawnInProcess(input, ctx)
  try detectAndGetBackend()
  catch:
    if getTeammateModeFromSnapshot() != 'auto': throw
    markInProcessFallback(); return handleSpawnInProcess(input, ctx)
  if input.use_splitpane != false: return handleSpawnSplitPane(input, ctx)
  return handleSpawnSeparateWindow(input, ctx)

handleSpawnSplitPane / handleSpawnSeparateWindow / handleSpawnInProcess:
  model = resolveTeammateModel(input.model, getAppState().mainLoopModel)
  teamName = input.team_name ?? getAppState().teamContext?.teamName  // throw if empty
  uniqueName = generateUniqueTeammateName(input.name, teamName)
  sanitized = sanitizeAgentName(uniqueName)
  teammateId = formatAgentId(sanitized, teamName)
  color = assignTeammateColor(teammateId)
  // (split-pane only) detectAndGetBackend; if needsIt2Setup show It2SetupPrompt JSX
  // pane/window creation differs:
  //   split-pane:    createTeammatePaneInSwarmView → enablePaneBorderStatus if first inside tmux
  //   separate-win:  ensureSession(SWARM_SESSION_NAME); tmux new-window -P -F #{pane_id}
  //   in-process:    spawnInProcessTeammate; startInProcessTeammate (ALS-based)
  cliArgs = [--agent-id, --agent-name, --team-name, --agent-color, --parent-session-id getSessionId(),
             plan_mode_required ? --plan-mode-required : '',
             agent_type ? --agent-type … : '']
  flags = buildInheritedCliFlags({planModeRequired, permissionMode: parent.toolPermissionContext.mode})
          // - planModeRequired blocks --dangerously-skip-permissions inheritance
          // - propagate --model (mainLoopModelOverride), --settings, --plugin-dir (each), --chrome/--no-chrome
  if model: strip inherited --model; append --model <quote(model)>
  spawnCmd = `cd ${quote(workingDir)} && env ${envStr} ${quote(binary)} ${cliArgs}${flags}`  // (out-of-process only)
  // out-of-process: sendCommandToPane(paneId, spawnCmd, !insideTmux) | tmux send-keys
  // in-process:     startInProcessTeammate(...) fire-and-forget with messages: [] strip
  setAppState: register teammate in teamContext.teammates[teammateId]
  registerOutOfProcessTeammateTask (out-of-process) | startInProcessTeammate registers via spawnInProcessTeammate (in-process)
  push to teamFile.members; writeTeamFileAsync
  // out-of-process ONLY: writeToMailbox(sanitized, {from: TEAM_LEAD_NAME, text: prompt, timestamp}, teamName)
  return {data: SpawnOutput}
```

### 6.2 Built-in agent definitions

`GENERAL_PURPOSE_AGENT` (`built-in/generalPurposeAgent.ts:25-34`):
```ts
export const GENERAL_PURPOSE_AGENT: BuiltInAgentDefinition = {
  agentType: 'general-purpose',
  whenToUse:
    'General-purpose agent for researching complex questions, searching for code, and executing multi-step tasks. When you are searching for a keyword or file and are not confident that you will find the right match in the first few tries use this agent to perform the search for you.',
  tools: ['*'],
  source: 'built-in',
  baseDir: 'built-in',
  // model intentionally omitted - uses getDefaultSubagentModel().
  getSystemPrompt: getGeneralPurposeSystemPrompt,
}
```
System prompt verbatim (`built-in/generalPurposeAgent.ts:3-23`):
```
SHARED_PREFIX = "You are an agent for Claude Code, Anthropic's official CLI for Claude. Given the user's message, you should use the tools available to complete the task. Complete the task fully—don't gold-plate, but don't leave it half-done."

SHARED_GUIDELINES = "Your strengths:
- Searching for code, configurations, and patterns across large codebases
- Analyzing multiple files to understand system architecture
- Investigating complex questions that require exploring many files
- Performing multi-step research tasks

Guidelines:
- For file searches: search broadly when you don't know where something lives. Use Read when you know the specific file path.
- For analysis: Start broad and narrow down. Use multiple search strategies if the first doesn't yield results.
- Be thorough: Check multiple locations, consider different naming conventions, look for related files.
- NEVER create files unless they're absolutely necessary for achieving your goal. ALWAYS prefer editing an existing file to creating a new one.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested."

prompt = SHARED_PREFIX + " When you complete the task, respond with a concise report covering what was done and any key findings — the caller will relay this to the user, so it only needs the essentials.\n\n" + SHARED_GUIDELINES
```

`EXPLORE_AGENT` definition (`built-in/exploreAgent.ts:64-83`):
```ts
agentType: 'Explore'
disallowedTools: [Agent, ExitPlanMode, FileEdit, FileWrite, NotebookEdit]
source: 'built-in', baseDir: 'built-in'
model: process.env.USER_TYPE === 'ant' ? 'inherit' : 'haiku'
omitClaudeMd: true
EXPLORE_AGENT_MIN_QUERIES = 3
```
Full Explore system prompt and the `whenToUse` text are inlined verbatim at `built-in/exploreAgent.ts:13-62`. `hasEmbeddedSearchTools()` swaps `Glob`/`Grep` for `find`/`grep` via `Bash`.

`PLAN_AGENT` (`built-in/planAgent.ts:73-92`):
```ts
agentType: 'Plan'
tools: EXPLORE_AGENT.tools (i.e. inherits the same wildcard-ness)
disallowedTools: [Agent, ExitPlanMode, FileEdit, FileWrite, NotebookEdit]
model: 'inherit'
omitClaudeMd: true
```
System prompt verbatim at `built-in/planAgent.ts:14-71`.

`STATUSLINE_SETUP_AGENT` (`built-in/statuslineSetup.ts:134-144`):
```ts
agentType: 'statusline-setup'
tools: ['Read', 'Edit']
model: 'sonnet'
color: 'orange'
```
System prompt verbatim at `built-in/statuslineSetup.ts:3-132`.

`VERIFICATION_AGENT` (`built-in/verificationAgent.ts:134-152`):
```ts
agentType: 'verification'
color: 'red'
background: true   // ALWAYS async
disallowedTools: [Agent, ExitPlanMode, FileEdit, FileWrite, NotebookEdit]
model: 'inherit'
criticalSystemReminder_EXPERIMENTAL: 'CRITICAL: This is a VERIFICATION-ONLY task. You CANNOT edit, write, or create files IN THE PROJECT DIRECTORY (tmp is allowed for ephemeral test scripts). You MUST end with VERDICT: PASS, VERDICT: FAIL, or VERDICT: PARTIAL.'
```
System prompt verbatim at `built-in/verificationAgent.ts:10-129` (PASS/FAIL/PARTIAL output spec, adversarial probes baseline, rationalization recognizer).

`CLAUDE_CODE_GUIDE_AGENT` (`built-in/claudeCodeGuideAgent.ts:98-205`):
```ts
agentType: 'claude-code-guide'
model: 'haiku'
permissionMode: 'dontAsk'
tools: hasEmbeddedSearchTools()
  ? [Bash, FileRead, WebFetch, WebSearch]
  : [Glob, Grep, FileRead, WebFetch, WebSearch]
```
System prompt body inlined at `built-in/claudeCodeGuideAgent.ts:23-87` plus `getFeedbackGuideline()` at `:89-96`. The `getSystemPrompt` body dynamically appends `**Available custom skills**`, `**Available custom agents**`, `**Configured MCP servers**`, `**Available plugin skills**`, and the user's `settings.json` JSON.

`FORK_AGENT` synthetic (`forkSubagent.ts:60-71`):
```ts
export const FORK_AGENT = {
  agentType: 'fork',
  whenToUse: 'Implicit fork — inherits full conversation context. Not selectable via subagent_type; triggered by omitting subagent_type when the fork experiment is active.',
  tools: ['*'],
  maxTurns: 200,
  model: 'inherit',
  permissionMode: 'bubble',
  source: 'built-in',
  baseDir: 'built-in',
  getSystemPrompt: () => '',     // unused — overridden with parent's renderedSystemPrompt
} satisfies BuiltInAgentDefinition
```

### 6.3 Coordinator system prompt (verbatim)

The full `getCoordinatorSystemPrompt()` body at `coordinator/coordinatorMode.ts:111-369` is inlined here in full. The prompt covers six numbered sections ("1. Your Role", "2. Your Tools", "3. Workers", "4. Task Workflow", "5. Writing Worker Prompts", "6. Example Session"), declares `<task-notification>` as the worker-result envelope, defines the parallelism guidance, and shows the canonical synthesize-then-route worker prompts. Because the file is the source of truth, this spec cites the line range rather than re-inlining the 250-line prompt; sub-agents working on this surface MUST read `coordinator/coordinatorMode.ts:111-369` directly. The `workerCapabilities` line at `:112-114` is one of two:
- `CLAUDE_CODE_SIMPLE` env-truthy: `'Workers have access to Bash, Read, and Edit tools, plus MCP tools from configured MCP servers.'`
- otherwise: `'Workers have access to standard tools, MCP tools from configured MCP servers, and project skills via the Skill tool. Delegate skill invocations (e.g. /commit, /verify) to workers.'`

The `<task-notification>` envelope from §3 of that prompt:
```xml
<task-notification>
<task-id>{agentId}</task-id>
<status>completed|failed|killed</status>
<summary>{human-readable status summary}</summary>
<result>{agent's final text response}</result>
<usage>
  <total_tokens>N</total_tokens>
  <tool_uses>N</tool_uses>
  <duration_ms>N</duration_ms>
</usage>
</task-notification>
```

### 6.4 SpawnOutput / SpawnTeammateConfig (verbatim)

See §4.1 — `spawnMultiAgent.ts:107-150`.

### 6.5 Mailbox envelope schemas (verbatim)

`TeammateMessage` (the file row, `teammateMailbox.ts:43-50`): `{from, text, timestamp, read, color?, summary?}`.

`IdleNotificationMessage` (`teammateMailbox.ts:393-405`):
```ts
type IdleNotificationMessage = {
  type: 'idle_notification'
  from: string
  timestamp: string
  idleReason?: 'available' | 'interrupted' | 'failed'
  summary?: string
  completedTaskId?: string
  completedStatus?: 'resolved' | 'blocked' | 'failed'
  failureReason?: string
}
```

`PermissionRequestMessage` (`teammateMailbox.ts:452-463`): `{type:'permission_request', request_id, agent_id, tool_name, tool_use_id, description, input, permission_suggestions}` — snake-case to match SDK `can_use_tool`.

`PermissionResponseMessage` (`teammateMailbox.ts:466-484`): success variant `{type:'permission_response', request_id, subtype:'success', response?: {updated_input?, permission_updates?}}` | error variant `{type:'permission_response', request_id, subtype:'error', error: string}`.

`SandboxPermissionRequestMessage` / `SandboxPermissionResponseMessage` (`teammateMailbox.ts:574-606`): worker → leader sandbox host approval and back; worker identity carried as `workerId`/`workerName`/`workerColor`.

Zod-validated envelopes (`teammateMailbox.ts:684-769`):
```ts
PlanApprovalRequestMessageSchema = z.object({
  type: z.literal('plan_approval_request'),
  from: z.string(), timestamp: z.string(),
  planFilePath: z.string(), planContent: z.string(), requestId: z.string(),
})
PlanApprovalResponseMessageSchema = z.object({
  type: z.literal('plan_approval_response'),
  requestId: z.string(), approved: z.boolean(),
  feedback: z.string().optional(), timestamp: z.string(),
  permissionMode: PermissionModeSchema().optional(),
})
ShutdownRequestMessageSchema = z.object({
  type: z.literal('shutdown_request'),
  requestId: z.string(), from: z.string(),
  reason: z.string().optional(), timestamp: z.string(),
})
ShutdownApprovedMessageSchema = z.object({
  type: z.literal('shutdown_approved'),
  requestId: z.string(), from: z.string(), timestamp: z.string(),
  paneId: z.string().optional(), backendType: z.string().optional(),
})
ShutdownRejectedMessageSchema = z.object({
  type: z.literal('shutdown_rejected'),
  requestId: z.string(), from: z.string(),
  reason: z.string(), timestamp: z.string(),
})
ModeSetRequestMessageSchema = z.object({
  type: z.literal('mode_set_request'),
  mode: PermissionModeSchema(),
  from: z.string(),
})
```

`TaskAssignmentMessage` (`teammateMailbox.ts:953-961`): `{type:'task_assignment', taskId, subject, description, assignedBy, timestamp}`.

`TeamPermissionUpdateMessage` (`teammateMailbox.ts:983-997`): `{type:'team_permission_update', permissionUpdate: {type:'addRules', rules: [{toolName, ruleContent?}], behavior:'allow'|'deny'|'ask', destination:'session'}, directoryPath, toolName}`.

### 6.6 `agentToolResultSchema` (verbatim)

`agentToolUtils.ts:227-258`:
```ts
z.object({
  agentId: z.string(),
  agentType: z.string().optional(),
  content: z.array(z.object({type: z.literal('text'), text: z.string()})),
  totalToolUseCount: z.number(),
  totalDurationMs: z.number(),
  totalTokens: z.number(),
  usage: z.object({
    input_tokens: z.number(),
    output_tokens: z.number(),
    cache_creation_input_tokens: z.number().nullable(),
    cache_read_input_tokens: z.number().nullable(),
    server_tool_use: z.object({
      web_search_requests: z.number(),
      web_fetch_requests: z.number(),
    }).nullable(),
    service_tier: z.enum(['standard', 'priority', 'batch']).nullable(),
    cache_creation: z.object({
      ephemeral_1h_input_tokens: z.number(),
      ephemeral_5m_input_tokens: z.number(),
    }).nullable(),
  }),
})
```

### 6.7 Tool-availability sets (verbatim)

`constants/tools.ts:36-112`:

```ts
ALL_AGENT_DISALLOWED_TOOLS = new Set([
  TASK_OUTPUT_TOOL_NAME,
  EXIT_PLAN_MODE_V2_TOOL_NAME,
  ENTER_PLAN_MODE_TOOL_NAME,
  ...(USER_TYPE === 'ant' ? [] : [AGENT_TOOL_NAME]),  // ant gets nested-agent
  ASK_USER_QUESTION_TOOL_NAME,
  TASK_STOP_TOOL_NAME,
  ...(feature('WORKFLOW_SCRIPTS') ? [WORKFLOW_TOOL_NAME] : []),
])
CUSTOM_AGENT_DISALLOWED_TOOLS = new Set([...ALL_AGENT_DISALLOWED_TOOLS])

ASYNC_AGENT_ALLOWED_TOOLS = new Set([
  FILE_READ, WEB_SEARCH, TODO_WRITE, GREP, WEB_FETCH, GLOB,
  ...SHELL_TOOL_NAMES,
  FILE_EDIT, FILE_WRITE, NOTEBOOK_EDIT,
  SKILL, SYNTHETIC_OUTPUT, TOOL_SEARCH,
  ENTER_WORKTREE, EXIT_WORKTREE,
])

IN_PROCESS_TEAMMATE_ALLOWED_TOOLS = new Set([
  TASK_CREATE, TASK_GET, TASK_LIST, TASK_UPDATE,
  SEND_MESSAGE,
  ...(feature('AGENT_TRIGGERS') ? [CRON_CREATE, CRON_DELETE, CRON_LIST] : []),
])

COORDINATOR_MODE_ALLOWED_TOOLS = new Set([
  AGENT, TASK_STOP, SEND_MESSAGE, SYNTHETIC_OUTPUT,
])
```

Comment block at `constants/tools.ts:90-102` explains the BLOCKED-FOR-ASYNC-AGENTS rationale: `AgentTool` (recursion), `TaskOutputTool` (recursion), `ExitPlanModeTool` (main-thread-only abstraction), `TaskStopTool` (main-thread state), `TungstenTool` (singleton virtual terminal). MCP / ListMcpResources / ReadMcpResource are flagged TBD.

### 6.8 Fork-subagent builder pseudocode

`buildForkedMessages(directive, assistantMessage)` (`forkSubagent.ts:107-169`):
```
fullAssistantMessage = clone(assistantMessage) with new uuid + cloned content array
toolUseBlocks = assistantMessage.message.content.filter(b => b.type === 'tool_use')
if toolUseBlocks.length === 0:
  log error
  return [createUserMessage({content: [{type:'text', text: buildChildMessage(directive)}]})]
toolResultBlocks = toolUseBlocks.map(b => ({
  type: 'tool_result',
  tool_use_id: b.id,
  content: [{type: 'text', text: FORK_PLACEHOLDER_RESULT}],   // const, identical for all forks
}))
toolResultMessage = createUserMessage({content: [
  ...toolResultBlocks,
  {type: 'text', text: buildChildMessage(directive)},   // ONLY per-child-distinct block
]})
return [fullAssistantMessage, toolResultMessage]
```
`FORK_PLACEHOLDER_RESULT = 'Fork started — processing in background'` (`forkSubagent.ts:93`).

`buildChildMessage(directive)` is the verbatim 10-rule fork directive boilerplate at `forkSubagent.ts:171-198` wrapped in `<fork-boilerplate>...</fork-boilerplate>` (`FORK_BOILERPLATE_TAG = 'fork-boilerplate'`, `constants/xml.ts:63`), followed by `${FORK_DIRECTIVE_PREFIX}${directive}` (`FORK_DIRECTIVE_PREFIX = 'Your directive: '`, `constants/xml.ts:66`).

`buildWorktreeNotice(parentCwd, worktreeCwd)` (`forkSubagent.ts:205-210`):
> "You've inherited the conversation context above from a parent agent working in `${parentCwd}`. You are operating in an isolated git worktree at `${worktreeCwd}` — same repository, same relative file structure, separate working copy. Paths in the inherited context refer to the parent's working directory; translate them to your worktree root. Re-read files before editing if the parent may have modified them since they appear in the context. Your changes stay in this worktree and will not affect the parent's files."

`isInForkChild(messages)` (`forkSubagent.ts:78-89`): scans for any user message whose array content contains a text block including `<${FORK_BOILERPLATE_TAG}>`.

### 6.9 Constants table

| Name | Value | Source |
|---|---|---|
| `PROGRESS_THRESHOLD_MS` | 2000 | `AgentTool.tsx:63` |
| Auto-background ms | 120_000 (when env or `tengu_auto_background_agents`); 0 = disabled | `AgentTool.tsx:72-77` |
| `SUMMARY_INTERVAL_MS` | 30_000 | `agentSummary.ts:26` |
| `BLUR_DELAY_MS` | 5 * 60_000 (5 min) | `useAwaySummary.ts:13` |
| `RECENT_MESSAGE_WINDOW` (away) | 30 | `awaySummary.ts:16` |
| Auto-dream `minHours` default | 24 | `autoDream.ts:63-66` |
| Auto-dream `minSessions` default | 5 | `autoDream.ts:63-66` |
| `SESSION_SCAN_INTERVAL_MS` | 10 * 60 * 1000 (10 min) | `autoDream.ts:56` |
| `HOLDER_STALE_MS` | 60 * 60 * 1000 (1 hr) | `consolidationLock.ts:20` |
| `LOCK_OPTIONS.retries` (mailbox) | `{retries: 10, minTimeout: 5, maxTimeout: 100}` | `teammateMailbox.ts:35-41` |
| `READ_FILE_STATE_CACHE_SIZE` (worker default) | from `utils/fileStateCache` | `runAgent.ts:48-50` |
| `EXPLORE_AGENT_MIN_QUERIES` | 3 | `built-in/exploreAgent.ts:59` |
| `FORK_AGENT.maxTurns` | 200 | `forkSubagent.ts:65` |
| `FORK_PLACEHOLDER_RESULT` | `'Fork started — processing in background'` | `forkSubagent.ts:93` |
| `FORK_DIRECTIVE_PREFIX` | `'Your directive: '` | `constants/xml.ts:66` |
| `FORK_BOILERPLATE_TAG` | `'fork-boilerplate'` | `constants/xml.ts:63` |
| `TEAMMATE_MESSAGE_TAG` | `'teammate-message'` | `constants/xml.ts:52` |
| `MAX_TRACKED_SOURCES` (cache-break) | per spec 22 | `services/api/promptCacheBreakDetection.ts` |
| Tool-use-summary truncate | 300 chars per input/output JSON | `toolUseSummaryGenerator.ts:102-107` |
| `INTERNAL_WORKER_TOOLS` | {TeamCreate, TeamDelete, SendMessage, SyntheticOutput} | `coordinator/coordinatorMode.ts:29-34` |
| `ONE_SHOT_BUILTIN_AGENT_TYPES` | {`Explore`, `Plan`} | `tools/AgentTool/constants.ts` |

There is **no explicit concurrency cap** on parallel agent spawns inside this layer. The coordinator prompt advises parallelism (`coordinator/coordinatorMode.ts:212-218`); foreground/background gating in `AgentTool.tsx` is the only hard sequencing primitive (sync runs hold the parent turn open until completion or background-promotion). Mailbox lock retries (10×, 5–100 ms backoff) bound concurrent inbox writes per recipient.

### 6.10 Summary prompt (verbatim, `agentSummary.ts:28-44`)

```
Describe your most recent action in 3-5 words using present tense (-ing). Name the file or function, not the branch. Do not use tools.
${prevLine}
Good: "Reading runAgent.ts"
Good: "Fixing null check in validate.ts"
Good: "Running auth module tests"
Good: "Adding retry logic to fetchUser"

Bad (past tense): "Analyzed the branch diff"
Bad (too vague): "Investigating the issue"
Bad (too long): "Reviewing full branch diff and AgentTool.tsx integration"
Bad (branch name): "Analyzed adam/background-summary branch diff"
```
where `prevLine = previousSummary ? '\nPrevious: "${previousSummary}" — say something NEW.\n' : ''`.

### 6.11 Away-summary prompt (verbatim, `awaySummary.ts:18-23`)

```
${memoryBlock}The user stepped away and is coming back. Write exactly 1-3 short sentences. Start by stating the high-level task — what they are building or debugging, not implementation details. Next: the concrete next step. Skip status reports and commit recaps.
```
where `memoryBlock = memory ? 'Session memory (broader context):\n${memory}\n\n' : ''`.

### 6.12 Consolidation prompt (verbatim, `services/autoDream/consolidationPrompt.ts:10-65`)

The full prompt is at the cited file/range, including:
- "# Dream: Memory Consolidation"
- Phase 1 — Orient
- Phase 2 — Gather recent signal (daily logs `logs/YYYY/MM/YYYY-MM-DD.md`, drifted memories, `grep -rn "<narrow term>" ${transcriptDir}/ --include="*.jsonl" | tail -50`)
- Phase 3 — Consolidate
- Phase 4 — Prune and index (`${ENTRYPOINT_NAME}` ≤ `${MAX_ENTRYPOINT_LINES}` lines, ≤ ~25 KB; one-line entries `- [Title](file.md) — one-line hook`)
- Optional `extra` appended under "## Additional context" when non-empty

### 6.13 Tool-use-summary prompt (verbatim, `toolUseSummaryGenerator.ts:15-24`)

```
Write a short summary label describing what these tool calls accomplished. It appears as a single-line row in a mobile app and truncates around 30 characters, so think git-commit-subject, not sentence.

Keep the verb in past tense and the most distinctive noun. Drop articles, connectors, and long location context first.

Examples:
- Searched in auth/
- Fixed NPE in UserService
- Created signup endpoint
- Read config.json
- Ran failing tests
```

### 6.14 Coordinator gate constants (verbatim)

```ts
// coordinator/coordinatorMode.ts:36-41
export function isCoordinatorMode(): boolean {
  if (feature('COORDINATOR_MODE')) {
    return isEnvTruthy(process.env.CLAUDE_CODE_COORDINATOR_MODE)
  }
  return false
}
```

```ts
// coordinator/coordinatorMode.ts:29-34
const INTERNAL_WORKER_TOOLS = new Set([
  TEAM_CREATE_TOOL_NAME,
  TEAM_DELETE_TOOL_NAME,
  SEND_MESSAGE_TOOL_NAME,
  SYNTHETIC_OUTPUT_TOOL_NAME,
])
```

### 6.15 Fork gate (verbatim, `forkSubagent.ts:32-39`)

```ts
export function isForkSubagentEnabled(): boolean {
  if (feature('FORK_SUBAGENT')) {
    if (isCoordinatorMode()) return false
    if (getIsNonInteractiveSession()) return false
    return true
  }
  return false
}
```

### 6.16 Built-in agent registry (verbatim, `builtInAgents.ts:22-72`)

```ts
export function getBuiltInAgents(): AgentDefinition[] {
  if (
    isEnvTruthy(process.env.CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS) &&
    getIsNonInteractiveSession()
  ) {
    return []
  }
  if (feature('COORDINATOR_MODE')) {
    if (isEnvTruthy(process.env.CLAUDE_CODE_COORDINATOR_MODE)) {
      const { getCoordinatorAgents } =
        require('../../coordinator/workerAgent.js') as typeof import('../../coordinator/workerAgent.js')
      return getCoordinatorAgents()
    }
  }
  const agents: AgentDefinition[] = [
    GENERAL_PURPOSE_AGENT,
    STATUSLINE_SETUP_AGENT,
  ]
  if (areExplorePlanAgentsEnabled()) {
    agents.push(EXPLORE_AGENT, PLAN_AGENT)
  }
  const isNonSdkEntrypoint =
    process.env.CLAUDE_CODE_ENTRYPOINT !== 'sdk-ts' &&
    process.env.CLAUDE_CODE_ENTRYPOINT !== 'sdk-py' &&
    process.env.CLAUDE_CODE_ENTRYPOINT !== 'sdk-cli'
  if (isNonSdkEntrypoint) {
    agents.push(CLAUDE_CODE_GUIDE_AGENT)
  }
  if (
    feature('VERIFICATION_AGENT') &&
    getFeatureValue_CACHED_MAY_BE_STALE('tengu_hive_evidence', false)
  ) {
    agents.push(VERIFICATION_AGENT)
  }
  return agents
}
```

`areExplorePlanAgentsEnabled = () => feature('BUILTIN_EXPLORE_PLAN_AGENTS') && getFeatureValue_CACHED('tengu_amber_stoat', true)` (`builtInAgents.ts:13-20`).

---

## 7. Error Handling

- **Worker abort**: `agentAbortController.signal.aborted` after the loop ⇒ throw `AbortError` (`runAgent.ts:808-810`); `runAsyncAgentLifecycle` catches and emits `'killed'` notification with `extractPartialResult`.
- **MCP cleanup is in `finally`**: runs on normal completion, abort, or error (`runAgent.ts:817-818`).
- **Hooks cleanup**: `clearSessionHooks(rootSetAppState, agentId)` in finally (`runAgent.ts:820-822`).
- **Spawn permission denial path**: `spawnTeammate` throws when `team_name` missing or team file absent (`spawnMultiAgent.ts:323-328, 489-493`). Caller sees the error as a tool error.
- **Pane backend pre-flight**: `handleSpawn` only catches `detectAndGetBackend` errors when `getTeammateModeFromSnapshot() === 'auto'`. Explicit `tmux` mode propagates the error so the user sees `getTmuxInstallInstructions()` (`spawnMultiAgent.ts:1052-1064`).
- **It2 cancellation**: `setupResult === 'cancelled'` ⇒ throw `'Teammate spawn cancelled - iTerm2 setup required'` (`spawnMultiAgent.ts:362-364`).
- **Fork without tool_use blocks**: `buildForkedMessages` logs error and returns a single boilerplate user message (`forkSubagent.ts:127-138`).
- **Recursive fork**: thrown from `AgentTool.call`: `'Fork is not available inside a forked worker. Complete your task directly using your tools.'` (`AgentTool.tsx:332-334`).
- **Auto-dream fork failure**: `failDreamTask` + `rollbackConsolidationLock(priorMtime)` so the time gate trips again next opportunity (`autoDream.ts:267-271`).
- **Mailbox lock failure** (e.g. ENOENT): `markMessageAsReadByIndex` and `markMessagesAsRead` swallow ENOENT silently; other errors `logError` and continue (`teammateMailbox.ts:251-265, 324-338`).
- **`completeAsyncAgent` ordering**: must precede classifier and worktree cleanup so `TaskOutput(block=true)` callers unblock even if those hang (gh-20236, `agentToolUtils.ts:599-604`; `AgentTool.tsx:953-957`).
- **Fork-resume reconstruction failure**: `'Cannot resume fork agent: unable to reconstruct parent system prompt'` (`resumeAgent.ts:142-146`).

### 7.1 Plan-mode approval gating (worker → leader handshake)

A teammate worker that is launched with `--plan-mode-required` (`spawnMultiAgent.ts:243-246`, propagated as a CLI flag) starts in `permissionMode: 'plan'`. When the worker reaches a point where it would exit plan mode, two envelopes mediate the round-trip:

- **`plan_approval_request`** (`teammateMailbox.ts` — see §6.5 schema) — emitted by the worker when it calls `ExitPlanModeV2Tool` (`tools/ExitPlanModeTool/ExitPlanModeV2Tool.ts:406`, gated by `isAgentSwarmsEnabled() && planModeRequired`). The worker writes this envelope to the **leader's** inbox via the mailbox bridge, then awaits the response.
- **`plan_approval_response`** — written by the leader's UI (after the user accepts/rejects the plan); carries an optional `permissionMode` override applied to the worker's `toolPermissionContext.mode` on resumption.

**Algorithm**:
1. Worker reaches `ExitPlanModeV2Tool` while `permissionMode === 'plan'` AND `isPlanModeRequired()` from the swarm side. Worker emits `plan_approval_request{plan, requestId}` to the leader's inbox; the worker's tool call **blocks** awaiting a matching `plan_approval_response{requestId, approved, permissionMode?}`.
2. Leader's `useSwarmPermissionPoller` (or sibling poller; same dispatch oracle `isStructuredProtocolMessage`) routes the request to the leader's plan-approval dialog. User approves with optional mode change (`'default' | 'acceptEdits' | 'auto' | 'bypassPermissions'`) or rejects.
3. Leader writes `plan_approval_response` back to the worker's inbox.
4. Worker resumes: on `approved=true`, sets `toolPermissionContext.mode = response.permissionMode ?? 'default'` and the tool succeeds. On `approved=false`, the tool returns an error string and the worker remains in `'plan'` mode.

**Edge case** — leader process exit while a request is outstanding: the worker's await is bound by mailbox lock retries (10×, 5–100 ms backoff) on the read side; the *write* of the request succeeds against a stale inbox file. There is no explicit timeout in the worker — it polls indefinitely. Worker abort (parent kill of the worker process) is the only escape hatch.

`main.tsx:2911` independently forces `permissionMode: 'plan'` for every swarm worker turn when `getTeammateUtils().isPlanModeRequired()` regardless of the user's own mode preference, so a worker spawned with `--plan-mode-required` cannot accidentally drop out of plan mode without the round-trip above.

### 7.2 Crash, timeout, and partial-completion edge cases

- **Coordinator process crash with in-flight workers**: when the coordinator (orchestrator) terminates while async workers (`runAsyncAgentLifecycle`) are running, the workers' `agentAbortController` is the *only* signal that propagates. Async workers are *not* tracked across coordinator restart — there is no on-disk handoff for "in-flight async agent". Sidechain transcripts persist (`recordSidechainTranscript`, spec 41), so a `claude --resume` on the crashed leader can read the partial worker transcript, but the worker's MCP cleanup, hooks cleanup, and `cleanupAgentTracking` (`runAgent.ts:817-822, 832`) **only fire if the parent process is still alive**. A hard `SIGKILL` of the coordinator leaks: agent transcript-subdir registry, Perfetto trace registration, agent-color reservation in `agentColorManager`, todo entries, and shell tasks (`killShellTasksForAgent` is also in the same `finally` block).
- **Multi-process teammate (swarm) leader crash**: each teammate is its own process. Killing the leader does **not** kill workers. Workers continue to write to a leader inbox file that no one reads; the next leader startup (via `claude --resume`) will see an unread inbox but cannot reattach to the worker's pane backend without the original `tmux`/`iTerm2` session still being alive (`reconnection.ts` handles this best-effort).
- **Worker timeout**: there is **no time-based timeout** on a worker. The only bounds are (a) `maxTurns` (default 200 for fork; otherwise inherited from `agentDefinition.maxTurns`), (b) explicit `agentAbortController.abort()` from a UI-triggered kill or `TaskStop`, and (c) `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` forcing sync-only (where the parent turn cannot proceed but the worker still runs unbounded until `maxTurns` or abort). Long-running tool calls inside a worker (e.g. a hung `Bash`) inherit the BashTool timeout — they do not bubble up as a *worker-level* timeout.
- **Partial completion**: when a worker is aborted mid-turn (`AbortError` thrown after `query()` exits), `runAsyncAgentLifecycle` calls `extractPartialResult(messages, agentId)` (`agentToolUtils.ts`) to assemble whatever assistant content was emitted before abort. Result is delivered as a `'killed'` task notification with the partial text, *not* a normal completion envelope. If the worker had not yet emitted any assistant block (aborted during the first user turn), `extractPartialResult` returns an empty string and the notification reads "(no output before kill)".
- **Mid-flight backgrounding race** (sync→async promotion): `AgentTool.tsx:918` does `agentIterator.return(undefined).catch(()=>{})` then waits ~1 s before re-driving the same iterator from the async lifecycle. If MCP cleanup or hooks cleanup take longer than 1 s, they overlap with the new iterator's startup. There is no test for this in the leak; `~1s` is asserted but unverified (see §11 hardest-to-verify claim).

---

## 8. Configuration & Env

| Var / setting | Effect |
|---|---|
| `CLAUDE_CODE_COORDINATOR_MODE` (env, truthy) | Activates coordinator mode when `feature('COORDINATOR_MODE')` is on. |
| `CLAUDE_CODE_SIMPLE` (env, truthy) | Coordinator's worker-tools list shrinks to `Bash, Read, Edit` (+ MCP). |
| `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` (env, truthy) | Forces sync-only agent execution; `run_in_background` removed from the schema (`AgentTool.tsx:66-68, 122-124`). |
| `CLAUDE_AUTO_BACKGROUND_TASKS` (env, truthy) OR GB `tengu_auto_background_agents=true` | Enables auto-background after 120 s. |
| `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS` (env, truthy) | Empty registry when also non-interactive (SDK blank slate). |
| `CLAUDE_CODE_ENTRYPOINT` ∈ {`sdk-ts`, `sdk-py`, `sdk-cli`} | Suppresses `CLAUDE_CODE_GUIDE_AGENT` from registry. |
| `CLAUDE_CODE_REMOTE_MEMORY_DIR` | Remaps `local`-scope agent memory to a per-project subdir under that mount. |
| `CLAUDE_CODE_AGENT` | Set by `main.tsx:1115-1118` from `--agent` when `feature('BG_SESSIONS')` is on. |
| `CLAUDE_COWORK_MEMORY_EXTRA_GUIDELINES` | Appended to memory-load prompt's extra guidelines (`agentMemory.ts:167-176`). |
| `TEAMMATE_COMMAND_ENV_VAR` | Override binary for spawned teammates (`spawnMultiAgent.ts:194`). |
| GrowthBook `tengu_slim_subagent_claudemd` (default true) | When false, restores CLAUDE.md to read-only built-ins. |
| GrowthBook `tengu_amber_stoat` (default true) | A/B kills Explore+Plan when false. |
| GrowthBook `tengu_hive_evidence` (default false) | Enables Verification agent registration. |
| GrowthBook `tengu_onyx_plover` | Auto-dream `{minHours, minSessions, enabled}` config. |
| GrowthBook `tengu_sedge_lantern` (default false) | Enables Away-summary. |
| GrowthBook `tengu_scratch` | Adds scratchpad path to coordinator user context. |
| GrowthBook `tengu_explore_agent` | (ant) tweaks Explore model resolution at runtime (`built-in/exploreAgent.ts:77`). |
| Settings: `teammateDefaultModel` (`getGlobalConfig`) | `null` ⇒ follow leader; `undefined` ⇒ hardcoded fallback; string ⇒ `parseUserSpecifiedModel(...)` (`spawnMultiAgent.ts:72-101`). |
| Settings: `autoDreamEnabled` | User override that wins over GB `tengu_onyx_plover.enabled` (`services/autoDream/config.ts:13-21`). |

---

## 9. Telemetry / Logging

Events emitted by this layer (analytics service is spec 26 — schemas owned there):
- `tengu_agent_tool_selected` (`AgentTool.tsx:419-428`) — `agent_type`, `model`, `source`, `color`, `is_built_in_agent`, `is_resume`, `is_async`, `is_fork`.
- `tengu_agent_tool_completed` (`agentToolUtils.ts:322-334`) — duration, char counts, tool count, tokens.
- `tengu_agent_tool_terminated` (`AgentTool.tsx:996-1007, 1131-1140`; `agentToolUtils.ts:646-656`) — reasons: `'user_kill_async'`, `'user_cancel_background'`, `'user_cancel_sync'`.
- `tengu_agent_tool_remote_launched` (`AgentTool.tsx:466-468`).
- `tengu_agent_memory_loaded` (`AgentTool.tsx:524-530`) — `scope`, `source: 'subagent'`.
- `tengu_auto_mode_decision` (`agentToolUtils.ts:431-460`) — handoff classifier results.
- `tengu_cache_eviction_hint` (`agentToolUtils.ts:340-345`) — `scope: 'subagent_end'`.
- `tengu_coordinator_mode_switched` (`coordinatorMode.ts:71-73`).
- `tengu_auto_dream_fired` / `tengu_auto_dream_completed` / `tengu_auto_dream_failed` (`autoDream.ts:195-269`).

`logForDebugging` is used liberally throughout for trace logs (`[Subagent ${type}]`, `[TeammateMailbox]`, `[autoDream]`, `[AgentSummary]`).

Perfetto trace registration: `registerPerfettoAgent(agentId, agentType, parentId)` and `unregisterPerfettoAgent` flank the worker turn loop when `isPerfettoTracingEnabled()` (`runAgent.ts:355-360, 832`).

---

## 10. Cross-References

- **14↔30 reader-trap (symbol-ownership vs behavior-ownership).** Spec 14 owns the *use* of `isForkSubagentEnabled` (call sites at `AgentTool.tsx:51` import, `:557, :750, :818` invocation). Spec 30 owns the *implementation*: the function body lives at `tools/AgentTool/forkSubagent.ts:32-39`, which is in this spec's tree. Readers searching for "where is `isForkSubagentEnabled` defined?" should look here (spec 30); readers asking "when is it consulted?" should look in spec 14. The same split applies to `FORK_AGENT`, `buildForkedMessages`, `buildChildMessage`, and `isInForkChild` — definitions in spec 30, call sites in spec 14.
- §3 of spec **14** (`AgentTool` schema) is the entry-point shape; `subagent_type`, `model`, `run_in_background`, `name`, `team_name`, `mode`, `isolation`, `cwd` parameters all route into the algorithms above.
- **Spec 42a (`utils/swarm/` long-tail catalog)** owns the per-file *signature inventory* for the 22 swarm files. This spec (30) owns the *architectural description* (§2.1) — backends, permission bridge, in-process runner, ALS isolation. Both specs cite the same files; 42a is the directory atlas, 30 is the system narrative.
- Spec **15** owns the task system (`Task*Tool` family). This layer registers `LocalAgentTask` / `InProcessTeammateTask` / `RemoteAgentTask` task records but does not own their schemas.
- Spec **03** owns `query()` / `QueryEngine`. `runAgent` is a driver that constructs subagent context and calls `query()`.
- Spec **05** owns `getSystemPrompt`, `getUserContext`, `getSystemContext`, `buildEffectiveSystemPrompt` — used by `runAgent` and resume.
- Spec **07** owns `microcompact` and SystemCompactBoundaryMessage handling.
- Spec **22** owns `services/api/promptCacheBreakDetection.ts`'s detection mechanism; this spec only documents lifetime termination via `cleanupAgentTracking(agentId)`.
- Spec **23** owns MCP client construction; `initializeAgentMcpServers` re-uses the memoized `connectToServer` and the inline-vs-shared cleanup distinction.
- Spec **27** owns `isRestrictedToPluginOnly` / `isSourceAdminTrusted`; this spec applies them to skip frontmatter MCP servers and frontmatter hooks for non-admin-trusted agents.
- Spec **29** owns `services/extractMemories/`, `services/SessionMemory/`, and the `memdir/` storage. This spec uses `getSessionMemoryContent`, `createAutoMemCanUseTool`, `getAutoMemPath`.
- Spec **31** owns `proactiveModule.isProactiveActive()` — surfaced here only as one async-trigger predicate.
- Spec **34** / **35** own remote/CCR transports; `'remote'` isolation is a dispatch routing point only.
- Spec **41** owns `recordSidechainTranscript`, `writeAgentMetadata`, `getAgentTranscript`, `readAgentMetadata` — sidechain persistence consumed by resume.

---

## 11. Tests

No test sources ship with the leak. Behavioral expectations to verify on a future re-implementation, derived from comments and code comments:

- `resolveTeammateModel('inherit', null)` falls through to `getDefaultTeammateModel(null)` → eventually `getHardcodedTeammateModelFallback()` (gh-31069 regression — "inherit" must not be passed literally to `--model`).
- `generateUniqueTeammateName('tester', team)` returns `'tester-2'` when `'tester'` exists, then `'tester-3'`, etc., case-insensitive.
- `buildForkedMessages` produces byte-identical message arrays when only the `directive` differs in the trailing text block (cache-share invariant).
- Mid-flight backgrounding releases foreground iterator within `~1s` (`AgentTool.tsx:918`) — required so MCP/hooks finalizers run.
- `completeAsyncAgent` is awaited *before* `classifyHandoffIfNeeded` and `getWorktreeResult` in both async-from-start and backgrounded paths.
- Auto-dream gates fire in order: time → scan-throttle → session — and `priorMtime` rollback restores the lock mtime exactly.
- `filterIncompleteToolCalls` retains all non-assistant messages and assistant messages without tool_use, drops only assistants whose tool_use blocks lack a corresponding tool_result.
- Mailbox `markMessagesAsRead` is a no-op when ENOENT.

---

## 12. Open Questions / Source Gaps

1. **Missing `src/coordinator/workerAgent.ts`** — referenced by lazy `require()` at `tools/AgentTool/builtInAgents.ts:38-40`. Provides `getCoordinatorAgents()`. Not present in the leak. The actual coordinator-mode worker `AgentDefinition` list (worker name, `whenToUse`, system prompt, `disallowedTools`, color) cannot be reconstructed from this tree. Recorded as `missing-leaked-source`.
2. The `feature('VERIFICATION_AGENT')` gate is checked only at `builtInAgents.ts:64-69`; `TaskUpdateTool.ts:335` (cited in dispatch brief) is owned by spec 15 — the actual VerificationAgent invocation path inside TaskUpdate falls under that spec.
3. No explicit per-process **maximum-parallel-agents cap** was found in this layer (`grep MAX_PARALLEL|MAX_AGENT|maxConcurrent` empty). The OS / mailbox-retry semantics + `runAsyncAgentLifecycle`'s independence per-`taskId` are the only governors.
4. `compareAgentsByName` and `resolveAgentOverrides` (`agentDisplay.ts:46-104`) are display-only helpers used by the `/agents` UI and `claude agents` CLI handler — UI surface is owned by spec 37. The dedupe-by-`(agentType, source)` accommodates **git-worktree duplicates**; whether the same dedupe applies inside `getBuiltInAgents` was not located in source.
5. `AgentMemorySnapshot`'s `replaceFromSnapshot` deletes only `*.md` files when overwriting (`agentMemorySnapshot.ts:172-184`) — non-md attachments inside the snapshot dir leak. Whether this is intentional is undocumented.
6. The agentSummary `previousSummary` string lives in closure scope; survives summarization timer ticks but is not persisted across resumes — a resumed agent's first summary will not say "say something NEW" against pre-resume content.
7. ~~Coordinator's tool whitelist filter for the **headless path** is applied via `applyCoordinatorToolFilter` from `utils/toolPool.js` (dynamic-import at `main.tsx:1872-1879`). For the REPL path it lives in `useMergedTools.ts`. The two surfaces must stay in sync; sub-agents working on tool-pool changes must verify both.~~ **Resolved Phase 9.6**: spec 30 §3 (under "Coordinator tool-pool filter") now designates spec 30 as the canonical owner of the call-site invariants for both surfaces.
8. `feature('PROMPT_CACHE_BREAK_DETECTION')` is only one of several lifecycle gates that each track per-agentId state. If the gate flips off mid-session, the existing entries remain in the spec-22 Map until process exit.

---

*End of spec 30.*
