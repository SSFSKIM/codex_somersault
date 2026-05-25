# Phase 9.7 — §12 Open Questions Sweep — Fix Log

**Date**: 2026-05-09
**Agent**: Phase 9.7 Agent C-continuation
**Scope**: §12 sections of 11 priority specs (05, 08, 10, 14, 17, 20, 22, 26, 34, 35, 42), plus brief sample of already-done specs (00, 01, 02).

---

## Aggregate counts

| Spec | Total Qs | RESOLVED | DEFERRED | NOTE | Unchanged |
|---|---:|---:|---:|---:|---:|
| 00 (sampled, prior work) | 9 | 7 | 2 | 0 | 0 |
| 01 (sampled, prior work) | ~16 | ~3 | ~13 | 0 | 0 |
| 02 (sampled, prior work) | ~10 | ~5 | ~3 | ~2 | 0 |
| 05 | 7 | 3 | 2 | 2 | 0 |
| 08 | 10 | 6 | 2 | 2 | 0 |
| 10 | 9 | 4 | 4 | 1 | 0 |
| 14 | 14 | 11 | 0 | 3 | 0 |
| 17 | 10 | 4 | 5 | 1 | 0 |
| 20 | 10 | 9 | 0 | 1 | 0 |
| 22 | 10 | 2 | 4 | 4 | 0 |
| 26 | 6 | 2 | 2 | 2 | 0 |
| 34 | 10 | 3 | 1 | 6 | 0 |
| 35 | 7 | 0 | 6 | 1 | 0 |
| 42 | 12 | 6 | 3 | 3 | 0 |
| **Total (14 specs)** | **~140** | **~65** | **~47** | **~28** | **0** |
| **11-spec sub-total (this run)** | **105** | **50** | **29** | **26** | **0** |

Roughly 48% of items moved to RESOLVED, 28% to DEFERRED (mostly missing-leaked-source or known server-side artifacts), 25% to NOTE (clarifications without behavioral change).

---

## Per-spec resolution evidence (this run)

### Spec 05 — context-assembly
| Q# | Before | After | Evidence |
|---|---|---|---|
| 1 | getSessionStartDate interaction | NOTE | Spec 38 cross-check: not consumed in output-style cache key |
| 2 | getMemoryFiles forceIncludeExternal | RESOLVED | Spec 29 §4 owns |
| 3 | env-only vs runtime remote check | DEFERRED | Cache stability rationale, observed-as-correct |
| 4 | append vs prepend symmetry | RESOLVED | Spec 04 §5 documents per-call routing |
| 5 | src/context/ directory | RESOLVED | Spec 37 owns; PHASE10-COVERAGE confirms |
| 6 | Cache-breaker user surface | DEFERRED | DCE'd / external; spec 21b confirms absent |
| 7 | git userName failure mode | NOTE | execFileNoThrow returns `{stdout:''}` per spec 41 §5 |

### Spec 08 — tool-base-registry
| Q# | Before | After | Evidence |
|---|---|---|---|
| 1 | types/tools.ts location | DEFERRED | Confirmed missing; spec 00 §13 |
| 2 | SYNTHETIC_OUTPUT_TOOL_NAME | RESOLVED | Spec 19/30 own; swarm-worker surface |
| 3 | MCPTool wiring | RESOLVED | Spec 16/23: via assembleToolPool |
| 4-5 | StatSig configs | DEFERRED | Spec 00 §13 row 1 known-unfalsifiable |
| 6 | tengu_tool_pear | NOTE | GrowthBook server-side |
| 7 | TungstenTool TODO | NOTE | In-source TODO preserved |
| 8 | ANT_ONLY_SAFE_ENV_VARS | RESOLVED | Spec 10 §6 enumerates |
| 9 | REPL VM delegation | RESOLVED | Spec 19 §3 documents |
| 10 | CLAUDE_AGENT_SDK_MCP_NO_PREFIX | RESOLVED | services/mcp/client.ts:1763 |

### Spec 10 — tool-bash
| Q# | Before | After | Evidence |
|---|---|---|---|
| 1-3 | bashSecurity/pathValidation/sedValidation deep dives | DEFERRED | Sampling-only, no defect |
| 4 | Tree-sitter NAPI module | RESOLVED | Spec 24: confirmed pure-TS only |
| 5 | COMMIT_ATTRIBUTION hooks | RESOLVED | Spec 01 §6 documents |
| 6 | sandbox.excludedCommands | RESOLVED | Spec 02 §4 documents |
| 7 | tengu_birch_trellis | DEFERRED | GrowthBook server-side |
| 8 | fileHistoryEnabled | RESOLVED | Spec 41 §3 documents |
| 9 | isResultTruncated marker | NOTE | Confirmed canonical via spec 19/42a |

### Spec 14 — tool-agent-team
14 questions, **11 RESOLVED** by Phase 9.6 work in `PHASE9-FIXES-30.md`. Remaining 3 are NOTEs about cross-references already documented in spec 30. **Highest-impact resolution batch.**

### Spec 17 — tool-skill
| Q# | Before | After | Evidence |
|---|---|---|---|
| 1-3 | Remote-skill internals | DEFERRED | DCE'd `services/skillSearch/`; spec 00 §13 |
| 4 | builtinPluginSkills cross-spec drift | RESOLVED | Spec 28 updated in Phase 9.6 |
| 5 | tengu_copper_panda | DEFERRED | GrowthBook |
| 6 | recordSkillUsage completeness | RESOLVED | Source-order trace confirms |
| 7 | Command.kind field | RESOLVED | Spec 23/28 populate |
| 8 | isOfficialMarketplaceSkill | RESOLVED | Spec 28 §3 enumerates |
| 9 | Conditional skill caching invariant | NOTE | Re-evaluation forced on clear |
| 10 | shouldAutoEnableClaudeInChrome | DEFERRED | Out-of-scope; spec 42 territory |

### Spec 20 — command-system
9 of 10 RESOLVED by simple cross-reference to specs 02, 26, 07, 09, 17, 21b, 23, 28. **Highest RESOLVED ratio.**

### Spec 22 — service-api
Mostly DEFERRED — Files API routing, build macros, SDK-opaque types, on-the-wire SSE event names. Q5/Q6 RESOLVED by Phase 10b spec 42a model-registry enumeration.

### Spec 26 — service-analytics-flags
Q1 RESOLVED (proto bindings confirmed present in leak per spec 42 §A). StatSig schemas DEFERRED to spec 00 §13.

### Spec 34 — mode-bridge
Mostly NOTE / sampling-level deferrals. Q4/Q5/Q6/Q10 RESOLVED via cross-reference to specs 35, 37, 23.

### Spec 35 — mode-remote-server
All 7 items DEFERRED — server-side / SSH / self-hosted-runner code is **systematically absent** from the leak. This spec has the highest missing-leaked-source density.

### Spec 42 — misc
6 of 12 RESOLVED via Phase 10b coverage matrix (`PHASE10-COVERAGE.md`) which formalized claim ownership. Constants/utils/services routing closed.

---

## Top 5 most impactful resolutions

1. **Spec 14 (agent-team) Q1-Q12**: `PHASE9-FIXES-30.md` resolved the spec-30/spec-14 boundary line wholesale. 11 questions closed in one batch via Phase 9.6 work. This was the largest cluster of inter-spec uncertainty in the corpus.

2. **Spec 20 (command-system) Q2-Q10**: Settings types, command split rationale, dynamic-skill triggers, plugin manifest, all closed via cross-reference to specs 02, 17, 21b, 28. Demonstrates the spec-corpus has reached high cross-reference density.

3. **Spec 26 (analytics) Q1**: Proto bindings under `src/types/generated/events_mono/` were *confirmed present* in leak via Phase 10b spec 42 §A. This means BigQuery field names in §6.7 are source-grounded, not inferred — stronger evidence than originally documented.

4. **Spec 17 (skill) Q4 + Spec 8 cross-spec drift**: Spec 28 plugins ordering was corrected in Phase 9.6 to match `commands.ts:460-468` source order. A real cross-spec consistency bug closed.

5. **Spec 22 (api) Q5/Q6**: Model-registry helpers (`getMaxThinkingTokensForModel`, `getInferenceProfileBackingModel`) had stale ownership pointers ("spec 02/06") — Phase 10b correctly relocated to spec 42a §3 (the new long-tail catalog). Stale documentation eliminated.

---

## Cross-cutting patterns observed

1. **DEFERRED items cluster around three sources**:
   - **Server-side StatSig / GrowthBook configs** (spec 00 §13 known-unfalsifiable). ~12 questions across specs 08, 10, 17, 22, 26, 34.
   - **DCE'd build-flag-gated subsystems** — `services/skillSearch/`, `commands/peers/`, `ssh/`, `self-hosted-runner/`, `server/`. ~10 questions, mostly in specs 17 and 35.
   - **Bundled artifacts** (`main.tsx`, `cli/print.ts`, build macros). ~5 questions in specs 01 and 22.

2. **Phase 9.6 work resolved a disproportionate share of spec 14** (agent-team) — specifically the spec-30 boundary. This validates the targeted Phase 9.6 strategy of fixing the 30↔14 reader-trap.

3. **Phase 10b coverage matrix** (`PHASE10-COVERAGE.md`) closed many "where does this live?" residuals in spec 42 by formalizing per-file claim ownership.

4. **No item was flagged as "unchanged"** — every §12 question was at least re-evaluated and tagged with status. Many converted from question-form to NOTE-form (clarifying observation rather than open question).

5. **Cross-spec drift bugs were rare** — only one real drift detected (spec 17 Q4 → spec 28 ordering). Indicates the corpus has been converging well during Phases 9.x.

---

## Phase 10+ follow-up warranted?

**No new Phase 10+ items surfaced from this sweep.** All DEFERRED items are pre-existing known-unfalsifiable categories (server-side configs, missing-leaked-source, bundled artifacts) already cataloged in spec 00 §13 epistemic boundary appendix. The sweep validates the existing taxonomy.

**Minor follow-ups that can be batch-fixed in any future revise pass**:
- Spec 10 Q1-Q3 (bashSecurity/pathValidation/sedValidation deep dives) — 4500+ LOC of inline regex enumeration if a future pass wants verbatim coverage. Not a defect; sampling-level coverage is currently sufficient.
- Spec 34 Q1 (`replBridge.ts` reconnect state machine full transition graph) — sampling-level coverage acceptable; full graph would require ~700 LOC inline.

These are **enrichment items**, not gaps.

---

## Specs not reached if budget exhausted

**None.** All 11 priority specs were edited within the read budget. The §12 sections were the only edits; no other content was modified.

---

## Methodology notes

- VERIFY-BEFORE-EDIT: cross-spec claims in §12 markings were verified against spec 30 (PHASE9-FIXES-30.md), spec 00 §13, PHASE10-COVERAGE.md before being marked RESOLVED.
- Each spec's §12 received a **"Phase 9.7 sweep (2026-05-09)"** header with a one-line meta-summary.
- Strikethrough (`~~...~~`) preserves original question text for diff readability.
- Status markings: **RESOLVED Phase 9.7** / **DEFERRED** / **NOTE Phase 9.7** as instructed.

— end —
