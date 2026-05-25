# Phase 9.5b Adversarial Review — Spec 25 (OAuth, JWT & macOS Keychain Auth)

Reviewer role: Skeptic. Read-only verification against `src/services/oauth/`, `src/utils/auth.ts`, `src/bridge/bridgeApi.ts`, `src/bridge/bridgeMain.ts`, `src/bridge/replBridge.ts`. Security-critical pass.

## Severity counts

- CRITICAL: 0
- HIGH: 2
- MEDIUM: 3
- LOW: 4
- NIT: 2

## Top findings

### F1 (HIGH) — `isExpiredErrorType` substring fragility (Phase 9.7 BUGS-IN-SOURCE) is NOT cited or documented.

`src/bridge/bridgeApi.ts:503-509` defines `isExpiredErrorType` as a naïve substring match: `errorType.includes('expired') || errorType.includes('lifetime')`. Spec 25 §10/§11 lists every other refresh/lock telemetry event, but never mentions this function or its callers (`bridgeMain.ts:1245,1261`, `replBridge.ts:2274`). The classifier is consumed at the boundary between OAuth-401 recovery and bridge-fatal-error path: a server adding any unrelated `errorType` containing the substring "expired" (e.g., `feature_expired`, `quota_lifetime_exceeded`) would silently downgrade an unrelated 4xx to an "info" status. Spec 25 must either cite spec 22/34 ownership or carry the warning, since §1 includes "the 1P credential authority" and §11 prescribes 401 recovery — `isExpiredErrorType` is the *gating predicate* on the partner side.

### F2 (HIGH) — `withOAuthRetry` early-exit `!deps.onAuth401 → return response` (Phase 9.6 spec 34 finding) is not flagged.

`src/bridge/bridgeApi.ts:117-120`:
```
if (!deps.onAuth401) {
  debug(...); return response   // 401 returned silently to caller
}
```
Spec 25 §5.5 documents `handleOAuth401Error` (auth.ts:1360) but does not mention that the bridge wrapper *is the only consumer* of the dedup'd handler in non-API-client code, nor that omitting `onAuth401` (e.g., misconfigured `createBridgeApiClient` callers) silently turns every 401 into a fatal `BridgeFatalError` rather than an attempted refresh. This is a real cross-spec gap with 34. The spec's §11 reimplementation checklist item 11 ("dedup by failed access token; force refresh only if same access token still resident") is correct for `handleOAuth401Error` itself but does not require partners to wire it.

### F3 (HIGH→MEDIUM) — Unbounded refresh-chain / "perpetual 30-min timer" (Phase 9.6 spec 34): spec 25 does NOT flag it, but the chain is NOT in this service.

Spec 34's finding is about `bridgeMain.ts` proactive refresh loops. I verified `src/bridge/bridgeMain.ts:2747` and `:2377` — these are *call sites*, not timers. There is no `setInterval` / `setTimeout` chain in `services/oauth/` or `utils/auth.ts`. `pendingRefreshCheck` (auth.ts:1425) is a *promise dedup*, not a timer. **Verdict:** the unbounded-chain risk lives in spec 34's bridge polling, not here. Spec 25 *correctly* does not document a timer that doesn't exist. Recommend a cross-ref note in §1 or §13 disclaiming "no proactive scheduling lives in this service; refresh is reactive (call-site or 401 path)."

### F4 (MEDIUM) — `refreshOAuthToken` profile-fetch skip is *correctly* documented but the `??` chain has a latent bug the spec misses.

`src/services/oauth/client.ts:209-249`: When `haveProfileAlready` is true, `profileInfo = null`; returned `subscriptionType = null ?? existing?.subscriptionType ?? null`. Spec §5.4 captures this correctly. **However**, the `existing` is read via the *sync memoized* `getClaudeAIOAuthTokens()` (client.ts:201) — not the freshly-locked `lockedTokens` from `auth.ts:1521`. Under the lock-and-refresh path, the memoize was just cleared at auth.ts:1519, so `existing` may re-trigger a sync `security` read on the lock holder while the *async* `lockedTokens` has the up-to-date value. Functionally correct but introduces a sync subprocess spawn inside a process holding the cross-process refresh lock. Spec is silent on this perf risk.

### F5 (MEDIUM) — Trusted-device enrollment cross-spec to 21d/34 has documentation hole on `tokenAccount` fallback.

Spec 25 §5.6 / §5.7 documents `installOAuthTokens` and `enrollTrustedDevice` but does not state that enrollment must follow `installOAuthTokens` ordering or who calls `enrollTrustedDevice` (called in cli/handlers/auth.ts? in the bridge?). The 10-min server-side window (§6.16) makes this load-bearing. Cross-spec to 21a is mentioned but the call-site for enrollment is left to the reader.

### F6 (MEDIUM) — JWT section §6.17 + §12.1 honestly admits "no JWT issuer/verifier in tree" but the spec title still includes "JWT".

The spec is titled "OAuth, JWT & macOS Keychain Auth Service Specification". §6.17 then concedes JWT signing/verifying lives outside the leak. This is technically correct (consumers of opaque bearer JWTs only) but the title oversells the scope. NIT-level naming concern, but for a security-critical spec the gap matters because readers may assume claim validation occurs here.

### F7 (LOW) — `CLAUDE_CODE_OAUTH_TOKEN` env var produces `expiresAt: null` which `isOAuthTokenExpired` treats as "never expired" (client.ts:344-353). Spec §4.1 documents the null path but does not enumerate the security implication: a leaked env-var token cannot be locally invalidated by expiry; only revocation server-side recovers. Fine, but worth a §9 "edge case" line.

### F8 (LOW) — `auth-code-listener.ts:111-115` "TODO: swap to a different url once we have an error page" causes error redirects to land on the *success* URL (`CLAUDEAI_SUCCESS_URL`). Spec §7 captures the TODO; §9 should call out user-perceived-success on error.

### F9 (LOW) — Stale-while-error for keychain reads (`macOsKeychainStorage.ts`) + `keychainLockedCache` process-lifetime caching: a keychain that becomes locked mid-session is cached `true` forever (auth.ts/macOsKeychainStorage.ts:198, spec §4.5 row "keychainLockedCache"). The spec mentions process-lifetime but not the SSH unlock-mid-session case explicitly.

### F10 (LOW) — Bare-mode `getAuthTokenSource` (auth.ts:157-162) returns `apiKeyHelper` even though `getAnthropicApiKeyWithSource` (`isBareMode` branch line 235-247) also reads `ANTHROPIC_API_KEY` env. The two functions diverge on env-precedence under `--bare`. Spec §5.2 shows both branches but does not flag the asymmetry.

### F11 (NIT) — §6.14 lists `CLAUDE_CODE_API_KEY_HELPER_TTL_MS` defaulting to 5 min; auth.ts:560 also documents the *execa* timeout as 10 min. Spec lists both; correct.

### F12 (NIT) — §6.16 lists `Refresh-lock max retries 5` and `1000 + Math.random()*1000 ms` backoff; verified at auth.ts:1451,1501. Correct.

## Verdict

**Pass with cross-spec qualifications.** Spec 25 is a strong, source-anchored security spec. It correctly disclaims absent JWT machinery, accurately captures the PKCE + lockfile + dedup architecture, and lists nearly every telemetry event. It is **incomplete** on (a) the `isExpiredErrorType` substring fragility (F1), (b) the bridge `!onAuth401` early-exit silent-fatal path (F2), and (c) the cross-spec disclaimer that no proactive refresh timer lives in this service (F3). None are blocking; all should be addressed in a §13 "Bugs in source observed during spec authoring" appendix. Recommend MERGE with patch.

## Cross-spec impact

- **Spec 22** (API client retry on 401): consumes `handleOAuth401Error`. F1/F2 propagate.
- **Spec 27** (managed settings): `isManagedOAuthContext()` truthiness gates `apiKeyHelper`/`ANTHROPIC_AUTH_TOKEN`. Spec 25 §5.2 covers; verify spec 27 mirrors.
- **Spec 34** (bridge auth): `withOAuthRetry`/`onAuth401` wiring is owned by 34 but the contract lives here. F2 is jointly owned.
- **Spec 35** (remote auth / CCR JWT): §12.7 correctly defers `CCR_OAUTH_TOKEN_FILE` to spec 35.
- **Spec 21a** (`/login` UX): §5.6 ordering (`performLogout` → `storeOAuthAccountInfo` → `saveOAuthTokensIfNeeded` → `clearOAuthTokenCache` → `fetchAndStoreUserRoles`) must match what 21a's command handler invokes.

## Hardest-to-verify claim

§6.13 / §11.16: "*The `cch=00000` placeholder is rewritten in-place by Bun's native HTTP path (`bun-anthropic/src/http/Attestation.zig`) immediately before the request body is sent; same-length replacement avoids `Content-Length` recomputation.*" — `bun-anthropic` is **not in the leak**. There is no JS-side rewriter or test fixture in `src/`, only the literal placeholder (`constants/system.ts:73-95`) and a comment. The same-length / no-Content-Length-recomputation invariant cannot be verified from this tree, and a native-side regression that *does* recompute or *fails to* rewrite would silently leak the placeholder upstream as a billing-attestation header. Spec authors flagged this in §12 but the claim sits in the verbatim asset section as if proven; recommend moving to §12.
