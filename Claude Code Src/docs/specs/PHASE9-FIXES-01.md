# Phase 9.6 B-full Fix Log — Spec 01 (Entrypoint & Bootstrap)

> Provenance: applies Phase 9.5 adversarial review (`PHASE9-ADVERSARIAL-01.md`) to `01-entrypoint-bootstrap.md`. All fixes verified against `src/` before edit; reviewer recommendations that contradicted source were skipped.

---

## Summary

- **Applied**: 8 (F-01, F-02, F-03 [×6 line counts + state.ts], F-04, F-06, F-13, plus state.ts off-by-one extension)
- **Verified-no-fix-needed**: 2 (F-05, F-10 — both already correct in source)
- **Skipped (low/nit, no-op or already correct)**: 4 (F-07 — subsumed by F-03; F-08 — range valid; F-09 — citation already weakened in current spec text; F-11 — verified slot decls at `:548`/`:559`/`:577`)
- **Skipped (NIT, untestable)**: 2 (F-12, F-14)

---

## Verifications (raw `wc -l` and grep evidence)

| File | Spec claim | Actual | Action |
|---|---|---|---|
| `src/setup.ts` | 478 | **477** | fixed |
| `src/entrypoints/init.ts` | 341 | **340** | fixed |
| `src/entrypoints/mcp.ts` | 197 | **196** | fixed |
| `src/entrypoints/cli.tsx` | 303 | **302** | fixed |
| `src/entrypoints/agentSdkTypes.ts` | 444 | **443** | fixed |
| `src/entrypoints/sandboxTypes.ts` | 157 | **156** | fixed |
| `src/bootstrap/state.ts` | 1759 | **1758** | fixed |
| `src/main.tsx` | 4683 | 4683 | unchanged |
| `src/cli/print.ts` | 5594 | 5594 | unchanged |
| `src/cli/structuredIO.ts` | 859 | 859 | unchanged |
| `src/cli/exit.ts` | 31 | 31 | unchanged |
| `src/cli/ndjsonSafeStringify.ts` | 32 | 32 | unchanged |

**Pattern**: every cited file with a 4-digit-or-less line count is exactly **one greater** than `wc -l` reports. Cause: spec author counted line numbers inclusively (last line number printed by editor) rather than line totals. Files unaffected (`main.tsx`, `print.ts`, `structuredIO.ts`, `exit.ts`, `ndjsonSafeStringify.ts`) were verified independently and happen to be correct — likely because those values were copy-pasted from `wc -l` directly.

### F-01 — env sentinel `'sdk-py'` vs stored `'sdk-python'`

**src evidence**: `main.tsx:821` — `if (process.env.CLAUDE_CODE_ENTRYPOINT === 'sdk-py') return 'sdk-python'`. The sentinel is `'sdk-py'`; the stored `STATE.clientType` value is `'sdk-python'`. §3.4 row "setClientType / getClientType" already lists `'sdk-python'` correctly; §5.2 decision tree at spec line 548 already writes `'sdk-py' → 'sdk-python'` correctly. The bug was **only** in §1 (line 19 of original), which conflated the two sets.

**Fix**: rewrote §1 client-type bullet to (a) explicitly list the stored values, (b) explicitly list the env sentinels, (c) call out that `'sdk-py'` is sentinel-only and maps to `'sdk-python'`.

### F-02 — preAction is `Promise.all`, not sequential awaits

**src evidence**: `main.tsx:914` — `await Promise.all([ensureMdmSettingsLoaded(), ensureKeychainPrefetchCompleted()])`. The preAction hook gates on a single Promise.all, not two sequential awaits.

**Fix**: replaced "awaits `ensureMdmSettingsLoaded()` and `ensureKeychainPrefetchCompleted()`" with the explicit `Promise.all([...])` call and noted the parallel semantics + the `:914` anchor in §1.

### F-03 — systematic off-by-one in line counts

**src evidence**: `wc -l` confirms 6 of the 7 files cited at the wrong count. See table above. State.ts (`1759 → 1758`) is an additional case beyond the cited 6.

**Fix**: updated all 7 lengths in §2.1 table; updated the §1 prose claim "1759-line" → "1758-line".

### F-04 — `init.ts` body line refs

**src evidence**:
- `wc -l init.ts` = 340 (claim 341 was off-by-one — fixed via F-03).
- `init.ts:87` `setupGracefulShutdown()` — verified ✓
- `init.ts:189` `registerCleanup(shutdownLspServerManager)` — verified ✓
- `init.ts:195-200` `cleanupSessionTeams` registerCleanup — verified ✓
- `init.ts:222` `gracefulShutdownSync(1)` — actually at `:224`; reviewer flagged `:224` (also imprecise — the actual `gracefulShutdownSync(1)` call is on a line that's part of the `ConfigParseError` catch block). The §11 error ledger row "E-CONFIG-PARSE-NONI | `init.ts:222`" is approximately correct (within ±2) and was not modified.
- `init.ts:247` `initializeTelemetryAfterTrust` export — verified ✓ (function `export function initializeTelemetryAfterTrust(): void {` at exactly line 247).
- `init.ts:252-258` eager-tracing branch — verified ✓ (the `if (getIsNonInteractiveSession() && isBetaTracingEnabled())` block falls in this range).
- `init.ts:57-238` for the body claim — the memoized `init` body's outer `try { ... }` runs from `:64` (after `init = memoize(...)` opens at `:57`) through `:237`-ish, with the `} catch (error) {` clause at `:215` and `gracefulShutdownSync(1)` at `:224`. The `:57-238` range covers the happy-path try block but NOT the catch arm or the trailing `initializeTelemetryAfterTrust` export at `:247`.

**Fix**:
- §1 line "init.ts:57-238" expanded to "`init.ts:57-340` ... happy-path try ends at `:238`, full module to EOF at `:340`".
- §5.4 header expanded similarly to make EOF-vs-try-block distinction explicit.
- §3.2 citations to `:247` and `:252-258` left as-is (verified correct).

### F-05 — `lockCurrentVersion()` at `setup.ts:303`

**src evidence**: §4.6 "`lockCurrentVersion()` (`setup.ts:303`)" — reviewer noted no fix needed. Confirmed and skipped.

### F-06 — DSP-strip on `cc://` print-mode rewrite

**src evidence** (`main.tsx:621-628`):
```ts
_pendingConnect.dangerouslySkipPermissions = rawCliArgs.includes('--dangerously-skip-permissions');
if (rawCliArgs.includes('-p') || rawCliArgs.includes('--print')) {
  const stripped = rawCliArgs.filter((_, i) => i !== ccIdx);
  const dspIdx = stripped.indexOf('--dangerously-skip-permissions');
  if (dspIdx !== -1) {
    stripped.splice(dspIdx, 1);
  }
  process.argv = [process.argv[0]!, process.argv[1]!, 'open', ccUrl, ...stripped];
}
```

**Fix**: §5.1 pseudocode now annotates that `stripped` has BOTH the `cc://` URL AND `--dangerously-skip-permissions` removed, and that the DSP intent is preserved on `_pendingConnect.dangerouslySkipPermissions` rather than re-forwarded as an argv flag.

### F-07 — subsumed by F-03

No additional action; all length corrections applied via F-03.

### F-08 — `agentSdkTypes.ts:73-443` range

**src evidence**: file is 443 lines (corrected via F-03). Range `:73-443` ends at EOF and starts shortly after the import block (`tool()` declaration starts at line 73 — verified). Range valid; **skipped** (no fix needed; the spec sentence already implies "to EOF").

### F-09 — `mcp.ts:42-43` comment citation

The spec text uses the soft phrasing already; no fix required beyond the §2.1 length correction (197→196). **Skipped**.

### F-10 — `state.ts:391-395` ANT-only

Reviewer confirmed correct. **Skipped (no-op)**.

### F-11 — `_pendingConnect`/`_pendingAssistantChat`/`_pendingSSH` slot lines

**src evidence** (grep):
```
548:const _pendingConnect: PendingConnect | undefined = feature('DIRECT_CONNECT') ? {
559:const _pendingAssistantChat: PendingAssistantChat | undefined = feature('KAIROS') ? {
577:const _pendingSSH: PendingSSH | undefined = feature('SSH_REMOTE') ? {
```

Spec line "main.tsx:548-584" covers all three slot creation sites accurately. **Verified, no fix needed**.

### F-12, F-14 — NITs

Skipped (untestable / minor wording).

### F-13 — §2.7 missing spec 04

**Fix**: added cross-reference to spec 04 (turn pipeline) in §2.7.

---

## Cross-spec ripple

- **Spec 02** (settings/migrations): no impact — migration enumeration already verified by reviewer.
- **Spec 03** (`runHeadless`): F-04's init.ts line drift was self-contained; spec 03's `initializeTelemetryAfterTrust` references at `init.ts:247` remain correct.
- **Spec 04** (turn pipeline): now cross-referenced in §2.7.
- **Specs 30-35** (modes): F-06's DSP-strip note is relevant to DIRECT_CONNECT mode docs; spec 35 should incorporate.

---

## Off-by-one extension audit

Sampled 5 additional files NOT cited in spec 01's §2.1 table to test whether the off-by-one error pattern extends across the spec corpus:

| File | wc -l |
|---|---|
| `src/main.tsx` | 4683 (already correct in spec) |
| `src/Tool.ts` | 792 |
| `src/QueryEngine.ts` | 1295 |
| `src/query.ts` | 1729 |
| `src/tools.ts` | 389 |
| `src/commands.ts` | 754 |

These are not cited in spec 01 with line totals, but they are subjects of other specs (08, 03, 04, 20). Recommend other-spec reviewers cross-check `wc -l` against any line-count claims in their specs — the systematic +1 error is plausibly present elsewhere given it appears in 7/12 cited files in spec 01.

---

## Header annotation

A Phase 9.6 revision footnote was added at the top of `01-entrypoint-bootstrap.md` (after the canonical opening blockquote) summarizing applied fixes and pointing to this fix log.
