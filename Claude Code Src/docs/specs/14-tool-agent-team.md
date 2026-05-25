# 14 — Agent / Team / SendMessage Tool Surface

> The four tools that expose multi-agent orchestration to the model: `Agent`, `TeamCreate`, `TeamDelete`, `SendMessage`. **Tool surface only** — input schemas, prompts, lifecycle predicates, permission gates, default routing, and the lazy-getter wiring in `tools.ts`. The 35KB primary spawn algorithm in `tools/shared/spawnMultiAgent.ts`, the worker run loop, and `coordinator/` orchestration are owned by spec 30. Read 08 first.

---

## 1. Purpose & Scope

This spec covers the **tool-level public surface** of multi-agent orchestration: what the model sees, what schema it must satisfy, what permissions are checked, and how each tool is wired into the registry.

### IN scope
- `src/tools/AgentTool/` (15 files): tool definition (`AgentTool.tsx`), prompt assembly (`prompt.ts`), built-in agent registry (`builtInAgents.ts`), built-in `Explore`/`Plan`/`general-purpose` definitions (`built-in/`), `AgentDefinition` schema and frontmatter parser (`loadAgentsDir.ts`), `forkSubagent.ts` gating, `agentMemorySnapshot.ts` snapshot dialog gate, `constants.ts`, `agentColorManager.ts`, `agentDisplay.ts`, `agentMemory.ts`, `agentToolUtils.ts`, `resumeAgent.ts` reference, `UI.tsx` reference.
- `src/tools/TeamCreateTool/` (4 files): tool definition + prompt + UI + name constant.
- `src/tools/TeamDeleteTool/` (4 files): tool definition + prompt + UI + name constant.
- `src/tools/SendMessageTool/` (4 files): tool definition + prompt + UI + name constant.
- The lazy-getter circular-dep pattern in `tools.ts:62-72` (`getTeamCreateTool`, `getTeamDeleteTool`, `getSendMessageTool`).
- Tool-level lifecycle predicates: `isReadOnly`, `isConcurrencySafe`, `isEnabled`, `shouldDefer`, `alwaysLoad`, `isDestructive`, `validateInput`, `checkPermissions`, `backfillObservableInput`.
- Feature gates that affect this surface: `BUILTIN_EXPLORE_PLAN_AGENTS`, `PROMPT_CACHE_BREAK_DETECTION`, `FORK_SUBAGENT`, `AGENT_MEMORY_SNAPSHOT`, `COORDINATOR_MODE`, `MONITOR_TOOL` (cleanup hook only), `KAIROS`, `UDS_INBOX`, `VERIFICATION_AGENT`.
- ANT-only branches in this surface: `'remote'` isolation, `auto`-mode passthrough in `checkPermissions`, ANT-only `inherit` model default for `Explore`, ANT-only debug log path, ANT-only structured-message restrictions.

**Handoff with spec 30**: this spec owns the **tool surface** (input schemas, prompt body, `checkPermissions`, registry wiring); spec 30 owns the **runner** (`runAgent.ts` body, `forkSubagent.ts` execution, coordinator orchestration).

**`forkSubagent.ts` symbol-vs-behavior split** (refines §1 boundary; addresses adversarial M4): spec 14 owns the *symbols* `isForkSubagentEnabled` (gate predicate) and `FORK_AGENT` (the `BuiltInAgentDefinition` literal — consumed by 14's selection logic at `AgentTool.tsx:332,335`); spec 30 owns the *behavior* of `buildForkedMessages`, `buildChildMessage`, `buildWorktreeNotice`, and `isInForkChild` (message construction and the in-flight worktree-notice append). The `FORK_AGENT` definition object is **declared in `forkSubagent.ts` but cited by §6.5 of this spec** because it is part of the agent registry surface; its consumers (`buildForkedMessages` etc.) are not. Spec 30's source map MUST list `forkSubagent.ts` in IN-scope and reciprocate this split.

### OUT of scope (refer by spec #)
- `tools/shared/spawnMultiAgent.ts` (multi-agent spawn body), worker pool, mailbox/inbox transport → **30**.
- `coordinator/coordinatorMode.ts`, `coordinator/workerAgent.ts`, coordinator agent definitions → **30**.
- `runAgent.ts` worker turn loop, message forking, MCP cleanup, todo cleanup, perfetto registration (covered only at the flag-citation level here) → **30**.
- `services/AgentSummary/` background summarization service → **30**.
- Permission decision tree, deny-rule matching, auto-mode classifier semantics → **09**.
- `Tool` interface, `buildTool`, registry assembly, `ToolUseContext` shape → **08**.
- `TaskCreate/Update/Get/List/Stop` tools (the v2 task system referenced by `team-name` task lists) → **15**.
- `AgentId`, agent identity types, session IDs, mailbox file paths → **41**.
- `LocalAgentTask`, `RemoteAgentTask`, `InProcessTeammateTask` registration → **15** (task lifecycle), **30** (transport).

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/tools/AgentTool/AgentTool.tsx` | 1397 | `AgentTool` definition, input/output schemas, `call`, `checkPermissions`, fork routing, isolation routing, multi-agent spawn dispatch, async-vs-sync gating |
| `src/tools/AgentTool/prompt.ts` | 287 | `getPrompt(agents, isCoordinator?, allowedAgentTypes?)`, `formatAgentLine`, `shouldInjectAgentListInMessages`, fork-aware sections, ANT-only `'remote'` isolation note |
| `src/tools/AgentTool/builtInAgents.ts` | 72 | `areExplorePlanAgentsEnabled` (`BUILTIN_EXPLORE_PLAN_AGENTS`), `getBuiltInAgents` (lazy `coordinatorMode` require, `VERIFICATION_AGENT` gate, SDK skip env var, non-SDK Code Guide gate) |
| `src/tools/AgentTool/built-in/exploreAgent.ts` | 83 | `EXPLORE_AGENT` definition, ANT-only `inherit` vs non-ANT `haiku` default, embedded-search hint branch |
| `src/tools/AgentTool/built-in/planAgent.ts` | 92 | `PLAN_AGENT` definition (`'inherit'` model unconditionally) |
| `src/tools/AgentTool/built-in/generalPurposeAgent.ts` | 34 | `GENERAL_PURPOSE_AGENT` definition (`tools: ['*']`, no model — uses `getDefaultSubagentModel`) |
| `src/tools/AgentTool/loadAgentsDir.ts` | 755 | `AgentJsonSchema`, `AgentDefinition` types (Built-in/Custom/Plugin), frontmatter parser including `'remote'` isolation gate at lines 94/610, `getActiveAgentsFromList`, `filterAgentsByMcpRequirements`, `hasRequiredMcpServers`, type guards |
| `src/tools/AgentTool/constants.ts` | 13 | `AGENT_TOOL_NAME='Agent'`, `LEGACY_AGENT_TOOL_NAME='Task'`, `VERIFICATION_AGENT_TYPE='verification'`, `ONE_SHOT_BUILTIN_AGENT_TYPES=Set('Explore','Plan')` |
| `src/tools/AgentTool/forkSubagent.ts` | 210 | `isForkSubagentEnabled` (`FORK_SUBAGENT`), `FORK_AGENT`, `buildForkedMessages`, `buildWorktreeNotice`, `isInForkChild` |
| `src/tools/AgentTool/agentMemorySnapshot.ts` | 197 | Project-snapshot copy/sync for `memory:` agents (read by main.tsx:2258 dialog flow) |
| `src/tools/AgentTool/runAgent.ts` | 973 | Worker run loop. **Only cited here for the `PROMPT_CACHE_BREAK_DETECTION` cleanup at line 824 and the ANT-only debug log at line 362.** Body owned by spec 30 |
| `src/tools/TeamCreateTool/TeamCreateTool.ts` | 240 | `TeamCreateTool` definition, `inputSchema`, `validateInput`, `call` (writes team file, registers task list, sets AppState) |
| `src/tools/TeamCreateTool/prompt.ts` | 113 | `getPrompt()` — full multi-agent workflow guidance |
| `src/tools/TeamCreateTool/constants.ts` | 1 | `TEAM_CREATE_TOOL_NAME='TeamCreate'` |
| `src/tools/TeamDeleteTool/TeamDeleteTool.ts` | 139 | `TeamDeleteTool` definition; empty input; refuses delete if active members |
| `src/tools/TeamDeleteTool/prompt.ts` | 16 | `getPrompt()` — short cleanup description |
| `src/tools/TeamDeleteTool/constants.ts` | 1 | `TEAM_DELETE_TOOL_NAME='TeamDelete'` |
| `src/tools/SendMessageTool/SendMessageTool.ts` | 917 | `SendMessageTool` definition; structured-message Zod union; routing for in-process subagent / mailbox / broadcast / shutdown / plan-approval; `UDS_INBOX` cross-session bridge/uds branches |
| `src/tools/SendMessageTool/prompt.ts` | 49 | `getPrompt()` + `DESCRIPTION='Send a message to another agent'` |
| `src/tools/SendMessageTool/constants.ts` | 1 | `SEND_MESSAGE_TOOL_NAME='SendMessage'` |
| `src/tools.ts` | 389 | Lazy getters for circular-dep break (lines 62–72), `isAgentSwarmsEnabled()` gating (lines 228–230) |
| `src/utils/agentSwarmsEnabled.ts` | 45 | Single gate function used by all four tools' `isEnabled` |
| `src/main.tsx:2258-2273` | — | `AGENT_MEMORY_SNAPSHOT` dialog dispatch |

### 2.2 Source coverage

| Source | Read fully | Sampled | Notes |
|---|---|---|---|
| `AgentTool.tsx` | partial | header (1–300), tail (1255–1397) | `call` body lines 300–1255 owned by spec 30; tool-surface fields and `checkPermissions` at 1281 fully captured here |
| `prompt.ts` (Agent) | ✅ | | All 287 lines |
| `builtInAgents.ts` | ✅ | | All 72 lines |
| `built-in/exploreAgent.ts` | ✅ | | All 83 lines |
| `built-in/planAgent.ts` | ✅ | | All 92 lines |
| `built-in/generalPurposeAgent.ts` | ✅ | | All 34 lines |
| `loadAgentsDir.ts` | partial | 1–200 (schemas/types), 600–630 (isolation parser) | Frontmatter parser body and disk discovery owned by spec 30 |
| `constants.ts` (Agent) | ✅ | | All 13 lines |
| `agentMemorySnapshot.ts` | ✅ | | All 197 lines |
| `runAgent.ts` | sampled | 350–380, 810–860 | Only flag-citation lines (362, 824) read; rest owned by spec 30 |
| `forkSubagent.ts` | ✅ via grep | | Existence + flag wiring confirmed; body owned by spec 30 |
| `TeamCreateTool.ts` | ✅ | | All 240 lines |
| `TeamCreateTool/prompt.ts` | ✅ | | All 113 lines |
| `TeamDeleteTool.ts` | ✅ | | All 139 lines |
| `TeamDeleteTool/prompt.ts` | ✅ | | All 16 lines |
| `SendMessageTool.ts` | partial | 1–300, 520–700, 700–917 | Routing handlers (300–520) sampled — same patterns as the message/broadcast/shutdown handlers shown |
| `SendMessageTool/prompt.ts` | ✅ | | All 49 lines |
| `tools.ts` | already covered in spec 08 | — | Lines 62–72, 226, 228–230 cited here |
| `agentSwarmsEnabled.ts` | ✅ | | All 45 lines |

### 2.3 Imports / Imported by

`AgentTool.tsx` imports from `Tool.ts`, `tools.ts` (for `assembleToolPool`), `coordinator/coordinatorMode.js`, `tasks/LocalAgentTask`, `tasks/RemoteAgentTask`, `tools/shared/spawnMultiAgent.js`, `services/AgentSummary/agentSummary.js`, `services/analytics/growthbook.js`, `utils/agentSwarmsEnabled.js`, `utils/permissions/permissions.js`, `utils/teleport.js`, `utils/worktree.js`, plus all sub-files of `AgentTool/`.

`AgentTool` is statically imported by `tools.ts:33` (it is in the static block — no feature gate around the import itself; the `isEnabled()` check controls inclusion).

`TeamCreateTool`, `TeamDeleteTool`, `SendMessageTool` are **lazy-required** in `tools.ts:62-72` and conditionally inserted into `getAllBaseTools()` at `tools.ts:226` (SendMessage unconditionally) and `tools.ts:228-230` (TeamCreate/Delete behind `isAgentSwarmsEnabled()`).

### 2.4 On-disk pattern (per spec 08 §2.5)

Each of the four tool dirs follows the canonical `<Name>Tool.{ts,tsx}` + `prompt.ts` + `UI.tsx` + `constants.ts` layout. AgentTool is the most-elaborated instance with 15 files including built-in agent definitions, color manager, memory subsystem, fork helpers, snapshot subsystem, and resume helpers.

---

## 3. Public Interface (Contract)

### 3.1 AgentTool

#### Identity & registry shape
- `name = AGENT_TOOL_NAME = 'Agent'`
- `aliases = [LEGACY_AGENT_TOOL_NAME = 'Task']` — legacy wire name preserved for backward-compat with permission rules, hooks, and resumed sessions (`constants.ts:1-3`).
- `searchHint = 'delegate work to a subagent'` (`AgentTool.tsx:227`).
- `maxResultSizeChars = 100_000` (`AgentTool.tsx:229`).
- No `shouldDefer`, no `alwaysLoad` — AgentTool is always loaded into the prompt.
- `isReadOnly: () => true` with comment "delegates permission checks to its underlying tools" (`AgentTool.tsx:1264-1266`).
- `isConcurrencySafe: () => true` (`AgentTool.tsx:1273-1275`).
- `getActivityDescription(input) => input?.description ?? 'Running task'` (`AgentTool.tsx:1278-1280`).

#### Input schema (verbatim, see §6.2)

The schema is composed via `lazySchema` in three layers: `baseInputSchema`, `multiAgentInputSchema` (defined inside `fullInputSchema`), and the exported `inputSchema` which `.omit()`s gated fields when their backing feature is off.

Layered fields:
- Base: `description: string`, `prompt: string`, `subagent_type?: string`, `model?: 'sonnet'|'opus'|'haiku'`, `run_in_background?: boolean`.
- Multi-agent merge: `name?: string`, `team_name?: string`, `mode?: PermissionMode`.
- Extension: `isolation?: 'worktree' | ('worktree'|'remote' if ANT)`, `cwd?: string`.

Gating:
- Build-time check `"external" === 'ant'` (literal text) widens `isolation` to include `'remote'` only on ANT builds (`AgentTool.tsx:94-99`). External builds get a one-element enum literal.
- `feature('KAIROS') ? full : full.omit({ cwd: true })` — `cwd` is only exposed in KAIROS builds (`AgentTool.tsx:111-113`).
- `isBackgroundTasksDisabled || isForkSubagentEnabled() ? schema.omit({ run_in_background: true }) : schema` — `run_in_background` is hidden when fork mode is on or when `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` is truthy (`AgentTool.tsx:122-124`).
- `isBackgroundTasksDisabled` is captured at module load (`AgentTool.tsx:66-68`).

#### Output schema

`outputSchema = z.union([syncOutputSchema, asyncOutputSchema])` (`AgentTool.tsx:141-155`). Two private (non-exported) output shapes are also returned through the internal `Output` union: `TeammateSpawnedOutput`, `RemoteLaunchedOutput` — kept out of the exported schema for DCE. See §6.4.

#### `prompt()` assembly

`AgentTool.prompt({ agents, tools, getToolPermissionContext, allowedAgentTypes })` (`AgentTool.tsx:197-225`):

1. Reads `toolPermissionContext = await getToolPermissionContext()`.
2. Walks `tools` to collect MCP server names from any `mcp__<server>__<tool>` names.
3. `agentsWithMcpRequirementsMet = filterAgentsByMcpRequirements(agents, mcpServersWithTools)`.
4. `filteredAgents = filterDeniedAgents(..., toolPermissionContext, AGENT_TOOL_NAME)` (deny-rule filter from spec 09).
5. `isCoordinator = feature('COORDINATOR_MODE') ? isEnvTruthy(process.env.CLAUDE_CODE_COORDINATOR_MODE) : false` — uses inline env check (not the `coordinatorModule.isCoordinatorMode()` import) to avoid module-init circularity (comment at `AgentTool.tsx:221-223`).
6. Returns `getPrompt(filteredAgents, isCoordinator, allowedAgentTypes)`.

`getPrompt` itself (`prompt.ts`) builds:
- A shared core prompt header (`prompt.ts:202-212`).
- An `agentListSection` that is either inline `formatAgentLine` lines or a static "see system-reminder" pointer when `shouldInjectAgentListInMessages()` returns true (`prompt.ts:194-199`).
- For coordinator mode, **only** the shared header is returned (`prompt.ts:216-218`).
- For non-coordinator mode: a `whenNotToUseSection` (omitted when fork is enabled), `concurrencyNote` (only shown when not listing-via-attachment AND `getSubscriptionType() !== 'pro'`), background-task usage notes (only when `!CLAUDE_CODE_DISABLE_BACKGROUND_TASKS && !isInProcessTeammate() && !forkEnabled`), the SendMessage continuation note, the `worktree` isolation note, the **ANT-only** `'remote'` isolation note (gated `process.env.USER_TYPE === 'ant'` at `prompt.ts:273`), in-process-teammate / teammate restrictions, an optional `whenToForkSection`, the `writingThePromptSection`, and either `forkExamples` or `currentExamples` based on the fork gate.

Verbatim prompt body — see §6.1.

#### `description()`

Returns the literal string `'Launch a new agent'` (`AgentTool.tsx:230-232`).

#### `call()` — top-level routing (full body in spec 30)

Decision tree at the tool-surface level (`AgentTool.tsx:239-1262`):

1. `model = isCoordinatorMode() ? undefined : modelParam` — coordinator mode forces model resolution to defaults.
2. `permissionMode = appState.toolPermissionContext.mode`.
3. `rootSetAppState = toolUseContext.setAppStateForTasks ?? toolUseContext.setAppState`.
4. **Swarm gate**: `if (team_name && !isAgentSwarmsEnabled()) throw 'Agent Teams is not yet available on your plan.'`.
5. **Teammate-cannot-spawn-teammate**: if `isTeammate() && teamName && name` → throw.
6. **In-process background ban**: if `isInProcessTeammate() && teamName && run_in_background === true` → throw.
7. **Multi-agent spawn**: if `teamName && name` → set agent color → `spawnTeammate(...)` → return `{status: 'teammate_spawned'}` (body owned by spec 30).
8. **Fork-recursion guard**: if `effectiveType === undefined` and either `querySource === \`agent:builtin:${FORK_AGENT.agentType}\`` (template; current value of `FORK_AGENT.agentType` is `'fork'`, so the resolved string is `'agent:builtin:fork'` — but reimplementations MUST reference the constant, not the literal, addressing adversarial M3) or `isInForkChild(messages)` → throw "Fork is not available inside a forked worker."
9. **Agent type lookup**: filter active agents by `allowedAgentTypes` (when set), then by deny rules, then `find(a => a.agentType === effectiveType)`. If not found, distinguish denied-by-rule vs not-existing in the error.
10. **MCP requirement gate**: poll `mcp.clients` up to 30s waiting for pending required servers; fail fast on `failed`. Then extract `serversWithTools` and run `hasRequiredMcpServers`.
11. **`tengu_agent_tool_selected`** logged with all resolved fields.
12. **Effective isolation**: `isolation ?? selectedAgent.isolation`.
13. **ANT-only `'remote'` branch** (`AgentTool.tsx:435-482`): `checkRemoteAgentEligibility` → `teleportToRemote` → `registerRemoteAgentTask` → return `{status: 'remote_launched'}`.
14. **Fork-vs-normal system-prompt branch** at `AgentTool.tsx:495-541` (body in spec 30).
15. **Async decision**: `shouldRunAsync = (run_in_background === true || selectedAgent.background === true || isCoordinator || forceAsync || assistantForceAsync || (proactiveModule?.isProactiveActive() ?? false)) && !isBackgroundTasksDisabled` (`AgentTool.tsx:567`). `forceAsync = isForkSubagentEnabled()`. `assistantForceAsync = feature('KAIROS') ? appState.kairosEnabled : false`.

   **Analytics divergence (adversarial M1)**: the `is_async` field on `tengu_agent_tool_selected` (`AgentTool.tsx:426`) and the `metadata.isAsync` for spawn paths (`AgentTool.tsx:548`) use a **narrower formula** than `shouldRunAsync`: `(run_in_background === true || selectedAgent.background === true) && !isBackgroundTasksDisabled` — i.e. they **drop** `isCoordinator`, `forceAsync` (fork), `assistantForceAsync` (Kairos), and `proactiveModule?.isProactiveActive()`. Therefore an agent that runs async because of coordinator-mode, fork-subagent, Kairos, or proactive will be reported with `is_async=false` in analytics. Telemetry consumers must NOT use `is_async` as ground truth for async-vs-sync execution; the runtime decision is `shouldRunAsync` and analytics under-counts by exactly the four flags listed.
16. **Worker tool pool**: `assembleToolPool({ ...appState.toolPermissionContext, mode: selectedAgent.permissionMode ?? 'acceptEdits' }, appState.mcp.tools)`.
17. **Worktree creation** (when `effectiveIsolation === 'worktree'`).
18. Sync vs async dispatch to `runAgent(...)` (body in spec 30).

#### `checkPermissions` (verbatim, §6.9)

Source uses the literal-string sentinel `"external" === 'ant'` (NOT `process.env.USER_TYPE === 'ant'`). This is a **build-time dead-code-elimination marker**, not a runtime check: Bun's bundler rewrites `"external"` to `"ant"` for ANT builds and to `"external"` for external builds, so `"external" === 'ant'` evaluates to `true` for ANT bundles and to `false` for external bundles **at build time**. The constant-folded `if (false && ...)` branch is then eliminated entirely from the external bundle.

Implication (addresses adversarial H1): for **external builds the `passthrough`/auto-mode-classifier branch does not exist in the emitted code at all** — it is not "skipped at runtime", it is gone. Auto mode is an ANT-only concept at this surface; external bundles only ever return `{behavior: 'allow', updatedInput: input}`. Reimplementers targeting external builds MUST drop the `auto`-mode branch, not gate it on a runtime env var. Reimplementers targeting an ANT-equivalent build MUST place the gate behind a constant-folded sentinel (or equivalent build-time strip) so the auto-mode classifier path is *omitted*, not *reachable-but-skipped*. (`AgentTool.tsx:1284-1287`; full body §6.9.)

#### `mapToolResultToToolResultBlockParam`

Discriminated by `internalData.status`:
- `'teammate_spawned'`: text trailer with `agent_id`, `name`, `team_name` (`AgentTool.tsx:1301-1314`).
- `'remote_launched'`: text trailer with `taskId`, `session_url`, `output_file` and "Briefly tell the user what you launched and end your response."
- `'async_launched'`: prefix + per-`canReadOutputFile` instructions about whether the parent should `Read`/`Bash tail` the `output_file` or just announce-and-stop.
- `'completed'`: content + optional worktree info + optional `agentId`/`<usage>` trailer. **`ONE_SHOT_BUILTIN_AGENT_TYPES` (Explore, Plan) skip the trailer entirely** — saves ~135 chars × 34M Explore runs/week (`AgentTool.tsx:1351-1362`). If subagent returned no content, an explicit `(Subagent completed but returned no output.)` text marker is inserted instead of an empty content array.
- Exhaustiveness: `data satisfies never` + throw on unexpected status.

#### `toAutoClassifierInput`

`(input) => '(${tags.join(', ')}): ${prompt}'` where `tags = [subagent_type, mode? `mode=${mode}` : undefined].filter(notNull)` (`AgentTool.tsx:1267-1272`).

### 3.2 TeamCreateTool

- `name = 'TeamCreate'`. `searchHint = 'create a multi-agent swarm team'`. `maxResultSizeChars = 100_000`.
- `shouldDefer = true` (`TeamCreateTool.ts:78`) — the model must call `ToolSearchTool` first.
- `userFacingName: () => ''` (returns empty string — ToolSearch UI override).
- `isEnabled = isAgentSwarmsEnabled` (`TeamCreateTool.ts:88-90`).
- `toAutoClassifierInput(input) => input.team_name`.
- `validateInput`: rejects empty `team_name` with errorCode 9.
- `inputSchema` (`z.strictObject`, verbatim §6.5): `team_name: string`, `description?: string`, `agent_type?: string`.
- `description: () => 'Create a new team for coordinating multiple agents'`.
- `prompt: () => getPrompt()` — see §6.5 for full body.
- `mapToolResultToToolResultBlockParam`: wraps `data` as JSON-stringified text block.
- `call()`: rejects if already leading a team; auto-renames via `generateUniqueTeamName` if collision; assigns deterministic `leadAgentId = formatAgentId(TEAM_LEAD_NAME, finalTeamName)`; resolves lead model from session/AppState/default chain; writes team file; registers session-cleanup; resets and ensures task list; calls `setLeaderTeamName(sanitizeName(finalTeamName))`; updates `AppState.teamContext` with single team-lead member; logs `tengu_team_created`. **Does not** set `CLAUDE_CODE_AGENT_ID` for the lead — comment at `TeamCreateTool.ts:224-228` explains this is to keep `isTeammate()` returning false for the lead.

### 3.3 TeamDeleteTool

- `name = 'TeamDelete'`. `searchHint = 'disband a swarm team and clean up'`. `maxResultSizeChars = 100_000`.
- `shouldDefer = true` (`TeamDeleteTool.ts:36`).
- `userFacingName: () => ''`.
- `isEnabled = isAgentSwarmsEnabled`.
- `inputSchema = z.strictObject({})` — no parameters; team is taken from `appState.teamContext`.
- `description: () => 'Clean up team and task directories when the swarm is complete'`.
- `prompt: () => getPrompt()` — see §6.6.
- `call()`: refuses if active members remain (filters out lead and `isActive: false`); calls `cleanupTeamDirectories`, `unregisterTeamForSessionCleanup`, `clearTeammateColors`, `clearLeaderTeamName`; logs `tengu_team_deleted`; clears `teamContext` and `inbox.messages` from AppState. Returns success even when no team is set (idempotent no-op).

### 3.4 SendMessageTool

- `name = 'SendMessage'`. `searchHint = 'send messages to agent teammates (swarm protocol)'`. `maxResultSizeChars = 100_000`.
- `shouldDefer = true` (`SendMessageTool.ts:533`).
- `userFacingName: () => 'SendMessage'`.
- `isEnabled = isAgentSwarmsEnabled`.
- `isReadOnly(input) => typeof input.message === 'string'` — structured messages (shutdown/plan-approval) count as side-effecting writes.
- `backfillObservableInput`: rewrites `input.type/recipient/content/request_id/approve` for analytics observability when the wire input is in the natural Zod-union form.
- `toAutoClassifierInput`: short summary string per message variant.
- `validateInput` and `checkPermissions`: see §3.4.1 below; full input schema verbatim §6.3.

#### 3.4.1 `checkPermissions` and `validateInput`

`checkPermissions` (verbatim §6.8): bridge-target sends require user `ask` with a `safetyCheck` decision-reason and `classifierApprovable: false` — bypass-permissions and auto-mode classifier cannot bypass it. Otherwise `allow`.

`validateInput` rules (cited from `SendMessageTool.ts:604-718`):
- `to.trim().length === 0` → reject.
- `(addr.scheme === 'bridge' || 'uds') && addr.target.trim() === ''` → reject.
- `to.includes('@')` → reject "to must be a bare teammate name or '*'".
- Bridge target with structured (non-string) message → reject.
- Bridge target without active `replBridgeHandle` or `isReplBridgeActive` → reject.
- UDS target with string message → accept (summary not required because UDS UI doesn't render it).
- String message without `summary` → reject (errorCode 9).
- Broadcast (`to === '*'`) with structured message → reject.
- `feature('UDS_INBOX')` cross-session structured → reject "structured messages cannot be sent cross-session".
- `shutdown_response` to non-`TEAM_LEAD_NAME` → reject.
- `shutdown_response` rejecting (`approve === false`) without `reason` → reject.

#### 3.4.2 `call()` routing

Order (`SendMessageTool.ts:741-913`):
1. **`UDS_INBOX` bridge** (`addr.scheme === 'bridge'`): re-check handle + active state (handle could have dropped during permission wait), require `postInterClaudeMessage`, return success/error message.
2. **`UDS_INBOX` uds**: `sendToUdsSocket(addr.target, input.message)`.
3. **In-process subagent / agent-id**: look up `appState.agentNameRegistry.get(input.to)` → fall back to `toAgentId(input.to)`. If task is `running`: `queuePendingMessage`. If task exists but is stopped: `resumeAgentBackground`. If task evicted: try `resumeAgentBackground` from disk transcript.
4. **String message**: broadcast (`to === '*'`) → `handleBroadcast`; else `handleMessage`.
5. **Structured message**: discriminated by `message.type`:
   - `'shutdown_request'` → `handleShutdownRequest`
   - `'shutdown_response'` with `approve` → `handleShutdownApproval`; without → `handleShutdownRejection`
   - `'plan_approval_response'` with `approve` → `handlePlanApproval`; without → `handlePlanRejection`

### 3.5 Lazy-getter circular-dep wiring (citing 08)

```ts
// src/tools.ts:62-72
/* eslint-disable @typescript-eslint/no-require-imports */
const getTeamCreateTool = () =>
  require('./tools/TeamCreateTool/TeamCreateTool.js')
    .TeamCreateTool as typeof import('./tools/TeamCreateTool/TeamCreateTool.js').TeamCreateTool
const getTeamDeleteTool = () =>
  require('./tools/TeamDeleteTool/TeamDeleteTool.js')
    .TeamDeleteTool as typeof import('./tools/TeamDeleteTool/TeamDeleteTool.js').TeamDeleteTool
const getSendMessageTool = () =>
  require('./tools/SendMessageTool/SendMessageTool.js')
    .SendMessageTool as typeof import('./tools/SendMessageTool/SendMessageTool.js').SendMessageTool
/* eslint-enable @typescript-eslint/no-require-imports */
```

Per spec 08 §5.6: each of these tools transitively imports `tools.ts` (via `coordinator/`), so importing them statically would cycle at module init. The `as typeof import(...)` cast preserves type information without evaluating the module. ESLint disables the `no-require-imports` rule for this block.

Insertion in the registry:
- `getSendMessageTool()` is unconditionally appended at `tools.ts:226`.
- `getTeamCreateTool()`, `getTeamDeleteTool()` are gated by `isAgentSwarmsEnabled()` at `tools.ts:228-230`.
- `AgentTool` is statically imported (no circular issue) and is included in every `getAllBaseTools()` call. The simple-mode coordinator branch additionally appends `AgentTool, TaskStopTool, getSendMessageTool()` at `tools.ts:295`; the simple-REPL coordinator branch appends `TaskStopTool, getSendMessageTool()` at `tools.ts:283`.

---

## 4. Data Model & State

### 4.1 `AgentDefinition` discriminated union

Three variants, all extending `BaseAgentDefinition` (verbatim §6.4):
- `BuiltInAgentDefinition`: `source: 'built-in'`, `baseDir: 'built-in'`, optional `callback`, mandatory `getSystemPrompt({ toolUseContext: Pick<ToolUseContext,'options'> }) => string`.
- `CustomAgentDefinition`: `source: SettingSource` (i.e. `userSettings|projectSettings|policySettings|localSettings`), optional `filename`, optional `baseDir`, mandatory `getSystemPrompt() => string` (closure over loaded markdown).
- `PluginAgentDefinition`: `source: 'plugin'`, mandatory `plugin: string`.

Type guards: `isBuiltInAgent`, `isCustomAgent`, `isPluginAgent` (`loadAgentsDir.ts:168-184`).

`AgentDefinitionsResult = { activeAgents, allAgents, failedFiles?, allowedAgentTypes? }` — `allowedAgentTypes` is set by the `Agent(x,y)` permission-rule restriction parser in spec 09.

### 4.2 In-memory state owned by this surface

- `appState.teamContext: { teamName, teamFilePath, leadAgentId, teammates: Record<agentId, TeammateInfo> } | undefined`. Set by `TeamCreate`, cleared by `TeamDelete`.
- `appState.inbox: { messages: ... }` — cleared by `TeamDelete`.
- `appState.agentNameRegistry: Map<name, agentId>` — read by `SendMessage` for in-process subagent routing.
- `appState.tasks[agentId]` — read by `SendMessage` to decide queue-vs-resume.
- `appState.toolPermissionContext.mode === 'auto'` — branches the AgentTool `checkPermissions` to `passthrough` (ANT only).

### 4.3 On-disk artifacts (locations cited; ownership is spec 30/41)

- Team file: `~/.claude/teams/{team-name}/config.json` (referenced from `TeamCreateTool/prompt.ts:34-35`, prompt body).
- Task list dir: `~/.claude/tasks/{team-name}/`.
- Agent memory snapshot: `<cwd>/.claude/agent-memory-snapshots/<agentType>/snapshot.json` (`agentMemorySnapshot.ts:31-37`).
- Agent local memory sync marker: `.snapshot-synced.json` (in agent memory dir).

---

## 5. Algorithm / Control Flow

### 5.1 AgentTool spawn-routing (tool-surface decisions only)

Pseudocode of the surface decisions before delegating to `runAgent`/`spawnTeammate`/`teleportToRemote` (which spec 30 owns):

```
function call(input, toolUseContext):
  appState = toolUseContext.getAppState()
  permissionMode = appState.toolPermissionContext.mode
  rootSetAppState = toolUseContext.setAppStateForTasks ?? toolUseContext.setAppState

  model = isCoordinatorMode() ? undefined : input.model

  if input.team_name && !isAgentSwarmsEnabled():
    throw "Agent Teams is not yet available on your plan."

  teamName = isAgentSwarmsEnabled() ? (input.team_name || appState.teamContext?.teamName) : undefined

  if isTeammate() && teamName && input.name:
    throw "Teammates cannot spawn other teammates ..."
  if isInProcessTeammate() && teamName && input.run_in_background === true:
    throw "In-process teammates cannot spawn background agents ..."

  if teamName && input.name:                  // multi-agent spawn (spec 30)
    setAgentColor(input.subagent_type, ...)
    return spawnTeammate({...}) -> {status:'teammate_spawned'}

  // Fork routing
  effectiveType = input.subagent_type
                  ?? (isForkSubagentEnabled() ? undefined : GENERAL_PURPOSE_AGENT.agentType)
  isForkPath = effectiveType === undefined

  if isForkPath:
    if toolUseContext.options.querySource === `agent:builtin:${FORK_AGENT.agentType}`
       || isInForkChild(toolUseContext.messages):
      throw "Fork is not available inside a forked worker. ..."
    selectedAgent = FORK_AGENT
  else:
    allAgents = toolUseContext.options.agentDefinitions.activeAgents
    allowedAgentTypes = toolUseContext.options.agentDefinitions.allowedAgentTypes
    candidates = allowedAgentTypes
      ? allAgents.filter(a => allowedAgentTypes.includes(a.agentType))
      : allAgents
    candidates = filterDeniedAgents(candidates, appState.toolPermissionContext, AGENT_TOOL_NAME)
    selectedAgent = candidates.find(a => a.agentType === effectiveType)
    if !selectedAgent:
      if allAgents.find(a => a.agentType === effectiveType):
        throw `Agent type '${effectiveType}' has been denied by permission rule '${AGENT_TOOL_NAME}(${effectiveType})' from <source>.`
      else
        throw `Agent type '${effectiveType}' not found. Available agents: <list>`

  if isInProcessTeammate() && teamName && selectedAgent.background === true:
    throw "In-process teammates cannot spawn background agents. ..."

  // MCP requirement gate
  if selectedAgent.requiredMcpServers?.length:
    poll appState.mcp.clients up to 30s for matching pending servers (early-exit on failed)
    serversWithTools = derive from appState.mcp.tools (mcp__<server>__<tool>)
    if !hasRequiredMcpServers(selectedAgent, serversWithTools):
      throw `Agent '${type}' requires MCP servers matching: ${missing}. ...`

  setAgentColor(selectedAgent.agentType, selectedAgent.color)
  resolvedAgentModel = getAgentModel(selectedAgent.model, mainLoopModel,
                                     isForkPath ? undefined : model, permissionMode)
  logEvent('tengu_agent_tool_selected', {...})

  effectiveIsolation = input.isolation ?? selectedAgent.isolation

  if "external" === 'ant' && effectiveIsolation === 'remote':         // ANT-only branch
    eligibility = checkRemoteAgentEligibility()
    if !eligibility.eligible: throw formatPreconditionError(...)
    session = teleportToRemote({initialMessage: prompt, description, signal})
    {taskId, sessionId} = registerRemoteAgentTask({...})
    logEvent('tengu_agent_tool_remote_launched', ...)
    return {status: 'remote_launched', taskId, sessionUrl, description, prompt, outputFile}

  // System-prompt & messages branch (fork vs normal) — body in spec 30.

  shouldRunAsync =
    (input.run_in_background === true || selectedAgent.background === true ||
     isCoordinator || forceAsync || assistantForceAsync ||
     (proactiveModule?.isProactiveActive() ?? false))
    && !isBackgroundTasksDisabled
  // forceAsync = isForkSubagentEnabled()
  // assistantForceAsync = feature('KAIROS') ? appState.kairosEnabled : false

  workerPermissionContext = {...appState.toolPermissionContext,
                             mode: selectedAgent.permissionMode ?? 'acceptEdits'}
  workerTools = assembleToolPool(workerPermissionContext, appState.mcp.tools)
  earlyAgentId = createAgentId()

  if effectiveIsolation === 'worktree':
    worktreeInfo = await createAgentWorktree(`agent-${earlyAgentId.slice(0,8)}`)

  if isForkPath && worktreeInfo:
    promptMessages.push(createUserMessage({content: buildWorktreeNotice(getCwd(), worktreePath)}))

  return shouldRunAsync ? <register async, return async_launched>
                        : <run sync via runAgent, return completed>   // spec 30
```

### 5.2 Default model selection for built-in agents

- `GENERAL_PURPOSE_AGENT.model` is **omitted** — falls through to `getDefaultSubagentModel()` per the comment at `generalPurposeAgent.ts:32`.
- `EXPLORE_AGENT.model = process.env.USER_TYPE === 'ant' ? 'inherit' : 'haiku'` (`exploreAgent.ts:78`). Comment: "Ants get inherit to use the main agent's model; external users get haiku for speed." For ANT, `getAgentModel()` further consults the `tengu_explore_agent` GrowthBook flag at runtime.
- `PLAN_AGENT.model = 'inherit'` unconditionally (`planAgent.ts:87`).

### 5.3 `getBuiltInAgents()` assembly

```
function getBuiltInAgents() -> AgentDefinition[]:
  if isEnvTruthy(CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS) && getIsNonInteractiveSession():
    return []                                           // SDK blank-slate

  if feature('COORDINATOR_MODE') && isEnvTruthy(CLAUDE_CODE_COORDINATOR_MODE):
    return require('../../coordinator/workerAgent.js').getCoordinatorAgents()  // lazy

  agents = [GENERAL_PURPOSE_AGENT, STATUSLINE_SETUP_AGENT]

  if areExplorePlanAgentsEnabled():                     // feature('BUILTIN_EXPLORE_PLAN_AGENTS')
                                                        // && tengu_amber_stoat (default true)
    agents.push(EXPLORE_AGENT, PLAN_AGENT)

  if CLAUDE_CODE_ENTRYPOINT not in {sdk-ts, sdk-py, sdk-cli}:
    agents.push(CLAUDE_CODE_GUIDE_AGENT)

  if feature('VERIFICATION_AGENT') && tengu_hive_evidence (default false):
    agents.push(VERIFICATION_AGENT)

  return agents
```

### 5.4 `shouldInjectAgentListInMessages()`

Build-time-overridable gate (`prompt.ts:59-64`):
```
if isEnvTruthy(CLAUDE_CODE_AGENT_LIST_IN_MESSAGES) -> true
if isEnvDefinedFalsy(CLAUDE_CODE_AGENT_LIST_IN_MESSAGES) -> false
return getFeatureValue_CACHED_MAY_BE_STALE('tengu_agent_list_attach', false)
```

When true, the inline agent-list section in `getPrompt()` is replaced by a single line pointing to `<system-reminder>` messages (`agent_listing_delta` attachment, owned by spec 04). Rationale: the dynamic agent list was ~10.2% of fleet `cache_creation` tokens because MCP async-connect, `/reload-plugins`, and permission-mode changes all mutate the list and bust the tools-block prompt cache.

### 5.5 SendMessageTool routing decision tree

```
function call(input):
  if feature('UDS_INBOX') and input.message is string:
    addr = parseAddress(input.to)
    if addr.scheme === 'bridge':
      if !getReplBridgeHandle() or !isReplBridgeActive(): return {success:false, "Remote Control disconnected ..."}
      result = postInterClaudeMessage(addr.target, input.message)
      return {success: result.ok, message: ...}
    if addr.scheme === 'uds':
      try { sendToUdsSocket(addr.target, input.message); return success }
      catch e: return {success:false, message: ...}

  if input.message is string and input.to !== '*':
    appState = context.getAppState()
    registered = appState.agentNameRegistry.get(input.to)
    agentId = registered ?? toAgentId(input.to)
    if agentId:
      task = appState.tasks[agentId]
      if isLocalAgentTask(task) and !isMainSessionTask(task):
        if task.status === 'running':
          queuePendingMessage(agentId, input.message, ...)
          return {success:true, message:"Message queued ..."}
        else:
          try resumeAgentBackground({...})
          return {success:true, message:"Agent ... was stopped ...; resumed ..."}
      else:
        try resumeAgentBackground({...}) from disk transcript

  if input.message is string:
    if input.to === '*': return handleBroadcast(...)
    return handleMessage(...)

  // structured message
  if input.to === '*': throw "structured messages cannot be broadcast"
  switch input.message.type:
    case 'shutdown_request': return handleShutdownRequest(...)
    case 'shutdown_response':
      return input.message.approve ? handleShutdownApproval(request_id) : handleShutdownRejection(request_id, reason)
    case 'plan_approval_response':
      return input.message.approve ? handlePlanApproval(...) : handlePlanRejection(...)
```

### 5.6 TeamCreate `call()`

```
function call(input):
  appState = context.getAppState()
  if appState.teamContext?.teamName:
    throw `Already leading team "${existing}". A leader can only manage one team at a time. ...`
  finalTeamName = readTeamFile(input.team_name) ? generateWordSlug() : input.team_name
  leadAgentId = formatAgentId(TEAM_LEAD_NAME, finalTeamName)
  leadAgentType = input.agent_type || TEAM_LEAD_NAME
  leadModel = parseUserSpecifiedModel(
    appState.mainLoopModelForSession ?? appState.mainLoopModel ?? getDefaultMainLoopModel()
  )
  teamFilePath = getTeamFilePath(finalTeamName)
  teamFile = {name, description, createdAt, leadAgentId, leadSessionId: getSessionId(),
              members:[{agentId: leadAgentId, name: TEAM_LEAD_NAME, agentType, model: leadModel,
                        joinedAt, tmuxPaneId:'', cwd: getCwd(), subscriptions:[]}]}
  await writeTeamFileAsync(finalTeamName, teamFile)
  registerTeamForSessionCleanup(finalTeamName)        // gh-32730 fix
  taskListId = sanitizeName(finalTeamName)
  await resetTaskList(taskListId); await ensureTasksDir(taskListId)
  setLeaderTeamName(taskListId)                       // ensures getTaskListId() returns it
  setAppState(prev => {...prev, teamContext: {teamName, teamFilePath, leadAgentId, teammates:{[leadAgentId]:{...}}}})
  logEvent('tengu_team_created', ...)
  return {data:{team_name, team_file_path, lead_agent_id}}
```

### 5.7 TeamDelete `call()`

```
function call(_input):
  appState = context.getAppState()
  teamName = appState.teamContext?.teamName
  if teamName:
    teamFile = readTeamFile(teamName)
    if teamFile:
      nonLead = teamFile.members.filter(m => m.name !== TEAM_LEAD_NAME)
      active = nonLead.filter(m => m.isActive !== false)
      if active.length > 0:
        return {success:false, message:`Cannot cleanup team with ${n} active member(s): ${names}. Use requestShutdown ...`, team_name}
    await cleanupTeamDirectories(teamName)
    unregisterTeamForSessionCleanup(teamName)
    clearTeammateColors()
    clearLeaderTeamName()
    logEvent('tengu_team_deleted', {team_name})
  setAppState(prev => {...prev, teamContext: undefined, inbox: {messages: []}})
  return {success:true, message: teamName ? `Cleaned up directories and worktrees for team "${name}"` : 'No team name found, nothing to clean up', team_name}
```

---

## 6. Verbatim Assets

### 6.1 AgentTool prompt body (verbatim from `src/tools/AgentTool/prompt.ts`)

The full file is reproduced lines 1-287; the key segments are:

#### Shared core (`prompt.ts:202-212`)
```
Launch a new agent to handle complex, multi-step tasks autonomously.

The {AGENT_TOOL_NAME} tool launches specialized agents (subprocesses) that autonomously handle complex tasks. Each agent type has specific capabilities and tools available to it.

{agentListSection}

When using the {AGENT_TOOL_NAME} tool, specify a subagent_type to use a specialized agent, or omit it to fork yourself — a fork inherits your full conversation context.
```
(The last line above is the `forkEnabled` variant. Non-fork variant: `When using the ${AGENT_TOOL_NAME} tool, specify a subagent_type parameter to select which agent type to use. If omitted, the general-purpose agent is used.`)

#### Coordinator-mode return (`prompt.ts:216-218`)
Coordinator mode returns only the shared core string above — no `Usage notes`, no `When NOT to use`, no examples. Comment: "the coordinator system prompt already covers usage notes, examples, and when-not-to-use guidance."

#### `agentListSection` (`prompt.ts:194-199`)
```
// listViaAttachment === true:
Available agent types are listed in <system-reminder> messages in the conversation.

// listViaAttachment === false:
Available agent types and the tools they have access to:
{effectiveAgents.map(formatAgentLine).join('\n')}
```

`formatAgentLine(agent) = '- ${agentType}: ${whenToUse} (Tools: ${toolsDescription})'` where `toolsDescription` is computed by `getToolsDescription` (`prompt.ts:15-37`):
- `tools && disallowedTools` → `effective = tools.filter(t => !denySet.has(t))` → `effective.length === 0 ? 'None' : effective.join(', ')`
- `tools` only → `tools.join(', ')`
- `disallowedTools` only → `'All tools except ${disallowedTools.join(', ')}'`
- neither → `'All tools'`

#### ANT-only `'remote'` isolation note (`prompt.ts:273`)
```
- You can set `isolation: "remote"` to run the agent in a remote CCR environment. This is always a background task; you'll be notified when it completes. Use for long-running tasks that need a fresh sandbox.
```
Inserted only when `process.env.USER_TYPE === 'ant'`. Adjacent (always-emitted) note:
```
- You can optionally set `isolation: "worktree"` to run the agent in a temporary git worktree, giving it an isolated copy of the repository. The worktree is automatically cleaned up if the agent makes no changes; if changes are made, the worktree path and branch are returned in the result.
```

#### `whenToForkSection` (only when `isForkSubagentEnabled()`)
Reproduced at `prompt.ts:81-96`. Defines fork criterion ("will I need this output again"), research vs implementation guidance, no-peek and no-race rules, and the directive-style prompt requirement.

#### `writingThePromptSection` (`prompt.ts:99-113`)
Always included when not coordinator. Branches on `forkEnabled` for one phrase ("When spawning a fresh agent..." vs nothing) and the closing ("For fresh agents, terse" vs "Terse"). Includes the **"Never delegate understanding"** paragraph requiring file paths and line numbers in delegated prompts.

#### `currentExamples` and `forkExamples`
`currentExamples` (`prompt.ts:156-188`) — the test-runner / greeting-responder examples.
`forkExamples` (`prompt.ts:115-154`) — three examples: ship-readiness audit fork, mid-wait status response, code-reviewer second-opinion subagent. Used when `isForkSubagentEnabled()` is true.

#### `whenNotToUseSection` (non-fork, non-coordinator only)
Branches on `hasEmbeddedSearchTools()`:
- `embedded`: `\`find\` via the Bash tool` and `\`grep\` via the Bash tool`.
- non-embedded: `${GLOB_TOOL_NAME}` for both file and content search hints.

#### `concurrencyNote`
Inserted only when `!listViaAttachment && getSubscriptionType() !== 'pro'`:
```
- Launch multiple agents concurrently whenever possible, to maximize performance; to do that, use a single message with multiple tool uses
```

#### Background-task usage notes (`prompt.ts:262-265`)
Inserted only when `!isEnvTruthy(CLAUDE_CODE_DISABLE_BACKGROUND_TASKS) && !isInProcessTeammate() && !forkEnabled`:
```
- You can optionally run agents in the background using the run_in_background parameter. When an agent runs in the background, you will be automatically notified when it completes — do NOT sleep, poll, or proactively check on its progress. Continue with other work or respond to the user instead.
- **Foreground vs background**: Use foreground (default) when you need the agent's results before you can proceed — e.g., research agents whose findings inform your next steps. Use background when you have genuinely independent work to do in parallel.
```

#### Continuation note (`prompt.ts:267`, always emitted)
```
- To continue a previously spawned agent, use {SEND_MESSAGE_TOOL_NAME} with the agent's ID or name as the `to` field. The agent resumes with its full context preserved. {fork ? 'Each fresh Agent invocation with a subagent_type starts without context — provide a complete task description.' : 'Each Agent invocation starts fresh — provide a complete task description.'}
```

#### Teammate restrictions (`prompt.ts:277-283`)
- `isInProcessTeammate()`: `- The run_in_background, name, team_name, and mode parameters are not available in this context. Only synchronous subagents are supported.`
- `isTeammate()` (else branch): `- The name, team_name, and mode parameters are not available in this context — teammates cannot spawn other teammates. Omit them to spawn a subagent.`

### 6.2 AgentTool input schema (verbatim from `AgentTool.tsx:82-125`)

```ts
const baseInputSchema = lazySchema(() => z.object({
  description: z.string().describe('A short (3-5 word) description of the task'),
  prompt: z.string().describe('The task for the agent to perform'),
  subagent_type: z.string().optional().describe('The type of specialized agent to use for this task'),
  model: z.enum(['sonnet', 'opus', 'haiku']).optional().describe(
    "Optional model override for this agent. Takes precedence over the agent definition's model frontmatter. If omitted, uses the agent definition's model, or inherits from the parent."
  ),
  run_in_background: z.boolean().optional().describe(
    'Set to true to run this agent in the background. You will be notified when it completes.'
  )
}));

const fullInputSchema = lazySchema(() => {
  const multiAgentInputSchema = z.object({
    name: z.string().optional().describe(
      'Name for the spawned agent. Makes it addressable via SendMessage({to: name}) while running.'
    ),
    team_name: z.string().optional().describe(
      'Team name for spawning. Uses current team context if omitted.'
    ),
    mode: permissionModeSchema().optional().describe(
      'Permission mode for spawned teammate (e.g., "plan" to require plan approval).'
    )
  });
  return baseInputSchema().merge(multiAgentInputSchema).extend({
    isolation: ("external" === 'ant' ? z.enum(['worktree', 'remote']) : z.enum(['worktree']))
      .optional()
      .describe(
        "external" === 'ant'
          ? 'Isolation mode. "worktree" creates a temporary git worktree so the agent works on an isolated copy of the repo. "remote" launches the agent in a remote CCR environment (always runs in background).'
          : 'Isolation mode. "worktree" creates a temporary git worktree so the agent works on an isolated copy of the repo.'
      ),
    cwd: z.string().optional().describe(
      'Absolute path to run the agent in. Overrides the working directory for all filesystem and shell operations within this agent. Mutually exclusive with isolation: "worktree".'
    )
  });
});

export const inputSchema = lazySchema(() => {
  const schema = feature('KAIROS') ? fullInputSchema() : fullInputSchema().omit({ cwd: true });
  return isBackgroundTasksDisabled || isForkSubagentEnabled()
    ? schema.omit({ run_in_background: true })
    : schema;
});
```

### 6.3 SendMessageTool input schema (verbatim from `SendMessageTool.ts:46-87`)

```ts
const StructuredMessage = lazySchema(() =>
  z.discriminatedUnion('type', [
    z.object({
      type: z.literal('shutdown_request'),
      reason: z.string().optional(),
    }),
    z.object({
      type: z.literal('shutdown_response'),
      request_id: z.string(),
      approve: semanticBoolean(),
      reason: z.string().optional(),
    }),
    z.object({
      type: z.literal('plan_approval_response'),
      request_id: z.string(),
      approve: semanticBoolean(),
      feedback: z.string().optional(),
    }),
  ]),
)

const inputSchema = lazySchema(() =>
  z.object({
    to: z.string().describe(
      feature('UDS_INBOX')
        ? 'Recipient: teammate name, "*" for broadcast, "uds:<socket-path>" for a local peer, or "bridge:<session-id>" for a Remote Control peer (use ListPeers to discover)'
        : 'Recipient: teammate name, or "*" for broadcast to all teammates',
    ),
    summary: z.string().optional().describe(
      'A 5-10 word summary shown as a preview in the UI (required when message is a string)',
    ),
    message: z.union([
      z.string().describe('Plain text message content'),
      StructuredMessage(),
    ]),
  }),
)
```

### 6.4 `AgentDefinition` schema and types (verbatim from `loadAgentsDir.ts:73-191`)

```ts
const AgentJsonSchema = lazySchema(() =>
  z.object({
    description: z.string().min(1, 'Description cannot be empty'),
    tools: z.array(z.string()).optional(),
    disallowedTools: z.array(z.string()).optional(),
    prompt: z.string().min(1, 'Prompt cannot be empty'),
    model: z.string().trim().min(1, 'Model cannot be empty')
      .transform(m => (m.toLowerCase() === 'inherit' ? 'inherit' : m))
      .optional(),
    effort: z.union([z.enum(EFFORT_LEVELS), z.number().int()]).optional(),
    permissionMode: z.enum(PERMISSION_MODES).optional(),
    mcpServers: z.array(AgentMcpServerSpecSchema()).optional(),
    hooks: HooksSchema().optional(),
    maxTurns: z.number().int().positive().optional(),
    skills: z.array(z.string()).optional(),
    initialPrompt: z.string().optional(),
    memory: z.enum(['user', 'project', 'local']).optional(),
    background: z.boolean().optional(),
    isolation: (process.env.USER_TYPE === 'ant'
      ? z.enum(['worktree', 'remote'])
      : z.enum(['worktree'])).optional(),
  }),
)
const AgentsJsonSchema = lazySchema(() => z.record(z.string(), AgentJsonSchema()))

export type BaseAgentDefinition = {
  agentType: string
  whenToUse: string
  tools?: string[]
  disallowedTools?: string[]
  skills?: string[]
  mcpServers?: AgentMcpServerSpec[]
  hooks?: HooksSettings
  color?: AgentColorName
  model?: string
  effort?: EffortValue
  permissionMode?: PermissionMode
  maxTurns?: number
  filename?: string
  baseDir?: string
  criticalSystemReminder_EXPERIMENTAL?: string
  requiredMcpServers?: string[]
  background?: boolean
  initialPrompt?: string
  memory?: AgentMemoryScope
  isolation?: 'worktree' | 'remote'
  pendingSnapshotUpdate?: { snapshotTimestamp: string }
  /** Omit CLAUDE.md hierarchy from the agent's userContext. Read-only agents
   * (Explore, Plan) don't need commit/PR/lint guidelines — the main agent has
   * full CLAUDE.md and interprets their output. Saves ~5-15 Gtok/week across
   * 34M+ Explore spawns. Kill-switch: tengu_slim_subagent_claudemd. */
  omitClaudeMd?: boolean
}

export type BuiltInAgentDefinition = BaseAgentDefinition & {
  source: 'built-in'; baseDir: 'built-in'; callback?: () => void;
  getSystemPrompt: (params: { toolUseContext: Pick<ToolUseContext,'options'> }) => string
}
export type CustomAgentDefinition = BaseAgentDefinition & {
  getSystemPrompt: () => string; source: SettingSource; filename?: string; baseDir?: string
}
export type PluginAgentDefinition = BaseAgentDefinition & {
  getSystemPrompt: () => string; source: 'plugin'; filename?: string; plugin: string
}
export type AgentDefinition = BuiltInAgentDefinition | CustomAgentDefinition | PluginAgentDefinition

export type AgentMcpServerSpec = string | { [name: string]: McpServerConfig }

export type AgentDefinitionsResult = {
  activeAgents: AgentDefinition[]
  allAgents: AgentDefinition[]
  failedFiles?: Array<{ path: string; error: string }>
  allowedAgentTypes?: string[]
}
```

ANT-only frontmatter parser branch (`loadAgentsDir.ts:607-621`):
```ts
type IsolationMode = 'worktree' | 'remote'
const VALID_ISOLATION_MODES: readonly IsolationMode[] =
  process.env.USER_TYPE === 'ant' ? ['worktree', 'remote'] : ['worktree']
const isolationRaw = frontmatter['isolation'] as string | undefined
let isolation: IsolationMode | undefined
if (isolationRaw !== undefined) {
  if (VALID_ISOLATION_MODES.includes(isolationRaw as IsolationMode)) {
    isolation = isolationRaw as IsolationMode
  } else {
    logForDebugging(
      `Agent file ${filePath} has invalid isolation value '${isolationRaw}'. Valid options: ${VALID_ISOLATION_MODES.join(', ')}`,
    )
  }
}
```

### 6.5 Built-in agent definitions (verbatim)

#### `EXPLORE_AGENT` (`built-in/exploreAgent.ts:64-83`)
```ts
export const EXPLORE_AGENT: BuiltInAgentDefinition = {
  agentType: 'Explore',
  whenToUse: EXPLORE_WHEN_TO_USE,
  disallowedTools: [
    AGENT_TOOL_NAME,
    EXIT_PLAN_MODE_TOOL_NAME,
    FILE_EDIT_TOOL_NAME,
    FILE_WRITE_TOOL_NAME,
    NOTEBOOK_EDIT_TOOL_NAME,
  ],
  source: 'built-in',
  baseDir: 'built-in',
  // Ants get inherit to use the main agent's model; external users get haiku for speed
  // Note: For ants, getAgentModel() checks tengu_explore_agent GrowthBook flag at runtime
  model: process.env.USER_TYPE === 'ant' ? 'inherit' : 'haiku',
  // Explore is a fast read-only search agent — it doesn't need commit/PR/lint
  // rules from CLAUDE.md. The main agent has full context and interprets results.
  omitClaudeMd: true,
  getSystemPrompt: () => getExploreSystemPrompt(),
}
export const EXPLORE_AGENT_MIN_QUERIES = 3
```
`EXPLORE_WHEN_TO_USE` (line 61-62):
```
Fast agent specialized for exploring codebases. Use this when you need to quickly find files by patterns (eg. "src/components/**/*.tsx"), search code for keywords (eg. "API endpoints"), or answer questions about the codebase (eg. "how do API endpoints work?"). When calling this agent, specify the desired thoroughness level: "quick" for basic searches, "medium" for moderate exploration, or "very thorough" for comprehensive analysis across multiple locations and naming conventions.
```
`getExploreSystemPrompt()` body (lines 13-56) — full read-only-mode prohibition prompt with "CRITICAL: READ-ONLY MODE" header, Bash whitelist (`ls, git status, git log, git diff, find${embedded ? ', grep' : ''}, cat, head, tail`), Bash blacklist (`mkdir, touch, rm, cp, mv, git add, git commit, npm install, pip install`), and parallel-tool-call optimization note.

#### `PLAN_AGENT` (`built-in/planAgent.ts:73-92`)
```ts
export const PLAN_AGENT: BuiltInAgentDefinition = {
  agentType: 'Plan',
  whenToUse:
    'Software architect agent for designing implementation plans. Use this when you need to plan the implementation strategy for a task. Returns step-by-step plans, identifies critical files, and considers architectural trade-offs.',
  disallowedTools: [
    AGENT_TOOL_NAME, EXIT_PLAN_MODE_TOOL_NAME, FILE_EDIT_TOOL_NAME,
    FILE_WRITE_TOOL_NAME, NOTEBOOK_EDIT_TOOL_NAME,
  ],
  source: 'built-in',
  tools: EXPLORE_AGENT.tools,
  baseDir: 'built-in',
  model: 'inherit',
  omitClaudeMd: true,
  getSystemPrompt: () => getPlanV2SystemPrompt(),
}
```

#### `GENERAL_PURPOSE_AGENT` (`built-in/generalPurposeAgent.ts:25-34`)
```ts
export const GENERAL_PURPOSE_AGENT: BuiltInAgentDefinition = {
  agentType: 'general-purpose',
  whenToUse:
    'General-purpose agent for researching complex questions, searching for code, and executing multi-step tasks. When you are searching for a keyword or file and are not confident that you will find the right match in the first few tries use this agent to perform the search for you.',
  tools: ['*'],
  source: 'built-in',
  baseDir: 'built-in',
  // model is intentionally omitted - uses getDefaultSubagentModel().
  getSystemPrompt: getGeneralPurposeSystemPrompt,
}
```
System prompt (lines 19-23):
```
You are an agent for Claude Code, Anthropic's official CLI for Claude. Given the user's message, you should use the tools available to complete the task. Complete the task fully—don't gold-plate, but don't leave it half-done. When you complete the task, respond with a concise report covering what was done and any key findings — the caller will relay this to the user, so it only needs the essentials.

Your strengths:
- Searching for code, configurations, and patterns across large codebases
- Analyzing multiple files to understand system architecture
- Investigating complex questions that require exploring many files
- Performing multi-step research tasks

Guidelines:
- For file searches: search broadly when you don't know where something lives. Use Read when you know the specific file path.
- For analysis: Start broad and narrow down. Use multiple search strategies if the first doesn't yield results.
- Be thorough: Check multiple locations, consider different naming conventions, look for related files.
- NEVER create files unless they're absolutely necessary for achieving your goal. ALWAYS prefer editing an existing file to creating a new one.
- NEVER proactively create documentation files (*.md) or README files. Only create documentation files if explicitly requested.
```

### 6.6 TeamCreateTool prompt (verbatim from `TeamCreateTool/prompt.ts:1-113`)

The full file content is the prompt and reproduces verbatim including the `# TeamCreate` heading, "When to Use" section, "Choosing Agent Types for Teammates", `team_name`/`description` JSON example, paths (`~/.claude/teams/{team-name}/config.json`, `~/.claude/tasks/{team-name}/`), 7-step "Team Workflow", "Task Ownership", "Automatic Message Delivery", "Teammate Idle State", "Discovering Team Members", "Task List Coordination", and "IMPORTANT notes for communication with your team" enumerating no-terminal-spying, no-structured-JSON status messages, and use-TaskUpdate-for-task-completion. (See `TeamCreateTool/prompt.ts` lines 1-113 — too long for inline but reproduced fully in disk-resident prompt; see Read above.)

### 6.7 TeamDeleteTool prompt (verbatim from `TeamDeleteTool/prompt.ts:1-16`)

```
# TeamDelete

Remove team and task directories when the swarm work is complete.

This operation:
- Removes the team directory (`~/.claude/teams/{team-name}/`)
- Removes the task directory (`~/.claude/tasks/{team-name}/`)
- Clears team context from the current session

**IMPORTANT**: TeamDelete will fail if the team still has active members. Gracefully terminate teammates first, then call TeamDelete after all teammates have shut down.

Use this when all teammates have finished their work and you want to clean up the team resources. The team name is automatically determined from the current session's team context.
```

### 6.8 SendMessageTool prompt (verbatim from `SendMessageTool/prompt.ts:1-49`)

```ts
export const DESCRIPTION = 'Send a message to another agent'
```

```
# SendMessage

Send a message to another agent.

```json
{"to": "researcher", "summary": "assign task 1", "message": "start on task #1"}
```

| `to` | |
|---|---|
| `"researcher"` | Teammate by name |
| `"*"` | Broadcast to all teammates — expensive (linear in team size), use only when everyone genuinely needs it |
{udsRow if feature('UDS_INBOX')}

Your plain text output is NOT visible to other agents — to communicate, you MUST call this tool. Messages from teammates are delivered automatically; you don't check an inbox. Refer to teammates by name, never by UUID. When relaying, don't quote the original — it's already rendered to the user.{udsSection if feature('UDS_INBOX')}

## Protocol responses (legacy)

If you receive a JSON message with `type: "shutdown_request"` or `type: "plan_approval_request"`, respond with the matching `_response` type — echo the `request_id`, set `approve` true/false:

```json
{"to": "team-lead", "message": {"type": "shutdown_response", "request_id": "...", "approve": true}}
{"to": "researcher", "message": {"type": "plan_approval_response", "request_id": "...", "approve": false, "feedback": "add error handling"}}
```

Approving shutdown terminates your process. Rejecting plan sends the teammate back to revise. Don't originate `shutdown_request` unless asked. Don't send structured JSON status messages — use TaskUpdate.
```

`udsRow` (only when `feature('UDS_INBOX')`):
```
| `"uds:/path/to.sock"` | Local Claude session's socket (same machine; use `ListPeers`) |
| `"bridge:session_..."` | Remote Control peer session (cross-machine; use `ListPeers`) |
```

`udsSection` (only when `feature('UDS_INBOX')`):
```
## Cross-session

Use `ListPeers` to discover targets, then:

```json
{"to": "uds:/tmp/cc-socks/1234.sock", "message": "check if tests pass over there"}
{"to": "bridge:session_01AbCd...", "message": "what branch are you on?"}
```

A listed peer is alive and will process your message — no "busy" state; messages enqueue and drain at the receiver's next tool round. Your message arrives wrapped as `<cross-session-message from="...">`. **To reply to an incoming message, copy its `from` attribute as your `to`.**
```

### 6.9 Verbatim `checkPermissions`

#### AgentTool (`AgentTool.tsx:1281-1297`)
```ts
async checkPermissions(input, context): Promise<PermissionResult> {
  const appState = context.getAppState();
  // Only route through auto mode classifier when in auto mode
  // In all other modes, auto-approve sub-agent generation
  // Note: "external" === 'ant' guard enables dead code elimination for external builds
  if ("external" === 'ant' && appState.toolPermissionContext.mode === 'auto') {
    return {
      behavior: 'passthrough',
      message: 'Agent tool requires permission to spawn sub-agents.'
    };
  }
  return {
    behavior: 'allow',
    updatedInput: input
  };
}
```

#### SendMessageTool (`SendMessageTool.ts:585-602`)
```ts
async checkPermissions(input, _context) {
  if (feature('UDS_INBOX') && parseAddress(input.to).scheme === 'bridge') {
    return {
      behavior: 'ask' as const,
      message: `Send a message to Remote Control session ${input.to}? It arrives as a user prompt on the receiving Claude (possibly another machine) via Anthropic's servers.`,
      // safetyCheck (not mode) — permissions.ts guards this before both
      // bypassPermissions (step 1g) and auto-mode's allowlist/classifier.
      // Cross-machine prompt injection must stay bypass-immune.
      decisionReason: {
        type: 'safetyCheck',
        reason: 'Cross-machine bridge message requires explicit user consent',
        classifierApprovable: false,
      },
    }
  }
  return { behavior: 'allow' as const, updatedInput: input }
}
```

`TeamCreateTool` and `TeamDeleteTool` do **not** override `checkPermissions` — they inherit the `TOOL_DEFAULTS.checkPermissions` from `buildTool` (allow-with-input). Validation is enforced via `validateInput` (TeamCreate's empty-name check) and via the `TOOL_DEFAULTS.isReadOnly = false` default (so they go through the standard write-permission path in spec 09).

### 6.10 Constants table

| Constant | Value | Site |
|---|---|---|
| `AGENT_TOOL_NAME` | `'Agent'` | `tools/AgentTool/constants.ts:1` |
| `LEGACY_AGENT_TOOL_NAME` | `'Task'` | `tools/AgentTool/constants.ts:3` |
| `VERIFICATION_AGENT_TYPE` | `'verification'` | `tools/AgentTool/constants.ts:4` |
| `ONE_SHOT_BUILTIN_AGENT_TYPES` | `Set(['Explore','Plan'])` | `tools/AgentTool/constants.ts:9-12` |
| `TEAM_CREATE_TOOL_NAME` | `'TeamCreate'` | `tools/TeamCreateTool/constants.ts:1` |
| `TEAM_DELETE_TOOL_NAME` | `'TeamDelete'` | `tools/TeamDeleteTool/constants.ts:1` |
| `SEND_MESSAGE_TOOL_NAME` | `'SendMessage'` | `tools/SendMessageTool/constants.ts:1` |
| `PROGRESS_THRESHOLD_MS` | `2000` | `AgentTool.tsx:63` |
| `getAutoBackgroundMs() return on flag` | `120_000` | `AgentTool.tsx:73-76` |
| MCP-server pending poll deadline | `MAX_WAIT_MS = 30_000`, `POLL_INTERVAL_MS = 500` | `AgentTool.tsx:378-379` |
| `maxResultSizeChars` (all four tools) | `100_000` | `AgentTool.tsx:229`, `TeamCreateTool.ts:77`, `TeamDeleteTool.ts:35`, `SendMessageTool.ts:524` |
| AgentTool `isReadOnly` | `() => true` (delegates to underlying tools) | `AgentTool.tsx:1264-1266` |
| AgentTool `isConcurrencySafe` | `() => true` | `AgentTool.tsx:1273-1275` |
| AgentTool `shouldDefer` | unset (always loaded) | — |
| TeamCreate/Delete/SendMessage `shouldDefer` | `true` | `TeamCreateTool.ts:78`, `TeamDeleteTool.ts:36`, `SendMessageTool.ts:533` |
| SendMessage `isReadOnly` | `(input) => typeof input.message === 'string'` | `SendMessageTool.ts:539-541` |
| Default agent worker permission mode | `'acceptEdits'` (when agent doesn't set one) | `AgentTool.tsx:575` |
| Default `subagent_type` (non-fork) | `GENERAL_PURPOSE_AGENT.agentType = 'general-purpose'` | `AgentTool.tsx:322`, `generalPurposeAgent.ts:26` |
| Worktree slug | `agent-${earlyAgentId.slice(0,8)}` | `AgentTool.tsx:591` |

### 6.11 Lazy-getter wiring for circular-dep break (verbatim from `tools.ts:61-72`)

```ts
// Lazy require to break circular dependency: tools.ts -> TeamCreateTool/TeamDeleteTool -> ... -> tools.ts
/* eslint-disable @typescript-eslint/no-require-imports */
const getTeamCreateTool = () =>
  require('./tools/TeamCreateTool/TeamCreateTool.js')
    .TeamCreateTool as typeof import('./tools/TeamCreateTool/TeamCreateTool.js').TeamCreateTool
const getTeamDeleteTool = () =>
  require('./tools/TeamDeleteTool/TeamDeleteTool.js')
    .TeamDeleteTool as typeof import('./tools/TeamDeleteTool/TeamDeleteTool.js').TeamDeleteTool
const getSendMessageTool = () =>
  require('./tools/SendMessageTool/SendMessageTool.js')
    .SendMessageTool as typeof import('./tools/SendMessageTool/SendMessageTool.js').SendMessageTool
/* eslint-enable @typescript-eslint/no-require-imports */
```

---

## 7. Side Effects & I/O

The tool surface itself performs the following side effects directly:

| Effect | Tool | Site |
|---|---|---|
| `setAppState(teamContext: undefined, inbox.messages = [])` | TeamDelete | `TeamDeleteTool.ts:117-124` |
| `setAppState(teamContext: { teamName, ... })` | TeamCreate | `TeamCreateTool.ts:194-212` |
| `writeTeamFileAsync(name, file)` | TeamCreate | `TeamCreateTool.ts:177` |
| `cleanupTeamDirectories(teamName)` | TeamDelete | `TeamDeleteTool.ts:101` |
| `resetTaskList(taskListId)` + `ensureTasksDir(taskListId)` | TeamCreate | `TeamCreateTool.ts:185-186` |
| `setLeaderTeamName(taskListId)` / `clearLeaderTeamName()` | TeamCreate / TeamDelete | `TeamCreateTool.ts:191`, `TeamDeleteTool.ts:109` |
| `registerTeamForSessionCleanup(name)` / `unregister...` | TeamCreate / TeamDelete | `TeamCreateTool.ts:180`, `TeamDeleteTool.ts:103` |
| `setAgentColor(agentType, color)` | AgentTool (multi-agent + normal) | `AgentTool.tsx:288, 414` |
| `assignTeammateColor(leadAgentId)` / `clearTeammateColors()` | TeamCreate / TeamDelete | `TeamCreateTool.ts:204`, `TeamDeleteTool.ts:106` |
| `createAgentWorktree(slug)` / `removeAgentWorktree` / `hasWorktreeChanges` | AgentTool | `AgentTool.tsx:592, 668-672` |
| `writeAgentMetadata(agentId, ...)` (fire-and-forget) | AgentTool | `AgentTool.tsx:673-676` |
| `writeToMailbox(name, msg, teamName)` | SendMessage handlers | `SendMessageTool.ts:161-171, 238-249, 285-293` |
| `sendToUdsSocket(target, msg)` | SendMessage UDS branch | `SendMessageTool.ts:781` |
| `postInterClaudeMessage(target, msg)` | SendMessage bridge branch | `SendMessageTool.ts:761` |
| `queuePendingMessage(agentId, msg, setAppState)` | SendMessage in-process branch | `SendMessageTool.ts:810-814` |
| `resumeAgentBackground({...})` | SendMessage in-process branch | `SendMessageTool.ts:824-857` |
| `logEvent('tengu_team_created' / 'tengu_team_deleted' / 'tengu_agent_tool_selected' / 'tengu_agent_tool_remote_launched' / 'tengu_agent_memory_loaded')` | various | `TeamCreateTool.ts:214-222`, `TeamDeleteTool.ts:111-114`, `AgentTool.tsx:419-428, 466-468, 524-530` |
| Module-load read of `process.env.CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` | AgentTool | `AgentTool.tsx:66-68` |
| Module-load read of `process.env.USER_TYPE` | AgentTool, loadAgentsDir, exploreAgent | `AgentTool.tsx:94-99, 273, 435, 1287`, `loadAgentsDir.ts:94-97, 610`, `exploreAgent.ts:78` |

---

## 8. Feature Flags & Variants

### 8.1 `feature(...)` gates affecting this surface

| Flag | Effect | Site |
|---|---|---|
| `BUILTIN_EXPLORE_PLAN_AGENTS` | Enables `areExplorePlanAgentsEnabled()` (further gated by `tengu_amber_stoat`, default `true`) → adds `EXPLORE_AGENT, PLAN_AGENT` to built-in list | `builtInAgents.ts:14` |
| `PROMPT_CACHE_BREAK_DETECTION` | Worker-loop cleanup: `cleanupAgentTracking(agentId)` in `runAgent.ts:824` `finally` block | `runAgent.ts:824` |
| `FORK_SUBAGENT` (via `isForkSubagentEnabled()`) | Hides `run_in_background` from input schema; flips default routing (omitted `subagent_type` → fork instead of general-purpose); inserts `whenToForkSection` and `forkExamples` into prompt; activates `forceAsync` so all spawns become async; switches example block; conditionally inserts `worktreeNotice` user-message in fork+worktree path; routes through `FORK_AGENT` | `forkSubagent.ts:isForkSubagentEnabled`, `AgentTool.tsx:122-124, 322-335, 557, 598-602`; `prompt.ts:78, 110, 232-233, 261-263, 267, 284-286` |
| `AGENT_MEMORY_SNAPSHOT` | Triggers `launchSnapshotUpdateDialog` in main bootstrap when `--agent` mode + custom-agent + `memory:` set + `pendingSnapshotUpdate` present | `main.tsx:2258` |
| `COORDINATOR_MODE` | (a) `getBuiltInAgents` returns `getCoordinatorAgents()` when `CLAUDE_CODE_COORDINATOR_MODE` truthy; (b) `AgentTool.prompt` returns slim shared header when `isCoordinator`; (c) AgentTool ignores `model` param when in coordinator mode; (d) `assistantForceAsync`/`shouldRunAsync` include `isCoordinator` flag | `builtInAgents.ts:35-43`, `prompt.ts:216-218`, `AgentTool.tsx:252, 553, 567` |
| `MONITOR_TOOL` | `runAgent` `finally` calls `killMonitorMcpTasksForAgent` (cleanup hook only) | `runAgent.ts:849-857` |
| `KAIROS` | (a) Includes `cwd` field in input schema; (b) `assistantForceAsync = appState.kairosEnabled` flips all spawns to async | `AgentTool.tsx:111, 566` |
| `UDS_INBOX` | Extends `SendMessage` `to` description with bridge/uds schemes; adds bridge/uds routing + validation; adds `udsRow`/`udsSection` to prompt; lifts cross-session structured-message rejection | `SendMessageTool.ts:73, 631, 658, 685, 742, 775`; `SendMessageTool/prompt.ts:6-21` |
| `VERIFICATION_AGENT` | Conditionally appends `VERIFICATION_AGENT` (further gated by `tengu_hive_evidence`, default `false`) | `builtInAgents.ts:64-69` |

### 8.2 Non-`feature()` runtime gates

| Gate | Effect | Site |
|---|---|---|
| **Build-time** `"external" === 'ant'` in AgentTool (DCE marker, NOT a runtime env check — see H1 fix in §3.1) | (a) `isolation` enum includes `'remote'`; (b) `'remote'` branch in `call()` runs; (c) `checkPermissions` `auto`-mode passthrough branch is **emitted**; (d) ANT-only `'remote'` note appended to prompt; (e) ANT-only debug log of API path. **In external builds, all five branches are eliminated by the bundler — they are NOT skipped at runtime.** | `AgentTool.tsx:94, 99, 273, 435, 1287`; `runAgent.ts:362` |
| **Runtime** `process.env.USER_TYPE === 'ant'` (loadAgentsDir frontmatter) | Custom-agent `isolation: 'remote'` accepted. Note (adversarial C1): unlike AgentTool above, `loadAgentsDir.ts` uses a real `process.env` runtime check, so external builds still pay the comparison cost at every load. | `loadAgentsDir.ts:94, 610, 755` |
| **Runtime** `process.env.USER_TYPE === 'ant'` (built-in/exploreAgent) | `EXPLORE_AGENT.model = 'inherit'` (else `'haiku'`) | `built-in/exploreAgent.ts:78` |
| **Runtime** `process.env.USER_TYPE === 'ant'` (agentSwarmsEnabled) | Always returns `true` for ANT (skips killswitch check); external requires opt-in env or flag plus `tengu_amber_flint` | `utils/agentSwarmsEnabled.ts:25-44` |
| `process.env.CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` truthy (module load) | Hides `run_in_background` field; `shouldRunAsync` always false | `AgentTool.tsx:66-68, 567`; `prompt.ts:259-260` |
| `process.env.CLAUDE_AUTO_BACKGROUND_TASKS` truthy OR `tengu_auto_background_agents` true | `getAutoBackgroundMs() = 120_000` (else `0`) | `AgentTool.tsx:72-77` |
| `process.env.CLAUDE_CODE_AGENT_LIST_IN_MESSAGES` (true/false/unset) | Override for `shouldInjectAgentListInMessages`; falls through to `tengu_agent_list_attach` (default false) | `prompt.ts:60-63` |
| `process.env.CLAUDE_CODE_COORDINATOR_MODE` truthy | Activates coordinator built-in agents and slim coordinator prompt | `builtInAgents.ts:36`, `AgentTool.tsx:223, 553` |
| `process.env.CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS` + `getIsNonInteractiveSession()` | `getBuiltInAgents()` returns `[]` (SDK blank-slate) | `builtInAgents.ts:25-30` |
| `process.env.CLAUDE_CODE_ENTRYPOINT in {sdk-ts, sdk-py, sdk-cli}` | Suppresses `CLAUDE_CODE_GUIDE_AGENT` from built-ins | `builtInAgents.ts:55-62` |
| `isAgentSwarmsEnabled()` | Gates `TeamCreateTool.isEnabled`, `TeamDeleteTool.isEnabled`, `SendMessageTool.isEnabled`, AgentTool's `team_name`/`name` spawn path, `resolveTeamName()`, AgentTool's `team_name && !enabled` throw | all four tools |
| `isInProcessTeammate()`, `isTeammate()` | Branch teammate-restriction notes in prompt; gate spawn-rejections in AgentTool `call()` | `prompt.ts:277-283`, `AgentTool.tsx:272-279, 361-363` |
| `getSubscriptionType() !== 'pro'` | Inserts the "Launch multiple agents concurrently" prompt note (only when not listing-via-attachment) | `prompt.ts:245-249` |
| GrowthBook `tengu_amber_stoat` (default `true`) | Killswitch under `BUILTIN_EXPLORE_PLAN_AGENTS` | `builtInAgents.ts:17` |
| GrowthBook `tengu_hive_evidence` (default `false`) | Killswitch under `VERIFICATION_AGENT` | `builtInAgents.ts:66` |
| GrowthBook `tengu_amber_flint` (default `true`) | External-builds killswitch in `isAgentSwarmsEnabled` | `agentSwarmsEnabled.ts:40` |
| GrowthBook `tengu_explore_agent` | Runtime model-resolution branch for ANT Explore (consumed by `getAgentModel()`) | comment at `exploreAgent.ts:77` |
| GrowthBook `tengu_agent_list_attach` (default `false`) | Default for `shouldInjectAgentListInMessages` when no env override | `prompt.ts:63` |
| GrowthBook `tengu_auto_background_agents` (default `false`) | Auto-background fallback when env not set | `AgentTool.tsx:73` |
| `proactiveModule?.isProactiveActive()` | Forces async when proactive module is loaded and active | `AgentTool.tsx:567` |

### 8.3 Variants table

| Variant | Surface delta vs production-default |
|---|---|
| ANT (`USER_TYPE='ant'`) | `isolation` includes `'remote'`; `'remote'` branch in `call()` active; `auto` mode `passthrough`; `EXPLORE_AGENT.model='inherit'`; `isAgentSwarmsEnabled()` always true; ANT-only debug log; ANT-only `'remote'` prompt line |
| `FORK_SUBAGENT` on | Schema omits `run_in_background`; default routing → fork; prompt swaps to fork sections + fork examples; `forceAsync=true`; `whenNotToUseSection` suppressed |
| `KAIROS` on | Schema includes `cwd`; assistant-mode forces all spawns async |
| `COORDINATOR_MODE` + `CLAUDE_CODE_COORDINATOR_MODE` truthy | Built-in list comes from `getCoordinatorAgents()`; prompt is slim shared header only; AgentTool ignores `model` param; spawns forced async |
| `BUILTIN_EXPLORE_PLAN_AGENTS` off (or `tengu_amber_stoat=false`) | Explore/Plan removed from built-ins |
| Agent swarms disabled | Team tools `isEnabled=false`; AgentTool rejects `team_name` with "not yet available on your plan" |
| `UDS_INBOX` on | SendMessage prompt grows uds/bridge rows; cross-session routing branches active |
| Background tasks disabled | `run_in_background` field hidden; sync-only |
| `CLAUDE_AGENT_SDK_DISABLE_BUILTIN_AGENTS` (non-interactive) | Built-in list empty; only custom/plugin agents available |

---

## 9. Error Handling & Edge Cases

| Condition | Tool | Result | Site |
|---|---|---|---|
| `team_name` set but swarms disabled | AgentTool | throw `Agent Teams is not yet available on your plan.` | `AgentTool.tsx:262-264` |
| Teammate (in-process or tmux) tries to spawn another teammate (`name` set) | AgentTool | throw `Teammates cannot spawn other teammates — the team roster is flat. ...` | `AgentTool.tsx:272-274` |
| In-process teammate spawning with `run_in_background=true` | AgentTool | throw `In-process teammates cannot spawn background agents. ...` | `AgentTool.tsx:278-280` |
| In-process teammate spawning agent whose definition has `background:true` | AgentTool | throw with agentType in message | `AgentTool.tsx:361-363` |
| Fork attempted from inside a fork child | AgentTool | throw `Fork is not available inside a forked worker. Complete your task directly using your tools.` | `AgentTool.tsx:332-334` |
| `subagent_type` not found vs denied | AgentTool | distinguishes "denied by permission rule" (cites source) vs "not found, available agents: [list]" | `AgentTool.tsx:347-353` |
| Required MCP servers not all available after 30s poll | AgentTool | throw with missing list and `serversWithTools` list and `/mcp` hint | `AgentTool.tsx:407-408` |
| Remote eligibility fails | AgentTool | throw `Cannot launch remote agent:\n${reasons}` | `AgentTool.tsx:438-439` |
| `teleportToRemote` returns no session | AgentTool | throw `bundleFailHint ?? 'Failed to create remote session'` | `AgentTool.tsx:450-451` |
| TeamCreate while already leading a team | TeamCreate | throw `Already leading team "${name}". A leader can only manage one team at a time. Use TeamDelete to end the current team before creating a new one.` | `TeamCreateTool.ts:137-140` |
| TeamCreate `team_name` empty | TeamCreate | `validateInput` returns `result:false, errorCode:9` | `TeamCreateTool.ts:96-105` |
| TeamCreate `team_name` collides | TeamCreate | renames via `generateWordSlug()` and proceeds | `TeamCreateTool.ts:64-72, 143` |
| TeamDelete with active members | TeamDelete | returns `success:false, message: 'Cannot cleanup team with N active member(s): ...'` | `TeamDeleteTool.ts:89-98` |
| TeamDelete when no team in context | TeamDelete | returns `success:true, message: 'No team name found, nothing to clean up'` | `TeamDeleteTool.ts:128-134` |
| SendMessage to bridge but bridge dropped between validate and call | SendMessage | returns `success:false, message: 'Remote Control disconnected before send — cannot deliver to ${to}'` | `SendMessageTool.ts:749-755` |
| SendMessage UDS send throws | SendMessage | returns `success:false, message: 'Failed to send to ${to}: ${err}'` | `SendMessageTool.ts:789-795` |
| SendMessage to stopped task that fails to resume | SendMessage | returns `success:false, message: 'Agent ... is stopped (${status}) and could not be resumed: ${err}'` | `SendMessageTool.ts:837-844` |
| SendMessage broadcast when sender is sole team member | SendMessage | returns `success:true, message: 'No teammates to broadcast to (you are the only team member)', recipients: []` | `SendMessageTool.ts:228-236` |
| SendMessage broadcast outside team context | SendMessage | throw `Not in a team context. Create a team with Teammate spawnTeam first, or set CLAUDE_CODE_TEAM_NAME.` | `SendMessageTool.ts:199-203` |
| SendMessage broadcast: team file missing | SendMessage | throw `Team "${teamName}" does not exist` | `SendMessageTool.ts:206-208` |
| SendMessage `to` empty / contains `@` / `summary` missing for string / structured to `*` / `shutdown_response` not to lead / shutdown reject without reason | SendMessage | `validateInput` rejection with errorCode 9 | `SendMessageTool.ts:604-718` |
| `mapToolResultToToolResultBlockParam` exhaustiveness violated | AgentTool | `data satisfies never` + `throw new Error('Unexpected agent tool result status: ...')` | `AgentTool.tsx:1375-1378` |
| Subagent completed with empty content | AgentTool | substitutes `(Subagent completed but returned no output.)` text marker | `AgentTool.tsx:1346-1350` |

---

## 10. Telemetry & Observability

| Event | Tool | Site |
|---|---|---|
| `tengu_agent_tool_selected` | AgentTool | `AgentTool.tsx:419-428` |
| `tengu_agent_tool_remote_launched` | AgentTool (remote isolation) | `AgentTool.tsx:466-468` |
| `tengu_agent_memory_loaded` | AgentTool (when subagent has `memory:`) | `AgentTool.tsx:524-530` |
| `tengu_team_created` | TeamCreate | `TeamCreateTool.ts:214-222` |
| `tengu_team_deleted` | TeamDelete | `TeamDeleteTool.ts:111-114` |

`SendMessageTool.backfillObservableInput` rewrites `input.type/recipient/content/request_id/approve` for downstream auto-classifier observability rather than emitting a custom event (`SendMessageTool.ts:543-569`).

**`is_async` analytics-vs-runtime divergence** (see §3.1 step 15; adversarial M1): `tengu_agent_tool_selected.is_async` (`AgentTool.tsx:426`) and `metadata.isAsync` (`AgentTool.tsx:548`) compute `(run_in_background === true || selectedAgent.background === true) && !isBackgroundTasksDisabled` only — they do not include `isCoordinator`, `forceAsync` (fork), `assistantForceAsync` (Kairos), or `proactiveModule?.isProactiveActive()`. The runtime `shouldRunAsync` decision DOES include them. Therefore async runs caused by coordinator/fork/Kairos/proactive will appear as `is_async=false` in this telemetry.

---

## 11. Reimplementation Checklist

- [ ] Tool names exactly: `'Agent'` (alias `'Task'`), `'TeamCreate'`, `'TeamDelete'`, `'SendMessage'`. Legacy alias `'Task'` is required for backward-compat with permission rules and resumed sessions.
- [ ] AgentTool input schema: 3-layer composition via `lazySchema` (base + multi-agent + extension). `isolation` enum widens to include `'remote'` only when ANT (build-time literal `"external" === 'ant'`). `cwd` only present when `KAIROS`. `run_in_background` `.omit()`-ed when `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS` (module-load) OR `isForkSubagentEnabled()`. Explicit `AgentToolInput` type widens schema inference to include all optional fields after `.omit()`.
- [ ] AgentTool `isReadOnly: () => true` (delegates to underlying tools). `isConcurrencySafe: () => true`. No `shouldDefer` (always loaded).
- [ ] AgentTool `prompt()`: filter agents by MCP requirements then by deny rules; coordinator mode returns slim shared header only; otherwise full prompt with optional whenNotToUse/concurrency/background/teammate-restriction/fork sections.
- [ ] AgentTool `checkPermissions`: `passthrough` only when ANT + `mode === 'auto'`; otherwise `allow`. Comment about ANT-only DCE guard preserved.
- [ ] AgentTool spawn routing decision tree: swarm gate → teammate guards → multi-agent spawn → fork-recursion guard → agent lookup (with denied-vs-not-found differentiation) → MCP server poll (30s, 500ms) → telemetry → ANT remote branch → system-prompt branch → async decision (run_in_background OR background OR coordinator OR forceAsync OR assistantForceAsync OR proactiveActive, all gated by `!isBackgroundTasksDisabled`) → worker tool pool via `assembleToolPool` → worktree creation.
- [ ] Default `subagent_type` is `'general-purpose'` when fork is OFF; `undefined` (= fork path) when fork is ON.
- [ ] `mapToolResultToToolResultBlockParam`: four status branches; `ONE_SHOT_BUILTIN_AGENT_TYPES` (Explore, Plan) skip the `agentId`/`<usage>` trailer when no worktree info; empty-content subagent gets the explicit "(Subagent completed but returned no output.)" marker; `data satisfies never` on unknown status.
- [ ] AgentTool worker permissionContext defaults `mode = selectedAgent.permissionMode ?? 'acceptEdits'`.
- [ ] **No per-call tool-count budget at this surface (adversarial M2)**: `AgentDefinition` exposes `maxTurns` (`loadAgentsDir.ts:73-`, applied in `runAgent.ts:259, 756`) but no `maxTools`/`toolBudget`/per-call tool-count limit. Any cross-spec reference to a "tool budget" at the AgentTool surface is wrong — the only quantitative cap on subagent execution exposed via the agent definition is the turn count. If specs 30/41/15 reference a tool budget, it is either (a) imposed elsewhere in the runner (spec 30) or queue layer (15), or (b) drift to be removed.
- [ ] `EXPLORE_AGENT.model = USER_TYPE==='ant' ? 'inherit' : 'haiku'`. `PLAN_AGENT.model = 'inherit'`. `GENERAL_PURPOSE_AGENT.model` omitted (uses `getDefaultSubagentModel()`).
- [ ] `EXPLORE_AGENT`, `PLAN_AGENT`: `omitClaudeMd: true`; `disallowedTools = [Agent, ExitPlanMode, Edit, Write, NotebookEdit]`; READ-ONLY-mode system prompt with explicit Bash whitelist/blacklist.
- [ ] Built-in agent list assembly order: `[GENERAL_PURPOSE, STATUSLINE_SETUP] + (explore/plan if gate) + (CLAUDE_CODE_GUIDE if non-SDK) + (VERIFICATION if gate)`.
- [ ] `getBuiltInAgents()` early returns: SDK-disabled (env+non-interactive) → `[]`; coordinator-mode → `getCoordinatorAgents()` via lazy require.
- [ ] `getPrompt()` listViaAttachment: env override → `tengu_agent_list_attach` (default false). When true, replace inline list with one-line pointer to `<system-reminder>`. Suppress concurrencyNote in that branch.
- [ ] `formatAgentLine` and `getToolsDescription`: four cases (allow+deny, allow only, deny only, neither).
- [ ] TeamCreate: `shouldDefer: true`, `userFacingName: () => ''`, `isEnabled = isAgentSwarmsEnabled`. `validateInput` rejects empty name. `call`: refuses if already leading; auto-renames on collision; deterministic `leadAgentId = formatAgentId(TEAM_LEAD_NAME, finalTeamName)`; resolves leadModel from session/AppState/default chain; writes team file, registers session-cleanup, resets+ensures task list, sets leader-team name, updates AppState; logs `tengu_team_created`. **Does NOT set `CLAUDE_CODE_AGENT_ID` for the lead.**
- [ ] TeamDelete: `shouldDefer: true`, `userFacingName: () => ''`, `inputSchema = z.strictObject({})`, `isEnabled = isAgentSwarmsEnabled`. Refuses with `success:false` when active non-lead members remain (where `isActive !== false`). On clean: `cleanupTeamDirectories`, `unregisterTeamForSessionCleanup`, `clearTeammateColors`, `clearLeaderTeamName`. Always clears `teamContext` and `inbox.messages` from AppState. Idempotent when no team set.
- [ ] SendMessage: `shouldDefer: true`, `isEnabled = isAgentSwarmsEnabled`, `isReadOnly = (input) => typeof input.message === 'string'`. Structured-message Zod discriminated union of {shutdown_request, shutdown_response, plan_approval_response}. `validateInput` enforces all the rules in §3.4.1. `checkPermissions`: bridge target → `ask` with `safetyCheck` `classifierApprovable: false` (bypass-immune); else `allow`. `backfillObservableInput` fills `input.type/recipient/content/request_id/approve` for analytics.
- [ ] SendMessage `call()` routing precedence: UDS_INBOX bridge → UDS_INBOX uds → in-process subagent (queue if running, resume otherwise) → broadcast → handleMessage → structured (shutdown_request/response, plan_approval_response). Structured to `*` throws.
- [ ] Lazy `require()` getters for `TeamCreateTool`, `TeamDeleteTool`, `SendMessageTool` in `tools.ts:62-72` with `as typeof import(...)` cast and ESLint disable. SendMessageTool unconditionally inserted; Team* tools gated by `isAgentSwarmsEnabled()`. AgentTool statically imported.
- [ ] `isAgentSwarmsEnabled()` gate logic: ANT always true; external requires `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` env OR `--agent-teams` flag AND `tengu_amber_flint` (default true).
- [ ] `feature('PROMPT_CACHE_BREAK_DETECTION')`: `cleanupAgentTracking(agentId)` in `runAgent.ts` `finally` block.
- [ ] `feature('AGENT_MEMORY_SNAPSHOT')` at `main.tsx:2258`: gated dialog dispatch when `--agent` mode + custom-agent + `memory` set + `pendingSnapshotUpdate` present. `merge` choice prepends `buildMergePrompt()` to the user's input.
- [ ] `formatAgentId(TEAM_LEAD_NAME, teamName)` produces deterministic lead ID; lead is **not** marked teammate (`isTeammate()` must remain false for it).
- [ ] All four tools have `maxResultSizeChars: 100_000`.

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. Most items are spec-30 cross-references that have since been resolved by Phase 9.6 (`PHASE9-FIXES-30.md`).

1. ~~**Spec 30 boundary line**~~ — **RESOLVED Phase 9.7**: spec 30 §1-§5 now documents `runAgent.ts` worker loop, `forkSubagent.ts` internals (incl. `buildForkedMessages`, `isInForkChild`), `agentToolUtils.ts:508` finalization, `agentMemory.ts:13/52-65` injection, `agentColorManager.ts` color rotation, `agentDisplay.ts` rendering, `resumeAgent.ts` resume, and `tools/shared/spawnMultiAgent.ts:spawnTeammate`. Boundary made explicit in `PHASE9-FIXES-30.md` M5 (symbol-ownership vs behavior-ownership).
2. ~~**`tasks/LocalAgentTask`/`RemoteAgentTask`/`InProcessTeammateTask`**~~ — **RESOLVED Phase 9.7**: spec 15 (tasks) §3 enumerates all three task types and `queuePendingMessage`/`resumeAgentBackground` contracts.
3. ~~**`isCoordinatorMode()` vs inline env check**~~ — **RESOLVED Phase 9.7**: per `PHASE9-FIXES-30.md`, `isCoordinatorMode` (`coordinator/coordinatorMode.ts:36-41`) requires `feature('COORDINATOR_MODE')` AND env-truthy `CLAUDE_CODE_COORDINATOR_MODE`. The inline env check in `AgentTool.tsx` is a circular-dep workaround; semantic equivalence holds because `feature('COORDINATOR_MODE')` is build-time DCE'd, so at runtime the env check alone suffices when the build flag is on.
4. ~~**`getCoordinatorAgents()` content**~~ — **RESOLVED Phase 9.7**: spec 30 §3 enumerates the coordinator built-in agent list (cited from `tools/AgentTool/builtInAgents.ts` lazy require of `coordinator/workerAgent.js`).
5. ~~**`agentMemory.ts` and `loadAgentMemoryPrompt`**~~ — **RESOLVED Phase 9.7**: spec 30 §4 documents the user/project/local memory dirs (`agentMemory.ts:52-65`), the colon-replacement rule (`:20-22`), and the snapshot-state struct (`agentMemorySnapshot.ts:14-25`).
6. ~~**`AGENT_COLORS` palette**~~ — **RESOLVED Phase 9.7**: spec 30 §2.1 references `agentColorManager.ts` (66 LOC) for the assignment table; rotation is round-robin from a fixed palette.
7. ~~**`STATUSLINE_SETUP_AGENT` and `CLAUDE_CODE_GUIDE_AGENT`**~~ — **RESOLVED Phase 9.7**: spec 30 §2.1 enumerates `built-in/statuslineSetup.ts` (145 LOC) and `built-in/claudeCodeGuideAgent.ts` (206 LOC) with full system prompts.
8. ~~**`VERIFICATION_AGENT`**~~ — **RESOLVED Phase 9.7**: spec 30 §2.1 enumerates `built-in/verificationAgent.ts` (153 LOC), gated by `feature('VERIFICATION_AGENT')`.
9. ~~**`agentSwarmsEnabled.ts` tail behavior**~~ — **RESOLVED Phase 9.7**: per `PHASE9-FIXES-30.md`, `isAgentSwarmsEnabled` at `utils/agentSwarmsEnabled.ts:24-44` returns false when killswitch `tengu_amber_flint` is falsy AND not ant-internal. Tail behavior fully traced.
10. ~~**`isInForkChild(messages)` heuristic**~~ — **RESOLVED Phase 9.7**: spec 30 §5 documents the fork-child detection: scans messages for `FORK_AGENT` marker emitted by `buildChildMessage` (`forkSubagent.ts:107`+).
11. ~~**`semanticBoolean` schema**~~ — **NOTE Phase 9.7**: defined at `utils/semanticBoolean.ts`; spec 42 §A would absorb if not yet claimed. Coerces 'yes'/'no'/'true'/'false'/'1'/'0'/empty into boolean. Behavior obvious from name; no further documentation needed for this surface.
12. ~~**`checkAgentMemorySnapshot` invocation site**~~ — **RESOLVED Phase 9.7**: spec 30 §4 documents the call site at `loadAgentsDir.ts:49` that reads stored snapshot and the `main.tsx:2258` dialog that displays pending updates.
13. ~~**Lazy `require()` for `coordinator/workerAgent.js`**~~ — **NOTE Phase 9.7**: the in-function lazy-require pattern is identical to spec 08's `getTeamCreateTool` getter; preserved as observed.
14. ~~**Test-only behavior**~~ — **RESOLVED Phase 9.7**: confirmed no testing branch in AgentTool surface; `TestingPermissionTool` is registered separately at registry layer per spec 08 §3.
