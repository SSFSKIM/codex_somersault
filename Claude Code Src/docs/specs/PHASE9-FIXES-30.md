# Phase 9.6 B-full — Spec 30 fix log

Source: `PHASE9-ADVERSARIAL-30.md`. Target: `30-coordinator-multiagent.md`. Date: 2026-05-09.

## Verified-before-edit (source spot-checks)

- `find src/utils/swarm -type f \( -name "*.ts" -o -name "*.tsx" \)` → **22 files, 7,548 LOC** (review estimate of 13/4486 was undercounted; total higher because backends/ alone is 9 files).
- `forkSubagent.ts` lives at **`src/tools/AgentTool/forkSubagent.ts`** (not `src/utils/swarm/`). `isForkSubagentEnabled` is at `:32-39`, mutual-exclusion with coordinator at `:34` (`if (isCoordinatorMode()) return false`). Confirmed.
- `isAgentSwarmsEnabled` at `src/utils/agentSwarmsEnabled.ts:24-44`. Triggers: ant always-on; external requires (`CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` env OR `--agent-teams` CLI) AND GrowthBook `tengu_amber_flint`. Confirmed.
- `isCoordinatorMode` at `src/coordinator/coordinatorMode.ts:36-41`. Requires `feature('COORDINATOR_MODE')` AND env-truthy `CLAUDE_CODE_COORDINATOR_MODE`. Confirmed.
- Swarm permission flow: `swarmWorkerHandler.ts:43` short-circuits on `isAgentSwarmsEnabled() && isSwarmWorker()`; `permissionSync.ts:596` defines `isSwarmWorker`; `:676` defines `sendPermissionRequestViaMailbox`. `useSwarmPermissionPoller` at `hooks/useSwarmPermissionPoller.ts:268`. Confirmed all four call sites.
- Spec 42a (`42a-utils-long-tail.md`) catalogs swarm files at line 34 inventory and lines 139, 193, 252, 312, 337, 388, 396, 411, 416, 417, 458, 462, 574–581 — and line 749 explicitly delegates ownership to spec 30: "14 / 30 — agent/team/coordinator | all `swarm/*` (21 files)…". Spec 30 referenced 42a in §10 instead of duplicating per-file roles.

## Findings applied

| ID | Severity | Action |
|---|---|---|
| **H1** | High | **§1 IN-scope** expanded to list all swarm subsystem files + ALS + permission-poller hooks. **§1** now contains a "Two distinct gates" subsection disambiguating `isCoordinatorMode()` vs `isAgentSwarmsEnabled()` (independent gates, mutually exclusive personas at the *process* level). **§2.1** new subsection: architectural description of the swarm subsystem broken into 7 layers (in-process runner / permission bridge / spawn-process glue / backend abstraction / reconnection-layout / identity-model-prompt / it2-setup UI). Each layer cites file(s) + LOC + role. AsyncLocalStorage isolation (M6) explained at the end of §2.1. Per-file signature inventory delegated to spec 42a per its own line 749. |
| **H2** | High | **§3** new paragraph "Coordinator tool-pool filter — canonical owner: spec 30." Names both call sites (`main.tsx:1872-1879` headless, `hooks/useMergedTools.ts` REPL), declares spec 30 owns the invariant that both surfaces produce identical filtered lists, and lists the three places that change must touch in lockstep (`INTERNAL_WORKER_TOOLS`, `COORDINATOR_MODE_ALLOWED_TOOLS`, both call sites). **§12 q.7** marked resolved with strikethrough + pointer to §3. |
| **M5** | Medium | **§10** new bullet "14↔30 reader-trap (symbol-ownership vs behavior-ownership)" makes the boundary explicit: spec 30 owns the *implementation* (`forkSubagent.ts:32-39`), spec 14 owns the *call sites* (`AgentTool.tsx:51, 557, 750, 818`). Same split applied to `FORK_AGENT`, `buildForkedMessages`, `buildChildMessage`, `isInForkChild`. Plus a separate bullet pointing at spec 42a for swarm-file signatures. |
| **M6** | Medium | Addressed in §2.1 final paragraph — names `utils/teammateContext.ts` and `isInProcessTeammate` (`AgentTool.tsx:38`), describes the ALS store key shape `{teammateId, teamName, agentName, parentSessionId}` and the `runWith(ctx, fn)` wrap in `swarm/spawnInProcess.ts`. Notes guarantee is per-async-context (not per-thread). |
| **L5** | Low | **§7.1** new subsection "Plan-mode approval gating (worker → leader handshake)". Covers: emitter (`ExitPlanModeV2Tool.ts:406`), envelope schemas (`plan_approval_request` / `plan_approval_response`), 4-step algorithm, leader process exit edge case (no worker timeout — polls indefinitely), and the orthogonal force at `main.tsx:2911` that overrides per-turn `permissionMode` to `'plan'` for swarm workers. |
| **L6** | Low | **§7.2** new subsection "Crash, timeout, and partial-completion edge cases". Covers: coordinator crash with in-flight workers (no on-disk handoff; sidechain persists but `finally` cleanup leaks on SIGKILL); swarm leader crash (workers survive; reconnection.ts best-effort reattach); worker timeout (none — `maxTurns` is the only bound; long-running tool inherits tool-level timeout, not worker-level); partial completion via `extractPartialResult` returning `'killed'` notification (with empty-string fallback when no assistant block emitted yet); mid-flight-backgrounding 1 s race cross-referenced to §11. |

## Findings not applied

| ID | Severity | Reason |
|---|---|---|
| M1, M2, M3, M4 | Medium | All marked ✅ verified in the adversarial review — no spec change required. |
| L1 | Low | `buildInheritedEnvVars` env var list — review explicitly marked "did not verify against source… non-load-bearing". Not worth a re-verification round given the 25-read budget. |
| L2 | Low | The "verbatim coordinator prompt is only in source" trade-off is intentional (250-line prompt). Spec already documents the non-inline. No change. |
| L3 | Low | `useMergedTools.ts` exact line range — addressed transitively by H2 (now declared as a co-owned call site in §3). |
| L4 | Low | `coordinator/workerAgent.ts` missing-from-leak — already documented in §2 and §12 q.1. No change needed. |
| I1, I2, I3, I4 | Info | Off-by-N line numbers in `constants/tools.ts` (info I1=line 55 actual vs spec ~36, I2=line 77 vs ~36) — the constants exist with matching content; correcting line numbers across the spec is a churn cost not justified for info-level drift. The spec's range citation `36-112` is a *block range* covering the whole table, which is technically still accurate. Skipped. |

## Two-gate disambiguation summary

`isCoordinatorMode()` and `isAgentSwarmsEnabled()` are **independent feature flags with independent killswitches**:

- A coordinator process orchestrates worker subagents through the AgentTool dispatch path. It is identified by `feature('COORDINATOR_MODE') && CLAUDE_CODE_COORDINATOR_MODE`. Coordinator workers run in the *same* process via `runAgent`.
- A swarm-enabled session can spawn multi-*process* teammates (own pane, own claude binary, own inbox). Each teammate is `isSwarmWorker() === true` in its own process. Identified by `(--agent-teams || CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS) && tengu_amber_flint`.
- The fork gate `isForkSubagentEnabled` explicitly excludes coordinator (`forkSubagent.ts:34`), making coordinator and fork mutually exclusive. Coordinator and swarm are mutually exclusive at the *process* level (a coordinator process is not a swarm worker), but a coordinator session running as ant-internal will have `isAgentSwarmsEnabled() === true` due to the ant-always-on branch — the gates *can* both return true in the same process; what cannot happen is `isCoordinatorMode() && isSwarmWorker()` (a coordinator is never spawned as someone else's teammate).

## Swarm subsystem section structure (added at §2.1)

7 layers, each with file list + LOC + architectural role:
1. In-process runner (`inProcessRunner.ts`, 1552 LOC)
2. Permission bridge (`permissionSync.ts`, `leaderPermissionBridge.ts`, `useSwarmPermissionPoller.ts`, `swarmWorkerHandler.ts`, ~1100 LOC)
3. Spawn-process glue (`spawnInProcess.ts`, `spawnUtils.ts`, `teammateInit.ts`, ~600 LOC)
4. Backend abstraction (9 backends/ files, ~3060 LOC)
5. Reconnection / layout (~226 LOC)
6. Identity / model / prompt / team-helpers (~744 LOC)
7. It2-setup UI (`It2SetupPrompt.tsx`, 379 LOC)

Total: ~7,548 LOC documented. Per-file signatures delegated to spec 42a (cross-ref in §10).

## M5 boundary clarification summary

Reader trap: searching by symbol name (`isForkSubagentEnabled`) lands you in spec 30's tree (`tools/AgentTool/forkSubagent.ts`); searching by *behavior* ("when does fork fire?") lands you in spec 14 (`AgentTool.tsx` call sites). §10 now states this explicitly, applied to all five fork-related symbols owned by spec 30 but consumed by spec 14.

---

*End of fix log.*
