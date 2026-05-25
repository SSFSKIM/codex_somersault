# Phase 9.6c — fix log for spec 15 (tool-tasks)

Source: `docs/specs/PHASE9-ADVERSARIAL-15.md` (9 findings, 0 CRITICAL / 1 HIGH / 3 MED / 3 LOW / 2 NIT). Verdict: ACCEPT WITH AMENDMENTS — additive only, no claim corrections.

This pass lands the HIGH, the three MEDs, and the LOW that the adversarial review elevated to a confirmed bundler artifact. NITs deferred (no behavioral impact).

## Verifications performed before edit

| Claim | Source | Verified |
|---|---|---|
| `LEGACY_TOOL_NAME_ALIASES` map maps `AgentOutputTool`/`BashOutputTool` → `TASK_OUTPUT_TOOL_NAME` and `KillShell` → `TASK_STOP_TOOL_NAME` | `src/utils/permissions/permissionRuleParser.ts:21-29` | Yes — also includes `Task → AGENT_TOOL_NAME` and optional `Brief` under `KAIROS`/`KAIROS_BRIEF` |
| `api.ts` legacy parameter normalizer for TaskOutput rewrites `agentId`/`bash_id`/`wait_up_to` | `src/utils/api.ts:661-677` | Yes — multiplies `wait_up_to` (s) by 1000 → `timeout` (ms); defaults `block:true`, `timeout:30000` |
| `LocalAgentTask.autoBackgroundMs` flips `isBackgrounded:true` and resolves `backgroundSignal` after N ms | `src/tasks/LocalAgentTask/LocalAgentTask.tsx:526-614` | Yes — caller-supplied; returns `cancelAutoBackground` to disarm |
| `RemoteAgentTask` 30-min review timeout | `src/tasks/RemoteAgentTask/RemoteAgentTask.tsx:686` | Yes — `REMOTE_REVIEW_TIMEOUT_MS`, gated on `task.isRemoteReview` |
| `InProcessTeammateTask` not imported by `src/tasks.ts` | `src/tasks.ts:1-32` | Yes — only `LocalShellTask`/`LocalAgentTask`/`RemoteAgentTask`/`DreamTask` (+ feature-gated `LocalWorkflowTask`/`MonitorMcpTask`) |
| `stopTask` `emitTaskTerminatedSdk` gated on `prevTask.notified === false` | `src/tasks/stopTask.ts:70-95` | Yes — `suppressed` set only inside the reducer when `prevTask.notified` was falsy; `emit` only when `suppressed === true` |
| `process.env.USER_TYPE === 'ant'` un-folded form in `TaskStopTool.ts:46` proves the `--define` substitution hypothesis for the folded form | `src/tools/TaskStopTool/TaskStopTool.ts:46` (cited in spec §6.8) | Yes — same source pattern, different file, un-folded inside getter closure |

## Edits to `docs/specs/15-tool-tasks.md`

1. **§3.6 (HIGH)** — added xref pointer from the `aliases:` line to new §3.6.1.
2. **§3.6.1 (HIGH, new)** — enumerates the three alias-resolution sites: tool `aliases`, `permissionRuleParser.LEGACY_TOOL_NAME_ALIASES`, `api.ts` parameter normalizer. Documents the wire shim `wait_up_to (s) × 1000 → timeout (ms)` and the test invariant `getLegacyToolNames(canonical) === aliases`.
3. **§5.6 (MED)** — added "Single-shot SDK emit guarantee" paragraph documenting that `emitTaskTerminatedSdk` fires only when `prevTask.notified` was false, and that dual-stop races silently lose the SDK event.
4. **§5.7 (MED)** — strengthened "not in `getAllTasks()`" to "not imported by `src/tasks.ts` at all", and made the `TaskStop` unreachability explicit (returns `unsupported_type` via `getTaskByType('in_process_teammate') === undefined`).
5. **§5.12.1 (MED, new)** — full subsection on `LocalAgentTask.autoBackgroundMs` (the worker-timeout analogue Phase 9.6/30 flagged), plus the `RemoteAgentTask` hardcoded 30-min review timeout. Cross-referenced from §5.13 LocalMainSessionTask placement.
6. **§10 (MED)** — added bullets for `autoBackgroundMs` and `REMOTE_REVIEW_TIMEOUT_MS`.
7. **§6.8 (LOW)** — promoted the `"external" === 'ant'` line from "build-time replacement that did not fire" to "Bun `--define`-folded `process.env.USER_TYPE`" with cross-reference to §12.1.
8. **§12.1 (LOW, new)** — promoted Open Question 1 to a Resolved subsection. Cited the un-folded `process.env.USER_TYPE === 'ant'` at `TaskStopTool.ts:46` as evidence for the `--define` hypothesis.
9. **§12.2 (LOW, renumber)** — remaining open questions renumbered 1–5 (was 2–6 with item 1 promoted to §12.1). Item 1 (in_process_teammate) updated to cite §5.7's stronger "not imported" claim. Item 2 fixed typo: `TaskOutputTool` → `TodoWriteTool` (the asymmetry is in TodoWrite's return shape, not TaskOutput).

## NITs deferred (no behavioral impact)

- Duplicate `MAX_RECENT_ACTIVITIES = 5` (now §12.2 item 5).
- Stale "comments" mention in `TaskGetTool/prompt.ts:23` (now §12.2 item 4).

## Cross-spec ripples queued

- **Spec 09 (permissions)**: spec 15 §3.6.1 cites `permissionRuleParser.LEGACY_TOOL_NAME_ALIASES`. Spec 09 owns the rule-resolution flow and should mirror this enumeration.
- **Spec 14 (AgentTool spawns)**: spec 14 should cite `autoBackgroundMs` as the parameter AgentTool supplies when configuring foreground-then-background dispatch.
- **Spec 30 (coordinator)**: §5.12.1's mention of `REMOTE_REVIEW_TIMEOUT_MS` answers the Phase 9.6/30 worker-timeout finding for `remote_agent` review tasks.
- **Spec 41 (AppState)**: §12.2 item 2 (TodoWrite collapse asymmetry) still survives — needs replay-invariant confirmation.

## No corrections (claim integrity)

The adversarial review found zero claim errors in spec 15. All edits above are purely additive enumeration — no rewording of behavioral claims, no line-number corrections, no removed assertions.
