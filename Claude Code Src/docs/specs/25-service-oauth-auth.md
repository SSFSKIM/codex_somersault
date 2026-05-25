# 25 — OAuth & macOS Keychain Auth Service Specification (JWT consumer-only)

> **Title scope disclaimer (Phase 9.6c, F5):** Earlier drafts titled this spec
> "OAuth, JWT & macOS Keychain". The leak contains **no JWT issuer or verifier**
> in `src/services/oauth/` (no `signJwt`/`verifyJwt`, no JWT library import) —
> this service only **consumes opaque bearer tokens** that may or may not be
> JWTs server-side. Claim shape, signing algorithm, and key-rotation policy
> live in `bun-anthropic` / server code outside this dump. See §6.17, §12.1.

> **Sub-agent:** sub-F5 · **Phase:** 6 · **Adjacent:** 01, 22, 23, 34, 35
>
> Adjacent specs already cover, and are NOT redocumented here:
> - 01 — boot-time call site for `startKeychainPrefetch()` and `ensureKeychainPrefetchCompleted()`.
> - 22 — how the Anthropic API client consumes `apiKey` / `authToken` resolved by this service.
> - 23 — MCP server-specific OAuth (CIMD/SEP-991, dynamic client registration).
> - 34 — how the bridge consumes `X-Trusted-Device-Token`.
> - 35 — how `claude ssh` / CCR consume infrastructure-injected JWTs.
> - 21a — `/login` and `/logout` user-facing surface.

## 1. Purpose & Scope

The OAuth/auth service is the single authority for Anthropic-1P credentials inside Claude Code. It owns:

- The OAuth 2.0 Authorization Code + PKCE flow (browser-automatic and copy/paste-manual variants), including the localhost callback listener.
- Refresh-token rotation (`grant_type=refresh_token`) with a filesystem-locked, cross-process race-free refresh path.
- The `OAuthTokens` shape (access, refresh, expiry, scopes, subscription type, rate-limit tier, profile, account UUIDs) that every other Anthropic-1P client reads.
- API-key vs OAuth-token resolution priority, including all environment-variable, file-descriptor, `apiKeyHelper`, keychain, and config sources.
- `darwin` Keychain integration: `startKeychainPrefetch()` (parallel pre-import side effect at `src/main.tsx:20`), the `security(1)` subprocess read/write/delete protocol, the 30 s in-process cache, the readAsync de-dup/generation guard, the cross-process `.credentials.json` mtime invalidator, and the plain-text fallback file.
- The token storage layout (`SecureStorageData.claudeAiOauth`, `SecureStorageData.trustedDeviceToken`).
- Trusted-device enrollment (`POST /api/auth/trusted_devices`), the `tengu_sessions_elevated_auth_enforcement` GrowthBook gate, and the `X-Trusted-Device-Token` header consumed by spec 34.
- The `NATIVE_CLIENT_ATTESTATION` placeholder injected into the `x-anthropic-billing-header` and overwritten in-place by Bun's HTTP stack.
- 3P provider auth (Bedrock / Vertex / Foundry) only insofar as it disables the 1P OAuth path via `isAnthropicAuthEnabled()`.

> **Refresh model disclaimer (Phase 9.6c, F3 — verify-clean).** This service
> performs **reactive refresh only**: refresh is triggered at call sites
> (`checkAndRefreshOAuthTokenIfNeeded` invoked before token use) or by the
> 401 recovery path (`handleOAuth401Error`). There is **no `setInterval`
> / `setTimeout` chain in `src/services/oauth/` or `src/utils/auth.ts`**;
> `pendingRefreshCheck` (auth.ts:1425) is a promise-dedup, not a timer.
> Any "30-min proactive refresh loop" risk discussed in spec 34 lives in
> the bridge polling layer, *not* this service.

Out of scope (defer to adjacent spec):
- Anthropic SDK construction, retries, streaming, `x-api-key` vs `Authorization` header header selection beyond the resolved value → 22.
- MCP authorization-server discovery, DCR, CIMD, per-server token storage → 23.
- The bridge poll/heartbeat/ack request loop that *uses* the trusted-device token → 34.
- CCR JWT issuance and the `ANTHROPIC_UNIX_SOCKET` proxy that injects them → 35.
- The Ink components and message strings of `/login`, `/logout`, `/setup-token` → 21a.

## 2. Source Map

| Concern | Path |
|---|---|
| OAuth service entry, PKCE flow orchestration | `src/services/oauth/index.ts` |
| OAuth wire client (auth URL, code exchange, refresh, profile, API-key creation, roles) | `src/services/oauth/client.ts` |
| Localhost redirect listener | `src/services/oauth/auth-code-listener.ts` |
| PKCE / state crypto | `src/services/oauth/crypto.ts` |
| Profile fetch (OAuth bearer + API-key variants) | `src/services/oauth/getOauthProfile.ts` |
| OAuth/keychain endpoint constants, scope sets, env overrides | `src/constants/oauth.ts` |
| `NATIVE_CLIENT_ATTESTATION` placeholder | `src/constants/system.ts` |
| Token resolution priority, persistence, refresh, 401 recovery | `src/utils/auth.ts` |
| CLI handler — `installOAuthTokens()`, refresh-token env login | `src/cli/handlers/auth.ts` |
| Secure-storage façade (platform select) | `src/utils/secureStorage/index.ts` |
| Pre-import keychain prefetch (boot-critical) | `src/utils/secureStorage/keychainPrefetch.ts` |
| `security(1)` cache + helpers shared with prefetch | `src/utils/secureStorage/macOsKeychainHelpers.ts` |
| Sync/async `security(1)` read/write/delete | `src/utils/secureStorage/macOsKeychainStorage.ts` |
| `.credentials.json` plain-text fallback (mode 0600) | `src/utils/secureStorage/plainTextStorage.ts` |
| Primary-with-fallback composition + migration delete | `src/utils/secureStorage/fallbackStorage.ts` |
| Trusted-device enrollment + read | `src/bridge/trustedDevice.ts` |
| Boot-time prefetch fire + await | `src/main.tsx` (lines 17, 20, 914) |

The leak does not include `src/services/oauth/types.ts` or `src/utils/secureStorage/types.ts`; both are imported but not present in the dump (`OAuthTokens`, `OAuthTokenExchangeResponse`, `OAuthProfileResponse`, `SubscriptionType`, `RateLimitTier`, `BillingType`, `UserRolesResponse`, `SecureStorageData`, `SecureStorage`). Field shapes are reverse-engineered from call sites and recorded in §4.

## 3. Public Interface (Contract)

**Token acquisition / installation.**

```
new OAuthService()                                          // src/services/oauth/index.ts:21
OAuthService.startOAuthFlow(authURLHandler, options?)        // :32 -> Promise<OAuthTokens>
OAuthService.handleManualAuthCodeInput({authorizationCode,state})  // :157
OAuthService.cleanup()                                       // :194
installOAuthTokens(tokens: OAuthTokens): Promise<void>       // src/cli/handlers/auth.ts:50
```

`startOAuthFlow` options (all optional): `loginWithClaudeAi`, `inferenceOnly`, `expiresIn`, `orgUUID`, `loginHint`, `loginMethod`, `skipBrowserOpen` (`src/services/oauth/index.ts:34-48`).

**Wire-level operations.**

```
buildAuthUrl({...})                                          // src/services/oauth/client.ts:46
exchangeCodeForTokens(code, state, verifier, port, useManualRedirect?, expiresIn?)  // :107
refreshOAuthToken(refreshToken, {scopes?})                   // :146
fetchAndStoreUserRoles(accessToken)                          // :276
createAndStoreApiKey(accessToken)                            // :311
fetchProfileInfo(accessToken)                                // :355
isOAuthTokenExpired(expiresAt: number | null): boolean       // :344
parseScopes(scopeString?): string[]                          // :42
shouldUseClaudeAIAuth(scopes?): boolean                      // :38
getOrganizationUUID(): Promise<string | null>                // :426
populateOAuthAccountInfoIfNeeded(): Promise<boolean>         // :451
storeOAuthAccountInfo({...})                                 // :517
```

**Token resolution / persistence.**

```
isAnthropicAuthEnabled(): boolean                            // src/utils/auth.ts:100
getAuthTokenSource(): { source, hasToken }                   // :153
getAnthropicApiKey(): string | null                          // :214
getAnthropicApiKeyWithSource(opts?): { key, source }         // :226
hasAnthropicApiKeyAuth(): boolean                            // :219
saveOAuthTokensIfNeeded(tokens: OAuthTokens): {success,warning?}  // :1194
getClaudeAIOAuthTokens(): OAuthTokens | null                 // :1255 (memoized)
getClaudeAIOAuthTokensAsync(): Promise<OAuthTokens | null>   // :1399
clearOAuthTokenCache(): void                                 // :1308
checkAndRefreshOAuthTokenIfNeeded(retry?, force?): Promise<boolean>  // :1427
handleOAuth401Error(failedAccessToken): Promise<boolean>     // :1360
isClaudeAISubscriber(): boolean                              // :1564
hasProfileScope(): boolean                                   // :1580
is1PApiCustomer(): boolean                                   // :1586
getSubscriptionType(): SubscriptionType | null               // :1662
getOauthAccountInfo(): AccountInfo | undefined               // :1615
saveApiKey(apiKey: string): Promise<void>                    // :1094
removeApiKey(): Promise<void>                                // :1170
getApiKeyFromConfigOrMacOSKeychain (memoized)                // :1051
```

**API-key helper (sync cache, async fetch).**

```
getApiKeyFromApiKeyHelper(isNonInteractive): Promise<string|null>  // src/utils/auth.ts:469
getApiKeyFromApiKeyHelperCached(): string | null                    // :581
getApiKeyHelperElapsedMs(): number                                  // :464
clearApiKeyHelperCache(): void                                      // :585
prefetchApiKeyFromApiKeyHelperIfSafe(isNonInteractive): void        // :591
calculateApiKeyHelperTTL(): number                                  // :435
```

**Keychain prefetch.**

```
startKeychainPrefetch(): void                                // src/utils/secureStorage/keychainPrefetch.ts:69
ensureKeychainPrefetchCompleted(): Promise<void>             // :96
getLegacyApiKeyPrefetchResult(): { stdout: string|null }|null  // :104
clearLegacyApiKeyPrefetch(): void                            // :114
```

**Secure-storage façade.**

```
getSecureStorage(): SecureStorage                            // src/utils/secureStorage/index.ts:9
SecureStorage = { name, read(), readAsync(), update(data), delete() }
```

`darwin` returns `createFallbackStorage(macOsKeychainStorage, plainTextStorage)` with `name = "keychain-with-plaintext-fallback"`. All other platforms return raw `plainTextStorage` with `name = "plaintext"` (`src/utils/secureStorage/index.ts:9-17`). A TODO indicates Linux libsecret is not yet implemented.

**Trusted device.**

```
getTrustedDeviceToken(): string | undefined                  // src/bridge/trustedDevice.ts:54
clearTrustedDeviceToken(): void                              // :72
clearTrustedDeviceTokenCache(): void                         // :61
enrollTrustedDevice(): Promise<void>                         // :98
```

## 4. Data Model & State

### 4.1 `OAuthTokens` (reverse-engineered from `src/services/oauth/index.ts:175-191`, `src/services/oauth/client.ts:241-258`, `src/utils/auth.ts:1217-1229`, `1262-1283`)

```
OAuthTokens {
  accessToken:     string
  refreshToken:    string | null
  expiresAt:       number | null      // ms since epoch (Date.now()-relative)
  scopes:          string[]
  subscriptionType: SubscriptionType | null  // 'max' | 'pro' | 'enterprise' | 'team' | null
  rateLimitTier:   RateLimitTier | null
  profile?:        OAuthProfileResponse
  tokenAccount?:   { uuid: string; emailAddress: string; organizationUuid?: string }
}
```

`refreshToken` and `expiresAt` are `null` exactly when the source is the `CLAUDE_CODE_OAUTH_TOKEN` env var or the file-descriptor token (inference-only, scopes hardcoded to `['user:inference']`) — see `src/utils/auth.ts:1260-1283`.

### 4.2 `OAuthTokenExchangeResponse` (server payload — `src/services/oauth/index.ts:175-189`, `src/services/oauth/client.ts:175-189`)

```
{
  access_token:  string
  refresh_token: string
  expires_in:    number          // seconds -> ms*1000 added to Date.now() to form expiresAt
  scope:         string          // space-delimited
  account?:      { uuid: string; email_address: string }
  organization?: { uuid: string }
}
```

### 4.3 `SecureStorageData` (call-site shape)

```
SecureStorageData {
  claudeAiOauth?:      OAuthTokens (subset: accessToken,refreshToken,expiresAt,scopes,subscriptionType,rateLimitTier)
  trustedDeviceToken?: string
  // additional fields written by other subsystems are preserved by read-modify-write callers
}
```

Persistence skipped when `!shouldUseClaudeAIAuth(scopes)` or when `refreshToken`/`expiresAt` are null (`src/utils/auth.ts:1198-1207`).

### 4.4 OAuth account info (`AccountInfo`, persisted in `globalConfig.oauthAccount`)

Fields written by `storeOAuthAccountInfo` (`src/services/oauth/client.ts:517-566`): `accountUuid`, `emailAddress`, `organizationUuid`, `displayName?`, `hasExtraUsageEnabled?`, `billingType?`, `accountCreatedAt?`, `subscriptionCreatedAt?`. Additional fields written by `fetchAndStoreUserRoles`: `organizationRole`, `workspaceRole`, `organizationName` (`src/services/oauth/client.ts:293-303`).

### 4.5 Module-level mutable state

| State | File:line | Semantics |
|---|---|---|
| `keychainCacheState.cache: { data, cachedAt }` | `macOsKeychainHelpers.ts:71-85` | 30 s TTL cache of decoded keychain JSON. `cachedAt = 0` => invalid. |
| `keychainCacheState.generation` | same | Bumped on every `clearKeychainCache()`; readAsync skips stale subprocess writes. |
| `keychainCacheState.readInFlight` | same | Dedups concurrent async reads (TTL expiry under load). |
| `legacyApiKeyPrefetch: { stdout: string\|null } \| null` | `keychainPrefetch.ts:39` | `null` = not started; `{stdout:null}` = completed with no key. |
| `prefetchPromise` | `keychainPrefetch.ts:41` | Single prefetch fire per process. |
| `_apiKeyHelperCache`, `_apiKeyHelperInflight`, `_apiKeyHelperEpoch` | `auth.ts:456-462` | SWR cache for `apiKeyHelper`. Epoch bumps on `clearApiKeyHelperCache()`. |
| `lastCredentialsMtimeMs` | `auth.ts:1313` | Cross-process invalidator for `.credentials.json`. |
| `pending401Handlers: Map<failedToken, Promise>` | `auth.ts:1343` | Dedup of concurrent 401 recovery. |
| `pendingRefreshCheck` | `auth.ts:1425` | Dedups non-force `checkAndRefreshOAuthTokenIfNeeded()`. |
| `keychainLockedCache` | `macOsKeychainStorage.ts:198` | Process-lifetime cache; only re-set on first call. |
| `readStoredToken` (lodash memoize) | `trustedDevice.ts:45-52` | Cleared on enroll, logout, token-clear. |

## 5. Algorithm / Control Flow

### 5.1 Boot-time keychain prefetch

```
main.tsx:5  startupProfiler import (warms envUtils, oauth constants, crypto, os)
main.tsx:17 import { startKeychainPrefetch, ensureKeychainPrefetchCompleted }
main.tsx:20 startKeychainPrefetch()        // fires execFile spawn x2, NON-BLOCKING
... (~65 ms of subsequent imports) ...
main.tsx:914 await Promise.all([ensureMdmSettingsLoaded(), ensureKeychainPrefetchCompleted()])
```

Prefetch (`keychainPrefetch.ts:69-89`):

```
if (process.platform !== 'darwin' || prefetchPromise || isBareMode()) return
oauthSpawn  = spawnSecurity(getMacOsKeychainStorageServiceName('-credentials'))
legacySpawn = spawnSecurity(getMacOsKeychainStorageServiceName())
prefetchPromise = Promise.all([oauthSpawn, legacySpawn]).then(([oauth, legacy]) => {
  if (!oauth.timedOut)  primeKeychainCacheFromPrefetch(oauth.stdout)
  if (!legacy.timedOut) legacyApiKeyPrefetch = { stdout: legacy.stdout }
})
```

`spawnSecurity` runs `security find-generic-password -a $USER -w -s <serviceName>` with `KEYCHAIN_PREFETCH_TIMEOUT_MS = 10_000` (`keychainPrefetch.ts:33,45-63`). Exit non-zero with `err.killed === true` => `timedOut: true` (don't prime). All other non-zero (e.g. exit 44 = entry not found) => `stdout: null` (safe to prime).

`primeKeychainCacheFromPrefetch` only writes if `keychainCacheState.cache.cachedAt === 0` (sync `read()` or `update()` haven't run yet) (`macOsKeychainHelpers.ts:98-111`). Uses `JSON.parse` directly to avoid pulling `slowOperations` into the early-startup chain.

### 5.2 Token resolution priority (`getAuthTokenSource()`, `src/utils/auth.ts:153-206`)

```
if isBareMode():
   getConfiguredApiKeyHelper() ? 'apiKeyHelper' : 'none'
if ANTHROPIC_AUTH_TOKEN && !isManagedOAuthContext()   -> 'ANTHROPIC_AUTH_TOKEN'
if CLAUDE_CODE_OAUTH_TOKEN                            -> 'CLAUDE_CODE_OAUTH_TOKEN'
if getOAuthTokenFromFileDescriptor():
   if CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR        -> 'CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR'
   else                                              -> 'CCR_OAUTH_TOKEN_FILE'
if apiKeyHelper && !isManagedOAuthContext()          -> 'apiKeyHelper'
if shouldUseClaudeAIAuth(oauthTokens.scopes) && oauthTokens.accessToken  -> 'claude.ai'
else                                                 -> 'none'
```

`isManagedOAuthContext()` <=> `CLAUDE_CODE_REMOTE` truthy OR `CLAUDE_CODE_ENTRYPOINT === 'claude-desktop'` (`src/utils/auth.ts:91-96`).

API-key-only resolution (`getAnthropicApiKeyWithSource()`, `src/utils/auth.ts:226-348`):

```
bare:        ANTHROPIC_API_KEY -> apiKeyHelper -> none
homespace:   skip ANTHROPIC_API_KEY env entirely (use Console key path)
preferThirdPartyAuthentication() && apiKeyEnv -> ANTHROPIC_API_KEY
CI || NODE_ENV==='test':
   getApiKeyFromFileDescriptor() -> ANTHROPIC_API_KEY env (else throw if no OAuth env)
ANTHROPIC_API_KEY env in customApiKeyResponses.approved -> ANTHROPIC_API_KEY
getApiKeyFromFileDescriptor() -> ANTHROPIC_API_KEY
apiKeyHelper configured -> 'apiKeyHelper' (sync cache; never blocks)
getApiKeyFromConfigOrMacOSKeychain() -> '/login managed key'
else null/'none'
```

### 5.3 PKCE flow (`OAuthService.startOAuthFlow`)

```
codeVerifier   = base64url(randomBytes(32))                // crypto.ts:11
codeChallenge  = base64url(sha256(codeVerifier))           // crypto.ts:15
state          = base64url(randomBytes(32))                // crypto.ts:21
listener       = new AuthCodeListener(); port = await listener.start()  // OS-assigned port
manualUrl      = buildAuthUrl({...,isManual:true})         // redirect_uri = MANUAL_REDIRECT_URL
automaticUrl   = buildAuthUrl({...,isManual:false})        // redirect_uri = http://localhost:{port}/callback
authCode       = await waitForAuthorizationCode(state, () => {
   if skipBrowserOpen: authURLHandler(manualUrl, automaticUrl)
   else:               authURLHandler(manualUrl); openBrowser(automaticUrl)
})
isAutomatic    = listener.hasPendingResponse()
tokenResponse  = await exchangeCodeForTokens(authCode, state, codeVerifier, port, !isAutomatic, opts.expiresIn)
profileInfo    = await fetchProfileInfo(tokenResponse.access_token)
if isAutomatic: listener.handleSuccessRedirect(parseScopes(tokenResponse.scope))
else if error:  listener.handleErrorRedirect()
finally:        listener.close()
return formatTokens(tokenResponse, profileInfo.subscriptionType, profileInfo.rateLimitTier, profileInfo.rawProfile)
```

Listener (`auth-code-listener.ts:134-175`): handles GET `<callbackPath>` only, validates `state === expectedState` (CSRF), captures `code`, stores `pendingResponse` for later 302 redirect. Mismatch => HTTP 400 + reject. No code => HTTP 400 + reject. Other paths => HTTP 404.

### 5.4 Refresh path (`refreshOAuthToken` + `checkAndRefreshOAuthTokenIfNeeded`)

`isOAuthTokenExpired(expiresAt)` returns `true` iff `Date.now() + 5*60*1000 >= expiresAt` (5-minute buffer; `null` => never expired) (`src/services/oauth/client.ts:344-353`).

`checkAndRefreshOAuthTokenIfNeededImpl` (`src/utils/auth.ts:1447-1561`):

```
MAX_RETRIES = 5
await invalidateOAuthCacheIfDiskChanged()    // stat .credentials.json mtime
tokens = getClaudeAIOAuthTokens() (sync cache)
if !force and (!refreshToken or !expired):    return false
if !refreshToken:                              return false
if !shouldUseClaudeAIAuth(scopes):             return false
clearKeychainCache(); freshTokens = await getClaudeAIOAuthTokensAsync()
if !freshTokens.refreshToken or !expired:     return false
mkdir(claudeDir); release = await lockfile.lock(claudeDir)
   `-> ELOCKED: sleep 1000 + Math.random()*1000 ms; retry up to MAX_RETRIES
clear caches; lockedTokens = await getClaudeAIOAuthTokensAsync()
if !expired (race winner already refreshed): return false  // 'tengu_oauth_token_refresh_race_resolved'
refreshedTokens = await refreshOAuthToken(lockedTokens.refreshToken,
   {scopes: shouldUseClaudeAIAuth(...) ? undefined : lockedTokens.scopes})
saveOAuthTokensIfNeeded(refreshedTokens); clear caches
finally: await release()
```

`refreshOAuthToken` (`src/services/oauth/client.ts:146-274`):

- POST `TOKEN_URL` with `grant_type=refresh_token`, `refresh_token`, `client_id`, `scope = (requestedScopes ?? CLAUDE_AI_OAUTH_SCOPES).join(' ')`. Backend allows scope expansion via `ALLOWED_SCOPE_EXPANSIONS`.
- Timeout: **15000 ms**.
- Skips `/api/oauth/profile` (`fetchProfileInfo`) when both `globalConfig.oauthAccount.{billingType,accountCreatedAt,subscriptionCreatedAt}` are defined AND existing keychain `subscriptionType`/`rateLimitTier` are non-null. Cuts ~7M req/day fleet-wide. Uses `??` to preserve existing values across the `CLAUDE_CODE_OAUTH_REFRESH_TOKEN` re-login path (where `performLogout` wipes secure storage AFTER return).
- New `expiresAt = Date.now() + expires_in*1000`. Reuses old `refresh_token` if response omits one.
- On error: logs `tengu_oauth_token_refresh_failure` with response body slice.

> **Phase 9.6c (F4) — perf footgun in profile-skip `??` chain.**
> `refreshOAuthToken` at `src/services/oauth/client.ts:201` reads `existing`
> via the **sync-memoized** `getClaudeAIOAuthTokens()`, not the freshly-locked
> async `lockedTokens` available to the caller in `auth.ts:1521`. Under the
> lock-and-refresh path, `clearOAuthTokenCache()` was just invoked at
> `auth.ts:1519`, so `existing` may force a **synchronous `security(1)`
> subprocess spawn** (~30-100 ms on macOS) **while this process holds the
> cross-process refresh lockfile**. Other CLI processes contending for the
> same lock observe `ELOCKED` and incur the 1-2 s jittered backoff.
> Functionally correct (the `??` chain still preserves paying-user state),
> but every refresh on a cold sync cache pays a subprocess fork inside the
> critical section. Recommended fix: thread `lockedTokens` through as an
> optional param so the existing-value lookup avoids the sync read. See
> spec 22 for the API-client retry path that triggers this most frequently.

### 5.5 401 recovery (`handleOAuth401Error`, `src/utils/auth.ts:1360-1392`)

```
dedupe by failedAccessToken in pending401Handlers Map
clearOAuthTokenCache()
currentTokens = await getClaudeAIOAuthTokensAsync()
if !currentTokens.refreshToken:                   return false
if currentTokens.accessToken !== failedAccessToken:
   logEvent('tengu_oauth_401_recovered_from_keychain')
   return true                                     // another tab refreshed
return checkAndRefreshOAuthTokenIfNeeded(0, /*force=*/true)
```

**Cross-spec wiring caveat (Phase 9.6c, F2 — joint with spec 34).** This
service exposes `handleOAuth401Error` but does *not* own the call site that
wires it into the bridge. `src/bridge/bridgeApi.ts:117-120` (`withOAuthRetry`):

```ts
if (!deps.onAuth401) {
  debug(`[bridge:api] ${context}: 401 received, no refresh handler`)
  return response  // raw 401 propagates to caller
}
```

When a `createBridgeApiClient` caller omits `onAuth401`, the 401 is **returned
unchanged** — `withOAuthRetry` does *not* itself raise `BridgeFatalError`;
the fatal classification happens downstream when the caller surfaces the
401 status. (Earlier Phase 9.6 spec 34 wording "immediate `BridgeFatalError`"
overstated the proximate cause; corrected per Phase 9.5b spec 33 reviewer.)
Net effect is still a missed refresh attempt and a user-visible auth failure
where one was recoverable. Spec 34 owns the wrapper; this spec owns the
handler contract; partners reimplementing must wire both ends.

**`isExpiredErrorType` substring fragility (Phase 9.6c, F1 — Phase 9.7
BUGS-IN-SOURCE candidate).** The bridge classifier at
`src/bridge/bridgeApi.ts:503-509` is a naïve `String.prototype.includes`
match:

```ts
return errorType.includes('expired') || errorType.includes('lifetime')
```

Consumers: `src/bridge/bridgeMain.ts:1245,1261`, `src/bridge/replBridge.ts:2274`.
This predicate gates the boundary between OAuth-401 recovery and the
bridge-fatal-error path. If the server ever introduces an unrelated
`errorType` whose name contains `'expired'` (e.g., `feature_expired`,
`trial_expired`) or `'lifetime'` (e.g., `quota_lifetime_exceeded`), the
4xx is **silently downgraded to a session-expiry "info" status** and the
user sees a re-auth prompt instead of the real error. Recommended fix:
exact-string match against an enumerated allow-list. Tracked under Phase 9.7
BUGS-IN-SOURCE; cross-ref spec 22 (API client retry consumes the same
predicate's outcome) and spec 34 (bridge fatal classification).

### 5.6 `installOAuthTokens` (post-token-acquisition glue, `src/cli/handlers/auth.ts:50-110`)

```
performLogout({clearOnboarding: false})           // wipes prior state
profile = tokens.profile ?? await getOauthProfileFromOauthToken(tokens.accessToken)
if profile:           storeOAuthAccountInfo({...full profile})
elif tokenAccount:    storeOAuthAccountInfo({uuid,email,orgUUID})  // fallback
saveOAuthTokensIfNeeded(tokens); clearOAuthTokenCache()
fetchAndStoreUserRoles(accessToken).catch(log)
if shouldUseClaudeAIAuth(scopes):
   fetchAndStoreClaudeCodeFirstTokenDate().catch(log)
else:
   apiKey = await createAndStoreApiKey(accessToken)   // throws if null
clearAuthRelatedCaches()
```

Console flow => `createAndStoreApiKey` => POST `API_KEY_URL` with bearer; response `data.raw_key` saved via `saveApiKey()` (`src/services/oauth/client.ts:311-342`).

### 5.7 Trusted-device enrollment (`enrollTrustedDevice`, `src/bridge/trustedDevice.ts:98-210`)

```
if !(await checkGate_CACHED_OR_BLOCKING('tengu_sessions_elevated_auth_enforcement')): return
if process.env.CLAUDE_TRUSTED_DEVICE_TOKEN: return    // env var precedence
accessToken = require('../utils/auth.js').getClaudeAIOAuthTokens()?.accessToken
if !accessToken: return
if isEssentialTrafficOnly(): return
POST {BASE_API_URL}/api/auth/trusted_devices
   Authorization: Bearer {accessToken}
   body: { display_name: "Claude Code on {hostname()} - {process.platform}" }
   timeout: 10_000, validateStatus: s => s < 500
on 200|201:
   token = response.data.device_token
   storage = getSecureStorage().read(); storage.trustedDeviceToken = token; update(storage)
   readStoredToken.cache.clear()
```

Server-side gate: enrollment only succeeds when `account_session.created_at < 10 min`, so enrollment must follow `/login` immediately.

`getTrustedDeviceToken()` returns `process.env.CLAUDE_TRUSTED_DEVICE_TOKEN` if set, else `getSecureStorage().read()?.trustedDeviceToken`, gated by the same GrowthBook flag. Token rolls every 90 days server-side.

### 5.8 macOS Keychain sync read (`macOsKeychainStorage.read()`, `macOsKeychainStorage.ts:28-66`)

```
if Date.now() - prev.cachedAt < KEYCHAIN_CACHE_TTL_MS (30_000):
   return prev.data
result = execSync('security find-generic-password -a "$USER" -w -s "<serviceName>"')
if result:
   data = jsonParse(result); cache = {data, cachedAt: Date.now()}; return data
on error or empty:
   if prev.data !== null: log "[keychain] read failed; serving stale cache"; refresh timestamp; return stale
   else: cache = {data:null, cachedAt: now}; return null
```

`readAsync()` (`:67-96`) captures `keychainCacheState.generation` before spawning; if `clearKeychainCache()` has bumped the generation, the subprocess result is discarded and not cached, so the in-flight result can't clobber a fresh `update()`.

### 5.9 macOS Keychain write (`update()`, `:97-158`)

```
clearKeychainCache()
hexValue = Buffer.from(JSON.stringify(data),'utf-8').toString('hex')
command  = `add-generic-password -U -a "${user}" -s "${service}" -X "${hexValue}"\n`
SECURITY_STDIN_LINE_LIMIT = 4096 - 64 = 4032
if command.length <= 4032:
   execaSync('security', ['-i'], { input: command, ... })   // hides hex from `ps`
else:
   execaSync('security', [add-generic-password,...,-X,hexValue]) // argv visible in `ps`; warns
on exit 0: cache = {data, cachedAt: now}; return {success:true}
else:      return {success:false}
```

The 4032-byte limit exists because `security -i` reads stdin with a 4096-byte fgets() buffer; longer commands silently corrupt prior keychain entries (#30337).

### 5.10 Fallback storage composition (`fallbackStorage.ts`)

```
read():    primary.read() ?? secondary.read() ?? {}
update(d): pBefore = primary.read(); r = primary.update(d)
   if r.success and pBefore === null: secondary.delete()       // first migration
   if !r.success:
      f = secondary.update(d)
      if f.success and pBefore !== null: primary.delete()      // prevent stale read shadowing
delete():  primary.delete() || secondary.delete()
```

Plain-text fallback writes `~/.claude/.credentials.json` with `chmodSync(path, 0o600)` and warning string `'Warning: Storing credentials in plaintext.'` (`plainTextStorage.ts:57-65`).

### 5.11 Cross-process refresh-token race and the `getApiKeyFromConfigOrMacOSKeychain` prefetch shortcut

`invalidateOAuthCacheIfDiskChanged()` (`src/utils/auth.ts:1320-1336`) `stat`s `.credentials.json`; on mtime change clears `getClaudeAIOAuthTokens.cache` (and the keychain cache via `clearOAuthTokenCache`). On `ENOENT` (macOS path) it only clears the memoize and lets the keychain TTL handle freshness.

`getApiKeyFromConfigOrMacOSKeychain` (`src/utils/auth.ts:1051-1087`) consults the prefetch result first (`getLegacyApiKeyPrefetchResult()`); if the prefetch completed with a key, returns it as `'/login managed key'`; if completed with no key, falls through to `globalConfig.primaryApiKey`. If prefetch isn't complete, it falls back to a sync `security` spawn (~33 ms).

## 6. Verbatim Assets

### 6.1 OAuth scopes (`src/constants/oauth.ts:33-58`)

```
CLAUDE_AI_INFERENCE_SCOPE = 'user:inference'
CLAUDE_AI_PROFILE_SCOPE   = 'user:profile'
CONSOLE_SCOPE             = 'org:create_api_key'
OAUTH_BETA_HEADER         = 'oauth-2025-04-20'

CONSOLE_OAUTH_SCOPES = [
  'org:create_api_key',
  'user:profile',
]

CLAUDE_AI_OAUTH_SCOPES = [
  'user:profile',
  'user:inference',
  'user:sessions:claude_code',
  'user:mcp_servers',
  'user:file_upload',
]

ALL_OAUTH_SCOPES = unique union of CONSOLE_OAUTH_SCOPES + CLAUDE_AI_OAUTH_SCOPES
```

### 6.2 Production OAuth endpoints (`src/constants/oauth.ts:84-104`)

```
BASE_API_URL          : 'https://api.anthropic.com'
CONSOLE_AUTHORIZE_URL : 'https://platform.claude.com/oauth/authorize'
CLAUDE_AI_AUTHORIZE_URL: 'https://claude.com/cai/oauth/authorize'   // 307s twice to claude.ai/oauth/authorize
CLAUDE_AI_ORIGIN      : 'https://claude.ai'
TOKEN_URL             : 'https://platform.claude.com/v1/oauth/token'
API_KEY_URL           : 'https://api.anthropic.com/api/oauth/claude_cli/create_api_key'
ROLES_URL             : 'https://api.anthropic.com/api/oauth/claude_cli/roles'
CONSOLE_SUCCESS_URL   : 'https://platform.claude.com/buy_credits?returnUrl=/oauth/code/success%3Fapp%3Dclaude-code'
CLAUDEAI_SUCCESS_URL  : 'https://platform.claude.com/oauth/code/success?app=claude-code'
MANUAL_REDIRECT_URL   : 'https://platform.claude.com/oauth/code/callback'
CLIENT_ID             : '9d1c250a-e61b-44d9-88ed-5944d1962f5e'
OAUTH_FILE_SUFFIX     : ''                                          // prod
MCP_PROXY_URL         : 'https://mcp-proxy.anthropic.com'
MCP_PROXY_PATH        : '/v1/mcp/{server_id}'

PROFILE_ENDPOINT      = `${BASE_API_URL}/api/oauth/profile`         (getOauthProfile.ts:40)
CLI_PROFILE_ENDPOINT  = `${BASE_API_URL}/api/claude_cli_profile`    (getOauthProfile.ts:19)
TRUSTED_DEVICES_ENDPT = `${BASE_API_URL}/api/auth/trusted_devices`  (trustedDevice.ts:149)

MCP_CLIENT_METADATA_URL = 'https://claude.ai/oauth/claude-code-client-metadata'  (oauth.ts:113-114)
```

### 6.3 Staging OAuth endpoints (ANT-only, `src/constants/oauth.ts:118-143`)

```
BASE_API_URL          : 'https://api-staging.anthropic.com'
CONSOLE_AUTHORIZE_URL : 'https://platform.staging.ant.dev/oauth/authorize'
CLAUDE_AI_AUTHORIZE_URL: 'https://claude-ai.staging.ant.dev/oauth/authorize'
CLAUDE_AI_ORIGIN      : 'https://claude-ai.staging.ant.dev'
TOKEN_URL             : 'https://platform.staging.ant.dev/v1/oauth/token'
API_KEY_URL           : 'https://api-staging.anthropic.com/api/oauth/claude_cli/create_api_key'
ROLES_URL             : 'https://api-staging.anthropic.com/api/oauth/claude_cli/roles'
CONSOLE_SUCCESS_URL   : 'https://platform.staging.ant.dev/buy_credits?returnUrl=/oauth/code/success%3Fapp%3Dclaude-code'
CLAUDEAI_SUCCESS_URL  : 'https://platform.staging.ant.dev/oauth/code/success?app=claude-code'
MANUAL_REDIRECT_URL   : 'https://platform.staging.ant.dev/oauth/code/callback'
CLIENT_ID             : '22422756-60c9-4084-8eb7-27705fd5cf9a'
OAUTH_FILE_SUFFIX     : '-staging-oauth'
MCP_PROXY_URL         : 'https://mcp-proxy-staging.anthropic.com'
MCP_PROXY_PATH        : '/v1/mcp/{server_id}'
```

### 6.4 Local-dev defaults (ANT + `USE_LOCAL_OAUTH`, `src/constants/oauth.ts:148-174`)

```
api      = CLAUDE_LOCAL_OAUTH_API_BASE      ?? 'http://localhost:8000'
apps     = CLAUDE_LOCAL_OAUTH_APPS_BASE     ?? 'http://localhost:4000'
console  = CLAUDE_LOCAL_OAUTH_CONSOLE_BASE  ?? 'http://localhost:3000'
CLIENT_ID = '22422756-60c9-4084-8eb7-27705fd5cf9a'
OAUTH_FILE_SUFFIX = '-local-oauth'
MCP_PROXY_URL  = 'http://localhost:8205'
MCP_PROXY_PATH = '/v1/toolbox/shttp/mcp/{server_id}'
```

### 6.5 `CLAUDE_CODE_CUSTOM_OAUTH_URL` allow-list (`src/constants/oauth.ts:179-183`)

```
ALLOWED_OAUTH_BASE_URLS = [
  'https://beacon.claude-ai.staging.ant.dev',
  'https://claude.fedstart.com',
  'https://claude-staging.fedstart.com',
]
```

Mismatch throws `'CLAUDE_CODE_CUSTOM_OAUTH_URL is not an approved endpoint.'` (`oauth.ts:204-206`). When honored, the override sets `OAUTH_FILE_SUFFIX = '-custom-oauth'` and rewrites all 7 endpoint URLs as `${base}/...` (`oauth.ts:208-221`).

### 6.6 Authorize-URL query parameters (`src/constants/oauth.ts:71-104`, `client.ts:71-87`)

```
code              = 'true'                       // shows Claude Max upsell
client_id         = CLIENT_ID
response_type     = 'code'
redirect_uri      = isManual ? MANUAL_REDIRECT_URL : `http://localhost:${port}/callback`
scope             = (inferenceOnly ? ['user:inference'] : ALL_OAUTH_SCOPES).join(' ')
code_challenge    = base64url(sha256(verifier))
code_challenge_method = 'S256'
state             = base64url(randomBytes(32))
[orgUUID]         = options.orgUUID
[login_hint]      = options.loginHint
[login_method]    = options.loginMethod          // 'sso' | 'magic_link' | 'google' | ...
```

### 6.7 Token-exchange request body (`src/services/oauth/client.ts:115-132`)

```
POST TOKEN_URL  Content-Type: application/json   timeout: 15000 ms
{
  grant_type:    'authorization_code',
  code:          authorizationCode,
  redirect_uri:  useManualRedirect ? MANUAL_REDIRECT_URL : `http://localhost:${port}/callback`,
  client_id:     CLIENT_ID,
  code_verifier: codeVerifier,
  state:         state,
  expires_in?:   options.expiresIn,            // optional (long-lived inference tokens)
}
401 -> 'Authentication failed: Invalid authorization code'
other -> `Token exchange failed (${status}): ${statusText}`
```

### 6.8 Refresh-token request body (`src/services/oauth/client.ts:150-163`)

```
POST TOKEN_URL  Content-Type: application/json   timeout: 15000 ms
{
  grant_type:    'refresh_token',
  refresh_token: <token>,
  client_id:     CLIENT_ID,
  scope:         (requestedScopes ?? CLAUDE_AI_OAUTH_SCOPES).join(' '),
}
non-200 -> `Token refresh failed: ${statusText}`
```

### 6.9 Profile request shapes (`src/services/oauth/getOauthProfile.ts`)

```
OAuth-bearer:
   GET `${BASE_API_URL}/api/oauth/profile`
   Authorization: Bearer ${accessToken}
   Content-Type: application/json
   timeout: 10000

API-key:
   GET `${BASE_API_URL}/api/claude_cli_profile`
   x-api-key:      ${apiKey}
   anthropic-beta: oauth-2025-04-20
   params:         account_uuid=${accountUuid}
   timeout: 10000
```

`OAuthProfileResponse` (reverse-engineered, used in `client.ts:355-420`, `client.ts:498-510`):

```
{
  account: {
    uuid:         string
    email:        string
    display_name: string | null
    created_at:   string
  },
  organization: {
    uuid:                       string
    organization_type:          'claude_max'|'claude_pro'|'claude_enterprise'|'claude_team'|other
    rate_limit_tier:            RateLimitTier | null
    has_extra_usage_enabled:    boolean | null
    billing_type:               BillingType | null
    subscription_created_at:    string | null
  }
}
```

`organization_type -> SubscriptionType` mapping: `claude_max -> 'max'`, `claude_pro -> 'pro'`, `claude_enterprise -> 'enterprise'`, `claude_team -> 'team'`, otherwise `null`.

### 6.10 Trusted-device request (`src/bridge/trustedDevice.ts:142-159`)

```
POST `${BASE_API_URL}/api/auth/trusted_devices`
Authorization: Bearer ${accessToken}
Content-Type:  application/json
timeout:       10_000
validateStatus: s => s < 500
body: { display_name: `Claude Code on ${hostname()} - ${process.platform}` }
response (200|201): { device_token?: string; device_id?: string }
header on bridge calls: X-Trusted-Device-Token: <device_token>     // codeSessionApi.ts:103
GrowthBook gate: 'tengu_sessions_elevated_auth_enforcement' (default false)
env-var precedence: CLAUDE_TRUSTED_DEVICE_TOKEN
```

### 6.11 Keychain naming (`src/utils/secureStorage/macOsKeychainHelpers.ts:27-42`)

```
CREDENTIALS_SERVICE_SUFFIX = '-credentials'

getMacOsKeychainStorageServiceName(serviceSuffix = ''):
   isDefaultDir = !process.env.CLAUDE_CONFIG_DIR
   dirHash = isDefaultDir ? '' : '-' + sha256(configDir).hex.substring(0, 8)
   return `Claude Code${OAUTH_FILE_SUFFIX}${serviceSuffix}${dirHash}`

# default-dir, prod:
#   OAuth tokens entry  : "Claude Code-credentials"
#   Legacy API key entry: "Claude Code"
# custom CLAUDE_CONFIG_DIR adds e.g. "-a1b2c3d4" suffix; staging uses '-staging-oauth' suffix.

getUsername(): process.env.USER || os.userInfo().username || 'claude-code-user'

KEYCHAIN_CACHE_TTL_MS = 30_000
SECURITY_STDIN_LINE_LIMIT = 4096 - 64 = 4032
KEYCHAIN_PREFETCH_TIMEOUT_MS = 10_000
```

`security(1)` argv:

```
read   : security find-generic-password -a "$USER" -w -s "<serviceName>"
write  : (stdin via -i)  add-generic-password -U -a "$USER" -s "<serviceName>" -X "<hex>"
delete : security delete-generic-password -a "$USER" -s "<serviceName>"
locked : security show-keychain-info       # exit 36 => locked
```

### 6.12 Plain-text fallback (`src/utils/secureStorage/plainTextStorage.ts:13-65`)

```
storageDir  = getClaudeConfigHomeDir()
storagePath = join(storageDir, '.credentials.json')
chmod       = 0o600
warning     = 'Warning: Storing credentials in plaintext.'
```

### 6.13 `NATIVE_CLIENT_ATTESTATION` placeholder (`src/constants/system.ts:73-95`)

```
isAttributionHeaderEnabled():
   if isEnvDefinedFalsy(process.env.CLAUDE_CODE_ATTRIBUTION_HEADER): return false
   return getFeatureValue_CACHED_MAY_BE_STALE('tengu_attribution_header', true)

getAttributionHeader(fingerprint: string):
   version    = `${MACRO.VERSION}.${fingerprint}`
   entrypoint = process.env.CLAUDE_CODE_ENTRYPOINT ?? 'unknown'
   cch        = feature('NATIVE_CLIENT_ATTESTATION') ? ' cch=00000;' : ''
   workload   = getWorkload() ? ` cc_workload=${getWorkload()};` : ''
   return `x-anthropic-billing-header: cc_version=${version}; cc_entrypoint=${entrypoint};${cch}${workload}`
```

> **Phase 9.6c (F-§6.13):** the same-length-rewrite invariant is **unverifiable
> from this leak** and has been moved to §12 Open Questions. Only the JS-side
> placeholder emission above is verbatim from the leak.

### 6.14 Auth env vars / file descriptors

| Variable | File:line | Effect |
|---|---|---|
| `ANTHROPIC_API_KEY` | `auth.ts:236-292` | Top-priority API key (skipped on homespace). |
| `ANTHROPIC_AUTH_TOKEN` | `auth.ts:164-166` | Bearer token; suppressed in managed OAuth context. |
| `CLAUDE_CODE_OAUTH_TOKEN` | `auth.ts:168-170,1260-1270` | Inference-only OAuth bearer; scopes hardcoded `['user:inference']`. |
| `CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR` | `auth.ts:181-186` | OAuth token via inherited pipe FD. |
| `CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR` | `auth.ts:127`, `authFileDescriptor.ts` | API key via inherited pipe FD. |
| `CLAUDE_CODE_OAUTH_REFRESH_TOKEN` | `cli/handlers/auth.ts:140-186` | Headless login: requires `CLAUDE_CODE_OAUTH_SCOPES`; calls `refreshOAuthToken` then `installOAuthTokens`. |
| `CLAUDE_CODE_OAUTH_SCOPES` | same | Space-separated; required when refresh-token env var is set. |
| `CLAUDE_CODE_OAUTH_CLIENT_ID` | `oauth.ts:225-230` | Overrides `CLIENT_ID` (e.g. Xcode integration). |
| `CLAUDE_CODE_CUSTOM_OAUTH_URL` | `oauth.ts:200-222` | FedStart override; allow-listed only. |
| `USE_STAGING_OAUTH`, `USE_LOCAL_OAUTH` | `oauth.ts:5-16` | ANT-only; selects staging/local config. |
| `CLAUDE_LOCAL_OAUTH_{API,APPS,CONSOLE}_BASE` | `oauth.ts:148-157` | Per-component local-dev overrides. |
| `CLAUDE_TRUSTED_DEVICE_TOKEN` | `trustedDevice.ts:46-50, 112-117` | Forces a specific token; suppresses enrollment. |
| `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_ENTRYPOINT=claude-desktop` | `auth.ts:91-96` | `isManagedOAuthContext()`: causes `ANTHROPIC_AUTH_TOKEN`/`apiKeyHelper`/`ANTHROPIC_API_KEY` to be ignored in favor of OAuth. |
| `ANTHROPIC_UNIX_SOCKET` | `auth.ts:111-113` | `claude ssh` proxy mode; `CLAUDE_CODE_OAUTH_TOKEN` becomes a no-op placeholder. |
| `CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, `CLAUDE_CODE_USE_FOUNDRY` | `auth.ts:116-118` | Disables 1P OAuth; provider creds take over. |
| `CLAUDE_CODE_ACCOUNT_UUID`, `CLAUDE_CODE_USER_EMAIL`, `CLAUDE_CODE_ORGANIZATION_UUID` | `client.ts:457-471` | SDK fallback for `populateOAuthAccountInfoIfNeeded`. |
| `CLAUDE_CODE_API_KEY_HELPER_TTL_MS` | `auth.ts:436-449` | Override default 5-minute helper cache. |
| `AWS_BEARER_TOKEN_BEDROCK` | `services/api/client.ts:172-178` | Bedrock bearer auth (3P; `skipAuth=true`). |

### 6.15 User-facing strings (login/logout)

```
Refresh-token env login (cli/handlers/auth.ts):
   "CLAUDE_CODE_OAUTH_SCOPES is required when using CLAUDE_CODE_OAUTH_REFRESH_TOKEN.\n
    Set it to the space-separated scopes the refresh token was issued with\n
    (e.g. \"user:inference\" or \"user:profile user:inference user:sessions:claude_code user:mcp_servers\").\n"
   "Login successful.\n"
   "Login failed: ${errorMessage(err)}\n${sslHint ? sslHint + '\\n' : ''}"
   "Opening browser to sign in...\n"
   "If the browser didn't open, visit: ${url}\n"
   "Error: --console and --claudeai cannot be used together.\n"

Listener (auth-code-listener.ts):
   400 body: "Authorization code not found"
   400 body: "Invalid state parameter"
   throw:    "No authorization code received"
   throw:    "Invalid state parameter"
   throw:    "Failed to start OAuth callback server: ${err.message}"

API-key flow (auth.ts:1097, 1281, 565-573):
   "Invalid API key format. API key must contain only alphanumeric characters, dashes, and underscores."
   "ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN env var is required"
   apiKeyHelper failure (red): `apiKeyHelper failed: ${detail}`

installOAuthTokens (cli/handlers/auth.ts:103-105):
   "Unable to create API key. The server accepted the request but did not return a key."

Plain-text fallback (plainTextStorage.ts):
   "Warning: Storing credentials in plaintext."
```

`/login`, `/logout`, and `/setup-token` Ink-component strings live in `src/commands/{login,logout,setup-token}/` and are documented in 21a.

### 6.16 Constants table

| Constant | Value | File:line |
|---|---|---|
| Token-expiry buffer | `5 * 60 * 1000` ms (5 min) | `services/oauth/client.ts:349` |
| Refresh-lock max retries | `5` | `utils/auth.ts:1451` |
| Refresh-lock backoff | `1000 + Math.random()*1000` ms | `utils/auth.ts:1501` |
| Token exchange / refresh timeout | `15000` ms | `services/oauth/client.ts:132, 169` |
| Profile-fetch timeout | `10000` ms | `services/oauth/getOauthProfile.ts:29, 47` |
| Trusted-device enrollment timeout | `10_000` ms | `bridge/trustedDevice.ts:156` |
| Keychain prefetch timeout | `10_000` ms | `secureStorage/keychainPrefetch.ts:33` |
| Keychain in-process cache TTL | `30_000` ms | `secureStorage/macOsKeychainHelpers.ts:69` |
| `security -i` stdin line limit | `4096 - 64 = 4032` bytes | `secureStorage/macOsKeychainStorage.ts:24` |
| Default `apiKeyHelper` TTL | `5 * 60 * 1000` ms | `utils/auth.ts:81` |
| `apiKeyHelper` execa timeout | `10 * 60 * 1000` ms | `utils/auth.ts:560` |
| Plain-text file mode | `0o600` | `secureStorage/plainTextStorage.ts:61` |
| Trusted-device rolling expiry | 90 d (server-side) | `bridge/trustedDevice.ts:28` |
| Trusted-device enrollment window | `account_session.created_at < 10 min` (server-side) | `bridge/trustedDevice.ts:26-27` |
| Locked-keychain exit code | `36` | `secureStorage/macOsKeychainStorage.ts:225` |
| Keychain-not-found exit code | `44` (treated as "no key") | `secureStorage/keychainPrefetch.ts:51-58` |

### 6.17 JWT (claim shape)

The leaked `src/services/oauth/` directory contains no JWT issuer or verifier. The only JWT-shaped values produced or consumed by Claude Code in this tree are:

1. **CCR session JWTs** (spec 35): produced by Anthropic infrastructure, not by this CLI; consumed by the API client (`services/api/errors.ts:214,817,869`, `services/api/withRetry.ts:708`) which short-circuits 401/403 retry logic in CCR mode because "auth is via JWTs provided by the infrastructure."
2. **Session-ingress logging JWTs** (`services/api/sessionIngress.ts:189-228`): consumed as `Authorization: Bearer ${sessionToken}`.
3. **OAuth bridge JWTs** (spec 34): the `X-Trusted-Device-Token` is opaque to the CLI; the device-token itself is treated as a string and is not parsed.

There is no `signJwt` / `verifyJwt` symbol nor `jose`/`jsonwebtoken` import in this directory. Reproducing claim shapes requires the server-side or `bun-anthropic` source not included in the leak. **Open question — recorded in §12.**

## 7. Side Effects & I/O

- **Subprocesses** (darwin only): `security find-generic-password`, `security add-generic-password`, `security delete-generic-password`, `security show-keychain-info`. The first is fired pre-import via `execFile` from `keychainPrefetch.ts`; subsequent reads use cached results. Writes hide payload from `ps` by piping `add-generic-password -X <hex>` through `security -i` stdin when the command fits in 4032 bytes.
- **Filesystem.** `~/.claude/.credentials.json` (mode `0600`); `lockfile.lock(claudeDir)` for cross-process refresh exclusivity (`utils/auth.ts:1485-1492`); `.credentials.json` mtime polled in `invalidateOAuthCacheIfDiskChanged()`.
- **Network.** axios POST to `TOKEN_URL` (timeout 15 s); axios POST to `API_KEY_URL` and `BASE_API_URL/api/auth/trusted_devices`; axios GET to `BASE_API_URL/api/oauth/profile`, `BASE_API_URL/api/claude_cli_profile`, and `ROLES_URL`.
- **Local HTTP server.** OS-assigned port on `localhost`, route `<callbackPath>` (default `/callback`), 302-redirects browser to `CONSOLE_SUCCESS_URL` or `CLAUDEAI_SUCCESS_URL` on success and to `CLAUDEAI_SUCCESS_URL` on error (TODO note: error page not yet distinct).
- **Browser.** `openBrowser(automaticUrl)` is called only when `skipBrowserOpen !== true` (`services/oauth/index.ts:81-84`).
- **Process env reads** at boot affect prefetch service-name selection: `USER`, `CLAUDE_CONFIG_DIR`, `USE_STAGING_OAUTH`, `USE_LOCAL_OAUTH`, `CLAUDE_CODE_CUSTOM_OAUTH_URL`, `USER_TYPE`.

## 8. Feature Flags & Variants

| Flag/source | Effect |
|---|---|
| `feature('NATIVE_CLIENT_ATTESTATION')` (`bun:bundle`) | Adds ` cch=00000;` placeholder to `x-anthropic-billing-header`; otherwise omitted. (`constants/system.ts:82`) |
| `process.env.USER_TYPE === 'ant'` | Enables `STAGING_OAUTH_CONFIG` literal; enables `USE_LOCAL_OAUTH`/`USE_STAGING_OAUTH` env-var honoring. (`constants/oauth.ts:7-14, 118-143`) |
| `isBareMode()` | Disables OAuth entirely; only `ANTHROPIC_API_KEY` env or `--settings`-sourced `apiKeyHelper`. Skips keychain prefetch. (`utils/auth.ts:102-148, 235-247`; `keychainPrefetch.ts:70`) |
| GrowthBook `tengu_attribution_header` (default `true`) | Killswitch for the entire attribution header. (`constants/system.ts:53-57`) |
| GrowthBook `tengu_sessions_elevated_auth_enforcement` (default `false`) | Gates trusted-device read AND enrollment. (`bridge/trustedDevice.ts:33-37, 103`) |
| `CLAUDE_CODE_USE_{BEDROCK,VERTEX,FOUNDRY}` | Disables 1P OAuth (`isAnthropicAuthEnabled() === false`); 3P providers use SDK-native creds (AWS SigV4 / GoogleAuth ADC / Azure AD `DefaultAzureCredential`); only `AWS_BEARER_TOKEN_BEDROCK` adds an `Authorization: Bearer` header (`services/api/client.ts:172-178`). |
| `homespace` runtime detection | `ANTHROPIC_API_KEY` env is ignored in favor of Console-issued key (`utils/auth.ts:252`). |
| `CI` truthy or `NODE_ENV === 'test'` | Throws if neither `ANTHROPIC_API_KEY` nor `CLAUDE_CODE_OAUTH_TOKEN(_FILE_DESCRIPTOR)` is present (`utils/auth.ts:265-296`). |
| `preferThirdPartyAuthentication()` && `--print` | Lets `ANTHROPIC_API_KEY` env beat `/login`-managed key. |
| Platform != darwin | `getSecureStorage()` returns plain-text only; `startKeychainPrefetch()` is a no-op. (`secureStorage/index.ts:9-17`; `keychainPrefetch.ts:70`) |

## 9. Error Handling & Edge Cases

- **State mismatch** => HTTP 400 + `Error('Invalid state parameter')` rejection; listener stays open until `close()`.
- **No code in callback** => HTTP 400 + `Error('No authorization code received')`.
- **Token exchange 401** => `'Authentication failed: Invalid authorization code'`.
- **Token exchange other non-200** => `'Token exchange failed (${status}): ${statusText}'`.
- **Refresh non-200** => `'Token refresh failed: ${statusText}'`; `tengu_oauth_token_refresh_failure` logged with sliced response body.
- **Refresh 401** => `handleOAuth401Error` dedupes by failed access token, re-reads keychain, and either picks up a sibling-process refresh or forces a new refresh.
- **Refresh-lock contention** => retry up to 5x with 1-2 s jittered sleep; afterward returns `false` and emits `tengu_oauth_token_refresh_lock_retry_limit_reached`.
- **Profile fetch transient failure** => swallowed; preserves prior `subscriptionType`/`rateLimitTier` via `??` chain to avoid wiping paying-user state during the `CLAUDE_CODE_OAUTH_REFRESH_TOKEN` re-login path.
- **`security -i` overflow** => falls back to argv-visible `add-generic-password` and logs a warning; the alternative (silent corruption per #30337) is strictly worse.
- **Keychain read transient failure** => stale-while-error; serves last good value and refreshes timestamp so 30 s TTL doesn't cause re-spawn storms.
- **Keychain timeout during prefetch** => does not prime the cache; sync `read()` retries with its own (longer) timeout.
- **Prefetch malformed JSON** => `primeKeychainCacheFromPrefetch` returns silently; sync read re-fetches.
- **Plain-text fallback write success** => returns `{success:true, warning:'Warning: Storing credentials in plaintext.'}`; surfaced via `tengu_oauth_storage_warning` in `installOAuthTokens`.
- **Migration delete** => on first successful primary write, secondary file is deleted to avoid a stale fallback masking fresh primary on next read; conversely, on primary write failure with secondary success, the stale primary entry is deleted (#30337).
- **Custom OAuth URL not allow-listed** => `throw 'CLAUDE_CODE_CUSTOM_OAUTH_URL is not an approved endpoint.'`.
- **Bare mode** => no OAuth, no keychain, no `apiKeyHelper` from non-flag sources.
- **Keychain locked (SSH session)** => `isMacOsKeychainLocked()` caches `true`; downstream callers may degrade.
- **Trusted-device enrollment failure** => logged and swallowed; never blocks `/login`.

## 10. Telemetry & Observability

Analytics events emitted by this service (logEvent name, file:line):

```
tengu_oauth_auth_code_received                services/oauth/index.ts:90
tengu_oauth_token_exchange_success            services/oauth/client.ts:142
tengu_oauth_token_refresh_success             services/oauth/client.ts:185
tengu_oauth_token_refresh_failure             services/oauth/client.ts:264
tengu_oauth_roles_stored                      services/oauth/client.ts:305
tengu_oauth_api_key                           services/oauth/client.ts:322, 331
tengu_oauth_profile_fetch_success             services/oauth/client.ts:417
tengu_oauth_automatic_redirect                services/oauth/auth-code-listener.ts:90, 104
tengu_oauth_automatic_redirect_error          services/oauth/auth-code-listener.ts:122
tengu_oauth_tokens_not_claude_ai              utils/auth.ts:1199
tengu_oauth_tokens_inference_only             utils/auth.ts:1205
tengu_oauth_tokens_saved                      utils/auth.ts:1234
tengu_oauth_tokens_save_failed                utils/auth.ts:1236
tengu_oauth_tokens_save_exception             utils/auth.ts:1245
tengu_oauth_storage_warning                   cli/handlers/auth.ts:83
tengu_oauth_401_recovered_from_keychain       utils/auth.ts:1386
tengu_oauth_token_refresh_lock_acquiring      utils/auth.ts:1490
tengu_oauth_token_refresh_lock_acquired       utils/auth.ts:1492
tengu_oauth_token_refresh_lock_retry          utils/auth.ts:1497
tengu_oauth_token_refresh_lock_retry_limit_reached utils/auth.ts:1504
tengu_oauth_token_refresh_lock_error          utils/auth.ts:1510
tengu_oauth_token_refresh_starting            utils/auth.ts:1530
tengu_oauth_token_refresh_race_resolved       utils/auth.ts:1526
tengu_oauth_token_refresh_race_recovered      utils/auth.ts:1552
tengu_oauth_token_refresh_lock_releasing      utils/auth.ts:1558
tengu_oauth_token_refresh_lock_released       utils/auth.ts:1560
tengu_login_from_refresh_token                cli/handlers/auth.ts:155
tengu_oauth_flow_start                        cli/handlers/auth.ts:193
tengu_oauth_success                           cli/handlers/auth.ts:173, 216
tengu_api_key_saved_to_keychain               utils/auth.ts:1123
tengu_api_key_keychain_error                  utils/auth.ts:1127
tengu_api_key_saved_to_config                 utils/auth.ts:1132, 1135
tengu_apiKeyHelper_missing_trust11            utils/auth.ts:553
```

Debug logs (`logForDebugging`) tagged `[keychain]` and `[trusted-device]`.

## 11. Reimplementation Checklist

1. PKCE: `base64url(randomBytes(32))` for verifier and state; `base64url(sha256(verifier))` for challenge; `+/=` -> `-_<strip>`.
2. Run a local `http.createServer()` on an OS-assigned port; only honor `<callbackPath>` (default `/callback`); validate `state`; 302 to success URL on the captured `pendingResponse` after token exchange (or 302 to error URL on failure).
3. Always offer both flows: `manualUrl` (`MANUAL_REDIRECT_URL` redirect_uri) and `automaticUrl` (`http://localhost:${port}/callback`); `skipBrowserOpen` hands both to the SDK caller without `openBrowser()`.
4. Token request bodies and timeouts match §6.7 / §6.8 exactly. Always include `state` on exchange. Always include `client_id`. Send `expires_in` only when explicitly requested.
5. `expiresAt = Date.now() + expires_in*1000`. Treat `expiresAt === null` as never-expired. Use a 5-minute `now + buffer >= expiresAt` expiry check.
6. Persist via the secureStorage façade only; never bypass to write `.credentials.json` directly. Skip persistence when `!shouldUseClaudeAIAuth(scopes)` or when `refreshToken`/`expiresAt` are null.
7. macOS service names: `Claude Code${OAUTH_FILE_SUFFIX}-credentials${dirHash}` (OAuth) and `Claude Code${OAUTH_FILE_SUFFIX}${dirHash}` (legacy API key). `dirHash = '-' + sha256(configDir).hex.slice(0,8)` only when `CLAUDE_CONFIG_DIR` is set.
8. Use `security -i` with stdin command for writes <= 4032 bytes; argv fallback above that with a warning.
9. 30 s in-process keychain cache; bump `generation` on every invalidation; readAsync drops stale subprocess writes; cross-process invalidator polls `.credentials.json` mtime.
10. Refresh path: file-lock via `proper-lockfile` against `~/.claude`; 5 retries, 1-2 s jittered backoff; double-check expiry after lock acquire; persist via `saveOAuthTokensIfNeeded`; release in `finally`.
11. 401 recovery: dedup by failed access token; re-read keychain; force refresh only if same access token still resident.
12. `installOAuthTokens` order: `performLogout({clearOnboarding:false})` -> `storeOAuthAccountInfo` (profile preferred, `tokenAccount` fallback) -> `saveOAuthTokensIfNeeded` -> `clearOAuthTokenCache` -> `fetchAndStoreUserRoles` (best-effort) -> if Claude.ai-scoped: `fetchAndStoreClaudeCodeFirstTokenDate`; else: `createAndStoreApiKey` (throw on null) -> `clearAuthRelatedCaches`.
13. `startKeychainPrefetch()` must run before any heavy module import; await via `ensureKeychainPrefetchCompleted()` in `preAction` alongside other parallelizable boot work.
14. `keychainPrefetch.ts` MUST NOT import `execa`, `execFileNoThrow`, or anything that pulls in `human-signals`/`cross-spawn`; only `child_process.execFile` plus the lightweight helpers.
15. Trusted device: gated by `tengu_sessions_elevated_auth_enforcement`; honor `CLAUDE_TRUSTED_DEVICE_TOKEN` precedence; enrollment immediately after `/login`; persist into `SecureStorageData.trustedDeviceToken`.
16. `NATIVE_CLIENT_ATTESTATION`: insert `cch=00000;` placeholder, never compute the token in JS — it is rewritten by Bun's native HTTP path before bytes leave the process.
17. Honor the auth env-var precedence in §6.14 exactly; treat `CLAUDE_CODE_REMOTE` and `CLAUDE_CODE_ENTRYPOINT=claude-desktop` as managed-OAuth contexts that suppress non-OAuth sources.
18. Allow-list `CLAUDE_CODE_CUSTOM_OAUTH_URL` against the three FedStart URLs and throw on mismatch.
19. 3P providers (Bedrock/Vertex/Foundry) bypass this service entirely — `isAnthropicAuthEnabled() === false`; provider SDKs handle auth.

## 12. Open Questions / Unknowns

1. **JWT issuer/verifier code is not in the leak.** `src/services/oauth/` contains no `signJwt`/`verifyJwt` and no JWT library import; all JWT references in this tree (CCR, session-ingress logging, bridge) are *consumers* of opaque bearer tokens. The exact claim set, signing algorithm, and key-rotation policy live in `bun-anthropic` or in server-side code outside this dump. Spec 34/35 should resolve.
2. **`OAuthTokens`, `OAuthProfileResponse`, `OAuthTokenExchangeResponse`, `UserRolesResponse`, `SubscriptionType`, `RateLimitTier`, `BillingType`, `SecureStorageData`, `SecureStorage` types** are imported from `./types.ts` paths that are not present in the leak. Field shapes recorded in §4 are reverse-engineered from call sites; nullability of fields not exercised in the read paths is unverified.
3. **`logout.ts` / `performLogout` body** is not transcribed here. Verify: (a) which keychain entries are deleted, (b) whether `globalConfig.oauthAccount` and `customApiKeyResponses` are cleared, (c) interaction with `clearTrustedDeviceToken` and `clearAuthRelatedCaches`.
4. **Linux libsecret support is a TODO** in `secureStorage/index.ts:14`. Linux currently uses plain-text only.
5. **`isMacOsKeychainLocked()` exit-code-36 semantics**: the comment claims this is the "locked" code; verify against macOS `security(1)` source for completeness across keychain types (login.keychain vs custom).
6. **Bedrock `AWS_BEARER_TOKEN_BEDROCK` flow** sets `skipAuth: true` but still injects `Authorization: Bearer`; the SDK behavior under both signals at once is undocumented here.
7. **`CCR_OAUTH_TOKEN_FILE` source** (the disk fallback for FD-injected OAuth tokens when subprocesses can't inherit pipes) is referenced but the file path/format is in `authFileDescriptor.ts`, not yet read; spec 35 should cover.
8. **`fetchAndStoreClaudeCodeFirstTokenDate`** is referenced from `installOAuthTokens` but its endpoint and response shape live in `services/api/firstTokenDate.ts`; spec 22 should cover.
9. **(Moved from §6.13, Phase 9.6c)** "*The `cch=00000` placeholder is
   rewritten in-place by Bun's native HTTP path
   (`bun-anthropic/src/http/Attestation.zig`) immediately before the
   request body is sent; same-length replacement avoids `Content-Length`
   recomputation.*" — `bun-anthropic` is **not in the leak**. There is no
   JS-side rewriter or test fixture in `src/`, only the literal placeholder
   (`constants/system.ts:73-95`) and a comment. The same-length /
   no-`Content-Length`-recomputation invariant is **unfalsifiable from
   this tree**. A native-side regression that recomputes or fails to rewrite
   would silently leak the placeholder upstream as a billing-attestation
   header. Resolution requires `bun-anthropic` source.
9. **Browser-side success pages** (`CONSOLE_SUCCESS_URL`, `CLAUDEAI_SUCCESS_URL`) are server-rendered; their behavior (e.g. extracting `?error=...` query) is server-side and not in this tree. The "TODO: swap to a different url once we have an error page" at `auth-code-listener.ts:114` flags this.
