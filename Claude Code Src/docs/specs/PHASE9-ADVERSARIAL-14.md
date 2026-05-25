# Phase 9.5 Adversarial Review — Spec 14 (Tool/Agent/Team Surface)

Reviewer: Opus side. Source verification limited to ~14 files in `src/tools/AgentTool/`, `TeamCreateTool/`, `TeamDeleteTool/`, `SendMessageTool/`, `coordinator/`, `tools.ts`. Spec is 1300+ lines; covers Agent, TeamCreate, TeamDelete, SendMessage tool surfaces.

## Severity counts
- Critical: 0
- High: 1
- Medium: 4
- Low: 5
- Cosmetic: 3

## Top 5 findings

### H1 (High) — `checkPermissions` `passthrough` claim mis-states the gate scope vs source
Spec §3.1/§6.7 says: "Routes to `passthrough` only on ANT builds when in `auto` mode; otherwise always `allow`." Source at `AgentTool.tsx:1287` is `if ("external" === 'ant' && appState.toolPermissionContext.mode === 'auto')`. The literal-string `"external" === 'ant'` is the **build-time DCE marker** (per §3.1.fn / §6.2 it is documented elsewhere as such, but here in §3.1 the prose drops the DCE framing and reads as a runtime ANT check). For external builds this branch is **eliminated entirely** — `auto` mode does not exist for them. The spec phrasing "ANT builds when in `auto` mode" is true but obscures that external builds have *no* auto-mode classifier path at all and the comment in source is identical. Tighten wording or add note that DCE removes the branch for external bundles.

### M1 (Medium) — `is_async` analytics field uses different formula than `shouldRunAsync`
Spec §3.1 step 15 / §5.1 documents `shouldRunAsync = (run_in_background || selectedAgent.background || isCoordinator || forceAsync || assistantForceAsync || isProactiveActive) && !isBackgroundTasksDisabled`. Source confirms at `AgentTool.tsx:557+`. But at `AgentTool.tsx:426` the analytics field is `is_async: (run_in_background === true || selectedAgent.background === true) && !isBackgroundTasksDisabled` — **drops `isCoordinator`, `forceAsync`, `assistantForceAsync`, `proactive`**. Same shape repeats at line 548 (`isAsync` for spawn). Spec does not call out this divergence; analytics consumers will under-count async runs caused by fork/coordinator/Kairos. Either correct or document.

### M2 (Medium) — Tool budget enforcement claim is unsubstantiated
Review prompt asks about "max tools per subagent". Spec does not claim per-call tool budget enforcement at the tool surface. Source confirms only `maxTurns` (`runAgent.ts:259, 756`). There is no `maxTools` field in `AgentJsonSchema` (`loadAgentsDir.ts:73-`). If 30/41 reference a tool budget, this is a cross-spec drift to verify; spec 14 correctly omits it but should add an explicit "no tool-count budget at this surface — only `maxTurns`" note to forestall misreads.

### M3 (Medium) — Fork-recursion guard wording elides `querySource` shape
Spec §3.1 step 8: `querySource === 'agent:builtin:fork'`. Source `AgentTool.tsx:332` uses template `agent:builtin:${FORK_AGENT.agentType}` where `FORK_AGENT.agentType = 'fork'`. Equivalent today but brittle if `FORK_SUBAGENT_TYPE` is ever renamed; spec should reference the constant rather than the literal so adversarial test does not hard-pin.

### M4 (Medium) — `forkSubagent.ts` ownership boundary violation in source map
Spec §1 declares: "`forkSubagent.ts` is referenced here only at the *gate-predicate* level (`isForkSubagentEnabled`) — fork-message construction and execution belong to spec 30." But §6 / cross-spec claims must hold up: `forkSubagent.ts` (read in full, 210 lines) defines `FORK_AGENT` definition object, `buildForkedMessages`, `buildChildMessage`, and `buildWorktreeNotice` — these are message construction, not predicates. The Phase 9.4 boundary handoff annotation is therefore **partially inaccurate**: §3.1 step 14 ("Worktree creation … `buildWorktreeNotice` call") is in spec 30's body, but `FORK_AGENT` itself (a `BuiltInAgentDefinition` literal) is needed by 14's selection logic at `AgentTool.tsx:335`. Either (a) move `FORK_AGENT` definition citation into §4 of 14 explicitly (it currently appears only in §6 verbatim asset), or (b) clarify that 14 owns the *symbol* and 30 owns the *behavior*. Today the spec gestures at this but is fuzzy.

### L1 (Low) — `aliases` claim verified but spec says "permission rules, hooks, resumed sessions"
`AgentTool.tsx:228 aliases: [LEGACY_AGENT_TOOL_NAME]` confirmed. Spec §3.1 attributes alias survival to "permission rules, hooks, and resumed sessions" — none of those three uses are cited from `src/`; this is asserted, not evidenced. Add file refs or weaken to "presumed for backward-compat".

## Other findings (Low / Cosmetic)
- **L2** §3.4 says SendMessage `userFacingName: () => 'SendMessage'`; source `SendMessageTool.ts:526` confirmed but spec §3.2/§3.3 has TeamCreate/TeamDelete `userFacingName: () => ''` — verified. No issue, but the asymmetry deserves a one-liner explaining why TeamCreate/Delete return empty (ToolSearch UI override) while SendMessage does not.
- **L3** §3.1 claims `maxResultSizeChars = 100_000` for all four tools. Verified for AgentTool (line 229), TeamCreate (line 77), TeamDelete and SendMessage not directly verified in this pass (boundary line ranges read). Likely correct; flag as un-spot-checked.
- **L4** §5.4 cites "10.2% of fleet `cache_creation` tokens" for `shouldInjectAgentListInMessages` rationale. Number is unverifiable from source; flag as marketing/comment lift, not architectural fact.
- **L5** §3.1 "saves ~135 chars × 34M Explore runs/week" trailer-skip rationale — same: unverifiable, comment lift.
- **C1** §6.4 isolation enum line `process.env.USER_TYPE === 'ant'` — DCE here is **runtime not build-time** (no `bun:bundle feature()`). Verified at `loadAgentsDir.ts:755` and `607`. Spec correctly cites both; minor: external builds still pay the runtime check cost. Cosmetic only.
- **C2** Spec consistently writes `"external" === 'ant'` as a literal — readers unfamiliar with the bundler convention may misread as a typo. One-line gloss in §3 would help.
- **C3** §3.4.2 routing order: spec lists UDS_INBOX before in-process; source at `SendMessageTool.ts:741+` not fully read in this pass — accepted as plausible.

## Verdict
**Accept with revisions.** Findings are all surface-level (analytics divergence, ownership-boundary fuzziness, unverifiable comment-lift stats). No structural bugs. M1 and M4 should be addressed before merge; H1 is a wording fix.

## Cross-spec impact
- **30 (coordinator)**: M4 boundary clarification needs reciprocal note in 30's source map — `FORK_AGENT` literal lives in `forkSubagent.ts` but is consumed by 14's selection logic.
- **34 (bridge)**: SendMessage bridge-target `safetyCheck` decision-reason mentioned in §3.4.1 — verify 34 documents the mirrored `classifierApprovable: false` behavior.
- **08 (tool registry)**: lazy-getter pattern §3.5 cleanly imported; no drift.
- **09 (permissions)**: `filterDeniedAgents` interface is borrowed from 09; H1 wording fix should propagate.
- **15 (task lifecycle)**: `LocalAgentTask`/`RemoteAgentTask` registration cited correctly as 15's domain.
- **41 (agent IDs)**: `formatAgentId(TEAM_LEAD_NAME, finalTeamName)` deterministic-ID claim verified at `TeamCreateTool.ts:146`; no drift.
- **37/37b/42a**: not exercised in this surface; no impact found.

## Hardest-to-verify claim
The §5.3 `getBuiltInAgents` ordering and gate fall-through (`tengu_amber_stoat` default-true GrowthBook flag, `tengu_hive_evidence` default-false, SDK env-skip, `coordinatorMode` lazy require precedence over Explore/Plan). The spec asserts a specific evaluation order that interleaves a build-time `feature()` flag, a runtime env var, two GrowthBook flags with different defaults, and a lazy `require()`. To fully verify requires reading `builtInAgents.ts` (72 lines, not read here), `coordinator/workerAgent.ts` (spec 30), and the GrowthBook flag-registry — three subsystems' interaction. The flag-default claims (`amber_stoat=true`, `hive_evidence=false`) are in particular lifted from comments and **not independently verifiable** from this repo (no GrowthBook config ships with the leak).
