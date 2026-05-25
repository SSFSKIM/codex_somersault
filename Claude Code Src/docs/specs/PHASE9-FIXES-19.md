# Phase 9.6c Fixes — Spec 19 (Tool Misc)

**Phase**: 9.6c (CRITICAL ripple from 9.6 B-mini spec-00 §2.5 Cron fix)
**Target**: `docs/specs/19-tool-misc.md`
**Adversarial input**: `docs/specs/PHASE9-ADVERSARIAL-19.md` (1 Critical, 3 Major, 4 Minor)

## Source verification (pre-edit)

```
$ ls src/tools/ScheduleCronTool/
CronCreateTool.ts  (158 lines)
CronDeleteTool.ts
CronListTool.ts
UI.tsx
prompt.ts
```

Runtime `isEnabled()` confirmed at:
- `CronCreateTool.ts:67-69` → `isKairosCronEnabled()`
- `CronDeleteTool.ts:46-48` → `isKairosCronEnabled()`
- `CronListTool.ts:48-50`  → `isKairosCronEnabled()`

Build-time gate confirmed at `tools.ts:29-35`
(`feature('AGENT_TRIGGERS') ? [...] : []`).
Single inclusion site at `tools.ts:235` (`...cronTools` spread).

## Fixes applied

### CRITICAL-1 — Cron source mis-classified as missing
- **§0 row L43** — flipped `Source present?` to `yes`; rewrote gate cell
  to two-layer (build-time `feature('AGENT_TRIGGERS')` DCE + runtime
  `isKairosCronEnabled()` per-tool `isEnabled()`) with exact line
  citations. Added a second HTML-comment naming-note on the two-layer
  pattern.
- **§2** — count of missing-source entries dropped from **18 → 15**;
  added explicit clause "The Cron triplet … is **not** missing — full
  source is in the leak". Aligns with spec 00 §2.5 post-Phase-9.6 B-mini.
- **§3.10-3.27 placeholder** — replaced with a concrete **§3.10
  ScheduleCron triplet** subsection covering: Citation, Two-layer gate
  (mirrors §3.3 BriefTool), CronCreate input schema (`cron`/`prompt`/
  `recurring`/`durable`), output schema (`{id, humanSchedule, recurring,
  durable?}`), the four `validateInput` errorCodes (1: invalid cron, 2:
  no-match-in-year, 3: `MAX_JOBS = 50`, 4: durable+teammate refusal),
  `call()` (addCronTask + setScheduledTasksEnabled), CronDelete schema
  + teammate-ownership errorCode 2, CronList schema + teammate
  filtering. Cross-spec links to 21c (`/cron`), 32 (Kairos), 35 (UDS).
  Renumbered tail header to "§3.11-3.25 Missing-source tools".
- **§4.1 inclusion-order comment L470** — `// tools.ts:235  - missing
  source` → `// tools.ts:235  - present (CronCreate/Delete/List, see
  §3.10)`.
- **§12 ledger** — removed three rows 12.15/12.16/12.17 (the Cron
  triplet); promoted old 12.18 (ReviewArtifactTool) to 12.15. Added
  explicit migration HTML-comment so future readers know the
  renumbering. Updated trailing notes paragraph: dropped the
  "12.15 spreads multiple tools" bullet, replaced with a note that the
  triplet is now in §3.10 with a single `...cronTools` inclusion site.

### MAJOR-1 — `isEnabled()` gate misstated
Resolved as part of CRITICAL-1: §0 row and new §3.10 both document the
two-layer gate (build-time `feature('AGENT_TRIGGERS')` ternary +
runtime `isKairosCronEnabled()`), with `isDurableCronEnabled()` called
out separately as a narrower kill-switch for durable persistence
(`CronCreateTool.ts:120` `effectiveDurable = durable && isDurableCronEnabled()`).
Mirrors the §3.3 BriefTool "Build-time entitlement / Runtime entitlement"
structure verbatim.

### MAJOR-2 — §13.3 vs §12 internal contradiction
Resolved by: (a) removing the contradiction at the source (§12 no
longer claims missing); (b) rewriting §13.3 as a 5-row sub-file catalog
(CronCreate / CronDelete / CronList / prompt.ts / UI.tsx) anchored to
§3.10 as the canonical body. CronCreate is now a row (was previously
"enumerated in the main spec body" only).

### MAJOR-3 — Citation drift (3 rows for 1 inclusion site)
Resolved by collapsing §12.15/16/17 into a single statement in §3.10
("single spread `...cronTools` — there is one inclusion line, not
three") and by removing the three §12 rows. The §0 row continues to
cite `tools.ts:29-35` for the build-time array literal and `:235` for
the single inclusion.

### Minor-1..4
Triaged as either OK-as-is or out-of-scope cross-spec hygiene — no
edits needed in spec 19 itself. Cross-spec follow-ups already noted in
the adversarial review (spec 26 `tengu_brief_send` index, spec 31
`<TICK_TAG>` cross-link, spec 32 BriefTool cite-don't-redoc, spec 09
ReviewArtifact ownership) remain on the cross-spec todo list and are
not within Phase 9.6c's scope.

## Cross-spec ripple (no edits needed in this phase)

- **Spec 00 §2.5** — already correct post-Phase-9.6 B-mini.
- **Spec 21c** — `/cron` slash command spec; spec 19 §3.10 now
  cross-refs it for command surface.
- **Spec 26 / 31 / 32** — cross-ref instead of redoc (per task
  instructions); no spec-19 edits.
- **Spec 09** — ReviewArtifactTool permission renderer ownership; spec
  19 §12.15 remains a citation-only reference.

## Post-fix verdict

**Minor revise.** All 1 Critical and all 3 Major findings resolved
in-place with src-grounded edits. Spec 19 is now internally consistent
(§0 ↔ §3.10 ↔ §12 ↔ §13.3 all agree the Cron source is present), the
two-layer gate is documented to the same depth as §3.3 BriefTool, and
the registry citation drift is collapsed to one inclusion site. No
new claims were invented — every line cite was verified against the
ScheduleCronTool source files. Remaining items are minor cross-spec
hygiene (telemetry index in spec 26, Kairos cite-don't-redoc in spec
32) flagged but not owned by spec 19.
