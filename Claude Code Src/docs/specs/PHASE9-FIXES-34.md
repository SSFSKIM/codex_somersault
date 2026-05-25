# Phase 9.6 B-full Fix Log — Spec 34 (Bridge Mode)

**Scope**: Apply remaining adversarial findings from `PHASE9-ADVERSARIAL-34.md` after B-mini landed CRIT (RPC count Eight → Nine).
**Status**: All H1, H2, H3 + M1, M2, M3 applied. Source unchanged (read-only).

---

## Findings applied

| ID | Sev | Section(s) edited | Finding | Resolution |
|---|---|---|---|---|
| F3 (CRIT) | — | §3.3 | RPC count Eight → Nine | **Already fixed by B-mini.** Confirmed line 200 reads "Nine RPCs". No re-edit. |
| H1 | HIGH | §5.8, §9.4 | `withOAuthRetry` flow chart omitted `!deps.onAuth401 → return response` short-circuit; daemon-mode callers without a refresh handler get *immediate* `BridgeFatalError(401)` on first 401, not "retry once". | §5.8 pseudocode now flags `(*) no refresh handler` branch with explicit comment. Added "Daemon-mode caveat" paragraph stating no retry path exists for env-var/daemon callers. §9.4 cross-references §5.8. |
| H2 | HIGH | §5.9 (table + footnote) | 410 row said `"detail OR expired-message"` — wrong precedence. Source uses `detail ?? expired-message` (detail wins **only when non-null**); `errorType` defaults to `'environment_expired'` regardless of detail. `isExpiredErrorType` is brittle substring match (`'expired'\|'lifetime'`). | Rewrote 404 + 410 rows with precise `??` semantics and explicit "detail null/undefined" wording. Added paragraph documenting substring-match brittleness with `'lifetime_extension_pending'` false-positive example. Flagged isExpiredErrorType for BUGS-IN-SOURCE.md. |
| H3 | HIGH | §9.3, §5.3 step 7c, §6.1 row, §11 item 15 | Spec implied 4090/4091/4092 are wire-protocol close codes. Verified all three are client-synthesized in `replBridgeTransport.ts:220, 313, 365` and never traverse the network. | §9.3 rewritten with **CLIENT-SYNTHESIZED** banner and per-code synthesis-site citations. §5.3 / §6.1 / §11 cross-references annotated. Telemetry-consumer guidance added: grep CLI-side logs only. |
| M1 | MED | §9.4 | Token-refresh chain unbounded on success. `MAX_REFRESH_FAILURES=3` only caps the failure path; every successful refresh schedules a fresh 30-min follow-up. | Added bullet documenting unbounded success path, citing `jwtUtils.ts:217-226`. Enumerated all `cancelAll()` call sites (`remoteBridgeCore.ts:503, 667`; `bridgeMain.ts:1470`). Flagged absence of `cancelAll()` in v1 REPL bridge teardown as a leak risk for reimpls. |
| M2 | MED | §5.7 | Trusted-device clear-then-enroll race during `/login` not flagged. | Added 3-point race description: header omission window, 10-min `account_session.created_at` server gate, fire-and-forget ordering requirement. Cited `commands/login/login.tsx:40-42` and `trustedDevice.ts:25-26, 67-72, 95`. |
| M3 | MED | §6.1 (constants), §9.11 (new) | SSETransport POST retry contract missing. | Added 4 constants rows to §6.1 (`POST_MAX_RETRIES=10`, `POST_BASE_DELAY_MS=500`, `POST_MAX_DELAY_MS=8000`, ±25% jitter). New §9.11 documents backoff formula and the **silent-drop-after-10** semantic — reimpls must not promote exhaustion to `BridgeFatalError`. |

## Findings NOT addressed (out of scope for B-full)

- **F1 cross-cite at §3.3**: F1's "withOAuthRetry is internal-only" was already implicit; H1 fix now states this explicitly in §5.8. No additional edit needed.
- **LOW (stale)**: §3.4 line range `main.tsx:4322-4325` flagged as imprecise — left as-is; spec body already shows the exact text. Cosmetic.
- **MEDIUM (untestable)**: §11 item 22 "3× margin under server 60s TTL" — server-side claim, no client citation possible. Left as-is.
- **MEDIUM (split-host)**: `sessionIngressUrl` vs `apiBaseUrl` divergent-DNS reconnect behavior — not in scope of provided findings list. Defer to next pass.
- **LOW (drift)**: §6.1 transport-default vs schema-default heartbeat mismatch — flagged in adversarial but resolution requires GrowthBook precedence claim outside `bridge/`. Left as-is.

---

## Top 3 fixes (ranked by reimpl impact)

1. **§9.3 client-synthesized close codes (H3)** — telemetry teams reading the prior text would have searched server logs and packet captures for 4090/4091/4092 indefinitely. The fix redirects them to CLI-side logs and names the exact synthesis sites.
2. **§5.8 daemon-mode no-refresh branch (H1)** — daemon and env-var-auth callers (the entire spec-33 path) previously read as "single retry on 401". Reimpls would have built a refresh handler that never gets invoked. The new caveat tells implementers to surface daemon-mode 401s immediately as `BridgeFatalError`, no retry.
3. **§9.11 SSETransport POST silent-drop (M3)** — reimpls without this constant table tend to (a) shorten the retry budget and (b) promote exhaustion to fatal. Both regressions break long-running bridges. The new section pins the contract.

---

## Source bugs surfaced (route to BUGS-IN-SOURCE.md)

1. **`isExpiredErrorType` substring matching is brittle** (`bridgeApi.ts:503-508`). `errorType.includes('expired') || errorType.includes('lifetime')` will false-positive on any future server error type containing the literal token `lifetime` (e.g. `'lifetime_extension_pending'`, `'lifetime_warning'`, `'lifetime_extension_required'`). Recommendation: replace with a closed allow-set (`['environment_expired', 'session_expired', 'lifetime_exceeded']`) or require an exact match. Risk: silently misclassifies non-expiry 403/410s as expired-session and triggers user-facing "session has expired" messaging plus expired-session teardown branch (`replBridge.ts:2274`, `bridgeMain.ts:1245-1261`).

2. **v1 REPL bridge teardown does not invoke `tokenRefresh.cancelAll()`** (verified absence: `grep cancelAll src/bridge/replBridge.ts` returns no hits while `remoteBridgeCore.ts` and `bridgeMain.ts` both do). On v1 REPL teardown, the 30-min follow-up timer continues to fire (`onRefresh` becomes a no-op against the dead transport, but the timer keeps re-scheduling itself indefinitely). Per-process leak grows with bridge churn. Recommendation: wire `tokenRefresh?.cancelAll()` into v1 teardown alongside the v2 site at `remoteBridgeCore.ts:667`.

---

## Hidden state-machine quirks documented

1. **`withOAuthRetry` has FOUR exit paths**, not two: (a) success on first try, (b) success on retry, (c) original 401 returned because `!deps.onAuth401`, (d) original 401 returned because refresh failed OR retry also got 401. Paths (c) and (d) both surface as `BridgeFatalError(401)` via `handleErrorStatus`, but (c) fires *without* attempting refresh — daemon callers will never see a refresh round-trip.

2. **Token-refresh scheduler self-perpetuates** unless `cancelAll()` is invoked. `doRefresh` schedules its own follow-up at `FALLBACK_REFRESH_INTERVAL_MS` *inside the success branch*, and `failureCounts.delete(sessionId)` runs on success — so a flapping connection that alternates success/failure never exhausts the failure cap.

3. **Trusted-device clear/enroll order is load-bearing**: `clearTrustedDeviceToken()` runs synchronously before `void enrollTrustedDevice()` so that bridge HTTP calls fired from concurrent `/login` post-hooks send no token rather than the previous account's token. Reimpls that await `enrollTrustedDevice()` will block the post-login UI for the round-trip; reimpls that drop the explicit `clearTrustedDeviceToken()` call will leak the prior account's token in the race window.

4. **SSETransport POST retry exhaustion is silent**: 10 failed attempts → log + continue, **not** throw. Bridge stays attached; outbound event is dropped. This is intentional best-effort delivery for `worker/events`; reimpls that promote to fatal will break across normal server blips.

5. **WS close codes 4090/4091/4092 are local conventions**: they exist purely so CLI telemetry can disambiguate three teardown causes that all funnel through one `onClose` callback. They're not part of any RFC, never sent by the server, and the numeric values are stable contract only with respect to CLI-side log consumers.
