# PHASE9-ADVERSARIAL-SUMMARY.md — Aggregate of 18 spec adversarial reviews

**Date:** 2026-05-09
**Phase:** 9.5 (adversarial review)
**Method:** 18 spec reviews × 2 model lenses (9 Claude Opus + 2 Codex CLI + 7 Opus fallback)
**Original design:** 9 Opus + 9 Codex (50/50 model diversity). **Actual:** 16 Opus + 2 Codex due to mid-flight Codex auth/rate-limit failures (`401 token_revoked` then `rate limit until 14:14 PM` after account switch).

---

## Per-spec verdict matrix

| Spec | Reviewer | Crit | High/Major | Med | Low | Verdict |
|---|---|---:|---:|---:|---:|---|
| 00 (overview) | Opus | 1 | 3 | 0 | 6 | minor revise |
| 01 (entrypoint) | Opus | 0 | 2 | 4 | 8 | accept w/ revisions |
| 02 (settings) | Opus | 0 | 1 | 4 | 9 | accept w/ minor revisions |
| 04 (turn pipeline) | **Codex** | 0 | **9** | 0 | 2 | **not-safe-as-rebuild-guide** |
| 05 (context) | Opus | 0 | 1 | 2 | 7 | approve w/ fixes |
| 08 (tool base) | Opus | 0 | 1 | 5 | 2 | accept w/ minor fixes |
| 09 (permissions) | Opus fallback | 0 | 2 | 0 | 4 | minor revise |
| 10 (bash) | Opus fallback | 0 | 1 | 4 | 5 | accept w/ minor revisions |
| 11 (files) | Opus fallback | 0 | 1 | 3 | 7 | accept w/ edits |
| 14 (agent-team) | Opus | 0 | 1 | 4 | 8 | accept w/ revisions |
| 17 (skill) | **Codex** | 0 | **3** | **6** | 3 | **needs major revision** |
| 20 (commands) | Opus | 0 | 0 | 2 | 7 | **ship-ready** ✨ |
| 21 (command catalog) | Opus | 0 | **3** | 4 | 5 | **needs revision** |
| 22 (api) | Opus fallback | 0 | 3 | 5 | 4 | accept w/ minor corrections |
| 26 (analytics) | Opus | 0 | 1 | 4 | 8 | accept w/ minor revisions |
| 30 (coordinator) | Opus fallback | 0 | 2 | 5 | 10 | accept w/ revisions |
| 34 (bridge) | Opus fallback | **1** | 3 | 5 | 4 | accept w/ fixes |
| 35 (remote) | Opus fallback | 0 | 2 | 4 | 7 | accept w/ revisions |
| **Total** | | **2** | **39** | **61** | **106** | **208 findings** |

**Verdict distribution:**
- 1 ship-ready (spec 20)
- 12 accept w/ fixes/minor revisions (00, 01, 02, 05, 08, 10, 11, 14, 22, 26, 35, 04)
- 2 minor revise (09, 30)
- 1 accept w/ critical-fix (34)
- 2 needs major revision (17, 21)

---

## Critical findings (2)

### CRITICAL-00 — Phantom missing-source claim
- **Spec:** 00 §2.5
- **Claim:** `ScheduleCronTool/{Cron*}Tool` listed as missing-source.
- **Reality:** `src/tools/ScheduleCronTool/` exists with all three tool files.
- **Impact:** Spec 19 inherits a false gap. Note: Phase 9.4 fix log (`PHASE9-FIXES-APPLIED.md`) already verified these exist; spec 00 §2.5 was NOT updated then. Phase 10 ripple gap.
- **Cross-spec:** 19.

### CRITICAL-34 — RPC miscount
- **Spec:** 34 §3.3
- **Claim:** "Eight RPCs" in BridgeApiClient.
- **Reality:** 9 RPCs in `types.ts:133-176` (heartbeat is 9th).
- **Impact:** Reimplementation tests would under-cover heartbeat path.
- **Cross-spec:** 33 (daemon's no-refresh heartbeat path obscured).

---

## High/Major findings (39 — 5 most important)

### HIGH-04-1 — Streaming ownership misplaced (codex)
- Spec 04 places `queryModelWithStreaming` in QueryEngine; src has it at `src/services/api/claude.ts:752`. Cross-spec impact on 03/22.

### HIGH-04-2 — `src/query/transitions.ts` cited but MISSING
- Spec 04 says inspected; `src/query.ts:104` imports `Terminal`/`Continue` from this missing path. **Path doesn't exist in leak.**

### HIGH-09-1 — `bubble` IS a runtime mode (codex partial → Opus triple-confirm)
- **Phase 9.4 inversion #6 confirmed false.** Spec 09 §3, §4.1, §12.1 say `bubble` is type-only placeholder. Verified at:
  - `src/tools/AgentTool/forkSubagent.ts:67` — `permissionMode: 'bubble'` on FORK_AGENT
  - `src/tools/AgentTool/runAgent.ts:443` — runtime `agentPermissionMode === 'bubble'` controls `shouldAvoidPrompts`
  - `src/tools/AgentTool/runAgent.ts:430-433` — writes `'bubble'` into `ToolPermissionContext.mode`
- **Phase 9.6 must restore `bubble` to spec 00 §6.2 + spec 09 §3/§4.1/§12.1.**

### HIGH-21-1 — 21d not in parent (Phase 10 ripple)
- Spec 21 sub-file partition table lists only 21a/21b/21c. INDEX.md registers 21d (76 commands), but spec 21 body wasn't updated. Parent's promise of complete enumeration via three sub-files is false.

### HIGH-17-1 — §11.5 bundled catalog incomplete (codex)
- Phase 10d's bundled-skills enumeration in spec 17 §11.5 is materially incomplete and falsely says every bundled file registers a skill. Two files (`claudeApiContent.ts`, `verifyContent.ts`) are pure data-inlining modules, not skills.

---

## Cross-cutting failure patterns (Phase 9.6 priorities)

### Pattern A — Phase 10 ripple incomplete (4 specs)
Phase 10.5 INDEX update added 5 catalog companions but **affected core specs' bodies were not updated**:
- Spec 21: 21d not in parent's sub-file table
- Spec 22: §12 Q5 still points to "spec 02/06" for model registry (now in 42a)
- Spec 26: §6.3 says "41 Datadog allow-list", verbatim list has 43
- Spec 35: §1 sub-modes list omits `/ultrareview`; §11 cross-refs missing 21d

**Phase 9.6 sweep**: For each catalog companion (21d, 37a/b/c, 42a), audit affected core specs' §1, §11, §12 sections for stale references.

### Pattern B — False enumerations (4 specs, 5 instances)
"Total N items" claim mismatches actual count:
- Spec 04: "12 terminal reasons" — actually 10 in `src/query.ts`
- Spec 21: "Total registered ~108" — arithmetic doesn't match (105/106/109)
- Spec 26: "Datadog allow-list 41 entries" — actually 43
- Spec 34: "Eight RPCs" — actually 9

**Phase 9.6 sweep**: Grep all "(\d+) (items|entries|RPCs|tools|commands|reasons|...)" in spec set, recount each against src.

### Pattern C — Build-time DCE vs runtime check confusion (3 specs)
Spec wording obscures whether mechanism is compile-time DCE (literal-string `"external" === 'ant'`) or runtime check (`process.env.USER_TYPE !== 'ant'`):
- Spec 14 §3.1, §6.7: `checkPermissions` reads as runtime ANT check, actually DCE'd
- Spec 02: `migrateFennecToOpus` has BOTH mechanisms (dual gate); spec only documents inner runtime gate
- Spec 01: `clientType` enum drift — `'sdk-py'` is env sentinel, stored is `'sdk-python'`

**Phase 9.6 sweep**: Per CLAUDE.md, the codebase uses both `feature('FLAG')` (DCE) and `process.env.USER_TYPE === 'ant'` (DCE). Audit each spec's gate descriptions for runtime-vs-build-time clarity.

### Pattern D — Off-by-one line citations (1+ spec)
Spec 01 has 5 files with line-counts off by +1 (inclusive-endpoint counting bug). Spec 22 has 30-line drifts. Likely pervasive across the spec set.

**Phase 9.6 sweep**: Sample 50 random "filename:N-M" citations from spec set, verify each. If failure rate >10%, do full audit.

### Pattern E — §12 Open Questions stale (3 specs)
§12 sections contradict their own §11 invariants or other sections, because fix iteration didn't reset them:
- Spec 09 §12.3 contradicts §4.6 (Phase 9.4 fix)
- Spec 11 §12.2 contradicts §11.2 (false alarm flag)
- Spec 35 §3.3 names `server-lock.json` but §12.5 defers; src has `server-sessions.json`

**Phase 9.6 sweep**: Re-verify each spec's §12 against current §1-§11 content. Mark resolved questions.

### Pattern F — Intentional asymmetry undocumented (3 specs)
Source has dual mechanisms with opposite/different semantics, both intentional, neither flagged at spec level:
- Spec 02 §5.4: arrays replace on write, concat-dedupe on read (same file, two customizers)
- Spec 11 NotebookEdit: PROMPT mentions `cell_number`, schema accepts only `cell_id`
- Spec 22: `updateUsage` keeps prior `service_tier`, `accumulateUsage` takes most-recent

**Phase 9.6 sweep**: For each spec, look for dual mechanisms and note asymmetry explicitly with reimplementer-hazard warning.

### Pattern G — Phase 9.4 inversions discovered post-hoc (1 spec confirmed)
Phase 9.4 consistency reviewer made 11 recommendations; 5 were inverted by source verification at fix-apply time. Phase 9.5 adversarial review found a **6th inversion not caught at Phase 9.4**:
- `bubble` IS a runtime permission mode for forked subagents (spec 09 + spec 00 both wrong post-Phase-9.4).

**Phase 9.6 must apply**: spec 00 §6.2 restore `bubble` to runtime list; spec 09 §3, §4.1, §12.1 acknowledge runtime use.

---

## Leak-external unfalsifiable contracts (Phase 9.7 appendix candidate)

**8+ specs** depend on contracts that cannot be verified from the leaked tree alone:

| # | Spec | Section | Contract | Why unfalsifiable |
|---|---|---|---|---|
| 1 | 00 | §5.3 | Two StatSig configs gating tool ordering for cache invariance | StatSig configs server-side |
| 2 | 05 | §5.3 | "Stale prefix-date wins over ~920K tokens cache_creation per midnight crossing" | Three-source date reconciliation, no telemetry |
| 3 | 08 | §5.1 | Tool ordering must match upstream StatSig config | Server-side cache breakpoint logic |
| 4 | 14 | §5.3 | `getBuiltInAgents` evaluation order with GrowthBook defaults | GrowthBook configs absent |
| 5 | 01 | §2.6 | Pre-`main.tsx:12` boot ordering via undocumented `bin/claude` shim | `package.json`/`bin/` absent |
| 6 | 26 | §6.6 | `to1PEventFormat` proto-field hoisting | `src/types/generated/events_mono/` absent |
| 7 | 11 | §6.4 | image base64 → token 0.125 ratio | Anthropic server-side tokenization |
| 8 | 35 | §5.10 | SSH reverse-forwarded unix-socket auth proxy | `src/ssh/` absent |
| 9 | 10 | §11.16 | `ANT_ONLY_*` DCE'd via Bun bundler constant-fold | Bun build config absent |
| 10 | 34 | §9.3/etc. | 4090/4091/4092 close codes are client-synthesized cross-module convention | Verifiable only by reading 3+ files simultaneously |
| 11 | 22 | §4 | Session-stable header latch invariant (cost-multiplier risk: 10×) | Requires exhaustive call-site enumeration |

**Phase 9.7 candidate:** Add a `00-overview.md §13 — Epistemic Boundary` section listing these as "spec set's true unknowns".

---

## Cost-multiplier invariants (high-blast-radius)

Two findings flagged invariants where a wrong implementation silently inflates cost ~10×:

- Spec 22 §4: header latch (busts prompt cache)
- Spec 05 §5.3: 920K-token midnight cache_creation

These should be elevated to "Critical implementation invariants" sections in their respective specs and cross-referenced from spec 06 (cost-token-tracking).

---

## Hidden src bugs / drift (3 specs)

The reviews surfaced bugs in **src itself** (not spec error):
- Spec 11 §6.4.A: NotebookEdit PROMPT references `cell_number`, but schema only accepts `cell_id`. **Stale prompt text in src.**
- Spec 34 H3: `isExpiredErrorType` uses substring match `'expired'||'lifetime'`, false-positive on future `lifetime_*` server types.
- Spec 22 §4: `updateUsage` vs `accumulateUsage` `service_tier` asymmetry has no test/doc.

These are not spec errors — they are observations about the source. Phase 9.7 should catalog these in a `BUGS-IN-SOURCE.md`.

---

## Reviewer-error catches (Phase 9.5 self-reflection)

Two cases where reviewers caught the **prompt's own errors**:
1. Spec 02 H1: Review prompt cited `src/services/configLoader.ts` which doesn't exist; spec 02 correctly maps to `src/utils/settings/`.
2. Spec 21 H2: Review prompt asserted Phase 9.4 added "Gate column"; spec 21c §1 already had it as the first column.

This is the verify-before-trust pattern working at meta-level: sub-agents declined to follow my brief when src disagreed.

---

## Phase 9.6 fix scope estimate

| Work category | Specs touched | Est. time |
|---|---:|---:|
| Critical fixes (00 phantom missing-source, 34 RPC count) | 2 | 15 min |
| `bubble` runtime restoration (Phase 9.4 inversion #6) | 2 (00, 09) | 30 min |
| Pattern A sweep (Phase 10 ripple) | 4 (21, 22, 26, 35) | 45 min |
| Pattern B sweep (false enumerations) | 4 (04, 21, 26, 34) | 30 min |
| Pattern C sweep (build-time vs runtime) | 3 (14, 02, 01) | 40 min |
| Pattern D sweep (off-by-one line citations) | 1+ (sample) → potentially 18 | 1-3h |
| Pattern E sweep (§12 Open Questions) | 3 (09, 11, 35) + audit all 18 | 1h |
| Pattern F sweep (intentional asymmetry) | 3 (02, 11, 22) | 30 min |
| Per-spec High/Med findings (39 high + 61 med) | varies | 1.5-2h |
| **Total Phase 9.6 estimate** | ~14 spec edits + 7 sweeps | **3-5h** |

---

## Status

- ✅ All 18 reviews on disk (`PHASE9-ADVERSARIAL-NN.md`, total 1740 lines)
- ✅ This summary aggregated
- 🔜 Phase 9.6: fix iteration
- 🔜 Phase 9.7: final pass + leak-external contracts appendix + bugs-in-source catalog

---

## Sister documents

- `PHASE9-COVERAGE.md` — initial residual diff
- `PHASE9-CONSISTENCY.md` — internal drift findings
- `PHASE9-FIXES-APPLIED.md` — Phase 9.4 fix log (5 inverted recommendations)
- `PHASE10-CLEANUP.md` — Phase 10d residual cleanup
- `PHASE10-COVERAGE.md` — 100% basename coverage milestone
- `PHASE9-ADVERSARIAL-NN.md` — per-spec findings (18 files)
