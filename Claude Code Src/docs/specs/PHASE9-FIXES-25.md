# Phase 9.6c Fixes — Spec 25 (OAuth & macOS Keychain Auth)

Source: `docs/specs/PHASE9-ADVERSARIAL-25.md` (Phase 9.5b adversarial review).
Spec: `docs/specs/25-service-oauth-auth.md`.
Security-critical pass.

## Findings addressed

| ID | Severity | Status | Where |
|----|----------|--------|-------|
| F1 | HIGH | FIXED | §5.5 — added `isExpiredErrorType` substring-fragility callout (Phase 9.7 BUGS-IN-SOURCE candidate); enumerated silent-downgrade examples (`feature_expired`, `quota_lifetime_exceeded`); cross-ref spec 22, spec 34 |
| F2 | HIGH | FIXED (with corrected wording) | §5.5 — added `withOAuthRetry` `!deps.onAuth401 → return response` cross-spec wiring caveat; corrected Phase 9.6 spec 34's "immediate `BridgeFatalError`" wording per Phase 9.5b spec 33 reviewer (`bridgeApi.ts:117-120` *returns* the 401; fatal classification is downstream) |
| F3 | MED (verify-clean) | FIXED | §1 — added explicit refresh-model disclaimer: reactive only, no `setInterval`/`setTimeout` chain in `services/oauth/` or `utils/auth.ts`; the 30-min proactive-refresh risk lives in spec 34's bridge poll layer |
| F4 | MED | FIXED | §5.4 — documented profile-skip `??` chain perf risk: sync-memoized `getClaudeAIOAuthTokens()` may force a `security(1)` subprocess fork while holding the cross-process refresh lockfile; recommended fix to thread `lockedTokens` through |
| F5 | MED (NIT scope) | FIXED | Title — renamed to "OAuth & macOS Keychain Auth Service Specification (JWT consumer-only)" with explicit disclaimer header; cross-ref §6.17 / §12.1 |
| §6.13 | unfalsifiable | MOVED | §6.13 retains JS placeholder emission; same-length-rewrite invariant moved to §12 item 9 (Open Questions) |

## Source claims verified before edit

- `src/bridge/bridgeApi.ts:503-509` — `isExpiredErrorType = errorType.includes('expired') || errorType.includes('lifetime')` (verbatim).
- `src/bridge/bridgeApi.ts:117-120` — confirmed early-exit `if (!deps.onAuth401) { debug(...); return response }` *returns* the 401 (does not throw).
- `src/services/oauth/client.ts:201` — `existing = getClaudeAIOAuthTokens()` (sync), not `lockedTokens`.
- `src/services/oauth/client.ts:241-258` — `subscriptionType: profileInfo?.subscriptionType ?? existing?.subscriptionType ?? null` `??` chain confirmed.
- `src/utils/auth.ts:1519` — `clearOAuthTokenCache()` invoked before lockedTokens reload, confirming sync-cache miss under lock.
- `src/services/oauth/`, `src/utils/auth.ts` — no `setInterval`/`setTimeout` for refresh; `pendingRefreshCheck` is promise-dedup only.
- Spec title — no JWT issuer/verifier in `src/services/oauth/`; only opaque bearer consumption.

## Findings deferred (LOW / NIT)

- F6 (LOW) — trusted-device enrollment ordering: deferred to spec 21a / 34 review pass.
- F7 (LOW) — `CLAUDE_CODE_OAUTH_TOKEN` `expiresAt: null` security implication: noted but not a Phase 9.6c blocker.
- F8 (LOW) — auth-code-listener error redirect lands on success URL: §7 captures TODO.
- F9 (LOW) — `keychainLockedCache` SSH unlock-mid-session: §4.5 mentions process-lifetime; refinement deferred.
- F10 (LOW) — bare-mode `getAuthTokenSource` vs `getAnthropicApiKeyWithSource` env-precedence asymmetry: deferred.
- F11/F12 (NIT) — already correct; no action.

## Cross-spec ripple

- **Spec 22** — consumes `handleOAuth401Error`; F1 (`isExpiredErrorType`) and F2 (`onAuth401` wiring) both propagate to API-client retry path.
- **Spec 34** — owns the `withOAuthRetry` wrapper and `isExpiredErrorType` consumers (`bridgeMain.ts:1245,1261`, `replBridge.ts:2274`); F1/F2 jointly owned. Spec 34 reviewer should verify the corrected "fatal happens downstream, not in the wrapper" wording lands there too.
- **Spec 21a** — `installOAuthTokens` ordering still load-bearing.
- **Phase 9.7 BUGS-IN-SOURCE** — F1 added as a candidate (substring classifier on a security-relevant boundary).
