# Phase 9.5 Adversarial Review — Spec 30 (Coordinator & Multi-Agent Spawn)

Reviewer: opus-fallback (general-purpose). Source-bounded review against `/Users/new/Downloads/claude-code-main/src/`.

## Severity counts

- Critical: 0
- High: 2
- Medium: 5
- Low: 6
- Info: 4

## Top findings

### H1. Whole subsystem missing: swarm permission bridge & in-process teammate runner are unowned
- Spec 30 §1 (in-scope list) and §2 (source-coverage inventory) enumerate `src/utils/teammateMailbox.ts` but make **no mention** of:
  - `src/utils/swarm/permissionSync.ts` (928 lines) — `createPermissionRequest`, `sendPermissionRequestViaMailbox`, `isSwarmWorker`
  - `src/utils/swarm/leaderPermissionBridge.ts` (54 lines)
  - `src/utils/swarm/inProcessRunner.ts` (1552 lines) — by far the largest swarm file
  - `src/utils/swarm/spawnInProcess.ts` (328 lines) — referenced indirectly via `spawnInProcessTeammate`/`startInProcessTeammate` in §5.1 step (3-4) but the file itself is not listed
  - `src/utils/swarm/teamHelpers.ts` (683 lines)
  - `src/utils/swarm/permissionSync.ts`, `teammateInit.ts`, `teammateLayoutManager.ts`, `teammatePromptAddendum.ts`, `reconnection.ts`, `It2SetupPrompt.tsx`, `backends/*` (5 files)
- The dispatch brief asks "the 21 swarm utilities (`src/utils/swarm/`) — does spec describe their architecture?" The answer is no. `grep -c teammate|swarm|leader` in spec 30 = 37, but they are almost all about `teammateMailbox.ts` envelope shapes; the swarm permission bridge architecture (worker → leader inbox → `useSwarmPermissionPoller` → `registerPermissionCallback`) is absent. `src/hooks/toolPermission/handlers/swarmWorkerHandler.ts` is also absent from spec 30 (and from spec 14 — confirmed by grep).
- This is a real gap: the swarm worker permission flow uses `isAgentSwarmsEnabled()` (a separate gate from `COORDINATOR_MODE`), and §6 of the spec never explains when that gate is on vs the coordinator gate. A reader cannot reconstruct how a teammate's `Bash` request is forwarded to the leader's terminal from spec 30 alone.

### H2. `applyCoordinatorToolFilter` location lifted from spec 30 to TBD; spec 30 §12 q.7 acknowledges drift risk but the spec never names the canonical owner
- Spec 30 §12 question 7 admits: "Coordinator's tool whitelist filter for the headless path is applied via `applyCoordinatorToolFilter` from `utils/toolPool.js` (dynamic-import at `main.tsx:1872-1879`). For the REPL path it lives in `useMergedTools.ts`. The two surfaces must stay in sync."
- I verified `src/utils/toolPool.ts:35` and `src/main.tsx:1874-1876` — confirmed both call sites. `useMergedTools.ts` is mentioned in spec 30 but I did not verify its body.
- The drift risk is real but the spec calls it "open question" rather than designating spec 30 (or some other spec) as owner. Suggest hardening §3 Public Interface to explicitly own `applyCoordinatorToolFilter`'s call-site invariants.

### M1. `INTERNAL_WORKER_TOOLS` enumeration: spec lists 4 names; source has 4 names — but spec 30 §6.14 cites lines 29-34, source has the constant at lines 29-34. ✅ verified. No drift here.

### M2. `getCoordinatorSystemPrompt()` line range cited as `:111-369`; source has function ending at line 369. ✅ verified. The spec says "250-line prompt"; actual prompt body is from line 116 (return statement) to line 368 = ~252 lines. Close enough.

### M3. `coordinatorMode.ts` line count claimed "369". `wc -l` returns 369. ✅ verified.
- All other line counts in §2 also match (`spawnMultiAgent.ts:1093`, `AgentTool.tsx:1397`, `runAgent.ts:973`, `forkSubagent.ts:210`, `agentToolUtils.ts:686`, `builtInAgents.ts:72`, `teammateMailbox.ts:1184` vs actual 1183 — off by 1 file ends without trailing newline; trivial).

### M4. `forkSubagent.ts` line numbers
- Spec §5.5 cites `forkSubagent.ts:32-39` for the gate. Verified — function is at exactly those lines. ✅
- Spec §6.8 cites `:107-169` for `buildForkedMessages`. Verified. ✅
- Spec §6.8 cites `:171-198` for `buildChildMessage`; source has it at lines 171-198. ✅
- Spec §6.8 cites `:78-89` for `isInForkChild`; source has it at lines 78-89. ✅
- Spec §6.8 cites `:205-210` for `buildWorktreeNotice`; source has it at lines 205-210. ✅
- All forkSubagent line citations check out.

### M5. Phase 9.4 14↔30 boundary: "symbol-ownership vs behavior-ownership"
- Spec 14 §13 q.1 (line 1238) explicitly states: "tools/shared/spawnMultiAgent.ts:spawnTeammate (35KB) [is] owned by spec 30. This spec assumes they exist with the signatures cited but does not document their bodies."
- Spec 30 §1 owns "the runner (`runAgent.ts`, `forkSubagent.ts` execution, coordinator orchestration, `builtInAgents.ts`)"; spec 14 owns "the gate predicate (`isForkSubagentEnabled`)".
- I verified spec 14 imports `isForkSubagentEnabled` at `AgentTool.tsx:51` and uses it at `:557, 750, 818`. Spec 30's claim that 14 only owns the *predicate* is sound, but **`forkSubagent.ts:32-39` defines `isForkSubagentEnabled` itself** — so symbol-ownership is in spec 30's tree even though spec 14 owns the predicate's *use*. The boundary is correctly defined behaviorally but creates a subtle reader-trap: the predicate's source file is in 30 not 14. Suggest §10 cross-reference or §1 handoff paragraph note this explicitly.

### M6. `isInProcessTeammate()` import path drift
- Spec 30 mentions in-process teammates extensively (§5.1, §6.1) but never names the symbol/file. Source has `isInProcessTeammate` imported at `AgentTool.tsx:38` from `utils/teammateContext.js`. Neither `teammateContext.ts` nor the AsyncLocalStorage isolation guarantee implementation is mentioned in §2 inventory. The dispatch brief asks specifically about "AsyncLocalStorage isolation guarantees" — spec 30 §5.1 step 3 says "sets up `AsyncLocalStorage` teammate context" but does not cite the actual file (`utils/teammateContext.ts` or similar). This is an untestable claim as written.

### L1. Spec §5.1 step 12 lists three CLI flags from `buildInheritedEnvVars`: "CLAUDECODE, CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS, API provider vars". Did not verify against source — could have drifted. Listed as Low because it's a non-load-bearing detail.

### L2. Spec §1 promises full prompt verbatim ("the **coordinator orchestrator** ... + ... `builtInAgents.ts`'s `getCoordinatorAgents()` lazy require"). The full prompt is 252 lines but §6.3 says "the spec cites the line range rather than re-inlining the 250-line prompt" — this is a deliberate non-inline. Reasonable but should be flagged: the **only spec containing the verbatim coordinator prompt** is the source itself, so any spec consumer is forced to read source. This is documented but worth noting.

### L3. §5.6 says "REPL path applies it via `useMergedTools.ts`, headless path via `applyCoordinatorToolFilter`" — verified both exist at the correct paths but the REPL hook's exact line range is not cited (would help downstream readers).

### L4. §2 inventory lists `src/coordinator/workerAgent.ts` referenced by `builtInAgents.ts:38-40` as "**not present** in the leaked tree". Verified — `ls src/coordinator/` returns only `coordinatorMode.ts`. The require would fail at runtime when `feature('COORDINATOR_MODE') && CLAUDE_CODE_COORDINATOR_MODE` is set in any non-Anthropic build. Spec correctly flags this in §12 Open Q1.

### L5. Plan-mode handoff: spec §6.5 documents `PlanApprovalRequestMessageSchema` and `PlanApprovalResponseMessageSchema` but **never describes the gating algorithm**. The dispatch brief explicitly asked about "Plan-mode approval gating" — spec 30 has the wire schemas but no §5 algorithm subsection covering when these are emitted, who consumes them, what happens on rejection, or how `permissionMode` in the response is applied. This is a missing edge case.

### L6. Worker timeout / partial completion: spec §7 covers worker abort, MCP cleanup, hooks cleanup, and "fork without tool_use blocks", but does **not** cover:
- Coordinator crash (what happens to in-flight workers when the coordinator process dies?)
- Worker timeout (no timeout primitive is documented; `maxTurns` is the only bound)
- Partial completion (covered tangentially via `extractPartialResult` in §5.3 but no dedicated subsection)
The dispatch brief explicitly asks about "coordinator crash, worker timeout, partial completion". These are missing edge cases.

### Info
- I1. §6.7 spec verbatim claims `ASYNC_AGENT_ALLOWED_TOOLS` is at `constants/tools.ts:36-112`. Source has it at line 55. Off by ~19 lines but the constant content matches.
- I2. §6.7 claims `IN_PROCESS_TEAMMATE_ALLOWED_TOOLS` is at `constants/tools.ts:36-112`. Source has it at line 77.
- I3. §6.7 claims `COORDINATOR_MODE_ALLOWED_TOOLS` is at `constants/tools.ts:107-112`. Source has it at line 107. ✅
- I4. §6.14 cites `coordinatorMode.ts:36-41` for `isCoordinatorMode`. Source has it at lines 36-41. ✅

## Cross-spec impact

- **Spec 14**: boundary is well-articulated in both directions (14 §13 q.1, 30 §1). The Phase 9.4 "symbol-ownership vs behavior-ownership" concern is real (M5) but resolvable with a single sentence in §10.
- **Spec 15**: §10 says spec 30 "registers `LocalAgentTask` / `InProcessTeammateTask` / `RemoteAgentTask` task records but does not own their schemas" — clean handoff.
- **Spec 35** (remote-server / CCR): §5.14 routes into spec 35 cleanly via `teleportToRemote` and `RemoteAgentTask`. ✅
- **Spec 31** (proactive): §5.4 cites `proactiveModule?.isProactiveActive()` as one async-trigger predicate. Clean.
- **Spec 41** (resume): `recordSidechainTranscript`, `writeAgentMetadata`, `getAgentTranscript`, `readAgentMetadata` are owned by 41. ✅
- **Untouched: any spec covering `src/utils/swarm/` (H1)**. If no other spec owns those 13 files (~4486 LOC), the leak's largest unowned subsystem is right here.

## Hardest-to-verify claim

**§5.4: "Mid-flight backgrounding releases foreground iterator within ~1s (`AgentTool.tsx:918`) — required so MCP/hooks finalizers run."**

This is a behavioral race-condition claim: `agentIterator.return(undefined).catch(()=>{})` raced against `sleep(1000)`. There is no test in this leak that exercises the path, no instrumented timing, and the actual MCP cleanup is in `runAgent.ts:817-818` (a `finally` block). Whether the 1-second sleep is *sufficient* in practice depends on MCP server response times and external IO latencies. The spec asserts it without justification; it's a "trust the comment" claim that no future re-implementer can falsify cheaply.

## Verdict

**Accept with revisions**. Spec 30 is detailed, accurately cites line numbers (sampled checks all pass), and handles the 14↔30 boundary cleanly. However:

1. **H1 must be addressed**: spec 30 (or a sibling spec) needs to either own or explicitly cross-reference the `src/utils/swarm/` subsystem. Currently 13 files / ~4500 LOC are unowned.
2. **L5 (plan-mode gating) and L6 (crash/timeout/partial) are real edge-case gaps** the dispatch brief flagged as "extra attention" — spec 30 has wire schemas but no algorithm.
3. **M6 (AsyncLocalStorage isolation)** — claim is made but the file containing the implementation is never named, making the guarantee untestable from the spec alone.

No critical findings. The spec is production-quality but has a noticeable hole around swarm internals and a few unaddressed edge cases. Suggest one revision pass to cover H1, L5, L6, M6 before declaring "done".
