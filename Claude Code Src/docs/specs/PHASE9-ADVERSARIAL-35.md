# PHASE 9.5 — Adversarial review: spec 35 (mode-remote-server)

Reviewer: opus fallback. READ-ONLY. ~14 src reads.

## Severity counts

- BLOCKER: 0
- HIGH: 2
- MEDIUM: 4
- LOW: 4
- NIT: 3

## Top 5 findings

### H1 — Spec elides /ultrareview, the *primary* CCR-launch path
Spec §1 enumerates "five sub-modes" (CCR, direct-connect, SSH, self-hosted-runner, UDS) and §3.1 lists entrypoints — but `/ultrareview` is not mentioned anywhere despite being a major remote-launch path. `src/commands/review.ts:45-57` ships an `ultrareview` Command whose `reviewRemote.ts` ("Teleported `/ultrareview` execution. Creates a CCR session with the current repo") is *the* user-visible CCR entrypoint, gated by `fetchUltrareviewQuota` and `isUltrareviewEnabled`. Spec 35 should at minimum cross-reference 21d (where /review and /ultrareview live) and document `reviewRemote.ts` + the quota gate. Currently §11 only lists 01/22/25/26/27/33/34/41 — no 21*. This is the explicit "ultrareview launch path" called out by the reviewer prompt.

### H2 — `src/cli/transports/` and `src/remote/SessionsWebSocket.ts` are described as if they share lineage; they don't
§13's transport catalog (HybridTransport, SSETransport, WebSocketTransport — all in `src/cli/transports/`) and §6.1's CCR `SessionsWebSocket` (in `src/remote/`) are presented as if they're a coherent stack. They're *parallel* implementations:
- `src/remote/SessionsWebSocket.ts` powers `RemoteSessionManager` (the viewer-side CCR bridge described in §3-§5) — uses `MAX_RECONNECT_ATTEMPTS=5`, `RECONNECT_DELAY_MS=2000`, no time budget, permanent codes `{4003}` only.
- `src/cli/transports/WebSocketTransport.ts` powers `getTransportForUrl`-selected transports (the SDK/CCR-v2 worker path) — uses time-budget-based reconnect (`DEFAULT_RECONNECT_GIVE_UP_MS=600_000`), 1s base / 30s max delay, permanent codes `{1002, 4001, 4003}`, has 4003-with-token-refresh recovery, has buffered-message replay via `lastSentId`/`X-Last-Request-Id`, has 5-min `keepAliveInterval` data frames.

The spec's §6.4 constants table mixes both without distinguishing — a reader will assume "the Sessions WS path" applies to all CCR traffic, but `cli/transports/WebSocketTransport.ts` is what actually carries the SDK session ingress. Spec needs a clear table separating the two transport stacks. The §6.6 line "WebSocketTransport gating: `cli/transports/WebSocketTransport.ts:771`" actually references *only* the keepalive-suppression branch under CLAUDE_CODE_REMOTE — it should not be folded into the broader §6.6 list without context.

### M1 — §3.1 incorrectly says `claude server` writes `~/.claude/server-lock.json`
Spec §3.1 / §3.3: "writes `~/.claude/server-lock.json`". I cannot find this string in `src/server/` or `src/main.tsx`. The grep only locates `server-sessions.json` (`src/server/types.ts:43`). The lock-file path is acknowledged as not in the leak (§12.5) — but §3.3 still asserts the filename. Either retract the filename to "lock file (path TBD per §12.5)" or cite the source.

### M2 — SSETransport reconnect/liveness numbers correct, but POST retry contract is missing
§13's SSETransport row says "POST retries" but doesn't capture: `POST_MAX_RETRIES=10`, `POST_BASE_DELAY_MS=500`, `POST_MAX_DELAY_MS=8000`, ±25% jitter, 4xx-non-429 = permanent drop. HybridTransport's row covers retries via `SerialBatchEventUploader`; SSETransport's POST path is *inline* (sleep+retry) not via the uploader, despite §13 framing them as parallel.

### M3 — Sandbox is mentioned as cross-spec target but the link is one-way
Reviewer asks about "sandbox toggle (cross-spec to /sandbox-toggle command)". `/sandbox-toggle` exists (`src/commands.ts:149`, `src/commands/sandbox-toggle/index.ts:47`) and `SandboxManager` is wired at `main.tsx:201,314-316`. Spec 35 never mentions sandboxing. If sandbox state interacts with CCR (e.g. `dangerouslySkipPermissions` direct-connect bypass at `directConnectManager`), §9 should cover the boundary. Otherwise an explicit "OUT of scope: sandbox → 42 / sandbox-toggle" would close the loop.

### M4 — `cc+unix://` scheme claimed but not verified
§3.1 / §4.3 mention `cc+unix://` URLs alongside `cc://`. The `parseConnectUrl.js` source isn't in the leak (acknowledged §12.5). Spec asserts this scheme as if confirmed; should mark with the same TBD treatment given to `serverBanner`.

## Verdict

**ACCEPT WITH REVISIONS.** Quality is high overall — the §6.4 constants table, §5 algorithms, and §6.1 protocol envelopes are correctly cited and load-bearing. Two HIGH issues (missing /ultrareview path, conflated transport stacks) merit a follow-up edit before Phase 10 is sealed. Other findings are tightenings.

## Cross-spec impact

- **21d** (command catalog plugin/misc): /ultrareview must appear in spec 35's §11 cross-refs; 21d should likewise back-link.
- **22** (api): SSETransport/HybridTransport POST retry semantics overlap `withRetry` — coordinate so `withRetry` isn't redundantly described.
- **25** (oauth): refresh-on-4003 in `cli/transports/WebSocketTransport.ts:428-438` is a behavior 25 likely doesn't cover (the closure-getAccessToken pattern in §9 is from `SessionsWebSocket`, not the SDK transport).
- **33** (daemon): self-hosted-runner is correctly deferred (`src/self-hosted-runner/` not in leak — verified absent).
- **34** (bridge): `BRIDGE_SAFE_COMMANDS` vs `REMOTE_SAFE_COMMANDS` distinction in §6.5 is correct and useful.
- **37a** (components): nine Teleport*.tsx files exist (TeleportError, TeleportProgress, TeleportRepoMismatchDialog, TeleportResumeWrapper, TeleportStash, ResumeTask refs); spec 35 doesn't claim them, which is correct (37a covers them).
- **42a** (utils long-tail): `src/utils/teleport/{api,environmentSelection,environments,gitBundle}.ts` exist; `sendEventToRemoteSession` is at `api.ts:361` — spec correctly defers (§12.6).

## Hardest-to-verify claim

§5.10 step 4: *"ssh -R unix-socket: reverse-forwards to a local auth proxy so the remote uses the local user's API token without re-login"*. `src/ssh/createSSHSession.ts` is **not in the leak** (verified: `src/ssh/` does not exist). Spec acknowledges this in §12.3 but states the reverse-forward mechanism in §5.10 step 4 with prose certainty drawn solely from a CLI-flag description string at `main.tsx:4046-4052`. The actual transport mechanics (whether it's `ssh -R`, what protocol the proxy speaks, how token refresh propagates back, what happens on host-key mismatch, what happens when the local proxy dies mid-session) are unverifiable from the leak alone. Treat all of §5.10 as "described from call-sites only" — spec already does this implicitly but a more prominent disclaimer at §5.10's head would help. Same caveat applies to §6.11 self-hosted-runner.
