# Phase 9.5 Adversarial Review — Spec 34 (Bridge Mode)

**Reviewer**: Opus fallback (general-purpose agent ad952540aa9d10b25)
**Spec**: `docs/specs/34-mode-bridge.md` (1,081 lines)
**Source**: `src/bridge/` (30 files), `src/hooks/useReplBridge.tsx`, `src/cli/transports/`
**Verdict**: ACCEPT WITH FIXES — spec is high-fidelity but contains a handful of factual drift points and missing edge cases.

## Severity counts

- Critical (blocks reimpl correctness): **1**
- High (factual contradiction with src/): **3**
- Medium (missing edge case / underspecified): **5**
- Low (cosmetic, naming, stale): **4**

## Top 5 findings

### F1 — HIGH — `withOAuthRetry` retry semantics misstated (§5.8 / §9.4)
Spec §5.8 says: 401 → refresh → retry once → "if retry.status !== 401 return retry; **return response (caller's handleErrorStatus throws BridgeFatalError)**". But `bridgeApi.ts:124-138` actually returns the *original* 401 response when refresh **fails OR** when retry also returns 401. So `BridgeFatalError(401)` is thrown twice in the failure path: once if `onAuth401` is absent (no refresh handler at all), and once after the retry path. The spec also omits the `!deps.onAuth401 → return response` branch (line 117-120), which means daemon callers without a refresh handler get an *immediate* `BridgeFatalError`, not a "retry once" — this is contradicted by §5.8. §3.3 should note `withOAuthRetry` is internal only, not in the eight-RPC `BridgeApiClient` table.

### F2 — HIGH — `handleErrorStatus` 410 detail-OR-message wrong (§5.9 table)
Spec §5.9 row 410: `"detail OR expired-message"`. Source (`bridgeApi.ts:486-492`): the message is `detail ?? 'Remote Control session has expired...'` — i.e. detail wins **only if non-null**. Same template, but errorType defaults to `'environment_expired'` regardless of detail presence. The table should clarify "default errorType applied when server omits one". Additionally, `isExpiredErrorType` checks for `'expired' || 'lifetime'` substrings — this is a brittle string-match on server-controlled types and the spec does not warn about that (false-positives e.g. `'lifetime_extension_pending'`).

### F3 — CRITICAL — `BridgeApiClient` "eight RPCs" miscount (§3.3)
Spec §3.3 says "Eight RPCs over `axios`: register, poll, ack, stop, deregister, sendPermissionResponseEvent, archive, reconnect, **heartbeat**" — that's NINE. `types.ts:133-176` confirms 9 methods. The reimpl checklist (§11) and §6.5 must be cross-checked; downstream agents counting on the cardinality (e.g. test plan generators) will under-spec. Fix: change "Eight RPCs" → "Nine RPCs".

### F4 — MEDIUM — Missing edge case: token-refresh chain has no upper bound after first success (§5.6 reimpl item 23 / §9.4)
`jwtUtils.ts:217-226` schedules a follow-up at `FALLBACK_REFRESH_INTERVAL_MS = 30 min` *after* every successful refresh. If `getAccessToken` keeps returning a token but `onRefresh` is a no-op (e.g. transport already torn down between refresh and timer fire), the chain runs forever, leaking timers per session. Spec mentions `cancelAll()` but never says when the bridge calls it on permanent teardown for v2. Verify: spec §9.4 mentions cap of 3 only for the *failure* path (`MAX_REFRESH_FAILURES`), not the success path.

### F5 — MEDIUM — SSE close code 4092 is synthetic, not server-sent (§9.3)
Spec §9.3 lists 4092 as "SSE reconnect-budget exhausted" but `replBridgeTransport.ts:311-314` shows 4092 is **synthesized client-side** when `sse.setOnClose` fires with `undefined` (CCR's reconnect budget exhaustion, mapped locally). Important for telemetry: 4092 will *never* appear in network traces. Same for 4090/4091 — both client-synthesized. Spec implies these are wire-protocol close codes; they aren't.

## Cross-spec impact

- **Spec 25 (OAuth)**: §5.7 trusted-device enrollment correctly cites `account_session.created_at < 10min` server gate. But spec 34 §11 item 19 says "secureStorage used for trusted-device token" without flagging the `clearTrustedDeviceToken()` ordering requirement before re-enrollment on `/login` (spec 25 must own this race).
- **Spec 21d (login flow)**: Spec 34 says enrollment "Never blocks login flow" — verify 21d's post-login hook waits/awaits properly. `enrollTrustedDevice` is async best-effort but not awaited; bridge calls between login and enrollment send the *prior account's* token despite cache-clear (race window).
- **Spec 33 (Daemon)**: Spec correctly notes daemon callers omit `onAuth401`. But §3.3's "eight RPCs" omission obscures that `heartbeatWork` in daemon mode lacks any 401 path at all (no env-var refresh).
- **Spec 35 (remote-server)**: §12 Q8 acknowledges `tengu_ccr_bridge_multi_environment` deferred to spec 35 — good. But §1 OUT scope omits multi-session capacity-wake mechanics, which the spec body (§4.5 `createCapacityWake`) covers in scope. Boundary unclear.
- **Spec 41 (session state)**: §2.4 cites `bridge-pointer.json` as adjacency; §4.1 specifies path inside `getProjectsDir()`. Worktree fanout cap interaction with 41's transcript path scheme not cross-cited.

## Other findings

- **LOW (stale)**: §6.6 quotes `'Connect this terminal for remote-control sessions'` from `commands/bridge/index.ts:16` — verified accurate. But §3.4 quotes `'Connect your local environment for remote-control sessions via claude.ai/code'` from `main.tsx:4322-4325` — line range looks wrong (Commander descriptions usually at single line), needs a precise re-anchor.
- **LOW (naming)**: Spec uses "CCR = Code Cloud Runtime" (§1), but `bridgeEnabled.ts:18` says "CCR" without expansion; comments use both "Code Cloud Runtime" and "CCR v2" interchangeably. Some files reference "Bridge worker" (server-side) without disambiguation.
- **MEDIUM (false enumeration)**: §6.7 cross-cites `ALLOWED_IDE_TOOLS = ['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']` "verbatim" from `services/mcp/client.ts:568`. Not verified in this review (out of scope per spec); risk of drift if spec 23 list grew.
- **MEDIUM (untestable claim)**: §11 reimpl item 22 says "v2 default heartbeat 20_000 ms (3× margin under server 60s TTL)". The "3× margin under server 60s TTL" is a server-side claim with no client-side citation. Untestable from this codebase.
- **MEDIUM (missing edge)**: Spec does not address what happens when `sessionIngressUrl` and `apiBaseUrl` diverge (§4.4 BridgeConfig has both fields) and one rotates DNS mid-session. Reconnect uses `apiBaseUrl`; SSE keeps `sessionIngressUrl`. Spec §5.2 line 13 mentions ANT-only override but production split-host behavior is undocumented.
- **LOW (drift)**: §6.1 lists `Default v2 heartbeat = 20_000 ms, jitter = 0` (transport defaults) but two rows later `v2 heartbeat config schema default 20_000, jitter default 0.1`. The transport's per-call default differs from the Zod schema default — flagged but not explained which wins when GrowthBook returns no value.
- **LOW (unverified)**: §11 item 25 about `isInBundledMode()` arg-prepending quirk references "gh-28334"; cannot verify external link from leaked source.

## Hardest-to-verify claim

> §5.3 step 7c: "WS close codes — 4090 = epoch superseded (409); 4091 = init failure; 4092 = SSE reconnect-budget exhausted"

These codes are **client-synthesized** and never traverse the network (see F5). Verifying their semantics requires reading `CCRClient.onEpochMismatch`, `SSETransport`'s reconnect-budget logic, and `ccr.initialize`'s rejection path simultaneously. The 4090 path further depends on a `throw` in `onEpochMismatch` that the spec correctly identifies (§9.8) but whose interaction with `SerialBatchEventUploader`'s catch block is verifiable only by reading both transport files end-to-end. No bridge-internal code asserts these semantics — they're cross-module conventions.

## Verdict

**ACCEPT WITH FIXES (4 high/critical to land before merge)**:
1. Fix "eight RPCs" → "nine RPCs" (F3, critical; downstream tests will under-cover heartbeat).
2. Correct `withOAuthRetry` flow chart (F1) — daemon-no-refresh path is currently misdocumented.
3. Annotate close codes 4090/4091/4092 as "client-synthesized, not wire codes" (F5).
4. Fix §5.9 410-row template to make detail-OR semantics precise (F2).

The spec is otherwise unusually thorough — verbatim citations match source, constants table is accurate to line numbers, gate names verified. Strongly recommend incorporating fixes before downstream specs (33, 35) reference §3.3's RPC count.
