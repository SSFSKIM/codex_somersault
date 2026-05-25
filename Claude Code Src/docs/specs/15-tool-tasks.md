# 15 — Tool: Tasks (TaskCreate / TaskUpdate / TaskGet / TaskList / TaskStop / TaskOutput / TodoWrite) and the Background Task Runtime

> Spec template: see `00-overview.md` §1. Adjacent specs: 08 (registry), 09 (permissions), 14 (agent — shares agent identity), 30 (coordinator/multi-agent), 41 (session/AppState).

## §0 Source-coverage inventory

Files read in full (canonical for behavioral claims):

- `src/Task.ts` (126 lines) — task type enum, ID generation, `TaskStateBase`, `Task` interface.
- `src/tasks.ts` (40 lines) — top-level registry `getAllTasks()` / `getTaskByType()`.
- `src/tasks/types.ts` (47 lines) — union `TaskState`, `BackgroundTaskState`, `isBackgroundTask`.
- `src/tasks/stopTask.ts` (100 lines) — shared kill helper used by `TaskStopTool` and SDK.
- `src/tasks/pillLabel.ts` (82 lines) — footer-pill label compositor.
- `src/tasks/LocalMainSessionTask.ts` (479 lines) — Ctrl+B-twice main-session backgrounding.
- `src/tasks/LocalShellTask/guards.ts` (41 lines) — `LocalShellTaskState` + `isLocalShellTask`.
- `src/tasks/LocalShellTask/killShellTasks.ts` (76 lines) — pure kill helpers.
- `src/tasks/InProcessTeammateTask/types.ts` (121 lines) — teammate task state, `TEAMMATE_MESSAGES_UI_CAP`.
- `src/tasks/DreamTask/DreamTask.ts` (157 lines) — auto-dream task (full).
- `src/tools/TaskCreateTool/{TaskCreateTool.ts,prompt.ts,constants.ts}` (full).
- `src/tools/TaskUpdateTool/{TaskUpdateTool.ts,prompt.ts,constants.ts}` (full).
- `src/tools/TaskGetTool/{TaskGetTool.ts,prompt.ts,constants.ts}` (full).
- `src/tools/TaskListTool/{TaskListTool.ts,prompt.ts,constants.ts}` (full).
- `src/tools/TaskStopTool/{TaskStopTool.ts,prompt.ts,UI.tsx}` (full).
- `src/tools/TaskOutputTool/{TaskOutputTool.tsx,constants.ts}` — Tool def + result formatting (file is 583 lines; lines 1-352 give full Tool def, the remainder is React display components).
- `src/tools/TodoWriteTool/{TodoWriteTool.ts,prompt.ts,constants.ts}` (full).
- `src/utils/tasks.ts:1-630` — `TaskStatusSchema`, `TaskSchema`, `isTodoV2Enabled`, `getTaskListId`, locking, CRUD, `claimTask`.
- `src/utils/todo/types.ts` (full).

Sample-and-grep:

- `src/tasks/LocalShellTask/LocalShellTask.tsx` (522 lines): header (1-80) + grep for `name: 'LocalShellTask'` and `kill` (line 174-176).
- `src/tasks/LocalAgentTask/LocalAgentTask.tsx` (682 lines): header (1-100) + `name: 'LocalAgentTask'` (line 271-273).
- `src/tasks/RemoteAgentTask/RemoteAgentTask.tsx` (855 lines): header (1-100) + `name: 'RemoteAgentTask'` (line 809-811).
- `src/tasks/InProcessTeammateTask/InProcessTeammateTask.tsx`: `name: 'InProcessTeammateTask'` (line 25-27).
- `src/tools.ts:218-219` — gated registration of TodoV2 tools.

Out-of-scope cross-references (cited, not redocumented): permission flow (09), tool registry plumbing (08), agent dispatch & teammate identity (14), coordinator/team layout (30), `AppState.tasks` persistence and replay (41).

---

## §1 Purpose

Two parallel todo subsystems plus a background-task runtime:

1. **TodoWrite (V1)** — single-tool, in-memory session checklist keyed by `agentId ?? sessionId`; rewrites the entire list on every call. Active when `!isTodoV2Enabled()` (`src/utils/tasks.ts:133-139`).
2. **Task tools (V2)** — TaskCreate/Update/Get/List operating on durable, file-backed, per-task-list JSON store with file locks, auto-incrementing numeric IDs, and a high-water mark. Active when `isTodoV2Enabled()` (added to registry at `src/tools.ts:218-219`).
3. **TaskStop / TaskOutput** — operate on the *background-task runtime* (`AppState.tasks`, populated by `LocalShellTask`, `LocalAgentTask`, `RemoteAgentTask`, `InProcessTeammateTask`, `LocalWorkflowTask`, `MonitorMcpTask`, `DreamTask`). Distinct from todo storage.

The two subsystems are deliberately layered: TodoWrite/Task tools mutate **task lists**; TaskStop/TaskOutput mutate **runtime tasks**. They do not share a key space.

## §2 Scope

In: every file enumerated in §0. The ANT-only `userFacingName: () => (process.env.USER_TYPE === 'ant' ? '' : 'Stop Task')` at `src/tools/TaskStopTool/TaskStopTool.ts:46`. The `feature('VERIFICATION_AGENT')` dispatch at `src/tools/TaskUpdateTool/TaskUpdateTool.ts:335` and `src/tools/TodoWriteTool/TodoWriteTool.ts:78`. Migration of legacy ant-only status names (`open`/`resolved`/`planning`/`implementing`/`reviewing`/`verifying`) at `src/utils/tasks.ts:319-332`.

Out: registry/tool-base contract → 08; permission gating (these tools mostly use `shouldDefer: true` and `isConcurrencySafe: true` with no explicit permission rules) → 09; multi-agent dispatch and `team-name` resolution → 14, 30; `AppState` schema and replay-on-resume → 41.

---

## §3 Tools (one sub-section per invokable tool)

### §3.1 TaskCreate (`TaskCreate`)

`src/tools/TaskCreateTool/TaskCreateTool.ts:48-138`.

- `name: 'TaskCreate'`; `searchHint: 'create a task in the task list'`; `maxResultSizeChars: 100_000`; `shouldDefer: true`; `isConcurrencySafe: () => true`; `userFacingName: () => 'TaskCreate'`.
- `isEnabled() { return isTodoV2Enabled() }`.
- `toAutoClassifierInput(input) { return input.subject }`.
- `renderToolUseMessage() { return null }` (line 77-79) — UI-suppressed.
- Output mapping (line 130-137): `\`Task #${task.id} created successfully: ${task.subject}\``.

Behavior (line 80-129):
1. `createTask(getTaskListId(), { subject, description, activeForm, status: 'pending', owner: undefined, blocks: [], blockedBy: [], metadata })`.
2. Iterate `executeTaskCreatedHooks(...)` async generator; collect `getTaskCreatedHookMessage(result.blockingError)` into `blockingErrors`.
3. If any blocking errors: `await deleteTask(getTaskListId(), taskId)` and `throw new Error(blockingErrors.join('\n'))` (line 110-113) — task is rolled back.
4. `setAppState(prev => prev.expandedView === 'tasks' ? prev : { ...prev, expandedView: 'tasks' })` (line 115-119).

### §3.2 TaskUpdate (`TaskUpdate`)

`src/tools/TaskUpdateTool/TaskUpdateTool.ts:88-406`.

- Same flag-gate, defer, concurrency-safe, UI-suppressed semantics as TaskCreate.
- Status accepts `'pending' | 'in_progress' | 'completed' | 'deleted'` (line 33-35: `TaskStatusSchema().or(z.literal('deleted'))`).
- `toAutoClassifierInput` joins `[taskId, status?, subject?]` with spaces (line 114-119).

Behavior (line 123-363) — pseudocode:

```
if expandedView !== 'tasks': set expandedView = 'tasks'
existingTask = await getTask(taskListId, taskId)
if !existingTask: return { success:false, taskId, updatedFields:[], error:'Task not found' }

updatedFields = []
updates = {}
if subject !== undefined && subject !== existing.subject:        updates.subject; push 'subject'
if description !== undefined && != existing.description:         updates.description; push 'description'
if activeForm !== undefined && != existing.activeForm:           updates.activeForm; push 'activeForm'
if owner !== undefined && != existing.owner:                     updates.owner; push 'owner'

# Auto-claim on in_progress when on a swarm and no owner:
if isAgentSwarmsEnabled() && status === 'in_progress' && owner === undefined && !existing.owner:
    if getAgentName(): updates.owner = agentName; push 'owner'

# Metadata merge with null-deletes:
if metadata !== undefined:
    merged = { ...existing.metadata }
    for [k,v] in entries(metadata): if v === null: delete merged[k] else merged[k]=v
    updates.metadata = merged; push 'metadata'

if status !== undefined:
    if status === 'deleted':
        deleted = await deleteTask(taskListId, taskId)
        return { success:deleted, taskId, updatedFields: deleted?['deleted']:[],
                 error: deleted?undefined:'Failed to delete task',
                 statusChange: deleted?{from:existing.status,to:'deleted'}:undefined }
    if status !== existing.status:
        if status === 'completed':
            run executeTaskCompletedHooks(...) async generator;
            collect getTaskCompletedHookMessage(blockingError)
            if any blocking errors: return { success:false, taskId, updatedFields:[], error:join('\n') }
        updates.status = status; push 'status'

if Object.keys(updates).length: await updateTask(taskListId, taskId, updates)

# Mailbox notification on owner change (swarm only):
if updates.owner && isAgentSwarmsEnabled():
    senderName = getAgentName() || 'team-lead'
    payload = JSON.stringify({type:'task_assignment', taskId, subject, description,
                              assignedBy:senderName, timestamp: ISO})
    await writeToMailbox(updates.owner, {from:senderName, text:payload, timestamp, color:senderColor}, taskListId)

# addBlocks: filter out IDs already in existing.blocks; for each new id: blockTask(taskListId, taskId, blockId); push 'blocks'
# addBlockedBy: filter out IDs already in existing.blockedBy; for each new blockerId: blockTask(taskListId, blockerId, taskId); push 'blockedBy'

# Verification nudge — only on main-thread completion that closed all >=3 tasks with no 'verif' subject:
verificationNudgeNeeded = false
if feature('VERIFICATION_AGENT')
   && getFeatureValue_CACHED_MAY_BE_STALE('tengu_hive_evidence', false)
   && !context.agentId
   && updates.status === 'completed':
    allTasks = await listTasks(taskListId)
    if allTasks.every(t=>t.status==='completed') && allTasks.length >= 3 && !allTasks.some(t=>/verif/i.test(t.subject)):
        verificationNudgeNeeded = true

return { success:true, taskId, updatedFields,
         statusChange: updates.status?{from:existing.status,to:updates.status}:undefined,
         verificationNudgeNeeded }
```

Result rendering (line 364-405):
- On `success === false`: returns a non-error tool_result with `error || \`Task #${taskId} not found\`` so it does NOT trigger sibling tool cancellation in `StreamingToolExecutor` (comment at line 374-378).
- On success: `\`Updated task #${taskId} ${updatedFields.join(', ')}\``.
- If `statusChange?.to === 'completed' && getAgentId() && isAgentSwarmsEnabled()`: append `'\n\nTask completed. Call TaskList now to find your next available task or see if your work unblocked others.'` (line 386-394).
- If `verificationNudgeNeeded`: append the verification reminder (verbatim string in §6).

### §3.3 TaskGet (`TaskGet`)

`src/tools/TaskGetTool/TaskGetTool.ts:38-128`. Adds `isReadOnly: () => true`. Output is full task or null. Result rendering composes `Task #${id}: ${subject}\nStatus: ${status}\nDescription: ${description}` plus optional `Blocked by:` / `Blocks:` lines (line 109-126).

### §3.4 TaskList (`TaskList`)

`src/tools/TaskListTool/TaskListTool.ts:33-116`. Read-only, empty input schema. Filters out tasks whose `metadata?._internal` is truthy (line 68-70). For each remaining task it returns `{ id, subject, status, owner?, blockedBy }` where `blockedBy` is filtered to drop already-completed task IDs (line 73-83). Empty list → `'No tasks found'`. Otherwise lines of the form `#${id} [${status}] ${subject}${owner ? ' (owner)' : ''}${blockedBy.length ? ' [blocked by #...]' : ''}` (line 91-114).

### §3.5 TaskStop (`TaskStop`)

`src/tools/TaskStopTool/TaskStopTool.ts:39-131`.

- `aliases: ['KillShell']` (line 44).
- `userFacingName: () => (process.env.USER_TYPE === 'ant' ? '' : 'Stop Task')` (line 46) — ANT-only blank string.
- `shouldDefer: true`; `isConcurrencySafe: () => true`; no `isEnabled` gate (always available).
- `validateInput` (line 60-91): rejects when neither `task_id` nor `shell_id` provided (errorCode 1), task not found (errorCode 1), or task status not `'running'` (errorCode 3).
- `toAutoClassifierInput`: `input.task_id ?? input.shell_id ?? ''`.
- `description()` returns `'Stop a running background task by ID'`; `prompt()` returns the constant `DESCRIPTION` from `prompt.ts`.
- Body: dispatches to shared `stopTask(id, { getAppState, setAppState })` from `src/tasks/stopTask.ts`. Returns `{ message, task_id, task_type, command }`.
- `mapToolResultToToolResultBlockParam` returns `jsonStringify(output)` as content.

UI render (`src/tools/TaskStopTool/UI.tsx`):
- `renderToolUseMessage()` returns `''`.
- `renderToolResultMessage` returns `null` when literal `'external' === 'ant'` (a defunct branch — string literal compares to `'ant'` so it's always false in this build; comment present in source). Otherwise truncates command to `MAX_COMMAND_DISPLAY_LINES = 2` lines and `MAX_COMMAND_DISPLAY_CHARS = 160` characters and renders `<MessageResponse><Text>{command}{suffix}</Text></MessageResponse>` with suffix `'… · stopped'` if truncated, else `' · stopped'`.

### §3.6 TaskOutput (`TaskOutput`)

`src/tools/TaskOutputTool/TaskOutputTool.tsx:30-352`.

- `aliases: ['AgentOutputTool', 'BashOutputTool']` (line 150). See §3.6.1 for the full alias-resolution surface (this list is one of three co-conspiring sites; if it drifts from the others, legacy permission rules and legacy parameter-name normalization silently break).
- `isEnabled() { return "external" !== 'ant' }` (line 163-165) — same dead-string idiom; effectively always enabled in this build.
- `userFacingName: () => 'Task Output'`; `isReadOnly: () => true`; `isConcurrencySafe(input) { return this.isReadOnly?.(input) ?? false }`.
- Description: `'[Deprecated] — prefer Read on the task output file path'` (line 158).
- `validateInput` rejects missing or unknown task_id (lines 183-207, errorCode 1 / 2).
- `block: semanticBoolean(z.boolean().default(true))`, `timeout: z.number().min(0).max(600000).default(30000)`.

Behavior:
1. Look up task in `appState.tasks`. Throw if missing (line 215-218).
2. If `!block` and task is terminal (not running/pending): mark `notified: true`, return `{ retrieval_status: 'success', task: getTaskOutputData(task) }`. If still running/pending: return `{ retrieval_status: 'not_ready', task: getTaskOutputData(task) }`.
3. Else (blocking): emit a one-shot `onProgress({ toolUseID: \`task-output-waiting-${Date.now()}\`, data: { type:'waiting_for_task', taskDescription, taskType }})` (line 243-252), then `waitForTaskCompletion` polls every 100 ms up to `timeout` (default 30 000), respecting `abortController.signal` (throws `AbortError`).
4. On null/timeout return `'timeout'` retrieval status; else mark notified and return `'success'`.

`getTaskOutputData` (line 60-115):
- `local_bash`: prefer `bashTask.shellCommand?.taskOutput` (`stdout + '\n' + stderr`) over disk; sets `exitCode = bashTask.result?.code ?? null`.
- `local_agent`: `cleanResult = agentTask.result ? extractTextContent(agentTask.result.content, '\n') : undefined`; populates `prompt`, `result = cleanResult || output`, `output = cleanResult || output`, `error = agentTask.error`. Comment explains the disk file is the full session JSONL transcript (a symlink) while the in-memory `result` is just the final assistant text.
- `remote_agent`: `prompt = remoteTask.command`.

Output rendering (line 283-307) wraps fields in XML-ish tags `<retrieval_status>`, `<task_id>`, `<task_type>`, `<status>`, optional `<exit_code>`, `<output>` (run through `formatTaskOutput`), `<error>`. Joined by `\n\n`.

#### §3.6.1 Alias resolution sites (load-bearing — must stay in sync)

The TaskOutput / TaskStop renames from `AgentOutputTool` / `BashOutputTool` / `KillShell` are resolved at **three** distinct layers. All three must be updated together when an alias changes; drift silently breaks legacy permission rules, hooks, and persisted wire payloads.

1. **Tool-definition aliases** — `src/tools/TaskOutputTool/TaskOutputTool.tsx:150` (`aliases: ['AgentOutputTool', 'BashOutputTool']`) and `src/tools/TaskStopTool/TaskStopTool.ts:44` (`aliases: ['KillShell']`). Consumed by the registry's name-resolution pass (spec 08).
2. **Legacy permission-rule rewriter** — `src/utils/permissions/permissionRuleParser.ts:21-29`. Static map `LEGACY_TOOL_NAME_ALIASES` rewrites `Task → AGENT_TOOL_NAME`, `KillShell → TASK_STOP_TOOL_NAME`, `AgentOutputTool → TASK_OUTPUT_TOOL_NAME`, `BashOutputTool → TASK_OUTPUT_TOOL_NAME` (with optional `Brief` under `KAIROS`/`KAIROS_BRIEF`). Exposed via `normalizeLegacyToolName(name)` and `getLegacyToolNames(canonicalName)`. Consumers: settings.json permission rules and stored hook patterns. Comment: "When a tool is renamed, add old → new here so permission rules, hooks, and persisted wire names resolve to the canonical name." See spec 09 for permission-rule plumbing.
3. **Legacy parameter-name normalizer** — `src/utils/api.ts:661-677`. In the `case TASK_OUTPUT_TOOL_NAME` branch the wire input is rewritten: `task_id ?? agentId ?? bash_id` collapses to `task_id`, and `wait_up_to` (seconds, legacy AgentOutputTool/BashOutputTool) is multiplied by 1000 into `timeout` (ms); defaults `block:true`, `timeout:30000`. This is the on-the-wire shim that lets old tool_use blocks emitted under the AgentOutputTool/BashOutputTool names round-trip through the current TaskOutput input schema (§6.4).

Test invariant: `getLegacyToolNames(TASK_OUTPUT_TOOL_NAME)` MUST equal the tool's `aliases` array (modulo order). If the tool ever adds a new alias, both `LEGACY_TOOL_NAME_ALIASES` and the api.ts param normalizer need a paired update.

### §3.7 TodoWrite (`TodoWrite`)

`src/tools/TodoWriteTool/TodoWriteTool.ts:31-115`.

- `name: 'TodoWrite'`; `userFacingName() { return '' }`; `strict: true`; `shouldDefer: true`; `searchHint: 'manage the session task checklist'`.
- `isEnabled() { return !isTodoV2Enabled() }` — V1 only.
- `checkPermissions` always returns `{ behavior: 'allow', updatedInput: input }` (line 58-60).
- `toAutoClassifierInput(input) { return \`${input.todos.length} items\` }`.
- `maxResultSizeChars: 100_000`.

Behavior (line 65-103):
1. `todoKey = context.agentId ?? getSessionId()`.
2. `oldTodos = appState.todos[todoKey] ?? []`.
3. `allDone = todos.every(_ => _.status === 'completed')`.
4. `newTodos = allDone ? [] : todos` — list collapses to `[]` once every entry is completed.
5. `verificationNudgeNeeded` follows the same V1-side rule as TaskUpdate: gated on `feature('VERIFICATION_AGENT')`, `tengu_hive_evidence`, `!context.agentId`, `allDone`, `todos.length >= 3`, no item has subject matching `/verif/i`.
6. Persist `setAppState(prev => ({ ...prev, todos: { ...prev.todos, [todoKey]: newTodos } }))`. Note `newTodos` here matches the local variable assignment but the returned `data.newTodos` is the input `todos` (line 99).
7. Result content base string verbatim in §6.

---

## §4 Permissions

All Task tools rely on registry-level deferral (`shouldDefer: true`) plus `isConcurrencySafe: true`; see 09. None defines a per-call permission rule beyond:

- TodoWriteTool: explicit `checkPermissions` returning `{ behavior: 'allow', updatedInput: input }` (line 58-60). Comment: "No permission checks required for todo operations".
- TaskStopTool: pre-execution `validateInput` denies with errorCode 1/3 when task is missing or not running.
- TaskOutputTool: pre-execution `validateInput` denies with errorCode 1/2 when input is missing or task is unknown.

Disk side effects flow into `getClaudeConfigHomeDir()/tasks/<sanitizedTaskListId>/`; sanitization replaces `[^a-zA-Z0-9_-]` with `-` (line 217-219 of `src/utils/tasks.ts`). Lockfiles are kept per-task (`<taskPath>.lock`) and per-list (`<dir>/.lock`); proper-lockfile retries `30 × [5..100ms]` (line 102-108).

---

## §5 Algorithms (per tool)

### §5.1 ID minting (`createTask`)

`src/utils/tasks.ts:284-308`. Inside the per-list lockfile: `id = String(findHighestTaskId(taskListId) + 1)` where `findHighestTaskId` is the `max` of `findHighestTaskIdFromFiles` (parsed `<id>.json`) and the high-water mark file `.highwatermark` (lines 245-277). On reset/delete the high-water mark is bumped so IDs never recycle (lines 147-188 reset; 393-441 delete cascade).

### §5.2 Task list resolution (`getTaskListId`)

`src/utils/tasks.ts:199-210`, priority order:
1. `process.env.CLAUDE_CODE_TASK_LIST_ID` (explicit).
2. In-process teammate: `getTeammateContext().teamName`.
3. `getTeamName()` (process-based teammate or leader after TeamCreate).
4. Module-level `leaderTeamName` (set via `setLeaderTeamName`).
5. `getSessionId()`.

### §5.3 `isTodoV2Enabled`

`src/utils/tasks.ts:133-139`. True if `CLAUDE_CODE_ENABLE_TASKS` is truthy OR session is interactive (`!getIsNonInteractiveSession()`).

### §5.4 `claimTask` (called via `TaskUpdate.owner`/external claim path)

`src/utils/tasks.ts:541-612` (with-busy-check variant 618-…). Steps under per-task lock:
1. `getTask` pre-check returns `{ success:false, reason:'task_not_found' }` if missing.
2. Owner ≠ claimant → `'already_claimed'`.
3. `status === 'completed'` → `'already_resolved'`.
4. Compute `unresolvedTaskIds = ids of all tasks not 'completed'`; if any of `task.blockedBy` is unresolved → `'blocked'` with list.
5. Else `updateTaskUnsafe` with new owner; success.

### §5.5 Status legacy migration (ant-only)

`src/utils/tasks.ts:319-332`. On `getTask` when `process.env.USER_TYPE === 'ant'`:
- `'open' → 'pending'`
- `'resolved' → 'completed'`
- `{'planning','implementing','reviewing','verifying'} → 'in_progress'`

Then `TaskSchema().safeParse`. On parse failure: `logForDebugging` and return `null`.

### §5.6 `stopTask` algorithm (`src/tasks/stopTask.ts:38-100`)

Pseudocode:
```
task = appState.tasks[id]; if !task: throw StopTaskError('No task found', 'not_found')
if task.status !== 'running': throw StopTaskError(`Task ${id} is not running (status: ${task.status})`, 'not_running')
taskImpl = getTaskByType(task.type); if !taskImpl: throw StopTaskError('Unsupported task type: '+task.type, 'unsupported_type')
await taskImpl.kill(taskId, setAppState)
if isLocalShellTask(task):
    suppressed = false
    setAppState(prev => set tasks[id].notified=true ONCE; suppressed = !prevTask.notified)
    if suppressed: emitTaskTerminatedSdk(taskId, 'stopped', { toolUseId, summary: description })
command = isLocalShellTask(task) ? task.command : task.description
return { taskId, taskType, command }
```

The post-kill suppression for shell tasks is intentional: bash kills emit a noisy "exit code 137" notification that is silenced; agent kills retain their abort-payload notification.

**Single-shot SDK emit guarantee.** The `emitTaskTerminatedSdk(taskId, 'stopped', { toolUseId, summary })` call at `stopTask.ts:89-94` is gated on the `setAppState` reducer observing the previous `notified === false` and flipping it to `true` (the `suppressed` flag at lines 71-85 is set inside the reducer only when `prevTask.notified` was falsy). Re-issuing `TaskStop` against an already-notified shell task — for example a dual-stop race between an LLM-issued `TaskStop` and the SDK `stop_task` control request landing on the same task — is a **silent no-op for SDK consumers**: the second call still returns success and emits a tool result, but no `taskTerminatedSdk` event is re-published. Callers that need at-least-once semantics for the SDK terminate event must coordinate around this gate.

### §5.7 `getAllTasks` registry

`src/tasks.ts:22-32`. Inline list `[LocalShellTask, LocalAgentTask, RemoteAgentTask, DreamTask]` plus `LocalWorkflowTask` if `feature('WORKFLOW_SCRIPTS')` and `MonitorMcpTask` if `feature('MONITOR_TOOL')` (gated `require()`s at lines 9-14). `getTaskByType(type)` is a linear `find`.

Note: `InProcessTeammateTask` is **not imported by `src/tasks.ts` at all** — there is no static or feature-gated import of it in this module. Its kill path is invoked from teammate teardown directly via `InProcessTeammateTask.kill` at `src/tasks/InProcessTeammateTask/InProcessTeammateTask.tsx:25-27`, never through `stopTask`. Consequence: `TaskStop` (which dispatches via `getTaskByType(task.type)` at `stopTask.ts:57-63`) is **structurally unreachable** for any `'in_process_teammate'` task ID — `getTaskByType('in_process_teammate')` returns `undefined`, and `stopTask` throws `StopTaskError('Unsupported task type: in_process_teammate', 'unsupported_type')`. To stop a teammate task you must go through the teammate tear-down path (spec 14/30), not `TaskStop`.

### §5.8 Task ID minting at runtime

`src/Task.ts:79-106`. Prefix table:

| TaskType              | Prefix |
|-----------------------|--------|
| `local_bash`          | `b`    |
| `local_agent`         | `a`    |
| `remote_agent`        | `r`    |
| `in_process_teammate` | `t`    |
| `local_workflow`      | `w`    |
| `monitor_mcp`         | `m`    |
| `dream`               | `d`    |
| (default)             | `x`    |

Random suffix: 8 chars from `'0123456789abcdefghijklmnopqrstuvwxyz'` (line 96), each `bytes[i] % 36` from `randomBytes(8)`. Total ≈ `36^8` ≈ 2.8 × 10¹². `LocalMainSessionTask` uses prefix `'s'` instead (`src/tasks/LocalMainSessionTask.ts:73-82`).

### §5.9 TaskStateBase factory

`src/Task.ts:108-125`. `{ id, type, status:'pending', description, toolUseId, startTime: Date.now(), outputFile: getTaskOutputPath(id), outputOffset:0, notified:false }`.

### §5.10 `isTerminalTaskStatus`

`src/Task.ts:27-29`. True for `'completed' | 'failed' | 'killed'`.

### §5.11 `isBackgroundTask`

`src/tasks/types.ts:37-46`. Filter:
1. `task.status` must be `'running'` or `'pending'`.
2. If task has `'isBackgrounded' in task && isBackgrounded === false` → exclude (foregrounded teammates/main-session tasks).

### §5.12 Pill label composition

`src/tasks/pillLabel.ts:10-67`. Selected branches when all tasks share a type:
- `local_bash`: split into shells vs `kind==='monitor'` monitors; `'1 shell'` / `'N shells'` / `'1 monitor'` / `'N monitors'` joined by `, `.
- `in_process_teammate`: distinct count of `identity.teamName` → `'1 team'` / `'N teams'`.
- `local_agent`: `'1 local agent'` / `'N local agents'`.
- `remote_agent`: with single-task ultraplan, switch on `ultraplanPhase`: `'plan_ready' → '◆ ultraplan ready'`, `'needs_input' → '◇ ultraplan needs your input'`, default `'◇ ultraplan'`. Otherwise `'◇ 1 cloud session'` / `'◇ N cloud sessions'`. Diamonds are the constants `DIAMOND_FILLED` / `DIAMOND_OPEN` from `src/constants/figures.js`.
- `local_workflow`: `'1 background workflow'` / `'N background workflows'`.
- `monitor_mcp`: `'1 monitor'` / `'N monitors'`.
- `dream`: literal `'dreaming'`.
- Heterogeneous: `'N background task'` / `'N background tasks'`.

`pillNeedsCta` returns true only for a single `remote_agent` task with `isUltraplan === true && ultraplanPhase !== undefined`.

### §5.12.1 LocalAgentTask `autoBackgroundMs` (worker auto-background timeout)

`src/tasks/LocalAgentTask/LocalAgentTask.tsx:526-614` (`registerAgentForeground`). Optional `autoBackgroundMs?: number` parameter on the spawn input (line 532, 540). When `autoBackgroundMs !== undefined && autoBackgroundMs > 0` (line 582), `setTimeout` schedules a closure that:

1. Updates `AppState.tasks[agentId]` setting `isBackgrounded: true` if-and-only-if the task is still a `LocalAgentTask` and not already backgrounded (lines 585-600).
2. Resolves the per-agent `backgroundSignal` promise via `backgroundSignalResolvers.get(agentId)` and removes the resolver from the map (lines 601-605).

Returns `cancelAutoBackground = () => clearTimeout(timer)` (line 607) so the dispatcher can disarm the timer when the agent finishes / errors / is killed before the backgrounding deadline. This is the closest analogue to a "worker timeout" for `local_agent` tasks (no hard kill — the agent keeps running, just demoted off-screen). Default behavior with `autoBackgroundMs` unset: the agent runs in foreground until natural completion. See spec 14 (AgentTool spawns) for the call site that supplies this argument.

`RemoteAgentTask.tsx:686` has a hardcoded `REMOTE_REVIEW_TIMEOUT_MS` (≈30 minutes per the surrounding comment at lines 670-680: "the <remote-review> tag or the 30min timeout complete the task"). This is the **review-mode** completion deadline, not a generic worker timeout: when `task.isRemoteReview && Date.now() - task.pollStartedAt > REMOTE_REVIEW_TIMEOUT_MS`, the task transitions to `'completed'` (line 687). Non-review remote-agent tasks have no equivalent timeout in this module.

### §5.13 LocalMainSessionTask (Ctrl+B-twice backgrounding)

`src/tasks/LocalMainSessionTask.ts`. Full lifecycle:
- ID prefix `'s'`. Reuses `LocalAgentTaskState` shape with `agentType: 'main-session'`.
- `registerMainSessionTask` (line 94-162): initializes the disk symlink to per-task transcript via `initTaskOutputAsSymlink(taskId, getAgentTranscriptPath(asAgentId(taskId)))`; comment warns that using the main session transcript path would corrupt post-`/clear` conversation.
- `completeMainSessionTask` (line 168-219): atomic `notified` flip-and-set; foregrounded variant sets `notified:true` quietly and emits `emitTaskTerminatedSdk(taskId, 'completed' | 'failed', { toolUseId, summary:'Background session' })` directly; backgrounded variant calls `enqueueMainSessionNotification` which constructs the `<task_notification>...<status>...<summary>...` XML payload (line 254-263).
- `foregroundMainSessionTask` (line 270-302): swaps `foregroundedTaskId`, demotes the previously foregrounded `local_agent` task back to `isBackgrounded:true`.
- `startBackgroundSession` (line 338-479): wraps `query()` inside `runWithAgentContext({ agentId: taskId, agentType:'subagent', subagentName:'main-session', isBuiltIn:true })`; per-event it persists transcript via `recordSidechainTranscript([event], taskId, lastRecordedUuid)`; abort path emits `emitTaskTerminatedSdk(taskId,'stopped',{summary:description})` if not previously notified; on success/failure delegates to `completeMainSessionTask`. Recent-activity ring caps at `MAX_RECENT_ACTIVITIES = 5`.

### §5.14 Dream task (auto-memory)

`src/tasks/DreamTask/DreamTask.ts`. `MAX_TURNS = 30`; phase flips `'starting' → 'updating'` when first Edit/Write tool_use lands (no prompt parsing). `kill` aborts the controller, marks `'killed'`, and rolls back the consolidation lock mtime via `rollbackConsolidationLock(priorMtime)` (line 136-156). `completeDreamTask` and `failDreamTask` set `notified:true` immediately because there is no model-facing notification; the user-facing surface is an inline `appendSystemMessage`.

### §5.15 LocalShellTask kill helpers

`src/tasks/LocalShellTask/killShellTasks.ts:16-46` `killTask`: under `updateTaskState`, only acts if `status==='running' && isLocalShellTask(task)`. Calls `task.shellCommand?.kill()` then `cleanup()` in a try/catch, runs `unregisterCleanup`, clears `cleanupTimeoutId`, sets `status:'killed'`, `notified:true`, `shellCommand:null`, `endTime`. Outside the updater: `void evictTaskOutput(taskId)`.

`killShellTasksForAgent` (line 53-76): for each task in `AppState.tasks` matching `agentId === target && status==='running'`, calls `killTask`, then `dequeueAllMatching(cmd => cmd.agentId === agentId)` to drop queued notifications. Comment: "prevents 10-day fake-logs.sh zombies".

---

## §6 Verbatim assets

### §6.1 Constants

| Constant | Source | Value |
|---|---|---|
| `TASK_CREATE_TOOL_NAME` | `TaskCreateTool/constants.ts:1` | `'TaskCreate'` |
| `TASK_UPDATE_TOOL_NAME` | `TaskUpdateTool/constants.ts:1` | `'TaskUpdate'` |
| `TASK_GET_TOOL_NAME` | `TaskGetTool/constants.ts:1` | `'TaskGet'` |
| `TASK_LIST_TOOL_NAME` | `TaskListTool/constants.ts:1` | `'TaskList'` |
| `TASK_STOP_TOOL_NAME` | `TaskStopTool/prompt.ts:1` | `'TaskStop'` |
| `TASK_OUTPUT_TOOL_NAME` | `TaskOutputTool/constants.ts:1` | `'TaskOutput'` |
| `TODO_WRITE_TOOL_NAME` | `TodoWriteTool/constants.ts:1` | `'TodoWrite'` |
| `MAX_COMMAND_DISPLAY_LINES` | `TaskStopTool/UI.tsx:10` | `2` |
| `MAX_COMMAND_DISPLAY_CHARS` | `TaskStopTool/UI.tsx:11` | `160` |
| `MAX_RECENT_ACTIVITIES` (LocalAgentTask) | `LocalAgentTask.tsx:40` | `5` |
| `MAX_RECENT_ACTIVITIES` (LocalMainSessionTask) | `LocalMainSessionTask.ts:325` | `5` |
| `MAX_TURNS` (DreamTask) | `DreamTask.ts:13` | `30` |
| `TEAMMATE_MESSAGES_UI_CAP` | `InProcessTeammateTask/types.ts:101` | `50` |
| `BACKGROUND_BASH_SUMMARY_PREFIX` | `LocalShellTask.tsx:23` | `'Background command '` |
| `STALL_CHECK_INTERVAL_MS` / `STALL_THRESHOLD_MS` / `STALL_TAIL_BYTES` | `LocalShellTask.tsx:24-26` | `5_000` / `45_000` / `1024` |
| `LOCK_OPTIONS.retries` | `utils/tasks.ts:102-108` | `{ retries: 30, minTimeout: 5, maxTimeout: 100 }` |
| `HIGH_WATER_MARK_FILE` | `utils/tasks.ts:92` | `'.highwatermark'` |
| `TASK_ID_ALPHABET` | `Task.ts:96` | `'0123456789abcdefghijklmnopqrstuvwxyz'` |
| `TASK_STATUSES` | `utils/tasks.ts:69` | `['pending','in_progress','completed'] as const` |
| `REMOTE_TASK_TYPES` | `RemoteAgentTask.tsx:60` | `['remote-agent','ultraplan','ultrareview','autofix-pr','background-pr'] as const` |

### §6.2 Status enums

`TaskStatusSchema` (`src/utils/tasks.ts:71-73`):
```ts
export const TaskStatusSchema = lazySchema(() =>
  z.enum(['pending', 'in_progress', 'completed']),
)
```

`TaskStatus` (runtime task, `src/Task.ts:15-21`):
```ts
export type TaskStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'killed'
```

`TaskType` (`src/Task.ts:6-13`):
```ts
export type TaskType =
  | 'local_bash'
  | 'local_agent'
  | 'remote_agent'
  | 'in_process_teammate'
  | 'local_workflow'
  | 'monitor_mcp'
  | 'dream'
```

TodoWrite status enum (`src/utils/todo/types.ts:4-6`):
```ts
const TodoStatusSchema = lazySchema(() =>
  z.enum(['pending', 'in_progress', 'completed']),
)
```

### §6.3 Data shapes (verbatim)

**Task (V2, on-disk schema)** — `src/utils/tasks.ts:76-89`:
```ts
export const TaskSchema = lazySchema(() =>
  z.object({
    id: z.string(),
    subject: z.string(),
    description: z.string(),
    activeForm: z.string().optional(), // present continuous form for spinner (e.g., "Running tests")
    owner: z.string().optional(), // agent ID
    status: TaskStatusSchema(),
    blocks: z.array(z.string()), // task IDs this task blocks
    blockedBy: z.array(z.string()), // task IDs that block this task
    metadata: z.record(z.string(), z.unknown()).optional(), // arbitrary metadata
  }),
)
export type Task = z.infer<ReturnType<typeof TaskSchema>>
```

**TodoItem / TodoList** — `src/utils/todo/types.ts:8-18`:
```ts
export const TodoItemSchema = lazySchema(() =>
  z.object({
    content: z.string().min(1, 'Content cannot be empty'),
    status: TodoStatusSchema(),
    activeForm: z.string().min(1, 'Active form cannot be empty'),
  }),
)
export type TodoItem = z.infer<ReturnType<typeof TodoItemSchema>>

export const TodoListSchema = lazySchema(() => z.array(TodoItemSchema()))
export type TodoList = z.infer<ReturnType<typeof TodoListSchema>>
```

**TaskStateBase (runtime)** — `src/Task.ts:45-57`:
```ts
export type TaskStateBase = {
  id: string
  type: TaskType
  status: TaskStatus
  description: string
  toolUseId?: string
  startTime: number
  endTime?: number
  totalPausedMs?: number
  outputFile: string
  outputOffset: number
  notified: boolean
}
```

**Task interface (kill dispatch)** — `src/Task.ts:72-76`:
```ts
export type Task = {
  name: string
  type: TaskType
  kill(taskId: string, setAppState: SetAppState): Promise<void>
}
```

**LocalShellSpawnInput** — `src/Task.ts:59-67`:
```ts
export type LocalShellSpawnInput = {
  command: string
  description: string
  timeout?: number
  toolUseId?: string
  agentId?: AgentId
  /** UI display variant: description-as-label, dialog title, status bar pill. */
  kind?: 'bash' | 'monitor'
}
```

### §6.4 Tool input schemas (verbatim Zod)

**TaskCreate** (`TaskCreateTool.ts:18-33`):
```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    subject: z.string().describe('A brief title for the task'),
    description: z.string().describe('What needs to be done'),
    activeForm: z
      .string()
      .optional()
      .describe(
        'Present continuous form shown in spinner when in_progress (e.g., "Running tests")',
      ),
    metadata: z
      .record(z.string(), z.unknown())
      .optional()
      .describe('Arbitrary metadata to attach to the task'),
  }),
)
```

**TaskUpdate** (`TaskUpdateTool.ts:33-66`):
```ts
const inputSchema = lazySchema(() => {
  // Extended status schema that includes 'deleted' as a special action
  const TaskUpdateStatusSchema = TaskStatusSchema().or(z.literal('deleted'))

  return z.strictObject({
    taskId: z.string().describe('The ID of the task to update'),
    subject: z.string().optional().describe('New subject for the task'),
    description: z.string().optional().describe('New description for the task'),
    activeForm: z
      .string()
      .optional()
      .describe(
        'Present continuous form shown in spinner when in_progress (e.g., "Running tests")',
      ),
    status: TaskUpdateStatusSchema.optional().describe(
      'New status for the task',
    ),
    addBlocks: z
      .array(z.string())
      .optional()
      .describe('Task IDs that this task blocks'),
    addBlockedBy: z
      .array(z.string())
      .optional()
      .describe('Task IDs that block this task'),
    owner: z.string().optional().describe('New owner for the task'),
    metadata: z
      .record(z.string(), z.unknown())
      .optional()
      .describe(
        'Metadata keys to merge into the task. Set a key to null to delete it.',
      ),
  })
})
```

**TaskGet** (`TaskGetTool.ts:13-17`):
```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    taskId: z.string().describe('The ID of the task to retrieve'),
  }),
)
```

**TaskList** (`TaskListTool.ts:13`): `z.strictObject({})`.

**TaskStop** (`TaskStopTool.ts:10-19`):
```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    task_id: z
      .string()
      .optional()
      .describe('The ID of the background task to stop'),
    // shell_id is accepted for backward compatibility with the deprecated KillShell tool
    shell_id: z.string().optional().describe('Deprecated: use task_id instead'),
  }),
)
```

**TaskOutput** (`TaskOutputTool.tsx:30-34`):
```ts
const inputSchema = lazySchema(() => z.strictObject({
  task_id: z.string().describe('The task ID to get output from'),
  block: semanticBoolean(z.boolean().default(true)).describe('Whether to wait for completion'),
  timeout: z.number().min(0).max(600000).default(30000).describe('Max wait time in ms')
}));
```

**TodoWrite** (`TodoWriteTool.ts:13-17`):
```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    todos: TodoListSchema().describe('The updated todo list'),
  }),
)
```

### §6.5 Tool output schemas

`TaskCreate` output: `{ task: { id: string, subject: string } }`.

`TaskUpdate` (lines 69-83):
```ts
z.object({
  success: z.boolean(),
  taskId: z.string(),
  updatedFields: z.array(z.string()),
  error: z.string().optional(),
  statusChange: z.object({ from: z.string(), to: z.string() }).optional(),
  verificationNudgeNeeded: z.boolean().optional(),
})
```

`TaskGet` (lines 20-33): nullable task `{ id, subject, description, status, blocks, blockedBy }`.

`TaskList` (lines 16-28): `{ tasks: array of { id, subject, status, owner?, blockedBy } }`.

`TaskStop` (lines 22-34):
```ts
z.object({
  message: z.string().describe('Status message about the operation'),
  task_id: z.string().describe('The ID of the task that was stopped'),
  task_type: z.string().describe('The type of the task that was stopped'),
  command: z.string().optional().describe('The command or description of the stopped task'),
})
```

`TaskOutput` (lines 51-54): `{ retrieval_status: 'success' | 'timeout' | 'not_ready', task: TaskOutput | null }` with `TaskOutput = { task_id, task_type:TaskType, status, description, output, exitCode?, error?, prompt?, result? }`.

`TodoWrite` (lines 20-26): `{ oldTodos: TodoList, newTodos: TodoList, verificationNudgeNeeded?: boolean }`.

### §6.6 Tool prompt strings (verbatim)

**TaskCreate `DESCRIPTION`** (`prompt.ts:3`): `'Create a new task in the task list'`.

**TaskCreate `getPrompt()`** template — full conditional text reproduced verbatim from `prompt.ts:5-56`. The `${teammateContext}` insertion is `' and potentially assigned to teammates'` when `isAgentSwarmsEnabled()`, otherwise empty. The `${teammateTips}` block when swarms-enabled is:
```
- Include enough detail in the description for another agent to understand and complete the task
- New tasks are created with status 'pending' and no owner - use TaskUpdate with the `owner` parameter to assign them
```
Body (verbatim, with placeholders shown):
```
Use this tool to create a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.
It also helps the user understand the progress of the task and overall progress of their requests.

## When to Use This Tool

Use this tool proactively in these scenarios:

- Complex multi-step tasks - When a task requires 3 or more distinct steps or actions
- Non-trivial and complex tasks - Tasks that require careful planning or multiple operations${teammateContext}
- Plan mode - When using plan mode, create a task list to track the work
- User explicitly requests todo list - When the user directly asks you to use the todo list
- User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
- After receiving new instructions - Immediately capture user requirements as tasks
- When you start working on a task - Mark it as in_progress BEFORE beginning work
- After completing a task - Mark it as completed and add any new follow-up tasks discovered during implementation

## When NOT to Use This Tool

Skip using this tool when:
- There is only a single, straightforward task
- The task is trivial and tracking it provides no organizational benefit
- The task can be completed in less than 3 trivial steps
- The task is purely conversational or informational

NOTE that you should not use this tool if there is only one trivial task to do. In this case you are better off just doing the task directly.

## Task Fields

- **subject**: A brief, actionable title in imperative form (e.g., "Fix authentication bug in login flow")
- **description**: What needs to be done
- **activeForm** (optional): Present continuous form shown in the spinner when the task is in_progress (e.g., "Fixing authentication bug"). If omitted, the spinner shows the subject instead.

All tasks are created with status `pending`.

## Tips

- Create tasks with clear, specific subjects that describe the outcome
- After creating tasks, use TaskUpdate to set up dependencies (blocks/blockedBy) if needed
${teammateTips}- Check TaskList first to avoid creating duplicate tasks
```

**TaskUpdate `DESCRIPTION`**: `'Update a task in the task list'`. **`PROMPT`** (`prompt.ts:3-77`) verbatim:
```
Use this tool to update a task in the task list.

## When to Use This Tool

**Mark tasks as resolved:**
- When you have completed the work described in a task
- When a task is no longer needed or has been superseded
- IMPORTANT: Always mark your assigned tasks as resolved when you finish them
- After resolving, call TaskList to find your next task

- ONLY mark a task as completed when you have FULLY accomplished it
- If you encounter errors, blockers, or cannot finish, keep the task as in_progress
- When blocked, create a new task describing what needs to be resolved
- Never mark a task as completed if:
  - Tests are failing
  - Implementation is partial
  - You encountered unresolved errors
  - You couldn't find necessary files or dependencies

**Delete tasks:**
- When a task is no longer relevant or was created in error
- Setting status to `deleted` permanently removes the task

**Update task details:**
- When requirements change or become clearer
- When establishing dependencies between tasks

## Fields You Can Update

- **status**: The task status (see Status Workflow below)
- **subject**: Change the task title (imperative form, e.g., "Run tests")
- **description**: Change the task description
- **activeForm**: Present continuous form shown in spinner when in_progress (e.g., "Running tests")
- **owner**: Change the task owner (agent name)
- **metadata**: Merge metadata keys into the task (set a key to null to delete it)
- **addBlocks**: Mark tasks that cannot start until this one completes
- **addBlockedBy**: Mark tasks that must complete before this one can start

## Status Workflow

Status progresses: `pending` → `in_progress` → `completed`

Use `deleted` to permanently remove a task.

## Staleness

Make sure to read a task's latest state using `TaskGet` before updating it.

## Examples

Mark task as in progress when starting work:
```json
{"taskId": "1", "status": "in_progress"}
```

Mark task as completed after finishing work:
```json
{"taskId": "1", "status": "completed"}
```

Delete a task:
```json
{"taskId": "1", "status": "deleted"}
```

Claim a task by setting owner:
```json
{"taskId": "1", "owner": "my-name"}
```

Set up task dependencies:
```json
{"taskId": "2", "addBlockedBy": ["1"]}
```
```

**TaskGet `DESCRIPTION`**: `'Get a task by ID from the task list'`. **`PROMPT`** verbatim from `prompt.ts:3-24`:
```
Use this tool to retrieve a task by its ID from the task list.

## When to Use This Tool

- When you need the full description and context before starting work on a task
- To understand task dependencies (what it blocks, what blocks it)
- After being assigned a task, to get complete requirements

## Output

Returns full task details:
- **subject**: Task title
- **description**: Detailed requirements and context
- **status**: 'pending', 'in_progress', or 'completed'
- **blocks**: Tasks waiting on this one to complete
- **blockedBy**: Tasks that must complete before this one can start

## Tips

- After fetching a task, verify its blockedBy list is empty before beginning work.
- Use TaskList to see all tasks in summary form.
```

**TaskList `DESCRIPTION`**: `'List all tasks in the task list'`. `getPrompt()` swarm/non-swarm variations verbatim from `prompt.ts:5-49`. The non-swarm body is the template with empty `${teammateUseCase}` and empty `${teammateWorkflow}`:
```
Use this tool to list all tasks in the task list.

## When to Use This Tool

- To see what tasks are available to work on (status: 'pending', no owner, not blocked)
- To check overall progress on the project
- To find tasks that are blocked and need dependencies resolved
${teammateUseCase}- After completing a task, to check for newly unblocked work or claim the next available task
- **Prefer working on tasks in ID order** (lowest ID first) when multiple tasks are available, as earlier tasks often set up context for later ones

## Output

Returns a summary of each task:
${idDescription}
- **subject**: Brief description of the task
- **status**: 'pending', 'in_progress', or 'completed'
- **owner**: Agent ID if assigned, empty if available
- **blockedBy**: List of open task IDs that must be resolved first (tasks with blockedBy cannot be claimed until dependencies resolve)

Use TaskGet with a specific task ID to view full details including description and comments.
${teammateWorkflow}
```
With swarms enabled, `${teammateUseCase}` is `- Before assigning tasks to teammates, to see what's available\n` and `${teammateWorkflow}` is the literal block from `prompt.ts:16-25` (Teammate Workflow section quoted in §3 above; both `idDescription` branches resolve to `- **id**: Task identifier (use with TaskGet, TaskUpdate)`).

**TaskStop `DESCRIPTION`** (`prompt.ts:3-9`):
```
- Stops a running background task by its ID
- Takes a task_id parameter identifying the task to stop
- Returns a success or failure status
- Use this tool when you need to terminate a long-running task
```
TaskStop `description()` (line 92-94 of TaskStopTool.ts) returns the literal `'Stop a running background task by ID'`.

**TaskOutput `prompt()`** (TaskOutputTool.tsx:172-181):
```
DEPRECATED: Prefer using the Read tool on the task's output file path instead. Background tasks return their output file path in the tool result, and you receive a <task-notification> with the same path when the task completes — Read that file directly.

- Retrieves output from a running or completed task (background shell, agent, or remote session)
- Takes a task_id parameter identifying the task
- Returns the task output along with status information
- Use block=true (default) to wait for task completion
- Use block=false for non-blocking check of current status
- Task IDs can be found using the /tasks command
- Works with all task types: background shells, async agents, and remote sessions
```
TaskOutput `description()` returns `'[Deprecated] — prefer Read on the task output file path'`.

**TodoWrite `PROMPT`** (TodoWriteTool/prompt.ts:3-181) — large verbatim block including 7 examples, "Task States and Management" with three states `pending` / `in_progress` / `completed`, IMPORTANT instruction "Exactly ONE task must be in_progress at any time (not less, not more)", and four "Task Completion Requirements" bullets. The single embedded interpolation `${FILE_EDIT_TOOL_NAME}` resolves at module load via `import { FILE_EDIT_TOOL_NAME } from '../FileEditTool/constants.js'`. **`DESCRIPTION`** (line 183-184):
```
Update the todo list for the current session. To be used proactively and often to track progress and pending tasks. Make sure that at least one task is in_progress at all times. Always provide both content (imperative) and activeForm (present continuous) for each task.
```

### §6.7 User-facing strings (result content templates)

| Tool | Template / literal | Source |
|---|---|---|
| TaskCreate result | `` `Task #${task.id} created successfully: ${task.subject}` `` | TaskCreateTool.ts:135 |
| TaskUpdate result (success) | `` `Updated task #${taskId} ${updatedFields.join(', ')}` `` | TaskUpdateTool.ts:384 |
| TaskUpdate result (failure) | `` error || `Task #${taskId} not found` `` | TaskUpdateTool.ts:380 |
| TaskUpdate teammate-completed suffix | `\n\nTask completed. Call TaskList now to find your next available task or see if your work unblocked others.` | TaskUpdateTool.ts:392-393 |
| TaskUpdate verification nudge | `` `\n\nNOTE: You just closed out 3+ tasks and none of them was a verification step. Before writing your final summary, spawn the verification agent (subagent_type="${VERIFICATION_AGENT_TYPE}"). You cannot self-assign PARTIAL by listing caveats in your summary — only the verifier issues a verdict.` `` | TaskUpdateTool.ts:397 |
| TaskGet result (not found) | `'Task not found'` | TaskGetTool.ts:104 |
| TaskGet result (found) | `\`Task #${id}: ${subject}\nStatus: ${status}\nDescription: ${description}\`` plus optional `\nBlocked by: #a, #b` and `\nBlocks: #c, #d` | TaskGetTool.ts:109-126 |
| TaskList row | `` `#${id} [${status}] ${subject}${owner ? ` (${owner})` : ''}${blockedBy.length ? ` [blocked by ${ids.join(', ')}]` : ''}` `` | TaskListTool.ts:101-107 |
| TaskList empty | `'No tasks found'` | TaskListTool.ts:97 |
| TaskStop result message | `` `Successfully stopped task: ${result.taskId} (${result.command})` `` | TaskStopTool.ts:124 |
| TaskStop UI suffix (truncated) | `'… · stopped'` else `' · stopped'` | UI.tsx:33 |
| TodoWrite result base | `Todos have been modified successfully. Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable` | TodoWriteTool.ts:105 |
| TodoWrite verification nudge | identical to TaskUpdate's, with single-quoted `—` em-dash | TodoWriteTool.ts:107 |
| MainSession notification summary | `` `Background session "${description}" completed` `` / `failed` | LocalMainSessionTask.ts:248-249 |

### §6.8 ANT-only / feature-gated assets

- `TaskStopTool.userFacingName: () => (process.env.USER_TYPE === 'ant' ? '' : 'Stop Task')` — `TaskStopTool.ts:46`. ANT users see no display name (the tool collapses out of `/tools` listings that key on `userFacingName`).
- `TodoWriteTool.userFacingName() { return '' }` always (`TodoWriteTool.ts:48-50`) — non-ANT distinction is the gate `isEnabled = !isTodoV2Enabled()`.
- Status-name migration on read for ANT only — `src/utils/tasks.ts:319-332`.
- `feature('VERIFICATION_AGENT')` gate at `TaskUpdateTool.ts:335` and `TodoWriteTool.ts:78`. Plus the GrowthBook flag `getFeatureValue_CACHED_MAY_BE_STALE('tengu_hive_evidence', false)` evaluated alongside.
- `feature('WORKFLOW_SCRIPTS')` and `feature('MONITOR_TOOL')` gate `LocalWorkflowTask` / `MonitorMcpTask` registration in `src/tasks.ts:9-14`.
- The `"external" === 'ant'` literal in `TaskStopTool/UI.tsx:28` and `TaskOutputTool.tsx:163-165` evaluates to `false` in the leaked build — both branches stay live. **Resolved in §12.1**: this is a Bun `--define`-folded `process.env.USER_TYPE === 'ant'` check (the bundler replaced `process.env.USER_TYPE` with the literal `"external"` for the external build target, then string-comparison against `'ant'` constant-folds to `false`). The folded form is dead code in this build but the source intent is the standard ANT-only gate.

---

## §7 Cross-references

- Registry plumbing for inclusion at `src/tools.ts:218-219` and the unconditional inclusions of `TaskOutputTool` (line 196), `TodoWriteTool` (208), `TaskStopTool` (210). Coordinator-mode injection of `TaskStopTool` into `simpleTools`/`replSimple` lives at lines 283 and 295. See spec 08 §3.
- Permission flow (no per-tool predicates apart from `validateInput` short-circuit and TodoWrite's blanket allow): see 09.
- Multi-agent dispatch (the `team-name` lookup in `getTaskListId`, the `writeToMailbox` notification for owner change, and `claimTask`'s busy-check variant) is owned by 14 / 30. Specifically `agentSwarmsEnabled()` callers and `VERIFICATION_AGENT_TYPE` import resolve to spec 14.
- `AppState.tasks` shape, persistence, eviction, and `--resume` replay live in 41. The `notified` field flip is the eviction guard.

---

## §8 Test surface (referenced, not enumerated)

The leaked tree omits tests (no `*.test.*` peer files for these modules). Behavior-of-record is the source above. The verification-nudge logic specifically references "BQ analysis (round 9, 2026-03-20)" in the InProcessTeammateTask comment block (line 96-99 of `types.ts`).

---

## §9 Failure modes / error codes

- TaskUpdate "Task not found" returns `{ success:false }` and is rendered as a non-error tool_result so sibling tools in the same StreamingToolExecutor batch are NOT cancelled (TaskUpdateTool.ts:373-381).
- TaskStop pre-call: `errorCode 1` for missing input or unknown task, `errorCode 3` for non-running task.
- `stopTask` post-call: `StopTaskError.code ∈ { 'not_found' | 'not_running' | 'unsupported_type' }` (stopTask.ts:10-18). `'unsupported_type'` is reachable for `'in_process_teammate'` because `getAllTasks()` does not include it.
- TaskOutput pre-call: `errorCode 1` (missing) / `errorCode 2` (not found). On blocking-mode timeout returns `{ retrieval_status: 'timeout' }`.
- TodoWrite never fails on input validation past Zod `strict: true` because `checkPermissions` always allows.
- `getTask` returns `null` (not throw) when the on-disk file fails Zod parse (utils/tasks.ts:333-340) — silently dropping malformed tasks.
- `claimTask` reasons: `'task_not_found' | 'already_claimed' | 'already_resolved' | 'blocked' | 'agent_busy'`.

---

## §10 Performance / sizing constants

- `maxResultSizeChars: 100_000` for every Task* tool and TodoWrite.
- `LOCK_OPTIONS.retries.retries = 30` with backoff 5–100 ms gives ~2.6 s worst-case wait for a 10-way swarm race (comment at utils/tasks.ts:99-101).
- TaskOutput poll interval `100 ms`; default timeout `30 000 ms`; max `600 000 ms`.
- Stall watchdog (LocalShellTask): tick `5_000` ms, threshold `45_000` ms, tail `1024` bytes.
- LocalAgentTask `autoBackgroundMs` (`LocalAgentTask.tsx:540`): optional, caller-supplied per-spawn timer. When unset there is no foreground-deadline; when set, the agent flips `isBackgrounded:true` after that many ms (no kill). See §5.12.1.
- RemoteAgentTask `REMOTE_REVIEW_TIMEOUT_MS` (`RemoteAgentTask.tsx:686`): hardcoded ≈30-minute review-mode completion deadline; only applies when `task.isRemoteReview === true`.

---

## §11 ID space caveats

- Runtime task IDs (`AppState.tasks` keys) are alphanumeric with a 1-char prefix + 8-char random suffix (`Task.ts:98-106`). LocalMainSessionTask uses prefix `'s'` outside the table in `Task.ts`.
- V2 task list IDs (`utils/tasks.ts`) are pure decimal strings starting at `'1'`. The deletion cascade in `deleteTask` removes `taskId` from every other task's `blocks` and `blockedBy` (line 421-434). The high-water mark prevents reuse across deletes/resets.
- `sanitizePathComponent` only allows `[A-Za-z0-9_-]` for filesystem paths (utils/tasks.ts:217-219); team/session names with other chars become hyphens.

---

## §12 Open questions / uncertainty

### §12.1 Resolved: `"external" === 'ant'` literal is a Bun `--define`-folded `process.env.USER_TYPE` access

The literal `"external" === 'ant'` short-circuit at `TaskStopTool/UI.tsx:28` and `TaskOutputTool.tsx:163-165` is provably a Bun `--define`-substituted `process.env.USER_TYPE === 'ant'`. The bundler replaces `process.env.USER_TYPE` with the literal string `"external"` for external builds (and presumably `"ant"` for ANT-internal builds) at compile time; the surrounding `=== 'ant'` then constant-folds to `false`, leaving the literal `"external" === 'ant'` token in the emitted source.

Evidence: the symmetric un-folded form `process.env.USER_TYPE === 'ant'` appears at `TaskStopTool.ts:46` (the `userFacingName` getter) — same source pattern, different file, not folded because it sits inside a getter closure that the bundler did not inline. The `--define process.env.USER_TYPE="external"` build flag is the simplest explanation that fits both shapes.

Consequence for the leaked build: every `"external" === 'ant'` branch is dead, every `"external" !== 'ant'` branch is live. `TaskOutputTool.isEnabled()` always returns `true`; `TaskStopTool/UI.tsx`'s ANT-suppress branch never fires.

### §12.2 Other open questions

1. `AppState.tasks` invariants for `'in_process_teammate'`: confirmed `stopTask` cannot kill it (no entry in `getAllTasks()`, no import in `src/tasks.ts` — see §5.7). The tool surface implies `TaskStop` accepts any `task_id`, but the dispatch path is structurally unreachable. Whether the SDK route reaches a different kill path is owned by 14/30.

2. `TodoWriteTool` returns `data.newTodos: todos` (input list) while persisting `newTodos` (possibly `[]` after auto-collapse) into `AppState.todos[todoKey]`. The asymmetry is intentional per the surrounding comment at TodoWriteTool.ts:69-70 ("the list collapses to `[]`"), but the externally visible `oldTodos`/`newTodos` reporting hides the collapse — confirm with 41 if any consumer relies on the persisted `[]` value.

3. The `validateInput.errorCode` numbering for TaskOutputTool (1, 2) and TaskStopTool (1, 3) is not aligned with any global enum I located in this scope; whether codes are stable for SDK consumers belongs to 22.

4. The TaskList "comments" mentioned in `TaskGetTool/prompt.ts:23` ("view full details including description and comments") has no schema field — comments may be a stale prompt artifact or live elsewhere in `metadata`.

5. The two `MAX_RECENT_ACTIVITIES = 5` constants (LocalAgentTask + LocalMainSessionTask) are duplicated rather than imported; a future restructure may unify them. No behavioral consequence.
