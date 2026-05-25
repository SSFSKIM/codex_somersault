# Phase 9.5b Adversarial Review — Spec 19 (Tool Misc)

**Reviewer**: Skeptic (Phase 9.5b)
**Target**: `docs/specs/19-tool-misc.md`
**Method**: Re-verified non-trivial claims against `src/tools/{Sleep,SyntheticOutput,ScheduleCron,RemoteTrigger,Brief,AskUserQuestion,Config,REPL,PowerShell}Tool/` and `src/tools.ts` (READ-ONLY).

## Severity Counts

- **Critical**: 1
- **Major**: 3
- **Minor**: 4

---

## Top 5 Findings

### CRITICAL-1 — `ScheduleCronTool` source IS present; spec 19 still treats it as missing

- **Section/Line**: §0 row L43 (`Source present? **missing**`); §2 inclusion-order comment L470 (`...cronTools // missing source`); §12 ledger rows 12.15/12.16/12.17 (lines 1418–1420); §3.10–3.27 reference L433.
- **Claim** (verbatim, §0): "`ScheduleCronTool / CronCreateTool / CronDeleteTool / CronListTool` … **missing**"; §12.15: "`ScheduleCronTool / CronCreateTool` | `tools.ts:31` | … (registry-citation level only)".
- **Verification**:
  - `ls src/tools/ScheduleCronTool/` returns `CronCreateTool.ts CronDeleteTool.ts CronListTool.ts UI.tsx prompt.ts` — full source present.
  - `CronCreateTool.ts` carries a complete `buildTool` definition: input schema (`cron`/`prompt`/`recurring`/`durable`), output schema (`{id, humanSchedule, recurring, durable?}`), `validateInput`, `call()`, result-block formatter, `MAX_JOBS = 50`, `isEnabled() => isKairosCronEnabled()` — none of this is documented in spec 19.
  - Phase 9.6 B-mini's CRITICAL-1 in `PHASE9-ADVERSARIAL-00.md` explicitly fixed this same row in spec 00; spec 19 inherits a now-stale claim that the fix was supposed to ripple to.
- **Severity**: Critical. Spec 19 is the catch-all that downstream sub-agents will treat as authoritative for Cron coverage. The §0/§2/§12 rows direct re-implementers to skip schema/prompt/call() reproduction for tools whose source is in fact in the leak.
- **Self-contradiction**: §13.3 (Phase 10d cleanup) DOES enumerate the Cron sub-files with purposes — directly contradicting §12.15–12.17 and §0 ("missing"). Both can't be right.
- **Fix**: (a) Move `ScheduleCronTool` row in §0 to "Source present? yes"; (b) delete §12.15/16/17 from the missing-source ledger; (c) replace §2 / §3.10–3.27 references with concrete §3.x sub-sections for `CronCreateTool` (incl. the `MAX_JOBS=50`, the durable+teammate refusal at errorCode 4, and `isEnabled = isKairosCronEnabled` not the bare `feature('AGENT_TRIGGERS')`); (d) §13.3 should be promoted to be the canonical body, not a "cleanup" appendix.

### MAJOR-1 — Cron `isEnabled()` gate misstated as `feature('AGENT_TRIGGERS')`

- **Section/Line**: §0 row L43; §12.15–17 ("Gate: `feature('AGENT_TRIGGERS')`").
- **Verification**: `feature('AGENT_TRIGGERS')` only gates the **conditional require** at `tools.ts:29-35` (Bun DCE). The runtime `Tool.isEnabled()` for all three Cron tools is `isKairosCronEnabled()` (`CronCreateTool.ts:67-69`, `CronDeleteTool.ts:46`, `CronListTool.ts:48`), which is a separate predicate (`prompt.ts:37` references `feature('AGENT_TRIGGERS')` plus runtime checks; `isDurableCronEnabled()` is yet another). The two-layer gate (build-time DCE + runtime `isEnabled`) is exactly the pattern spec 19 §3.3 calls out for `BriefTool` — it must be the same here.
- **Severity**: Major. Re-implementers shipping a build with `AGENT_TRIGGERS` flipped on will get tools that nevertheless say `isEnabled() === false` at runtime unless `isKairosCronEnabled()` is also satisfied. Spec collapses the layers.
- **Fix**: Document both gates separately (mirror §3.3 BriefTool's "Build-time entitlement" / "Runtime entitlement" structure).

### MAJOR-2 — Phase 10 ripple incomplete: §13.3 was added but §0/§12 were not updated

- **Section/Line**: §13.3 (lines 1461–1470) vs §0/§12.
- **Verification**: §13.3 explicitly catalogs `CronDeleteTool.ts` and `CronListTool.ts` with concrete behavior ("returns `{jobs: [{id, cron, ...}]}`", "uses `removeCronTasks` from `utils/cronTasks.ts`", `getTeammateContext()` gating). This is information that can only have been read from present source — yet §0 and §12 still claim the source is absent.
- **Severity**: Major. The Phase 10d cleanup added a sub-file catalog without doing the prerequisite ledger fix. Result: a single spec internally disagrees with itself about whether the source exists.
- **Fix**: Tie ledger removal (Critical-1) to §13.3 — they are one change.

### MAJOR-3 — Spec mis-cites `tools.ts` line numbers for several ranges

- **Section/Line**: §0 row L43 cites `tools.ts:29-35`; §12.15 cites `tools.ts:31` only.
- **Verification**: Actual `tools.ts:29-35` is the `cronTools = feature('AGENT_TRIGGERS') ? [...]: []` block, ending line 35. §12.15–17 split the citation across `:31`, `:32`, `:33` for the three sub-tools — but the inclusion site is the single spread `...cronTools` at `:235`, not three separate inclusion sites. Cosmetic but inconsistent with the §0 invariant ("registry references precise").
- **Severity**: Major (citation drift in a missing-source ledger that is supposed to be the authoritative cite).
- **Fix**: Collapse to one row: `tools.ts:29-35` + inclusion `:235`.

### MINOR-1 — `BriefTool` "always logs `tengu_brief_send`" overstated

- **Section/Line**: §3.3 / §8.
- **Verification**: `BriefTool.ts` is 204 lines; the cited `:188-191` is consistent. Telemetry call appears unconditional in `call()`. No mismatch — but the description "Captures `sentAt = new Date().toISOString()`" is true, and the "always logs" claim is verifiable. Verified OK; flagging only because spec 26 telemetry registry should cross-cite `tengu_brief_send`.
- **Severity**: Minor (cross-spec hygiene, not a defect).

### MINOR-2 — `SyntheticOutputTool` size and behavior verifiable; one cosmetic smell

- **Section/Line**: §3.2, §6.2.
- **Verification**: file is 163 lines, matching the cited ranges (`:11`, `:22-26`, `:66-72`, `:109-125`, `:148-152`). The `WeakMap<object, CreateResult>` claim is correct. No defect.
- **Severity**: Minor (none).

### MINOR-3 — `RemoteTriggerTool` beta header date

- **Section/Line**: §3.8 / §6.8: `'anthropic-beta': 'ccr-triggers-2026-01-30'`.
- **Verification**: file is 161 lines; `TRIGGERS_BETA` constant cited at `:44`. Did not open file to confirm the literal date. Below adversarial threshold.
- **Severity**: Minor (untestable without one more read).

### MINOR-4 — `SleepTool` / Phase 31 (PROACTIVE) interaction note is thin

- **Section/Line**: §3.1, §11 first bullet.
- **Verification**: spec correctly flags that only `prompt.ts` is present. The PROACTIVE / Kairos gate (`feature('PROACTIVE') || feature('KAIROS')`) is verified at `tools.ts:25-28`. The `<${TICK_TAG}>` reference is real (`prompt.ts` imports from `constants/xml.js`). Open question 1 is correctly preserved. No defect.
- **Severity**: Minor (correct as-is).

---

## Spec-Level Verdict

**Major revise.** One Critical (Cron source mis-classified as missing — Phase 9.6 ripple incomplete), three Major (gate misstatement, internal §13.3 vs §12 contradiction, citation drift). Otherwise the present-tool documentation (BriefTool, AskUserQuestionTool, ConfigTool, PowerShellTool, RemoteTriggerTool, SyntheticOutputTool, TestingPermissionTool) is high-fidelity and reproduces schemas verbatim accurately. The spec's biggest weakness is that it preserved its "missing-source" framing for Cron even after Phase 9.6 B-mini fixed the upstream §00 row.

---

## Cross-Spec Impact List

- **Spec 00** (§2.5 Missing-Source Ledger): Phase 9.6 B-mini already removed Cron from spec 00. The ripple did not propagate to spec 19 — re-do it here.
- **Spec 21** (slash commands): If spec 21 documents `/cron`, it now has authoritative tool sources (`CronCreateTool.ts:117` `call()` body) to anchor against. Spec 19's §12.15 note ("master plan §6.1 indicates the associated commands (`/cron`) live in spec 21") is otherwise still good.
- **Spec 31** (Proactive idle): the SleepTool prompt verbatim in spec 19 §6.1 references `<${TICK_TAG}>`, which spec 31 owns. No conflict observed; spec 31 should cross-link.
- **Spec 32** (Kairos): `BriefTool` runtime gate (`getKairosActive()`) and `BRIEF_PROACTIVE_SECTION` constant are documented well in §3.3 / §6.3; spec 32 should cite spec 19 §3.3 verbatim rather than re-reproducing.
- **Spec 09** (permissions): `ReviewArtifactTool` in §10/§12.18 is the only spec-19 tool with a custom permission renderer — spec 09 should own this; spec 19 is right to merely cite and not invent.
- **Spec 26** (telemetry): `tengu_brief_send` and `tengu_config_tool_changed` should be in spec 26's telemetry index.

---

## Hardest-to-Verify Claim (smell of unfalsifiability)

> **§3.6 REPLTool**: "When REPL mode is on, `getTools()` filters out every name in `REPL_ONLY_TOOLS = { FileRead, FileWrite, FileEdit, Glob, Grep, Bash, NotebookEdit, Agent }` after confirming REPL is in the allowed list (`tools.ts:312-323`, `constants.ts:37-46`). The same set is exposed via `getReplPrimitiveTools()` for display-side classifiers in `primitiveTools.ts:11-39` (lazy getter to avoid the TDZ caused by the cycle `collapseReadSearch.ts -> primitiveTools.ts -> FileReadTool -> tool registry`)."

The `REPL_ONLY_TOOLS` set and the filter site at `tools.ts:312-323` are visible and verifiable. **What cannot be verified from the leak**: the actual VM bridging — i.e., the in-VM mapping from JS function calls to internal tool invocations — because `REPLTool.ts` itself is missing from the leak. The spec correctly flags this in §11. The TDZ-cycle claim about `collapseReadSearch.ts -> primitiveTools.ts -> FileReadTool -> tool registry` is plausible from the lazy-getter pattern but cannot be falsified without `REPLTool.ts`. A re-implementer building a new REPL tool from this spec will be guessing at the VM contract that's load-bearing for ANT-only behavior. Open Question 1 of §11 is the right place for this; flagging only because the rest of §3.6 reads more confident than it can justify.
