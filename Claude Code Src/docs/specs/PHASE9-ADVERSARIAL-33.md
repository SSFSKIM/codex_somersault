# Phase 9.5b Adversarial Review — Spec 33 (Mode: DAEMON)

**Reviewer role:** Skeptic. Read-only verification of `docs/specs/33-mode-daemon.md`
against `/Users/new/Downloads/claude-code-main/src/`.

**Reads used:** 6 (cli.tsx slices, commands.ts, concurrentSessions.ts, bridgeApi.ts,
plus 3 grep sweeps).

---

## Severity counts

| Severity | Count |
|---|---|
| Critical | 0 |
| High | 1 |
| Medium | 3 |
| Low / Nit | 3 |
| Confirmed-correct claims sampled | 9 |

The spec is unusually disciplined for this phase: every behavioral claim that
*could* be lifted from the leak is line-cited verbatim, and every claim that
cannot is annotated *missing-source* in §12. The findings below are mostly
gap-coverage, not falsification.

---

## Top 5 findings

### F1 — [HIGH] Cross-spec to 34: spec 33 does not flag the asymmetric heartbeat path

Phase 9.6 spec-34 finding states daemon callers without an `onAuth401` handler
get an "immediate `BridgeFatalError`" via the `!deps.onAuth401 → return response`
branch. **Verification of `src/bridge/bridgeApi.ts:106-134` does not match
that characterisation.**

The actual code path:
- L113-115: `if (response.status !== 401) return response` — happy path.
- L117-120: `if (!deps.onAuth401) { debug(...); return response }` — when
  no refresh handler is wired, the 401 response is **returned to the caller**,
  not thrown. The caller's `handleErrorStatus` is what eventually throws
  `BridgeFatalError`, per the L104 comment ("the 401 response is returned for
  handleErrorStatus to throw BridgeFatalError").

**Implication for spec 33:** the `remoteControlServer` command (commands.ts:76-79)
runs inside a process that — on the supervisor side — *may* call into
`bridgeApi` without wiring `handleOAuth401Error`. If it does, every 401 becomes
a fatal even though refresh credentials exist on disk. Spec 33 §3.5 ("Hosted
services") and §12 Q4 ("Authentication") do not call this out as a delta the
daemon implementer must satisfy. **Add a §11 checklist item: "When the
supervisor opens a `bridgeApi` client it MUST pass `onAuth401:
handleOAuth401Error` (see initReplBridge.ts:430,531; bridgeMain.ts:2351,2901)
or accept fatal-on-stale-token semantics."**

The spec-34 finding's wording ("immediate BridgeFatalError") is also imprecise
and should be softened in spec 34 to "the 401 propagates to caller and
typically becomes fatal via handleErrorStatus."

### F2 — [MEDIUM] Adjacent `SELF_HOSTED_RUNNER` flag exists but spec-35 cross-ref is incomplete

`src/entrypoints/cli.tsx:235-243` contains a sibling fast-path:

```ts
if (feature('SELF_HOSTED_RUNNER') && args[0] === 'self-hosted-runner') {
  ...
  await selfHostedRunnerMain(args.slice(1));
```

The `src/self-hosted-runner/` directory is **absent** from the leak (verified
via `ls`). Spec 33 currently treats `claude daemon` and `claude --daemon-worker`
as the only DAEMON-like supervisor entries, but `self-hosted-runner` is a
*third* feature-gated long-running entry sharing structural shape (lazy import,
fast-path before command dispatch, missing implementation directory). Spec 35
mentions self-hosted-runner per §1; spec 33 should add a one-line §1 disclaim
that `SELF_HOSTED_RUNNER` is a sibling-but-distinct gate (not part of `DAEMON`)
to prevent future readers conflating them — analogous to the existing
`--assistant`/Kairos disclaim at §1 / §12 Q9.

### F3 — [MEDIUM] §3.3 omits that legacy `remote-control` aliases are bridge-mode entries (not daemon)

cli.tsx:108-162 dispatches on `'remote-control' | 'rc' | 'remote' | 'sync' |
'bridge'` under `feature('BRIDGE_MODE')` alone (no DAEMON co-gate). Spec 33 §1
correctly omits these, but a reader looking at spec 33's §6.6 ("DAEMON adds no
user-facing string") could plausibly miss that `remote-control`/`bridge` are
*also* bridge-only and not daemon-related. Recommend adding a §1 OUT-of-scope
bullet: "The `claude remote-control|rc|remote|sync|bridge` CLI fast-paths
(cli.tsx:112) are BRIDGE_MODE-only and unrelated to DAEMON; see spec 34."

### F4 — [MEDIUM] §3.2 unverifiable: `remoteControlServerCommand` directory absence + co-gate semantics

Confirmed: `src/commands/remoteControlServer/` is absent from the leak. §12 Q6
correctly flags this. However the spec asserts at §1 item 3 that the command
"is the only DAEMON surface that reaches the in-REPL command system." This is
**conditionally** true — only when `BRIDGE_MODE` is also live. A pure-DAEMON
build (DAEMON=on, BRIDGE_MODE=off) would expose **zero** DAEMON-attributable
slash-commands. The spec's §1 wording is technically defensible ("the only
DAEMON surface that reaches in-REPL", treating BRIDGE_MODE=off as
"DAEMON-mode-with-no-REPL-surface") but a stricter reader would say §1 item 3
overstates the relationship. Recommend re-phrasing to: "When co-gated with
`BRIDGE_MODE`, the only DAEMON surface that reaches in-REPL is..."

### F5 — [LOW] §4.1 PID-file record verbatim is incomplete

The verbatim block at spec lines 173-191 ends after the `BG_SESSIONS` spread.
Verified file `concurrentSessions.ts:77-97` matches exactly. However the spec
omits that the surrounding `writeFile()` call is inside a `try { mkdir(...,
{recursive: true, mode: 0o700}); chmod(dir, 0o700); ... }` block whose error
path is in §9 but not stitched together. This is a pedagogical nit, not a
correctness issue.

---

## Other minor findings

- **§5.4** says `kind selection follows envSessionKind() ?? 'interactive'`. Verified at
  concurrentSessions.ts:62. Correct.
- **§6.4 "Default daemon port" missing-source.** Confirmed; no daemon port
  constant in any of the surveyed files.
- **§12 Q9** (Kairos `--assistant` overlap) is correctly disclaimed; spec 32
  should mirror this.

---

## Verdict

**APPROVE with minor revisions.** Spec 33 is one of the most epistemically
honest mode specs in the set — its severely-partial source coverage is
documented, every behavioral claim is line-cited, and missing-source
annotations dominate §3.4-§5.5. The single material issue (F1) is a
cross-spec mischaracterization originating in spec 34's Phase 9.6 finding,
not in spec 33 itself; spec 33 should still gain a §11 checklist item to
defend daemon implementers from the same trap.

No claim in spec 33 was found to contradict the leaked source.

---

## Cross-spec impact

| Affected spec | Action required |
|---|---|
| **34 (bridge)** | Soften "immediate BridgeFatalError" wording in Phase 9.6 finding; the no-onAuth401 path returns the 401 response and lets `handleErrorStatus` throw downstream (bridgeApi.ts:117-120, comment at L104). |
| **35 (remote/CCR)** | Confirm `self-hosted-runner` ownership; spec 33 should add a one-line disclaim that `SELF_HOSTED_RUNNER` is a sibling gate, not a DAEMON variant (cli.tsx:238). |
| **32 (Kairos assistant)** | Mirror spec 33 §12 Q9's "Agent SDK daemon"/`--assistant` disclaim. |
| **41 (UDS_INBOX)** / **19 (concurrent sessions)** | Already correctly cross-referenced from §4.1 and §8. |

---

## Hardest-to-verify claim

**§3.3 row "Gate on env-var read: requires `feature('BG_SESSIONS')` at runtime"
— specifically, the *consequence* that "Without BG_SESSIONS, daemon-spawned
children fall back to `kind: 'interactive'` (concurrentSessions.ts:62)."**

Verified mechanically (envSessionKind returns undefined when BG_SESSIONS is
off; line 62 falls back to `'interactive'`). However the *practical*
implication — that a DAEMON-on / BG_SESSIONS-off build silently mislabels
every supervisor-spawned worker as interactive in `claude ps` — cannot be
checked end-to-end because (a) `daemonMain`/`runDaemonWorker` are absent, so
we cannot confirm they actually call `registerSession()`; (b) the supervisor's
spawn logic (whether it sets `CLAUDE_CODE_SESSION_KIND`) is unverifiable; and
(c) `claude ps`'s rendering of `kind: 'interactive'` for what is really a
worker is not testable from source alone. The spec correctly flags this as
runtime-conditional but cannot prove the failure mode is real. Whoever
recovers `src/daemon/main.ts` should re-verify this row first.
