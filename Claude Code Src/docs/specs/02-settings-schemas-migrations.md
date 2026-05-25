# Settings, Schemas, and Migrations Specification

> Owner of `src/schemas/`, `src/migrations/`, and the entire settings cascade under `src/utils/settings/`. This subsystem is the single source of truth for **on-disk user-facing configuration** (settings files, MDM/registry/plist, drop-ins, CLI `--settings`/inline-SDK settings, plugin base layer). Tool/permission/hook **runtime semantics** are owned by 09; this spec owns the schema entries that **define them on disk**.

Adjacent specs: 00 (overview, conventions, glossary), 01 (entrypoint/bootstrap — owns the `startMdmRawRead` prefetch wiring), 09 (permission rule runtime), 26 (analytics — consumes `tengu_*` migration events), 27 (policy/limits — consumes `policySettings`), 28 (plugins — owns plugin-manifest schema + writes to `pluginSettingsBase`), 17 (skills — consumes `SkillHookMatcher`), 23 (MCP — consumes `enabledMcpjsonServers`/`allowedMcpServers`/`deniedMcpServers`/`pluginConfigs.mcpServers`).

---

## 1. Purpose & Scope

The settings subsystem is the input layer for nearly every other subsystem. It is responsible for:

1. Locating settings on disk for **five named sources**: `userSettings`, `projectSettings`, `localSettings`, `flagSettings`, `policySettings`. (`src/utils/settings/constants.ts:7-22`)
2. Reading those sources, including OS-level **MDM/registry/plist** for `policySettings` and a separate `managed-settings.d/` drop-in directory.
3. Validating each source via **Zod v4** (`SettingsSchema`, hook schemas, permission-rule schema, sandbox schema, plugin marketplace schemas), filtering invalid permission rules pre-validation so one bad rule doesn't poison the file.
4. Merging sources via `lodash mergeWith` with a **per-key array union+dedupe** customizer plus an explicit `undefined === delete` rule for object writes.
5. Caching at three levels (per-path parse cache, per-source cache, session merged cache) and invalidating on (a) explicit reset, (b) chokidar file events, (c) MDM 30-minute poll.
6. Watching all five sources on disk via chokidar (`awaitWriteFinish` + delete grace + internal-write echo suppression) and dispatching `ConfigChange` hooks before applying.
7. Running an ordered chain of **one-shot migrations** at startup that mutate `userSettings` and the legacy `~/.claude.json` `GlobalConfig`.
8. Exposing a derived `getSettingsWithErrors()` with formatted `ValidationError`s mapped to inline tips and doc links.

**In scope** (this spec owns these source paths in full):

| Path | Coverage |
|---|---|
| `src/schemas/hooks.ts` | read fully (222 lines) — Zod schemas for all four hook types |
| `src/migrations/*.ts` (11 files) | read fully (557 lines combined) — every migration verbatim |
| `src/utils/settings/types.ts` | read fully (1148 lines) — `SettingsSchema`, `PermissionsSchema`, MCP entry schemas, marketplace entry schema |
| `src/utils/settings/settings.ts` | read fully (1015 lines) — load/parse/merge/cache/update pipeline |
| `src/utils/settings/constants.ts` | read fully — `SETTING_SOURCES` and display names |
| `src/utils/settings/managedPath.ts` | read fully — OS-specific managed paths |
| `src/utils/settings/permissionValidation.ts` | read fully — `PermissionRuleSchema`, parens/escape validation |
| `src/utils/settings/validation.ts` | read fully — `formatZodError`, `filterInvalidPermissionRules` |
| `src/utils/settings/validationTips.ts` | read fully — TIP_MATCHERS table |
| `src/utils/settings/toolValidationConfig.ts` | read fully — file-pattern/bash-prefix/custom validators (WebSearch, WebFetch) |
| `src/utils/settings/changeDetector.ts` | read fully — chokidar watcher, MDM poll, deletion grace, internal-write suppression |
| `src/utils/settings/settingsCache.ts` | read fully — three caches |
| `src/utils/settings/internalWrites.ts` | read fully — echo suppression timestamp map |
| `src/utils/settings/applySettingsChange.ts` | read fully — bridge from change detector → AppState |
| `src/utils/settings/allErrors.ts` | read fully — settings + MCP error fold |
| `src/utils/settings/pluginOnlyPolicy.ts` | read fully — `strictPluginOnlyCustomization` runtime checks |
| `src/utils/settings/schemaOutput.ts` | read fully — `toJSONSchema(SettingsSchema())` for FileEditTool error UI |
| `src/utils/settings/validateEditTool.ts` | read fully — gate that blocks invalid edits to settings files |
| `src/utils/settings/mdm/constants.ts` | read fully — registry paths, plist domain, plutil constants |
| `src/utils/settings/mdm/rawRead.ts` | read fully — startup-fired plutil/reg subprocesses |
| `src/utils/settings/mdm/settings.ts` | read fully — first-source-wins parse + cache |

**Out of scope (cite peer spec):**

- Boot-time wiring that calls `startMdmRawRead()`/`startKeychainPrefetch()` and the broader entrypoint plumbing → **01**.
- Permission rule runtime evaluation and rule cascading at decision time → **09** (this spec owns the disk schema that defines `permissions.allow`/`deny`/`ask`/`defaultMode`).
- MCP server configuration and `.mcp.json` parsing → **23** (this spec owns the on-disk fields `enabledMcpjsonServers` / `disabledMcpjsonServers` / `enableAllProjectMcpServers` / `allowedMcpServers` / `deniedMcpServers` and the MCP-related migration).
- Hook execution semantics (matcher, dispatch, async, asyncRewake, blocking) → **09** (this spec owns the four `HookCommand` Zod variants and the `HooksSchema` shape).
- Plugin manifest schema (`plugin.json`, marketplace manifest) → **28** (this spec owns the **settings-side** `enabledPlugins`, `extraKnownMarketplaces`, `strictKnownMarketplaces`, `blockedMarketplaces`, `pluginConfigs`, and the `MarketplaceSourceSchema` reference).
- Sandbox runtime semantics → 10/19; this spec owns only the `sandbox` field as `SandboxSettingsSchema().optional()` reference.
- Remote managed-settings sync (server pull) → **27**; we read from its in-memory cache (`getRemoteManagedSettingsSyncFromCache`).
- `GlobalConfig` / `~/.claude.json` field semantics → **41**; this spec describes only the **migration boundary** (which keys move from `GlobalConfig` to `SettingsJson`).
- AppState wiring (`useSettingsChange`, `setAppState`) → **41** / **37**; we provide `applySettingsChange(source, setAppState)` as the bridge.
- Output styles, vim, voice, modes, etc. — they consume named settings keys; the cross-spec contract table in §3.4 enumerates which key flows where.

---

## 2. Source Map

### 2.1 Source coverage table

| Path | Lines | Coverage | Owner notes |
|---|---|---|---|
| `src/schemas/hooks.ts` | 222 | read fully | Hook Zod schemas (broken out to avoid plugins ↔ settings cycle). Imports `HOOK_EVENTS` from `entrypoints/agentSdkTypes.js`, `SHELL_TYPES` from `utils/shell/shellProvider.js`. |
| `src/migrations/migrateAutoUpdatesToSettings.ts` | 61 | read fully | Moves `autoUpdates=false` from `~/.claude.json` to `userSettings.env.DISABLE_AUTOUPDATER='1'`. |
| `src/migrations/migrateBypassPermissionsAcceptedToSettings.ts` | 40 | read fully | `bypassPermissionsModeAccepted` (GlobalConfig) → `skipDangerousModePermissionPrompt` (userSettings). |
| `src/migrations/migrateEnableAllProjectMcpServersToSettings.ts` | 118 | read fully | Moves three MCP-approval fields from `currentProjectConfig` (in `~/.claude.json`) to `localSettings`. |
| `src/migrations/migrateFennecToOpus.ts` | 45 | read fully | ANT-only model alias rewrite. |
| `src/migrations/migrateLegacyOpusToCurrent.ts` | 57 | read fully | First-party only; explicit Opus 4.0/4.1 ID → `'opus'`. |
| `src/migrations/migrateOpusToOpus1m.ts` | 43 | read fully | Eligible (Max/Team Premium, 1P) → upgrade `'opus'` → `'opus[1m]'`. |
| `src/migrations/migrateReplBridgeEnabledToRemoteControlAtStartup.ts` | 22 | read fully | GlobalConfig key rename. |
| `src/migrations/migrateSonnet1mToSonnet45.ts` | 48 | read fully | `'sonnet[1m]'` → `'sonnet-4-5-20250929[1m]'`. |
| `src/migrations/migrateSonnet45ToSonnet46.ts` | 67 | read fully | Pro/Max/Team Premium, 1P; explicit Sonnet 4.5 IDs → `'sonnet'`. |
| `src/migrations/resetAutoModeOptInForDefaultOffer.ts` | 51 | read fully | Behind `feature('TRANSCRIPT_CLASSIFIER')`; one-shot re-arm of opt-in dialog. |
| `src/migrations/resetProToOpusDefault.ts` | 51 | read fully | Pro 1P first-time Opus default notification. |
| `src/utils/settings/types.ts` | 1148 | read fully | `SettingsSchema`, `PermissionsSchema`, marketplace and MCP entry schemas. |
| `src/utils/settings/settings.ts` | 1015 | read fully | Pipeline. |
| `src/utils/settings/constants.ts` | 202 | read fully | `SETTING_SOURCES` order. |
| `src/utils/settings/managedPath.ts` | 34 | read fully | Per-OS managed root + drop-in dir. |
| `src/utils/settings/permissionValidation.ts` | 262 | read fully | `PermissionRuleSchema` and validators. |
| `src/utils/settings/validation.ts` | 265 | read fully | Error formatter, MCP-meta-aware. |
| `src/utils/settings/validationTips.ts` | 164 | read fully | Inline tip table. |
| `src/utils/settings/toolValidationConfig.ts` | 103 | read fully | Tool-specific permission-rule validators. |
| `src/utils/settings/changeDetector.ts` | 488 | read fully | Watcher pipeline. |
| `src/utils/settings/settingsCache.ts` | 80 | read fully | Three caches + plugin base layer. |
| `src/utils/settings/internalWrites.ts` | 37 | read fully | Echo suppression. |
| `src/utils/settings/applySettingsChange.ts` | 92 | read fully | Bridge to AppState. |
| `src/utils/settings/allErrors.ts` | 32 | read fully | settings+MCP error fold. |
| `src/utils/settings/pluginOnlyPolicy.ts` | 60 | read fully | Surface lock checks. |
| `src/utils/settings/schemaOutput.ts` | 8 | read fully | `toJSONSchema()` adapter. |
| `src/utils/settings/validateEditTool.ts` | 45 | read fully | Pre/post Edit-tool guard. |
| `src/utils/settings/mdm/constants.ts` | 81 | read fully | Plist domain, registry keys. |
| `src/utils/settings/mdm/rawRead.ts` | 130 | read fully | Subprocess fan-out. |
| `src/utils/settings/mdm/settings.ts` | 316 | read fully | Cache + first-source-wins. |

**Total**: 5,008 lines fully read across 30 files. Three trivial peer files (`hooks.ts` schema, `permissionValidation.ts`, etc.) are repeated above for completeness.

### 2.2 Feature-flag and ANT guards in scope

Owned-source feature/ANT gates (every guard in this subsystem):

| File:line | Guard | Effect |
|---|---|---|
| `src/utils/settings/types.ts:61-64` | `feature('TRANSCRIPT_CLASSIFIER')` | `permissions.defaultMode` enum widens from `EXTERNAL_PERMISSION_MODES` (5 values) to `PERMISSION_MODES` (adds `'auto'`). |
| `src/utils/settings/types.ts:71-78` | `feature('TRANSCRIPT_CLASSIFIER')` | Adds `permissions.disableAutoMode: 'disable'`. |
| `src/utils/settings/types.ts:284-310` | `isEnvTruthy(process.env.CLAUDE_CODE_ENABLE_XAA)` | Adds `xaaIdp` block (issuer, clientId, callbackPort). |
| `src/utils/settings/types.ts:703-711` | `process.env.USER_TYPE === 'ant'` (inline ternary at `:705`) | `effortLevel` enum is `['low','medium','high','max']` for ant, `['low','medium','high']` otherwise. |
| `src/utils/settings/types.ts:808-817` | `feature('LODESTONE')` | Adds `disableDeepLinkRegistration: 'disable'`. |
| `src/utils/settings/types.ts:831-840` | `process.env.USER_TYPE === 'ant'` | Adds `classifierPermissionsEnabled: boolean`. |
| `src/utils/settings/types.ts:841-863` | `feature('PROACTIVE') \|\| feature('KAIROS')` | Adds `minSleepDurationMs`, `maxSleepDurationMs`. |
| `src/utils/settings/types.ts:864-871` | `feature('VOICE_MODE')` | Adds `voiceEnabled: boolean`. |
| `src/utils/settings/types.ts:872-887` | `feature('KAIROS')` | Adds `assistant`, `assistantName`. |
| `src/utils/settings/types.ts:922-931` | `feature('KAIROS') \|\| feature('KAIROS_BRIEF')` | Adds `defaultView: 'chat' \| 'transcript'`. |
| `src/utils/settings/types.ts:968-1008` | `feature('TRANSCRIPT_CLASSIFIER')` | Adds `skipAutoPermissionPrompt`, `useAutoModeDuringPlan`, `autoMode { allow, soft_deny, environment, deny? (ant only at 992-997) }`. |
| `src/utils/settings/types.ts:992-997` | `process.env.USER_TYPE === 'ant'` (nested) | Adds `autoMode.deny` (ant back-compat alias for `soft_deny`). |
| `src/schemas/hooks.ts` | (no flags) | Hook schemas are flag-free. |
| `src/utils/settings/settings.ts:577` | `feature('TRANSCRIPT_CLASSIFIER')` | Includes `'disableAutoMode'` in valid-nested-keys for managed-settings logging. |
| `src/utils/settings/settings.ts:897` / `:919` / `:939` | `feature('TRANSCRIPT_CLASSIFIER')` | Gate `hasAutoModeOptIn`, `getUseAutoModeDuringPlan`, `getAutoModeConfig` to no-op outside the flag. |
| `src/utils/settings/settings.ts:965` | `process.env.USER_TYPE === 'ant'` | Inside `getAutoModeConfig`, fold `autoMode.deny` into `soft_deny` for ant. |
| `src/utils/settings/settings.ts:51-54` | `process.env.USER_TYPE === 'ant'` AND `process.env.CLAUDE_CODE_ENTRYPOINT !== 'local-agent'` | In `applySettingsChange.ts`, re-strip overly broad Bash allow rules after every settings reload. |
| `src/utils/settings/managedPath.ts:10-15` | `process.env.USER_TYPE === 'ant' && process.env.CLAUDE_CODE_MANAGED_SETTINGS_PATH` | ANT-only override of managed-settings root for testing/demos. |
| `src/utils/settings/mdm/constants.ts:67-79` | `process.env.USER_TYPE === 'ant'` | Adds **user-writable** `~/Library/Preferences/com.anthropic.claudecode.plist` to plist priority list (lowest position). |
| `src/migrations/migrateFennecToOpus.ts:19-21` | `process.env.USER_TYPE === 'ant'` (runtime gate) | Whole migration is ANT-only at runtime. **Dual-gated**: also wrapped in `if ("external" === 'ant')` at the call site (`main.tsx:340`), which is a **build-time DCE marker** (Bun's `bun:bundle` evaluates the literal-string compare and strips the call entirely from external builds). Reimplementer hazard: keeping only the inner runtime gate (without the outer build-time gate) leaks an `import` of the migration module into the external bundle, which is the opposite of what the leaked `"external"` string in `main.tsx` is meant to elide. See §5.10 entry 11 and §10.3 (build-time DCE pattern). |
| `src/migrations/migrateLegacyOpusToCurrent.ts:30` | `getAPIProvider() === 'firstParty'` + `isLegacyModelRemapEnabled()` | First-party only. |
| `src/migrations/migrateSonnet45ToSonnet46.ts:30-35` | `getAPIProvider() === 'firstParty'` AND (`isProSubscriber() \|\| isMaxSubscriber() \|\| isTeamPremiumSubscriber()`) | 1P billing-tier gated. |
| `src/migrations/migrateOpusToOpus1m.ts:25` | `isOpus1mMergeEnabled()` | Eligibility helper (1P + Max/Team Premium per docstring). |
| `src/migrations/resetAutoModeOptInForDefaultOffer.ts:26` | `feature('TRANSCRIPT_CLASSIFIER')` | Whole migration. |
| `src/migrations/resetProToOpusDefault.ts:17-23` | `getAPIProvider() === 'firstParty'` + `isProSubscriber()` | Pro 1P only. |

### 2.3 Imports from (upstream)

- `zod/v4` (`z`) — schemas.
- `bun:bundle` (`feature`) — DCE-evaluated flags.
- `lodash-es/mergeWith` (settings cascade), `lodash-es/memoize` (managed path).
- `chokidar` (file watching).
- `path`, `fs`, `child_process` — managed file paths and MDM subprocesses.
- `src/entrypoints/agentSdkTypes.js` (`HOOK_EVENTS`, `HookEvent`) — hook event enum.
- `src/entrypoints/sandboxTypes.js` (`SandboxSettingsSchema`) — sandbox sub-schema.
- `src/utils/plugins/schemas.js` (`MarketplaceSourceSchema`) — marketplace source sub-schema (owned by 28).
- `src/utils/permissions/permissionRuleParser.js` (`permissionRuleValueFromString`).
- `src/utils/permissions/PermissionMode.js` re-exports `EXTERNAL_PERMISSION_MODES`, `PERMISSION_MODES` from `src/types/permissions.ts`.
- `src/utils/permissions/permissionSetup.js` (`findOverlyBroadBashPermissions`, `removeDangerousPermissions`, `transitionPlanAutoMode`, `isBypassPermissionsModeDisabled`, `createDisabledBypassPermissionsContext`, `getAutoModeEnabledState`).
- `src/utils/permissions/permissions.js` (`syncPermissionRulesFromDisk`).
- `src/utils/permissions/permissionsLoader.js` (`loadAllPermissionRulesFromDisk`).
- `src/utils/lazySchema.js` — defers schema construction (avoids module-init cycles).
- `src/utils/shell/shellProvider.js` (`SHELL_TYPES = ['bash','powershell']`).
- `src/utils/config.js` (`getGlobalConfig`, `saveGlobalConfig`, `getCurrentProjectConfig`, `saveCurrentProjectConfig`) — `~/.claude.json`-side state for migrations.
- `src/utils/auth.js` (`isProSubscriber`, `isMaxSubscriber`, `isTeamPremiumSubscriber`).
- `src/utils/model/model.js` (`isLegacyModelRemapEnabled`, `isOpus1mMergeEnabled`, `getDefaultMainLoopModelSetting`, `parseUserSpecifiedModel`).
- `src/utils/model/providers.js` (`getAPIProvider`).
- `src/bootstrap/state.js` (`getMainLoopModelOverride`, `setMainLoopModelOverride`, `getFlagSettingsInline`, `getFlagSettingsPath`, `getOriginalCwd`, `getUseCoworkPlugins`, `getAllowedSettingSources`, `getIsRemoteMode`).
- `src/services/remoteManagedSettings/syncCacheState.js` (`getRemoteManagedSettingsSyncFromCache`) — owned by 27.
- `src/services/analytics/index.js` (`logEvent`) — migrations emit `tengu_*` events.
- `src/utils/hooks.js` (`executeConfigChangeHooks`, `hasBlockingResult`) — owned by 09.
- `src/utils/hooks/hooksConfigSnapshot.js` (`updateHooksConfigSnapshot`).
- `src/utils/cleanupRegistry.js` (`registerCleanup`).
- `src/utils/signal.js` (`createSignal`).

### 2.4 Imported by (downstream)

Major consumers (selected; see §3.4 for the full key→consumer table):

- `src/main.tsx` — calls 11 migrations in fixed order (lines 328-341), reads `getInitialSettings()` for boot-time decisions.
- `src/state/AppState.tsx` — holds `settings: SettingsJson` and re-renders on `applySettingsChange`.
- `src/utils/permissions/*` — entire permission rules subsystem.
- `src/services/mcp/*` — `enabledMcpjsonServers` etc.
- `src/services/plugins/*` — `enabledPlugins`, `extraKnownMarketplaces`, `pluginConfigs`.
- `src/skills/*` — `pluginConfigs.options` mediates skill config (per spec 17 surface).
- Every command in `src/commands/` that reads or writes settings.

### 2.5 Source files referenced but absent — N/A

No registry-referenced files within this spec's owned scope are absent. (The boot-side `mdmRawRead.startMdmRawRead()` is invoked by `main.tsx`; that file is bundled.)

---

## 3. Public Interface (Contract)

### 3.1 Read APIs

```ts
// src/utils/settings/settings.ts
export function getSettingsForSource(source: SettingSource): SettingsJson | null
export function getSettingsFilePathForSource(source: SettingSource): string | undefined
export function getRelativeSettingsFilePathForSource(source: 'projectSettings' | 'localSettings'): string
export function getSettingsRootPathForSource(source: SettingSource): string
export function getInitialSettings(): SettingsJson
export const getSettings_DEPRECATED: typeof getInitialSettings  // alias
export function getSettingsWithErrors(): SettingsWithErrors
export function getSettingsWithSources(): SettingsWithSources
export function rawSettingsContainsKey(key: string): boolean
export function hasSkipDangerousModePermissionPrompt(): boolean
export function hasAutoModeOptIn(): boolean
export function getUseAutoModeDuringPlan(): boolean
export function getAutoModeConfig(): { allow?: string[]; soft_deny?: string[]; environment?: string[] } | undefined
export function getPolicySettingsOrigin(): 'remote' | 'plist' | 'hklm' | 'file' | 'hkcu' | null
export function loadManagedFileSettings(): { settings: SettingsJson | null; errors: ValidationError[] }
export function getManagedFileSettingsPresence(): { hasBase: boolean; hasDropIns: boolean }
export function getManagedSettingsKeysForLogging(settings: SettingsJson): string[]
export function parseSettingsFile(path: string): { settings: SettingsJson | null; errors: ValidationError[] }
export function settingsMergeCustomizer(objValue: unknown, srcValue: unknown): unknown
```

### 3.2 Write API

```ts
// src/utils/settings/settings.ts
export function updateSettingsForSource(
  source: EditableSettingSource,
  settings: SettingsJson,
): { error: Error | null }
```

`EditableSettingSource = Exclude<SettingSource, 'policySettings' | 'flagSettings'>` (`constants.ts:182-185`). Writes to read-only sources are silently no-ops returning `{ error: null }` (`settings.ts:420-425`).

### 3.3 Cache and watcher APIs

```ts
// src/utils/settings/settingsCache.ts
export function getSessionSettingsCache(): SettingsWithErrors | null
export function setSessionSettingsCache(value: SettingsWithErrors): void
export function getCachedSettingsForSource(source): SettingsJson | null | undefined
export function setCachedSettingsForSource(source, value): void
export function getCachedParsedFile(path): ParsedSettings | undefined
export function setCachedParsedFile(path, value): void
export function resetSettingsCache(): void
export function getPluginSettingsBase(): Record<string, unknown> | undefined
export function setPluginSettingsBase(settings): void
export function clearPluginSettingsBase(): void

// src/utils/settings/internalWrites.ts
export function markInternalWrite(path: string): void
export function consumeInternalWrite(path: string, windowMs: number): boolean
export function clearInternalWrites(): void

// src/utils/settings/changeDetector.ts
export const settingsChangeDetector = { initialize, dispose, subscribe, notifyChange, resetForTesting }
export function notifyChange(source: SettingSource): void

// src/utils/settings/applySettingsChange.ts
export function applySettingsChange(source: SettingSource, setAppState): void

// src/utils/settings/allErrors.ts
export function getSettingsWithAllErrors(): SettingsWithErrors  // settings + MCP errors
```

### 3.4 Cross-spec contracts (downstream consumers — settings key → consumer spec)

This is the contract surface for specs 22-36. Every key in `SettingsSchema` is listed; the consumer column is the spec that uses the key at runtime.

> **Default-column convention (reimplementer hazard).** Many entries below show a numeric or boolean "default" sourced from the schema's `.describe()` text (e.g. `"default: 30"` for `cleanupPeriodDays`). These are **describe-only defaults** — they are documentation strings, **not** Zod `.default(...)` calls. Zod parses these fields as `.optional()`, so the parsed value is `undefined` until a consumer applies a `??` fallback at the read site. Affected keys (verified `.describe()`-only, no Zod `.default`): `cleanupPeriodDays`, `respectGitignore`, `defaultShell`, `terminalTitleFromRename`, `spinnerTipsEnabled`, `alwaysThinkingEnabled`, `promptSuggestionEnabled`, `useAutoModeDuringPlan` (this last is also subject to the special "trusted source explicit `false`" rule in §12.12 — its computed value is **not** `settings.useAutoModeDuringPlan ?? true`). Consequence for reimplementers: **do not** translate "(per .describe)" entries below into Zod `.default(...)` calls — that would change merge-cascade semantics, because a Zod-default flips an `undefined` source into a real value that participates in `mergeWith` and can shadow a higher-priority source. The consumer-side `??` fallback is intentional: it preserves "field absent" through the cascade and only applies the literal default at final read.

| Settings key (path) | Type | Default | Consumer spec |
|---|---|---|---|
| `$schema` | literal URL | absent | (validation only) |
| `apiKeyHelper` | string | absent | 22, 25 |
| `awsCredentialExport` | string | absent | 22 |
| `awsAuthRefresh` | string | absent | 22 |
| `gcpAuthRefresh` | string | absent | 22 |
| `xaaIdp.{issuer,clientId,callbackPort}` | object | absent | 25 (XAA OIDC; SEP-990) |
| `fileSuggestion.{type,command}` | object | absent | 37 |
| `respectGitignore` | boolean | true (per .describe) | 11, 12 |
| `cleanupPeriodDays` | int ≥0 | 30 (per .describe; 0 disables) | 41 |
| `env` | record<string, coerced string> | absent | 01 (env merging at boot), all subsystems |
| `attribution.{commit,pr}` | object | absent | 10 (gitOperationTracking) |
| `includeCoAuthoredBy` | boolean | true | 10 (deprecated; superseded by `attribution`) |
| `includeGitInstructions` | boolean | true | 05 (system prompt) |
| `permissions.{allow,deny,ask,defaultMode,disableBypassPermissionsMode,disableAutoMode,additionalDirectories}` | object | absent | 09 |
| `model` | string | absent | 22 (model resolution), migrations rewrite this |
| `availableModels` | string[] | absent (all allowed) | 22, 27 |
| `modelOverrides` | record<string,string> | absent | 22 (Bedrock inference profiles) |
| `enableAllProjectMcpServers` | boolean | absent | 23 |
| `enabledMcpjsonServers` | string[] | absent | 23 |
| `disabledMcpjsonServers` | string[] | absent | 23 |
| `allowedMcpServers` | `AllowedMcpServerEntry[]` | absent (all allowed) | 23, 27 |
| `deniedMcpServers` | `DeniedMcpServerEntry[]` | absent | 23, 27 |
| `hooks` | `HooksSchema` | absent | 09 |
| `worktree.{symlinkDirectories,sparsePaths}` | object | absent | 18 |
| `disableAllHooks` | boolean | absent | 09 |
| `defaultShell` | `'bash' \| 'powershell'` | `'bash'` (incl. Windows; per .describe) | 10, 39 |
| `allowManagedHooksOnly` | boolean | absent | 09, 27 |
| `allowedHttpHookUrls` | string[] | absent | 09 (HTTP hook gating) |
| `httpHookAllowedEnvVars` | string[] | absent | 09 (HTTP hook env-var interpolation) |
| `allowManagedPermissionRulesOnly` | boolean | absent | 09, 27 |
| `allowManagedMcpServersOnly` | boolean | absent | 23, 27 |
| `strictPluginOnlyCustomization` | bool \| array of CUSTOMIZATION_SURFACES | absent | 28, 17, 09 (`pluginOnlyPolicy.ts`) |
| `statusLine.{type,command,padding}` | object | absent | 37 |
| `enabledPlugins` | record<string, string[] \| boolean \| undefined> | absent | 28 |
| `extraKnownMarketplaces` | record | absent | 28 |
| `strictKnownMarketplaces` | `MarketplaceSource[]` | absent | 28, 27 |
| `blockedMarketplaces` | `MarketplaceSource[]` | absent | 28, 27 |
| `forceLoginMethod` | `'claudeai' \| 'console'` | absent | 25 |
| `forceLoginOrgUUID` | string | absent | 25 |
| `otelHeadersHelper` | string | absent | 26 |
| `outputStyle` | string | absent | 38 |
| `language` | string | absent | 05, 36 |
| `skipWebFetchPreflight` | boolean | absent | 13 |
| `sandbox` | `SandboxSettingsSchema` | absent | 10, 19 |
| `feedbackSurveyRate` | 0..1 | absent | 26 |
| `spinnerTipsEnabled` | boolean | true | 37 |
| `spinnerVerbs.{mode,verbs}` | object | absent | 37 |
| `spinnerTipsOverride.{excludeDefault,tips}` | object | absent | 37 |
| `syntaxHighlightingDisabled` | boolean | false | 37 |
| `terminalTitleFromRename` | boolean | true | 37, 21 (`/rename`) |
| `alwaysThinkingEnabled` | boolean | true (when omitted) | 03 |
| `effortLevel` | enum (low/med/high(/max ant)) | absent | 03, 41 |
| `advisorModel` | string | absent | 22 (advisor.ts) |
| `fastMode` | boolean | absent | 03 |
| `fastModePerSessionOptIn` | boolean | absent | 03, 41 |
| `promptSuggestionEnabled` | boolean | true | 38 |
| `showClearContextOnPlanAccept` | boolean | false | 18, 37 |
| `agent` | string | absent | 14 (main-thread agent override) |
| `companyAnnouncements` | string[] | absent | 37 |
| `pluginConfigs.<id>.{mcpServers,options}` | record | absent | 28, 23 |
| `remote.defaultEnvironmentId` | string | absent | 35 |
| `autoUpdatesChannel` | `'latest' \| 'stable'` | `'latest'` | 01 (auto-updater) |
| `disableDeepLinkRegistration` | `'disable'` | absent | 01 (LODESTONE) |
| `minimumVersion` | string | absent | 01 (auto-updater downgrade guard) |
| `plansDirectory` | string | `~/.claude/plans/` | 18, 40 |
| `classifierPermissionsEnabled` | boolean | absent | 09 (ANT) |
| `minSleepDurationMs` / `maxSleepDurationMs` | number | absent | 19 (Sleep tool, PROACTIVE/KAIROS) |
| `voiceEnabled` | boolean | absent | 36 |
| `assistant`, `assistantName` | various | absent | 32 (KAIROS) |
| `defaultView` | `'chat' \| 'transcript'` | absent | 32 (KAIROS / KAIROS_BRIEF) |
| `channelsEnabled` | boolean | false | 23 (channel-capable MCP) |
| `allowedChannelPlugins` | array | absent | 23, 28 |
| `prefersReducedMotion` | boolean | false | 37 |
| `autoMemoryEnabled` | boolean | absent (server default) | 29 |
| `autoMemoryDirectory` | string | derived from cwd | 29 (note: ignored from `projectSettings`) |
| `autoDreamEnabled` | boolean | absent | 29 |
| `showThinkingSummaries` | boolean | false | 37 |
| `skipDangerousModePermissionPrompt` | boolean | absent | 09 |
| `skipAutoPermissionPrompt` | boolean | absent | 09 (TRANSCRIPT_CLASSIFIER) |
| `useAutoModeDuringPlan` | boolean | true | 09, 18 (TRANSCRIPT_CLASSIFIER) |
| `autoMode.{allow,soft_deny,environment,deny?}` | object | absent | 09 (TRANSCRIPT_CLASSIFIER) |
| `disableAutoMode` | `'disable'` | absent | 09 |
| `sshConfigs` | array | absent | 35 |
| `claudeMdExcludes` | string[] | absent | 05, 40 (CLAUDE.md loader) |
| `pluginTrustMessage` | string | absent | 28 (policy-only) |

### 3.5 Public types

```ts
// src/utils/settings/types.ts
export const CUSTOMIZATION_SURFACES = ['skills', 'agents', 'hooks', 'mcp'] as const
export type SettingsJson = z.infer<ReturnType<typeof SettingsSchema>>
export type AllowedMcpServerEntry = z.infer<ReturnType<typeof AllowedMcpServerEntrySchema>>
export type DeniedMcpServerEntry = z.infer<ReturnType<typeof DeniedMcpServerEntrySchema>>
export type UserConfigValues = Record<string, string | number | boolean | string[]>
export type PluginConfig = { mcpServers?: { [serverName: string]: UserConfigValues } }
export type PluginHookMatcher = { matcher?: string; hooks: HookCommand[]; pluginRoot: string; pluginName: string; pluginId: string }
export type SkillHookMatcher = { matcher?: string; hooks: HookCommand[]; skillRoot: string; skillName: string }

// src/utils/settings/constants.ts
export const SETTING_SOURCES = ['userSettings', 'projectSettings', 'localSettings', 'flagSettings', 'policySettings'] as const
export type SettingSource = (typeof SETTING_SOURCES)[number]
export type EditableSettingSource = Exclude<SettingSource, 'policySettings' | 'flagSettings'>
export const SOURCES = ['localSettings', 'projectSettings', 'userSettings'] as const
export const CLAUDE_CODE_SETTINGS_SCHEMA_URL = 'https://json.schemastore.org/claude-code-settings.json'

// src/schemas/hooks.ts
export type HookCommand = z.infer<ReturnType<typeof HookCommandSchema>>
export type BashCommandHook = Extract<HookCommand, { type: 'command' }>
export type PromptHook = Extract<HookCommand, { type: 'prompt' }>
export type AgentHook = Extract<HookCommand, { type: 'agent' }>
export type HttpHook = Extract<HookCommand, { type: 'http' }>
export type HookMatcher = z.infer<ReturnType<typeof HookMatcherSchema>>
export type HooksSettings = Partial<Record<HookEvent, HookMatcher[]>>

// src/utils/settings/validation.ts
export type FieldPath = string
export type ValidationError = { file?: string; path: FieldPath; message: string; expected?: string; invalidValue?: unknown; suggestion?: string; docLink?: string; mcpErrorMetadata?: { scope: ConfigScope; serverName?: string; severity?: 'fatal' | 'warning' } }
export type SettingsWithErrors = { settings: SettingsJson; errors: ValidationError[] }
```

---

## 4. Data Model & State

### 4.1 Five settings sources (priority and disk paths)

`SETTING_SOURCES` order in `constants.ts:7-22` — comment **"Order matters - later sources override earlier ones"**:

```
userSettings → projectSettings → localSettings → flagSettings → policySettings
   (lowest)                                                       (highest)
```

A separate `pluginSettingsBase` (in-memory layer written by the plugin loader, `settingsCache.ts:66-80`) merges **below** all five — see §5.1.

> **Verification of the overview's claim**: `00-overview.md §5.1` lists boot precedence as `env > project > user > MDM > defaults`. The actual chain in source is **NOT** that. Verified order from `loadSettingsFromDisk` (`settings.ts:670-784`):
>
> 1. `pluginSettingsBase` (lowest base)
> 2. Iterate `getEnabledSettingSources()` (preserves declaration order in `SETTING_SOURCES`)
> 3. For `policySettings`, "first source wins" picks one of: remote → MDM (HKLM/plist) → `managed-settings.json` (+ drop-ins) → HKCU. (`settings.ts:677-739`)
> 4. For `flagSettings`, the file is merged then any inline SDK settings are merged on top (`settings.ts:771-783`).
>
> **There is no env-var settings source** — `process.env.CLAUDE_CODE_*` runtime gates exist but are not part of the cascade. Only `settings.env` (a Zod field) is propagated as env vars by the bootstrap layer (01).
>
> **There is no "defaults" source** — defaults are encoded in either `.describe()` text or per-consumer `?? defaultValue` fallbacks at the call site.
>
> **MDM is not a separate source** in the cascade — it is one of four alternatives within `policySettings` (the **highest** priority, not below user).

#### 4.1.1 Disk paths per source

```
userSettings:    join(getClaudeConfigHomeDir(), getUserSettingsFilePath())
                   getUserSettingsFilePath() returns 'cowork_settings.json' if
                   getUseCoworkPlugins() OR isEnvTruthy(process.env.CLAUDE_CODE_USE_COWORK_PLUGINS),
                   else 'settings.json'.   (settings.ts:264-272)

projectSettings: join(originalCwd, '.claude/settings.json')                   (settings.ts:298-306)
localSettings:   join(originalCwd, '.claude/settings.local.json')             (settings.ts:298-306)
flagSettings:    getFlagSettingsPath() (from --settings CLI flag)             (settings.ts:292-294)
policySettings:  see §4.1.2.
```

`getClaudeConfigHomeDir()` is owned by 01 (env util) and resolves to `~/.claude` by default.

#### 4.1.2 `policySettings` four-way priority (first source wins)

`getPolicySettingsOrigin` and `getSettingsForSourceUncached` (`settings.ts:323-345`, `:375-407`):

1. **Remote** (highest): `getRemoteManagedSettingsSyncFromCache()` from spec 27.
2. **Admin-only MDM**:
   - macOS: `/Library/Managed Preferences/<username>/com.anthropic.claudecode.plist` then `/Library/Managed Preferences/com.anthropic.claudecode.plist` (`mdm/constants.ts:55-65`). ANT-only adds `~/Library/Preferences/com.anthropic.claudecode.plist` at lowest position (`mdm/constants.ts:67-79`).
   - Windows: `HKLM\SOFTWARE\Policies\ClaudeCode\Settings` (REG_SZ or REG_EXPAND_SZ).
3. **File-based**: `<managedFilePath>/managed-settings.json` merged first (base), then `<managedFilePath>/managed-settings.d/*.json` sorted alphabetically and merged on top — **systemd/sudoers drop-in convention** (`settings.ts:62-121`). `mergeWith` with `settingsMergeCustomizer`.
4. **HKCU** (Windows lowest): `HKCU\SOFTWARE\Policies\ClaudeCode\Settings`. Skipped if `hasManagedSettingsFile()` returns true (`mdm/settings.ts:255-258`).

`<managedFilePath>` per OS (`managedPath.ts:8-25`, **memoized**):
- macOS: `/Library/Application Support/ClaudeCode`
- Windows: `C:\Program Files\ClaudeCode`
- Linux/other: `/etc/claude-code`
- ANT-only override: `process.env.CLAUDE_CODE_MANAGED_SETTINGS_PATH` (`managedPath.ts:10-15`).

#### 4.1.3 Display names

`constants.ts:46-121` provides three display name maps. Verbatim:

```
SettingSource         | display name (lowercase)        | display (capitalized)            | short
userSettings          | user                            | user settings → User settings    | User
projectSettings       | project                         | shared project settings          | Project
localSettings         | project, gitignored             | project local settings           | Local
flagSettings          | cli flag                        | command line arguments           | Flag
policySettings        | managed                         | enterprise managed settings      | Managed
```

Plus extra source labels: `cliArg → 'CLI argument'`, `command → 'command configuration'`, `session → 'current session'`. `getSourceDisplayName` adds `'plugin' → 'Plugin'`, `'built-in' → 'Built-in'`.

### 4.2 Cache state

Three caches share invalidation (`settingsCache.ts`):

| Cache | Key | Stored value | Set by | Reset by |
|---|---|---|---|---|
| `sessionSettingsCache` | (singleton) | `SettingsWithErrors` | `getSettingsWithErrors()` | `resetSettingsCache()` |
| `perSourceCache` | `SettingSource` | `SettingsJson \| null` | `getSettingsForSource()` | `resetSettingsCache()` |
| `parseFileCache` | absolute path string | `{settings, errors}` | `parseSettingsFile()` | `resetSettingsCache()` |

A separate `pluginSettingsBase` (mutable singleton) is set/cleared by the plugin loader and is **not** flushed by `resetSettingsCache()`.

The MDM module has its own caches (`mdm/settings.ts:55-57`):
- `mdmCache: MdmResult | null`
- `hkcuCache: MdmResult | null`
- `mdmLoadPromise: Promise<void> | null`

### 4.3 Watcher state

`changeDetector.ts`:
- `watcher: FSWatcher | null` (chokidar).
- `mdmPollTimer: setInterval | null`.
- `lastMdmSnapshot: string | null` — `JSON.stringify({mdm, hkcu})` snapshot.
- `pendingDeletions: Map<path, Timeout>` — delete-and-recreate grace.
- `settingsChanged: Signal<[SettingSource]>`.
- `initialized`, `disposed` flags.

Internal-write echo suppression (`internalWrites.ts`): `Map<path, number>` of write timestamps. `consumeInternalWrite(path, windowMs=5000)` returns true and **deletes** the entry if it matches within window.

### 4.4 GlobalConfig (cross-spec) keys mutated by migrations

Migrations write to `~/.claude.json` via `saveGlobalConfig(updater)`. Keys touched (cite 41 for the full schema):

| Key | Set by | Read by |
|---|---|---|
| `autoUpdates`, `autoUpdatesProtectedForNative` | (deleted by) `migrateAutoUpdatesToSettings` | 01 |
| `bypassPermissionsModeAccepted` | (deleted by) `migrateBypassPermissionsAcceptedToSettings` | 09 (legacy) |
| `replBridgeEnabled` (untyped legacy) | (deleted by) `migrateReplBridgeEnabledToRemoteControlAtStartup` | 34 |
| `remoteControlAtStartup` | written by same | 34, 35 |
| `legacyOpusMigrationTimestamp` | `migrateLegacyOpusToCurrent` | 37 (one-time notification) |
| `sonnet1m45MigrationComplete` | `migrateSonnet1mToSonnet45` | itself (idempotency flag) |
| `sonnet45To46MigrationTimestamp` | `migrateSonnet45ToSonnet46` (only if `numStartups > 1`) | 37 |
| `opusProMigrationComplete` | `resetProToOpusDefault` | itself |
| `opusProMigrationTimestamp` | same (if user had no custom model) | 37 |
| `hasResetAutoModeOptInForDefaultOffer` | `resetAutoModeOptInForDefaultOffer` | itself |

---

## 5. Algorithm / Control Flow

### 5.1 `loadSettingsFromDisk()` — full pseudocode (`settings.ts:645-796`)

```
function loadSettingsFromDisk():
  if isLoadingSettings: return {settings: {}, errors: []}     # recursion guard
  isLoadingSettings = true
  try:
    profileCheckpoint('loadSettingsFromDisk_start')
    logForDiagnosticsNoPII('info', 'settings_load_started')
    pluginBase = getPluginSettingsBase()
    merged = pluginBase ? mergeWith({}, pluginBase, settingsMergeCustomizer) : {}
    allErrors = []; seenErrors = Set; seenFiles = Set

    for source in getEnabledSettingSources():       # constants.ts:159-167
      if source == 'policySettings':                # FIRST-SOURCE-WINS (NOT merge)
        policySettings = null; policyErrors = []
        # (1) remote
        remote = getRemoteManagedSettingsSyncFromCache()
        if remote && Object.keys(remote).length > 0:
          parsed = SettingsSchema().safeParse(remote)
          if parsed.success: policySettings = parsed.data
          else: policyErrors.push(...formatZodError(parsed.error, 'remote managed settings'))
        # (2) admin-only MDM
        if !policySettings:
          mdm = getMdmSettings()
          if Object.keys(mdm.settings).length > 0: policySettings = mdm.settings
          policyErrors.push(...mdm.errors)
        # (3) managed-settings.json + .d/
        if !policySettings:
          {settings, errors} = loadManagedFileSettings()
          if settings: policySettings = settings
          policyErrors.push(...errors)
        # (4) HKCU
        if !policySettings:
          hkcu = getHkcuSettings()
          if Object.keys(hkcu.settings).length > 0: policySettings = hkcu.settings
          policyErrors.push(...hkcu.errors)

        if policySettings:
          merged = mergeWith(merged, policySettings, settingsMergeCustomizer)
        for e in policyErrors: dedupe-add to allErrors
        continue

      # Normal source: parse file
      filePath = getSettingsFilePathForSource(source)
      if filePath && !seenFiles.has(resolve(filePath)):
        seenFiles.add(resolve(filePath))
        {settings, errors} = parseSettingsFile(filePath)
        for e in errors: dedupe-add
        if settings: merged = mergeWith(merged, settings, settingsMergeCustomizer)

      if source == 'flagSettings':                  # also merge inline SDK settings
        inline = getFlagSettingsInline()
        if inline:
          parsed = SettingsSchema().safeParse(inline)
          if parsed.success:
            merged = mergeWith(merged, parsed.data, settingsMergeCustomizer)

    logForDiagnosticsNoPII('info', 'settings_load_completed', {duration_ms, source_count, error_count})
    return {settings: merged, errors: allErrors}
  finally:
    isLoadingSettings = false
```

`getEnabledSettingSources()` returns `getAllowedSettingSources() ∪ {'policySettings','flagSettings'}` — policy and flag are **always loaded**, regardless of which sources the user opted out of via `--setting-sources` (`constants.ts:159-167`). `parseSettingSourcesFlag(flag)` accepts comma-separated `user,project,local`; throws on unknown name (`constants.ts:128-153`).

### 5.2 `parseSettingsFile()` — single file load (`settings.ts:178-231`)

```
function parseSettingsFile(path):
  cached = getCachedParsedFile(path); if cached: return clone(cached)
  result = parseSettingsFileUncached(path)
  setCachedParsedFile(path, result)
  return clone(result)         # always clone, even on first return; mergeWith will mutate

function parseSettingsFileUncached(path):
  try:
    {resolvedPath} = safeResolvePath(fs, path)
    content = readFileSync(resolvedPath)
    if content.trim() == '': return {settings: {}, errors: []}
    data = safeParseJSON(content, false)
    ruleWarnings = filterInvalidPermissionRules(data, path)   # mutates data.permissions.{allow,deny,ask}
    result = SettingsSchema().safeParse(data)
    if !result.success:
      errors = formatZodError(result.error, path)
      return {settings: null, errors: [...ruleWarnings, ...errors]}
    return {settings: result.data, errors: ruleWarnings}
  catch e:
    if e.code == 'ENOENT': log debug "Broken symlink or missing file..."
    else: logError(e)
    return {settings: null, errors: []}
```

### 5.3 `settingsMergeCustomizer` (`settings.ts:538-547`)

Used by **read** path. `lodash mergeWith` calls this for each value pair:
- If both are arrays: return `uniq([...target, ...source])` — concat + dedupe.
- Otherwise: return `undefined` (let lodash use default merge).

This is **distinct** from the customizer used by `updateSettingsForSource` (write path) at `settings.ts:476-494`:
- If `srcValue === undefined`: `delete object[key]; return undefined` — undefined means delete.
- If `srcValue` is an array: return `srcValue` — arrays **replace**, not merge (the write-side caller is expected to compute the desired final array).
- Otherwise: return `undefined`.

### 5.4 `updateSettingsForSource()` (`settings.ts:416-524`)

```
function updateSettingsForSource(source, settings):
  if source ∈ {'policySettings', 'flagSettings'}: return {error: null}    # silent no-op
  filePath = getSettingsFilePathForSource(source)
  if !filePath: return {error: null}
  try:
    fs.mkdirSync(dirname(filePath))
    existing = getSettingsForSourceUncached(source)         # bypass per-source cache
    if !existing:                                            # validation failed earlier?
      content = readFileSync(filePath)  # or null on ENOENT
      if content !== null:
        rawData = safeParseJSON(content)
        if rawData == null:                                  # syntax error
          return {error: Error("Invalid JSON syntax in settings file at " + filePath)}
        if rawData is object:
          existing = rawData                                 # use raw to preserve unknown fields
          logForDebugging("Using raw settings from <path> due to validation failure")
    updated = mergeWith(existing || {}, settings, writeSideCustomizer)  # see §5.3
    markInternalWrite(filePath)                              # for echo suppression
    writeFileSyncAndFlush(filePath, jsonStringify(updated, null, 2) + '\n')
    resetSettingsCache()                                     # invalidate all three caches
    if source == 'localSettings':
      void addFileGlobRuleToGitignore('.claude/settings.local.json', originalCwd)   # async
  catch e:
    logError(e); return {error: new Error("Failed to read raw settings from <path>: <e>")}
  return {error: null}
```

**Hard-to-spot invariants**:
- The "raw fallback" branch preserves user data even when the file currently fails Zod validation — we re-read `safeParseJSON` and merge into the parsed-as-untyped object. This is why `passthrough()` on `SettingsSchema` and `permissions` is critical (see §6.4).
- Arrays **replace** on write but **concat-dedupe** on read. This means if `userSettings` has `permissions.allow = ['A','B']` and you call `updateSettingsForSource('userSettings', {permissions:{allow:['C']}})`, the **on-disk** result is `['C']` only (the existing settings already include `['A','B']`, mergeWith calls customizer with `objValue=['A','B'], srcValue=['C']`, customizer returns `['C']`). Confirmed at `settings.ts:489-491` comment: **"For arrays, always replace with the provided array. This puts the responsibility on the caller to compute the desired final state."**

> **Pattern F — intentional read/write customizer asymmetry (reimplementer hazard).** Two `mergeWith` customizers live in `settings.ts` with **opposite** array semantics, and both are correct as-is:
>
> | Customizer | Location | Array behaviour | Used by |
> |---|---|---|---|
> | Inline write-side | `settings.ts:478-494` | Arrays **replace** (`return srcValue`); `srcValue === undefined` deletes the key from object | `updateSettingsForSource()` only |
> | Exported `settingsMergeCustomizer` | `settings.ts:538-547` | Arrays **concat + dedupe** (`uniq([...obj, ...src])` via `mergeArrays`) | All cascade reads — `getInitialSettings()`, `getSettingsForSource()`, every multi-source merge |
>
> A reimplementer who consolidates the two into a single "merge arrays" helper will silently corrupt `permissions.allow`/`deny`/`ask` cascades: a single-source write to `userSettings.permissions.allow=['C']` will then leak prior `['A','B']` back into the **on-disk** file, breaking the contract that the caller owns the final array. Conversely, a reimplementer who unifies on the write-side semantics will collapse the cascade so that a higher-priority `policySettings.permissions.allow=['X']` shadows the user/project rules instead of unioning them. Both `mergeArrays` and the inline write-side customizer must be preserved as separate functions on the same path. The asymmetry is **the design**, not an accident; spec 09 (permission rules runtime) depends on it.

### 5.5 Change-detector pipeline (`changeDetector.ts`)

#### 5.5.1 Constants

```
FILE_STABILITY_THRESHOLD_MS    = 1000
FILE_STABILITY_POLL_INTERVAL_MS= 500
INTERNAL_WRITE_WINDOW_MS       = 5000
MDM_POLL_INTERVAL_MS           = 30 * 60 * 1000     // 30 minutes
DELETION_GRACE_MS              = FILE_STABILITY_THRESHOLD_MS + FILE_STABILITY_POLL_INTERVAL_MS + 200  // 1700ms
```

#### 5.5.2 `initialize()` (lines 84-146)

1. Skip entirely if `getIsRemoteMode()` (35).
2. Idempotent guard via `initialized`/`disposed`.
3. Start MDM poll (`startMdmPoll()`).
4. `registerCleanup(dispose)` for graceful shutdown.
5. `getWatchTargets()` collects parent dirs of all source paths **except `flagSettings`** (which may live in `$TMPDIR` and may be a special file / FIFO — see GitHub issue #16469 cited in source).
6. Add the managed `managed-settings.d/` directory if it exists.
7. `chokidar.watch(dirs, {persistent:true, ignoreInitial:true, depth:0, awaitWriteFinish:{stabilityThreshold:1000, pollInterval:500}, ignored: <function>, ignorePermissionErrors:true, usePolling:false, atomic:true})`.
8. Custom `ignored` function:
   - Ignore non-file/non-directory inodes (sockets/FIFOs/devices) — would error EOPNOTSUPP on macOS.
   - Ignore any path containing a `.git` segment.
   - Allow directories (chokidar needs them).
   - Allow only known settings files (normalized) and `.json` files inside `dropInDir`.
9. Wire `change`, `unlink`, `add` handlers.

#### 5.5.3 Event handlers

`handleChange(path)`:
1. `source = getSourceForPath(path)`.
2. If a deletion was pending: clear timer, log "Cancelled pending deletion … file was recreated".
3. `consumeInternalWrite(path, 5000)` → if true, return (this is our own write).
4. `executeConfigChangeHooks(...)` and if `hasBlockingResult(results)` skip; else `fanOut(source)`.

`handleAdd(path)`: identical to `handleChange` but explicitly cancels pending deletions and falls through to `handleChange`.

`handleDelete(path)`:
1. Schedule `setTimeout(..., DELETION_GRACE_MS)` to absorb delete-and-recreate (auto-updater, sibling session).
2. On fire: run `executeConfigChangeHooks` → if non-blocking, `fanOut(source)`.

`fanOut(source)` (lines 437-440): single producer guarantees one disk reload per notification:
```
function fanOut(source):
  resetSettingsCache()      # MUST happen before listener iteration
  settingsChanged.emit(source)
```

The comment at `:421-435` documents the N-way thrashing bug previously caused by per-listener resets.

`getSourceForPath(path)`:
- Drop-in: if path starts with `dropInDir + sep` → `'policySettings'`.
- Otherwise: linear scan over `SETTING_SOURCES` matching against `getSettingsFilePathForSource`.

`settingSourceToConfigChangeSource` mapping (`changeDetector.ts:252-266`):
```
userSettings    → 'user_settings'
projectSettings → 'project_settings'
localSettings   → 'local_settings'
flagSettings    → 'policy_settings'    # collapsed
policySettings  → 'policy_settings'
```

> **Information-loss collision (reimplementer hazard).** Verified at `changeDetector.ts:262-264` — both `flagSettings` and `policySettings` are mapped to the same `ConfigChangeSource` value `'policy_settings'`. `ConfigChange` hook subscribers (owned by spec 09; see `src/utils/hooks.ts`) **cannot distinguish** a CLI-flag-driven change from a policy/MDM-driven change at the hook surface. In practice this rarely matters because `flagSettings` is set once at startup and not edited on-disk during a session (see `changeDetector.ts:190-194`, which short-circuits flag-source change events) — but the type-level collapse is a real lossy mapping that a reimplementer who derives a 1:1 enum from `SettingSource` will accidentally avoid. Cross-spec impact: spec 09 must document that `'policy_settings'` is a **union surface** covering both inputs.

#### 5.5.4 MDM poll (lines 381-418)

Every 30 min: call `refreshMdmSettings()` (fresh `fireRawRead` + parse), JSON-stringify `{mdm, hkcu}`, compare to `lastMdmSnapshot`. If changed: `setMdmSettingsCache(...)`, log, `fanOut('policySettings')`. Timer is `unref()`'d so it does not keep the process alive.

### 5.6 `applySettingsChange(source, setAppState)` (`applySettingsChange.ts`)

```
function applySettingsChange(source, setAppState):
  newSettings = getInitialSettings()
  log "Settings changed from <source>, updating app state"
  updatedRules = loadAllPermissionRulesFromDisk()
  updateHooksConfigSnapshot()
  setAppState(prev =>
    let newCtx = syncPermissionRulesFromDisk(prev.toolPermissionContext, updatedRules)
    if USER_TYPE === 'ant' && CLAUDE_CODE_ENTRYPOINT !== 'local-agent':
      overlyBroad = findOverlyBroadBashPermissions(updatedRules, [])
      if overlyBroad.length > 0: newCtx = removeDangerousPermissions(newCtx, overlyBroad)
    if newCtx.isBypassPermissionsModeAvailable && isBypassPermissionsModeDisabled():
      newCtx = createDisabledBypassPermissionsContext(newCtx)
    newCtx = transitionPlanAutoMode(newCtx)
    prevEffort = prev.settings.effortLevel
    newEffort = newSettings.effortLevel
    effortChanged = prevEffort !== newEffort
    return {
      ...prev,
      settings: newSettings,
      toolPermissionContext: newCtx,
      ...(effortChanged && newEffort !== undefined ? {effortValue: newEffort} : {})
    }
  )
```

The `effortValue` propagation rule is intentionally narrow (see source comment at `:80-90`): only propagate a defined new effort value, otherwise stale `prev.settings.effortLevel` would clobber a `--effort` CLI flag value.

### 5.7 `validateInputForSettingsFileEdit` (FileEdit guard) (`validateEditTool.ts`)

```
function validateInputForSettingsFileEdit(filePath, originalContent, getUpdatedContent):
  if !isClaudeSettingsPath(filePath): return null    # not our file
  before = validateSettingsFileContent(originalContent)
  if !before.isValid: return null                    # file was already broken — let user fix
  after = validateSettingsFileContent(getUpdatedContent())
  if !after.isValid:
    return {result:false, errorCode:10, message:
      `Claude Code settings.json validation failed after edit:\n${after.error}\n\nFull schema:\n${after.fullSchema}\nIMPORTANT: Do not update the env unless explicitly instructed to do so.`}
  return null
```

`validateSettingsFileContent` parses JSON, then `SettingsSchema().strict().safeParse()` (note: `.strict()` is used here — the regular read path uses `.passthrough()`/`safeParse()` without strict, so this guard is **stricter** than schema enforcement at load time).

### 5.8 Permission rule pre-filter (`validation.ts:224-265`)

```
function filterInvalidPermissionRules(data, filePath):
  if !data is object: return []
  perms = data.permissions
  if !perms is object: return []
  warnings = []
  for key in ['allow','deny','ask']:
    rules = perms[key]
    if !Array.isArray(rules): continue
    perms[key] = rules.filter(rule =>
      if typeof rule !== 'string':
        warnings.push({file, path:`permissions.${key}`, message:`Non-string value in ${key} array was removed`, invalidValue:rule})
        return false
      result = validatePermissionRule(rule)
      if !result.valid:
        msg = `Invalid permission rule "${rule}" was skipped` + (result.error ? `: ${result.error}` : '') + (result.suggestion ? `. ${result.suggestion}` : '')
        warnings.push({file, path:`permissions.${key}`, message:msg, invalidValue:rule})
        return false
      return true
    )
  return warnings
```

This **mutates** the input object in place before Zod sees it. Without this, a single bad rule (e.g. `'badtoolname()'`) would fail `superRefine` on `PermissionRuleSchema` and the whole file would be rejected.

### 5.9 `validatePermissionRule` (`permissionValidation.ts:58-238`)

Decision tree (in order; first failing branch returns):

1. Empty / whitespace-only → `'Permission rule cannot be empty'`.
2. Mismatched **unescaped** parens → `'Mismatched parentheses'`.
3. Empty unescaped `()` → `'Empty parentheses'` or `'Empty parentheses with no tool name'`.
4. Parse via `permissionRuleValueFromString(rule)`.
5. **MCP rules** (`mcpInfoFromString(parsed.toolName)` truthy):
   - Any parens content → `'MCP rules do not support patterns in parentheses'` with suggestion `mcp__server`, `mcp__server__*`, or `mcp__server__tool`.
   - Else → valid.
6. Empty tool name → `'Tool name cannot be empty'`.
7. Tool name not uppercase first letter → `'Tool names must start with uppercase'` + suggest `capitalize(toolName)`.
8. Custom validator (`getCustomValidation(toolName)`):
   - **WebSearch**: rejects `*` or `?` → `'WebSearch does not support wildcards'`.
   - **WebFetch**: rejects URL format (`://` or `http`) → `'WebFetch permissions use domain format, not URLs'`. Requires `domain:` prefix → `'WebFetch permissions must use "domain:" prefix'`.
9. **Bash** (`isBashPrefixTool(toolName)`):
   - `:*` not at end → `'The :* pattern must be at the end'`.
   - `:*` alone → `'Prefix cannot be empty before :*'`.
10. **File-pattern tools** (`isFilePatternTool(toolName)`, set: `Read, Write, Edit, Glob, NotebookRead, NotebookEdit`):
    - Contains `:*` → `'The ":*" syntax is only for Bash prefix rules'`.
    - Wildcard not at boundary → `'Wildcard placement might be incorrect'`.

`PermissionRuleSchema` (line 244-262) wraps as `z.string().superRefine` and emits `z.ZodIssueCode.custom` with `params: { received: val }`.

### 5.10 Migration pipeline (called by `main.tsx:328-341`)

Fixed order, all called within a try/catch in `main.tsx` (block ~ lines 327-342):

```
1. migrateAutoUpdatesToSettings
2. migrateBypassPermissionsAcceptedToSettings
3. migrateEnableAllProjectMcpServersToSettings
4. resetProToOpusDefault
5. migrateSonnet1mToSonnet45
6. migrateLegacyOpusToCurrent
7. migrateSonnet45ToSonnet46
8. migrateOpusToOpus1m
9. migrateReplBridgeEnabledToRemoteControlAtStartup
10. resetAutoModeOptInForDefaultOffer        // gated by feature('TRANSCRIPT_CLASSIFIER') internally
11. migrateFennecToOpus                       // DUAL-GATED: outer if ("external" === 'ant') at main.tsx:340 (build-time DCE) + inner if (process.env.USER_TYPE !== 'ant') return at migrateFennecToOpus.ts:19 (runtime). External bundles strip the call entirely; internal-runtime check is defense-in-depth in case the build-time gate is bypassed.
```

> **Build-time DCE vs runtime guard (Pattern C reminder).** The literal `"external" === 'ant'` compare in `main.tsx` is a **build-time** marker that Bun's `bun:bundle` (see CLAUDE.md "Two Dominant Patterns") replaces with `false` and dead-code-eliminates for external bundles. It is **not** a runtime check — `"external"` is the bundle-replaced literal, not a variable. The `process.env.USER_TYPE === 'ant'` check inside the migration body is the runtime guard. Spec readers must not collapse the two: a faithful reimplementation needs **both** (build-time DCE to strip the import; runtime gate for defense-in-depth). The same dual pattern applies to every other `"external" === 'ant'` site enumerated in §10.3.

Idempotency strategies (per migration):

| Migration | Idempotency mechanism |
|---|---|
| `migrateAutoUpdatesToSettings` | Reads `globalConfig.autoUpdates !== false` and `autoUpdatesProtectedForNative === true`; deletes both keys at end. |
| `migrateBypassPermissionsAcceptedToSettings` | Reads `bypassPermissionsModeAccepted` flag; calls `hasSkipDangerousModePermissionPrompt()` to skip if already migrated; deletes flag at end. |
| `migrateEnableAllProjectMcpServersToSettings` | Reads `currentProjectConfig` keys; checks `existingSettings.enableAllProjectMcpServers === undefined` for first-write; deletes keys from project config at end. |
| `migrateFennecToOpus` | Reads userSettings.model; idempotent because all branches rewrite to non-fennec aliases. |
| `migrateLegacyOpusToCurrent` | Reads userSettings.model exact-match; idempotent because `'opus'` doesn't match the legacy strings. Sets `legacyOpusMigrationTimestamp` for one-time UI notification. |
| `migrateOpusToOpus1m` | Idempotent because once written it's `'opus[1m]'` not `'opus'`. **Special**: writes `undefined` if `parseUserSpecifiedModel(migrated) === parseUserSpecifiedModel(getDefaultMainLoopModelSetting())` — so users whose default already resolves to opus[1m] get the key removed entirely. |
| `migrateReplBridgeEnabledToRemoteControlAtStartup` | Acts only if old key exists AND new key undefined. |
| `migrateSonnet1mToSonnet45` | `globalConfig.sonnet1m45MigrationComplete` flag; migrates in-memory `mainLoopModelOverride` too. |
| `migrateSonnet45ToSonnet46` | Reads userSettings.model exact-match; idempotent because `'sonnet'` doesn't match. Suppresses notification timestamp if `numStartups <= 1` (brand-new user). |
| `resetAutoModeOptInForDefaultOffer` | `globalConfig.hasResetAutoModeOptInForDefaultOffer` one-shot flag. |
| `resetProToOpusDefault` | `globalConfig.opusProMigrationComplete` flag; **always sets** the flag, even when not eligible (skipped path also marks complete). |

Ordering rationale (cited from source comments):
- `migrateSonnet1mToSonnet45` (5) precedes `migrateSonnet45ToSonnet46` (7) so the explicit Sonnet 4.5 string is materialized before being canonicalized.
- `migrateLegacyOpusToCurrent` (6) precedes `migrateOpusToOpus1m` (8) so explicit Opus 4.0/4.1 IDs are first remapped to `'opus'`, then potentially upgraded to `'opus[1m]'`.
- All Sonnet/Opus migrations are after `resetProToOpusDefault` (4) so the Pro default reset doesn't redo their work.

Migrations that depend on other subsystems:
- `migrateLegacyOpusToCurrent` requires `getAPIProvider()` and `isLegacyModelRemapEnabled()`.
- `migrateSonnet45ToSonnet46` requires `getAPIProvider()` and the three `isXSubscriber()` helpers.
- `migrateSonnet1mToSonnet45` reads/writes `bootstrap/state.getMainLoopModelOverride/setMainLoopModelOverride` — sync of in-memory override.
- `resetAutoModeOptInForDefaultOffer` requires `getAutoModeEnabledState() === 'enabled'` and reads/writes `userSettings.skipAutoPermissionPrompt` and checks `permissions.defaultMode !== 'auto'` (the comment at lines 19-24 explains the careful gating to avoid removing auto from the carousel for `'opt-in'` users).

### 5.11 MDM raw read (`mdm/rawRead.ts`)

`startMdmRawRead()` is called at the top of `main.tsx` **before heavy imports** (per overview §5.1). Internally caches as `rawReadPromise` (singleton). `getMdmRawReadPromise()` retrieves it.

`fireRawRead()` per platform (`mdm/rawRead.ts:55-114`):

- **darwin**: `getMacOSPlistPaths()` → `Promise.all` of `existsSync(path) ? execFile('/usr/bin/plutil', ['-convert','json','-o','-','--', path], {encoding:'utf-8', timeout:5000}) : {stdout:'', ok:false}`. **First successful result wins** in priority order.
- **win32**: `Promise.all` of `execFile('reg', ['query', 'HKLM\\SOFTWARE\\Policies\\ClaudeCode', '/v', 'Settings'])` and same for HKCU.
- **other**: empty result.

`existsSync` short-circuit is an explicit perf optimization (`rawRead.ts:67-70` comment): plutil takes ~5ms even for ENOENT. The synchronous `existsSync` is required to **preserve the spawn-during-imports invariant** — `execFilePromise` must be the first await so plutil spawns before the event loop polls.

`parseRegQueryStdout(stdout, valueName='Settings')` regex:
```
new RegExp(`^\\s+${escaped}\\s+REG_(?:EXPAND_)?SZ\\s+(.*)$`, 'i')
```
case-insensitive, matches both REG_SZ and REG_EXPAND_SZ. Extracts whitespace-trimmed value (`mdm/settings.ts:208-222`).

### 5.12 Cowork mode user settings file

`getUserSettingsFilePath()` (`settings.ts:264-272`):
- Returns `'cowork_settings.json'` if `getUseCoworkPlugins()` (CLI `--cowork`) OR `isEnvTruthy(process.env.CLAUDE_CODE_USE_COWORK_PLUGINS)`.
- Otherwise `'settings.json'`.

This is the only knob that changes the user-settings filename.

### 5.13 `strictPluginOnlyCustomization` lock (`pluginOnlyPolicy.ts`)

```
function isRestrictedToPluginOnly(surface):
  policy = getSettingsForSource('policySettings')?.strictPluginOnlyCustomization
  if policy === true: return true
  if Array.isArray(policy): return policy.includes(surface)
  return false

ADMIN_TRUSTED_SOURCES = Set('plugin','policySettings','built-in','builtin','bundled')

function isSourceAdminTrusted(source):
  return source !== undefined && ADMIN_TRUSTED_SOURCES.has(source)
```

Use pattern (cited verbatim from `pluginOnlyPolicy.ts:54-56`):
```
const allowed = !isRestrictedToPluginOnly(surface) || isSourceAdminTrusted(item.source)
if (item.hooks && allowed) { register(...) }
```

`CUSTOMIZATION_SURFACES = ['skills','agents','hooks','mcp']` (`types.ts:248-253`).

The schema field is **forwards-compat**: a `preprocess` step (`types.ts:519-533`) drops unknown surface names from arrays so a future enum value (e.g. `'commands'`) doesn't fail safeParse and null out the entire managed-settings file. Non-array invalid values fall through `.catch(undefined)` instead of nulling the file.

### 5.14 Trusted-source pattern for security-sensitive flags

Several "I accepted the dialog" flags must NOT honor `projectSettings` (otherwise a malicious project = RCE). Pattern (`settings.ts:882-928`):

```ts
hasSkipDangerousModePermissionPrompt()      // user, local, flag, policy — NOT project
hasAutoModeOptIn()                           // same exclusion (TRANSCRIPT_CLASSIFIER only)
getUseAutoModeDuringPlan()                   // same exclusion (default true)
getAutoModeConfig()                          // same exclusion + ant deny→soft_deny fold
```

The exclusion of `projectSettings` is explicit and commented; do not preserve this only by convention.

---

## 6. Verbatim Assets

### 6.1 `SettingsSchema` — full Zod definition

Verbatim from `src/utils/settings/types.ts:255-1073`. Long — kept inline to be the authoritative bit-exact reference. The schema is `lazySchema(() => z.object({...}).passthrough())`. Refer to lines 255-1073 of `src/utils/settings/types.ts` for the complete content; it is reproduced in full below.

(Begin verbatim — 819 lines)

```ts
export const SettingsSchema = lazySchema(() =>
  z
    .object({
      $schema: z
        .literal(CLAUDE_CODE_SETTINGS_SCHEMA_URL)
        .optional()
        .describe('JSON Schema reference for Claude Code settings'),
      apiKeyHelper: z
        .string()
        .optional()
        .describe('Path to a script that outputs authentication values'),
      awsCredentialExport: z
        .string()
        .optional()
        .describe('Path to a script that exports AWS credentials'),
      awsAuthRefresh: z
        .string()
        .optional()
        .describe('Path to a script that refreshes AWS authentication'),
      gcpAuthRefresh: z
        .string()
        .optional()
        .describe(
          'Command to refresh GCP authentication (e.g., gcloud auth application-default login)',
        ),
      ...(isEnvTruthy(process.env.CLAUDE_CODE_ENABLE_XAA)
        ? {
            xaaIdp: z
              .object({
                issuer: z.string().url().describe('IdP issuer URL for OIDC discovery'),
                clientId: z.string().describe("Claude Code's client_id registered at the IdP"),
                callbackPort: z.number().int().positive().optional().describe(
                  'Fixed loopback callback port for the IdP OIDC login. ' +
                    'Only needed if the IdP does not honor RFC 8252 port-any matching.',
                ),
              })
              .optional()
              .describe(
                'XAA (SEP-990) IdP connection. Configure once; all XAA-enabled MCP servers reuse this.',
              ),
          }
        : {}),
      fileSuggestion: z
        .object({ type: z.literal('command'), command: z.string() })
        .optional()
        .describe('Custom file suggestion configuration for @ mentions'),
      respectGitignore: z
        .boolean()
        .optional()
        .describe(
          'Whether file picker should respect .gitignore files (default: true). ' +
            'Note: .ignore files are always respected.',
        ),
      cleanupPeriodDays: z
        .number()
        .nonnegative()
        .int()
        .optional()
        .describe(
          'Number of days to retain chat transcripts (default: 30). Setting to 0 disables session persistence entirely: no transcripts are written and existing transcripts are deleted at startup.',
        ),
      env: EnvironmentVariablesSchema()
        .optional()
        .describe('Environment variables to set for Claude Code sessions'),
      attribution: z
        .object({
          commit: z.string().optional().describe(
            'Attribution text for git commits, including any trailers. ' +
              'Empty string hides attribution.',
          ),
          pr: z.string().optional().describe(
            'Attribution text for pull request descriptions. ' +
              'Empty string hides attribution.',
          ),
        })
        .optional()
        .describe(
          'Customize attribution text for commits and PRs. ' +
            'Each field defaults to the standard Claude Code attribution if not set.',
        ),
      includeCoAuthoredBy: z.boolean().optional().describe(
        'Deprecated: Use attribution instead. ' +
          "Whether to include Claude's co-authored by attribution in commits and PRs (defaults to true)",
      ),
      includeGitInstructions: z.boolean().optional().describe(
        "Include built-in commit and PR workflow instructions in Claude's system prompt (default: true)",
      ),
      permissions: PermissionsSchema().optional().describe('Tool usage permissions configuration'),
      model: z.string().optional().describe('Override the default model used by Claude Code'),
      availableModels: z.array(z.string()).optional().describe(
        'Allowlist of models that users can select. ' +
          'Accepts family aliases ("opus" allows any opus version), ' +
          'version prefixes ("opus-4-5" allows only that version), ' +
          'and full model IDs. ' +
          'If undefined, all models are available. If empty array, only the default model is available. ' +
          'Typically set in managed settings by enterprise administrators.',
      ),
      modelOverrides: z.record(z.string(), z.string()).optional().describe(
        'Override mapping from Anthropic model ID (e.g. "claude-opus-4-6") to provider-specific ' +
          'model ID (e.g. a Bedrock inference profile ARN). Typically set in managed settings by ' +
          'enterprise administrators.',
      ),
      enableAllProjectMcpServers: z.boolean().optional().describe(
        'Whether to automatically approve all MCP servers in the project',
      ),
      enabledMcpjsonServers: z.array(z.string()).optional().describe(
        'List of approved MCP servers from .mcp.json',
      ),
      disabledMcpjsonServers: z.array(z.string()).optional().describe(
        'List of rejected MCP servers from .mcp.json',
      ),
      allowedMcpServers: z.array(AllowedMcpServerEntrySchema()).optional().describe(
        'Enterprise allowlist of MCP servers that can be used. ' +
          'Applies to all scopes including enterprise servers from managed-mcp.json. ' +
          'If undefined, all servers are allowed. If empty array, no servers are allowed. ' +
          'Denylist takes precedence - if a server is on both lists, it is denied.',
      ),
      deniedMcpServers: z.array(DeniedMcpServerEntrySchema()).optional().describe(
        'Enterprise denylist of MCP servers that are explicitly blocked. ' +
          'If a server is on the denylist, it will be blocked across all scopes including enterprise. ' +
          'Denylist takes precedence over allowlist - if a server is on both lists, it is denied.',
      ),
      hooks: HooksSchema().optional().describe('Custom commands to run before/after tool executions'),
      worktree: z.object({
        symlinkDirectories: z.array(z.string()).optional().describe(
          'Directories to symlink from main repository to worktrees to avoid disk bloat. ' +
            'Must be explicitly configured - no directories are symlinked by default. ' +
            'Common examples: "node_modules", ".cache", ".bin"',
        ),
        sparsePaths: z.array(z.string()).optional().describe(
          'Directories to include when creating worktrees, via git sparse-checkout (cone mode). ' +
            'Dramatically faster in large monorepos — only the listed paths are written to disk.',
        ),
      }).optional().describe('Git worktree configuration for --worktree flag.'),
      disableAllHooks: z.boolean().optional().describe('Disable all hooks and statusLine execution'),
      defaultShell: z.enum(['bash', 'powershell']).optional().describe(
        'Default shell for input-box ! commands. ' +
          "Defaults to 'bash' on all platforms (no Windows auto-flip).",
      ),
      allowManagedHooksOnly: z.boolean().optional().describe(
        'When true (and set in managed settings), only hooks from managed settings run. ' +
          'User, project, and local hooks are ignored.',
      ),
      allowedHttpHookUrls: z.array(z.string()).optional().describe(
        'Allowlist of URL patterns that HTTP hooks may target. ' +
          'Supports * as a wildcard (e.g. "https://hooks.example.com/*"). ' +
          'When set, HTTP hooks with non-matching URLs are blocked. ' +
          'If undefined, all URLs are allowed. If empty array, no HTTP hooks are allowed. ' +
          'Arrays merge across settings sources (same semantics as allowedMcpServers).',
      ),
      httpHookAllowedEnvVars: z.array(z.string()).optional().describe(
        'Allowlist of environment variable names HTTP hooks may interpolate into headers. ' +
          "When set, each hook's effective allowedEnvVars is the intersection with this list. " +
          'If undefined, no restriction is applied. ' +
          'Arrays merge across settings sources (same semantics as allowedMcpServers).',
      ),
      allowManagedPermissionRulesOnly: z.boolean().optional().describe(
        'When true (and set in managed settings), only permission rules (allow/deny/ask) from managed settings are respected. ' +
          'User, project, local, and CLI argument permission rules are ignored.',
      ),
      allowManagedMcpServersOnly: z.boolean().optional().describe(
        'When true (and set in managed settings), allowedMcpServers is only read from managed settings. ' +
          'deniedMcpServers still merges from all sources, so users can deny servers for themselves. ' +
          'Users can still add their own MCP servers, but only the admin-defined allowlist applies.',
      ),
      strictPluginOnlyCustomization: z
        .preprocess(
          v =>
            Array.isArray(v)
              ? v.filter(x => (CUSTOMIZATION_SURFACES as readonly string[]).includes(x))
              : v,
          z.union([z.boolean(), z.array(z.enum(CUSTOMIZATION_SURFACES))]),
        )
        .optional()
        .catch(undefined)
        .describe(
          'When set in managed settings, blocks non-plugin customization sources for the listed surfaces. ' +
            'Array form locks specific surfaces (e.g. ["skills", "hooks"]); `true` locks all four; `false` is an explicit no-op. ' +
            'Blocked: ~/.claude/{surface}/, .claude/{surface}/ (project), settings.json hooks, .mcp.json. ' +
            'NOT blocked: managed (policySettings) sources, plugin-provided customizations. ' +
            'Composes with strictKnownMarketplaces for end-to-end admin control — plugins gated by ' +
            'marketplace allowlist, everything else blocked here.',
        ),
      statusLine: z.object({
        type: z.literal('command'),
        command: z.string(),
        padding: z.number().optional(),
      }).optional().describe('Custom status line display configuration'),
      enabledPlugins: z.record(
        z.string(),
        z.union([z.array(z.string()), z.boolean(), z.undefined()]),
      ).optional().describe(
        'Enabled plugins using plugin-id@marketplace-id format. Example: { "formatter@anthropic-tools": true }. Also supports extended format with version constraints.',
      ),
      extraKnownMarketplaces: z
        .record(z.string(), ExtraKnownMarketplaceSchema())
        .check(ctx => {
          for (const [key, entry] of Object.entries(ctx.value)) {
            if (entry.source.source === 'settings' && entry.source.name !== key) {
              ctx.issues.push({
                code: 'custom',
                input: entry.source.name,
                path: [key, 'source', 'name'],
                message:
                  `Settings-sourced marketplace name must match its extraKnownMarketplaces key ` +
                  `(got key "${key}" but source.name "${entry.source.name}")`,
              })
            }
          }
        })
        .optional()
        .describe(
          'Additional marketplaces to make available for this repository. Typically used in repository .claude/settings.json to ensure team members have required plugin sources.',
        ),
      strictKnownMarketplaces: z.array(MarketplaceSourceSchema()).optional().describe(
        'Enterprise strict list of allowed marketplace sources. When set in managed settings, ' +
          'ONLY these exact sources can be added as marketplaces. The check happens BEFORE ' +
          'downloading, so blocked sources never touch the filesystem. ' +
          'Note: this is a policy gate only — it does NOT register marketplaces. ' +
          'To pre-register allowed marketplaces for users, also set extraKnownMarketplaces.',
      ),
      blockedMarketplaces: z.array(MarketplaceSourceSchema()).optional().describe(
        'Enterprise blocklist of marketplace sources. When set in managed settings, ' +
          'these exact sources are blocked from being added as marketplaces. The check happens BEFORE ' +
          'downloading, so blocked sources never touch the filesystem.',
      ),
      forceLoginMethod: z.enum(['claudeai', 'console']).optional().describe(
        'Force a specific login method: "claudeai" for Claude Pro/Max, "console" for Console billing',
      ),
      forceLoginOrgUUID: z.string().optional().describe('Organization UUID to use for OAuth login'),
      otelHeadersHelper: z.string().optional().describe('Path to a script that outputs OpenTelemetry headers'),
      outputStyle: z.string().optional().describe('Controls the output style for assistant responses'),
      language: z.string().optional().describe(
        'Preferred language for Claude responses and voice dictation (e.g., "japanese", "spanish")',
      ),
      skipWebFetchPreflight: z.boolean().optional().describe(
        'Skip the WebFetch blocklist check for enterprise environments with restrictive security policies',
      ),
      sandbox: SandboxSettingsSchema().optional(),
      feedbackSurveyRate: z.number().min(0).max(1).optional().describe(
        'Probability (0–1) that the session quality survey appears when eligible. 0.05 is a reasonable starting point.',
      ),
      spinnerTipsEnabled: z.boolean().optional().describe('Whether to show tips in the spinner'),
      spinnerVerbs: z.object({
        mode: z.enum(['append', 'replace']),
        verbs: z.array(z.string()),
      }).optional().describe(
        'Customize spinner verbs. mode: "append" adds verbs to defaults, "replace" uses only your verbs.',
      ),
      spinnerTipsOverride: z.object({
        excludeDefault: z.boolean().optional(),
        tips: z.array(z.string()),
      }).optional().describe(
        'Override spinner tips. tips: array of tip strings. excludeDefault: if true, only show custom tips (default: false).',
      ),
      syntaxHighlightingDisabled: z.boolean().optional().describe('Whether to disable syntax highlighting in diffs'),
      terminalTitleFromRename: z.boolean().optional().describe(
        'Whether /rename updates the terminal tab title (defaults to true). Set to false to keep auto-generated topic titles.',
      ),
      alwaysThinkingEnabled: z.boolean().optional().describe(
        'When false, thinking is disabled. When absent or true, thinking is ' +
          'enabled automatically for supported models.',
      ),
      effortLevel: z
        .enum(process.env.USER_TYPE === 'ant'
          ? ['low', 'medium', 'high', 'max']
          : ['low', 'medium', 'high'])
        .optional()
        .catch(undefined)
        .describe('Persisted effort level for supported models.'),
      advisorModel: z.string().optional().describe('Advisor model for the server-side advisor tool.'),
      fastMode: z.boolean().optional().describe(
        'When true, fast mode is enabled. When absent or false, fast mode is off.',
      ),
      fastModePerSessionOptIn: z.boolean().optional().describe(
        'When true, fast mode does not persist across sessions. Each session starts with fast mode off.',
      ),
      promptSuggestionEnabled: z.boolean().optional().describe(
        'When false, prompt suggestions are disabled. When absent or true, ' +
          'prompt suggestions are enabled.',
      ),
      showClearContextOnPlanAccept: z.boolean().optional().describe(
        'When true, the plan-approval dialog offers a "clear context" option. Defaults to false.',
      ),
      agent: z.string().optional().describe(
        'Name of an agent (built-in or custom) to use for the main thread. ' +
          "Applies the agent's system prompt, tool restrictions, and model.",
      ),
      companyAnnouncements: z.array(z.string()).optional().describe(
        'Company announcements to display at startup (one will be randomly selected if multiple are provided)',
      ),
      pluginConfigs: z.record(
        z.string(),
        z.object({
          mcpServers: z.record(
            z.string(),
            z.record(z.string(), z.union([z.string(), z.number(), z.boolean(), z.array(z.string())])),
          ).optional().describe('User configuration values for MCP servers keyed by server name'),
          options: z.record(
            z.string(),
            z.union([z.string(), z.number(), z.boolean(), z.array(z.string())]),
          ).optional().describe(
            'Non-sensitive option values from plugin manifest userConfig, keyed by option name. Sensitive values go to secure storage instead.',
          ),
        }),
      ).optional().describe(
        'Per-plugin configuration including MCP server user configs, keyed by plugin ID (plugin@marketplace format)',
      ),
      remote: z.object({
        defaultEnvironmentId: z.string().optional().describe('Default environment ID to use for remote sessions'),
      }).optional().describe('Remote session configuration'),
      autoUpdatesChannel: z.enum(['latest', 'stable']).optional().describe('Release channel for auto-updates (latest or stable)'),
      ...(feature('LODESTONE')
        ? {
            disableDeepLinkRegistration: z.enum(['disable']).optional().describe(
              'Prevent claude-cli:// protocol handler registration with the OS',
            ),
          }
        : {}),
      minimumVersion: z.string().optional().describe(
        'Minimum version to stay on - prevents downgrades when switching to stable channel',
      ),
      plansDirectory: z.string().optional().describe(
        'Custom directory for plan files, relative to project root. ' +
          'If not set, defaults to ~/.claude/plans/',
      ),
      ...(process.env.USER_TYPE === 'ant'
        ? {
            classifierPermissionsEnabled: z.boolean().optional().describe(
              'Enable AI-based classification for Bash(prompt:...) permission rules',
            ),
          }
        : {}),
      ...(feature('PROACTIVE') || feature('KAIROS')
        ? {
            minSleepDurationMs: z.number().nonnegative().int().optional().describe(
              'Minimum duration in milliseconds that the Sleep tool must sleep for. ' +
                'Useful for throttling proactive tick frequency.',
            ),
            maxSleepDurationMs: z.number().int().min(-1).optional().describe(
              'Maximum duration in milliseconds that the Sleep tool can sleep for. ' +
                'Set to -1 for indefinite sleep (waits for user input). ' +
                'Useful for limiting idle time in remote/managed environments.',
            ),
          }
        : {}),
      ...(feature('VOICE_MODE')
        ? { voiceEnabled: z.boolean().optional().describe('Enable voice mode (hold-to-talk dictation)') }
        : {}),
      ...(feature('KAIROS')
        ? {
            assistant: z.boolean().optional().describe(
              'Start Claude in assistant mode (custom system prompt, brief view, scheduled check-in skills)',
            ),
            assistantName: z.string().optional().describe(
              'Display name for the assistant, shown in the claude.ai session list',
            ),
          }
        : {}),
      channelsEnabled: z.boolean().optional().describe(
        'Teams/Enterprise opt-in for channel notifications (MCP servers with the ' +
          'claude/channel capability pushing inbound messages). Default off. ' +
          'Set true to allow; users then select servers via --channels.',
      ),
      allowedChannelPlugins: z.array(z.object({
        marketplace: z.string(),
        plugin: z.string(),
      })).optional().describe(
        'Teams/Enterprise allowlist of channel plugins. When set, ' +
          'replaces the default Anthropic allowlist — admins decide which ' +
          'plugins may push inbound messages. Undefined falls back to the default. ' +
          'Requires channelsEnabled: true.',
      ),
      ...(feature('KAIROS') || feature('KAIROS_BRIEF')
        ? {
            defaultView: z.enum(['chat', 'transcript']).optional().describe(
              'Default transcript view: chat (SendUserMessage checkpoints only) or transcript (full)',
            ),
          }
        : {}),
      prefersReducedMotion: z.boolean().optional().describe(
        'Reduce or disable animations for accessibility (spinner shimmer, flash effects, etc.)',
      ),
      autoMemoryEnabled: z.boolean().optional().describe(
        'Enable auto-memory for this project. When false, Claude will not read from or write to the auto-memory directory.',
      ),
      autoMemoryDirectory: z.string().optional().describe(
        'Custom directory path for auto-memory storage. Supports ~/ prefix for home directory expansion. Ignored if set in projectSettings (checked-in .claude/settings.json) for security. When unset, defaults to ~/.claude/projects/<sanitized-cwd>/memory/.',
      ),
      autoDreamEnabled: z.boolean().optional().describe(
        'Enable background memory consolidation (auto-dream). When set, overrides the server-side default.',
      ),
      showThinkingSummaries: z.boolean().optional().describe(
        'Show thinking summaries in the transcript view (ctrl+o). Default: false.',
      ),
      skipDangerousModePermissionPrompt: z.boolean().optional().describe(
        'Whether the user has accepted the bypass permissions mode dialog',
      ),
      ...(feature('TRANSCRIPT_CLASSIFIER')
        ? {
            skipAutoPermissionPrompt: z.boolean().optional().describe(
              'Whether the user has accepted the auto mode opt-in dialog',
            ),
            useAutoModeDuringPlan: z.boolean().optional().describe(
              'Whether plan mode uses auto mode semantics when auto mode is available (default: true)',
            ),
            autoMode: z.object({
              allow: z.array(z.string()).optional().describe('Rules for the auto mode classifier allow section'),
              soft_deny: z.array(z.string()).optional().describe('Rules for the auto mode classifier deny section'),
              ...(process.env.USER_TYPE === 'ant'
                ? { deny: z.array(z.string()).optional() }
                : {}),
              environment: z.array(z.string()).optional().describe(
                'Entries for the auto mode classifier environment section',
              ),
            }).optional().describe('Auto mode classifier prompt customization'),
          }
        : {}),
      disableAutoMode: z.enum(['disable']).optional().describe('Disable auto mode'),
      sshConfigs: z.array(z.object({
        id: z.string().describe('Unique identifier for this SSH config. Used to match configs across settings sources.'),
        name: z.string().describe('Display name for the SSH connection'),
        sshHost: z.string().describe(
          'SSH host in format "user@hostname" or "hostname", or a host alias from ~/.ssh/config',
        ),
        sshPort: z.number().int().optional().describe('SSH port (default: 22)'),
        sshIdentityFile: z.string().optional().describe('Path to SSH identity file (private key)'),
        startDirectory: z.string().optional().describe(
          'Default working directory on the remote host. ' +
            'Supports tilde expansion (e.g. ~/projects). ' +
            'If not specified, defaults to the remote user home directory. ' +
            'Can be overridden by the [dir] positional argument in `claude ssh <config> [dir]`.',
        ),
      })).optional().describe(
        'SSH connection configurations for remote environments. ' +
          'Typically set in managed settings by enterprise administrators ' +
          'to pre-configure SSH connections for team members.',
      ),
      claudeMdExcludes: z.array(z.string()).optional().describe(
        'Glob patterns or absolute paths of CLAUDE.md files to exclude from loading. ' +
          'Patterns are matched against absolute file paths using picomatch. ' +
          'Only applies to User, Project, and Local memory types (Managed/policy files cannot be excluded). ' +
          'Examples: "/home/user/monorepo/CLAUDE.md", "**/code/CLAUDE.md", "**/some-dir/.claude/rules/**"',
      ),
      pluginTrustMessage: z.string().optional().describe(
        'Custom message to append to the plugin trust warning shown before installation. ' +
          'Only read from policy settings (managed-settings.json / MDM). ' +
          'Useful for enterprise administrators to add organization-specific context ' +
          '(e.g., "All plugins from our internal marketplace are vetted and approved.").',
      ),
    })
    .passthrough(),
)
```

(End verbatim)

### 6.2 `PermissionsSchema` — verbatim (`types.ts:42-85`)

```ts
export const PermissionsSchema = lazySchema(() =>
  z
    .object({
      allow: z.array(PermissionRuleSchema()).optional().describe('List of permission rules for allowed operations'),
      deny:  z.array(PermissionRuleSchema()).optional().describe('List of permission rules for denied operations'),
      ask:   z.array(PermissionRuleSchema()).optional().describe(
        'List of permission rules that should always prompt for confirmation',
      ),
      defaultMode: z
        .enum(feature('TRANSCRIPT_CLASSIFIER') ? PERMISSION_MODES : EXTERNAL_PERMISSION_MODES)
        .optional()
        .describe('Default permission mode when Claude Code needs access'),
      disableBypassPermissionsMode: z.enum(['disable']).optional().describe(
        'Disable the ability to bypass permission prompts',
      ),
      ...(feature('TRANSCRIPT_CLASSIFIER')
        ? { disableAutoMode: z.enum(['disable']).optional().describe('Disable auto mode') }
        : {}),
      additionalDirectories: z.array(z.string()).optional().describe(
        'Additional directories to include in the permission scope',
      ),
    })
    .passthrough(),
)
```

`EXTERNAL_PERMISSION_MODES = ['acceptEdits','bypassPermissions','default','dontAsk','plan']` (`types/permissions.ts:16-22`).
`PERMISSION_MODES = INTERNAL_PERMISSION_MODES = [...EXTERNAL_PERMISSION_MODES, ...(feature('TRANSCRIPT_CLASSIFIER') ? ['auto'] : [])]` (`types/permissions.ts:33-38`).

### 6.3 Hook schemas — verbatim (`schemas/hooks.ts:32-189`)

The four hook variants form a discriminated union on `type`:

```ts
const IfConditionSchema = lazySchema(() =>
  z.string().optional().describe(
    'Permission rule syntax to filter when this hook runs (e.g., "Bash(git *)"). ' +
      'Only runs if the tool call matches the pattern. Avoids spawning hooks for non-matching commands.',
  ),
)

// (1) command/Bash hook
const BashCommandHookSchema = z.object({
  type: z.literal('command').describe('Shell command hook type'),
  command: z.string().describe('Shell command to execute'),
  if: IfConditionSchema(),
  shell: z.enum(SHELL_TYPES).optional().describe(
    "Shell interpreter. 'bash' uses your $SHELL (bash/zsh/sh); 'powershell' uses pwsh. Defaults to bash.",
  ),
  timeout: z.number().positive().optional().describe('Timeout in seconds for this specific command'),
  statusMessage: z.string().optional().describe('Custom status message to display in spinner while hook runs'),
  once: z.boolean().optional().describe('If true, hook runs once and is removed after execution'),
  async: z.boolean().optional().describe('If true, hook runs in background without blocking'),
  asyncRewake: z.boolean().optional().describe(
    'If true, hook runs in background and wakes the model on exit code 2 (blocking error). Implies async.',
  ),
})

// (2) prompt hook
const PromptHookSchema = z.object({
  type: z.literal('prompt').describe('LLM prompt hook type'),
  prompt: z.string().describe(
    'Prompt to evaluate with LLM. Use $ARGUMENTS placeholder for hook input JSON.',
  ),
  if: IfConditionSchema(),
  timeout: z.number().positive().optional().describe('Timeout in seconds for this specific prompt evaluation'),
  model: z.string().optional().describe(
    'Model to use for this prompt hook (e.g., "claude-sonnet-4-6"). If not specified, uses the default small fast model.',
  ),
  statusMessage: z.string().optional(),
  once: z.boolean().optional(),
})

// (3) http hook
const HttpHookSchema = z.object({
  type: z.literal('http').describe('HTTP hook type'),
  url: z.string().url().describe('URL to POST the hook input JSON to'),
  if: IfConditionSchema(),
  timeout: z.number().positive().optional(),
  headers: z.record(z.string(), z.string()).optional().describe(
    'Additional headers to include in the request. Values may reference environment variables using $VAR_NAME or ${VAR_NAME} syntax (e.g., "Authorization": "Bearer $MY_TOKEN"). Only variables listed in allowedEnvVars will be interpolated.',
  ),
  allowedEnvVars: z.array(z.string()).optional().describe(
    'Explicit list of environment variable names that may be interpolated in header values. Only variables listed here will be resolved; all other $VAR references are left as empty strings. Required for env var interpolation to work.',
  ),
  statusMessage: z.string().optional(),
  once: z.boolean().optional(),
})

// (4) agent hook
const AgentHookSchema = z.object({
  type: z.literal('agent').describe('Agentic verifier hook type'),
  // DO NOT add .transform() here. parseSettingsFile + updateSettingsForSource
  // round-trips through JSON.stringify — a transformed function value is silently dropped,
  // deleting the user's prompt from settings.json (gh-24920, CC-79).
  prompt: z.string().describe(
    'Prompt describing what to verify (e.g. "Verify that unit tests ran and passed."). Use $ARGUMENTS placeholder for hook input JSON.',
  ),
  if: IfConditionSchema(),
  timeout: z.number().positive().optional().describe('Timeout in seconds for agent execution (default 60)'),
  model: z.string().optional().describe(
    'Model to use for this agent hook (e.g., "claude-sonnet-4-6"). If not specified, uses Haiku.',
  ),
  statusMessage: z.string().optional(),
  once: z.boolean().optional(),
})

export const HookCommandSchema = lazySchema(() => {
  const { BashCommandHookSchema, PromptHookSchema, AgentHookSchema, HttpHookSchema } = buildHookSchemas()
  return z.discriminatedUnion('type', [BashCommandHookSchema, PromptHookSchema, AgentHookSchema, HttpHookSchema])
})

export const HookMatcherSchema = lazySchema(() =>
  z.object({
    matcher: z.string().optional().describe('String pattern to match (e.g. tool names like "Write")'),
    hooks: z.array(HookCommandSchema()).describe('List of hooks to execute when the matcher matches'),
  }),
)

export const HooksSchema = lazySchema(() =>
  z.partialRecord(z.enum(HOOK_EVENTS), z.array(HookMatcherSchema())),
)
```

`SHELL_TYPES = ['bash', 'powershell'] as const` (`utils/shell/shellProvider.ts:1`).

`HOOK_EVENTS` is exported from `src/entrypoints/agentSdkTypes.js`. The exact enum is bundled (file lookup yielded no matches via grep on the unbundled tree) — see §12 open question. The `getManagedSettingsKeysForLogging` helper in `settings.ts:594-608` enumerates the keys it accepts as legitimate hook event names: **`PreToolUse, PostToolUse, Notification, UserPromptSubmit, SessionStart, SessionEnd, Stop, SubagentStop, PreCompact, PostCompact, TeammateIdle, TaskCreated, TaskCompleted`** (13 events; this is the **observed set** from this allowlist, but `HOOK_EVENTS` may be a superset — spec 09 owns the authoritative enum).

### 6.4 MCP entry schemas — verbatim (`types.ts:115-207`)

```ts
export const AllowedMcpServerEntrySchema = lazySchema(() =>
  z.object({
    serverName: z.string()
      .regex(/^[a-zA-Z0-9_-]+$/, 'Server name can only contain letters, numbers, hyphens, and underscores')
      .optional()
      .describe('Name of the MCP server that users are allowed to configure'),
    serverCommand: z.array(z.string())
      .min(1, 'Server command must have at least one element (the command)')
      .optional()
      .describe('Command array [command, ...args] to match exactly for allowed stdio servers'),
    serverUrl: z.string().optional().describe(
      'URL pattern with wildcard support (e.g., "https://*.example.com/*") for allowed remote MCP servers',
    ),
  }).refine(
    data => count([data.serverName !== undefined, data.serverCommand !== undefined, data.serverUrl !== undefined], Boolean) === 1,
    { message: 'Entry must have exactly one of "serverName", "serverCommand", or "serverUrl"' },
  ),
)
// DeniedMcpServerEntrySchema is structurally identical with same .refine().
```

### 6.5 `EnvironmentVariablesSchema` — verbatim (`types.ts:35-37`)

```ts
export const EnvironmentVariablesSchema = lazySchema(() => z.record(z.string(), z.coerce.string()))
```

`z.coerce.string()` means numbers/booleans in `env` are silently coerced to strings (e.g., `{ "DEBUG": true }` becomes `{ "DEBUG": "true" }`).

### 6.6 `ExtraKnownMarketplaceSchema` — verbatim (`types.ts:91-109`)

```ts
export const ExtraKnownMarketplaceSchema = lazySchema(() =>
  z.object({
    source: MarketplaceSourceSchema().describe('Where to fetch the marketplace from'),
    installLocation: z.string().optional().describe(
      'Local cache path where marketplace manifest is stored (auto-generated if not provided)',
    ),
    autoUpdate: z.boolean().optional().describe(
      'Whether to automatically update this marketplace and its installed plugins on startup',
    ),
  }),
)
```

`MarketplaceSourceSchema` is owned by **28** (`src/utils/plugins/schemas.ts`).

### 6.7 Defaults table

For schema fields, "default" means the value used by consumers when the key is absent. Sources: `.describe()` text or `?? <default>` in consumer code (caller-defined). All schema fields are `.optional()`.

| Field | Schema-stated default | Source |
|---|---|---|
| `respectGitignore` | `true` | describe |
| `cleanupPeriodDays` | `30` | describe |
| `includeCoAuthoredBy` | `true` | describe |
| `includeGitInstructions` | `true` | describe |
| `defaultShell` | `'bash'` (all platforms; **no Windows auto-flip**) | describe |
| `disableAllHooks` | absent (false) | (none) |
| `terminalTitleFromRename` | `true` | describe |
| `alwaysThinkingEnabled` | `true` (when omitted) | describe |
| `promptSuggestionEnabled` | `true` (when omitted) | describe |
| `showClearContextOnPlanAccept` | `false` | describe |
| `showThinkingSummaries` | `false` | describe |
| `useAutoModeDuringPlan` | `true` | describe |
| `prefersReducedMotion` | `false` | describe |
| `channelsEnabled` | `false` | describe |
| `autoUpdatesChannel` | `'latest'` | observed at `main.tsx:4597` |
| `agentHook.timeout` | `60` seconds | describe |
| `plansDirectory` | `~/.claude/plans/` | describe |
| `feedbackSurveyRate` | `0` (not surveyed) | implied; `0.05` is a "reasonable starting point" per describe |
| `sshConfigs[].sshPort` | `22` | describe |
| `agentHook.model` | Haiku | describe |
| `promptHook.model` | "default small fast model" | describe |

### 6.8 `TIP_MATCHERS` — verbatim (`validationTips.ts:28-132`)

The full table is reproduced in source; key entries:

| Path / code match | Suggestion |
|---|---|
| `permissions.defaultMode` + `invalid_value` | `'Valid modes: "acceptEdits" (ask before file changes), "plan" (analysis only), "bypassPermissions" (auto-accept all), or "default" (standard behavior)'` + docLink `iam#permission-modes` |
| `apiKeyHelper` + `invalid_type` | `'Provide a shell command that outputs your API key to stdout. The script should output only the API key. Example: "/bin/generate_temp_api_key.sh"'` |
| `cleanupPeriodDays` + `too_small` + expected '0' | `'Must be 0 or greater. Set a positive number for days to retain transcripts (default is 30). Setting 0 disables session persistence entirely…'` |
| `env.*` + `invalid_type` | `'Environment variables must be strings. Wrap numbers and booleans in quotes. Example: "DEBUG": "true", "PORT": "3000"'` + docLink `settings#environment-variables` |
| `permissions.allow`/`deny` + `invalid_type` array | `'Permission rules must be in an array. Format: ["Tool(specifier)"]. Examples: ["Bash(npm run build)", "Edit(docs/**)", "Read(~/.zshrc)"]. Use * for wildcards.'` |
| `hooks` + `invalid_type` | `'Hooks use a matcher + hooks array. The matcher is a string: a tool name ("Bash"), pipe-separated list ("Edit|Write"), or empty to match all. Example: {"PostToolUse": [{"matcher": "Edit|Write", "hooks": [{"type": "command", "command": "echo Done"}]}]}'` |
| `invalid_type` + expected `'boolean'` | `'Use true or false without quotes. Example: "includeCoAuthoredBy": true'` |
| `unrecognized_keys` (any) | `'Check for typos or refer to the documentation for valid fields'` + docLink `/settings` |
| `invalid_value` + has enumValues (no other suggestion) | (auto) `'Valid values: "<v1>", "<v2>", ...'` |
| `invalid_type` + expected `object` + received `null` + path `''` | `'Check for missing commas, unmatched brackets, or trailing commas. Use a JSON validator to identify the exact syntax error.'` |
| `permissions.additionalDirectories` + `invalid_type` | `'Must be an array of directory paths. Example: ["~/projects", "/tmp/workspace"]. You can also use --add-dir flag or /add-dir command'` |

`PATH_DOC_LINKS` (`validationTips.ts:134-138`):
```
permissions → https://code.claude.com/docs/en/iam#configuring-permissions
env         → https://code.claude.com/docs/en/settings#environment-variables
hooks       → https://code.claude.com/docs/en/hooks
```

`DOCUMENTATION_BASE = 'https://code.claude.com/docs/en'` (`validationTips.ts:26`).

### 6.9 User-facing error strings — verbatim

All strings emitted at validation time:

From `formatZodError` (`validation.ts:139-161`):
- `'Invalid value. Expected one of: "v1", "v2", ...'`
- `'Invalid or malformed JSON'` (when expected `object`, received `null`, root path)
- `'Expected <type>, but received <type>'`
- `'Unrecognized field: <key>'` / `'Unrecognized fields: <key1>, <key2>'`
- `'Number must be greater than or equal to <minimum>'`

From `validatePermissionRule` (`permissionValidation.ts:65-238`):
- `'Permission rule cannot be empty'`
- `'Mismatched parentheses'` (suggestion: `'Ensure all opening parentheses have matching closing parentheses'`)
- `'Empty parentheses with no tool name'` (suggestion: `'Specify a tool name before the parentheses'`)
- `'Empty parentheses'` (suggestion: `'Either specify a pattern or use just "<toolName>" without parentheses'`)
- `'MCP rules do not support patterns in parentheses'`
- `'Tool name cannot be empty'`
- `'Tool names must start with uppercase'` (suggestion: `'Use "<Capitalized>"'`)
- `'The :* pattern must be at the end'`
- `'Prefix cannot be empty before :*'`
- `'The ":*" syntax is only for Bash prefix rules'`
- `'Wildcard placement might be incorrect'`
- (WebSearch) `'WebSearch does not support wildcards'`
- (WebFetch) `'WebFetch permissions use domain format, not URLs'`
- (WebFetch) `'WebFetch permissions must use "domain:" prefix'`

From `filterInvalidPermissionRules` (`validation.ts:240-258`):
- `'Non-string value in <allow|deny|ask> array was removed'`
- `'Invalid permission rule "<rule>" was skipped[: <error>][. <suggestion>]'`

From `validateInputForSettingsFileEdit` (`validateEditTool.ts:39`):
- `'Claude Code settings.json validation failed after edit:\n<errors>\n\nFull schema:\n<schema>\nIMPORTANT: Do not update the env unless explicitly instructed to do so.'`

From `extraKnownMarketplaces.check` (`types.ts:586-593`):
- ``Settings-sourced marketplace name must match its extraKnownMarketplaces key (got key "<key>" but source.name "<source.name>")``

From `AllowedMcpServerEntrySchema` / `DeniedMcpServerEntrySchema` (`types.ts:154-156`, `:204-206`):
- `'Server name can only contain letters, numbers, hyphens, and underscores'`
- `'Server command must have at least one element (the command)'`
- `'Entry must have exactly one of "serverName", "serverCommand", or "serverUrl"'`

From `parseSettingSourcesFlag` (`constants.ts:147`):
- `'Invalid setting source: <name>. Valid options are: user, project, local'`

From `updateSettingsForSource` (`settings.ts:460-462`, `:516-518`):
- `'Invalid JSON syntax in settings file at <path>'`
- `'Failed to read raw settings from <path>: <e>'`

### 6.10 Constants table

| Name | Value | Location |
|---|---|---|
| `CLAUDE_CODE_SETTINGS_SCHEMA_URL` | `'https://json.schemastore.org/claude-code-settings.json'` | `constants.ts:201-202` |
| `MACOS_PREFERENCE_DOMAIN` | `'com.anthropic.claudecode'` | `mdm/constants.ts:12` |
| `WINDOWS_REGISTRY_KEY_PATH_HKLM` | `'HKLM\\SOFTWARE\\Policies\\ClaudeCode'` | `mdm/constants.ts:23-24` |
| `WINDOWS_REGISTRY_KEY_PATH_HKCU` | `'HKCU\\SOFTWARE\\Policies\\ClaudeCode'` | `mdm/constants.ts:25-26` |
| `WINDOWS_REGISTRY_VALUE_NAME` | `'Settings'` | `mdm/constants.ts:29` |
| `PLUTIL_PATH` | `'/usr/bin/plutil'` | `mdm/constants.ts:32` |
| `PLUTIL_ARGS_PREFIX` | `['-convert','json','-o','-','--']` | `mdm/constants.ts:35` |
| `MDM_SUBPROCESS_TIMEOUT_MS` | `5000` | `mdm/constants.ts:38` |
| `FILE_STABILITY_THRESHOLD_MS` | `1000` | `changeDetector.ts:31` |
| `FILE_STABILITY_POLL_INTERVAL_MS` | `500` | `changeDetector.ts:38` |
| `INTERNAL_WRITE_WINDOW_MS` | `5000` | `changeDetector.ts:45` |
| `MDM_POLL_INTERVAL_MS` | `30 * 60 * 1000` (30 min) | `changeDetector.ts:51` |
| `DELETION_GRACE_MS` | `1000 + 500 + 200 = 1700` | `changeDetector.ts:62-63` |
| `TOOL_VALIDATION_CONFIG.filePatternTools` | `['Read','Write','Edit','Glob','NotebookRead','NotebookEdit']` | `toolValidationConfig.ts:28-35` |
| `TOOL_VALIDATION_CONFIG.bashPrefixTools` | `['Bash']` | `toolValidationConfig.ts:38` |
| `CUSTOMIZATION_SURFACES` | `['skills','agents','hooks','mcp']` | `types.ts:248-253` |
| `ADMIN_TRUSTED_SOURCES` | Set of `'plugin'`, `'policySettings'`, `'built-in'`, `'builtin'`, `'bundled'` | `pluginOnlyPolicy.ts:40-46` |
| `SETTING_SOURCES` | `['userSettings','projectSettings','localSettings','flagSettings','policySettings']` | `constants.ts:7-22` |
| `SOURCES` | `['localSettings','projectSettings','userSettings']` | `constants.ts:191-195` |

---

## 7. Side Effects & I/O

### 7.1 Filesystem reads

- `getFsImplementation().readdirSync(<managedFilePath>/managed-settings.d)` (filtered, sorted alphabetically) — `settings.ts:91-103`.
- `readFileSync` for every settings path (synchronous). `safeResolvePath` resolves symlinks (with safety checks; owned by `utils/fsOperations.ts`).
- `mkdirSync(dirname(filePath))` before writing — `settings.ts:434`.
- `existsSync(plistPath)` short-circuit before plutil spawn — `mdm/rawRead.ts:68`.
- `chokidar.watch` on the deduped parent directories of all sources except `flagSettings`.

### 7.2 Filesystem writes

- `writeFileSyncAndFlush_DEPRECATED(filePath, jsonStringify(updatedSettings, null, 2) + '\n')` — `settings.ts:500-503`. Uses two-space indentation, trailing newline.
- `addFileGlobRuleToGitignore('.claude/settings.local.json', originalCwd)` — fired async (no await) after writing local settings — `settings.ts:508-513`.

### 7.3 Network

None directly. `getRemoteManagedSettingsSyncFromCache()` reads an in-memory cache populated by spec 27.

### 7.4 Process spawn

- macOS: `execFile('/usr/bin/plutil', ['-convert','json','-o','-','--', plistPath], {timeout:5000})` — `mdm/rawRead.ts:71-74`.
- Windows: `execFile('reg', ['query', '<key>', '/v', 'Settings'])` — `mdm/rawRead.ts:91-104`.

Spawned at module evaluation in `main.tsx` via `startMdmRawRead()` for warm parallelism.

### 7.5 Environment variables consumed

| Env var | Where | Purpose |
|---|---|---|
| `process.env.USER_TYPE` | `types.ts:705,831,992`; `mdm/constants.ts:68`; `managedPath.ts:11`; `settings.ts:965`; `applySettingsChange.ts:51`; `migrateFennecToOpus.ts:19` | ANT-only feature gating |
| `process.env.CLAUDE_CODE_ENABLE_XAA` | `types.ts:284` | Toggles `xaaIdp` schema field |
| `process.env.CLAUDE_CODE_USE_COWORK_PLUGINS` | `settings.ts:267` | Selects `cowork_settings.json` |
| `process.env.CLAUDE_CODE_MANAGED_SETTINGS_PATH` | `managedPath.ts:13` (ANT only) | Override managed root |
| `process.env.CLAUDE_CODE_ENTRYPOINT` | `applySettingsChange.ts:53` | Skip ant overly-broad strip when `=== 'local-agent'` |

### 7.6 Trust boundaries

- `policySettings` is treated as more trusted than user/project/local because (a) MDM/registry/plist requires admin to write and (b) `managed-settings.json` lives in a system path. Schema fields like `pluginTrustMessage`, `allowManagedHooksOnly`, `allowManagedPermissionRulesOnly`, `allowManagedMcpServersOnly`, `strictPluginOnlyCustomization`, `strictKnownMarketplaces`, `blockedMarketplaces` are **only** honored from `policySettings` (per `.describe()` text, enforced at consumer sites).
- `projectSettings` is treated as **untrusted** for security-sensitive flags: `hasSkipDangerousModePermissionPrompt`, `hasAutoModeOptIn`, `getUseAutoModeDuringPlan`, `getAutoModeConfig` all explicitly **exclude** `projectSettings` (`settings.ts:882-928`). Comment at `:879-881`: a malicious project could otherwise auto-bypass the dialog (RCE risk).
- `autoMemoryDirectory` is silently ignored when set in `projectSettings` (per `.describe()` at `types.ts:944-949`).
- `pluginTrustMessage` is read **only** from policy settings (per `.describe()` at `types.ts:1062-1070`).

---

## 8. Feature Flags & Variants

### 8.1 Per-flag schema diff

| Flag | When ON | When OFF |
|---|---|---|
| `feature('TRANSCRIPT_CLASSIFIER')` | Adds `permissions.disableAutoMode`, `'auto'` to `permissions.defaultMode` enum, `skipAutoPermissionPrompt`, `useAutoModeDuringPlan`, `autoMode { allow, soft_deny, environment, deny? (ant) }`. `hasAutoModeOptIn`/`getUseAutoModeDuringPlan`/`getAutoModeConfig` return live values. `resetAutoModeOptInForDefaultOffer` runs. | All four schema fields absent (and stripped on parse if present in the file via `passthrough` keeping but not validating); helpers return `false`/`true`/`undefined`; migration is a no-op. |
| `feature('LODESTONE')` | Adds `disableDeepLinkRegistration: 'disable'`. | Field absent. |
| `feature('PROACTIVE') \|\| feature('KAIROS')` | Adds `minSleepDurationMs`, `maxSleepDurationMs`. | Fields absent. |
| `feature('VOICE_MODE')` | Adds `voiceEnabled`. | Field absent. |
| `feature('KAIROS')` | Adds `assistant`, `assistantName`. | Fields absent. |
| `feature('KAIROS') \|\| feature('KAIROS_BRIEF')` | Adds `defaultView`. | Field absent. |

### 8.2 ANT-only behavior

| Site | ANT diff |
|---|---|
| `effortLevel` enum | adds `'max'` |
| `xaaIdp` | requires `CLAUDE_CODE_ENABLE_XAA` (env, not USER_TYPE; orthogonal) |
| `classifierPermissionsEnabled` | added to schema |
| `autoMode.deny` | added (back-compat alias for `soft_deny`); folded into `soft_deny` at read time |
| `applySettingsChange` | re-strips `findOverlyBroadBashPermissions` after every settings reload |
| `mdm/constants.ts` plist paths | adds user-writable `~/Library/Preferences/com.anthropic.claudecode.plist` at lowest priority |
| `managedPath.ts` | honors `CLAUDE_CODE_MANAGED_SETTINGS_PATH` override |
| `migrateFennecToOpus` | runs only for ANT |

### 8.3 Env/runtime gates that aren't `feature()`

- `process.env.USER_TYPE === 'ant'` (see §8.2).
- `isEnvTruthy(process.env.CLAUDE_CODE_ENABLE_XAA)` — toggles `xaaIdp` schema slot.
- `isEnvTruthy(process.env.CLAUDE_CODE_USE_COWORK_PLUGINS)` (and CLI `--cowork`) — switches user settings filename.
- `process.env.CLAUDE_CODE_MANAGED_SETTINGS_PATH` (only honored when ANT).
- `process.env.CLAUDE_CODE_ENTRYPOINT !== 'local-agent'` — guards ant overly-broad strip.

---

## 9. Error Handling & Edge Cases

### 9.1 Failure modes and recovery

| Failure | Recovery |
|---|---|
| Settings file missing (ENOENT) | Logged at debug as "Broken symlink or missing file…", returns `{settings: null, errors: []}` (no error surfaced) — `settings.ts:163-165`. |
| Empty file | Returns `{settings: {}, errors: []}` — `settings.ts:209`. |
| Malformed JSON | `safeParseJSON` returns null + logs; `parseSettingsFileUncached` returns `{settings: null, errors: []}` (no error). On `updateSettingsForSource`, returns explicit error `'Invalid JSON syntax in settings file at <path>'`. |
| Zod validation failure | Returns `{settings: null, errors: <formatted>}`. **File is preserved** — see `types.ts:240-241` and `settings.ts:233-249` rationale: invalid fields stay on disk, valid fields are picked up; on next write, `updateSettingsForSource` falls back to raw parse to preserve unknown/invalid fields. |
| Single bad permission rule | Removed by `filterInvalidPermissionRules` **before** schema validation; emitted as a `ValidationError` warning; rest of the file validates normally. |
| Drop-in dir missing/not a dir | `loadManagedFileSettings` swallows `ENOENT`/`ENOTDIR`, logs others — `settings.ts:113-117`. |
| MDM subprocess timeout | `execFile` returns code !==0; result drops to `''` and is treated as non-existent. 5000ms cap. |
| File watcher EOPNOTSUPP (special files) | Filtered via `ignored` function — `changeDetector.ts:113-117`. |
| Delete-and-recreate (auto-updater) | `DELETION_GRACE_MS=1700` absorbs; cancels pending deletion if `add`/`change` fires within window. |
| Internal-write echo | `markInternalWrite` + `consumeInternalWrite(path, 5000)` — silent skip. |
| `projectSettings` poisoning sensitive flag | Deliberately excluded from `hasSkipDangerousModePermissionPrompt`, `hasAutoModeOptIn`, `getUseAutoModeDuringPlan`, `getAutoModeConfig`. |
| `extraKnownMarketplaces` key/name mismatch (settings-sourced) | Custom Zod `check` emits issue path `[key, 'source', 'name']`. |
| `strictPluginOnlyCustomization` non-array invalid value | `.catch(undefined)` drops the field rather than nulling the whole managed-settings file. |
| `strictPluginOnlyCustomization` array with unknown member | `preprocess` filters unknowns rather than rejecting (forwards-compat). |
| Migration throws | All migrations log via `logError` and emit a `*_error` analytics event; do not rethrow. |
| Schema generation errors | `toJSONSchema(SettingsSchema(), { unrepresentable: 'any' })` — see §10. |

### 9.2 Schema versioning notice (verbatim, `types.ts:212-241`)

The schema enforces backward compatibility documentation. Verbatim notice is inlined in §6.1 above (the "BACKWARD COMPATIBILITY NOTICE" comment block).

---

## 10. Telemetry & Observability

### 10.1 `logEvent` analytics events (cited)

| Event | Payload | Source |
|---|---|---|
| `tengu_migrate_autoupdates_to_settings` | `{was_user_preference: true, already_had_env_var: boolean}` | `migrateAutoUpdatesToSettings.ts:38` |
| `tengu_migrate_autoupdates_error` | `{has_error: true}` | `migrateAutoUpdatesToSettings.ts:57` |
| `tengu_migrate_bypass_permissions_accepted` | `{}` | `migrateBypassPermissionsAcceptedToSettings.ts:28` |
| `tengu_migrate_mcp_approval_fields_success` | `{migratedCount: number}` | `migrateEnableAllProjectMcpServersToSettings.ts:110` |
| `tengu_migrate_mcp_approval_fields_error` | `{}` | `migrateEnableAllProjectMcpServersToSettings.ts:116` |
| `tengu_legacy_opus_migration` | `{from_model: <model string>}` | `migrateLegacyOpusToCurrent.ts:53` |
| `tengu_opus_to_opus1m_migration` | `{}` | `migrateOpusToOpus1m.ts:42` |
| `tengu_sonnet45_to_46_migration` | `{from_model: <model string>, has_1m: boolean}` | `migrateSonnet45ToSonnet46.ts:62` |
| `tengu_migrate_reset_auto_opt_in_for_default_offer` | `{}` | `resetAutoModeOptInForDefaultOffer.ts:40` |
| `tengu_reset_pro_to_opus_default` | `{skipped: boolean, had_custom_model?: boolean}` | `resetProToOpusDefault.ts:22, 36, 46` |

(Note: `migrateSonnet1mToSonnet45`, `migrateFennecToOpus`, `migrateReplBridgeEnabledToRemoteControlAtStartup` do **not** emit analytics — only state mutation.)

### 10.2 `logForDiagnosticsNoPII` events

| Event | Payload | Source |
|---|---|---|
| `settings_load_started` | (none) | `settings.ts:653` |
| `settings_load_completed` | `{duration_ms, source_count, error_count}` | `settings.ts:786-790` |
| `mdm_settings_loaded` | `{duration_ms, key_count, error_count}` | `mdm/settings.ts:88-92` |

### 10.3 Profile checkpoints

`profileCheckpoint('loadSettingsFromDisk_start' / 'loadSettingsFromDisk_end' / 'mdm_load_start' / 'mdm_load_end')` — `settings.ts:652,865`; `mdm/settings.ts:70,79`.

### 10.4 Debug logs

- `'Settings changed from <source>, updating app state'` — `applySettingsChange.ts:39`.
- `'Detected change to <path>'`, `'Detected deletion of <path>'`, `'Cancelled pending deletion of <path> — file was recreated'` / `'… re-added'` — `changeDetector.ts:278-318, 334`.
- `'Detected MDM settings change via poll'` — `changeDetector.ts:407`.
- `'Programmatic settings change notification for <source>'` — `changeDetector.ts:448`.
- `'Watching for changes in setting files <list>… and drop-in directory <dir>'` — `changeDetector.ts:99-101`.
- `'MDM poll error: <msg>'` — `changeDetector.ts:411`.
- `'MDM settings load completed in <ms>ms'`, `'MDM settings found: <keys>'` — `mdm/settings.ts:82-86`.
- `'[auto-mode] hasAutoModeOptIn=<bool> skipAutoPermissionPrompt: user=<v> local=<v> flag=<v> policy=<v>'` — `settings.ts:905-907`.

### 10.5 OpenTelemetry spans

None directly. Diagnostic logs above flow through `logForDiagnosticsNoPII` (spec 26).

---

## 11. Reimplementation Checklist

A reimplementer must preserve, in order of importance:

1. **Five named sources** with the exact merge order: `userSettings → projectSettings → localSettings → flagSettings → policySettings`. `pluginSettingsBase` merges below all five. Policy and flag are always loaded; the rest filter through `getAllowedSettingSources()`.
2. **Policy "first source wins"** with priority remote → admin MDM (HKLM/plist) → file (`managed-settings.json` + `managed-settings.d/*.json` sorted alphabetically) → HKCU.
3. **Drop-in convention**: alphabetical sort, base merged first, drop-ins on top using `settingsMergeCustomizer` (array concat-dedupe, default object merge).
4. **Disk paths**: `~/.claude/settings.json` (or `cowork_settings.json`); `<cwd>/.claude/settings.json`; `<cwd>/.claude/settings.local.json` (auto-gitignored on first write); `<flagSettingsPath>` (from `--settings`); platform-specific managed roots.
5. **Caching**: three caches (`sessionSettingsCache`, `perSourceCache`, `parseFileCache`) all flushed by `resetSettingsCache()`. Single `fanOut` resets cache before notifying listeners.
6. **Asymmetric merge customizer**: read path concatenates+dedupes arrays; write path replaces arrays and treats `undefined` srcValue as deletion.
7. **Settings file write atomicity**: `writeFileSyncAndFlush`, two-space indent, trailing newline, `markInternalWrite` precedes the write, `resetSettingsCache` follows.
8. **Permission rule pre-filter** mutates the parsed object in place before Zod sees it; the warnings are returned alongside.
9. **Permission rule validator** decision tree exactly as in §5.9, with escape-aware paren counting.
10. **Hook schemas** are a 4-arm discriminated union on `type`. The `agent` arm must NOT use `.transform()` (gh-24920).
11. **Migration order** exactly as in §5.10. Idempotency mechanisms differ per migration; preserve each.
12. **MDM startup prefetch** must run before heavy module imports (`startMdmRawRead`). `existsSync` short-circuit before `plutil` is required to keep total spawn-during-imports time low.
13. **MDM 30-min poll** with `unref()`'d timer. Update `mdmCache`/`hkcuCache` only on detected change.
14. **File watcher**: chokidar `awaitWriteFinish` (1000ms threshold, 500ms poll), `depth:0`, ignore non-files/directories and `.git`. Skip `flagSettings` entirely. `INTERNAL_WRITE_WINDOW_MS=5000`. `DELETION_GRACE_MS=1700`. Custom `ignored` function only allows known settings filenames + `.json` files in drop-in dir.
15. **`ConfigChange` hook** fires before applying any change; if `hasBlockingResult`, skip applying.
16. **Trust boundaries**: `projectSettings` excluded from `hasSkipDangerousModePermissionPrompt`, `hasAutoModeOptIn`, `getUseAutoModeDuringPlan`, `getAutoModeConfig`. `pluginTrustMessage` only from policy. `autoMemoryDirectory` ignored from project.
17. **`strictPluginOnlyCustomization`** with forwards-compat preprocess (drop unknown surface names) and `.catch(undefined)` (drop entirely on non-array invalid).
18. **`extraKnownMarketplaces`** key/source.name match check (settings-sourced only).
19. **MCP entry exclusivity** check: exactly one of serverName / serverCommand / serverUrl per entry.
20. **`SettingsSchema().passthrough()`** at outer object and `permissions.passthrough()` — preserves unknown fields rather than rejecting (per `types.ts:240-241`). Strict-mode is only used in `validateSettingsFileContent` (FileEdit guard).
21. **Bug-fix invariants**:
   - Always `clone()` before returning from `parseSettingsFile` to prevent caller mutation of cached entries.
   - `fanOut` does cache reset (single producer); listeners must not.
   - `applySettingsChange` propagates `effortValue` only on change to defined value.

The spec is complete when a reimplementer can rebuild a Claude-Code-compatible settings layer that produces identical merged output, identical validation errors (verbatim strings), identical migration mutations, and identical telemetry events, given the same on-disk inputs and feature-flag matrix.

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. RESOLVED items cite where the answer landed.

1. **`HOOK_EVENTS` enum verbatim** — **DEFERRED**: source is bundled in `src/entrypoints/agentSdkTypes.js` (not present in plain form). The 13-event allowlist in `getManagedSettingsKeysForLogging` (`settings.ts:594-608`) is the observed lower bound: `PreToolUse, PostToolUse, Notification, UserPromptSubmit, SessionStart, SessionEnd, Stop, SubagentStop, PreCompact, PostCompact, TeammateIdle, TaskCreated, TaskCompleted`. Spec 09 §4 documents the same set as authoritative observed.
2. **Default-mode value selection inside `permissions.defaultMode`** — *unchanged drift report*: Tip text at `validationTips.ts:29-37` lists 4 modes; schema has 5. Reported as drift, not corrected. (See spec 00 §6.2 which now documents the two-tier `bubble` classification — separate concern.)
3. ~~**`getAllowedSettingSources()` initial set**~~ — **RESOLVED Phase 9.7**: spec 01 §6 confirms default returns `['userSettings','projectSettings','localSettings']` (matching `parseSettingSourcesFlag` accept set).
4. ~~**`addFileGlobRuleToGitignore` semantics**~~ — **RESOLVED Phase 9.7**: implementation at `src/utils/git/gitignore.ts:53` (async, fire-and-forget). Fired without await on local-settings writes (`settings.ts:510`); failure logs but does not block the settings write. Spec 41 owns the gitignore utility surface.
5. **`getRemoteManagedSettingsSyncFromCache` cache lifetime** — *consumer-only here*: this spec reads from it; spec 27 owns. "First source wins" makes remote always preempt MDM/file when present.
6. **Cowork mode behavior for `projectSettings` / `localSettings`** — *cited for clarity, not a question*: only `getUserSettingsFilePath` switches to `cowork_settings.json`.
7. **`isLegacyModelRemapEnabled()`, `isOpus1mMergeEnabled()`, `getDefaultMainLoopModelSetting()`** — **RESOLVED Phase 9.7**: live in `src/utils/model/` (now enumerated in spec 42a §3, per Phase 10b). Spec 22 consumes these.
8. **`ConfigChangeSource` enum collapses `flagSettings` into `'policy_settings'`** — *unchanged*: owned by spec 09 (config-change hooks); whether downstream consumers can re-distinguish is a spec-09 question.
9. **`safeResolvePath`** — *consumer-only*: owned by `utils/fsOperations.ts` (spec 42a long-tail).
10. **`startMdmSettingsLoad` vs `startMdmRawRead`** — *cross-spec*: spec 01 owns entry-point ordering. Confirmed compatible here.
11. **Overview precedence claim** — *historical drift report; resolved by Phase 9.6 fixes to 00 §5.1*. Cited for trace.
12. **`useAutoModeDuringPlan` default** — *invariant noted, not a question*: documented behavior — undefined/missing/true → "true". Preserve current implementation.
13. **`autoUpdatesChannel` default** — *consumer fallback authoritative*: `main.tsx:4597` uses `?? 'latest'`. Not a question.
