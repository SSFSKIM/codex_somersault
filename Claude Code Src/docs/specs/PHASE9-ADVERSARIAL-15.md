# Phase 9.5b adversarial review — spec 15 (tool-tasks)

Reviewer role: skeptic. Source: `src/Task.ts`, `src/tasks.ts`, `src/tasks/`, `src/tools/{TaskCreate,TaskUpdate,TaskGet,TaskList,TaskOutput,TaskStop,TodoWrite}Tool/`, `src/tools.ts`, `src/utils/permissions/permissionRuleParser.ts`, `src/utils/api.ts`.

## Severity counts

- CRITICAL: 0
- HIGH: 1
- MEDIUM: 3
- LOW: 3
- NIT: 2

Total: 9 findings.

## Top 5 findings

### 1. [HIGH] BashOutputTool/AgentOutputTool aliasing — alias-list correct, but spec misses the *permission-rule* normalizer and the *legacy parameter normalizer*

Spec §3.6 documents `aliases: ['AgentOutputTool', 'BashOutputTool']` at `TaskOutputTool.tsx:150` (verified). However the spec does NOT mention two other places that participate in the alias resolution and could drift:

- `src/utils/permissions/permissionRuleParser.ts:24-25` — a static map `{ AgentOutputTool: TASK_OUTPUT_TOOL_NAME, BashOutputTool: TASK_OUTPUT_TOOL_NAME }` rewrites legacy permission rules. If this map ever falls out of sync with the alias list, settings.json rules from older sessions silently stop matching.
- `src/utils/api.ts:662` — comment "Normalize legacy parameter names from AgentOutputTool/BashOutputTool". Implies the historical input shape (likely `bash_id` or similar) is being remapped to `task_id` on the wire. Spec §6.4 only documents the current `task_id`/`shell_id` schema and does not cite this normalizer.

Compared to the Phase 9.6 spec 08 finding (path drift), spec 15 here has *no path drift* at the canonical alias declaration, but it under-covers the alias-resolution surface area. Recommend adding a §3.6.1 "Alias resolution sites" subsection enumerating: tool `aliases`, permissionRuleParser map, and api.ts parameter normalizer.

### 2. [MEDIUM] LocalAgentTask `autoBackgroundMs` worker-timeout is undocumented (echoes Phase 9.6 spec 30 finding)

Phase 9.6 spec 30 found "worker timeout absent, only maxTurns". Spec 15 §5.13/§5.14 documents `MAX_TURNS = 30` for DreamTask and `MAX_RECENT_ACTIVITIES = 5` for LocalAgentTask, but **omits `autoBackgroundMs`** at `src/tasks/LocalAgentTask/LocalAgentTask.tsx:532, 540, 582-606`. This is a real per-task timeout (auto-background after N ms), parameterized at spawn. It's the closest thing to a worker timeout for `local_agent` tasks and belongs in §5 / §10. Same defect class as Phase 9.6/30.

### 3. [MEDIUM] §5.7 claim that `'in_process_teammate'` produces `StopTaskError('unsupported_type')` is correct but missing in `getAllTasks`'s import list

Verified: `src/tasks.ts:22-32` lists only `[LocalShellTask, LocalAgentTask, RemoteAgentTask, DreamTask]` plus optional `LocalWorkflowTask`/`MonitorMcpTask`. `InProcessTeammateTask` is *not imported at all* in `tasks.ts` — spec §5.7 says it's "not in `getAllTasks()`" but implies it exists somewhere in the registry namespace. The teardown path is `InProcessTeammateTask/InProcessTeammateTask.tsx` `kill` directly. Minor wording fix: state explicitly that `InProcessTeammateTask` is not even imported by `src/tasks.ts`, so `TaskStop` against any teammate task ID is structurally unreachable for kill — only the teammate tear-down path can stop it.

### 4. [MEDIUM] §5.6 `stopTask` post-kill suppression: the SDK emit happens *only when the bash task was previously un-notified*

Spec §5.6 reads "post-kill suppression for shell tasks is intentional" — accurate. But the actual logic at `stopTask.ts:70-95` only fires `emitTaskTerminatedSdk` when `suppressed === true` (i.e., the previous `notified` was false). If a shell task was already marked notified (e.g., dual-stop race), the SDK event is *not re-emitted*, and the spec doesn't make this single-shot guarantee explicit. Recommend §5.6 add: "the `emitTaskTerminatedSdk('stopped', …)` call is gated on the previous `notified` being false — re-issuing TaskStop on an already-notified shell task is a silent no-op for SDK consumers."

### 5. [LOW] `TaskOutputTool.isEnabled() { return "external" !== 'ant' }` — spec §6.8 calls this a "build-time replacement that did not fire"

Verified at `TaskOutputTool.tsx:163-165`. Spec §12 open question 1 acknowledges the literal but defers. The same pattern exists at `TaskStopTool/UI.tsx`. Stronger claim available: this is almost certainly a `process.env.USER_TYPE` access that the bundler partially replaced (Bun's `--define` flag for `process.env.USER_TYPE`), then the `=== 'ant'` got constant-folded against the literal `'external'`. The branch is dead in the leaked build. Spec should promote this from "open question" to a confirmed bundler artifact, since the symmetric ANT-only gate `process.env.USER_TYPE === 'ant'` at `TaskStopTool.ts:46` (un-folded) shows the same source pattern in a different file.

## Verdict

**ACCEPT WITH AMENDMENTS.** Spec 15 is unusually thorough (verbatim Zod, line-cited tool surface, full output-rendering tags). Core claims about lifecycle, TodoV1 vs V2 gating (`isTodoV2Enabled`), `aliases: ['AgentOutputTool', 'BashOutputTool']`, `stopTask` algorithm, and ID-prefix table are all correct. Required amendments are additive (timeout enumeration, alias-resolution sites, single-shot SDK emit guarantee), not corrective. No CRITICAL findings.

## Cross-spec impact

- **Spec 08 (registry)**: spec 15 correctly cites `src/tools.ts:218-219` for V2 gated registration and lines 196/208/210 for unconditional inclusions, plus 283/295 coordinator injections. Verified. **No drift versus the Phase 9.6 spec 08 finding** — spec 15's alias claim is at a different layer (the tool definition itself, not the canonical-name registry) and is correct.
- **Spec 09 (permissions)**: spec 15 correctly defers per-tool predicates. The `permissionRuleParser.ts` normalizer mentioned in finding #1 is owned by 09 — flag for spec 09 to cite the AgentOutputTool/BashOutputTool legacy-rule rewriting.
- **Spec 14 (AgentTool spawns tasks)**: AgentTool spawns LocalAgentTask, which carries `autoBackgroundMs` (finding #2). Spec 14 should cite this if it claims to enumerate worker-task lifecycle.
- **Spec 30 (coordinator multi-agent)**: confirmed `simpleTools.push(AgentTool, TaskStopTool, getSendMessageTool())` at `tools.ts:295` and `replSimple.push(TaskStopTool, getSendMessageTool())` at line 283. Spec 15 §7 cross-reference is accurate. Phase 9.6/30 worker-timeout finding extends here: `autoBackgroundMs` is the answer for `local_agent`, but `RemoteAgentTask.tsx:676` has a hardcoded 30-min review timeout (`grep` line 676) — spec 30 ripple should land in spec 15 §10.
- **Spec 41 (AppState)**: spec 15 defers `AppState.tasks` shape correctly. Open question 3 (TodoWrite reporting `data.newTodos: todos` while persisting `[]` after collapse) survives — needs spec 41 to confirm replay invariants.

## Hardest-to-verify claim

The spec's claim in §5.5 that the ANT-only legacy status migration (`'open' → 'pending'`, `{'planning','implementing','reviewing','verifying'} → 'in_progress'`) at `src/utils/tasks.ts:319-332` runs on `getTask` *only when `process.env.USER_TYPE === 'ant'`* — without reading `utils/tasks.ts:319-332` directly (one of the unread files in this 20-read budget) I cannot independently verify the gate condition or that the listed source statuses are exhaustive. The spec's surrounding text (`then TaskSchema().safeParse`, `logForDebugging` on parse failure) gives high circumstantial confidence, but the ANT-gate strictness vs. universal-application is exactly the kind of subtle drift that bites consumers when external users inherit ant-shaped JSON from a borrowed task list.

Confidence: medium-high overall. Verified directly: `Task.ts`, `tasks.ts`, `stopTask.ts`, `TaskOutputTool.tsx` lines 1-360, `TaskStopTool.ts`, `TaskCreateTool.ts`, `TaskUpdateTool.ts`, plus targeted greps for aliases, registry placement, in_process_teammate references, and timeout/autoBackground constants.
