# Phase 9.6c — Spec 24 (Service: LSP) Fix Log

Adversarial findings addressed: F1 (HIGH), F2, F3, F4 (Medium), F5 (Low). Source-verified before edit.

## Source verifications

- `FileWriteTool.ts:6-7, 308-320` — confirmed `getLspServerManager` + `clearDeliveredDiagnosticsForFile` imports; `saveFile` called unconditionally for any successfully-written path; **no `.ipynb` extension gate**.
- `NotebookEditTool.ts` — grep returns **zero** LSP-related imports / callsites (no `saveFile`, no `clearDeliveredDiagnosticsForFile`, no `getLspServerManager`).
- `LSPClient.ts:425-444` — `stop()` finally block: `isInitialized = false; capabilities = undefined; isStopping = false`, comment "Don't reset startFailed - preserve error state for diagnostics", then `if (shutdownError) { startFailed = true; startError = shutdownError }` before re-throw.
- `config.ts:27-49` — `Promise.all` over plugins, then ordered for-loop with `Object.assign(allServers, scopedServers)` — confirms later-wins semantics applies to same-keyed scoped server names only.
- `LSPServerManager.ts:89-117, 200-207` — `Object.entries(serverConfigs)` iteration order pushes into `extensionMap[ext]` array; `getServerForFile` reads `extensionMap[ext][0]` → first-wins for different-named servers.
- `LSPClient.ts:156-167` (`process.on('exit')`) vs `:200-207` (`connection.onClose`) — confirmed: only the former invokes `onCrash`; `onClose` only sets `isInitialized = false` + logs.

## Edits to `24-service-lsp.md`

1. **§4.4** — Replaced single "first-registered wins" line with a numbered disambiguation of (1) same-named `Object.assign` later-wins in `config.ts:45-49` and (2) different-named first-iterated wins in `extensionMap`. Added explicit notebook non-filter note with cross-link to §9.6 / §12.8 / spec 11 / spec 16. (F1 + F4)
2. **§5.5** — Added "Ordering invariant" paragraph: `restartCount++` runs **only after `stop()` resolves successfully**; failed `stop()` re-throws without bumping. Inline comment in pseudocode block. (F2)
3. **§5.7** — Appended "stop() side-effect on failure (undocumented elsewhere)" paragraph: documents `startFailed = true; startError = shutdownError` mutation at `LSPClient.ts:431-436`, the closure-captured client persistence across `LSPServerInstance` cycles, and the lack of `startFailed` reset at the top of `start()`. (F3)
4. **§9** — Replaced single bullet with three: disambiguated `getServerForFile` priority, added graceful-close zombie state (`onClose` does not invoke `onCrash`; `crashRecoveryCount` never bumps; `isHealthy()` AND-check fails with no `lastError`); added notebook caller-responsibility bullet with full consequence chain. Flagged graceful-close as `BUGS-IN-SOURCE.md` candidate. (F1 + F4 + F5)
5. **§11** — Added four checklist items: `restart()` ordering invariant, `client.stop()` side-effect replication, dual extension-dispatch ordering, `.ipynb` carve-out is caller-side, `onClose` ↛ `onCrash` policy. (F1 + F2 + F3 + F4 + F5)
6. **§12** — Added items 8 and 9: §8 cross-spec rollup pinning the `.ipynb` work to specs 11 + 16 (Phase 9.6c forward-link); §9 graceful-close zombie state with proposed-fix sketch. (F1 + F5)

## Cross-spec ripples

- **Spec 11** — F1 forward-couples here: `FileWriteTool` / `FileEditTool` need `.ipynb` gate, *or* `NotebookEditTool` needs LSP callsite parity. Phase 9.6c on spec 11 should land the carve-out.
- **Spec 16** — LSPTool `preparePermissionMatcher` / input validation should reject `.ipynb`.
- **`BUGS-IN-SOURCE.md`** — F5 (graceful-close zombie state) is a new candidate entry.

No factual reversals. Verified post-edit via grep that all six insertions are present at lines 249, 373, 711, 713, 743-745, 775-776.
