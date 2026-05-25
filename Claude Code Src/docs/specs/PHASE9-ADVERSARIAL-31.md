# Phase 9.5b Adversarial Review — Spec 31 (`PROACTIVE` mode)

**Reviewer role:** Skeptic. **Scope:** `docs/specs/31-mode-proactive.md` vs `src/`.
**Method:** spot-check every cited line range, flag-disjunction discipline,
cross-spec consistency, and absent-module honesty. ~13 src reads.

## Severity counts

| Severity | Count |
|---|---|
| Blocker | 0 |
| Major   | 0 |
| Minor   | 3 |
| Nit     | 4 |
| Verified-as-stated | 14 |

The spec is unusually careful: every gate verified is `feature('PROACTIVE') || feature('KAIROS')` (not bare `PROACTIVE`), every cited line range matches src, and the absent-module surface (`src/proactive/index.js`, `src/commands/proactive.js`) is openly declared as inferred-from-callers in §0/§3/§12.

## Top 5 findings

### 1. Minor — `minSleepDurationMs` / `maxSleepDurationMs` settings absent from spec 31
`src/utils/settings/types.ts:841-863` declares both keys gated on `feature('PROACTIVE') || feature('KAIROS')`, with `maxSleepDurationMs: -1 = indefinite (waits for user input)` and a "Useful for throttling proactive tick frequency" doc string. Spec 31 §6.11 lists "Default sleep duration / max chain length — unknown" but never references these two **settings keys** that ARE in the leaked source under the proactive gate. They are documented in spec 19 (SleepTool), not 31, but spec 31's §6.11 constants table omitting them while §12 lists "tick-interval default" as unknown is a coverage gap — at minimum a forward pointer is owed. Cross-spec to 19 and Phase 9.5 spec 02.

### 2. Minor — Auto-wakeup self-scheduling claim is unverifiable from src
The user prompt asks specifically about "Auto-wakeup self-scheduling." Spec 31 documents headless tick scheduling (`scheduleProactiveTick` at `cli/print.ts:1834-1856`) and re-arm after `run()` (`:2475-2485`), which is **prompt-driven (synthetic `<tick>`), not callback-driven**. There is no auto-wakeup Zod schema, no scheduler beyond `setTimeout(0)`, and no MCP-channel-poll-driven wake mechanism in the leaked tree — the only hint is a comment at `src/services/mcp/channelNotification.ts:9` ("SleepTool polls hasCommandsInQueue() and wakes within 1s") which the spec correctly defers to §12 as Phase-0 gap. Spec is honest, but a reader expecting a "wakeup callback" surface (per §6.10 "N/A") may miss that the entire wake mechanism lives in absent files. Recommend §1 add one sentence: "There is no callback wakeup; ticks are synthetic prompts, and Sleep wakes via internal poll on the proactive module's queue inspector (absent module — see 19)."

### 3. Minor — Brief telemetry / Datadog allow-list cross-spec note missing
Phase 9.6 spec 26 finding flagged `tengu_brief_send`, `tengu_brief_mode_enabled`, `tengu_brief_mode_toggled` as Kairos-feature-flagged but in the Datadog allow-list (`src/services/analytics/datadog.ts:29-31`). Spec 31 §10 says "No analytics events are emitted from the proactive module's call-sites." Verified true for **proactive** events. But because spec 31 explicitly defines its scope to cover `feature('PROACTIVE') || feature('KAIROS')` disjunction and §6.1 quotes the brief-aware `briefVisibility` interpolation, a reader who turns on `KAIROS_BRIEF` while proactive is active will hit those telemetry names from spec 32's BriefTool path. §10 should add a sentence: "When `KAIROS` or `KAIROS_BRIEF` is co-enabled, the brief subsystem (32) emits `tengu_brief_*` events visible to Datadog (per `services/analytics/datadog.ts:29-31`)." Cross-spec to 32 and 26.

### 4. Nit — `set_proactive` payload typing deserves Zod-absence note
`cli/print.ts:3879-3882` casts the request as `{subtype: string; enabled: boolean}` with no Zod validator at the callsite. Spec 31 §3 quotes this verbatim and §6.10 correctly says "no Zod schema declared." This is fine but worth a single bullet in §9 (Error Handling): a malformed `enabled` value (non-boolean) would be truthy-coerced into activate. Other control-protocol subtypes in `cli/print.ts` use Zod-parsed payloads; this one does not. Phase 9.5 family theme.

### 5. Nit — `coordinatorModeModule` short-circuit asymmetry could be sharper
Spec §1 and §9 correctly note coordinator mode suppresses the **REPL prompt-append** but NOT activation. Verified at `main.tsx:2197-2199` (suppressed) vs `:4611-4621` (not suppressed). However spec §8's table row for `COORDINATOR_MODE` says "Coordinator's own prompt filters Sleep out per the inline comment" — that comment is paraphrased from `main.tsx:2195` ("the generic proactive prompt would tell it to call a tool it can't"). The spec should quote the comment verbatim or cite the exact line, since the assertion that coordinator filters Sleep is a cross-spec claim into 30 (coordinator) and the coordinator module's tool-filter source isn't cited here.

## Additional nits

- §0 cites `src/cli/print.ts:1831-1856` — verified, exact match.
- §0 cites `src/cli/print.ts:3875-3891` — verified, `set_proactive` handler matches.
- §0 cites `src/utils/settings/types.ts` not at all — minor (gate location for the two sleep-duration keys is the most-load-bearing PROACTIVE-gated settings surface in the leak, deserves at least one row pointing to spec 19/02).
- §6.11 "max chain length" — there is no chain-length setting in `settings/types.ts` under either flag; correctly listed as unknown.

## Verdict

**ACCEPT with three minor amendments.** Spec 31 demonstrates exemplary discipline on (a) the always-disjunction gate, (b) absent-module honesty (§0 source-coverage table, §3 inferred signatures, §12 gap list), and (c) bit-exact verbatim assets including the em-dash in `terminalFocus`. The three minor items above are documentation completeness (settings keys, brief telemetry cross-ref, wakeup-mechanism clarity) — none change the runtime contract. No blockers. Phase 9.5b can sign off with a §10 + §6.11 + §1 sentence patch.

## Cross-spec impact

| Target spec | Impact |
|---|---|
| 19 (SleepTool) | Owes the `minSleepDurationMs`/`maxSleepDurationMs` settings documentation; spec 31 §6.11 should forward-reference. |
| 32 (Kairos family) | Brief telemetry visibility under co-enabled KAIROS_BRIEF — spec 31 §10 should cross-link. |
| 30 (coordinator) | Coordinator's Sleep-tool filter is asserted in spec 31 §8 but the filter mechanism lives in coordinator source — that spec must own the assertion. |
| 41 (session lifetime) | Synthetic `'Proactive session'` resume title (`sessionStorage.ts:4889-4912`) verified consistent. |
| 02 (settings, Phase 9.5) | Confirms the two sleep-duration keys are PROACTIVE||KAIROS gated — match. |
| 26 (Phase 9.6 Datadog) | Brief telemetry allow-listing finding stands; spec 31 should not absorb it but should cross-link. |

## Hardest-to-verify claim

**The pause/resume + context-block state machine.** Spec §5.3-§5.4 describe `pauseProactive`/`resumeProactive`/`setContextBlocked` as a coherent state machine, but **every reader of these states lives inside the absent `src/proactive/index.js`**. Callers only invoke setters; no caller asks "are we blocked?" or "are we paused?" except the headless tick scheduler at `cli/print.ts:1838-1842` (which queries `isProactiveActive()` and `isProactivePaused()`). The claim that `setContextBlocked(true)` actually suppresses ticks during API error storms is sourced **only from a comment** at `screens/REPL.tsx:2631-2633`. The spec correctly flags this as an open question in §12, but a reimplementer cannot validate the intended semantics from the leak alone — only from runtime observation or from reconstructing `proactive/index.js` out of the bundled `main.tsx`. This is the single largest behavioral surface for which the spec must rely on inference rather than source.
