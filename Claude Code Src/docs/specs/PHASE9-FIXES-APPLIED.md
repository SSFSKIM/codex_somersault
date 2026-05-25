# Phase 9.4 — Consistency Fixes Applied

Iteration log for fixes applied against `PHASE9-CONSISTENCY.md` findings.
All fixes verified against `src/` before editing. Specs only — no source changes.

---

## Applied fixes

### F1. Spec 26 §8 — pointer to canonical flag matrix
- **Spec / section**: `26-service-analytics-flags.md` §8 / §8.1 heading
- **Change**: Added 2-line note above §8.1 saying "Canonical flag matrix lives in `00-overview.md` §8.1 + §8.1.B (89 flags)" and renamed §8.1 to "Telemetry/analytics build-time flags (this spec's narrow scope)".
- **Verification**: Spec 00 §8.1 + §8.1.B grepped — confirmed it lists all 89 flags. Spec 26 §1 already disclaims wider ownership.

### F2. Spec 09 §4.6 + Spec 08 §4.2 — `ToolPermissionContext` canonical-owner reconciliation
- **Spec / section**: `09-permission-system.md` §4.6 and `08-tool-base-registry.md` §4.2
- **Change**:
  - Spec 09 §4.6: corrected to acknowledge `Tool.ts:123-138` is the canonical owner; `types/permissions.ts:427-441` is a no-runtime-deps mirror that intentionally omits `isAutoModeAvailable`. Reworded the note about runtime injection to clarify the field IS declared on the canonical type and IS populated at `permissionSetup.ts:987`.
  - Spec 08 §4.2: added a "Canonical owner = `Tool.ts`; mirror in `types/permissions.ts`" header note plus an inline comment on the `isAutoModeAvailable?` line.
- **Verification**: Read both `src/Tool.ts:115-148` and `src/types/permissions.ts:1-45,420-441`. Confirmed:
  - `Tool.ts:130` declares `isAutoModeAvailable?: boolean` on the canonical `DeepImmutable<{...}>` type.
  - `types/permissions.ts:1-7` header explicitly says it's a "no runtime dependencies" cycle-breaker.
  - `permissionSetup.ts:987` sets the field via `{ isAutoModeAvailable: isAutoModeGateEnabled() }`.
- **DEPARTURE FROM CONSISTENCY REVIEW**: The review recommended "spec 08 should drop the field from its verbatim block." This is **wrong** — `Tool.ts:130` clearly has it. The opposite fix was applied: spec 09 was corrected to stop claiming the field is "not declared in this type" (it is, on the canonical type; only the mirror omits it).

### F3. Spec 00 §6.2 glossary — `Session` and `ToolUseBlock`
- **Spec / section**: `00-overview.md` §6.2, "Core concepts" subsection
- **Change**: Added two new bullets after the `Turn` definition:
  - `Session` — disambiguates the three uses (transcript-file via spec 41, Ink REPL run via spec 04/37, remote-server connection via spec 35).
  - `ToolUseBlock` — defined as SDK content block of type `tool_use` with `{id, name, input}`; cites spec 04 §5 and consumers 03/08/22.
- **Verification**: Re-read consistency review entry 4.8 + 4.9. Spec 41 / 35 / 04 use the term differently as the review noted.

### F4. SKIPPED — Spec 00 §6.2 PermissionMode
- **Reason**: Verification disagrees with the consistency review.
- **What review claimed**: "spec 00 §6.2 includes `dontAsk` in runtime set; spec 09 EXTERNAL list does not."
- **What src/spec actually shows**:
  - `src/types/permissions.ts:16-22` — `EXTERNAL_PERMISSION_MODES` includes `'dontAsk'` (line 20).
  - Spec 09 §4.1 (line 161) ALREADY lists `dontAsk` correctly inside the `EXTERNAL_PERMISSION_MODES` literal.
  - Spec 00 §6.2 (line 408) ALREADY cites `permissions.ts:33-36` and lists `default, acceptEdits, bypassPermissions, dontAsk, plan` correctly.
- **Conclusion**: Spec 00 and spec 09 are already aligned and both match `permissions.ts`. The consistency review's finding 4.1 was a misread.

### F5. Spec 19 §0 — `ScheduleCronTool` naming clarification
- **Spec / section**: `19-tool-misc.md` §0 inventory table
- **Change**: Added inline HTML comment under the cron tools row clarifying: "ScheduleCronTool" is the *directory* name; the three exported symbols are `CronCreateTool`, `CronDeleteTool`, `CronListTool`. There is no symbol named `ScheduleCronTool`.
- **Verification**:
  - `src/tools.ts:31-33` imports from `./tools/ScheduleCronTool/CronCreateTool.js` etc. — directory is `ScheduleCronTool/`, exports are `Cron{Create,Delete,List}Tool`.
  - `src/tools/ScheduleCronTool/CronCreateTool.ts:56` confirms `export const CronCreateTool = buildTool(...)`.
- **Note**: Did NOT rename `ScheduleCronTool` references — both names are valid (directory vs symbol). Just clarified.

### F6. Spec 19 — TungstenTool subsection
- **Spec / section**: `19-tool-misc.md` §0 / §3.10-3.27 / §12.5
- **Change**: NONE NEEDED. Spec 19 already lists `TungstenTool` in §0 inventory (line 33), in §3.10-3.27 missing-source catch-all, in §4.1 inclusion-order pseudocode (line 453), and in §12.5 missing-source ledger (line 1407).
- **Verification**:
  - `find src -path "*Tungsten*"` returned no files — TungstenTool source is not present in the leak (only the registry citation at `tools.ts:60,215`).
  - Therefore spec 19's existing missing-source treatment is appropriate; can't write §3 prompt/schema for non-existent source.
- **DEPARTURE FROM CONSISTENCY REVIEW**: The review recommended "add a TungstenTool subsection" — but the source file doesn't exist in the leak, so that would require invention. The existing missing-source ledger treatment is correct.

### F7. Spec 08 §5.1 — registry-completeness footnotes
- **Spec / section**: `08-tool-base-registry.md` §5.1 (after the registry pseudocode)
- **Change**: Added a "Tools NOT in the flat `getAllBaseTools()` array but still real" subsection citing:
  - `AgentOutputTool`, `BashOutputTool` as aliases on `TaskOutputTool` (spec 15 §3 / `TaskOutputTool.ts:184`).
  - `McpAuthTool` as a per-server factory via `createMcpAuthTool()` (spec 16 §3).
  - `ReviewArtifactTool` as a flag-gated tool registered via `src/components/permissions/PermissionRequest.tsx:36` (NOT through `tools.ts`); cross-references spec 19 §0 and `REVIEW_ARTIFACT` row in spec 00 §8.1.B.
- **Verification**:
  - `grep -rn ReviewArtifactTool src/` confirmed it's lazily required from `PermissionRequest.tsx:36` and gated by `feature('REVIEW_ARTIFACT')`. The directory `src/tools/ReviewArtifactTool/` is not present in the leak (the require path exists but file is missing).
  - Spec 19 §0 already lists `ReviewArtifactTool` correctly with the same registration site citation.

### F8. SKIPPED — Spec 21c "Gate" column
- **Reason**: Already present.
- **Detail**: Spec 21c §1 (line 13) already has a 4-column table whose first column is `Flag` (which IS the gate). Each §3 sub-section header also titles the command with its flag (e.g. `### 3.1 /proactive — Proactive mode (PROACTIVE ∨ KAIROS)`).

### F9. Spec 14 ↔ 30 handoff sentences
- **Spec / section**: `14-tool-agent-team.md` §1 (just before OUT-of-scope) and `30-coordinator-multiagent.md` §1 (just before OUT-of-scope)
- **Change**: Added a one-paragraph "Handoff with spec 30/14" boundary statement in each spec, explicitly assigning:
  - Spec 14 = AgentTool tool surface (input schema, prompt assembly, `checkPermissions`, registry wiring).
  - Spec 30 = runner (`runAgent.ts` body, `forkSubagent.ts` execution, coordinator orchestration, `builtInAgents.ts`).
  - `forkSubagent.ts` cited by both: spec 14 = gate predicate only (`isForkSubagentEnabled`); spec 30 = fork-message construction and execution.
- **Verification**: Cross-checked spec 14 §1 IN-scope (lines 12-19) and spec 30 §1 IN-scope (line 12). Both DO list `forkSubagent.ts`; the review's seam concern was correct.

### F10. Spec 29 ↔ 30 delegation bullets
- **Spec / section**: `29-service-memory.md` §1 (above "Out of scope") and `30-coordinator-multiagent.md` §1 (above the spec-14 handoff)
- **Change**:
  - Spec 29: bullet "Agent/away/toolUse/autoDream summary services live in spec 30; this spec owns memory service and MEMORY.md content only".
  - Spec 30: reciprocal bullet "Persistent memory writes belong to spec 29; this spec owns only the *summary* services which are scratch summarization runs, not memory writes."
- **Verification**: Spec 30 §1 (line 10) explicitly claims `services/AgentSummary/`, `services/awaySummary.ts`, `services/toolUseSummary/`, `services/autoDream/` IN scope. Spec 29 §1 IN scope is the three memory service dirs. The seam was correctly flagged.

### F11. Spec 00 §8.1.B — flag-row owner clarifications
- **Spec / section**: `00-overview.md` §8.1.B
- **Changes**:
  - `BUDDY` row: changed owner column from `42` to `42 (runtime) / 21c (command)`.
  - `PROMPT_CACHE_BREAK_DETECTION` row: changed owner column from `14 / 30` to `30 (cited by 14)`.
- **Verification**:
  - Spec 14 §1 lists `PROMPT_CACHE_BREAK_DETECTION` only at "flag-citation level here" — i.e. cites it but does not own.
  - Spec 30 §1 (line 12) and §5.12 own the cleanup logic at `runAgent.ts:824`.
  - Spec 21c §3.15 owns the `/buddy` command surface; spec 42 owns the runtime sprite/companion logic.

---

## Skipped fixes (verification disagreement)

| # | Item | Why skipped |
|---|---|---|
| F4 | Spec 00 §6.2 PermissionMode runtime list | Both spec 00 and spec 09 already match `src/types/permissions.ts:16-22`. The review's finding 4.1 was a misread — `dontAsk` IS in EXTERNAL_PERMISSION_MODES (line 20). |
| F8 | Spec 21c "Gate" column | Already present as the first column of the §1 master table (named "Flag"). |
| F6 | Spec 19 TungstenTool §3 subsection | Source file not present in the leak; writing §3 would require invention. Existing missing-source ledger is correct. |
| (review F2.2) | Rename `ScheduleCronTool` → `CronCreateTool` | Both names are valid. `ScheduleCronTool` is the directory name; the three exports are `Cron{Create,Delete,List}Tool`. Added a clarifying note instead of renaming. |
| (review F2.2) | "Add `ReviewArtifactTool` row to spec 08 registry table" | `ReviewArtifactTool` is NOT in `tools.ts` — it's registered via `PermissionRequest.tsx`. Added a footnote in spec 08 §5.1 explaining the off-registry registration site instead of inventing a row. |

---

## New findings (discovered during verification)

1. **`ReviewArtifactTool` source directory absent from leak.** Only the lazy require at `src/components/permissions/PermissionRequest.tsx:36` exists; the target file `src/tools/ReviewArtifactTool/ReviewArtifactTool.ts` is missing. Spec 19 §0 already records this correctly. (Verified by `ls src/tools/ReviewArtifactTool/` — no such directory.)

2. **`TungstenTool` source directory absent from leak.** `src/tools.ts:60` imports from `./tools/TungstenTool/TungstenTool.js`, but `find src -path "*Tungsten*"` returns nothing. Spec 19 §0/§12.5 already records this as missing-source.

3. **`types/permissions.ts` is explicitly a cycle-breaker mirror, NOT the canonical source.** The file's header comment (lines 1-7) says "Pure permission type definitions extracted to break import cycles ... no runtime dependencies." The `ToolPermissionContext` declared there at line 427 is intentionally a *strict subset* of the canonical Tool.ts type — it omits `isAutoModeAvailable`. Both copies are intentional; spec 09's prior framing of "the field is missing from the type" was wrong.

4. **`INTERNAL_PERMISSION_MODES` includes `'auto'` only when `feature('TRANSCRIPT_CLASSIFIER')`.** Confirmed at `permissions.ts:33-36`. This is correctly captured in spec 09 §4.1 and spec 00 §6.2 already — no fix needed.

5. **Spec 19's §0 inventory is the highest-quality registry mirror in the spec set** — every leak-missing tool has its `tools.ts` line citation, gating expression, and inclusion-line. Phase 10 reviewers should treat spec 19 §0 as the master "missing-source ledger" cross-reference.

---

## Phase 10 readiness

The spec set is **ready for Phase 10 enumeration sweep**. Remaining drift after these fixes is:
- Cosmetic (typos, redundant rows the review marked as "none required").
- The single intentional `ToolPermissionContext` duplication between Tool.ts and types/permissions.ts is now properly explained in both spec 08 and spec 09; no further reconciliation needed.
- All cross-spec seams flagged "Issue" in the consistency review (08↔09, 14↔30, 29↔30) now have explicit handoff/delegation paragraphs.
