# 28 — Service: Plugins (manifest loading and validation)

## 1. Purpose & Scope

This spec covers the plugin subsystem: **how plugin manifests are discovered, validated against Zod schemas, materialized to a versioned cache, and surfaced to the rest of the harness as `LoadedPlugin` objects**. The primary entry point is `loadAllPluginsCacheOnly()` (consumed by `QueryEngine.ts:67`). Plugins contribute commands, skills, agents, hooks, MCP servers, LSP servers, output styles, settings, and user-config — this spec owns *how plugins are loaded*; consumers own *what they do with the loaded surface*.

In scope:
- `src/plugins/builtinPlugins.ts` (built-in plugin registry, `BUILTIN_MARKETPLACE_NAME = 'builtin'`)
- `src/plugins/bundled/index.ts` (registration site)
- `src/services/plugins/PluginInstallationManager.ts` (background marketplace reconciliation)
- `src/services/plugins/pluginCliCommands.ts`, `pluginOperations.ts` (CLI install/enable/disable surface)
- `src/utils/plugins/` — manifest schemas, loader, marketplace manager, identifier parser, cache, seed dirs, dependency resolver, refresh
- The `loadAllPlugins` / `loadAllPluginsCacheOnly` boundary
- Plugin sources (relative path inside marketplace, npm, pip, github, git url, git-subdir)
- Marketplace sources (url, github, git, npm, file, directory, hostPattern, pathPattern, settings)
- Settings layer (`pluginSettingsBase` write-side; precedence machinery is spec 02)
- Env: `CLAUDE_CODE_SYNC_PLUGIN_INSTALL`, `CLAUDE_CODE_PLUGIN_SEED_DIR`, `CLAUDE_CODE_PLUGIN_CACHE_DIR`, `CLAUDE_CODE_USE_COWORK_PLUGINS`, `CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS`

Out of scope (refer):
- Settings precedence resolution → 02 (this spec writes `pluginSettingsBase`; 02 explains how it cascades).
- Skill matching / `SkillTool` semantics → 17. This spec produces `pluginSkills`/`builtinPluginSkills`/`pluginCommands`; 17 documents how they are consumed.
- Slash-command registration order → 20; catalog → 21.
- MCP server connection lifecycle → 23. This spec covers manifest validation of `mcpServers` declarations; 23 covers how they connect.
- Analytics events shape → 26.

## 2. Source Map

### 2.1 Owned files

| File | Lines | Read mode |
|---|---|---|
| `src/plugins/builtinPlugins.ts` | 159 | full |
| `src/plugins/bundled/index.ts` | — | grep |
| `src/services/plugins/PluginInstallationManager.ts` | 184 | full |
| `src/services/plugins/pluginCliCommands.ts` | 344 | grep |
| `src/services/plugins/pluginOperations.ts` | 1088 | grep |
| `src/utils/plugins/schemas.ts` | 1681 | full |
| `src/utils/plugins/pluginIdentifier.ts` | 123 | full |
| `src/utils/plugins/pluginDirectories.ts` | 178 | full |
| `src/utils/plugins/officialMarketplace.ts` | 25 | full |
| `src/utils/plugins/officialMarketplaceGcs.ts` | 216 | full |
| `src/utils/plugins/pluginLoader.ts` | 3302 | sampled by section |
| `src/utils/plugins/marketplaceManager.ts` | 2643 | grep + targeted reads |
| `src/utils/plugins/loadPluginHooks.ts` | 287 | grep |
| `src/utils/plugins/loadPluginCommands.ts` | 946 | grep |
| `src/utils/plugins/loadPluginAgents.ts` | 348 | grep |
| `src/utils/plugins/loadPluginOutputStyles.ts` | 178 | grep |
| `src/utils/plugins/mcpPluginIntegration.ts` | 634 | grep |
| `src/utils/plugins/lspPluginIntegration.ts` | 387 | grep |
| `src/utils/plugins/mcpbHandler.ts` | 968 | grep |
| `src/utils/plugins/installedPluginsManager.ts` | 1268 | grep |
| `src/utils/plugins/refresh.ts` | 215 | grep |
| `src/utils/plugins/reconciler.ts` | 265 | grep |
| `src/utils/plugins/dependencyResolver.ts` | 305 | grep |
| `src/utils/plugins/cacheUtils.ts` | 196 | full |
| `src/utils/plugins/zipCache.ts` | 406 | grep |
| `src/utils/plugins/headlessPluginInstall.ts` | 174 | grep |
| `src/utils/plugins/pluginVersioning.ts` | 157 | grep |
| `src/utils/plugins/managedPlugins.ts` | 27 | full |
| `src/utils/plugins/installCounts.ts` | 292 | grep |
| `src/utils/plugins/pluginAutoupdate.ts` | 284 | grep |
| `src/utils/plugins/pluginBlocklist.ts` | 127 | grep |
| `src/utils/plugins/pluginPolicy.ts` | 20 | grep |
| `src/utils/plugins/marketplaceHelpers.ts` | 592 | grep |
| `src/utils/plugins/orphanedPluginFilter.ts` | 114 | grep |
| `src/utils/plugins/pluginInstallationHelpers.ts` | 595 | grep |
| `src/utils/plugins/pluginOptionsStorage.ts` | 400 | grep |
| `src/utils/plugins/pluginStartupCheck.ts` | 341 | grep |
| `src/utils/plugins/officialMarketplaceStartupCheck.ts` | 439 | grep |
| `src/utils/plugins/walkPluginMarkdown.ts` | 69 | grep |
| `src/utils/plugins/zipCacheAdapters.ts` | 164 | grep |
| `src/utils/plugins/parseMarketplaceInput.ts` | 162 | grep |
| `src/utils/plugins/addDirPluginSettings.ts` | 71 | grep |
| `src/utils/plugins/dependencyResolver.ts` | 305 | grep |
| `src/types/plugin.ts` | 364 | full |

### 2.2 Imports from

`zod/v4`, `lodash-es/memoize`, `axios`, `fs/promises`, `path`, `src/schemas/hooks.js` (`HooksSchema`), `src/services/mcp/types.js` (`McpServerConfigSchema`), `src/utils/lazySchema.js`, `src/utils/settings/{settings,settingsCache,types}.js`, `src/services/lsp/types.js`, `src/skills/bundledSkills.js`, `src/bootstrap/state.js`, `src/utils/dxt/zip.js`.

### 2.3 Imported by

- `src/QueryEngine.ts:67` — `loadAllPluginsCacheOnly`
- `src/commands.ts:355,357,360,377,383,385,393,395,451,452,462,465,466` — `pluginCommands`, `pluginSkills`, `builtinPluginSkills`, `getSkills` ordering
- `src/setup.ts:309-317` — `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` plugin-prefetch decision
- `src/services/lsp/{config,manager}.ts`, `src/services/mcp/{config,useManageMCPConnections}.ts` — plugin-loaded LSP/MCP servers
- `src/cli/handlers/plugins.ts`, `src/cli/print.ts:1733-1897`
- `src/hooks/useManagePlugins.ts`, `src/hooks/notifs/usePluginInstallationStatus.tsx`
- `src/utils/settings/settingsCache.ts` — read/write of `pluginSettingsBase`
- `src/tools/AgentTool/built-in/claudeCodeGuideAgent.ts`

### 2.4 Feature flags / ANT

This subsystem has no `feature(...)` gates and no `USER_TYPE === 'ant'` branches. It is fully active in all builds.

### 2.5 Missing-leaked-source

None observed for this subsystem. `src/plugins/bundled/index.ts` exists and registers built-in plugins via `registerBuiltinPlugin()`.

---

## 3. Public Interface

### 3.1 Loader entry points (verbatim signatures)

```ts
// src/utils/plugins/pluginLoader.ts:3096, :3137
export const loadAllPlugins = memoize(async (): Promise<PluginLoadResult> => { ... })
export const loadAllPluginsCacheOnly = memoize(async (): Promise<PluginLoadResult> => { ... })

// :3225
export function clearPluginCache(reason?: string): void

// :3009
export function mergePluginSources(sources: {
  session: LoadedPlugin[]
  marketplace: LoadedPlugin[]
  builtin: LoadedPlugin[]
  managedNames?: Set<string> | null
}): { plugins: LoadedPlugin[]; errors: PluginError[] }

// :1147
export async function loadPluginManifest(
  manifestPath: string,
  pluginName: string,
  source: string,
): Promise<PluginManifest>

// :1348
export async function createPluginFromPath(
  pluginPath: string,
  source: string,
  enabled: boolean,
  fallbackName: string,
  strict?: boolean,
): Promise<{ plugin: LoadedPlugin; errors: PluginError[] }>

// :3281
export function cachePluginSettings(plugins: LoadedPlugin[]): void

// :126, :139, :172, :183, :249
export function getPluginCachePath(): string
export function getVersionedCachePathIn(baseDir, pluginId, version): string
export function getVersionedCachePath(pluginId, version): string
export function getVersionedZipCachePath(pluginId, version): string
export function getLegacyCachePath(pluginName): string
```

### 3.2 Identifier helpers (`pluginIdentifier.ts`)

```ts
// :37-41
export type ParsedPluginIdentifier = { name: string; marketplace?: string }

// :51-57
export function parsePluginIdentifier(plugin: string): ParsedPluginIdentifier {
  if (plugin.includes('@')) {
    const parts = plugin.split('@')
    return { name: parts[0] || '', marketplace: parts[1] }
  }
  return { name: plugin }
}

// :65-67
export function buildPluginId(name: string, marketplace?: string): string {
  return marketplace ? `${name}@${marketplace}` : name
}

// :75-82
export function isOfficialMarketplaceName(marketplace: string | undefined): boolean

// :104-111, :119-123
export function scopeToSettingSource(scope: PluginScope): EditableSettingSource
export function settingSourceToScope(source: EditableSettingSource): Exclude<PluginScope,'managed'>

// :14, :20, :26-32
export type ExtendedPluginScope = PluginScope | 'flag'
export type PersistablePluginScope = Exclude<ExtendedPluginScope,'flag'>
export const SETTING_SOURCE_TO_SCOPE = {
  policySettings: 'managed', userSettings: 'user', projectSettings: 'project',
  localSettings: 'local', flagSettings: 'flag',
} as const
```

Identifier semantics: only the FIRST `@` is the separator; later `@` segments (e.g., `plugin@market@^1.2`) are silently truncated by upstream `DependencyRefSchema`'s `.replace(/@\^[^@]*$/,'')` transform — see schema below.

### 3.3 Built-in plugin API (`builtinPlugins.ts`)

```ts
// :23
export const BUILTIN_MARKETPLACE_NAME = 'builtin'

// :28-32
export function registerBuiltinPlugin(definition: BuiltinPluginDefinition): void
// :37-39
export function isBuiltinPluginId(pluginId: string): boolean  // endsWith `@builtin`
// :46-50
export function getBuiltinPluginDefinition(name: string): BuiltinPluginDefinition | undefined
// :57-102
export function getBuiltinPlugins(): { enabled: LoadedPlugin[]; disabled: LoadedPlugin[] }
// :108-121
export function getBuiltinPluginSkillCommands(): Command[]
// :126-128
export function clearBuiltinPlugins(): void
```

### 3.4 Marketplace manager (cite into `marketplaceManager.ts`)

`registerSeedMarketplaces()` (`:380`), `loadKnownMarketplacesConfig()` (`:264`), `loadKnownMarketplacesConfigSafe()` (`:309`), `saveKnownMarketplacesConfig()` (`:327`), `getMarketplaceCacheOnly(name)` (`:2081`), `getPluginByIdCacheOnly(pluginId)` (`:2188`), `clearMarketplacesCache()` (`:122`), `getDeclaredMarketplaces()` (`:161`), `getMarketplaceDeclaringSource()` (`:200`), `saveMarketplaceToSettings()` (`:226`).

### 3.5 Refresh / install lifecycle

- `refreshActivePlugins(setAppState): Promise<RefreshActivePluginsResult>` — `refresh.ts:72`
- `performBackgroundPluginInstallations(setAppState)` — `PluginInstallationManager.ts:60`
- `reconcileMarketplaces({onProgress})` — `reconciler.ts`
- `runHeadlessPluginInstall(...)` — `headlessPluginInstall.ts`

`pluginOperations.ts` (1088 LOC) public lifecycle entry points — invoked by `/plugin` slash command and headless install paths:

- `installPluginOp(...)` — `:321`
- `uninstallPluginOp(...)` — `:427`
- `setPluginEnabledOp(...)` — `:573`
- `enablePluginOp(...)` — `:756` (thin wrapper over `setPluginEnabledOp`)
- `disablePluginOp(...)` — `:770` (thin wrapper)
- `disableAllPluginsOp()` — `:782`
- `updatePluginOp(...)` — `:829` (delegates to internal `performPluginUpdate` at `:896`)

Each returns a `PluginOperationResult` (`:141`) or `PluginUpdateResult` (`:154`). Scope helpers: `assertInstallableScope()` `:90`, `isInstallableScope()` `:104`, `getProjectPathForScope()` `:114`, `isPluginEnabledAtProjectScope()` `:128`. Constants: `VALID_INSTALLABLE_SCOPES` `:72` (`['user','project','local']`), `VALID_UPDATE_SCOPES` `:78`.

### 3.6 Settings-cache integration (`settingsCache.ts`)

`getPluginSettingsBase()`, `setPluginSettingsBase()`, `clearPluginSettingsBase()`, `resetSettingsCache()` — read by 02's settings cascade.

---

## 4. Data Model & State

### 4.1 LoadedPlugin (`types/plugin.ts:48-70` verbatim)

```ts
export type LoadedPlugin = {
  name: string
  manifest: PluginManifest
  path: string
  source: string
  repository: string  // Repository identifier, usually same as source
  enabled?: boolean
  isBuiltin?: boolean
  sha?: string
  commandsPath?: string
  commandsPaths?: string[]
  commandsMetadata?: Record<string, CommandMetadata>
  agentsPath?: string
  agentsPaths?: string[]
  skillsPath?: string
  skillsPaths?: string[]
  outputStylesPath?: string
  outputStylesPaths?: string[]
  hooksConfig?: HooksSettings
  mcpServers?: Record<string, McpServerConfig>
  lspServers?: Record<string, LspServerConfig>
  settings?: Record<string, unknown>
}

export type PluginComponent =
  | 'commands' | 'agents' | 'skills' | 'hooks' | 'output-styles'

export type PluginLoadResult = {
  enabled: LoadedPlugin[]; disabled: LoadedPlugin[]; errors: PluginError[]
}
```

`BuiltinPluginDefinition` (`types/plugin.ts:18-35`): `{ name, description, version?, skills?, hooks?, mcpServers?, isAvailable?, defaultEnabled? }`.

### 4.2 On-disk state

- **Plugins directory**: `~/.claude/plugins/` (or `cowork_plugins/`); override `CLAUDE_CODE_PLUGIN_CACHE_DIR`. Resolved by `getPluginsDirectory()` (`pluginDirectories.ts:53-63`) with `expandTilde` for settings-injected `~/...` values.
- **Marketplace cache**: `<plugins>/cache/` per `getPluginCachePath()` (`pluginLoader.ts:126-128`).
- **Versioned plugin cache**: `<plugins>/cache/{marketplace}/{plugin}/{version}/` (`pluginLoader.ts:139-162`). Sanitizers strip non-`[a-zA-Z0-9\-_]` from marketplace/plugin names; version sanitizer also keeps `.`.
- **Per-plugin data dir** (`${CLAUDE_PLUGIN_DATA}`): `<plugins>/data/<sanitized pluginId>/` — sanitized via `[^a-zA-Z0-9\-_] → '-'` (`pluginDirectories.ts:92-99`). Survives plugin updates; deleted only on last-scope uninstall (`deletePluginDataDir`, `:168-178`).
- **Seed dirs** (`CLAUDE_CODE_PLUGIN_SEED_DIR`): platform-`delimiter`-split, precedence-ordered, `expandTilde`'d (`pluginDirectories.ts:85-90`). Read-only fallback layer mirroring the primary plugins directory.
- **`installed_plugins.json`**: V1 (`{ version: 1, plugins: Record<PluginId, InstalledPlugin> }`) and V2 (`{ version: 2, plugins: Record<PluginId, PluginInstallationEntry[]> }`) — schema `schemas.ts:1446-1577`. V2 supports per-scope multi-installation (one plugin can be installed at user *and* project scope with different versions).
- **`known_marketplaces.json`**: `Record<MarketplaceName, KnownMarketplace>` per `KnownMarketplacesFileSchema` (`schemas.ts:1592-1629`).
- **`.gcs-sha`**: sentinel file at the official-marketplace install root holding the last extracted SHA (`officialMarketplaceGcs.ts:94-101`).
- **`.orphaned_at`**: per-versioned-cache marker; cleanup age = 7 days (`cacheUtils.ts:23-24`).

### 4.3 Memoization

- `loadAllPlugins` and `loadAllPluginsCacheOnly` use `lodash-es/memoize` independently (`pluginLoader.ts:3096`, `:3137`). A successful `loadAllPlugins` populates the cache-only memo: `loadAllPluginsCacheOnly.cache?.set(undefined, Promise.resolve(result))` (`:3106`). The reverse is NOT done — a cache-only result MUST NOT satisfy a fresh-source caller (`:3131-3135`).
- `loadPluginHooks` (`loadPluginHooks.ts:91`), `getPluginCommands` (`loadPluginCommands.ts:414`), `getPluginSkills` (`:840`) are independently memoized; cleared via `clearPluginHookCache`, `clearPluginCommandCache`, `clearPluginSkillsCache`.
- `getMarketplace` is memoized inside `marketplaceManager.ts`; cleared via `clearMarketplacesCache()`.

---

## 5. Algorithm / Control Flow

### 5.1 Top-level: `loadAllPluginsCacheOnly()` — `pluginLoader.ts:3137-3146`

```
if (isEnvTruthy(CLAUDE_CODE_SYNC_PLUGIN_INSTALL)) return loadAllPlugins()
return assemblePluginLoadResult(() => loadPluginsFromMarketplaces({cacheOnly: true}))
```

### 5.2 `assemblePluginLoadResult()` — `:3155-3211`

```
parallel:
  marketplaceResult = await marketplaceLoader()
  sessionResult     = inlinePlugins.length>0
                       ? loadSessionOnlyPlugins(getInlinePlugins())
                       : {plugins:[],errors:[]}
builtinResult = getBuiltinPlugins()                          // sync
{plugins, errors:mergeErrors} = mergePluginSources({
  session: sessionResult.plugins,
  marketplace: marketplaceResult.plugins,
  builtin: [...builtinResult.enabled, ...builtinResult.disabled],
  managedNames: getManagedPluginNames(),
})
allErrors = [...marketplaceResult.errors, ...sessionResult.errors, ...mergeErrors]

{demoted, errors:depErrors} = verifyAndDemote(plugins)
for p in plugins: if (demoted.has(p.source)) p.enabled = false
allErrors.push(...depErrors)

enabled  = plugins.filter(p => p.enabled)
disabled = plugins.filter(p => !p.enabled)
cachePluginSettings(enabled)                                  // → pluginSettingsBase
return {enabled, disabled, errors: allErrors}
```

### 5.3 `loadPluginsFromMarketplaces({cacheOnly})` — `:1888-2089`

```
settings = getSettings_DEPRECATED()
enabledPlugins = { ...getAddDirEnabledPlugins(), ...settings.enabledPlugins }
marketplacePluginEntries = entries(enabledPlugins).filter(([k,v]) =>
  PluginIdSchema.safeParse(k).success
  && v !== undefined
  && parsePluginIdentifier(k).marketplace !== BUILTIN_MARKETPLACE_NAME)
knownMarketplaces = await loadKnownMarketplacesConfigSafe()      // {} on corruption
strictAllowlist = getStrictKnownMarketplaces()                    // null|Source[]
blocklist       = getBlockedMarketplaces()                        // null|Source[]
hasEnterprisePolicy = strictAllowlist !== null
                    || (blocklist !== null && blocklist.length > 0)
uniqueMarketplaces = unique marketplace name from entries
for each name in uniqueMarketplaces (parallel):
  marketplaceCatalogs[name] = await getMarketplaceCacheOnly(name)
installedPluginsData = getInMemoryInstalledPlugins()
results = Promise.allSettled(marketplacePluginEntries.map(async ([pluginId, enabledVal]) => {
  {name, marketplace} = parsePluginIdentifier(pluginId)
  marketplaceConfig = knownMarketplaces[marketplace]
  // Fail-closed: unknown marketplace + active policy → block
  if (!marketplaceConfig && hasEnterprisePolicy) {
    errorsOut.push({type:'marketplace-blocked-by-policy', source:pluginId,
      plugin:name, marketplace, blockedByBlocklist: strictAllowlist===null,
      allowedSources: (strictAllowlist??[]).map(formatSourceForDisplay)})
    return null
  }
  if (marketplaceConfig && !isSourceAllowedByPolicy(marketplaceConfig.source)) {
    isBlocked = isSourceInBlocklist(marketplaceConfig.source)
    errorsOut.push({type:'marketplace-blocked-by-policy', source:pluginId,
      plugin:name, marketplace,
      blockedByBlocklist: isBlocked,
      allowedSources: isBlocked ? [] :
        (getStrictKnownMarketplaces()??[]).map(formatSourceForDisplay)})
    return null
  }
  // Resolve marketplace entry from pre-loaded catalog OR fallback
  if (marketplace catalog && marketplaceConfig) {
    entry = marketplace.plugins.find(p => p.name === pluginName)
    result = entry ? {entry, marketplaceInstallLocation: marketplaceConfig.installLocation} : null
  } else {
    result = await getPluginByIdCacheOnly(pluginId)             // raw cast, no schema
  }
  if (!result) {
    errorsOut.push({type:'plugin-not-found', source:pluginId,
                    pluginId:name, marketplace})
    return null
  }
  installEntry = installedPluginsData.plugins[pluginId]?.[0]
  return cacheOnly
    ? loadPluginFromMarketplaceEntryCacheOnly(result.entry,
        result.marketplaceInstallLocation, pluginId,
        enabledVal === true, errorsOut, installEntry?.installPath)
    : loadPluginFromMarketplaceEntry(result.entry,
        result.marketplaceInstallLocation, pluginId,
        enabledVal === true, errorsOut, installEntry?.version)
}))
collect fulfilled non-null into plugins; rejected → 'generic-error'
```

### 5.4 `loadPluginFromMarketplaceEntry()` (full, network) — `:2191-2410`

```
if (typeof entry.source === 'string') {  // local relative path
  marketplaceDir = stat(installLocation).isDirectory() ? installLocation : dirname(installLocation)
  sourcePluginPath = join(marketplaceDir, entry.source)
  if (!pathExists(sourcePluginPath)) {
    errorsOut.push({type:'generic-error', source:pluginId,
      error: `Plugin directory not found at path: ${sourcePluginPath}. Check that the marketplace entry has the correct path.`})
    return null
  }
  manifest = await loadPluginManifest(join(sourcePluginPath,'.claude-plugin','plugin.json'),
                                       entry.name, entry.source)  // tolerated to fail
  version = await calculatePluginVersion(pluginId, entry.source, manifest,
                                          marketplaceDir, entry.version)
  pluginPath = await copyPluginToVersionedCache(sourcePluginPath, pluginId,
                                                 version, entry, marketplaceDir)
  // on copy fail: pluginPath = sourcePluginPath
} else {  // external (npm/github/url/git-subdir/pip)
  version = await calculatePluginVersion(pluginId, entry.source, /*no manifest*/,
                                          /*no marketplaceDir*/,
                                          installedVersion ?? entry.version,
                                          'sha' in entry.source ? entry.source.sha : undefined)
  versionedPath = getVersionedCachePath(pluginId, version)
  zipPath       = getVersionedZipCachePath(pluginId, version)
  if (zipCacheEnabled && pathExists(zipPath))      pluginPath = zipPath
  elif (pathExists(versionedPath))                  pluginPath = versionedPath
  else {
    seedPath = (await probeSeedCache(pluginId, version))
            ?? (version === 'unknown' ? await probeSeedCacheAnyVersion(pluginId) : null)
    if (seedPath) pluginPath = seedPath
    else {
      cached = await cachePlugin(entry.source, {manifest:{name:entry.name}})
      // Re-resolve version only when pre-clone version was 'unknown'.
      actualVersion = version!=='unknown' ? version
        : await calculatePluginVersion(pluginId, entry.source, cached.manifest,
                                        cached.path,
                                        installedVersion ?? entry.version,
                                        cached.gitCommitSha)
      pluginPath = await copyPluginToVersionedCache(cached.path, pluginId,
                                                     actualVersion, entry, undefined)
      if (cached.path !== pluginPath) await rm(cached.path,{recursive,force})
    }
  }
}
if (zipCacheEnabled && pluginPath.endsWith('.zip')) {
  sessionDir = await getSessionPluginCachePath()
  extractDir = join(sessionDir, pluginId.replace(/[^a-zA-Z0-9@\-_]/g,'-'))
  try { extractZipToDirectory(pluginPath, extractDir); pluginPath = extractDir }
  catch { rm(pluginPath,{force}); throw }
}
return finishLoadingPluginFromPath(entry, pluginId, enabled, errorsOut, pluginPath)
```

Version-recompute invariant (`:2331-2349`): if the pre-clone version was deterministic (any of `source.sha`, `entry.version`, `installedVersion`), REUSE it; recompute only when pre-clone was `'unknown'`. Otherwise post-clone manifest.version (step 1 of `calculatePluginVersion`) outranks gitCommitSha (step 3) and the cache key would diverge from the warm-start probe key, causing re-clone-forever.

### 5.5 `loadPluginFromMarketplaceEntryCacheOnly()` — `:2098-2174`

```
if (typeof entry.source === 'string') {
  marketplaceDir = stat(installLocation).isDirectory() ? installLocation : dirname(installLocation)
  pluginPath = join(marketplaceDir, entry.source)
} else {
  if (!installPath || !(await pathExists(installPath))) {
    errorsOut.push({type:'plugin-cache-miss', source:pluginId,
      plugin: entry.name, installPath: installPath ?? '(not recorded)'})
    return null
  }
  pluginPath = installPath
}
if (zipCacheEnabled && pluginPath.endsWith('.zip')) {
  // identical extraction as full loader; on failure → plugin-cache-miss
}
return finishLoadingPluginFromPath(entry, pluginId, enabled, errorsOut, pluginPath)
```

### 5.6 `createPluginFromPath()` — `:1348-1769`

```
manifestPath = join(pluginPath, '.claude-plugin', 'plugin.json')
manifest     = await loadPluginManifest(manifestPath, fallbackName, source)
plugin = {name: manifest.name, manifest, path: pluginPath,
          source, repository: source, enabled}

// Auto-detect optional dirs IFF manifest doesn't declare them
parallel:
  commandsDirExists      = !manifest.commands     && pathExists('commands/')
  agentsDirExists        = !manifest.agents       && pathExists('agents/')
  skillsDirExists        = !manifest.skills       && pathExists('skills/')
  outputStylesDirExists  = !manifest.outputStyles && pathExists('output-styles/')
if (commandsDirExists)     plugin.commandsPath     = 'commands/'
if (agentsDirExists)       plugin.agentsPath       = 'agents/'
if (skillsDirExists)       plugin.skillsPath       = 'skills/'
if (outputStylesDirExists) plugin.outputStylesPath = 'output-styles/'

// Process manifest.commands (3 formats: string | string[] | object-mapping)
//   object-mapping: { name → CommandMetadata } where CommandMetadata has source XOR content
//   path/array: validate via parallel pathExists; missing → 'path-not-found' error
if (manifest.commands) {...}                  // → plugin.commandsPaths / commandsMetadata
if (manifest.agents)   {...}                  // RelativeMarkdownPath | RelativeMarkdownPath[]
if (manifest.skills)   {...}                  // RelativePath | RelativePath[]
if (manifest.outputStyles) {...}              // RelativePath | RelativePath[]

// Step 5: Hooks
//   standard hooks/hooks.json (if exists) → loadPluginHooks → mergedHooks
//   manifest.hooks (string path | inline | array) → also loaded and merged
//   if (strict && both standard hooks file present AND manifest declares hooks)
//      → 'hook-load-failed' error (duplicate)
if (mergedHooks) plugin.hooksConfig = mergedHooks

// Step 6: Settings
plugin.settings = await loadPluginSettings(pluginPath, manifest)
//   try settings.json in plugin dir → parsePluginSettings (allowlist: agent only)
//   else fall back to manifest.settings → parsePluginSettings
//   filtered.length === 0 → undefined (skipped)

return {plugin, errors}
```

### 5.7 `loadPluginSettings()` and `PluginSettingsSchema` — `:1776-1849`

```ts
const PluginSettingsSchema = lazySchema(() =>
  SettingsSchema().pick({ agent: true }).strip())
```

Plugins may only contribute the `agent` settings key; any other top-level keys are silently stripped. Empty result → `undefined` (so `cachePluginSettings` skips the cache reset).

### 5.8 `mergePluginSources()` precedence — `:3009-3064`

```
managed = sources.managedNames                   // names locked by policySettings
sessionPlugins = sources.session.filter(p => {
  if (managed?.has(p.name)) {
    errors.push({type:'generic-error', source:p.source, plugin:p.name,
      error: `--plugin-dir copy of "${p.name}" ignored: plugin is locked by managed settings`})
    return false  // managed wins over --plugin-dir
  }
  return true
})
sessionNames = new Set(sessionPlugins.map(p => p.name))
marketplacePlugins = sources.marketplace.filter(p => !sessionNames.has(p.name))
return {plugins: [...sessionPlugins, ...marketplacePlugins, ...sources.builtin], errors}
```

Precedence (downstream first-match consumers): **session > marketplace > builtin**, with **managed > session** override.

### 5.9 `loadAllPlugins()` warming `loadAllPluginsCacheOnly` — `:3096-3108`

```
result = await assemblePluginLoadResult(() => loadPluginsFromMarketplaces({cacheOnly:false}))
loadAllPluginsCacheOnly.cache?.set(undefined, Promise.resolve(result))
return result
```

Refresh paths (`refresh.ts`, `headlessPluginInstall.ts`, `/plugins`) call `loadAllPlugins()` to fetch fresh source; the in-memory cache-only memo is then warmed so downstream consumers (commands/skills/MCP/LSP/agents) see post-clone results without re-running the loader.

### 5.10 Built-in plugin enable resolution — `builtinPlugins.ts:65-101`

```
for (name, definition) in BUILTIN_PLUGINS:
  if (definition.isAvailable && !definition.isAvailable()) continue
  pluginId    = `${name}@builtin`
  userSetting = settings?.enabledPlugins?.[pluginId]
  isEnabled   = userSetting !== undefined
                  ? userSetting === true
                  : (definition.defaultEnabled ?? true)
  plugin = {name, manifest:{name, description, version}, path:'builtin', source:pluginId,
            repository:pluginId, enabled:isEnabled, isBuiltin:true,
            hooksConfig: definition.hooks, mcpServers: definition.mcpServers}
  push to enabled or disabled
```

`getBuiltinPluginSkillCommands()` emits `Command` objects with `source:'bundled'` (NOT `'builtin'` — see `:144-148`) and `isHidden = !(definition.userInvocable ?? true)`.

> **Caveat (shipped source).** `src/plugins/bundled/index.ts:20-23` — `initBuiltinPlugins()` registers **zero** built-in plugins; the function body is the comment "No built-in plugins registered yet — this is the scaffolding for migrating bundled skills that should be user-toggleable." Therefore `BUILTIN_PLUGINS` is empty at runtime in the leaked tree, `getBuiltinPluginSkillCommands()` always returns `[]`, and the `for (name, definition) in BUILTIN_PLUGINS` loop above is dead. The cross-references from this spec (and from spec 17 / spec 20 on command/skill ordering) describe the *contract* that holds when entries are added — but a faithful reimplementation of the leaked source produces no built-in plugin commands. Do not assume runtime activity here without first registering at least one definition via `registerBuiltinPlugin()`.

### 5.11 Background install reconciliation — `PluginInstallationManager.ts:60-184`

```
declared    = getDeclaredMarketplaces()                          // from settings + implicit official
materialized = await loadKnownMarketplacesConfig().catch(()=>{})
diff = diffMarketplaces(declared, materialized)
pendingNames = [...diff.missing, ...diff.sourceChanged.map(c=>c.name)]
setAppState pending UI status
if (pendingNames.length === 0) return
result = await reconcileMarketplaces({onProgress: event => updateMarketplaceStatus(...)})
logEvent('tengu_marketplace_background_install', {installed_count, updated_count, failed_count, up_to_date_count})
if (result.installed.length > 0):
  clearMarketplacesCache()
  try await refreshActivePlugins(setAppState)
  catch:
    clearPluginCache('performBackgroundPluginInstallations: auto-refresh failed')
    setAppState plugins.needsRefresh = true
elif (result.updated.length > 0):
  clearMarketplacesCache()
  clearPluginCache('performBackgroundPluginInstallations: marketplaces reconciled')
  setAppState plugins.needsRefresh = true
```

### 5.12 Official marketplace GCS fast-path — `officialMarketplaceGcs.ts:47-170`

```
guard: resolve(installLocation) MUST be inside resolve(marketplacesCacheDir) (or equal)
await waitForScrollIdle()
latest = (await axios.get(`${GCS_BASE}/latest`, {timeout: 10_000})).data.trim()
sentinelPath = join(installLocation, '.gcs-sha')
currentSha   = readFile(sentinelPath).trim()  // null on ENOENT
if (currentSha === latest)  outcome = 'noop'; return latest
zipBuf = (await axios.get(`${GCS_BASE}/${latest}.zip`, {timeout: 60_000})).data
files  = unzipFile(zipBuf)
modes  = parseZipModes(zipBuf)               // recover exec bits
staging = `${installLocation}.staging`
rm -rf staging; mkdir -p staging
for (arcPath, data) in files:
  if (!arcPath.startsWith('marketplaces/claude-plugins-official/')) continue
  rel = arcPath.slice(prefix.length)
  if (!rel || rel.endsWith('/')) continue
  dest = join(staging, rel); mkdir -p dirname(dest); writeFile(dest, data)
  if (mode & 0o111) chmod(dest, mode & 0o777)  // swallow EPERM/ENOTSUP
writeFile(join(staging,'.gcs-sha'), latest)
rm -rf installLocation; rename(staging, installLocation)  // atomic swap
logEvent('tengu_plugin_remote_fetch', {source:'marketplace_gcs', host:'downloads.claude.ai',
                                        is_official:true, outcome, duration_ms, ...})
on error: classifyGcsError → 'timeout'|'http_<code>'|'network'|'fs_<code>'|'fs_other'|'zip_parse'|'empty_latest'|'other'
```

### 5.13 Identifier rules — `pluginIdentifier.ts`

- `parsePluginIdentifier`: split on first `@`; everything after the second `@` ignored.
- `isOfficialMarketplaceName(name)`: `ALLOWED_OFFICIAL_MARKETPLACE_NAMES.has(name.toLowerCase())`.
- `scopeToSettingSource('managed')` THROWS `Error('Cannot install plugins to managed scope')`.

### 5.14 Cache invalidation — `clearPluginCache(reason?)` — `:3225-3243`

```
log if (reason)
loadAllPlugins.cache?.clear?.()
loadAllPluginsCacheOnly.cache?.clear?.()
if (getPluginSettingsBase() !== undefined) resetSettingsCache()
clearPluginSettingsBase()
```

> **Cross-spec hazard (→ spec 02 settings cascade).** `resetSettingsCache()` at `src/utils/settings/settingsCache.ts:55-59` clears only `sessionSettingsCache`, `perSourceCache`, and `parseFileCache` — it does **not** touch `pluginSettingsBase` (declared at `:66`, mutated by `setPluginSettingsBase()` / `clearPluginSettingsBase()` at `:72` / `:78`). The primitive ordering inside `clearPluginCache()` above is *correct* — it calls `resetSettingsCache()` first, then `clearPluginSettingsBase()` second — but any **other** caller that invokes `resetSettingsCache()` directly (settings write, `--add-dir`, hooks refresh — see `settingsCache.ts:18` JSDoc) will leave a stale `pluginSettingsBase` lingering. Subsequent `loadSettingsFromDisk()` reads will merge the stale base. Reimplementers of spec 02's cascade must either (a) extend `resetSettingsCache()` to also call `clearPluginSettingsBase()` (current source does NOT) or (b) document that every cascade-invalidation site must additionally call `clearPluginSettingsBase()` — only `clearPluginCache()` does so today.

### 5.15 `cachePluginSettings(plugins)` — `:3281-3295`

```
settings = mergePluginSettings(plugins)         // later plugin wins per key
setPluginSettingsBase(settings)
if (settings && Object.keys(settings).length > 0) resetSettingsCache()
```

### 5.16 Orphaned plugin GC — `cacheUtils.ts:74-117`

Background sweep enumerates `<plugins>/cache/<m>/<p>/<v>/`; entries not present in `installed_plugins.json` get a `.orphaned_at` marker; entries with marker older than `CLEANUP_AGE_MS` (7 days) are removed.

### 5.17 Command/skill ordering (cross-cutting → spec 20)

`commands.ts:449-468` consumes (verbatim — object destructure of `getSkills(cwd)`,
not array destructure; ordering inside the returned array is `bundledSkills`,
`builtinPluginSkills`, `skillDirCommands`, `workflowCommands`, `pluginCommands`,
`pluginSkills`, `COMMANDS()`):

```ts
const [
  { skillDirCommands, pluginSkills, bundledSkills, builtinPluginSkills },
  pluginCommands,
  workflowCommands,
] = await Promise.all([
  getSkills(cwd),
  getPluginCommands(),
  getWorkflowCommands ? getWorkflowCommands(cwd) : Promise.resolve([]),
])

return [
  ...bundledSkills,
  ...builtinPluginSkills,
  ...skillDirCommands,
  ...workflowCommands,
  ...pluginCommands,
  ...pluginSkills,
  ...COMMANDS(),
]
```

Note that `builtinPluginSkills` is concatenated SECOND (immediately after
`bundledSkills`), NOT last. This positions built-in plugin skills with
"first-wins" semantics adjacent to the bundled skills they impersonate (both
emit `source:'bundled', loadedFrom:'bundled'` per §5.10 — see `builtinPlugins.ts:145-150`),
while user/marketplace `pluginSkills` come AFTER the user's `~/.claude/skills/`
directory (`skillDirCommands`) and after `pluginCommands`.

Spec 20 owns the merge ordering and first-match dedup; this spec produces the
three plugin-derived arrays (`pluginCommands`, `pluginSkills`, `builtinPluginSkills`).

---

## 6. Verbatim Assets

### 6.1 `PluginManifestSchema` — `schemas.ts:884-898`

```ts
export const PluginManifestSchema = lazySchema(() =>
  z.object({
    ...PluginManifestMetadataSchema().shape,
    ...PluginManifestHooksSchema().partial().shape,
    ...PluginManifestCommandsSchema().partial().shape,
    ...PluginManifestAgentsSchema().partial().shape,
    ...PluginManifestSkillsSchema().partial().shape,
    ...PluginManifestOutputStylesSchema().partial().shape,
    ...PluginManifestChannelsSchema().partial().shape,
    ...PluginManifestMcpServerSchema().partial().shape,
    ...PluginManifestLspServerSchema().partial().shape,
    ...PluginManifestSettingsSchema().partial().shape,
    ...PluginManifestUserConfigSchema().partial().shape,
  }),
)
```

Top-level unknown fields are silently stripped (zod default `.strip()`); nested config objects (`userConfig` options, `channels`, `lspServers`) are `.strict()` so typos there fail validation.

### 6.2 `PluginManifestMetadataSchema` — `:274-320`

```ts
const PluginManifestMetadataSchema = lazySchema(() => z.object({
  name: z.string().min(1, 'Plugin name cannot be empty')
    .refine(name => !name.includes(' '),
      { message: 'Plugin name cannot contain spaces. Use kebab-case (e.g., "my-plugin")' }),
  version: z.string().optional(),
  description: z.string().optional(),
  author: PluginAuthorSchema().optional(),
  homepage: z.string().url().optional(),
  repository: z.string().optional(),
  license: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  dependencies: z.array(DependencyRefSchema()).optional(),
}))
```

### 6.3 `PluginAuthorSchema` — `:251-266`

```ts
export const PluginAuthorSchema = lazySchema(() => z.object({
  name: z.string().min(1, 'Author name cannot be empty'),
  email: z.string().optional(),
  url: z.string().optional(),
}))
```

### 6.4 `PluginHooksSchema` — `:328-340`

```ts
export const PluginHooksSchema = lazySchema(() => z.object({
  description: z.string().optional(),
  hooks: z.lazy(() => HooksSchema()),
}))
```

### 6.5 `PluginManifestHooksSchema` (manifest top-level) — `:348-373`

`hooks` is `RelativeJSONPath | HooksSchema | Array<RelativeJSONPath | HooksSchema>`.

### 6.6 `PluginManifestCommandsSchema` — `:429-452`

`commands` is one of: a single `RelativeCommandPath` (markdown `.md` OR any relative path), an array thereof, or a record `Record<string, CommandMetadata>`.

### 6.7 `CommandMetadataSchema` — `:385-416`

```ts
export const CommandMetadataSchema = lazySchema(() => z.object({
  source: RelativeCommandPath().optional(),
  content: z.string().optional(),
  description: z.string().optional(),
  argumentHint: z.string().optional(),
  model: z.string().optional(),
  allowedTools: z.array(z.string()).optional(),
}).refine(data => (data.source && !data.content) || (!data.source && data.content),
  { message: 'Command must have either "source" (file path) or "content" (inline markdown), but not both' }))
```

### 6.8 Path-shape lazy schemas — `:162-203`

```ts
const RelativePath          = lazySchema(() => z.string().startsWith('./'))
const RelativeJSONPath      = lazySchema(() => RelativePath().endsWith('.json'))
const RelativeMarkdownPath  = lazySchema(() => RelativePath().endsWith('.md'))
const RelativeCommandPath   = lazySchema(() => z.union([RelativeMarkdownPath(), RelativePath()]))
const McpbPath              = lazySchema(() => z.union([
  RelativePath().refine(p => p.endsWith('.mcpb') || p.endsWith('.dxt'),
    {message: 'MCPB file path must end with .mcpb or .dxt'}),
  z.string().url().refine(u => u.endsWith('.mcpb') || u.endsWith('.dxt'),
    {message: 'MCPB URL must end with .mcpb or .dxt'}),
]))
```

### 6.9 `PluginManifestUserConfigSchema` & options — `:587-654`

```ts
const PluginUserConfigOptionSchema = lazySchema(() => z.object({
  type: z.enum(['string','number','boolean','directory','file']),
  title: z.string(),
  description: z.string(),
  required: z.boolean().optional(),
  default: z.union([z.string(), z.number(), z.boolean(), z.array(z.string())]).optional(),
  multiple: z.boolean().optional(),
  sensitive: z.boolean().optional(),
  min: z.number().optional(),
  max: z.number().optional(),
}).strict())

const PluginManifestUserConfigSchema = lazySchema(() => z.object({
  userConfig: z.record(
    z.string().regex(/^[A-Za-z_]\w*$/,
      'Option keys must be valid identifiers (letters, digits, underscore; no leading digit) — they become CLAUDE_PLUGIN_OPTION_<KEY> env vars in hooks'),
    PluginUserConfigOptionSchema(),
  ).optional(),
}))
```

Sensitive values go to keychain / `.credentials.json`; non-sensitive to `settings.json` `pluginConfigs[pluginId].options`. Available to MCP/LSP env, hooks, and (non-sensitive only) skill/agent content as `${user_config.KEY}`. Sensitive values share one keychain entry with OAuth tokens — keep total under ~2KB stdin-safe limit (INC-3028).

### 6.10 `PluginManifestChannelsSchema` — `:670-703`

```ts
channels: z.array(z.object({
  server: z.string().min(1),
  displayName: z.string().optional(),
  userConfig: z.record(z.string(), PluginUserConfigOptionSchema()).optional(),
}).strict())
```

`server` must match a key in this plugin's `mcpServers`; cross-validated at load time in `mcpPluginIntegration.ts`, NOT at schema parse time.

### 6.11 `PluginManifestMcpServerSchema` — `:543-572`

`mcpServers`: `RelativeJSONPath | McpbPath | Record<string, McpServerConfigSchema> | Array<...>`. Detail of MCP server config schema → spec 23.

### 6.12 `LspServerConfigSchema` & `PluginManifestLspServerSchema` — `:708-820`

```ts
export const LspServerConfigSchema = lazySchema(() => z.strictObject({
  command: z.string().min(1).refine(cmd => {
    if (cmd.includes(' ') && !cmd.startsWith('/')) return false
    return true
  }, { message: 'Command should not contain spaces. Use args array for arguments.' }),
  args: z.array(nonEmptyString()).optional(),
  extensionToLanguage: z.record(fileExtension(), nonEmptyString())
    .refine(r => Object.keys(r).length > 0,
      {message: 'extensionToLanguage must have at least one mapping'}),
  transport: z.enum(['stdio','socket']).default('stdio'),
  env: z.record(z.string(), z.string()).optional(),
  initializationOptions: z.unknown().optional(),
  settings: z.unknown().optional(),
  workspaceFolder: z.string().optional(),
  startupTimeout: z.number().int().positive().optional(),
  shutdownTimeout: z.number().int().positive().optional(),
  restartOnCrash: z.boolean().optional(),
  maxRestarts: z.number().int().nonnegative().optional(),
}))
```

`fileExtension`: `z.string().min(2).refine(ext => ext.startsWith('.'), {message: 'File extensions must start with dot (e.g., ".ts", not "ts")'})`.

### 6.13 `PluginManifestSettingsSchema` — `:857-867`

```ts
const PluginManifestSettingsSchema = lazySchema(() => z.object({
  settings: z.record(z.string(), z.unknown()).optional(),
}))
// At load time, filtered through PluginSettingsSchema = SettingsSchema().pick({agent:true}).strip()
```

### 6.14 `PluginSourceSchema` — `:1062-1161`

```ts
export const PluginSourceSchema = lazySchema(() => z.union([
  RelativePath(),                               // local in marketplace
  z.object({ source: z.literal('npm'),
             package: NpmPackageNameSchema().or(z.string()),
             version: z.string().optional(),
             registry: z.string().url().optional() }),
  z.object({ source: z.literal('pip'),
             package: z.string(),
             version: z.string().optional(),
             registry: z.string().url().optional() }),
  z.object({ source: z.literal('url'),
             url: z.string(),
             ref: z.string().optional(),
             sha: gitSha().optional() }),
  z.object({ source: z.literal('github'),
             repo: z.string(),
             ref: z.string().optional(),
             sha: gitSha().optional() }),
  z.object({ source: z.literal('git-subdir'),
             url: z.string(),
             path: z.string().min(1),
             ref: z.string().optional(),
             sha: gitSha().optional() }),
]))

export const gitSha = lazySchema(() =>
  z.string().length(40).regex(/^[a-f0-9]{40}$/, 'Must be a full 40-character lowercase git commit SHA'))
```

`url` and `git` sources do NOT enforce `.endsWith('.git')` — Azure DevOps and AWS CodeCommit URLs lack that suffix (gh-31256).

### 6.15 `MarketplaceSourceSchema` — `:906-1044`

Discriminated union on `source`: `'url' | 'github' | 'git' | 'npm' | 'file' | 'directory' | 'hostPattern' | 'pathPattern' | 'settings'`. Settings-arm rejects reserved official names AND reserved literals `'inline'` and `'builtin'`.

`MarketplaceNameSchema` (`:216-246`):

```ts
const MarketplaceNameSchema = lazySchema(() => z.string().min(1, 'Marketplace must have a name')
  .refine(n => !n.includes(' '),
    {message: 'Marketplace name cannot contain spaces. Use kebab-case (e.g., "my-marketplace")'})
  .refine(n => !n.includes('/') && !n.includes('\\') && !n.includes('..') && n !== '.',
    {message: 'Marketplace name cannot contain path separators (/ or \\), ".." sequences, or be "."'})
  .refine(n => !isBlockedOfficialName(n),
    {message: 'Marketplace name impersonates an official Anthropic/Claude marketplace'})
  .refine(n => n.toLowerCase() !== 'inline',
    {message: 'Marketplace name "inline" is reserved for --plugin-dir session plugins'})
  .refine(n => n.toLowerCase() !== 'builtin',
    {message: 'Marketplace name "builtin" is reserved for built-in plugins'}))
```

### 6.16 Reserved official names + impersonation guard — `:19-101`

```ts
export const ALLOWED_OFFICIAL_MARKETPLACE_NAMES = new Set([
  'claude-code-marketplace',
  'claude-code-plugins',
  'claude-plugins-official',
  'anthropic-marketplace',
  'anthropic-plugins',
  'agent-skills',
  'life-sciences',
  'knowledge-work-plugins',
])

const NO_AUTO_UPDATE_OFFICIAL_MARKETPLACES = new Set(['knowledge-work-plugins'])
// Behavioral consequence: marketplaces whose name appears in this set are
// excluded from the background auto-update reconciliation pass even though
// they pass the official-name source-validation guard. Today only
// 'knowledge-work-plugins' is opted out — the GCS fast-path / git pull cycle
// in officialMarketplaceGcs.ts and reconciler.ts skips fetch-and-refresh for
// any source whose declared name is in this set. (See §5 cross-cut: §5.11
// background reconciliation; §5.12 GCS fast-path.) Reimplementations MUST
// honor this opt-out — failing to do so causes unwanted plugin churn for
// the 'knowledge-work-plugins' marketplace, which is curated externally.

export const BLOCKED_OFFICIAL_NAME_PATTERN =
  /(?:official[^a-z0-9]*(anthropic|claude)|(?:anthropic|claude)[^a-z0-9]*official|^(?:anthropic|claude)[^a-z0-9]*(marketplace|plugins|official))/i

const NON_ASCII_PATTERN = /[^ -~]/

export const OFFICIAL_GITHUB_ORG = 'anthropics'
```

`isBlockedOfficialName(name)`: returns `false` if name is in allowed set; returns `true` if non-ASCII; otherwise tests `BLOCKED_OFFICIAL_NAME_PATTERN`.

`validateOfficialNameSource(name, source)` — `:119-157`. For names in `ALLOWED_OFFICIAL_MARKETPLACE_NAMES`:
- `source.source === 'github'`: requires `repo.toLowerCase().startsWith('anthropics/')`.
- `source.source === 'git'`: requires URL containing `'github.com/anthropics/'` (HTTPS) OR `'git@github.com:anthropics/'` (SSH).
- Any other source type: rejected with reserved-name error.

Error string template:
```
The name '${name}' is reserved for official Anthropic marketplaces. Only repositories from 'github.com/${OFFICIAL_GITHUB_ORG}/' can use this name.
```
or
```
The name '${name}' is reserved for official Anthropic marketplaces and can only be used with GitHub sources from the '${OFFICIAL_GITHUB_ORG}' organization.
```

### 6.17 Identifier schemas — `:1339-1391`, `:1408-1428`

```ts
export const PluginIdSchema = lazySchema(() => z.string()
  .regex(/^[a-z0-9][-a-z0-9._]*@[a-z0-9][-a-z0-9._]*$/i,
         'Plugin ID must be in format: plugin@marketplace'))

const DEP_REF_REGEX = /^[a-z0-9][-a-z0-9._]*(@[a-z0-9][-a-z0-9._]*)?(@\^[^@]*)?$/i

export const DependencyRefSchema = lazySchema(() => z.union([
  z.string().regex(DEP_REF_REGEX,
    'Dependency must be a plugin name, optionally qualified with @marketplace')
   .transform(s => s.replace(/@\^[^@]*$/,'')),
  z.object({
    name: z.string().min(1).regex(/^[a-z0-9][-a-z0-9._]*$/i),
    marketplace: z.string().min(1).regex(/^[a-z0-9][-a-z0-9._]*$/i).optional(),
  }).loose().transform(o => o.marketplace ? `${o.name}@${o.marketplace}` : o.name),
]))

export const SettingsPluginEntrySchema = lazySchema(() => z.union([
  PluginIdSchema(),
  z.object({
    id: PluginIdSchema(),
    version: z.string().optional(),
    required: z.boolean().optional(),
    config: z.record(z.string(), z.unknown()).optional(),
  }),
]))

export const PluginScopeSchema = lazySchema(() =>
  z.enum(['managed', 'user', 'project', 'local']))
```

### 6.18 Marketplace schemas — `:1254-1326`

```ts
export const PluginMarketplaceEntrySchema = lazySchema(() =>
  PluginManifestSchema().partial().extend({
    name: z.string().min(1, 'Plugin name cannot be empty')
       .refine(n => !n.includes(' '),
         {message: 'Plugin name cannot contain spaces. Use kebab-case (e.g., "my-plugin")'}),
    source: PluginSourceSchema(),
    category: z.string().optional(),
    tags: z.array(z.string()).optional(),
    strict: z.boolean().optional().default(true),
  }))

export const PluginMarketplaceSchema = lazySchema(() => z.object({
  name: MarketplaceNameSchema(),
  owner: PluginAuthorSchema(),
  plugins: z.array(PluginMarketplaceEntrySchema()),
  forceRemoveDeletedPlugins: z.boolean().optional(),
  metadata: z.object({
    pluginRoot: z.string().optional(),
    version: z.string().optional(),
    description: z.string().optional(),
  }).optional(),
  allowCrossMarketplaceDependenciesOn: z.array(z.string()).optional(),
}))
```

### 6.19 Installed-plugin schemas — `:1446-1577`, `:1592-1629`

```ts
export const InstalledPluginSchema = lazySchema(() => z.object({
  version: z.string(),
  installedAt: z.string(),
  lastUpdated: z.string().optional(),
  installPath: z.string(),
  gitCommitSha: z.string().optional(),
}))

export const InstalledPluginsFileSchemaV1 = lazySchema(() => z.object({
  version: z.literal(1),
  plugins: z.record(PluginIdSchema(), InstalledPluginSchema()),
}))

export const PluginInstallationEntrySchema = lazySchema(() => z.object({
  scope: PluginScopeSchema(),
  projectPath: z.string().optional(),
  installPath: z.string(),
  version: z.string().optional(),
  installedAt: z.string().optional(),
  lastUpdated: z.string().optional(),
  gitCommitSha: z.string().optional(),
}))

export const InstalledPluginsFileSchemaV2 = lazySchema(() => z.object({
  version: z.literal(2),
  plugins: z.record(PluginIdSchema(),
                     z.array(PluginInstallationEntrySchema())),
}))

export const InstalledPluginsFileSchema = lazySchema(() => z.union([
  InstalledPluginsFileSchemaV1(), InstalledPluginsFileSchemaV2()
]))

export const KnownMarketplaceSchema = lazySchema(() => z.object({
  source: MarketplaceSourceSchema(),
  installLocation: z.string(),
  lastUpdated: z.string(),
  autoUpdate: z.boolean().optional(),
}))
```

### 6.20 Official marketplace constants — `officialMarketplace.ts:15-25`, `officialMarketplaceGcs.ts:28-34`

```ts
export const OFFICIAL_MARKETPLACE_SOURCE = {
  source: 'github',
  repo: 'anthropics/claude-plugins-official',
} as const satisfies MarketplaceSource

export const OFFICIAL_MARKETPLACE_NAME = 'claude-plugins-official'

const GCS_BASE =
  'https://downloads.claude.ai/claude-code-releases/plugins/claude-plugins-official'
const ARC_PREFIX = 'marketplaces/claude-plugins-official/'
```

GCS endpoints used:
- `${GCS_BASE}/latest` — text body, the SHA pointer; `Cache-Control: no-cache, max-age=300`; timeout 10_000 ms.
- `${GCS_BASE}/${sha}.zip` — content-addressed zip bundle; timeout 60_000 ms.

### 6.21 NPM package name regex — `:837-850`

```ts
const NpmPackageNameSchema = lazySchema(() => z.string()
  .refine(n => !n.includes('..') && !n.includes('//'),
          'Package name cannot contain path traversal patterns')
  .refine(n => {
    const scopedPackageRegex = /^@[a-z0-9][a-z0-9-._]*\/[a-z0-9][a-z0-9-._]*$/
    const regularPackageRegex = /^[a-z0-9][a-z0-9-._]*$/
    return scopedPackageRegex.test(n) || regularPackageRegex.test(n)
  }, 'Invalid npm package name format'))
```

### 6.22 Constants table

| Constant | Value | Source |
|---|---|---|
| `BUILTIN_MARKETPLACE_NAME` | `'builtin'` | `builtinPlugins.ts:23` |
| `OFFICIAL_MARKETPLACE_NAME` | `'claude-plugins-official'` | `officialMarketplace.ts:25` |
| `OFFICIAL_GITHUB_ORG` | `'anthropics'` | `schemas.ts:107` |
| `GCS_BASE` (official mkt mirror) | `'https://downloads.claude.ai/claude-code-releases/plugins/claude-plugins-official'` | `officialMarketplaceGcs.ts:28-29` |
| `ARC_PREFIX` (zip path prefix) | `'marketplaces/claude-plugins-official/'` | `officialMarketplaceGcs.ts:34` |
| `GCS latest fetch timeout` | `10_000` ms | `officialMarketplaceGcs.ts:83` |
| `GCS zip fetch timeout` | `60_000` ms | `officialMarketplaceGcs.ts:109` |
| `DEFAULT_PLUGIN_GIT_TIMEOUT_MS` | `120 * 1000` ms | `marketplaceManager.ts:515` |
| `Git timeout env override` | `CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS` | `marketplaceManager.ts:518` |
| `installCounts CACHE_TTL_MS` | `24 * 60 * 60 * 1000` ms (24 h) | `installCounts.ts:27` |
| `Orphaned plugin GC age` | `7 * 24 * 60 * 60 * 1000` ms (7 d) | `cacheUtils.ts:24` |
| `ORPHANED_AT_FILENAME` | `'.orphaned_at'` | `cacheUtils.ts:23` |
| `GCS sentinel file` | `'.gcs-sha'` | `officialMarketplaceGcs.ts:94, :138` |
| `Cache integrity hash algo (mcpb id)` | `sha256` (first 16 hex chars) | `mcpbHandler.ts:94` |
| `Plugin path-hash algo` | `sha256` (first 8 hex chars) | `pluginVersioning.ts:73-79` |
| `gitSha length / regex` | length 40, `/^[a-f0-9]{40}$/` | `schemas.ts:1046-1054` |
| `Allowlist of editable scopes` | `user, project, local` | `pluginIdentifier.ts:89-96` |
| Plugin settings allowlist | only `agent` | `pluginLoader.ts:1776-1782` |
| Plugin name validators | no spaces, kebab-case advised | `schemas.ts:278-285` |
| User-config option key regex | `/^[A-Za-z_]\w*$/` | `schemas.ts:638-642` |

There is **no plugin-count cap** and **no plugin signature/integrity verification** in the source — verification is policy-based (allowlist of marketplace sources, blocklist, reserved-name source check) rather than cryptographic. The mcpb handler hashes content with sha256 to derive an installation ID; the GCS path is content-addressed via SHA but uses HTTPS+CDN for transport authenticity, not an in-band signature.

### 6.23 User-facing error strings (verbatim, by error type)

From `getPluginErrorMessage()` (`types/plugin.ts:295-363`):

```
'generic-error':            ${error.error}
'path-not-found':           Path not found: ${path} (${component})
'git-auth-failed':          Git authentication failed (${authType}): ${gitUrl}
'git-timeout':              Git ${operation} timeout: ${gitUrl}
'network-error':            Network error: ${url}${details ? ` - ${details}` : ''}
'manifest-parse-error':     Manifest parse error: ${parseError}
'manifest-validation-error':Manifest validation failed: ${errors.join(', ')}
'plugin-not-found':         Plugin ${pluginId} not found in marketplace ${marketplace}
'marketplace-not-found':    Marketplace ${marketplace} not found
'marketplace-load-failed':  Marketplace ${marketplace} failed to load: ${reason}
'mcp-config-invalid':       MCP server ${serverName} invalid: ${validationError}
'mcp-server-suppressed-duplicate':
                            MCP server "${serverName}" skipped — same command/URL as
                            (server provided by plugin "${name}" | already-configured "${dup}")
'hook-load-failed':         Hook load failed: ${reason}
'component-load-failed':    ${component} load failed from ${path}: ${reason}
'mcpb-download-failed':     Failed to download MCPB from ${url}: ${reason}
'mcpb-extract-failed':      Failed to extract MCPB ${mcpbPath}: ${reason}
'mcpb-invalid-manifest':    MCPB manifest invalid at ${mcpbPath}: ${validationError}
'lsp-config-invalid':       Plugin "${plugin}" has invalid LSP server config for "${serverName}": ${validationError}
'lsp-server-start-failed':  Plugin "${plugin}" failed to start LSP server "${serverName}": ${reason}
'lsp-server-crashed':       Plugin "${plugin}" LSP server "${serverName}" crashed
                            (with signal ${signal} | with exit code ${exitCode ?? 'unknown'})
'lsp-request-timeout':      Plugin "${plugin}" LSP server "${serverName}" timed out on ${method} request after ${timeoutMs}ms
'lsp-request-failed':       Plugin "${plugin}" LSP server "${serverName}" ${method} request failed: ${error}
'marketplace-blocked-by-policy':
   if (blockedByBlocklist):  Marketplace '${marketplace}' is blocked by enterprise policy
   else:                     Marketplace '${marketplace}' is not in the allowed marketplace list
'dependency-unsatisfied':   Dependency "${dependency}" is (disabled — enable it or remove the dependency | not found in any configured marketplace)
'plugin-cache-miss':        Plugin "${plugin}" not cached at ${installPath} — run /plugins to refresh
```

Other verbatim strings emitted from the loader path:

- `pluginLoader.ts:1190-1191`:
  ```
  Plugin ${pluginName} has an invalid manifest file at ${manifestPath}.

  Validation errors: ${errors}
  ```
- `:1209-1210`:
  ```
  Plugin ${pluginName} has a corrupt manifest file at ${manifestPath}.

  JSON parse error: ${errorMsg}
  ```
- `:1230`: `Hooks file not found at ${hooksConfigPath} for plugin ${pluginName}. If the manifest declares hooks, the file must exist.`
- `:2222`: `Plugin directory not found at path: ${sourcePluginPath}. Check that the marketplace entry has the correct path.`
- `:2376`: `Failed to download/cache plugin ${entry.name}: ${errorMsg}`
- `:3040`: `--plugin-dir copy of "${p.name}" ignored: plugin is locked by managed settings`
- `pluginIdentifier.ts:108`: `Cannot install plugins to managed scope`
- Marketplace name validation messages — see §6.15.
- Git timeout messages — `marketplaceManager.ts:664, :913`:
  ```
  Git pull timed out after ${timeoutSec}s. Try increasing the timeout via CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS environment variable.
  Git clone timed out after ${Math.round(timeoutMs/1000)}s. The repository may be too large for the current timeout. Set CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS to increase it (e.g., 300000 for 5 minutes).
  ```

---

## 7. Side Effects & I/O

### 7.1 Filesystem

- Read/write `~/.claude/plugins/{cache, marketplaces, data, installed_plugins.json, known_marketplaces.json}` (or `cowork_plugins/`, or `CLAUDE_CODE_PLUGIN_CACHE_DIR`).
- Read-only seed scan over `CLAUDE_CODE_PLUGIN_SEED_DIR` (path-delimited list).
- Atomic-swap `<installLocation>.staging` → `<installLocation>` for GCS extracts.
- `mkdirSync` on `getPluginDataDir` (sync; called inside `String.replace` callback for `${CLAUDE_PLUGIN_DATA}`).
- `chmod` of unzipped files preserving exec bits; failure swallowed (NFS root_squash, FUSE).
- Cleanup: `.orphaned_at` markers, 7-day GC of orphaned plugin versions.

### 7.2 Network

- `axios.get` to `downloads.claude.ai` (GCS mirror) for the official marketplace.
- `git clone` / `git pull` via `gitExe`/`execFileNoThrow*` for github, git, git-subdir, url marketplace and plugin sources. Timeout: `CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS` ms (default 120 000).
- `npm install` / `pip install` for npm and pip plugin sources (handled by `cachePlugin`).
- HTTP fetch of MCPB bundles (`mcpbHandler.ts`).

### 7.3 Process spawn

- `git` (clone, pull, sparse-checkout, partial clone `--filter=tree:0` for git-subdir, rev-parse for SHAs).
- `npm`, possibly `pip` for npm/pip sources.

### 7.4 Environment variables

| Var | Effect |
|---|---|
| `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` | When truthy: `loadAllPluginsCacheOnly()` delegates to full `loadAllPlugins()`, blocking startup until clones complete. Used by CCR / first-run headless to populate the cache before MCP/agent setup. |
| `CLAUDE_CODE_PLUGIN_SEED_DIR` | Platform-`delimiter`-split (`:` Unix / `;` Windows). Read-only fallback layers; first hit wins. Mirrors primary dir layout. |
| `CLAUDE_CODE_PLUGIN_CACHE_DIR` | Override base plugins directory; `expandTilde`'d for `~/...` strings injected via settings.json `env`. |
| `CLAUDE_CODE_USE_COWORK_PLUGINS` | Switches plugin dir name `plugins` → `cowork_plugins`. Session state from `--cowork` CLI flag takes precedence. |
| `CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS` | Override 120 000 ms git clone/pull timeout. |
| `CLAUDE_CODE_SYNC_PLUGIN_INSTALL_TIMEOUT_MS` | Used by `cli/print.ts:1888` to race deferred plugin installation against a hard timeout. |

### 7.5 Trust boundaries

- Marketplace sources subject to `getStrictKnownMarketplaces()` allowlist and `getBlockedMarketplaces()` blocklist (`marketplaceHelpers.ts`).
- Reserved official marketplace names enforced via `validateOfficialNameSource` — only `github.com/anthropics/*` may register them.
- Settings-source plugins blocked from using reserved official names (`schemas.ts:1015-1024`) because validation runs after disk write.
- Path traversal defense: marketplace name forbids `/`, `\`, `..`, `'.'`; plugin/marketplace/version sanitized for cache paths via `[^a-zA-Z0-9\-_(.)]→'-'`.
- Homograph defense: marketplace names rejected if any non-ASCII (`NON_ASCII_PATTERN`).
- GCS extract guard: refuses to write outside the resolved `marketplacesCacheDir`.

---

## 8. Feature Flags & Variants

No `feature(...)` gates in this subsystem. No `USER_TYPE === 'ant'` branches.

Runtime variants:
- Cowork plugin dir (`--cowork` / `CLAUDE_CODE_USE_COWORK_PLUGINS`): isolated `cowork_plugins/` directory.
- ZIP cache mode: `isPluginZipCacheEnabled()` (`zipCache.ts`) controls whether plugins are stored as `.zip` and extracted to a session-temp dir on load.
- Sync vs cache-only loading: `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` collapses cache-only into full loader.

---

## 9. Error Handling & Edge Cases

### 9.1 PluginError discriminated union (`types/plugin.ts:101-283`)

24 variants. **Source caveat:** the source declares `lsp-config-invalid` *twice* (`:177-182` and `:220-225`, byte-identical shape) — the count of 24 is correct only because TypeScript treats structurally-identical arms as the same union member. See `BUGS-IN-SOURCE.md` entry 6. A reimplementation that enumerates union arms via codegen rather than `keyof` will need to dedupe.

Currently produced (status of "in production" vs "planned" per `:88-99`):

- **Active**: `generic-error`, `plugin-not-found`, `marketplace-blocked-by-policy`, `plugin-cache-miss`, `path-not-found`, `dependency-unsatisfied`, `mcp-server-suppressed-duplicate`, `lsp-config-invalid`, `lsp-server-start-failed`, `lsp-server-crashed`, `lsp-request-timeout`, `lsp-request-failed`, `mcpb-*`.
- **Planned** (UI formatters present, creation sites pending): `git-auth-failed`, `git-timeout`, `network-error`, `manifest-parse-error`, `manifest-validation-error`, `marketplace-not-found`, `marketplace-load-failed`, `mcp-config-invalid`, `hook-load-failed`, `component-load-failed`.

### 9.2 Manifest failures

- Missing `plugin.json` → `loadPluginManifest()` returns synthetic `{name: pluginName, description: 'Plugin from ${source}'}` (`pluginLoader.ts:1156-1159`). Plugin still loads; commands/agents auto-detected from filesystem.
- Corrupt JSON → throws with verbatim `JSON parse error:` message; caught by surrounding catch → 'generic-error'.
- Schema validation failure → throws with `Validation errors: ${path: message, ...}` formatted; caught → 'generic-error'.
- Hooks file declared but missing → throws verbatim hooks-file-not-found message.

### 9.3 Marketplace failures

- Corrupt `known_marketplaces.json`: `loadKnownMarketplacesConfigSafe()` returns `{}`. With `hasEnterprisePolicy` active, all plugins error with `marketplace-blocked-by-policy` (fail-closed); without policy, fallback path `getPluginByIdCacheOnly` reads raw cast.
- Plugin missing from marketplace catalog → `plugin-not-found`.
- GCS fetch failure: `classifyGcsError()` buckets into `timeout | http_<code> | network | fs_<code> | fs_other | zip_parse | empty_latest | other`; logs warn; returns `null` so caller can fall back to git clone.

### 9.4 Cache miss (cache-only mode)

External-source plugin not present at recorded `installPath` → `plugin-cache-miss` with display: `Plugin "${plugin}" not cached at ${installPath} — run /plugins to refresh`.

### 9.5 Dependency demotion

`verifyAndDemote(allPlugins)` (`dependencyResolver.ts`) runs after merge. Returns `{demoted: Set<source>, errors: dependency-unsatisfied[]}`. Demotion sets `enabled=false` for the session ONLY (does not mutate settings).

### 9.6 Duplicate suppression

`mergePluginSources` drops marketplace plugins whose name matches a session plugin (warn log only; not an error). `mcpPluginIntegration` emits `mcp-server-suppressed-duplicate` when two plugins (or plugin + non-plugin config) have the same MCP command/URL.

### 9.7 Error UI

- `tengu_marketplace_background_install` analytics event with `{installed_count, updated_count, failed_count, up_to_date_count}`.
- AppState `plugins.installationStatus.marketplaces[].status: 'pending' | 'installing' | 'installed' | 'failed'` for REPL notifs.
- `plugins.needsRefresh` flag → notification prompts user to `/reload-plugins`.

---

## 10. Telemetry & Observability

- `tengu_marketplace_background_install` — see §9.7.
- `tengu_plugin_remote_fetch` — `{source: 'marketplace_gcs', host: 'downloads.claude.ai', is_official: true, outcome, duration_ms, bytes?, sha?, error_kind?}` (`officialMarketplaceGcs.ts:159-168`).
- `logForDebugging` lines at every load step (parallel checks, version computation, cache hits, copy fallbacks, ZIP extracts, merge dedup).
- `logForDiagnosticsNoPII('info', 'tengu_marketplace_background_install', metrics)`.
- `logEvent`/`logError` for fetch, parse, and validation failures.
- `fetchTelemetry.ts` exports `classifyFetchError`, `logPluginFetch` for the install path.

---

## 11. Reimplementation Checklist

A reimplementer must preserve:

1. **Two-tiered loader memoization**: independent caches; full loader warms cache-only memo, never the reverse. `CLAUDE_CODE_SYNC_PLUGIN_INSTALL` delegates cache-only → full.
2. **Source precedence**: session > marketplace > builtin; managed > session.
3. **Manifest schema strip behavior**: top-level `.strip()` (silent unknown-field drop); nested `userConfig` options/`channels`/`lspServers` `.strict()` (typo rejection).
4. **Settings allowlist**: only `SettingsSchema.pick({agent: true})` survives; everything else stripped silently.
5. **Identifier parsing**: split on first `@` only.
6. **Reserved official names**: full set in §6.16; impersonation regex; non-ASCII rejection; `validateOfficialNameSource` requiring `anthropics/*` GitHub source.
7. **Reserved marketplace name literals**: `'inline'` (session plugins), `'builtin'` (built-ins).
8. **Versioned cache structure**: `<plugins>/cache/<m>/<p>/<v>/`. Fallbacks: zip variant `.zip`, seed cache (`probeSeedCache` with `probeSeedCacheAnyVersion` for `'unknown'` version case).
9. **Version-recompute invariant**: reuse pre-clone version unless it was `'unknown'`. Otherwise post-clone manifest.version (rank 1) supplants gitCommitSha (rank 3), causing cache-key drift.
10. **Cache invalidation order**: `clearPluginCache` clears both memos AND `pluginSettingsBase`; resets settings cache only when `pluginSettingsBase !== undefined`.
11. **Built-in plugin identifier**: `${name}@builtin`; skill commands emit `source: 'bundled'`, NOT `'builtin'` (which means hardcoded `/help`-class commands).
12. **Built-in plugin enable resolution**: `userSetting !== undefined ? userSetting === true : (defaultEnabled ?? true)`.
13. **Path sanitizers**: marketplace/plugin → `[^a-zA-Z0-9\-_]→'-'`; version → `[^a-zA-Z0-9\-_.]→'-'`; pluginId data dir → same as marketplace/plugin; ZIP extract dir → `[^a-zA-Z0-9@\-_]→'-'`.
14. **Settings-source plugin schema** narrows source to remote (no relative paths) and rejects reserved official names — `validateOfficialNameSource` runs AFTER disk write.
15. **Session-only `'flag'` scope** is NOT persisted to `installed_plugins.json`.
16. **InstalledPluginsFileSchema** accepts both V1 and V2; V2 supports per-scope multi-installation.
17. **Dependency parser** strips trailing `@^...` constraint and accepts both string and `{name, marketplace?}` forms; both normalize to bare-string identifier (no version constraints reach downstream).
18. **GCS atomic-swap path-safety guard**: refuse extract paths outside `marketplacesCacheDir`.
19. **Background install AppState transitions**: `pending → installing → installed | failed`.
20. **Plugin error display formatters**: verbatim per §6.23.
21. **Memoized commands/skills/hooks loaders** (`loadPluginCommands.ts`, `loadPluginHooks.ts`) consume `loadAllPluginsCacheOnly()` results; `clearPluginCache()` does NOT clear those — callers must clear them separately or rely on `refresh.ts:refreshActivePlugins` orchestration.

---

## 12. Open Questions / Unknowns

Phase 9.7 status legend: **DEFERRED** = genuinely unverifiable from leaked
source / cross-spec ownership; **RESOLVED Phase 9.7** = answered by other
sections of this spec or by Phase 9.6/9.7 ripple work. **NOTE Phase 9.7** =
clarified but kept open as caveat.

1. **`probeSeedCacheAnyVersion`** chooses the single version dir if exactly one exists; if 2+ exist within a seed, falls through to next seed (`pluginLoader.ts:228-232`). Behavior with `>=2` versions and no other seeds is "no match", silently dropping the seed. Documentation hints this is a "BYOC" (bring-your-own-cache) optimization rather than a generic guarantee. **NOTE Phase 9.7** — confirmed by source comment; intentional. No further verification possible.
2. **`getPluginByIdCacheOnly` raw cast**: when the schema-safe load returns `{}`, the fallback path bypasses `KnownMarketplacesFileSchema` validation and casts raw JSON. The `loadPluginsFromMarketplaces` fail-closed guard mitigates *some* cases but the comment at `pluginLoader.ts:1973-1981` notes a residual silent fail-open risk (malformed enough to fail validation, readable enough for raw cast). No verbatim contents of `getPluginByIdCacheOnly` are reproduced here — sample read shows it returns `null` when not found, but full implementation not inlined. **DEFERRED** — full implementation reading deferred; the documented residual-risk caveat is sufficient for the loader contract.
3. **`cachePlugin(source, options)`**: signature referenced from `pluginLoader.ts:2327` but its implementation in `marketplaceManager.ts` (or related) was not exhaustively read; the spec assumes it returns `{path, manifest, gitCommitSha?}`. **DEFERRED** — boundary signature is sufficient for this spec; deeper internals belong to a future fetch-path subspec if needed.
4. **`copyPluginToVersionedCache` failure semantics**: on copy failure for local paths, the loader falls back to using the marketplace path directly (`:2275`). This means `pluginPath` may be inside the marketplace repo rather than `<plugins>/cache/`. Downstream code paths that assume a versioned-cache location may behave differently. **NOTE Phase 9.7** — documented at §5.4 and §11 invariant 8 (fallback explicit); no behavior change required.
5. **No plugin signature/integrity verification**: trust is purely policy-based (allowlist/blocklist of marketplace sources; reserved-name source validation). The mcpb handler computes a sha256 hash for de-duplication, not for verification. The GCS path is content-addressed via SHA but uses HTTPS+CDN for transport authenticity. **RESOLVED Phase 9.7** — explicitly stated as "no signature/integrity verification" at §6.22; no source pretends otherwise.
6. **No max plugin count**: no enforced upper bound on installed plugin count or marketplace size. Memory risk on very large `marketplace.json` files is uncited. **RESOLVED Phase 9.7** — confirmed absent from source (`schemas.ts` does not gate `plugins.length`); §6.22 notes this; not a defect.
7. **The 24-variant `PluginError` union vs production**: 12 of 24 variants currently never created; `types/plugin.ts:88-99` notes this is intentional roadmap. The active error display strings are what users see today. **RESOLVED Phase 9.7** — §9.1 enumerates active vs planned; matches source comment.
8. **Cross-plugin dependency closure** (`allowCrossMarketplaceDependenciesOn` in marketplace schema, `:1319-1323`) "only the root marketplace's allowlist applies — no transitive trust"; the resolver implementation in `dependencyResolver.ts` was not read fully. **DEFERRED** — `dependencyResolver.ts` is grep-only per §2.1; full algorithm is appropriate to a follow-up dependency-resolution subspec.
9. **`mcpPluginIntegration.ts` channel-server cross-validation** of `channels[].server` against `mcpServers` keys is asserted (`schemas.ts:670-689`) but its exact algorithm is not inlined here. **DEFERRED to spec 23** — channel/MCP cross-validation belongs to the MCP service spec (`mcpPluginIntegration.ts` is grep-only per §2.1); §6.10 already declares the schema-time vs load-time split. Consumer-side detail will land when spec 23 absorbs the channel-server validation algorithm.

### 12.10 Phase 10 ripple gaps (catalog & plugin manifest UI)

Cross-checked against Phase 10 catalog companions:

- **Catalog companion 21d** (`docs/specs/21d-command-catalog-plugin-and-misc.md`)
  is the catalog-level enumeration of plugin and miscellaneous commands. This
  spec (28) owns the LOADER for plugin-derived commands; **21d owns the
  resulting catalog** (registered names, descriptions, flags). Cross-ref added
  at §1 OUT-of-scope: spec 21 / 21d covers the catalog surface.
- **Plugin marketplace UI**: not enumerated in this spec because the loader is
  UI-agnostic. The `/plugins` REPL UI and marketplace browser live in
  `cli/handlers/plugins.ts` (referenced in §2.3) and the React/Ink components
  under `hooks/useManagePlugins.ts` / `hooks/notifs/usePluginInstallationStatus.tsx`.
  UI behavior is owned by the relevant UI specs (Phase 10c-class enumeration);
  §2.3 already lists the consumers — sufficient for cross-spec routing.
- **`PluginManifest` schema fields**: §6.1 lists every top-level shape via
  composition (`metadata, hooks, commands, agents, skills, outputStyles,
  channels, mcpServers, lspServers, settings, userConfig`); §6.2 through §6.13
  inline each component. No schema field is missing from this spec.
