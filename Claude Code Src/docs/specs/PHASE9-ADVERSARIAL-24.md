# Phase 9.5b Adversarial Review — Spec 24 (Service: LSP)

**Reviewer role**: Skeptic. Verified `docs/specs/24-service-lsp.md` against `src/services/lsp/` (all 7 owned files fully read) and the spec-cited callsites in `src/tools/FileWriteTool/`, `src/tools/FileEditTool/`, `src/tools/NotebookEditTool/`.

## Severity Counts

| Severity | Count |
|---|---:|
| Critical | 0 |
| High | 1 |
| Medium | 3 |
| Low | 3 |
| Nit | 2 |
| **Total** | **9** |

The spec is **unusually accurate**. Every cited line range I spot-checked matches the source byte-for-byte (init params §6.2, error strings §6.5, retry constants §6.1, request shapes §6.3). Findings below are mostly omissions of cross-spec consequences, not factual errors in the LSP-service description.

## Top 5 Findings

### F1 (High) — Notebook (.ipynb) handling is undocumented in dispatch contract; FileWriteTool will fire `saveFile()` for `.ipynb` if a plugin claims that extension
**Where**: `LSPServerManager.ts:192-207` (extension dispatch is purely lower-cased `path.extname`); `FileWriteTool.ts:308-320` (calls `lspManager.saveFile(fullFilePath)` unconditionally for any successfully written file). `NotebookEditTool.ts` has **no** `saveFile`/`clearDeliveredDiagnosticsForFile` callsite at all (verified by grep) — so a notebook edit never updates LSP state.
**Spec gap**: §3.2/§5.6/§5.10 silently route `.ipynb` to whichever plugin server registers that extension; spec §9.6's stated direction is "exclude `.ipynb` from LSP." The manager does **not** carve out `.ipynb`. Two consequences:
  1. If `FileWriteTool` is used to write a `.ipynb` (which it can — there is no `.ipynb` block in `FileWriteTool.ts`), `saveFile` is called and an LSP server registered to `.ipynb` will be notified of a save it never received `didOpen` for. `LSPServerManager.saveFile` (`:349-368`) tolerates this (`if (!server || server.state !== 'running') return`) — but only because the *manager state* says the file was never opened, not because of any `.ipynb` filter. Add an explicit invariant in §5.6 that `.ipynb` is excluded by the file-tools layer (this is a spec-11 cross-spec gap, not a §24 implementation gap).
  2. `NotebookEditTool` editing a `.py`-cell-bearing notebook does not invoke `clearDeliveredDiagnosticsForFile`, so cross-turn dedup will suppress re-emitted diagnostics for that file path indefinitely (until LRU eviction). This compounds the spec-11 NotebookEdit/`preparePermissionMatcher` finding; it is **also** a stale-dedup bug if any cell-language ever flows to LSP via a Python-server plugin.
  Spec should explicitly state: "If `.ipynb` exclusion is required, it must be enforced by callers (spec 11) — the manager performs no notebook filtering." Cross-link to spec 11/16.

### F2 (Medium) — Spec says `restart()` "increments only `restartCount`" but source increments after an *uncaught* `stop()` failure too
**Where**: `LSPServerInstance.ts:300-331`. Source order is: `try stop()` → on failure throws (does **not** bump `restartCount`); `restartCount++` runs only after `stop()` succeeds. Spec §5.5 prose ("restartCount++; if restartCount > maxRestarts: throw…") is correct, but §11 ("`restart()` increments only `restartCount`; crash path increments `crashRecoveryCount`") may mislead a reimplementer into incrementing before/independent of the `stop()` outcome. Tighten the wording.

### F3 (Medium) — `stop()` re-throw side-effect is undocumented: it sets `startFailed = true` and `startError = shutdownError`
**Where**: `LSPClient.ts:432-444`. After cleanup, if `shutdownError` was captured during `shutdown`/`exit`, the client mutates `startFailed = true; startError = shutdownError` *before* re-throwing. This means a subsequent `start()` on the **same client instance** would immediately throw via `checkStartFailed()` even though `start()` itself doesn't read `startFailed` until inside `initialize`/`sendRequest` paths — but `LSPServerInstance.start()` calls `client.start()` which constructs a fresh process and *does not reset `startFailed`* before the spawn. Spec §5.7's `stop()` summary omits this; §11 reimplementation checklist would produce a subtly different state machine. Document the side-effect, or note that `LSPServerInstance` re-creates context (it does **not** — `client` is closure-captured at instance creation, `:121-125`).

### F4 (Medium) — `getServerForFile` priority claim glosses over a real plugin-loading-order hazard
**Where**: spec §4.4 + §9 both say "first-registered wins" but §4.4 attributes the order to "plugin loading order (`Object.assign` later-wins on the config dict)". `config.ts:45-49` does `Object.assign(allServers, scopedServers)` per plugin in `Promise.all` order. *Later* plugins overwrite the config dict but **all** servers still end up in `serverConfigs`. Then `LSPServerManager.initialize` (`:89-117`) iterates `Object.entries(serverConfigs)` and pushes each server into `extensionMap` — first-iterated wins. So "later-wins on config dict" applies only to **same-named** server overrides; for **different** server names competing for the same extension, **earlier**-iterated wins. Spec conflates these. Reimplementers will swap the meaning.

### F5 (Low) — `connection.onClose` does **not** trigger `onCrash`; spec implies any unexpected close is recoverable but only `process.on('exit', code !== 0)` calls `onCrash`
**Where**: `LSPClient.ts:200-207` vs `:156-167`. If a server cleanly closes its stdout (graceful exit code 0 or signal) without the process actually exiting non-zero, `connection.onClose` fires, `isInitialized = false`, but `onCrash` is **not** invoked → `LSPServerInstance.state` stays `'running'` while `client.isInitialized` is false. Next `sendRequest` fails `isHealthy()` (which checks `state === 'running' && client.isInitialized` — `:339`), throws "server is running, last error: …" with no `lastError` set, and never restarts because `crashRecoveryCount` was never bumped. Spec §9 "Crash recovery" understates this corner. Add a §9 bullet for "graceful-close-but-not-exit" zombie state.

## Cross-Spec Impact

- **Spec 11 (file edit tools)**: F1 promotes to a hard requirement. Spec 11 must (a) gate `saveFile`/`clearDeliveredDiagnosticsForFile` on **non-`.ipynb`** extensions in `FileWriteTool` and `FileEditTool`, *or* (b) add the same calls to `NotebookEditTool` for the underlying `.ipynb` path so dedup state stays consistent. Currently neither — `NotebookEditTool.ts` has zero LSP imports, while `FileWriteTool` will fire LSP for `.ipynb` if a plugin registers that extension.
- **Spec 16 (LSPTool)**: F1 also implies LSPTool must filter `.ipynb` in `preparePermissionMatcher`/input validation; the LSP service itself does not.
- **Spec 28 (plugin LSP integration)**: F4 (Object.assign ordering) and §12.7 (no default `startupTimeout`) both push requirements onto plugin loading.
- **Spec 34 (IDE bridge)**: spec §1 says "IDE LSP integration via the bridge → spec 34" but `src/services/lsp/` has zero references to `src/bridge/` (verified). The two systems do not interact at this layer — spec 34 should confirm the bridge path is independent (separate JSON-RPC channel).
- **Spec 37 (UI)**: `useLspInitializationNotification` callsite at `:97` confirmed; spec accurately defers UI rendering.

## Hardest-to-Verify Claim

**`startupTimeout`'s `withTimeout` "clears its timer in `finally`"** (§9 + §11 + `LSPServerInstance.ts:499-511`). The spec asserts cleanup prevents orphaned `setTimeout` callbacks, but the source uses `Promise.race(...).finally(() => clearTimeout(timer!))` where `timer` is assigned **inside** the `Promise` executor — a race where the promise rejects synchronously (e.g., `client.initialize` throws on the same tick) could `clearTimeout(undefined)`. Node tolerates this, but the "no orphan" guarantee is weaker than the spec implies: if `setTimeout` has already fired and rejected `timeoutPromise` before `finally` runs, the rejection is the winner of `Promise.race` and is propagated; `clearTimeout` after-fire is a no-op. The behavior is correct but the *invariant* the spec claims ("avoid orphaned setTimeout callbacks") is over-stated — the timer always fires once if its delay elapses; `finally` only avoids a *pending* timer leaking past resolution. Effectively unverifiable without runtime tracing; flag as docs-precision rather than a defect.

## Other Findings (brief)

- **F6 (Low)**: §5.7 says `process.kill()` is "wrapped in try/catch" — true at `:417-424` but spec omits that listeners are removed *before* `kill()`, which is the actual leak prevention; reimplementers reading only §11 may reorder.
- **F7 (Low)**: §6.1 table row "version field on didOpen / didChange | hard-coded 1" is correct but spec §11 reimplementation checklist phrases this as a no-state requirement. LSP servers that strictly validate monotonic versions (rust-analyzer in some configs) will reject the second `didChange`. Worth a `## 12` open question.
- **F8 (Nit)**: §3.5 lists `clearAllLSPDiagnostics` and `resetAllLSPDiagnosticState` in the public surface but no in-tree caller exists for either (verified — only `clearDeliveredDiagnosticsForFile` is consumed by file tools). They are public-but-unused; flag as dead-code candidates for spec 11/Phase 10.
- **F9 (Nit)**: §6.3 table omits the `process.pid` field of `initialize` params (it's in §6.2 but not the per-method shape table).

## Verdict

**APPROVED with minor fixes.** Spec 24 is high-fidelity to the source — line citations, error strings, JSON-RPC shapes, and constants all check out. Required edits: (1) F1 — add explicit `.ipynb` non-handling note + cross-link to spec 11/16; (2) F4 — disambiguate "first-registered wins" wrt `Object.assign` semantics; (3) F3, F5 — document `stop()` side-effect on `startFailed` and the graceful-close zombie state in §9. F2/F6/F7 are tightening; F8/F9 are nits. No factual reversals required.
