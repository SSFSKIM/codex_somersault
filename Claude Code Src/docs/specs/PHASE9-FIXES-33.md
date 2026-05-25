# Phase 9.6c Fix Log — Spec 33 (Mode: DAEMON)

Source review: `PHASE9-ADVERSARIAL-33.md` (1 HIGH, 3 MED, 1 LOW addressed).
All edits in `docs/specs/33-mode-daemon.md`. No source-tree changes.

---

## F1 — [HIGH, Pattern G2] Cross-spec auth-refresh wiring (§11)

**Finding:** Spec 33 did not warn daemon implementers that opening a
`bridgeApi` client without an `onAuth401` handler causes 401s to propagate to
`handleErrorStatus` and become `BridgeFatalError` even when refresh
credentials exist on disk. Phase 9.6 spec-34 wording ("immediate
BridgeFatalError") was also imprecise — the no-handler branch at
`bridgeApi.ts:117-120` returns the response; the caller's `handleErrorStatus`
is what throws (per `bridgeApi.ts:104` comment).

**Fix:** Added §11 item 9 — daemon callers MUST wire
`onAuth401: handleOAuth401Error` (citing `initReplBridge.ts:430,531`,
`bridgeMain.ts:2351,2901`) or accept fatal-on-stale-token semantics.
Explicitly notes the wording correction; spec 34 owns its own softening
(separate agent — not touched here).

## F2 — [MED] SELF_HOSTED_RUNNER sibling-gate disclaim (§1)

**Finding:** `cli.tsx:235-243` `feature('SELF_HOSTED_RUNNER')` fast-path with
absent `src/self-hosted-runner/` shares structural shape with DAEMON entries.
Risk of conflation.

**Fix:** Added OUT-of-scope bullet in §1 disclaiming SELF_HOSTED_RUNNER as a
distinct gate (not a DAEMON variant). Cross-references spec 35. Analogous to
existing Kairos `--assistant` disclaim.

## F3 — [MED] Legacy bridge aliases out-of-scope (§1)

**Finding:** `claude remote-control|rc|remote|sync|bridge` (cli.tsx:108-162)
gated on `BRIDGE_MODE` alone — no DAEMON co-gate. Reader could mis-attribute
to DAEMON.

**Fix:** Added OUT-of-scope bullet in §1 explicitly disclaiming these
fast-paths and pointing to spec 34.

## F4 — [MED] §1 item 3 wording overstates DAEMON-in-REPL relationship

**Finding:** §1 item 3 said `remoteControlServerCommand` is "the only DAEMON
surface that reaches the in-REPL command system" — only true when BRIDGE_MODE
is also live. Pure-DAEMON build exposes zero DAEMON-attributable slash
commands.

**Fix:** Rephrased item 3 to clarify the BRIDGE_MODE co-gate dependency and
explicitly state pure-DAEMON contributes zero in-REPL commands.

## F5 — [LOW] §4.1 PID-record verbatim stitched to mkdir/chmod machinery

**Finding:** Verbatim block was correct but disconnected from the surrounding
`try { mkdir... chmod... writeFile... }` block whose error path is in §9
(pedagogical nit).

**Fix:** Added a paragraph before the verbatim record describing the
`mkdir(recursive, mode:0o700) → chmod(0o700) → writeFile` wrapper and
swallow-and-log error path (concurrentSessions.ts:75-76, 105-108), with
explicit note that an unregistered daemon-worker is a `claude ps` visibility
gap.

---

## Verification

- All edits target spec 33 only.
- F1 wording correction is self-contained; spec 34's separate Phase 9.6
  finding is left for a different agent (per task brief).
- Line citations preserved verbatim from the leaked source (verified against
  `concurrentSessions.ts`, `cli.tsx`, `bridgeApi.ts` paths cited in the
  adversarial review).
- No claims about absent source (`src/daemon/`, `commands/remoteControlServer/`,
  `src/self-hosted-runner/`) — all flagged as missing-source where invoked.
