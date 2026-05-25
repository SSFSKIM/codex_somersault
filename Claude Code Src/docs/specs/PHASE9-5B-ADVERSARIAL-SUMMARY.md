# PHASE9-5B-ADVERSARIAL-SUMMARY.md — Aggregate of 25 spec adversarial reviews

**Date:** 2026-05-10
**Phase:** 9.5b (adversarial review of remaining 25 specs not in Phase 9.5's 18-spec scope)
**Method:** 25 parallel Opus sub-agents (codex was rate-limited so all 25 dispatched as Opus)
**Trigger:** Phase 9.5 covered 42% of specs (18/43). Phase 9.5b closes the gap to 100% adversarial coverage.

---

## Per-spec verdict matrix

| Spec | Crit | High/Major | Med | Low | Verdict |
|---|---:|---:|---:|---:|---|
| 03 (query-engine) | 0 | 2 | 8 | 0 | minor revise |
| 06 (cost-token-tracking) | 0 | 2 | 5 | 3 | pass w/ fixes |
| 07 (context-compaction) | 0 | 1 | 3 | 2 | approved w/ minor revisions |
| 12 (tool-search) | 0 | 0 | 3 | 7 | acceptable w/ fixes |
| 13 (tool-web) | 0 | 1 | 4 | 8 | accept w/ minor revisions |
| 15 (tool-tasks) | 0 | 1 | 3 | 5 | accept w/ amendments |
| 16 (tool-mcp-lsp) | 0 | 0 | 2 | 1 | **APPROVE (cleanest)** ✨ |
| 18 (tool-modes) | 0 | 0 | 2 | 7 | approved w/ minor revisions |
| **19 (tool-misc)** | **1** | 3 | 4 | 0 | **MAJOR REVISE** ⚠️ |
| 23 (service-mcp) | 0 | 2 | 3 | 4 | approve w/ patches |
| 24 (service-lsp) | 0 | 1 | 3 | 5 | approved w/ minor fixes |
| 25 (service-oauth-auth) | 0 | 2 | 3 | 6 | pass w/ qualifications |
| 27 (service-policy) | 0 | 1 | 4 | 5 | accept w/ revisions |
| 28 (service-plugins) | 0 | 1 | 3 | 1 | accept w/ minor revisions |
| 29 (service-memory) | 0 | 1 | 3 | 6 | accept w/ minor fixes |
| 31 (mode-proactive) | 0 | 0 | 3 | 4 | accept w/ amendments |
| 32 (mode-kairos) | 0 | 1 | 2 | 7 | pass w/ corrections |
| 33 (mode-daemon) | 0 | 1 | 3 | 3 | approve w/ minor revisions |
| 36 (mode-voice) | 0 | 2 | 5 | 2 | accept w/ minor revisions |
| 37 (ink-ui-shell) | 0 | 2 | 3 | 3 | pass w/ fixes |
| 38 (output-styles) | 0 | 1 | 3 | 6 | approve w/ minor revisions |
| 39 (vim-keybindings) | 0 | 2 | 3 | 5 | approve w/ fixes |
| 40 (persistent-memory) | 0 | 1 | 3 | 7 | high-fidelity |
| 41 (session-state-history) | 0 | 1 | 3 | 6 | accept w/ minor revisions |
| **42 (misc)** | **1** | 2 | 2 | 2 | **APPROVE w/ ONE CRITICAL FIX** ⚠️ |
| **Total** | **2** | **31** | **75** | **104** | **212 findings** |

**Verdict distribution:**
- 1 cleanest review (16 — comparable to spec 20 in Phase 9.5)
- 22 accept-w/-fixes / approve-w/-minor-revisions
- 2 needs major revision: **19** (phantom ScheduleCronTool ripple), **42** (Phase 9.7 deferral unfulfilled)

---

## Critical findings (2)

### CRITICAL-19 — Phase 9.6 B-mini ripple incomplete
- **Spec:** 19 §0, §2, §12.15-17
- **Claim:** ScheduleCronTool/{CronCreate,CronDelete,CronList}Tool listed as missing-source.
- **Reality:** `src/tools/ScheduleCronTool/` fully present (`CronCreateTool.ts` 158 lines complete). `CronDelete/CronList` similarly.
- **Impact:** Phase 9.6 B-mini fixed spec 00 §2.5 phantom but **did NOT ripple to spec 19**.
- **§13.3 (Phase 10d) vs §12 self-contradiction**: §13.3 catalogs files with concrete behavior (i.e., source read evidence), yet §12 says missing.
- **Cross-spec impact:** spec 32 has same phantom (RemoteTriggerTool — see HIGH-32 below).

### CRITICAL-42 — Phase 9.7 SandboxManager deferral unfulfilled
- **Spec:** 42 (no sandbox/SandboxManager mentions; verified via grep)
- **Background:** Phase 9.7 spec 35 fix added explicit OUT-of-scope line: SandboxManager / `/sandbox-toggle` / `src/main.tsx:201,314-316` deferred to spec 42 with §11 cross-ref.
- **Reality:** Spec 42 contains zero mentions. Backward link broken.
- **Impact:** Cross-spec promise unfulfilled; either spec 42 must add §13 sandbox section, or spec 35 must retract the deferral, or new 42b-sandbox.md created.

---

## Cross-cutting failure patterns identified (Phase 9.6c priorities)

### Pattern A2 — Phase 9.6/9.7 ripple incomplete (4+ specs)
B-mini fixed spec 00 ScheduleCronTool phantom, but ripple didn't reach:
- **Spec 19**: same phantom for Cron tools (CRITICAL-19)
- **Spec 32**: RemoteTriggerTool same pattern (HIGH-32-1)
- **Spec 42**: SandboxManager deferral not fulfilled (CRITICAL-42)
- **Spec 41**: bubble runtime mode not cross-referenced (toExternalPermissionMode flatten)
- **Spec 03**: snipCompact/snipProjection self-contradiction (§2.5 vs §2.2)
- **Spec 37**: chalk truecolor patch + React Compiler runtime not in core (only in 37c/37a catalog)
- **Spec 17**: built-in plugin skill masquerade documented but currently 0 plugins (spec 28 finding)

### Pattern B2 — False enumerations (2 more specs found)
Phase 9.5/9.6 found 4 (specs 04, 21, 26, 34). Phase 9.5b adds:
- **Spec 39**: KEYBINDING_ACTIONS "78 entries" — actual **86** (§6.3 + §11)
- **Spec 29 / 26 update**: Datadog allow-list 44 → likely **46** (`tengu_team_mem_secret_skipped` + `tengu_team_mem_push_suppressed` not in allow-list)

### Pattern D2 — Off-by-one line citations (2 more specs found)
Phase 9.7 sweep declared 0/211 drift; Phase 9.5b finds spec-specific:
- **Spec 16**: `MCPTool.ts "1-78"` (actual 77), `LSPTool.ts "1-861"` (actual 860)
- **Spec 29**: 10 paths in §2.1 all +1 systematic (spec counts trailing newline; `wc -l` does not)

### Pattern F2 — Intentional asymmetries undocumented (2 more specs)
- **Spec 24**: `Object.assign` later-wins vs `extensionMap.push` earlier-wins conflated
- **Spec 39**: "context priority" illusion — `resolveKeyWithChordState` wraps in `new Set()`, last-binding-wins; ordering only in `useKeybinding(s)`

### Pattern G2 — Cross-spec invalidation (Phase 9.6 fix wrong)
- **Spec 33 reviewer found Phase 9.6 spec 34 fix is OVERSTATED**: "daemon callers without `onAuth401` → immediate `BridgeFatalError`" is wrong. `bridgeApi.ts:117-120` *returns* the 401; fatal happens downstream in `handleErrorStatus`. **Phase 9.6 inversion #7 candidate**.

---

## BUGS-IN-SOURCE.md candidates (new from Phase 9.5b)

Adding to existing 5 entries from Phase 9.6/9.7. Each verified by sub-agent:

| # | Source | Description | Severity |
|---|---|---|---|
| 6 | `WebSearchTool.ts:237-242` | Unreachable `Error: Missing query` validator (Zod min(2) preempts) | minor |
| 7 | `utils.ts:199-202` + `:407-413` | logError double-emit on domain check failures | minor |
| 8 | `services/lsp/index.ts:?` | Graceful-close zombie state (connection.onClose doesn't invoke onCrash; only process.exit code !== 0 does) | major |
| 9 | `services/policyLimits/index.ts:?` | checksum localeCompare wrong (Python sort_keys=True uses code-point, not locale-aware) — silent policy-skip risk | major |
| 10 | `types/plugin.ts:177` + `:220` | `lsp-config-invalid` declared TWICE; TS dedups silently | minor |
| 11 | `outputStyles.ts:158` | Stale comment claims "managed, user, project" — actual array is `[plugin, user, project, managed]` (managed wins) | cosmetic |
| 12 | `keybindings/schema.ts:12-32` | `Scroll` context defined in defaultBindings.ts:196 but missing from KEYBINDING_CONTEXTS (Zod rejects user override) | minor |
| 13 | `services/extractMemories/index.ts:935` + `watcher.ts:112` | Two emit sites (`tengu_team_mem_secret_skipped`, `tengu_team_mem_push_suppressed`) not in Datadog allow-list | cosmetic |
| 14 | `cli/print.ts:3879-3882` | `set_proactive` control-protocol payload is raw cast, not Zod-validated | minor |
| 15 | `AppState.tsx:150` | `false && state === selected` identity-selector guard — dead branch in external builds | nit |
| 16 | `NotebookEditTool.ts` | (was already in BUGS) `cell_number` 3× references in PROMPT, schema only accepts `cell_id` — confirmed by spec 11 in Phase 9.6 |

---

## High/Major findings (selected — 31 total)

### Security-relevant
- **Spec 25 H1** — `isExpiredErrorType` substring fragility undocumented in spec 25 (spec 34 found this; spec 25 (security spec) doesn't reference it). Server adding `feature_expired` / `quota_lifetime_exceeded` would silently downgrade unrelated 4xx to "info".
- **Spec 25 H2** — `withOAuthRetry`'s `!deps.onAuth401 → return response` early-exit not flagged. Bridge clients without wired `onAuth401` silently turn 401s into BridgeFatalError. (Note: spec 33 found this fix's wording is wrong — see Pattern G2.)
- **Spec 27 F1** — TWO independent `resetSettingsCache` paths; spec collapses them. **Reset-window timing = moment policy-blocked features stop being permitted** — race-relevant.
- **Spec 27 §5.4** — Confirmed src bug (#9 above): `policyLimits` uses `localeCompare` for Python `sort_keys=True` checksum — wrong sort order. **Silent policy-skip risk**.
- **Spec 40 F3** — `isAutoMemPath` SECURITY comment dropped (`paths.ts:274-278` `..` bypass normalize). Reimplementer could reintroduce path traversal.

### Architectural / cross-spec
- **Spec 19 CRIT** — phantom missing-source for ScheduleCronTool family
- **Spec 42 CRIT** — SandboxManager deferral unfulfilled
- **Spec 23 H2** — `ElicitationDialog.tsx` 1168 lines never named; spec 37 (UI) ↔ spec 23 cross-ref missing
- **Spec 33 H1** — Phase 9.6 spec 34 fix wording wrong (Pattern G2)
- **Spec 36 F2** — "Auto-pause on terminal blur" is dormant in REPL (focus-mode only; push-to-talk has focusMode:false hardwired)
- **Spec 37 H1+H2** — React Compiler runtime + chalk truecolor patch not in core spec (only in 37a/37c)
- **Spec 41 HIGH** — `/rewind` slash command path invisible (only `--rewind-files` CLI documented)

---

## Hardest-to-verify claims discovered (Phase 9.7 §13 leak-external addendum candidates)

Adding to existing 11 entries:

| # | Spec | Section | Contract | Why unfalsifiable |
|---|---|---|---|---|
| 12 | spec 06 | §10 inv 8 | "side-query/classifier costs intentionally excluded" | Verifiable only by trusting comment in `utils/permissions/permissions.ts:766` |
| 13 | spec 13 | §6 web search | Foundry "ships only models that already support Web Search" | No foundry model registry in leak |
| 14 | spec 18 | §5.2 #4 | Read isAutoModeActive() BEFORE setAutoModeActive() (3 state stores + temporal invariant) | Cross-spec to 09 needed for transitionPlanAutoMode boundary |
| 15 | spec 23 | §5.6 | URL-elicitation retry+throw invariant (3 files reading required) | Multi-file convention not in single source |
| 16 | spec 24 | §9 | `withTimeout` finally-clearTimeout "avoids orphaned setTimeout" | Weaker than stated (timer fires once anyway) |
| 17 | spec 25 | §6.13 | Bun native HTTP `cch=00000` same-length rewrite | `bun-anthropic` not in leak |
| 18 | spec 27 | §5.4 | `notifyChange('policySettings')` resets cache BEFORE iterating listeners | Order-of-operations security invariant; needs body of changeDetector |
| 19 | spec 28 | §11 inv 9 | Version-recompute "re-clone-forever silent loop" risk | `calculatePluginVersion` + `probeSeedCache` unread |
| 20 | spec 31 | §5.3-5.4 | Pause/resume + context-block state machine | `proactive/index.js` absent |
| 21 | spec 33 | §3.3/§8 | BG_SESSIONS-off + DAEMON-on → daemon workers register as 'interactive' | `daemonMain` absent — observable user-facing bug |
| 22 | spec 36 | §7.2 | finishRecording silent-drop-replay 8-condition guard + 250ms backoff replay | State spans multiple closures across 378 LOC |
| 23 | spec 39 | §6.3 / types | `KeybindingContextName` broader than schema enum or unsafe cast | `src/keybindings/types.ts` absent |
| 24 | spec 40 | §5.4 | `-1`-age recall-shape telemetry semantics | `memoryShapeTelemetry.ts` missing |
| 25 | spec 41 | §5.6 inv 6 | "zero stale usage on preserved assistants — otherwise resume autocompacts on ~190K context" | Magic figure spans 4 subsystems |

---

## Coverage 100% — adversarial verification complete

| Phase | Specs reviewed | Cumulative |
|---|---:|---|
| Phase 3 (initial) | 1 (spec 03) | 1/43 = 2% |
| Phase 9.5 | 18 | 19/43 = 44% |
| Phase 9.5b | 25 | **44/43 = 100%** ✅ |

(Spec 03 reviewed twice: once in Phase 3 codex review, once in Phase 9.5b for re-verification post-Phase-9.6 fixes.)

---

## Phase 9.6c fix scope estimate

| Work | Specs touched | Est. time |
|---|---:|---:|
| Critical fixes (19, 42 + ripple from 32) | 3 | 30 min |
| Pattern A2 ripple (37 R-Compiler + chalk; 41 bubble cross-ref; 03 snipCompact self-fix; 17 builtin plugin caveat) | 4 | 45 min |
| Pattern B2 false enumerations (39 86, 26 → 46) | 2 | 15 min |
| Pattern G2: spec 34 wording correction (Phase 9.6 inversion #7) | 1 (34) | 15 min |
| BUGS-IN-SOURCE.md update with 11 new entries | (BUGS file) | 30 min |
| Per-spec High/Med findings | varies | 1.5-2h |
| **Total Phase 9.6c estimate** | ~10 spec edits + cross-cutting fixes + BUGS update | **3-4h** |

---

## Status

- ✅ All 25 Phase 9.5b reviews on disk (`PHASE9-ADVERSARIAL-NN.md`, total 2810 lines)
- ✅ This summary aggregated
- 🔜 Phase 9.6c: fix iteration (priority: 2 critical + 7+ high)
- 🔜 BUGS-IN-SOURCE.md update with 11 new candidates

## Sister documents

- `PHASE9-ADVERSARIAL-SUMMARY.md` — Phase 9.5 (18 specs)
- `PHASE9-ADVERSARIAL-NN.md` — per-spec findings (now 43 files: 18 + 25)
- `PHASE9-COVERAGE.md`, `PHASE9-CONSISTENCY.md`, `PHASE9-FIXES-APPLIED.md`
- `PHASE9-FIXES-NN.md` — Phase 9.6 fix logs (11 specs from B-full)
- `PHASE9-FIXES-OPEN-QUESTIONS.md` — Phase 9.7 §12 sweep
- `PHASE9-LINE-CITATION-AUDIT.md` — Phase 9.6 Pattern D audit
- `PHASE9-PATH-CANONICALIZATION.md` — Phase 9.7 path-prefix
- `BUGS-IN-SOURCE.md` — src bugs catalog (5 entries; +11 from Phase 9.5b → 16 total candidates)
- `PHASE10-CLEANUP.md`, `PHASE10-COVERAGE.md`
