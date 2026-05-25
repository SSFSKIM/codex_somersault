# 27 — Service: Policy Limits, Remote-Managed Settings, Settings Sync

## 1. Purpose & Scope

This subsystem owns three independent-but-coupled enterprise control planes:

1. **`policyLimits`** — a per-user/per-org allow-list of named CLI features fetched from the Anthropic API (`/api/claude_code/policy_limits`). Consumed by code paths that gate "remote sessions", "remote control", and "product feedback" via the sync `isPolicyAllowed(name)` predicate. Independent of `policySettings`.
2. **`remoteManagedSettings`** — server-pushed JSON `SettingsJson` blob fetched from `/api/claude_code/settings`, validated against `SettingsSchema`, persisted to `~/.claude/remote-settings.json`, and used as the **highest-priority source** in the `policySettings` cascade.
3. **`settingsSync`** — interactive-CLI ↔ remote settings/memory bidirectional sync (`/api/claude_code/user_settings`); upload from interactive CLI (`UPLOAD_USER_SETTINGS`), download from CCR/print mode (`DOWNLOAD_USER_SETTINGS`).

This spec OWNS the **resolver body for the `policySettings` source** (per spec 02): cascade `remote > HKLM/plist > file > HKCU` plus the file-source merge of `managed-settings.json` + `managed-settings.d/*.json`. It also OWNS `isInProtectedNamespace` (per spec 09).

**OUT of scope:** settings precedence chain mechanics (spec 02 owns `getSettings()`); permission rule application (spec 09); Bash policy classification (spec 10) — but note `sandbox.excludedCommands` is read from the `policySettings` layer at `BashTool.tsx:343` and therefore traverses **this spec's** cascade resolver (see §11 cross-link); OAuth (spec 25); analytics events (spec 26).

**Cross-spec ownership of rate-limit / quota files (Phase 9.6 arbitration).** `services/claudeAiLimits.ts` (515 lines), `services/claudeAiLimitsHook.ts`, `services/rateLimitMessages.ts` (344 lines), `services/rateLimitMocking.ts`, `services/mockRateLimits.ts` are **owned by spec 22** for HTTP 429 classification, mock infrastructure, and quota arithmetic. Spec 22 §12 Q8 originally proposed handing them to spec 27 because they share the `services/` namespace; that handoff is **rejected** here because (a) these files have no dependency on `policySettings` / `policyLimits` / `remoteManagedSettings` / `settingsSync`, (b) they are entirely about API-response shape and user-facing copy for HTTP 429s — orthogonal to the enterprise control planes covered here, and (c) splitting them across two specs duplicates their ~860-line surface area without benefit. The verbatim user-facing rate-limit denial strings remain entirely within spec 22; this spec's §6.8 only covers **policy** denial strings (the enterprise-policy "Remote sessions are disabled by your organization's policy" family — distinct from HTTP-429 quota messaging). If spec 22 ever drops these files, ownership transfers here.

**Plugin policy chokepoint cross-link (spec 28).** `utils/plugins/pluginPolicy.ts:isPluginBlockedByPolicy` is a 3-line wrapper over this spec's `getSettingsForSource('policySettings').enabledPlugins[id] === false` predicate. It is the single chokepoint for ~10 plugin install/enable/UI call sites in `commands/plugin/*`, `services/plugins/pluginOperations.ts`, `utils/plugins/{pluginInstallationHelpers,hintRecommendation}.ts`. Spec 28 OWNS the file body; this spec OWNS the underlying `policySettings.enabledPlugins` resolver. Reimplementers MUST preserve both halves: spec-28 wrapper consumes spec-27 cascade output.

## 2. Source Map

### 2.1 Source-coverage inventory

| Path | Status |
|---|---|
| `src/services/policyLimits/index.ts` | read fully (663 lines) |
| `src/services/policyLimits/types.ts` | read fully |
| `src/services/remoteManagedSettings/index.ts` | read fully (639 lines) |
| `src/services/remoteManagedSettings/securityCheck.tsx` | read fully |
| `src/services/remoteManagedSettings/syncCache.ts` | read fully |
| `src/services/remoteManagedSettings/syncCacheState.ts` | read fully |
| `src/services/remoteManagedSettings/types.ts` | read fully |
| `src/services/settingsSync/index.ts` | read fully (582 lines) |
| `src/services/settingsSync/types.ts` | read fully |
| `src/utils/settings/settings.ts` (policy cascade body) | sampled (lines 50–407, 660–740) |
| `src/utils/settings/mdm/settings.ts` | read fully |
| `src/utils/settings/mdm/rawRead.ts` | read fully |
| `src/utils/settings/mdm/constants.ts` | read fully |
| `src/utils/settings/managedPath.ts` | read fully |
| `src/utils/settings/changeDetector.ts` | sampled (lines 1–100, 220–419) |
| `src/utils/envUtils.ts` (`isInProtectedNamespace`) | sampled |
| `src/utils/protectedNamespace.ts` | absent in leak (see §12 — ANT-only require gated by `USER_TYPE === 'ant'`; not present in the external bundle and not present at the resolved path in this leak) |
| `src/services/claudeAiLimits.ts`, `claudeAiLimitsHook.ts`, `rateLimitMessages.ts`, `rateLimitMocking.ts`, `mockRateLimits.ts` | NOT read — spec 22 owns |

### 2.2 Imports from / Imported by (high-level)

- `policyLimits/index.ts` imports: `axios`, `crypto`, `fs/promises`, `path`, `constants/oauth.js`, `utils/auth.js`, `utils/cleanupRegistry.js`, `utils/debug.js`, `utils/envUtils.js`, `utils/errors.js`, `utils/json.js`, `utils/model/providers.js`, `utils/privacyLevel.js`, `utils/sleep.js`, `utils/slowOperations.js`, `utils/userAgent.js`, `services/api/withRetry.js`.
- `policyLimits` imported by: `main.tsx:42`, `cli/print.ts:136`, `entrypoints/cli.tsx:151`, `bridge/initReplBridge.ts:24`, `commands/bridge/bridge.tsx:470`, `commands/remote-setup/index.ts:3`, `commands/remote-env/index.ts:2`, `commands/feedback/index.ts:2`, `tools/RemoteTriggerTool/RemoteTriggerTool.ts:6`, `utils/teleport.tsx:8`, `utils/background/remote/remoteSession.ts:3`, `components/FeedbackSurvey/{useFeedbackSurvey,useMemorySurvey}.tsx`.
- `remoteManagedSettings/index.ts` imports `utils/settings/changeDetector.js` (notifies `'policySettings'`).
- `remoteManagedSettings/syncCacheState.ts` is consumed by `utils/settings/settings.ts:324,682,683` for the cascade.
- `settingsSync/index.ts` imports: `bun:bundle` (`feature`), `bootstrap/state.js` (`getIsInteractive`), `utils/git.js` (`getRepoRemoteHash`), `utils/settings/settings.js` (`getSettingsFilePathForSource`), `utils/settings/internalWrites.js` (`markInternalWrite`), `utils/settings/settingsCache.js` (`resetSettingsCache`), `utils/claudemd.js` (`clearMemoryFileCaches`), `utils/config.js` (`getMemoryPath`).

### 2.3 Feature flag and ANT guards

- `feature('UPLOAD_USER_SETTINGS')` — `services/settingsSync/index.ts:63`, also `main.tsx:963`.
- `feature('DOWNLOAD_USER_SETTINGS')` — `services/settingsSync/index.ts:160`, also `cli/print.ts:511`.
- `getFeatureValue_CACHED_MAY_BE_STALE('tengu_enable_settings_sync_push', false)` — `settingsSync/index.ts:64`.
- `getFeatureValue_CACHED_MAY_BE_STALE('tengu_strap_foyer', false)` — `settingsSync/index.ts:163`.
- ANT guard `process.env.USER_TYPE === 'ant'` — `mdm/constants.ts:68` (allow user-writable plist for local MDM testing); `envUtils.ts:139` (`isInProtectedNamespace` lazy-requires `protectedNamespace.js`); `managedPath.ts:11` (`CLAUDE_CODE_MANAGED_SETTINGS_PATH` override).
- Env: `process.env.CLAUDE_CODE_ENTRYPOINT === 'local-agent'` short-circuits eligibility (`syncCache.ts:66`); `process.env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` flips essential-traffic-only mode (`utils/privacyLevel.ts:21`).

## 3. Public Interface (Contract)

### 3.1 `services/policyLimits` exports

```ts
export function isPolicyAllowed(policy: string): boolean        // sync; fail-open
export function isPolicyLimitsEligible(): boolean
export function initializePolicyLimitsLoadingPromise(): void
export function waitForPolicyLimitsToLoad(): Promise<void>
export async function loadPolicyLimits(): Promise<void>
export async function refreshPolicyLimits(): Promise<void>
export async function clearPolicyLimitsCache(): Promise<void>
export function startBackgroundPolling(): void
export function stopBackgroundPolling(): void
export function _resetPolicyLimitsForTesting(): void
```

Schema (`types.ts:8-12` verbatim):

```ts
export const PolicyLimitsResponseSchema = lazySchema(() =>
  z.object({
    restrictions: z.record(z.string(), z.object({ allowed: z.boolean() })),
  }),
)
```

`PolicyLimitsFetchResult` (`types.ts:21-27`): `{ success, restrictions?, etag?, error?, skipRetry? }`.

### 3.2 `services/remoteManagedSettings` exports

```ts
export function isEligibleForRemoteManagedSettings(): boolean
export function initializeRemoteManagedSettingsLoadingPromise(): void
export function waitForRemoteManagedSettingsToLoad(): Promise<void>
export async function loadRemoteManagedSettings(): Promise<void>
export async function refreshRemoteManagedSettings(): Promise<void>
export async function clearRemoteManagedSettingsCache(): Promise<void>
export function startBackgroundPolling(): void
export function stopBackgroundPolling(): void
export function computeChecksumFromSettings(s: SettingsJson): string  // exported for testing
// from syncCache.ts:
export function isRemoteManagedSettingsEligible(): boolean
export function resetSyncCache(): void
// from syncCacheState.ts:
export function setSessionCache(value: SettingsJson | null): void
export function resetSyncCache(): void
export function setEligibility(v: boolean): boolean
export function getSettingsPath(): string
export function getRemoteManagedSettingsSyncFromCache(): SettingsJson | null
```

Response schema (`remoteManagedSettings/types.ts:10-16` verbatim):

```ts
export const RemoteManagedSettingsResponseSchema = lazySchema(() =>
  z.object({
    uuid: z.string(),
    checksum: z.string(),
    settings: z.record(z.string(), z.unknown()) as z.ZodType<SettingsJson>,
  }),
)
```

`SecurityCheckResult` (`securityCheck.tsx:12`): `'approved' | 'rejected' | 'no_check_needed'`.

### 3.3 `services/settingsSync` exports

```ts
export async function uploadUserSettingsInBackground(): Promise<void>
export function downloadUserSettings(): Promise<boolean>     // memoizes one in-flight promise
export function redownloadUserSettings(): Promise<boolean>   // 0 retries, bypasses cache
export function _resetDownloadPromiseForTesting(): void
```

Schemas (`settingsSync/types.ts` verbatim):

```ts
export const UserSyncContentSchema = lazySchema(() =>
  z.object({ entries: z.record(z.string(), z.string()) }),
)
export const UserSyncDataSchema = lazySchema(() =>
  z.object({
    userId: z.string(),
    version: z.number(),
    lastModified: z.string(),  // ISO 8601
    checksum: z.string(),      // MD5 hash
    content: UserSyncContentSchema(),
  }),
)
export const SYNC_KEYS = {
  USER_SETTINGS: '~/.claude/settings.json',
  USER_MEMORY:   '~/.claude/CLAUDE.md',
  projectSettings: (projectId: string) => `projects/${projectId}/.claude/settings.local.json`,
  projectMemory:   (projectId: string) => `projects/${projectId}/CLAUDE.local.md`,
} as const
```

### 3.4 `isInProtectedNamespace` (`utils/envUtils.ts:136-147`)

```ts
export function isInProtectedNamespace(): boolean {
  if (process.env.USER_TYPE === 'ant') {
    return (require('./protectedNamespace.js') as typeof import('./protectedNamespace.js'))
      .checkProtectedNamespace()
  }
  return false
}
```

External (non-ANT) builds DCE the require entirely — function returns `false`. Conservative: ambiguous signals → assume protected. Used for telemetry only (no permission gating). Call sites: `main.tsx:4583`, `tools/AgentTool/agentToolUtils.ts:437`, `bridge/{remoteBridgeCore,bridgeMain,replBridge}.ts`, `utils/permissions/permissions.ts:{630,670,737}`.

## 4. Data Model & State

### 4.1 Module-level state

`policyLimits/index.ts`:
- `pollingIntervalId: ReturnType<typeof setInterval> | null` — background polling handle, `unref()`'d (`:647`).
- `cleanupRegistered: boolean` — guard for `registerCleanup` (`:62`).
- `loadingCompletePromise / loadingCompleteResolve` — initial-load await rendezvous (`:65-66`).
- `sessionCache: PolicyLimitsResponse['restrictions'] | null` (`:72`).
- Cache file: `${getClaudeConfigHomeDir()}/policy-limits.json`, `mode 0o600` (`:55,418`).

`remoteManagedSettings`:
- `pollingIntervalId` (`index.ts:57`).
- `loadingCompletePromise / loadingCompleteResolve` (`index.ts:61-62`).
- `syncCache.ts: cached: boolean | undefined` (eligibility memo).
- `syncCacheState.ts: sessionCache: SettingsJson | null` and `eligible: boolean | undefined` (tri-state).
- Cache file: `${getClaudeConfigHomeDir()}/remote-settings.json`, opened with `mode 0o600`, `datasync()`'d (`index.ts:367-378`; filename `syncCacheState.ts:32`).

`settingsSync`:
- `downloadPromise: Promise<boolean> | null` (`index.ts:115`) — first call wins; subsequent calls join.

### 4.2 Cache file paths and modes

| Service | Path | Mode | Sync? |
|---|---|---|---|
| Policy limits | `${claudeConfigHomeDir}/policy-limits.json` | `0o600` | `fs.writeFile` (no fsync) |
| Remote managed settings | `${claudeConfigHomeDir}/remote-settings.json` | `0o600` | explicit `datasync()` |

### 4.3 `SettingSource` enum participation

`policyLimits` does NOT participate in `SettingSource`. `remoteManagedSettings` is the highest-priority slot of the **`policySettings` source** in the precedence chain owned by spec 02.

## 5. Algorithm / Control Flow

### 5.1 `policySettings` cascade resolver (this spec OWNS the body)

Pseudocode for `getSettingsForSourceUncached('policySettings')` (`utils/settings/settings.ts:319-345`) and `getSettings()` merge body (`:677-739`). Rule: **first source wins** — the highest-priority source with a non-empty object provides the entire `policySettings` layer; lower sources are NOT merged.

```
policy_resolve():
    # 1. Remote managed settings (highest)
    remote = getRemoteManagedSettingsSyncFromCache()       # syncCacheState.ts
    if remote and Object.keys(remote).length > 0:
        validate against SettingsSchema (in merge body) — on failure, push errors and FALL THROUGH
        return remote

    # 2. Admin-only MDM (HKLM on Windows, /Library/Managed Preferences/ on macOS)
    mdm = getMdmSettings()
    if Object.keys(mdm.settings).length > 0:
        return mdm.settings

    # 3. File-based managed settings (admin-deployed on disk)
    file = loadManagedFileSettings()    # managed-settings.json + managed-settings.d/*.json
    if file != null:
        return file

    # 4. HKCU (Windows user-writable, lowest)
    hkcu = getHkcuSettings()
    if Object.keys(hkcu.settings).length > 0:
        return hkcu.settings

    return null
```

Origin reporter `getPolicySettingsOrigin()` (`settings.ts:375-407`) returns `'remote' | 'plist' | 'hklm' | 'file' | 'hkcu' | null` (plist vs hklm chosen by `getPlatform() === 'macos'`).

### 5.2 File-based managed settings (slot 3) merge order

`loadManagedFileSettings()` (`settings.ts:74-121`):

1. Parse `${managedFilePath}/managed-settings.json` as base (lowest precedence within this slot).
2. `readdirSync(${managedFilePath}/managed-settings.d)`, filter to non-dot `.json` files (regular files or symlinks), `sort()` alphabetically, `mergeWith()` each in order using `settingsMergeCustomizer`. Later files override earlier.
3. Returns `{ settings: found ? merged : null, errors }`. On `ENOENT`/`ENOTDIR` of drop-in dir, ignore. Convention matches systemd/sudoers drop-in (`settings.ts:67-70`).

### 5.3 Remote managed settings fetch lifecycle

`fetchRemoteManagedSettings(cachedChecksum)` (`remoteManagedSettings/index.ts:248-361`):

1. `await checkAndRefreshOAuthTokenIfNeeded()`.
2. Build auth headers via `getRemoteSettingsAuthHeaders()`: try API key (`x-api-key`) first, fall back to OAuth (`Authorization: Bearer …` + `anthropic-beta: <OAUTH_BETA_HEADER>`). On no auth → `skipRetry:true`.
3. `axios.get(endpoint, { timeout: 10000, headers: { …auth, 'User-Agent': getClaudeCodeUserAgent(), 'If-None-Match': '"${cachedChecksum}"'? } })`. `validateStatus`: 200/204/304/404.
4. Handle status: 304 → `{ success:true, settings:null, checksum:cachedChecksum }` (cache valid); 204/404 → `{ success:true, settings:{}, checksum:undefined }`.
5. Validate: `RemoteManagedSettingsResponseSchema.safeParse(response.data)` then `SettingsSchema().safeParse(parsed.data.settings)`. Either failure → `{ success:false, error:... }` (no `skipRetry`, retried).
6. Errors via `classifyAxiosError`: `auth` → `skipRetry:true`; `timeout` / `network` / default mapped to error string. (`status===404` inside the catch-block at `index.ts:341-344` returns `{ success:true, settings:{}, checksum:'' }` — but this branch is **dead defensive code**: `validateStatus` at `:283-284` already includes 404, so a 404 response never throws and the catch-side 404 handler is unreachable through any normal axios behavior. Reimplementers should preserve the branch verbatim for source-compat (and as a guard against future `validateStatus` edits) but a faithful behavioral reimplementation can omit it without observable difference. NOTE: the catch-branch returns `checksum: ''` whereas the success-path 204/404 returns `checksum: undefined` — another reason to keep it textually but recognize it as belt-and-suspenders.)

`fetchWithRetry`: up to `DEFAULT_MAX_RETRIES = 5` retries (so 6 attempts), delay from `getRetryDelay(attempt)` between attempts; abort on `success` or `skipRetry`.

`fetchAndLoadRemoteManagedSettings()` (`:415-503`):
- Compute `cachedChecksum = computeChecksumFromSettings(cached)` (sha256 of canonicalised JSON) **locally** (NOT stored on disk; recomputed each load).
- Stale-on-failure: on fetch failure with cached present → reuse cache. 304 → reuse cache.
- Non-empty new settings → `checkManagedSettingsSecurity(cached, new)` (see §5.5); on `'rejected'` `gracefulShutdownSync(1)`; otherwise `setSessionCache(new)` then `saveSettings()`.
- Empty (404) → `setSessionCache({})` then `unlink(getSettingsPath())` (ignore ENOENT).

`loadRemoteManagedSettings()` (`:514-555`): cache-first — if `getRemoteManagedSettingsSyncFromCache()` returns non-null, resolve `loadingCompletePromise` immediately (saves ~77ms on print-mode startup, comment at `:526-529`); then run fetch in background; `notifyChange('policySettings')` if any settings loaded. Background polling kicked off on eligible.

Background poll: `setInterval(pollRemoteSettings, 60*60*1000)` (`:54,623`); `unref()`'d. Compares pre/post `jsonStringify` and fires `settingsChangeDetector.notifyChange('policySettings')` only when changed. Cleanup via `registerCleanup`.

### 5.4 Checksum canonicalisation

`computeChecksumFromSettings(settings)` (`remoteManagedSettings/index.ts:131-137`) and `policyLimits/index.ts: computeChecksum()` (`:152-159`):

```
sortKeysDeep(value):
    Array → map(sortKeysDeep)
    Plain object → new object with keys sorted (Object.keys(...).sort() for remote;
                   .sort(([a],[b]) => a.localeCompare(b)) for policyLimits — different ordering!)
    else → value
checksum = "sha256:" + sha256(jsonStringify(sortKeysDeep(input)))
```

Comment at `remoteManagedSettings/index.ts:128-129`: must match server Python `json.dumps(settings, sort_keys=True, separators=(",", ":"))`. **`policyLimits` uses `localeCompare` ordering whereas `remoteManagedSettings` uses default `Array.prototype.sort()` ordering** (verbatim divergence — `policyLimits/index.ts:139-141` vs `remoteManagedSettings/index.ts:118`).

**Confirmed source bug — policyLimits is wrong, remoteManagedSettings is correct.** Python's `sort_keys=True` performs a code-point sort (equivalent to JavaScript's default `Array.prototype.sort()` over strings). `localeCompare` performs a locale-sensitive sort that diverges for non-ASCII keys (and for some ASCII edge cases under non-`en-US` locales). The `remoteManagedSettings` implementation matches Python; `policyLimits` does not. For policy-limits, the divergence will produce a CLIENT-side checksum that disagrees with what the server would compute over the same restrictions object, leading to a forced-cache-miss on every request whose restrictions contain non-ASCII keys (the server returns 200 instead of 304, and the cache file is rewritten on every poll). No security impact (restrictions content is unchanged), but bandwidth and disk-write churn are above zero. Cataloged in `docs/specs/BUGS-IN-SOURCE.md`. Suggested upstream fix: change `policyLimits/index.ts:139-141` to `Object.keys(obj).sort()` matching `remoteManagedSettings/index.ts:118`.

### 5.5 Managed-settings security gate

`checkManagedSettingsSecurity(cached, new)` (`securityCheck.tsx:22-61`):

1. New has no dangerous settings (`hasDangerousSettings(extractDangerousSettings(new))`) → `'no_check_needed'`.
2. Dangerous unchanged (`!hasDangerousSettingsChanged(cached, new)`) → `'no_check_needed'`.
3. `!getIsInteractive()` → `'no_check_needed'`.
4. `logEvent('tengu_managed_settings_security_dialog_shown', {})`, render `<ManagedSettingsSecurityDialog>` blocking; on accept log `…_accepted` + resolve `'approved'`; on reject log `…_rejected` + resolve `'rejected'`.

`handleSecurityCheckResult(result)` (`:67-73`): `'rejected'` → `gracefulShutdownSync(1)` + return `false`. Otherwise return `true`.

### 5.6 Policy limits fetch + resolution

`fetchPolicyLimits(cachedChecksum)` (`policyLimits/index.ts:300-386`): same skeleton as remote settings. Auth: API key first, OAuth fallback. `validateStatus`: 200/304/404. Endpoint: `${BASE_API_URL}/api/claude_code/policy_limits`. Timeout 10s. 304 → reuse cache; 404 → `restrictions:{}` (treated as "no restrictions"); non-success retried up to 5×.

`isPolicyAllowed(policy)` (`:510-526`):

```
restrictions = getRestrictionsFromCache()          # null if ineligible or no cache
if restrictions == null:
    if isEssentialTrafficOnly() and policy in ESSENTIAL_TRAFFIC_DENY_ON_MISS:
        return false                               # fail-closed exception
    return true                                    # fail-open default
r = restrictions[policy]
return r ? r.allowed : true                        # unknown key = allowed
```

`ESSENTIAL_TRAFFIC_DENY_ON_MISS = new Set(['allow_product_feedback'])` (`:502`). `isEssentialTrafficOnly()` is true iff `process.env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is set (HIPAA orgs).

Background poll: `setInterval(pollPolicyLimits, 60*60*1000)` (`:58,646`), `unref()`'d. Pre/post compare via `jsonStringify` (no notifyChange — policy limits are not part of `SettingSource`).

### 5.7 Settings sync — upload (interactive CLI)

`uploadUserSettingsInBackground()` (`settingsSync/index.ts:60-111`):

Eligibility ALL of: `feature('UPLOAD_USER_SETTINGS')` AND GrowthBook `tengu_enable_settings_sync_push` AND `getIsInteractive()` AND `isUsingOAuth()` (firstParty + first-party base URL + token has `CLAUDE_AI_INFERENCE_SCOPE`). Failure → `tengu_settings_sync_upload_skipped_ineligible`, return.

```
result = fetchUserSettings()           # GET /user_settings, retries=3
if !result.success: log fetch_failed, return
projectId = await getRepoRemoteHash()  # may be null
local  = buildEntriesFromLocalFiles(projectId)
remote = result.isEmpty ? {} : result.data.content.entries
changed = pickBy(local, (v,k) => remote[k] !== v)        # lodash; keep only deltas
if Object.keys(changed).length === 0: log no_changes, return
upload = uploadUserSettings(changed)   # PUT /user_settings, no retry
```

`buildEntriesFromLocalFiles` (`:418-459`) reads via `tryReadFileForSync` (size cap 500KB, skip empty/whitespace-only):

| Key | Source path |
|---|---|
| `~/.claude/settings.json` | `getSettingsFilePathForSource('userSettings')` |
| `~/.claude/CLAUDE.md` | `getMemoryPath('User')` |
| `projects/${projectId}/.claude/settings.local.json` | `getSettingsFilePathForSource('localSettings')` |
| `projects/${projectId}/CLAUDE.local.md` | `getMemoryPath('Local')` |

Project-scoped entries only included when `projectId` is non-null.

### 5.8 Settings sync — download (CCR / print-mode)

`downloadUserSettings()` (`:129-135`) memoizes one in-flight promise. `redownloadUserSettings()` (`:152-155`) bypasses cache and disables retries (single attempt). Both call `doDownloadUserSettings(maxRetries=3 default | 0)`:

```
if !feature('DOWNLOAD_USER_SETTINGS'): return false
if !growthbook('tengu_strap_foyer', false) or !isUsingOAuth():
    log skipped + tengu_settings_sync_download_skipped, return false
result = fetchUserSettings(maxRetries)
if !result.success: log fetch_failed, return false
if result.isEmpty:  log empty + tengu_settings_sync_download_empty, return false
applyRemoteEntriesToLocal(result.data.content.entries, await getRepoRemoteHash())
log tengu_settings_sync_download_success
return true
```

`applyRemoteEntriesToLocal` (`:488-581`):
- Defense-in-depth size check: each entry > 500KB → skip with diagnostic.
- `markInternalWrite(path)` for `userSettings`/`localSettings` paths (suppresses `changeDetector` spurious detection — comment `:548-549`).
- Memory files written without `markInternalWrite` (comment at `redownloadUserSettings` `:151-153` — the caller is responsible for `notifyChange` mid-session).
- After loop: if any settings written → `resetSettingsCache()`; if any memory written → `clearMemoryFileCaches()`.

### 5.9 Eligibility decision tables

**`isPolicyLimitsEligible()`** (`policyLimits/index.ts:167-211`):
- `getAPIProvider() !== 'firstParty'` → false.
- `!isFirstPartyAnthropicBaseUrl()` → false.
- API key present (try `getAnthropicApiKeyWithSource({skipRetrievingKeyFromApiKeyHelper:true})`) → true.
- OAuth tokens missing `accessToken` → false; missing `CLAUDE_AI_INFERENCE_SCOPE` → false.
- Subscription neither `'enterprise'` nor `'team'` → false; otherwise true.

**`isRemoteManagedSettingsEligible()`** (`remoteManagedSettings/syncCache.ts:49-112`):
- Memoised in `cached: boolean | undefined`.
- 3p provider OR custom base URL → false.
- `process.env.CLAUDE_CODE_ENTRYPOINT === 'local-agent'` → false (cowork VM, comment `:64-67`).
- OAuth tokens with `subscriptionType === null` (externally injected via `CLAUDE_CODE_OAUTH_TOKEN`/`_FILE_DESCRIPTOR`/Agent SDK) → true (let API decide).
- OAuth + inference scope + (`'enterprise'` | `'team'`) → true.
- API key present → true.
- Else false. Mirrors result via `setEligibility()` to leaf state.

Eligibility tri-state in `syncCacheState.ts:34-35`: `undefined` (not yet determined) → `getRemoteManagedSettingsSyncFromCache()` returns null even with cached file; only after `setEligibility(true)` does the file load.

## 6. Verbatim Assets

### 6.1 API endpoints (verbatim)

| Service | Endpoint |
|---|---|
| Policy limits | `${getOauthConfig().BASE_API_URL}/api/claude_code/policy_limits` |
| Remote managed settings | `${getOauthConfig().BASE_API_URL}/api/claude_code/settings` |
| Settings sync (upload/download) | `${getOauthConfig().BASE_API_URL}/api/claude_code/user_settings` |

Backend issue ref (`settingsSync/index.ts:9`): `anthropic/anthropic#218817`. Common headers: `User-Agent: getClaudeCodeUserAgent()`. OAuth: `Authorization: Bearer ${accessToken}`, `anthropic-beta: ${OAUTH_BETA_HEADER}`. API key: `x-api-key: ${apiKey}`. ETag: `If-None-Match: "${checksum}"`.

### 6.2 macOS plist path layout (verbatim, `mdm/constants.ts:11-81`)

```
const MACOS_PREFERENCE_DOMAIN = 'com.anthropic.claudecode'

Plist paths (priority order, highest first):
  1. /Library/Managed Preferences/${username}/com.anthropic.claudecode.plist   (per-user managed preferences)  [included only if userInfo().username succeeds]
  2. /Library/Managed Preferences/com.anthropic.claudecode.plist               (device-level managed preferences)
  3. ${homedir()}/Library/Preferences/com.anthropic.claudecode.plist           (user preferences (ant-only))    [only when process.env.USER_TYPE === 'ant']

Tooling: /usr/bin/plutil
Args:    ['-convert', 'json', '-o', '-', '--', <path>]
Subprocess timeout: 5000 ms
Existence fast-path: existsSync(path) before spawn (saves ~5ms ENOENT)
```

### 6.3 Windows registry path layout (verbatim, `mdm/constants.ts:23-29`)

```
HKLM key:   HKLM\SOFTWARE\Policies\ClaudeCode      (admin-deployed; first wins)
HKCU key:   HKCU\SOFTWARE\Policies\ClaudeCode      (user-writable; lowest priority)
Value name: Settings
Type:       REG_SZ or REG_EXPAND_SZ (case-insensitive match by parser)
Tooling:    reg query <KEY> /v Settings
```

Both keys live under `SOFTWARE\Policies` (WOW64 shared list; no 32/64 redirection — comment `:18-22`). Stdout regex (`mdm/settings.ts:213-214`): `/^\s+${escaped}\s+REG_(?:EXPAND_)?SZ\s+(.*)$/i`.

### 6.4 File-based managed settings paths (verbatim, `managedPath.ts:8-25`)

```
macOS:    /Library/Application Support/ClaudeCode
Windows:  C:\Program Files\ClaudeCode
Linux:    /etc/claude-code

Files:
  ${managedFilePath}/managed-settings.json           — base (lowest within slot)
  ${managedFilePath}/managed-settings.d/*.json       — drop-ins, sorted ascii, later wins

ANT-only override: process.env.CLAUDE_CODE_MANAGED_SETTINGS_PATH (only when USER_TYPE === 'ant')
```

### 6.5 Cache file paths (verbatim)

```
${getClaudeConfigHomeDir()}/policy-limits.json        (mode 0600)
${getClaudeConfigHomeDir()}/remote-settings.json      (mode 0600, with datasync())
```

### 6.6 Constants (verbatim)

| Constant | Value | Source |
|---|---|---|
| `FETCH_TIMEOUT_MS` (policy) | 10000 | `policyLimits/index.ts:56` |
| `DEFAULT_MAX_RETRIES` (policy) | 5 | `policyLimits/index.ts:57` |
| `POLLING_INTERVAL_MS` (policy) | `60 * 60 * 1000` (1 h) | `policyLimits/index.ts:58` |
| `LOADING_PROMISE_TIMEOUT_MS` | 30000 | `policyLimits/index.ts:69`, `remoteManagedSettings/index.ts:66` |
| `SETTINGS_TIMEOUT_MS` (remote) | 10000 | `remoteManagedSettings/index.ts:52` |
| `DEFAULT_MAX_RETRIES` (remote) | 5 | `remoteManagedSettings/index.ts:53` |
| `POLLING_INTERVAL_MS` (remote) | `60 * 60 * 1000` (1 h) | `remoteManagedSettings/index.ts:54` |
| `MDM_SUBPROCESS_TIMEOUT_MS` | 5000 | `mdm/constants.ts:38` |
| `MDM_POLL_INTERVAL_MS` | `30 * 60 * 1000` (30 min) | `changeDetector.ts:51` |
| `SETTINGS_SYNC_TIMEOUT_MS` | 10000 | `settingsSync/index.ts:51` |
| `DEFAULT_MAX_RETRIES` (sync) | 3 | `settingsSync/index.ts:52` |
| `MAX_FILE_SIZE_BYTES` | `500 * 1024` (500 KB) | `settingsSync/index.ts:53` |
| `CACHE_FILENAME` (policy) | `'policy-limits.json'` | `policyLimits/index.ts:55` |
| `SETTINGS_FILENAME` (remote) | `'remote-settings.json'` | `syncCacheState.ts:32` |
| `ESSENTIAL_TRAFFIC_DENY_ON_MISS` | `new Set(['allow_product_feedback'])` | `policyLimits/index.ts:502` |

### 6.7 Known policy keys (from call sites)

| Policy key | Effect when `false` | Cited at |
|---|---|---|
| `allow_remote_sessions` | Block teleport, RemoteTriggerTool, `/remote-setup`, `/remote-env`, print-mode remote sessions | `main.tsx:3405`, `RemoteTriggerTool.ts:60`, `teleport.tsx:431`, `remoteSession.ts:53`, `print.ts:4991`, `remote-setup/index.ts:13`, `remote-env/index.ts:10` |
| `allow_remote_control` | Block bridge / IDE remote control | `initReplBridge.ts:155`, `cli.tsx:157`, `bridge.tsx:474` |
| `allow_product_feedback` | Block in-app feedback surveys + `/feedback`; **fail-closed under essential-traffic-only** | `useFeedbackSurvey.tsx:{136,237}`, `useMemorySurvey.tsx:{99,178}`, `feedback/index.ts:21` |

### 6.8 User-facing policy denial strings (verbatim)

```
"Remote sessions are disabled by your organization's policy."     teleport.tsx:432
"Remote sessions are disabled by your organization's policy.",    cli/print.ts:4993
"Error: Remote Control is disabled by your organization's policy."   entrypoints/cli.tsx:158
"Remote Control is disabled by your organization's policy."         commands/bridge/bridge.tsx:475
"disabled by your organization's policy"                            bridge/initReplBridge.ts:160 (passed to onStateChange('failed', …))
'Blocked by enterprise policy (allowedMcpServers/deniedMcpServers)' cli/print.ts:5368
'Cannot set permission mode to bypassPermissions because it is disabled by settings or configuration'   cli/print.ts:4583
```

Internal log strings (not user-visible): `'policy_denied'` (initReplBridge.ts:157), `{ type: 'policy_blocked' }` (remoteSession.ts:54).

### 6.9 Analytics events (verbatim — see also spec 26)

```
tengu_managed_settings_security_dialog_shown
tengu_managed_settings_security_dialog_accepted
tengu_managed_settings_security_dialog_rejected
tengu_settings_sync_upload_skipped_ineligible
tengu_settings_sync_upload_fetch_failed
tengu_settings_sync_upload_no_changes      (logged via diag only)
tengu_settings_sync_upload_skipped
tengu_settings_sync_upload_success         (entryCount)
tengu_settings_sync_upload_failed          (entryCount)
tengu_settings_sync_download_skipped
tengu_settings_sync_download_fetch_failed
tengu_settings_sync_download_empty
tengu_settings_sync_download_success       (entryCount)
tengu_settings_sync_download_error
mdm_settings_loaded                        (duration_ms, key_count, error_count)
```

### 6.10 Rate-limit / quota constants

The CLI has no internally-defined token quota or rate-limit *value* constants in this subsystem. Quota enforcement is server-side; the CLI simply maps HTTP responses to user messages via `services/claudeAiLimits*.ts` / `services/rateLimit*.ts` / `services/mockRateLimits.ts` — those files are owned by **spec 22** (per master-plan most-recent decision) and not redocumented here. This spec only provides the `isPolicyAllowed` boolean predicate and the in-band 1-hour and 30-minute polling intervals listed in §6.6.

## 7. Side Effects & I/O

- **Filesystem writes**: `~/.claude/policy-limits.json` (0600), `~/.claude/remote-settings.json` (0600 + datasync), `markInternalWrite()`-tagged writes to user/local settings paths (download path), memory files via `getMemoryPath()`. `unlink` on 404 + clear paths.
- **Filesystem reads**: managed-settings.json + drop-in dir (sync `readdirSync`/`readFileSync`); plist `existsSync` fast-path.
- **Network**: 3 endpoints in §6.1; axios; 10s timeout; ETag on remote-managed-settings + policy-limits; sync upload uses `PUT { entries }`, download uses `GET`.
- **Process spawn**: `/usr/bin/plutil -convert json -o - -- <path>` (macOS); `reg query <KEY> /v Settings` (Windows). 5s timeout. Stdout-only consumption.
- **Env vars consumed**: `USER_TYPE` (gate ANT plist + `CLAUDE_CODE_MANAGED_SETTINGS_PATH` + `protectedNamespace`), `CLAUDE_CODE_ENTRYPOINT` (`local-agent` opt-out), `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` (essential-traffic-only mode), `CLAUDE_CODE_OAUTH_TOKEN`/`_FILE_DESCRIPTOR` (eligibility hint).
- **External binaries**: `plutil`, `reg`.
- **Trust boundaries**: remote managed settings *can* alter every key in `SettingsJson`. `checkManagedSettingsSecurity` provides interactive consent on dangerous-setting changes; `gracefulShutdownSync(1)` on rejection. Non-interactive mode bypasses the dialog (consistent with trust-dialog behavior — `securityCheck.tsx:33-36`).

## 8. Feature Flags & Variants

| Flag / gate | Effect |
|---|---|
| `feature('UPLOAD_USER_SETTINGS')` off | `uploadUserSettingsInBackground` short-circuits |
| `feature('DOWNLOAD_USER_SETTINGS')` off | `doDownloadUserSettings` returns false immediately |
| `tengu_enable_settings_sync_push` (GrowthBook, default false) | Required true for upload |
| `tengu_strap_foyer` (GrowthBook, default false) | Required true for download |
| `USER_TYPE === 'ant'` | Adds user plist to MDM list; honors `CLAUDE_CODE_MANAGED_SETTINGS_PATH`; lazy-requires `protectedNamespace.js` for `isInProtectedNamespace` |
| `CLAUDE_CODE_ENTRYPOINT === 'local-agent'` | Skips remote managed settings entirely (cowork) |
| 3p provider / custom base URL | Skips both `policyLimits` and `remoteManagedSettings` |
| OAuth subscription type | Required `enterprise` or `team` (or `null` for externally-injected tokens — remote settings only) |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | Flips `isPolicyAllowed('allow_product_feedback')` from fail-open to fail-closed when cache absent |

## 9. Error Handling & Edge Cases

- **Fail-open default** for both `policyLimits` and `remoteManagedSettings`: cache missing + fetch failed → return null/empty, no restrictions.
- **Stale-cache fallback**: every fetch failure path uses `cachedRestrictions`/`cachedSettings` if present (`policyLimits/index.ts:449-452,488-492`; `remoteManagedSettings/index.ts:434-440,494-498`).
- **`skipRetry`**: auth errors (401/403) and missing-auth set `skipRetry:true` so `fetchWithRetry` returns immediately.
- **304 Not Modified**: cached version is reused; `etag`/`checksum` echo back unchanged.
- **404 / 204** on remote-managed-settings: empty `{}`, cache file is `unlink()`'d (with `ENOENT` ignored). On policy-limits: same — `restrictions:{}`.
- **Schema validation failure**: returns `{ success:false }` (retryable). Note remote-managed-settings does **two** safeParse passes: response envelope, then `SettingsSchema` over `parsed.data.settings`.
- **`subscriptionType === null`** OAuth tokens: still eligible (let API decide); empty-restrictions response handled gracefully.
- **Loading-promise deadlock guard**: 30s fallback resolves the wait promise even if `loadXxx()` is never called (Agent SDK / non-CLI contexts — `policyLimits/index.ts:103-111`, `remoteManagedSettings/index.ts:88-96`).
- **Settings-sync upload swallows all errors** to never block startup; download is fail-open (returns false). Per-file size cap 500KB skipped silently with diag log.
- **MDM subprocess errors / missing files**: 5s timeout; non-zero exit → empty stdout. plist `existsSync` fast-path avoids spawning when file absent.
- **Drop-in dir missing**: `readdirSync` throws ENOENT/ENOTDIR → silently treated as no drop-ins.
- **First-source-wins side effect**: a non-empty remote settings object that fails `SettingsSchema` validation **does NOT fall through** to MDM in the per-source uncached read (`settings.ts:319-345`); but the merge body `getSettings()` (`:677-739`) does collect errors from validation and DOES fall through (`if (!policySettings)` chain at `:696,705,714`).
- **TWO independent `resetSettingsCache()` paths exist for remote settings becoming visible**, and a reimplementer must preserve BOTH or risk a silent permission-bypass window:
  - **(a) Sync-getter direct reset** (`syncCacheState.ts:92`): `getRemoteManagedSettingsSyncFromCache()` calls `resetSettingsCache()` directly, exactly once, the first time `eligible === true` AND `sessionCache === null` AND `loadSettings()` returns non-null. Subsequent calls hit the `if (sessionCache)` early-return at `:72` and never reset again. Triggered by callers reading the sync cache before any async fetch (e.g. cascade in `settings.ts:324,682,683`). Workaround for gh-23085 / `managedSettingsHeadless.int.test.ts` poisoned-merged-cache scenario where `getSettings_DEPRECATED()` was reached at `auth.ts:115` from `isBridgeEnabled()` at Commander-definition time before `init()`.
  - **(b) Async-fetch indirect reset** (`index.ts:543-544` and similar `notifyChange('policySettings')` sites): `loadRemoteManagedSettings()` / `refreshRemoteManagedSettings()` / poll-diff path call `notifyChange('policySettings')`, which fans out through `changeDetector.ts` listeners to `applySettingsChange.ts` → `resetSettingsCache()`. Triggered when an async fetch produces new or changed settings.
  - **Why both must exist**: path (a) covers cold-start cache load before any network; path (b) covers in-process settings change. The reset window is the moment the policy-blocked feature stops being permitted for the running process — getting either path's lifetime wrong is a security regression (a stale merged-settings cache returns the pre-policy `policySettings` layer for the duration of the staleness). Spec 02 owns the merged-settings cache; this spec OWNS the trigger sites.
  - **NOT-equivalent note**: path (a) resets the merged cache **synchronously** before the cascade returns the value to its caller. Path (b) resets via the listener fan-out — ordering relative to other listeners is `changeDetector`-internal (see §12 hardest-to-verify and the Phase 9.5 adversarial F1 follow-up).

## 10. Telemetry & Observability

- Debug logs via `logForDebugging` covering: policy limits fetch/304/404, retry counts, save success/failure, "stale cache" reuse, "Changed during background poll".
- Diagnostic-no-PII logs (`logForDiagnosticsNoPII`) for settings-sync upload/download phases.
- `logEvent` events listed verbatim in §6.9.
- Per-`mdm_settings_loaded` event includes `duration_ms`, `key_count`, `error_count`.
- `profileCheckpoint('mdm_load_start' | 'mdm_load_end')` (`mdm/settings.ts:70,79`) — startup profiler hooks.
- `isInProtectedNamespace()` value attached to telemetry events at every `inProtectedNamespace` site (`main.tsx:4583`, `agentToolUtils.ts:437`, bridge*, permissions* — used purely as analytics dimension).

## 11. Reimplementation Checklist

A reimplementer must preserve:

- The exact `policySettings` cascade resolver (4-step first-source-wins; remote → MDM(HKLM/plist) → file → HKCU) and its **two** different orderings: per-source uncached path falls through on empty only; merge body falls through on `!policySettings` and accumulates errors (`settings.ts:677-739`).
- File-managed slot's exact merge order: `managed-settings.json` first, then `managed-settings.d/*.json` sorted ASCII ascending with `lodash.mergeWith(settingsMergeCustomizer)`; symlinks honored; dotfiles skipped.
- macOS plist priority: per-user (only with valid `userInfo().username`) → device → ant-only user-pref. `existsSync` fast-path. `plutil -convert json -o - --` invocation.
- Windows: HKLM and HKCU **both** under `\SOFTWARE\Policies\ClaudeCode`, value name `Settings`, parsed with `^\s+Settings\s+REG_(?:EXPAND_)?SZ\s+(.*)$/i`; HKLM consumed before file-based, HKCU after.
- Both checksum implementations: sha256 of canonicalised JSON, but `policyLimits` uses `localeCompare` and `remoteManagedSettings` uses default `Array.sort()`; both prefix `"sha256:"`. Match server-side Python canonicalisation.
- Cache files at `~/.claude/policy-limits.json` and `~/.claude/remote-settings.json` with mode `0o600`; remote settings uses `open + writeFile + datasync + close`.
- Eligibility tri-state for remote settings: undefined (not yet evaluated) returns null even with cached file present; only after `setEligibility(true)` does the file load.
- Loading-promise deadlock guard (30s timeout).
- 1-hour background polling for both services (`unref()`'d, `registerCleanup`'d). 30-minute MDM poll in `changeDetector` (separate timer; emits `notifyChange('policySettings')` only on snapshot diff).
- `notifyChange('policySettings')` calls on: initial load (if non-null), refresh after auth change, polling diff. The MDM poll fires `fanOut('policySettings')` directly via `changeDetector`.
- Security gate before applying new remote settings: `extractDangerousSettings` → `hasDangerousSettings` → `hasDangerousSettingsChanged` → interactive dialog only when `getIsInteractive()`; `gracefulShutdownSync(1)` on reject.
- `isPolicyAllowed`: fail-open default; unknown keys allowed; `allow_product_feedback` fail-closed under `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`.
- Settings sync: upload (interactive only) computes `pickBy` delta against remote before PUT; download memoizes single in-flight promise; `redownloadUserSettings` uses 0 retries; `markInternalWrite` only on settings paths, not memory paths; size limit 500 KB; defense-in-depth size check on apply.
- `SYNC_KEYS` literal values (with `~/.claude/...` and `projects/${projectId}/...` prefixes) verbatim — these are over-the-wire keys.
- `isInProtectedNamespace` returns `false` always in non-ANT builds (require call DCE'd). Used only for telemetry, never for permission gating.
- **`sandbox.excludedCommands` consumes this resolver (cross-link to spec 10).** `BashTool.tsx:343` reads `getSettings().policySettings.sandbox?.excludedCommands` to decide which bash commands bypass the sandbox. The `policySettings` layer it reads is fully resolved by this spec's cascade — meaning enterprise-managed sandbox exclusions take effect via the same `remote > MDM > file > HKCU` precedence as every other `policySettings` key. Spec 10 owns the consumer; spec 27 owns the resolver. (`tengu_sandbox_disabled_commands`, the GrowthBook payload at `shouldUseSandbox.ts:27`, is entirely separate and OWNED by spec 10 / spec 26.)
- **Plugin policy chokepoint (cross-link to spec 28).** `policySettings.enabledPlugins[<id>]` is consumed only via `utils/plugins/pluginPolicy.ts:isPluginBlockedByPolicy`. Reimplementers MUST keep that predicate as the single chokepoint — direct reads of `enabledPlugins` from plugin call sites would bypass any future hardening on the wrapper (see spec 28 §3 for the call graph).

## 12. Open Questions / Unknowns

- `src/utils/protectedNamespace.ts` is referenced by `envUtils.ts:142` but absent from the leaked tree. ANT-only — its source is presumably stripped during external bundling. Body of `checkProtectedNamespace()` cannot be reproduced bit-exact without ANT source. Behaviour comment promises "conservative — when signals ambiguous, assume protected" and references a "namespace allowlist" plus k8s/COO signals; not reproducible from external source.
- The exact list of `dangerous settings` keys consumed by `hasDangerousSettings` lives in `src/components/ManagedSettingsSecurityDialog/utils.ts` (not read here — owned by spec 02 / 37). Reimplementers MUST consult that module for the complete list.
- `OAUTH_BETA_HEADER` value and `BASE_API_URL` resolution — owned by spec 25 (OAuth) / spec 22 (api). This spec only references them.
- `claudeAiLimits.ts` / `rateLimitMessages.ts` and friends: this spec defers to spec 22 (per master-plan most-recent decision) for HTTP 429 mapping, mock rate limits, and quota messaging strings.
- The `settingsMergeCustomizer` semantics (merge vs replace per key) live in `utils/settings/settings.ts` and are owned by spec 02.
- `getRetryDelay(attempt)` curve — owned by spec 22 (`services/api/withRetry.js`).
- ~~The two checksum implementations differ in object-key sort order (`localeCompare` vs default lexicographic). Whether this is a bug or intentional (different server schemas) is unknown from source alone~~ — **resolved Phase 9.6c**: the divergence is a confirmed bug in `policyLimits/index.ts:139-141` (Python `sort_keys=True` matches default `.sort()`, not `localeCompare`). See §5.4 and `docs/specs/BUGS-IN-SOURCE.md`.
- The `'auto'` permission mode and `bubble` permission mode interactions with `policySettings.skipDangerousModePermissionPrompt` / `skipAutoPermissionPrompt` / `useAutoModeDuringPlan` (`settings.ts:887,903,921`) are owned by spec 09; mentioned here only for cross-reference.
