# 26 — Analytics, Feature Flags & Telemetry Specification

> Owner: sub-F6 · Adjacent: 01, 02, 03, 06, 09, 22, 27. Master plan §3, §5.3, §6.1, §7 Phase 6, §10. Reads: `00-overview.md`, `01-entrypoint-bootstrap.md`, `03-query-engine.md`, `HANDOFF.md` §6.

## 1. Purpose & Scope

This spec covers the analytics, feature-flag, and telemetry pipeline that every other subsystem of the Claude Code CLI calls into. Concretely:

- **Event logging pipeline** — `logEvent`/`logEventAsync` queue → sink fan-out to Datadog (third-party) and the Anthropic 1P event-logging API (BigQuery), with `_PROTO_*` PII-tagged fields hoisted to proto columns by 1P only.
- **GrowthBook client** — initialization, remote-eval payload normalization, periodic refresh, env/disk-cache layering, exposure de-dup, security-gate semantics. Owns `getFeatureValue_CACHED_MAY_BE_STALE` and siblings consumed everywhere (notably spec 03's tool ordering and spec 02's settings).
- **OpenTelemetry stack** — lazy-loaded OTLP exporters (gRPC ~700 KB and HTTP variants), BigQuery metrics exporter, Beta tracing, BasicTracerProvider/MeterProvider/LoggerProvider lifecycle, shutdown timeouts.
- **Perfetto local tracing**, **Stats** (`SHOT_STATS`), **Slow operation logging** (`SLOW_OPERATION_LOGGING`).
- **Three orphan services assigned here per `00-overview.md` §2.3 recommendation**: `services/diagnosticTracking.ts` (IDE diagnostic baseline/diff), `services/internalLogging.ts` (ANT-only k8s/container metadata), `services/notifier.ts` (terminal notifications + `tengu_notification_method_used` event).
- **Flag matrix** for the 89-flag system: build-time DCE via `bun:bundle.feature(...)` vs runtime GrowthBook resolution, plus the named StatSig dynamic configs that drive spec 03's tool-ordering invariant.

Out of scope: tool-call semantics that emit events (every tool spec 10..19 owns its own log call sites); Anthropic API client (22 — see `firstPartyEventLoggingExporter` only inherits `axios` patterns, not the api client); cost/token aggregation (06, ingests `EventMetadata.model`/`betas`); permission-denial tracking (09 owns the call sites; this spec owns `tengu_internal_record_permission_context`'s pipeline); per-feature behavior of any flag listed below — that lives in the owning spec.

## 2. Source Map

### 2.1 IN-scope files (owner = this spec)

| Path | Lines | Coverage |
|---|---|---|
| `src/services/analytics/index.ts` | 174 | full read |
| `src/services/analytics/growthbook.ts` | 1156 | full read |
| `src/services/analytics/firstPartyEventLogger.ts` | 450 | full read |
| `src/services/analytics/firstPartyEventLoggingExporter.ts` | 807 | full read |
| `src/services/analytics/datadog.ts` | 308 | full read |
| `src/services/analytics/sink.ts` | 115 | full read |
| `src/services/analytics/sinkKillswitch.ts` | 26 | full read |
| `src/services/analytics/metadata.ts` | 974 | full read |
| `src/services/analytics/config.ts` | 39 | full read |
| `src/services/diagnosticTracking.ts` | 397 | full read (assigned per 00 §2.3) |
| `src/services/internalLogging.ts` | 90 | full read (assigned per 00 §2.3) |
| `src/services/notifier.ts` | 156 | full read (assigned per 00 §2.3) |
| `src/utils/telemetry/instrumentation.ts` | 825 | full read |
| `src/utils/telemetry/sessionTracing.ts` | 927 | sampled (≤200, lines 1–100, 126–149, 152–340, 479–667 cited) |
| `src/utils/telemetry/events.ts` | 75 | full read |
| `src/utils/telemetry/perfettoTracing.ts` | 1120 | sampled (240–290 cited; 1080+ for cleanup) |
| `src/utils/telemetry/logger.ts` | 26 | grep-confirmed (DiagLogger) |
| `src/utils/telemetry/bigqueryExporter.ts` | 252 | grep-confirmed |
| `src/utils/telemetry/betaSessionTracing.ts` | 491 | grep-confirmed |
| `src/utils/telemetry/skillLoadedEvent.ts` | 39 | grep-confirmed |
| `src/utils/telemetry/pluginTelemetry.ts` | 289 | grep-confirmed |
| `src/utils/stats.ts` | (≥830) | grep-confirmed: `SHOT_STATS` gates at `:131`, `:214`, `:364`, `:610`, `:829` |
| `src/utils/slowOperations.ts` | (≥165) | grep-confirmed: `SLOW_OPERATION_LOGGING` gate at `:157` |
| `src/constants/keys.ts` | 12 | full read |

Assigned StatSig dynamic-config names: `tengu_event_sampling_config` (`firstPartyEventLogger.ts:38`), `tengu_1p_event_batch_config` (`firstPartyEventLogger.ts:87`), `tengu_log_datadog_events` (`sink.ts:20`), `tengu_frond_boric` (`sinkKillswitch.ts:4`).

### 2.2 Imports from / Imported by (high-level)

- Imports from: `bootstrap/state.ts` (sessionId, trust, cleanup-registry hooks), `utils/auth.ts` (subscriptionType, OAuth/api-key probes), `utils/config.ts` (`getGlobalConfig`, `saveGlobalConfig`, `getOrCreateUserID`), `utils/user.ts` (`getCoreUserData`, `getUserForGrowthBook`), `utils/http.ts` (`getAuthHeaders`), `utils/proxy.ts`, `utils/mtls.ts`, `utils/caCerts.ts`, `utils/cleanupRegistry.ts`, `services/oauth/client.ts` (token-expiry probe), `services/mcp/officialRegistry.ts` (PII gating).
- Imported by (selected): `QueryEngine.ts`/`query.ts` (per-turn `logEvent`, `getFeatureValue_CACHED_MAY_BE_STALE`), all of `tools/`, `setup.ts`/`main.tsx` (initialize order), `services/api/*` (22), permission system (09).

### 2.3 Feature-flag and ANT-guard locations (in this spec's IN scope)

| Flag/guard | File:line |
|---|---|
| `feature('PERFETTO_TRACING')` | `utils/telemetry/perfettoTracing.ts:260` |
| `feature('ENHANCED_TELEMETRY_BETA')` | `utils/telemetry/sessionTracing.ts:127` |
| `feature('SHOT_STATS')` | `utils/stats.ts:131,214,364,610,829` |
| `feature('SLOW_OPERATION_LOGGING')` | `utils/slowOperations.ts:157` |
| `feature('COWORKER_TYPE_TELEMETRY')` | `services/analytics/metadata.ts:603,846` |
| `feature('CHICAGO_MCP')` (built-in MCP allowlist) | `metadata.ts:130` |
| `feature('KAIROS')` (kairosActive tag) | `metadata.ts:735` |
| `USER_TYPE === 'ant'` debug logging / k8s | `growthbook.ts:107,163,498,524,538,567,593,604,640,707; firstPartyEventLogger.ts:122,187,202,254,316,418,549,562,576,600; firstPartyEventLoggingExporter.ts:254,263,269,282,366,388,409,418,457,478,493,513,549,562,576,600,622; internalLogging.ts:18,36,75; metadata.ts:205; datadog.ts:205,254` |
| Env-override (ant-only) | `growthbook.ts:163-202` (`CLAUDE_INTERNAL_FC_OVERRIDES`) |

### 2.4 Missing-source / unresolved

- `src/types/generated/events_mono/claude_code/v1/claude_code_internal_event.ts` and `.../growthbook/v1/growthbook_experiment_event.ts` and `.../common/v1/auth.ts` — referenced from `firstPartyEventLoggingExporter.ts:16-17`, `metadata.ts:33-34`. Generated proto bindings; existence in leaked tree unverified — log via §12.

## 3. Public Interface (Contract)

```ts
// services/analytics/index.ts
type AnalyticsSink = {
  logEvent(eventName: string, metadata: LogEventMetadata): void
  logEventAsync(eventName: string, metadata: LogEventMetadata): Promise<void>
}
type LogEventMetadata = { [key: string]: boolean | number | undefined } // index.ts:61

export function attachAnalyticsSink(newSink: AnalyticsSink): void          // idempotent (index.ts:95-123)
export function logEvent(eventName: string, metadata: LogEventMetadata): void
export async function logEventAsync(eventName: string, metadata: LogEventMetadata): Promise<void>
export function stripProtoFields<V>(metadata: Record<string,V>): Record<string,V>  // index.ts:45-58
export type AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS = never
export type AnalyticsMetadata_I_VERIFIED_THIS_IS_PII_TAGGED = never
export function _resetForTesting(): void

// services/analytics/sink.ts
export function initializeAnalyticsGates(): void          // sink.ts:96-99
export function initializeAnalyticsSink(): void           // sink.ts:109-114

// services/analytics/growthbook.ts
export type GrowthBookUserAttributes = {
  id: string; sessionId: string; deviceID: string;
  platform: 'win32'|'darwin'|'linux';
  apiBaseUrlHost?: string; organizationUUID?: string; accountUUID?: string;
  userType?: string; subscriptionType?: string; rateLimitTier?: string;
  firstTokenTime?: number; email?: string; appVersion?: string;
  github?: GitHubActionsMetadata
}                                                          // growthbook.ts:32-47

export function hasGrowthBookEnvOverride(feature: string): boolean
export function getAllGrowthBookFeatures(): Record<string, unknown>
export function getGrowthBookConfigOverrides(): Record<string, unknown>
export function setGrowthBookConfigOverride(feature: string, value: unknown): void  // ant-only, no-op otherwise
export function clearGrowthBookConfigOverrides(): void                              // ant-only
export function getApiBaseUrlHost(): string | undefined
export const initializeGrowthBook: () => Promise<GrowthBook | null>          // memoized
export async function getFeatureValue_DEPRECATED<T>(feature: string, defaultValue: T): Promise<T>
export function getFeatureValue_CACHED_MAY_BE_STALE<T>(feature: string, defaultValue: T): T
export function getFeatureValue_CACHED_WITH_REFRESH<T>(feature: string, defaultValue: T, _refreshIntervalMs: number): T
export function checkStatsigFeatureGate_CACHED_MAY_BE_STALE(gate: string): boolean
export async function checkSecurityRestrictionGate(gate: string): Promise<boolean>
export async function checkGate_CACHED_OR_BLOCKING(gate: string): Promise<boolean>
export function refreshGrowthBookAfterAuthChange(): void
export function resetGrowthBook(): void
export async function refreshGrowthBookFeatures(): Promise<void>
export function setupPeriodicGrowthBookRefresh(): void
export function stopPeriodicGrowthBookRefresh(): void
export async function getDynamicConfig_BLOCKS_ON_INIT<T>(name: string, defaultValue: T): Promise<T>
export function getDynamicConfig_CACHED_MAY_BE_STALE<T>(name: string, defaultValue: T): T
export function onGrowthBookRefresh(listener: () => void|Promise<void>): () => void

// services/analytics/firstPartyEventLogger.ts
export type EventSamplingConfig = { [eventName: string]: { sample_rate: number } }
export function getEventSamplingConfig(): EventSamplingConfig
export function shouldSampleEvent(eventName: string): number | null
export function is1PEventLoggingEnabled(): boolean
export function logEventTo1P(eventName: string, metadata?: ...): void
export function logGrowthBookExperimentTo1P(data: GrowthBookExperimentData): void
export function initialize1PEventLogging(): void
export async function reinitialize1PEventLoggingIfConfigChanged(): Promise<void>
export async function shutdown1PEventLogging(): Promise<void>

// services/analytics/datadog.ts
export const initializeDatadog: () => Promise<boolean>     // memoized
export async function trackDatadogEvent(...): Promise<void>
export async function shutdownDatadog(): Promise<void>

// services/analytics/sinkKillswitch.ts
export type SinkName = 'datadog' | 'firstParty'
export function isSinkKilled(sink: SinkName): boolean

// services/analytics/config.ts
export function isAnalyticsDisabled(): boolean
export function isFeedbackSurveyDisabled(): boolean

// services/analytics/metadata.ts (selected)
export function sanitizeToolNameForAnalytics(toolName: string): ...
export function isToolDetailsLoggingEnabled(): boolean
export function isAnalyticsToolDetailsLoggingEnabled(mcpServerType?, mcpServerBaseUrl?): boolean
export function mcpToolDetailsForAnalytics(toolName, mcpServerType?, mcpServerBaseUrl?): {...}
export function extractMcpToolDetails(toolName: string): { serverName, mcpToolName } | undefined
export function extractSkillName(toolName: string, input: unknown): string | undefined
export function extractToolInputForTelemetry(input: unknown): string | undefined
export function getFileExtensionForAnalytics(filePath: string): string | undefined
export function getFileExtensionsFromBashCommand(command, simulatedSedEditFilePath?): string|undefined
export type EnvContext = { ... 35 fields ... }     // metadata.ts:417-452
export type ProcessMetrics = { ... 9 fields ... }  // metadata.ts:457-467
export type EventMetadata = { ... 23 fields ... }  // metadata.ts:472-496
export async function getEventMetadata(options?: EnrichMetadataOptions): Promise<EventMetadata>
export function to1PEventFormat(metadata, userMetadata, additionalMetadata?): FirstPartyEventLoggingMetadata

// services/diagnosticTracking.ts
export const diagnosticTracker: DiagnosticTrackingService           // singleton (diagnosticTracking.ts:397)
// services/internalLogging.ts
export async function logPermissionContextForAnts(ctx, moment): Promise<void>      // ant-only (:71)
export const getContainerId: () => Promise<string|null>                            // ant-only memoized (:35)
// services/notifier.ts
export async function sendNotification(notif: NotificationOptions, terminal): Promise<void>

// utils/telemetry/instrumentation.ts (selected)
export function bootstrapTelemetry(): void                                          // :87
export function parseExporterTypes(value: string|undefined): string[]
export function isTelemetryEnabled(): boolean                                       // :324 — CLAUDE_CODE_ENABLE_TELEMETRY
export async function initializeTelemetry(): Promise<Meter>
export async function flushTelemetry(): Promise<void>

// utils/telemetry/sessionTracing.ts (selected)
export function isEnhancedTelemetryEnabled(): boolean                               // :126-131
export function startInteractionSpan(...): Span; export function endInteractionSpan(): void
export function startLLMRequestSpan(...): Span; export function endLLMRequestSpan(...): void
export function startToolSpan(...): Span; export function endToolSpan(...): void
export function startBlockedOnUserSpan(...): Span; export function endBlockedOnUserSpan(...): void
export function startToolExecutionSpan(...): Span; export function endToolExecutionSpan(...): void
export function startHookSpan(...): Span; export function endHookSpan(...): void

// constants/keys.ts
export function getGrowthBookClientKey(): string                                    // keys.ts:5-11
```

## 4. Data Model & State

### 4.1 Module-level state (process-singleton)

`services/analytics/index.ts`
- `eventQueue: QueuedEvent[]` (`:81`) — events buffered until sink attaches
- `sink: AnalyticsSink | null` (`:84`)

`services/analytics/growthbook.ts`
- `client: GrowthBook | null` (`:59`)
- `currentBeforeExitHandler / currentExitHandler: (()=>void) | null` (`:62-63`)
- `clientCreatedWithAuth: boolean` (`:67`)
- `experimentDataByFeature: Map<string, StoredExperimentData>` (`:77`) — `{experimentId, variationId, inExperiment?, hashAttribute?, hashValue?}`
- `remoteEvalFeatureValues: Map<string, unknown>` (`:81`) — populated by `processRemoteEvalPayload`, cleared on reset
- `pendingExposures: Set<string>` (`:84`) — features accessed before init
- `loggedExposures: Set<string>` (`:89`) — per-session de-dup; **NOT** cleared by `resetGrowthBook` for `loggedExposures` is **listed as cleared** (`:1004`); `pendingExposures` cleared (`:1003`); refresh signal `refreshed` is **NOT** cleared by `resetGrowthBook` (`:107`)
- `reinitializingPromise: Promise<unknown> | null` (`:94`)
- `envOverrides / envOverridesParsed` (`:167-168`) — parsed once unless reset
- `refreshInterval / beforeExitListener` (`:1017-1018`)
- `GROWTHBOOK_REFRESH_INTERVAL_MS = process.env.USER_TYPE !== 'ant' ? 6h : 20min` (`:1013-1016`)

`services/analytics/firstPartyEventLogger.ts`
- `firstPartyEventLogger: ReturnType<typeof logs.getLogger> | null` (`:105`)
- `firstPartyEventLoggerProvider: LoggerProvider | null` (`:106`)
- `lastBatchConfig: BatchConfig | null` (`:110`)

`services/analytics/firstPartyEventLoggingExporter.ts`
- `BATCH_UUID = randomUUID()` (`:38`) — unique per process; isolates failed-event files between runs
- `FILE_PREFIX = '1p_failed_events.'` (`:41`)
- Per-instance: `pendingExports`, `isShutdown`, `cancelBackoff`, `attempts`, `isRetrying`, `lastExportErrorContext`

`services/analytics/datadog.ts`
- `logBatch: DatadogLog[]` (`:98`); `flushTimer: NodeJS.Timeout | null` (`:99`); `datadogInitialized: boolean | null` (`:100`)

`services/analytics/sink.ts`
- `isDatadogGateEnabled: boolean | undefined` (`:23`); `DATADOG_GATE_NAME = 'tengu_log_datadog_events'` (`:20`)

`services/analytics/metadata.ts`
- Memoized: `getVersionBase` (`:566`), `buildEnvContext` (`:574`)
- `prevCpuUsage`, `prevWallTimeMs` for CPU% delta (`:642-643`)

`services/diagnosticTracking.ts` — `DiagnosticTrackingService` singleton (`:31, :397`):
- `baseline: Map<string, Diagnostic[]>`
- `lastProcessedTimestamps: Map<string, number>`
- `rightFileDiagnosticsState: Map<string, Diagnostic[]>`
- `initialized: boolean`, `mcpClient?: MCPServerConnection`

`utils/telemetry/sessionTracing.ts`
- `interactionContext`, `toolContext` ALS stores (`:69-70`)
- `activeSpans: Map<string, WeakRef<SpanContext>>` (`:71`)
- `strongSpans: Map<string, SpanContext>` (`:75`)
- `interactionSequence` (`:76`); `_cleanupIntervalStarted` (`:77`)
- `SPAN_TTL_MS = 30 * 60 * 1000` (30 minutes; `:79`)

### 4.2 Persistent state (on disk)

| Path | Producer | Format |
|---|---|---|
| `~/.claude.json#cachedGrowthBookFeatures` | `growthbook.ts:syncRemoteEvalToDisk():407-417` (wholesale replace) | JSON map of feature key → resolved value |
| `~/.claude.json#cachedStatsigGates` | populated by legacy migration consumer | JSON map; read by `checkStatsigFeatureGate_CACHED_MAY_BE_STALE`, `checkSecurityRestrictionGate` |
| `~/.claude.json#growthBookOverrides` (ant-only) | `setGrowthBookConfigOverride`/`clearGrowthBookConfigOverrides` | JSON; managed via `/config` Gates tab |
| `<getClaudeConfigHomeDir()>/telemetry/1p_failed_events.<sessionId>.<BATCH_UUID>.json` | `firstPartyEventLoggingExporter.queueFailedEvents → appendEventsToFile` (`:430-443`) | JSON-Lines |
| `<getClaudeConfigHomeDir()>/traces/trace-<sessionId>.json` | `perfettoTracing.ts:initializePerfettoTracing:273-274` | Perfetto/Chrome JSON trace |

### 4.3 Lifecycle

```
PROCESS START
  → analytics index.ts: queue events; sink null
  → setup.ts → setupBackend()
       → bootstrapTelemetry() (instrumentation.ts:87) re-maps ANT_OTEL_* → OTEL_*
       → initializeTelemetry() (lazy-imports OTLP exporters per protocol)
       → initializeAnalyticsGates() (sink.ts:96 — read tengu_log_datadog_events from disk cache)
       → initialize1PEventLogging() (firstPartyEventLogger.ts:312)
       → initializeAnalyticsSink() (sink.ts:109 — attachAnalyticsSink, drains queue via queueMicrotask)
       → initializeGrowthBook() (memoized; remoteEval init, then setupPeriodicGrowthBookRefresh)

QUERY / TURN
  → logEvent(name, meta) → sink.logEventImpl
       → shouldSampleEvent(name) (consults tengu_event_sampling_config)
       → if isSinkKilled('datadog')==false ∧ tengu_log_datadog_events==true → trackDatadogEvent(stripProtoFields(...))
       → logEventTo1P (full payload incl. _PROTO_*; emits OTel log record into firstPartyEventLoggerProvider)

GROWTHBOOK PERIODIC (20 min ANT / 6 h non-ANT)
  → refreshGrowthBookFeatures() → processRemoteEvalPayload → syncRemoteEvalToDisk → refreshed.emit()
  → onGrowthBookRefresh subscribers (e.g. reinitialize1PEventLoggingIfConfigChanged) re-build LoggerProvider

AUTH CHANGE (login/logout)
  → refreshGrowthBookAfterAuthChange() → resetGrowthBook (preserves refreshed signal subscribers)
                                       → reinitializingPromise tracked for security-gate awaits

PROCESS EXIT
  → cleanupRegistry runs shutdownTelemetry (≤ CLAUDE_CODE_OTEL_SHUTDOWN_TIMEOUT_MS = 2000)
  → shutdownDatadog (drain logBatch)
  → shutdown1PEventLogging (forceFlush + provider.shutdown)
  → process.on('beforeExit'/'exit') destroys GrowthBook client
```

## 5. Algorithm / Control Flow

### 5.1 `logEvent` fan-out (sink.ts:48-72)

```
logEvent(name, meta):
  sampleResult = shouldSampleEvent(name)
  if sampleResult === 0: return                       // dropped by sampling
  metaSampled = sampleResult==null ? meta : {...meta, sample_rate: sampleResult}
  if shouldTrackDatadog():                            // killswitch + cached gate
    trackDatadogEvent(name, stripProtoFields(metaSampled))
  logEventTo1P(name, metaSampled)                     // full payload incl. _PROTO_*
```

`shouldSampleEvent` (firstPartyEventLogger.ts:57-85): missing config or rate==1 → return null (no sampling); rate≤0 → return 0 (drop); else `Math.random() < rate ? rate : 0`. Validation: numeric ∧ 0 ≤ rate ≤ 1 else fall through to no-sampling.

### 5.2 Sink killswitch (sinkKillswitch.ts)

`isSinkKilled('datadog' | 'firstParty')` reads `tengu_frond_boric` (JSON `{datadog?:bool, firstParty?:bool}`) via `getDynamicConfig_CACHED_MAY_BE_STALE`. Strict `=== true` comparison (cached JSON null leaks past `!== undefined` else-branch).

### 5.3 GrowthBook resolution priority

`getFeatureValue_CACHED_MAY_BE_STALE` (`growthbook.ts:734-775`):
```
1. envOverrides[feature]      (CLAUDE_INTERNAL_FC_OVERRIDES, ant-only, parsed once)
2. configOverrides[feature]   (~/.claude.json#growthBookOverrides, ant-only, /config Gates tab)
3. !isGrowthBookEnabled() → defaultValue
4. log/defer experiment exposure (loggedExposures or pendingExposures)
5. remoteEvalFeatureValues.get(feature)         (in-memory authoritative after init)
6. getGlobalConfig().cachedGrowthBookFeatures[feature]   (disk cache)
7. defaultValue
```

Async `getFeatureValueInternal` (used by `_DEPRECATED`, `checkGate_CACHED_OR_BLOCKING`): same priority but blocks on `initializeGrowthBook` and uses `growthBookClient.getFeatureValue` only if remoteEval cache miss.

`checkStatsigFeatureGate_CACHED_MAY_BE_STALE` (`:804-836`): same prefix, then checks `cachedGrowthBookFeatures` first, falls back to `cachedStatsigGates`.

`checkSecurityRestrictionGate` (`:851-889`): awaits `reinitializingPromise` if set, then `cachedStatsigGates` first (safety bias), then `cachedGrowthBookFeatures`, else `false`.

`checkGate_CACHED_OR_BLOCKING` (`:904-935`): if disk cache says `true`, return immediately; else block on `getFeatureValueInternal(gate, false, true)`.

### 5.4 GrowthBook init (`getGrowthBookClient` memoized at `:490-617`)

```
1. !isGrowthBookEnabled() → null
2. attributes = getUserAttributes()
3. clientKey = getGrowthBookClientKey()
   apiHost  = USER_TYPE==='ant' ? (CLAUDE_CODE_GB_BASE_URL || 'https://api.anthropic.com/')
                                : 'https://api.anthropic.com/'
4. trustOK = checkHasTrustDialogAccepted() || getSessionTrustAccepted() || getIsNonInteractiveSession()
   authHeaders = trustOK ? getAuthHeaders() : {error:'trust not established'}
   clientCreatedWithAuth = !authHeaders.error
5. new GrowthBook({apiHost, clientKey, attributes, remoteEval:true,
                   cacheKeyAttributes:['id','organizationUUID'],
                   apiHostRequestHeaders:authHeaders.headers (if hasAuth),
                   log: ant ? logForDebugging : undefined})
6. if !hasAuth: return {client, initialized:Promise.resolve()}    // disk-cache only
7. initialized = client.init({timeout:5000})
                   .then(processRemoteEvalPayload → syncRemoteEvalToDisk → refreshed.emit())
                   .catch(logError) // ant-only
8. process.on('beforeExit', client.destroy); process.on('exit', client.destroy)
```

`processRemoteEvalPayload` (`:327-394`):
- Reads payload via `client.getPayload()`. Empty `{features:{}}` → return false (no clear, no disk write — prevents flag blackout from transient server bug).
- Clears `experimentDataByFeature`.
- Transforms `{value}` shape → `{defaultValue, value}` (API workaround) and stores experiment metadata `{experimentId:exp.key, variationId:expResult.variationId}` when `f.source==='experiment'`.
- `client.setPayload(...)` (await), then re-checks `client !== thisClient` (replaced-client guard).
- Rebuilds `remoteEvalFeatureValues` from `value ?? defaultValue`. Returns true.

`initializeGrowthBook` (memoized; `:622-664`): if `!clientCreatedWithAuth` and trust now OK and auth available → `resetGrowthBook()` then `getGrowthBookClient()`. Awaits `clientWrapper.initialized`. Calls `setupPeriodicGrowthBookRefresh()`.

### 5.5 Periodic refresh (`growthbook.ts:1013-1109`)

- Interval: `6h` non-ANT, `20min` ANT (constant).
- `refreshGrowthBookFeatures()` calls `client.refreshFeatures()` → `processRemoteEvalPayload` → if `hadFeatures`: `syncRemoteEvalToDisk()` + `refreshed.emit()`.
- Replaced-client guards at both `client.refreshFeatures()` boundary and after `processRemoteEvalPayload`'s `setPayload` await.
- `setupPeriodicGrowthBookRefresh()` clears any prior `refreshInterval`, creates a new `setInterval` (unref'd), registers `process.once('beforeExit', stopPeriodicGrowthBookRefresh)` exactly once.

### 5.6 `refreshGrowthBookAfterAuthChange` (`:943-982`)

```
resetGrowthBook()        // destroys client, clears all maps, removes process handlers
refreshed.emit()         // notify subscribers (post-reset state)
reinitializingPromise = initializeGrowthBook().catch(logError → null).finally(=>null)
```

`onGrowthBookRefresh` (`:139-157`): subscribes to `refreshed` signal; if `remoteEvalFeatureValues.size > 0` at registration time, fires once on next microtask (catch-up for late REPL mount). Subscribers do their own change detection (`isEqual` against last-seen). **Not cleared by `resetGrowthBook`** — subscribers register once in `init.ts` and survive auth resets.

### 5.7 1P event logging pipeline

`initialize1PEventLogging()` (`firstPartyEventLogger.ts:312-389`):
1. `enabled = !isAnalyticsDisabled()` else early return.
2. `batchConfig = getBatchConfig()` (`tengu_1p_event_batch_config`) → `lastBatchConfig = batchConfig`.
3. `scheduledDelayMillis = batchConfig.scheduledDelayMillis || parseInt(OTEL_LOGS_EXPORT_INTERVAL || '10000')` (default `DEFAULT_LOGS_EXPORT_INTERVAL_MS = 10000`).
4. `maxExportBatchSize = batchConfig.maxExportBatchSize || 200` (`DEFAULT_MAX_EXPORT_BATCH_SIZE`).
5. `maxQueueSize = batchConfig.maxQueueSize || 8192` (`DEFAULT_MAX_QUEUE_SIZE`).
6. Build resource: `{ATTR_SERVICE_NAME:'claude-code', ATTR_SERVICE_VERSION:MACRO.VERSION}` plus `{wsl.version}` if WSL.
7. `eventLoggingExporter = new FirstPartyEventLoggingExporter({maxBatchSize, skipAuth, maxAttempts, path, baseUrl, isKilled:()=>isSinkKilled('firstParty')})`.
8. `firstPartyEventLoggerProvider = new LoggerProvider({resource, processors:[new BatchLogRecordProcessor(eventLoggingExporter, {scheduledDelayMillis, maxExportBatchSize, maxQueueSize})]})`.
9. `firstPartyEventLogger = provider.getLogger('com.anthropic.claude_code.events', MACRO.VERSION)`.
   This is **NOT** registered globally (`logs.setGlobalLoggerProvider`) so customer telemetry stays separated.

`reinitialize1PEventLoggingIfConfigChanged` (`:407-449`): on `tengu_1p_event_batch_config` change, null logger first → `oldProvider.forceFlush()` → null provider → `initialize1PEventLogging()` (rolling back on throw to keep `oldProvider` viable). Disk-backed retry survives the swap (`BATCH_UUID + sessionId` filename is stable across reinit).

### 5.8 Exporter (FirstPartyEventLoggingExporter)

Constructor defaults (`:111-136`):
| Option | Default |
|---|---|
| `baseUrl` | `'https://api-staging.anthropic.com'` if `ANTHROPIC_BASE_URL===staging` else `'https://api.anthropic.com'` |
| `path` | `'/api/event_logging/batch'` |
| `timeout` | `10000` ms |
| `maxBatchSize` | `200` |
| `skipAuth` | `false` |
| `batchDelayMs` | `100` |
| `baseBackoffDelayMs` | `500` |
| `maxBackoffDelayMs` | `30000` |
| `maxAttempts` | `8` |

Backoff: quadratic `min(base * attempts², maxBackoffDelayMs)` (`:451-455`).

Auth choice (`sendBatchWithRetry:527-614`):
- `isKilled()` → throw → caller short-circuits remaining batches and writes them all to disk.
- Headers always include `'Content-Type': 'application/json'`, `'User-Agent': getClaudeCodeUserAgent()`, `'x-service-name': 'claude-code'`.
- `shouldSkipAuth = skipAuth || !hasTrust || (isClaudeAISubscriber && !hasProfileScope) || (isClaudeAISubscriber && tokens && isOAuthTokenExpired)`.
- If with-auth POST returns `401`: retry once without auth.
- On any other failure: throw → caller writes failed-batch + remaining batches to disk and schedules backoff.

Background retries (`retryFileInBackground`, `retryFailedEvents`): drain failed-events file in a loop while not shutdown, deleting the file before retry (events held in memory). On success: `resetBackoff()`. On retry exhaustion: `deleteFile`, drop. `attempts ≥ maxAttempts` short-circuits with `Dropped N events: max attempts (8) reached`.

`transformLogsToEvents` (`:635-762`): Filters `instrumentationScope.name === 'com.anthropic.claude_code.events'`. Two event types:
- `'GrowthbookExperimentEvent'` → `GrowthbookExperimentEvent.toJSON({event_id, timestamp, experiment_id, variation_id, environment, user_attributes, experiment_metadata, device_id, session_id, auth?})`.
- `'ClaudeCodeInternalEvent'` (default) → `to1PEventFormat(coreMetadata, userMetadata, eventMetadata)` then destructure known `_PROTO_skill_name`, `_PROTO_plugin_name`, `_PROTO_marketplace_name` → proto fields; remainder run through `stripProtoFields(rest)` → `additional_metadata` Base64-JSON.

### 5.9 Datadog dispatch (`datadog.ts`)

- `DATADOG_LOGS_ENDPOINT = 'https://http-intake.logs.us5.datadoghq.com/api/v2/logs'` (`:13`)
- `DATADOG_CLIENT_TOKEN = 'pubbbf48e6d78dae54bceaa4acf463299bf'` (`:14`)
- `DEFAULT_FLUSH_INTERVAL_MS = 15000`, `MAX_BATCH_SIZE = 100`, `NETWORK_TIMEOUT_MS = 5000` (`:15-17`)
- `NUM_USER_BUCKETS = 30` (`:281`)
- Override: `CLAUDE_CODE_DATADOG_FLUSH_INTERVAL_MS` env var (`:303-305`)

`trackDatadogEvent` (`:160-279`):
1. `process.env.NODE_ENV !== 'production'` → return.
2. `getAPIProvider() !== 'firstParty'` → return (no Bedrock/Vertex/Foundry sends).
3. `await initializeDatadog()` (memoized; just sets the flag).
4. `!DATADOG_ALLOWED_EVENTS.has(eventName)` → return. (See §6 for full allow-list.)
5. `getEventMetadata({model, betas})`. Destructure `envContext` and flatten alongside `properties`. Add `userBucket = sha256(userId).slice(0,8) % 30`.
6. Cardinality reduction: `toolName` starting `mcp__` → `'mcp'`; non-ANT model name normalized via `getCanonicalName(model.replace(/\[1m]$/i,''))` → if not in `MODEL_COSTS` → `'other'`; dev version `^(\d+\.\d+\.\d+-dev\.\d{8})\.t\d+\.sha[a-f0-9]+$` → keep `$1`.
7. `status` field renamed: integer first-digit ∈ {1..5} → `http_status_range = '<digit>xx'`, raw → `http_status`; original `status` deleted.
8. `ddtags = ['event:<name>', ...TAG_FIELDS.filter(present).map(camelToSnakeCase)]`.
9. Build `DatadogLog{ddsource:'nodejs', ddtags, message:eventName, service:'claude-code', hostname:'claude-code', env:USER_TYPE, ...allData}`.
10. Push; flush immediately if `length >= 100` else schedule `setTimeout(..., 15s).unref()`.

### 5.10 OTel initialization (instrumentation.ts:421-700)

Call chain inside `initializeTelemetry()`:
1. `bootstrapTelemetry()` re-maps any `ANT_OTEL_*` env vars → `OTEL_*` (ANT-only). Sets default temporality to `'delta'` if unset.
2. If `getHasFormattedOutput()` (stream-json / SDK mode): strip `'console'` from `OTEL_METRICS_EXPORTER`/`OTEL_LOGS_EXPORTER`/`OTEL_TRACES_EXPORTER`.
3. `diag.setLogger(new ClaudeCodeDiagLogger(), DiagLogLevel.ERROR)`.
4. `initializePerfettoTracing()` (independent code path; see §5.11).
5. If `isTelemetryEnabled()` (CLAUDE_CODE_ENABLE_TELEMETRY truthy): readers ← `getOtlpReaders()` (per-protocol lazy-import).
6. If `isBigQueryMetricsEnabled()` (1P API customer ∨ C4E ∨ team subscriber): readers ← `getBigQueryExportingReader()` (5-minute interval).
7. Build base resource: `{service.name:'claude-code', service.version:MACRO.VERSION, wsl.version?}`. Merge OS, host.arch, env detector resources (right-side wins).
8. If `isBetaTracingEnabled()`: `initializeBetaTracing(resource)` (separate path; uses `BETA_TRACING_ENDPOINT`, only http/protobuf via OTLP HTTP exporters).
9. Else: standard MeterProvider/LoggerProvider/TracerProvider built (TracerProvider only if `isEnhancedTelemetryEnabled()` ∧ telemetryEnabled).
10. `registerCleanup(shutdownTelemetry)` — race `Promise.all([providers shutdown])` vs `telemetryTimeout(CLAUDE_CODE_OTEL_SHUTDOWN_TIMEOUT_MS||2000)`.

Lazy-load policy (per-protocol):

| Protocol | gRPC import (~700 KB) | http/json | http/protobuf |
|---|---|---|---|
| metrics | `@opentelemetry/exporter-metrics-otlp-grpc` | `@opentelemetry/exporter-metrics-otlp-http` | `@opentelemetry/exporter-metrics-otlp-proto` |
| logs | `@opentelemetry/exporter-logs-otlp-grpc` | `@opentelemetry/exporter-logs-otlp-http` | `@opentelemetry/exporter-logs-otlp-proto` |
| traces | `@opentelemetry/exporter-trace-otlp-grpc` | `@opentelemetry/exporter-trace-otlp-http` | `@opentelemetry/exporter-trace-otlp-proto` |
| metrics(prom) | — | — | `@opentelemetry/exporter-prometheus` |

Default OTLP intervals: metrics `60000` ms (`OTEL_METRIC_EXPORT_INTERVAL`), logs `5000` ms (`OTEL_LOGS_EXPORT_INTERVAL`), traces `5000` ms (`OTEL_TRACES_EXPORT_INTERVAL`).

### 5.11 Perfetto tracing (perfettoTracing.ts:240-290)

`feature('PERFETTO_TRACING')` build-time gate. Trace path: `<getClaudeConfigHomeDir()>/traces/trace-<sessionId>.json` if env is truthy `1`, else env value used as path. Optional periodic full-trace write at `CLAUDE_CODE_PERFETTO_WRITE_INTERVAL_S` seconds. Eviction events emit `{name:'perfetto_eviction', dropped_events:N}` markers (`:240`).

### 5.12 Notifier (`services/notifier.ts`)

`sendNotification(notif, terminal)` (`:18-36`): runs `executeNotificationHooks(notif)`, calls `sendToChannel(channel,…)`, emits `tengu_notification_method_used` with `{configured_channel, method_used, term}`. Channels: `auto`, `iterm2`, `iterm2_with_bell`, `kitty`, `ghostty`, `terminal_bell`, `notifications_disabled`. `sendAuto` resolves: `Apple_Terminal` → bell if profile bell not disabled (else `'no_method_available'`); `iTerm.app` → iterm2; `kitty` → kitty (random `id` 0..9999); `ghostty` → ghostty; default → `'no_method_available'`. `'plist'` is dynamically imported (`:138`) only on Apple_Terminal+auto.

### 5.13 Diagnostic tracking (`services/diagnosticTracking.ts`)

Singleton holding three maps. `beforeFileEdited(filePath)` calls IDE RPC `getDiagnostics({uri:'file://'+filePath})`, normalizes path (strips `file://`, `_claude_fs_right:`, `_claude_fs_left:` prefixes; uses `pathsEqual` for Windows case-insensitivity), sets baseline. `getNewDiagnostics()` fetches all diagnostics, prefers `_claude_fs_right:` URIs when their diagnostic state has changed since last fetch, returns only diagnostics not in baseline. Equality: `(message, severity, source, code, range.start.line, range.start.character, range.end.line, range.end.character)`. Format severity symbols via `figures.cross|warning|info|star|bullet`. Truncates at `MAX_DIAGNOSTICS_SUMMARY_CHARS = 4000` with marker `'…[truncated]'`.

## 6. Verbatim Assets

### 6.1 GrowthBook init constants

| Constant | Value | Cite |
|---|---|---|
| Client key (ant prod) | `'sdk-xRVcrliHIlrg4og4'` | `constants/keys.ts:9` |
| Client key (ant dev when `ENABLE_GROWTHBOOK_DEV` truthy) | `'sdk-yZQvlplybuXjYh6L'` | `constants/keys.ts:8` |
| Client key (non-ANT) | `'sdk-zAZezfDKGoZuXXKe'` | `constants/keys.ts:10` |
| API host (ant) | `process.env.CLAUDE_CODE_GB_BASE_URL || 'https://api.anthropic.com/'` | `growthbook.ts:504` |
| API host (non-ANT) | `'https://api.anthropic.com/'` | `growthbook.ts:506` |
| Init timeout | `5000` ms | `growthbook.ts:555` |
| Periodic refresh (ANT) | `20 * 60 * 1000` (20 min) | `growthbook.ts:1016` |
| Periodic refresh (non-ANT) | `6 * 60 * 60 * 1000` (6 h) | `growthbook.ts:1015` |
| `cacheKeyAttributes` | `['id', 'organizationUUID']` | `growthbook.ts:532` |
| Constructor flag | `remoteEval: true` | `growthbook.ts:530` |
| Env-override env var (ant-only) | `CLAUDE_INTERNAL_FC_OVERRIDES` | `growthbook.ts:174` |

### 6.2 1P event-logging constants

| Constant | Value | Cite |
|---|---|---|
| Endpoint path | `'/api/event_logging/batch'` | `firstPartyEventLoggingExporter.ts:120` |
| Base URL (default) | `'https://api.anthropic.com'` | `:117` |
| Base URL (when `ANTHROPIC_BASE_URL==='https://api-staging.anthropic.com'`) | `'https://api-staging.anthropic.com'` | `:116-117` |
| Failed-events file prefix | `'1p_failed_events.'` | `:41` |
| Batch UUID | `randomUUID()` per process | `:38` |
| Storage dir | `path.join(getClaudeConfigHomeDir(), 'telemetry')` | `:45` |
| Default `timeout` | `10000` ms | `:122` |
| Default `maxBatchSize` | `200` | `:123` |
| Default `batchDelayMs` | `100` | `:125` |
| Default `baseBackoffDelayMs` | `500` | `:126` |
| Default `maxBackoffDelayMs` | `30000` | `:127` |
| Default `maxAttempts` | `8` | `:128` |
| Logger scope | `'com.anthropic.claude_code.events'` | `firstPartyEventLogger.ts:386` |
| Default scheduledDelay | `OTEL_LOGS_EXPORT_INTERVAL || 10000` | `:329-334` |
| Default maxExportBatchSize | `200` | `:336-337` |
| Default maxQueueSize | `8192` | `:339` |
| Required headers | `Content-Type: application/json`, `User-Agent: getClaudeCodeUserAgent()`, `x-service-name: claude-code` | `firstPartyEventLoggingExporter.ts:538-542` |
| Environment string | `'production'` (only env reachable via `api.anthropic.com`) | `firstPartyEventLogger.ts:245-247` |

### 6.3 Datadog constants

| Constant | Value | Cite |
|---|---|---|
| Logs endpoint | `'https://http-intake.logs.us5.datadoghq.com/api/v2/logs'` | `datadog.ts:13` |
| Client token | `'pubbbf48e6d78dae54bceaa4acf463299bf'` | `:14` |
| Default flush interval | `15000` ms | `:15` |
| Max batch size | `100` | `:16` |
| Network timeout | `5000` ms | `:17` |
| User-bucket count | `30` (sha256 of userId, first 8 hex chars `% 30`) | `:281,295-299` |
| Override env var | `CLAUDE_CODE_DATADOG_FLUSH_INTERVAL_MS` | `:304` |
| `ddsource` | `'nodejs'`; `service`/`hostname` `'claude-code'`; `env: process.env.USER_TYPE` | `:248-255` |
| Dev-version regex | `/^(\d+\.\d+\.\d+-dev\.\d{8})\.t\d+\.sha[a-f0-9]+$/` | `:213-216` |

`DATADOG_ALLOWED_EVENTS` (verbatim, `datadog.ts:19-64`, count = 44 — corrected Phase 9.6 from prior "41"; 7 `chrome_bridge_*` + 37 `tengu_*`):
```
chrome_bridge_connection_succeeded, chrome_bridge_connection_failed,
chrome_bridge_disconnected, chrome_bridge_tool_call_completed,
chrome_bridge_tool_call_error, chrome_bridge_tool_call_started,
chrome_bridge_tool_call_timeout, tengu_api_error, tengu_api_success,
tengu_brief_mode_enabled, tengu_brief_mode_toggled, tengu_brief_send,
tengu_cancel, tengu_compact_failed, tengu_exit, tengu_flicker, tengu_init,
tengu_model_fallback_triggered, tengu_oauth_error, tengu_oauth_success,
tengu_oauth_token_refresh_failure, tengu_oauth_token_refresh_success,
tengu_oauth_token_refresh_lock_acquiring, tengu_oauth_token_refresh_lock_acquired,
tengu_oauth_token_refresh_starting, tengu_oauth_token_refresh_completed,
tengu_oauth_token_refresh_lock_releasing, tengu_oauth_token_refresh_lock_released,
tengu_query_error, tengu_session_file_read, tengu_started,
tengu_tool_use_error, tengu_tool_use_granted_in_prompt_permanent,
tengu_tool_use_granted_in_prompt_temporary, tengu_tool_use_rejected_in_prompt,
tengu_tool_use_success, tengu_uncaught_exception, tengu_unhandled_rejection,
tengu_voice_recording_started, tengu_voice_toggled,
tengu_team_mem_sync_pull, tengu_team_mem_sync_push,
tengu_team_mem_sync_started, tengu_team_mem_entries_capped
```

`TAG_FIELDS` (`datadog.ts:66-83`):
```
arch, clientType, errorType, http_status_range, http_status, kairosActive, model,
platform, provider, skillMode, subscriptionType, toolName, userBucket, userType,
version, versionBase
```

### 6.4 OTel + lazy-load constants

| Constant | Value | Cite |
|---|---|---|
| Env: enable customer telemetry | `CLAUDE_CODE_ENABLE_TELEMETRY` (truthy) | `instrumentation.ts:325` |
| Default metrics interval | `60000` ms | `:69, :131-135` |
| Default logs interval | `5000` ms | `:70` |
| Default traces interval | `5000` ms | `:71` |
| Shutdown timeout | `CLAUDE_CODE_OTEL_SHUTDOWN_TIMEOUT_MS || 2000` | `:529, :656` |
| Flush timeout | `CLAUDE_CODE_OTEL_FLUSH_TIMEOUT_MS || 5000` | `:714` |
| BigQuery exporter interval | `5 * 60 * 1000` (5 min) | `:332` |
| Default temporality | `'delta'` (set on `OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE` if unset) | `:114-115` |
| Beta tracing env | `BETA_TRACING_ENDPOINT`, `ENABLE_BETA_TRACING_DETAILED` | `:356`, `:514` |
| Tracer name | `'com.anthropic.claude_code.tracing'`, version `'1.0.0'` | `sessionTracing.ts:153` |
| Meter name | `'com.anthropic.claude_code'`, version `MACRO.VERSION` | `instrumentation.ts:563, :700` |
| Logger scope (3P customer telemetry) | `'com.anthropic.claude_code.events'`, `MACRO.VERSION` | `:404, :603` |
| User-prompt logging env | `OTEL_LOG_USER_PROMPTS` (truthy) | `events.ts:14` |
| Tool-detail logging env | `OTEL_LOG_TOOL_DETAILS` (truthy) | `metadata.ts:87` |

ANT-only env remap (build-time prefix → runtime equivalent; `instrumentation.ts:88-110`):
```
ANT_OTEL_METRICS_EXPORTER       → OTEL_METRICS_EXPORTER
ANT_OTEL_LOGS_EXPORTER          → OTEL_LOGS_EXPORTER
ANT_OTEL_TRACES_EXPORTER        → OTEL_TRACES_EXPORTER
ANT_OTEL_EXPORTER_OTLP_PROTOCOL → OTEL_EXPORTER_OTLP_PROTOCOL
ANT_OTEL_EXPORTER_OTLP_ENDPOINT → OTEL_EXPORTER_OTLP_ENDPOINT
ANT_OTEL_EXPORTER_OTLP_HEADERS  → OTEL_EXPORTER_OTLP_HEADERS
```

### 6.5 OTel span names (verbatim)

`utils/telemetry/sessionTracing.ts`:
- `'claude_code.interaction'` (`:216`)
- `'claude_code.llm_request'` (`:319`)
- `'claude_code.tool'` (`:505`)
- `'claude_code.tool.blocked_on_user'` (`:559`)
- `'claude_code.tool.execution'` (`:640`)
- `'claude_code.hook'` (`:867`)
- Generic span name: `spanName` parameter to `withSpan(spanName, …)` (`:807`)
- `'dummy'` placeholder (when no active span; `:187,199,292,304,479,491,535,547,628,794,851`)

Span-attribute key conventions (snake-cased; not exhaustively enumerated):
- `interaction.sequence`, `interaction.duration_ms`, `llm_request.context: 'interaction'|'standalone'`, `span.type: 'tool'|'tool.blocked_on_user'|'tool.execution'|'hook'|'interaction'|'llm_request'`.

### 6.6 1P-only event types and BigQuery proto schemas

The exporter emits two top-level proto envelopes (`firstPartyEventLoggingExporter.ts:49-56`):
```
type FirstPartyEventLoggingEvent = {
  event_type: 'ClaudeCodeInternalEvent' | 'GrowthbookExperimentEvent'
  event_data: unknown   // proto JSON via toJSON()
}
type FirstPartyEventLoggingPayload = { events: FirstPartyEventLoggingEvent[] }
```

`ClaudeCodeInternalEvent` fields populated by `transformLogsToEvents` (`:728-758`): `event_id`, `event_name`, `client_timestamp`, `device_id`, `email`, `auth`, `core` (spread), `env`, `process`, `skill_name`, `plugin_name`, `marketplace_name`, `additional_metadata` (Base64-JSON, only when non-empty after `_PROTO_*` strip).

`GrowthbookExperimentEvent` fields (`:644-668`): `event_id`, `timestamp`, `experiment_id`, `variation_id`, `environment`, `user_attributes`, `experiment_metadata`, `device_id`, `session_id`, `auth?:{account_uuid?, organization_uuid?}`.

### 6.7 EnvContext / ProcessMetrics / EventMetadata (verbatim from §3 above)

35-field `EnvContext` (`metadata.ts:417-452`); 9-field `ProcessMetrics` (`:457-467`); 23-field `EventMetadata` (`:472-496`). Snake-case 1P transform `to1PEventFormat` at `:796-973`.

Tool-input truncation constants for OTel events (`metadata.ts:236-240`):
```
TOOL_INPUT_STRING_TRUNCATE_AT = 512
TOOL_INPUT_STRING_TRUNCATE_TO = 128
TOOL_INPUT_MAX_JSON_CHARS     = 4 * 1024
TOOL_INPUT_MAX_COLLECTION_ITEMS = 20
TOOL_INPUT_MAX_DEPTH          = 2
MAX_FILE_EXTENSION_LENGTH     = 10
```

Bash-command extraction allow-list (`metadata.ts:340-358`):
```
rm, mv, cp, touch, mkdir, chmod, chown, cat, head, tail, sort,
stat, diff, wc, grep, rg, sed
```

Compound-operator regex: `/\s*(?:&&|\|\||[;|])\s*/`; whitespace regex: `/\s+/`.

### 6.8 StatSig dynamic configs (named, all in IN scope)

| Config name | Read at | Owner spec | Default fallback |
|---|---|---|---|
| `tengu_event_sampling_config` | `firstPartyEventLogger.ts:38, :44-47` | 26 (this) | `{}` |
| `tengu_1p_event_batch_config` | `firstPartyEventLogger.ts:87, :97-101` | 26 | `{}` |
| `tengu_log_datadog_events` | `sink.ts:20, :39, :98` | 26 | `false` |
| `tengu_frond_boric` (sink killswitch JSON) | `sinkKillswitch.ts:4, :19` | 26 | `{}` |
| `claude_code_global_system_caching` | `tools.ts:191` (comment) | 03/08 (consumed here for invariant) | per `tools.ts` |
| `claude_code_system_cache_policy` | `tools.ts:354-365` (comment) | 03/08 | per `tools.ts` |

Spec 03's tool-ordering invariant (`getAllBaseTools` order ≡ `claude_code_global_system_caching`) is **enforced by reading the StatSig config name at this layer** even though the tool registry is owned by 08. The flag-resolution policy here decides reorder vs cache-invalidate: reordering `getAllBaseTools` invalidates the global system-prompt cache for all users.

### 6.9 Enumerated `tengu_*` configs — partial

`grep` on the `src/` tree finds **817 unique** `'tengu_*'` literals (full enumeration impractical; the strings live across all subsystems). The configs explicitly named in this spec's IN scope:
```
tengu_1p_event_batch_config         tengu_birch_trellis
tengu_cache_plum_violet             tengu_cobalt_lantern
tengu_copper_panda                  tengu_event_sampling_config
tengu_frond_boric                   tengu_glacier_2xr
tengu_hive_evidence                 tengu_log_datadog_events
tengu_otk_slot_v1                   tengu_plum_vx3
tengu_turtle_carbon                 tengu_internal_record_permission_context
tengu_notification_method_used
```
The full enumeration is pinned to 817 distinct names at audit time (Phase 9 cross-check). Each individual flag's behavior lives in its owning spec; this spec owns only the named StatSig configs and the resolution policy.

### 6.10 PII strip helper (`index.ts:45-58`)

```
stripProtoFields<V>(metadata):
  let result; for key in metadata:
    if key.startsWith('_PROTO_'):
      result ??= {...metadata}; delete result[key]
  return result ?? metadata
```
Applied at sink.ts before Datadog (`sink.ts:64-66`) and inside `firstPartyEventLoggingExporter.transformLogsToEvents` after destructuring known proto-keys (`:719-725`). Datadog never sees `_PROTO_*` values; 1P exporter strips any unknown future `_PROTO_*` defensively.

### 6.11 Diagnostic constants

| Constant | Value | Cite |
|---|---|---|
| `MAX_DIAGNOSTICS_SUMMARY_CHARS` | `4000` | `diagnosticTracking.ts:12` |
| Truncation marker | `'…[truncated]'` | `:353` |
| Severity symbols | `cross/warning/info/star/bullet` (figures package) | `:386-393` |
| URI prefixes recognised | `file://`, `_claude_fs_right:`, `_claude_fs_left:` | `:80-84` |

### 6.12 Internal-logging constants (ant-only)

| Path | Value | Cite |
|---|---|---|
| Namespace path | `/var/run/secrets/kubernetes.io/serviceaccount/namespace` | `internalLogging.ts:22` |
| Container ID source | `/proc/self/mountinfo` | `:39` |
| Container ID regex | `/(?:\/docker\/containers\/|\/sandboxes\/)([0-9a-f]{64})/` | `:51` |
| Sentinels | `'namespace not found'`, `'container ID not found'`, `'container ID not found in mountinfo'` | `:23,40,41` |
| Event emitted | `tengu_internal_record_permission_context` | `:79` |

## 7. Side Effects & I/O

- **Filesystem**: `~/.claude.json` reads/writes (GrowthBook caches and overrides); `<config>/telemetry/1p_failed_events.<sessionId>.<BATCH_UUID>.json` JSONL appends + reads + deletes; `<config>/traces/trace-<sessionId>.json` (Perfetto when env set). On ant: reads of `/var/run/secrets/kubernetes.io/serviceaccount/namespace` and `/proc/self/mountinfo`.
- **Network**: HTTPS POSTs to `https://api.anthropic.com/api/event_logging/batch` (or staging), Datadog `https://http-intake.logs.us5.datadoghq.com/api/v2/logs`, GrowthBook `https://api.anthropic.com/` (or `CLAUDE_CODE_GB_BASE_URL`), OTLP endpoint via `OTEL_EXPORTER_OTLP_ENDPOINT`. Custom auth headers via `getAuthHeaders()`; mTLS/proxy through `getMTLSConfig`/`HttpsProxyAgent`/`shouldBypassProxy`.
- **Process spawn**: only `services/notifier.ts` uses `osascript` and `defaults` (Apple_Terminal auto-channel; `:117-130`); analytics core has no spawns.
- **Signal handling**: `process.on('beforeExit', destroy)` and `process.on('exit', destroy)` registered for the GrowthBook client and OTel providers; periodic refresh registers `process.once('beforeExit', stopPeriodicGrowthBookRefresh)` exactly once (`growthbook.ts:1107-1109`).
- **Trust gate**: All auth-bearing operations check `checkHasTrustDialogAccepted() || getSessionTrustAccepted() || getIsNonInteractiveSession()` before invoking `getAuthHeaders()` (which may run `apiKeyHelper`).

Env vars consumed (master list):
```
USER_TYPE, NODE_ENV
CLAUDE_INTERNAL_FC_OVERRIDES (ant-only)            CLAUDE_CODE_GB_BASE_URL (ant-only)
ENABLE_GROWTHBOOK_DEV (ant-only)                   ANTHROPIC_BASE_URL
CLAUDE_CODE_USE_BEDROCK / _USE_VERTEX / _USE_FOUNDRY    CLAUDE_CODE_ENABLE_TELEMETRY
ANT_OTEL_{METRICS,LOGS,TRACES}_EXPORTER (ant-only)
ANT_OTEL_EXPORTER_OTLP_{PROTOCOL,ENDPOINT,HEADERS} (ant-only)
OTEL_{METRICS,LOGS,TRACES}_EXPORTER                OTEL_EXPORTER_OTLP_PROTOCOL
OTEL_EXPORTER_OTLP_{METRICS,LOGS,TRACES}_PROTOCOL  OTEL_EXPORTER_OTLP_ENDPOINT
OTEL_EXPORTER_OTLP_HEADERS                         OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE
OTEL_METRIC_EXPORT_INTERVAL                        OTEL_LOGS_EXPORT_INTERVAL
OTEL_TRACES_EXPORT_INTERVAL                        OTEL_LOG_USER_PROMPTS
OTEL_LOG_TOOL_DETAILS
CLAUDE_CODE_OTEL_SHUTDOWN_TIMEOUT_MS               CLAUDE_CODE_OTEL_FLUSH_TIMEOUT_MS
BETA_TRACING_ENDPOINT                              ENABLE_BETA_TRACING_DETAILED
CLAUDE_CODE_DATADOG_FLUSH_INTERVAL_MS
CLAUDE_CODE_PERFETTO_TRACE                         CLAUDE_CODE_PERFETTO_WRITE_INTERVAL_S
CLAUDE_CODE_ENTRYPOINT                             CLAUDE_AGENT_SDK_VERSION
CLAUDE_CODE_HOST_PLATFORM                          CLAUDE_CODE_REMOTE
CLAUDE_CODE_REMOTE_ENVIRONMENT_TYPE                CLAUDE_CODE_COWORKER_TYPE
CLAUDE_CODE_CONTAINER_ID                           CLAUDE_CODE_REMOTE_SESSION_ID
CLAUDE_CODE_TAGS                                   CLAUDE_CODE_WORKSPACE_HOST_PATHS
CLAUDE_CODE_AGENT_ID                               CLAUDE_CODE_PARENT_SESSION_ID
CLAUDE_CODE_ACTION                                 CI, CLAUBBIT, GITHUB_ACTIONS,
GITHUB_EVENT_NAME, RUNNER_ENVIRONMENT, RUNNER_OS, GITHUB_ACTION_PATH
SWE_BENCH_RUN_ID, SWE_BENCH_INSTANCE_ID, SWE_BENCH_TASK_ID
```

## 8. Feature Flags & Variants

> **Canonical flag matrix lives in `00-overview.md` §8.1 + §8.1.B** (89 flags with owning-spec column). The table below is **scoped to telemetry/analytics flags only** — flags whose `feature(...)` branches sit inside this spec's IN-scope files. For any other flag, consult spec 00.

### 8.1 Telemetry/analytics build-time flags (this spec's narrow scope)

Build-time (`bun:bundle.feature(...)`) flags evaluated at build, branch elided in non-matching builds.

| Flag | Behavior on / off | Cite |
|---|---|---|
| `PERFETTO_TRACING` | Entire `initializePerfettoTracing` body removed | `perfettoTracing.ts:260` |
| `ENHANCED_TELEMETRY_BETA` | Tracing init in `instrumentation.ts:628` skipped if false; `isEnhancedTelemetryEnabled` then short-circuits | `sessionTracing.ts:127` |
| `SHOT_STATS` | All shot-distribution map population/read paths elided | `stats.ts:131,214,364,610,829` |
| `SLOW_OPERATION_LOGGING` | `slowLogging` template tag bound to `slowLoggingAnt` (else `slowLoggingExternal` no-op) | `slowOperations.ts:157` |
| `COWORKER_TYPE_TELEMETRY` | `coworkerType` field added to `EnvContext` & 1P `env.coworker_type` | `metadata.ts:603,846` |
| `CHICAGO_MCP` | `BUILTIN_MCP_SERVER_NAMES` set populated (else empty Set) | `metadata.ts:130` |
| `KAIROS` | `kairosActive: true` tag added to `EventMetadata` when `getKairosActive()` true | `metadata.ts:735` |

### 8.2 Runtime (GrowthBook)

Resolution priority: env-overrides → config-overrides → in-memory remoteEval → disk cache → default. Refresh: 6 h (non-ANT) / 20 min (ANT). All `tengu_*` and `claude_code_*` configs flow through this path.

### 8.3 ANT vs production

- ANT users: `apiHost = CLAUDE_CODE_GB_BASE_URL || 'https://api.anthropic.com/'`, env-override and config-override paths active, debug `logForDebugging` calls active throughout, `ANT_OTEL_*` vars remapped, /config Gates tab can edit overrides, refresh interval 20 min.
- Non-ANT: env/config overrides ignored, no debug logs, no remap, refresh interval 6 h.

### 8.4 Other runtime gates

`CLAUDE_CODE_USE_BEDROCK` / `_USE_VERTEX` / `_USE_FOUNDRY` truthy ⇒ `isAnalyticsDisabled()` true ⇒ all sinks off. `NODE_ENV==='test'` ⇒ analytics disabled. `getAPIProvider() !== 'firstParty'` ⇒ Datadog off (datadog.ts:168-171). `NODE_ENV !== 'production'` ⇒ Datadog off (`:164-166`). `isTelemetryDisabled()` (privacy level) ⇒ everything off.

## 9. Error Handling & Edge Cases

- **GrowthBook init timeout (5s)**: caught (`logError` ant-only) and the `.then` chain skipped — `remoteEvalFeatureValues` stays empty; all `_CACHED_MAY_BE_STALE` readers fall to disk cache.
- **Empty payload `{features:{}}`**: `processRemoteEvalPayload` returns false without clearing maps or writing disk — prevents flag blackout from a transient server bug.
- **Replaced-client guards**: at the start of init `.then` and after every `await setPayload()` / `refreshFeatures()` boundary; if `client !== thisClient` the callback bails.
- **Auth-not-yet-available**: `getGrowthBookClient` returns `{client, initialized: Promise.resolve()}` (skip HTTP init), `clientCreatedWithAuth=false`. `initializeGrowthBook` later detects and forces a `resetGrowthBook()` + recreate.
- **`refreshGrowthBookAfterAuthChange`**: emits `refreshed` post-reset so subscribers re-read disk cache even if subsequent re-init fails or short-circuits on logout.
- **1P-exporter killswitch**: `isKilled()` throws inside `sendBatchWithRetry` → caller short-circuits remaining batches and writes them to disk; backoff retries continue probing.
- **401 from 1P endpoint**: retry the same payload once without auth (only applies when initial attempt was authed).
- **`maxAttempts=8`**: drop events with logged error `Dropped N events: max attempts (8) reached`; delete failed-events file.
- **Reinit of 1P logger fails**: rollback to `oldProvider`/`oldLogger` so a future GrowthBook refresh can retry; without rollback both stay null and recovery is impossible.
- **Datadog flush failure**: caught with `logError`; events lost (no disk persistence on this sink).
- **OTel shutdown timeout (default 2s)**: race against `Promise.all(provider.shutdown())`; on timeout error message is the constant string `'OpenTelemetry shutdown timeout'` and a multi-line guidance message is emitted via `logForDebugging({level:'error'})` listing remediation env vars.
- **Sample-rate validation**: non-numeric, NaN, or `< 0` / `> 1` returns `null` (no sampling) — i.e. fail-open keeps the event.
- **Diagnostic mismatch**: when expected vs returned URI differ (after normalization) the service logs a `DiagnosticsTrackingError` and skips the file. IDE not supporting `getDiagnostics` → silent fail (catch and return).
- **Notifier**: any switch-case throw → catch returns `'error'`; analytic event still logged with that string.

## 10. Telemetry & Observability

- **Log emit point**: every call to `logEvent(name, …)` reaches Datadog (subject to allow-list and sampling) and 1P exporter (subject to sampling and killswitch). The **two analytics consumers** of this spec's pipeline besides the wider system: `notifier.ts:29` emits `tengu_notification_method_used`; `internalLogging.ts:79` emits `tengu_internal_record_permission_context` (ant-only).
- **OTel spans**: see §6.5.
- **OTel metrics**: meter `'com.anthropic.claude_code'` (`MACRO.VERSION`); BigQuery exporter at 5 min interval; OTLP exporter at metrics interval (`OTEL_METRIC_EXPORT_INTERVAL || 60000`). Specific instrument names live in `bigqueryExporter.ts` (grep-confirmed; not enumerated here).
- **OTel logs**: scope `'com.anthropic.claude_code.events'` for both 1P and (when telemetry enabled) customer telemetry; the 1P provider is **NOT** registered globally to keep customer endpoints isolated.
- **Diag logger**: `ClaudeCodeDiagLogger` at `DiagLogLevel.ERROR` routes OTel internal errors/warnings through `logForDebugging` (`logger.ts:1-26`).
- **Beta tracing**: parallel span/log pipeline behind `BETA_TRACING_ENDPOINT`; `/v1/traces` and `/v1/logs` paths.

## 11. Reimplementation Checklist

Spec is complete when a reimplementer can recreate the following invariants:

- `attachAnalyticsSink` is idempotent and drains `eventQueue` on a microtask.
- `logEvent` order: sample → killswitch+gate → Datadog with `_PROTO_*` stripped → 1P with full payload.
- GrowthBook resolution priority preserved (env > config > in-memory > disk > default), env/config overrides ant-only, `setForcedFeatures`/server `evalFeature` bypassed.
- `processRemoteEvalPayload` empty-features guard preserved.
- `refreshed` signal **survives** `resetGrowthBook`; `loggedExposures`/`pendingExposures` **do not**.
- 1P endpoint default `'https://api.anthropic.com/api/event_logging/batch'`, staging override only when `ANTHROPIC_BASE_URL` exactly equals `'https://api-staging.anthropic.com'`.
- Failed-events file path stable across reinit (`<config>/telemetry/1p_failed_events.<sessionId>.<BATCH_UUID>.json`).
- 401 fallback: with-auth → without-auth retry exactly once.
- Quadratic backoff `min(base * attempts², maxBackoffDelayMs)`; reset on success; cancel on shutdown.
- OTLP exporters lazy-imported per protocol (gRPC ~700 KB elided unless `protocol==='grpc'`).
- Span names match §6.5 exactly.
- `MACRO.VERSION` and `MACRO.BUILD_TIME` substituted at build time (build constants pulled into resource).
- `claude_code_global_system_caching` ordering invariant declared (delegated enforcement to spec 03/08): reorder requires StatSig config update.
- `_PROTO_*` strip is **fail-closed** for general-access (Datadog) and the BQ `additional_metadata` blob.
- ANT-only debug paths hidden behind `process.env.USER_TYPE === 'ant'`.
- Cleanup-registry registration of `shutdownTelemetry` is mandatory (process exits otherwise leak unflushed metrics).
- Datadog allow-list (44 events; corrected Phase 9.6 from prior "41" miscount — the 4-entry `tengu_team_mem_*` cluster was added to source without count refresh) and `TAG_FIELDS` list (16 fields) preserved verbatim.
- Periodic-refresh interval (`6h non-ANT / 20 min ANT`) and OAuth-expiry skipAuth path preserved.

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited.

1. ~~**Generated proto bindings under `src/types/generated/events_mono/`**~~ — **RESOLVED Phase 9.7**: spec 42 §A (Phase 10b) confirms `src/types/generated/events_mono/{claude_code,common,growthbook}/v1/` exists in the leaked tree as bun-protobuf-gen output (directory-level enumeration). BigQuery field names in §6.7 are therefore source-grounded, not inferred. Cross-ref: spec 00 §12 Q4 (RESOLVED with same finding).
2. ~~**Full enumeration of 817 `tengu_*` configs**~~ — **DEFERRED (intentional)**: enumeration impractical inline; the named subset in §6.8/§6.9 remains authoritative. The full set is rebuildable on demand via `grep -ohrE "'tengu_[a-z_0-9]+'" src/`. Not a defect.
3. ~~**`bigqueryExporter.ts`, `betaSessionTracing.ts`, `pluginTelemetry.ts`, `skillLoadedEvent.ts` grep-only coverage**~~ — **NOTE Phase 9.7**: file-level enumeration sufficient for spec corpus; concrete metric names are consumer-side details. Future revise pass can deepen if needed; not blocking.
4. ~~**`claude_code_global_system_caching` / `claude_code_system_cache_policy` JSON shapes**~~ — **DEFERRED**: server-side StatSig artifacts. Recorded in spec 00 §13 row 1 as known-unfalsifiable. URL preserved for reimplementer reference.
5. ~~**`growthbook.ts:1006-1009` `loggedExposures` clearing**~~ — **RESOLVED Phase 9.7**: cross-state interaction with `:104-107` `refreshed` signal documented as consistent (different state); §4.1 clarification suffices.
6. ~~**`internalLogging.ts:90` `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` cast**~~ — **NOTE Phase 9.7**: the marker type is documenting-only (`= never`), no runtime enforcement. Permission-context payloads with file paths are an ANT-only-acceptable risk. Spec 09 (permissions) §11 notes this cross-cutting concern.

— end —
