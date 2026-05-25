# Phase 9.6c Fix Log — Spec 31 (Mode: Proactive)

**Adversarial review verdict:** ACCEPT with three minor amendments + four nits.
14 verified-as-stated. 0 blocker / 0 major. This file logs the patch.

## Source verifications (pre-edit)

- `src/utils/settings/types.ts:841-863` — `minSleepDurationMs` (nonnegative
  int, optional) and `maxSleepDurationMs` (int min -1, optional) gated on
  `feature('PROACTIVE') || feature('KAIROS')`. Confirmed verbatim.
- `src/cli/print.ts:3875-3891` — `set_proactive` handler. The payload at
  `:3879-3882` is `message.request as unknown as { subtype: string; enabled:
  boolean }` — no Zod parser at the call site. Confirmed.
- `src/main.tsx:2194-2199` — coordinator-mode short-circuit. Comment at
  `:2194-2196` reads verbatim: "Coordinator mode has its own system prompt
  and filters out Sleep, so / the generic proactive prompt would tell it to
  call a tool it can't / access and conflict with delegation instructions."
- `src/main.tsx:4611-4621` — `maybeActivateProactive` is NOT short-circuited
  by coordinator mode (asymmetry confirmed).

## Edits applied to `31-mode-proactive.md`

1. **§0 source-coverage table** — added row for
   `src/utils/settings/types.ts:841-863` pointing to spec 19 for semantics.
   (Minor 1.)
2. **§1 Purpose & Scope** — added a paragraph clarifying that **no callback
   wakeup exists in the leak**: ticks are synthetic prompts via
   `setTimeout(0)`; no Zod schema for a wake payload; Sleep's wake-from-queue
   path lives in absent `proactive/index.js` + spec 19. Forward to §12.
   (Minor 2.)
3. **§6.11 Constants table** — added two rows for `minSleepDurationMs` and
   `maxSleepDurationMs` with their Zod shapes, gate, and forward-pointer to
   spec 19. (Minor 1, follow-through.)
4. **§9 Error Handling** — added a bullet documenting that `set_proactive`
   payload is **not Zod-validated** at `src/cli/print.ts:3879-3882`; cast as
   `{ subtype: string; enabled: boolean }` with truthy coercion of `enabled`.
   Flagged as `BUGS-IN-SOURCE.md` candidate. (Nit 1.)
5. **§10 Telemetry** — added bullet noting that `tengu_brief_send`,
   `tengu_brief_mode_enabled`, `tengu_brief_mode_toggled`
   (`src/services/analytics/datadog.ts:29-31`) become observable when
   `KAIROS` / `KAIROS_BRIEF` co-enables Brief alongside proactive. Cross-link
   to specs 32 / 26. Proactive itself still emits no events. (Minor 3.)
6. **§8 Feature Flags & Variants** — replaced the paraphrased coordinator
   row with the verbatim comment from `src/main.tsx:2194-2196`, made the
   activate-vs-prompt-append asymmetry explicit, and pushed the Sleep-filter
   ownership down to spec 30. (Nit 2.)

## Not changed (out of scope for 9.6c)

- Hardest-to-verify claim (pause/resume + context-block state machine) —
  already disclosed in §12 as Phase-0 gap. No fix possible without recovering
  `src/proactive/index.js`.
- `--proactive` vs `CLAUDE_CODE_PROACTIVE` parity — already in §12.
- Coordinator-side Sleep filter — owned by spec 30, not 31.

## Cross-spec ripple

- **19 (SleepTool)** — owes the runtime semantics of `minSleepDurationMs` /
  `maxSleepDurationMs` (spec 31 now forward-links).
- **30 (coordinator)** — owns the assertion that coordinator filters Sleep
  out; spec 31 §8 now cites the rationale comment but defers ownership.
- **32 (Kairos family)** / **26 (Datadog)** — telemetry visibility under
  co-enabled `KAIROS_BRIEF` is now cross-linked from spec 31 §10.
- **`BUGS-IN-SOURCE.md`** — `set_proactive` Zod-absence flagged as
  candidate.

## Verification

All four edits were applied via `Edit` tool with unique `old_string` matches.
No source files modified. No Phase 0 gaps closed; only documentation
completeness improvements.
