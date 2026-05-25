# Phase 9.6 B-full — Spec 14 Fix Log

Target: `docs/specs/14-tool-agent-team.md` (Tool/Agent/Team surface).
Source of findings: `docs/specs/PHASE9-ADVERSARIAL-14.md`.

## Findings applied

### H1 (High) — `checkPermissions` build-time DCE framing
- §3.1 `checkPermissions` block rewritten. The literal `"external" === 'ant'` is now described as a Bun build-time DCE marker (constant-folded → branch eliminated for external bundles), NOT a runtime env check. External-bundle reimplementers MUST drop the `auto`-mode classifier path entirely; ANT-equivalent reimplementers MUST place it behind a constant-foldable sentinel so the branch is *omitted*, not *reachable-but-skipped*.
- §8.2 first row reworded to make build-time vs runtime explicit (the `loadAgentsDir.ts`, `exploreAgent.ts`, and `agentSwarmsEnabled.ts` rows are bona-fide runtime `process.env.USER_TYPE` checks; the AgentTool row is the only build-time DCE one).
- Cross-ref to §6.9 corrected (was §6.7).

### M1 (Medium) — `is_async` analytics divergence from `shouldRunAsync`
- §3.1 step 15 expanded with a paragraph documenting that `is_async` (`AgentTool.tsx:426`) and `metadata.isAsync` (`AgentTool.tsx:548`) drop `isCoordinator`, `forceAsync` (fork), `assistantForceAsync` (Kairos), and `proactiveModule?.isProactiveActive()`. Telemetry consumers must NOT treat `is_async` as ground truth.
- §10 telemetry section also gained a divergence note pointing back to §3.1 step 15.

### M2 (Medium) — No per-call tool-count budget at this surface
- §11 reimplementation checklist gained explicit "no `maxTools`/`toolBudget` at the AgentTool surface — only `maxTurns`" line citing `loadAgentsDir.ts:73-` and `runAgent.ts:259, 756`. Cross-spec drift in 30/15/41 is flagged as runner-layer or to-be-removed.

### M3 (Medium) — Fork-recursion guard wording uses constant, not literal
- §3.1 step 8 rewritten to use the template form `\`agent:builtin:${FORK_AGENT.agentType}\`` and explicitly note that the resolved string today is `'agent:builtin:fork'` but reimplementations MUST reference the constant, not the literal.

### M4 (Medium) — `forkSubagent.ts` symbol-vs-behavior split
- §1 ownership-handoff rewritten: spec 14 owns the *symbols* `isForkSubagentEnabled` (gate predicate) and `FORK_AGENT` (the `BuiltInAgentDefinition` literal consumed by the selection logic at `AgentTool.tsx:332,335`); spec 30 owns the *behavior* of `buildForkedMessages`, `buildChildMessage`, `buildWorktreeNotice`, `isInForkChild`. `FORK_AGENT` is *declared* in `forkSubagent.ts` but *cited* by §6.5 of this spec because it is registry surface; consumers are not. Reciprocal note required in spec 30's source map.

## Findings skipped / not actioned
- L1, L2, L3, L4, L5, C1, C2, C3 — Low/Cosmetic, not in scope of B-full per prompt (high/major/medium only). C1 is partially addressed as a side effect of §8.2 row reframing.

## Phase 9.6 B-mini ripple
- Spec 14 `grep` for `bubble` returned no matches; no §6.7 reference to bubble runtime mode exists in this spec. No alignment with corrected spec 09 §4.1 needed.

## Top 3 fixes (by impact)
1. **H1 — Build-time DCE explicit framing** (§3.1 + §8.2). Most consequential: prevents reimplementers from emitting a runtime auto-mode classifier in external builds and from misreading the four-row §8.2 ANT-gate cluster as homogeneous.
2. **M4 — Symbol-vs-behavior split for `forkSubagent.ts`** (§1). Removes the boundary fuzziness Phase 9.4 introduced; clarifies that `FORK_AGENT` is registry surface (14) but message-construction is runner (30).
3. **M1 — `is_async` analytics divergence** (§3.1 step 15 + §10). Surfaces a real telemetry under-count that would otherwise mislead anyone using `tengu_agent_tool_selected` to attribute fork/coordinator/Kairos/proactive runs.

## Build-time-vs-runtime patterns surfaced beyond H1
- `bun:bundle feature(...)` flag gates in §8.1 (e.g. `KAIROS`, `COORDINATOR_MODE`, `FORK_SUBAGENT`) — build-time DCE; aligned with project CLAUDE.md "Pattern 1".
- The literal-string `"external" === 'ant'` sentinel in `AgentTool.tsx` — also build-time DCE (project CLAUDE.md "Pattern 2" variant; the more common form is `process.env.USER_TYPE === 'ant'`).
- `process.env.USER_TYPE === 'ant'` in `loadAgentsDir.ts`, `exploreAgent.ts`, `agentSwarmsEnabled.ts` — genuine runtime check (no build-time strip), so external builds still pay the comparison cost. §8.2 now distinguishes the two with **bold** "Build-time" / "Runtime" prefixes.

No structural changes to schemas or §6 verbatim assets. All edits are documentation/clarification.
