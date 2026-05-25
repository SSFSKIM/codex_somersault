# 22 — Service: Anthropic API Layer Specification

> Authoritative anchor for the streaming/retry/SSE/auth-header behavior consumed by `QueryEngine` (spec 03), the turn pipeline (spec 04), and cost tracking (spec 06). All behavioral claims are cited as `src/<path>:<line-range>`.

## 1. Purpose & Scope

This subsystem owns every direct call to the Anthropic SDK. It builds Anthropic clients (first-party / Bedrock / Vertex / Foundry), assembles the streaming `messages.create` request (system blocks, tool schemas, beta headers, prompt-cache breakpoints, thinking/effort/task-budget params, fast-mode/AFK/cache-editing latches), drives the SSE event loop (`message_start` → `content_block_*` → `message_delta` → `message_stop`), implements retry with exponential backoff + 529 cascade fallback, falls back streaming → non-streaming on errors, surfaces normalized error messages to the user, accumulates usage (token totals) for cost (spec 06), uploads/downloads via the Files API, fetches `/api/claude_cli/bootstrap`, and provides the ANT-only VCR cassette/replay layer.

**IN scope:** `src/services/api/{claude.ts, withRetry.ts, errors.ts, errorUtils.ts, client.ts, emptyUsage.ts, logging.ts, promptCacheBreakDetection.ts, bootstrap.ts, filesApi.ts, dumpPrompts.ts, sessionIngress.ts, adminRequests.ts, grove.ts, firstTokenDate.ts, metricsOptOut.ts, overageCreditGrant.ts, referral.ts, ultrareviewQuota.ts, usage.ts}`; `src/services/{vcr.ts, claudeAiLimits.ts, claudeAiLimitsHook.ts, mockRateLimits.ts, rateLimitMessages.ts, rateLimitMocking.ts}`; the beta-header set in `src/constants/betas.ts`.

**OUT of scope (cite spec):**
- QueryEngine consumer logic (turn loop, top-level abort) → spec 03
- Turn pipeline / message normalization shells → spec 04
- Cost calculation and per-model pricing (`modelCost.ts`) → spec 06
- OAuth flow itself, keychain, scope handling → spec 25 (we cite its consumer endpoints)
- Analytics/GrowthBook flag resolution → spec 26
- Remote/CCR session control plane → spec 35

## 2. Source Map

### Owned files (read inventory)

| File | Lines | Inventory |
|---|---|---|
| `src/services/api/claude.ts` | 3419 | fully-read across SSE handler 1979-2304; thinking 1596-1630; updateUsage 2924-2987; accumulateUsage 2993-3038; addCacheBreakpoints 3063-3211; non-streaming fallback 2495-2806; ANTI_DISTILLATION_CC 302-313; CONNECTOR_TEXT, FAST_MODE, AFK_MODE latches 1412-1689 |
| `src/services/api/withRetry.ts` | 822 | fully-read (retry algorithm, backoff, persistent retry, fast-mode cooldown, error classifiers) |
| `src/services/api/errors.ts` | 1207 | fully-read (lines 1-1207: full error taxonomy, all user-facing strings, classifyAPIError, categorizeRetryableAPIError, getAssistantMessageFromError) |
| `src/services/api/errorUtils.ts` | 261 | fully-read (SSL codes, formatAPIError, extractConnectionErrorDetails) |
| `src/services/api/client.ts` | 390 | fully-read (provider construction, header set, custom headers, x-client-request-id) |
| `src/services/api/emptyUsage.ts` | 23 | fully-read (`EMPTY_USAGE`, `NonNullableUsage` re-export) |
| `src/services/api/logging.ts` | 789 | fully-read (gateway detection, logAPIQuery/Error/Success, NonNullableUsage re-export, GlobalCacheStrategy) |
| `src/services/api/promptCacheBreakDetection.ts` | 728 | fully-read (record/check, hash, diff, TTL constants `CACHE_TTL_5MIN_MS`, `CACHE_TTL_1HOUR_MS`) |
| `src/services/api/bootstrap.ts` | 142 | fully-read (`/api/claude_cli/bootstrap` GET, schema, persistence) |
| `src/services/api/filesApi.ts` | 749 | fully-read (download/upload/list, multipart, retry-with-backoff, BASE_DELAY_MS=500, MAX_RETRIES=3, FILES_API_BETA_HEADER) |
| `src/services/vcr.ts` | 406 | sampled (gating: `NODE_ENV==='test'` OR `USER_TYPE==='ant' && FORCE_VCR`; cassette paths under `getClaudeConfigHomeDir()`) |
| `src/services/claudeAiLimits.ts` | sampled | header parser, `getRateLimitErrorMessage`, `currentLimits`, `extractQuotaStatusFromError/Headers`, `OverageDisabledReason` |
| `src/services/claudeAiLimitsHook.ts` | sampled | UI hook surface (consumer-facing) |
| `src/services/mockRateLimits.ts` | sampled | ANT-only mock injection for `/mock-limits` |
| `src/services/rateLimitMessages.ts` | sampled | message bodies (verbatim assets owned by spec 27 — text-fragment caller is here) |
| `src/services/rateLimitMocking.ts` | sampled | `processRateLimitHeaders`, `shouldProcessRateLimits`, `isMockRateLimitError` |
| `src/constants/betas.ts` | 52 | fully-read (verbatim asset, see §6) |
| `src/services/api/{adminRequests, dumpPrompts, firstTokenDate, grove, metricsOptOut, overageCreditGrant, referral, sessionIngress, ultrareviewQuota, usage}.ts` | residual | grep-inspected; minor consumer surfaces, no streaming/retry behavior |

### Imports from
- `@anthropic-ai/sdk` (+ `bedrock-sdk`, `vertex-sdk`, `foundry-sdk`, `@azure/identity`, `google-auth-library` lazily)
- `src/utils/auth.js` (spec 25): `getAnthropicApiKey`, `getClaudeAIOAuthTokens`, `checkAndRefreshOAuthTokenIfNeeded`, `handleOAuth401Error`, `clear{ApiKey,Aws,Gcp}…Cache`, `isClaudeAISubscriber`, `isEnterpriseSubscriber`, `getOauthAccountInfo`, `hasProfileScope`, `getApiKeyFromApiKeyHelper`, `refreshAndGetAwsCredentials`, `refreshGcpCredentialsIfNeeded`
- `src/utils/model/{providers,model,bedrock,modelStrings}.js`: provider routing, AWS/Vertex region, normalize model strings
- `src/utils/messages.js`: `normalizeMessagesForAPI`, `ensureToolResultPairing`, `stripAdvisorBlocks`, `stripCallerFieldFromAssistantMessage`, `stripToolReferenceBlocksFromUserMessage`, `normalizeContentFromAPI`, `createAssistantAPIErrorMessage`, `createUserMessage`
- `src/utils/{betas,context,thinking,effort,fastMode,advisor,toolSearch,modelCost,fingerprint,proxy,sleep}.js`
- `src/services/{analytics,compact,lsp/manager,mcp/utils}.js`; `src/bootstrap/state.js` (latch storage); `src/types/{message,connectorText}.js`
- `src/constants/{betas,system,oauth,querySource,apiLimits}.js`
- `src/cost-tracker.js` (`addToTotalSessionCost`)

### Imported by
- `src/QueryEngine.ts` (spec 03) — calls `queryModelWithStreaming`, `queryModelWithoutStreaming`, `queryHaiku`, `queryWithModel`, consumes `EMPTY_USAGE`, `NonNullableUsage`, `accumulateUsage`, `MAX_NON_STREAMING_TOKENS`, `getMaxOutputTokensForModel`, `verifyApiKey`
- `src/query.ts` (spec 04) — uses `is529Error`, `isPromptTooLongMessage`, `getPromptTooLongTokenGap`, `isMediaSizeErrorMessage`, `categorizeRetryableAPIError`, `FallbackTriggeredError`
- `src/services/compact/*` (spec 07) — calls `queryHaiku`, consumes prompt-too-long/media error classifiers
- `src/cost-tracker.ts` (spec 06) — accumulates from `usage` produced here
- `src/bridge/replBridge.ts` — imports `EMPTY_USAGE` directly from `emptyUsage.ts` to avoid the `errors.ts` transitive graph (`src/services/api/emptyUsage.ts:1-22`)
- IDE bridge (spec 34), remote (spec 35), SDK (spec 01) entrypoints

### Feature-flag and ANT guards (locations)

| Symbol | Cite | Gate |
|---|---|---|
| `feature('ANTI_DISTILLATION_CC')` (sends `anti_distillation: ['fake_tools']` for 1P CLI) | `src/services/api/claude.ts:303-310` | flag + `CLAUDE_CODE_ENTRYPOINT==='cli'` + `shouldIncludeFirstPartyOnlyBetas()` + GrowthBook `tengu_anti_distill_fake_tool_injection` |
| `feature('CONNECTOR_TEXT')` (`SUMMARIZE_CONNECTOR_TEXT_BETA_HEADER`, connector_text deltas) | `src/constants/betas.ts:23-25`, `src/services/api/claude.ts:661-662, 2067-2081` | flag |
| `feature('UNATTENDED_RETRY')` (persistent retry mode) | `src/services/api/withRetry.ts:100-104` | flag + `process.env.CLAUDE_CODE_UNATTENDED_RETRY` truthy |
| `feature('NATIVE_CLIENT_ATTESTATION')` (adds `cch=00000;` placeholder in attribution) | `src/constants/system.ts:64,82` | flag |
| `feature('TRANSCRIPT_CLASSIFIER')` (`AFK_MODE_BETA_HEADER`, autoModeState) | `src/constants/betas.ts:26-28`, `src/services/api/claude.ts:105-107, 1413-1423, 1661-1670` | flag |
| `feature('CACHED_MICROCOMPACT')` (cache-editing beta, `cache_edits` blocks, `cache_deleted_input_tokens`) | `src/services/api/claude.ts:1190-1205, 1432-1442, 1672-1689, 2970-2982, 3022-3033, 3108-3163` | flag |
| `feature('PROMPT_CACHE_BREAK_DETECTION')` | `src/services/api/claude.ts:1460-1486, 2383-2392` | flag |
| `feature('BASH_CLASSIFIER')` (adds `bash_classifier` to FOREGROUND_529_RETRY_SOURCES) | `src/services/api/withRetry.ts:81` | flag |
| `feature('TRANSCRIPT_CLASSIFIER')` autoModeState lazy require | `src/services/api/claude.ts:105-107` | flag |
| `process.env.USER_TYPE === 'ant'` checks | `withRetry.ts:202-210, 354-356, 686-688, 746-751`; `claude.ts:457-465, 1987-1992, 2166-2168, 2204-2206, 2584-2587, 2681-2683`; `errors.ts:687-705, 753-770`; `client.ts:307-310`; `vcr.ts:28`; `logging.ts:746-755`; `betas.ts:29-30` (`CLI_INTERNAL_BETA_HEADER`) | ANT-only |
| `feature('TRANSCRIPT_CLASSIFIER')` + `'ant'` (staging OAuth baseURL override `USE_STAGING_OAUTH`) | `client.ts:307-310` | combined |
| `process.env.CLAUDE_CODE_REMOTE` (CCR) auth bypass on 401/403, 120s fallback timeout | `withRetry.ts:712-717`; `claude.ts:807-811`; `errors.ts:217-219, 818-822, 870-874` | env |
| `process.env.FALLBACK_FOR_ALL_PRIMARY_MODELS` | `withRetry.ts:330-333` | env |
| `process.env.CLAUDE_CODE_DISABLE_THINKING`, `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING` | `claude.ts:1597-1607` | env |
| `process.env.CLAUDE_CODE_MAX_OUTPUT_TOKENS` (via `validateBoundedIntEnvVar`) | `claude.ts:3399-3418` | env |
| `process.env.CLAUDE_CODE_MAX_RETRIES` | `withRetry.ts:789-793` | env |
| `process.env.CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK`; GrowthBook `tengu_disable_streaming_to_non_streaming_fallback` | `claude.ts:2469-2474` | env + flag |
| `process.env.CLAUDE_CODE_USE_BEDROCK / _USE_VERTEX / _USE_FOUNDRY` | `client.ts:153, 191, 221`; `errors.ts:632, 671, 888, 1137`; `withRetry.ts:632, 671` | env (provider switch) |
| `process.env.{ANTHROPIC_API_KEY, ANTHROPIC_AUTH_TOKEN, ANTHROPIC_CUSTOM_HEADERS, ANTHROPIC_BASE_URL, ANTHROPIC_MODEL, ANTHROPIC_SMALL_FAST_MODEL, ANTHROPIC_VERTEX_PROJECT_ID, ANTHROPIC_FOUNDRY_API_KEY, ANTHROPIC_FOUNDRY_RESOURCE, ANTHROPIC_FOUNDRY_BASE_URL, AWS_BEARER_TOKEN_BEDROCK, ANTHROPIC_SMALL_FAST_MODEL_AWS_REGION, AWS_REGION/AWS_DEFAULT_REGION, GCLOUD_PROJECT/GOOGLE_CLOUD_PROJECT, GOOGLE_APPLICATION_CREDENTIALS, CLOUD_ML_REGION, VERTEX_REGION_*}` | `client.ts:1-389` | env |
| `process.env.{API_TIMEOUT_MS, CLAUDE_ENABLE_STREAM_WATCHDOG, CLAUDE_STREAM_IDLE_TIMEOUT_MS, CLAUDE_CODE_REMOTE, CLAUDE_CODE_CONTAINER_ID, CLAUDE_CODE_REMOTE_SESSION_ID, CLAUDE_AGENT_SDK_CLIENT_APP, CLAUDE_CODE_ADDITIONAL_PROTECTION, CLAUDE_CODE_API_BASE_URL, CLAUDE_CODE_EXTRA_BODY, CLAUDE_CODE_EXTRA_METADATA, ENABLE_PROMPT_CACHING_1H_BEDROCK, DISABLE_PROMPT_CACHING, DISABLE_PROMPT_CACHING_HAIKU/_SONNET/_OPUS, CLAUDE_CODE_SKIP_BEDROCK_AUTH, CLAUDE_CODE_SKIP_VERTEX_AUTH, CLAUDE_CODE_SKIP_FOUNDRY_AUTH, IS_SANDBOX, FORCE_VCR, NODE_EXTRA_CA_CERTS}` | `client.ts`, `claude.ts`, `vcr.ts`, `errorUtils.ts:99` | env |

### Source-coverage status
fully-read: `claude.ts`, `withRetry.ts`, `errors.ts`, `errorUtils.ts`, `client.ts`, `emptyUsage.ts`, `logging.ts`, `promptCacheBreakDetection.ts`, `bootstrap.ts`, `filesApi.ts`, `constants/betas.ts`. sampled: `vcr.ts`, `claudeAiLimits.ts`, `mockRateLimits.ts`, `rateLimitMessages.ts`, `rateLimitMocking.ts`. grep-inspected: `adminRequests.ts`, `dumpPrompts.ts`, `firstTokenDate.ts`, `grove.ts`, `metricsOptOut.ts`, `overageCreditGrant.ts`, `referral.ts`, `sessionIngress.ts`, `ultrareviewQuota.ts`, `usage.ts`, `claudeAiLimitsHook.ts`. missing: none.

### Citation precision
Line-range citations (`file.ts:N-M`) in this spec were re-verified in Phase 9.6 against source at HEAD. Sampled 10 cited ranges (`claude.ts:676-707`, `1979-2297`, `2382-2392`, `2607-2666`, `2924-2987`, `2993-3038`, `3354`, `3399`, `withRetry.ts:799`, `:801`); 10/10 match exactly. Treat citations as precise to ±0 lines for spans listed in this section, and ±5 lines for any range not in that list. The `Cite span: claude.ts:1017-2892` annotation in §5.1 is a *function-bracketing* range (start of `queryModel` to end of its `finally`) and is approximate at the bounds.

## 3. Public Interface

Exported from `src/services/api/claude.ts`:

```ts
queryModelWithStreaming({ messages, systemPrompt, thinkingConfig, tools, signal, options }):
  AsyncGenerator<StreamEvent | AssistantMessage | SystemAPIErrorMessage, void>
queryModelWithoutStreaming({ messages, systemPrompt, thinkingConfig, tools, signal, options }):
  Promise<AssistantMessage>
queryHaiku({ systemPrompt, userPrompt, outputFormat, signal, options }): Promise<AssistantMessage>
queryWithModel({ systemPrompt, userPrompt, outputFormat, signal, options }): Promise<AssistantMessage>
verifyApiKey(apiKey, isNonInteractiveSession): Promise<boolean>
executeNonStreamingRequest(...): AsyncGenerator<SystemAPIErrorMessage, BetaMessage>
adjustParamsForNonStreaming<T>(params, maxTokensCap): T
addCacheBreakpoints(messages, enablePromptCaching, querySource?, useCachedMC?, newCacheEdits?, pinnedEdits?, skipCacheWrite?): MessageParam[]
buildSystemPromptBlocks(systemPrompt, enablePromptCaching, options?): TextBlockParam[]
userMessageToMessageParam(message, addCache, enablePromptCaching, querySource?): MessageParam
assistantMessageToMessageParam(message, addCache, enablePromptCaching, querySource?): MessageParam
updateUsage(usage, partUsage): NonNullableUsage
accumulateUsage(totalUsage, messageUsage): NonNullableUsage
cleanupStream(stream): void
stripExcessMediaItems(messages, limit): (UserMessage | AssistantMessage)[]
getMaxOutputTokensForModel(model): number
getCacheControl({ scope?, querySource? }): { type:'ephemeral'; ttl?:'1h'; scope?:CacheScope }
getPromptCachingEnabled(model): boolean
getExtraBodyParams(betaHeaders?): JsonObject
getAPIMetadata(): { user_id: string }
configureTaskBudgetParams(taskBudget, outputConfig, betas): void
type Options = { …see §4 }
const MAX_NON_STREAMING_TOKENS = 64_000
```

From `src/services/api/withRetry.ts`:

```ts
withRetry<T>(getClient, operation, options): AsyncGenerator<SystemAPIErrorMessage, T>
class CannotRetryError extends Error { originalError; retryContext }
class FallbackTriggeredError extends Error { originalModel; fallbackModel }
type RetryContext = { maxTokensOverride?; model; thinkingConfig; fastMode? }
type RetryOptions = { maxRetries?; model; fallbackModel?; thinkingConfig; fastMode?; signal?; querySource?; initialConsecutive529Errors? }
is529Error(error): boolean
getRetryDelay(attempt, retryAfterHeader?, maxDelayMs=32000): number
parseMaxTokensContextOverflowError(error): { inputTokens; maxTokens; contextLimit } | undefined
getDefaultMaxRetries(): number
const BASE_DELAY_MS = 500
```

From `src/services/api/errors.ts`:

```ts
getAssistantMessageFromError(error, model, options?): AssistantMessage
classifyAPIError(error): string                  // → 'aborted' | 'api_timeout' | 'repeated_529' | 'capacity_off_switch'
                                                 //   | 'rate_limit' | 'server_overload' | 'prompt_too_long' | 'pdf_too_large'
                                                 //   | 'pdf_password_protected' | 'image_too_large' | 'tool_use_mismatch'
                                                 //   | 'unexpected_tool_result' | 'duplicate_tool_use_id' | 'invalid_model'
                                                 //   | 'credit_balance_low' | 'invalid_api_key' | 'token_revoked'
                                                 //   | 'oauth_org_not_allowed' | 'auth_error' | 'bedrock_model_access'
                                                 //   | 'server_error' | 'client_error' | 'ssl_cert_error' | 'connection_error'
                                                 //   | 'unknown'
categorizeRetryableAPIError(error: APIError): SDKAssistantMessageError
                                                 // → 'rate_limit' | 'authentication_failed' | 'server_error' | 'unknown'
getErrorMessageIfRefusal(stopReason, model): AssistantMessage | undefined
isPromptTooLongMessage(msg): boolean
isMediaSizeErrorMessage(msg): boolean
parsePromptTooLongTokenCounts(rawMessage): { actualTokens?: number; limitTokens?: number }
getPromptTooLongTokenGap(msg): number | undefined
isMediaSizeError(raw): boolean
isValidAPIMessage(value): boolean
extractUnknownErrorFormat(value): string | undefined
startsWithApiErrorPrefix(text): boolean
```

From `src/services/api/client.ts`:
```ts
getAnthropicClient({ apiKey?, maxRetries, model?, fetchOverride?, source? }): Promise<Anthropic>
const CLIENT_REQUEST_ID_HEADER = 'x-client-request-id'
```

From `src/services/api/emptyUsage.ts` / `logging.ts`:
```ts
const EMPTY_USAGE: Readonly<NonNullableUsage>      // see §6.3 verbatim
type NonNullableUsage                              // re-exported from sdkUtilityTypes
type GlobalCacheStrategy = 'tool_based' | 'system_prompt' | 'none'
logAPIQuery(...); logAPIError(...); logAPISuccessAndDuration(...)
```

From `src/services/api/promptCacheBreakDetection.ts`:
```ts
recordPromptState(snapshot: PromptStateSnapshot): void
checkResponseForCacheBreak(querySource, cacheReadTokens, cacheCreationTokens, messages, agentId?, requestId?): Promise<void>
notifyCacheDeletion(querySource, agentId?): void
notifyCompaction(querySource, agentId?): void
cleanupAgentTracking(agentId): void
resetPromptCacheBreakDetection(): void
const CACHE_TTL_1HOUR_MS = 60 * 60 * 1000
```

From `src/services/api/bootstrap.ts`:
```ts
fetchBootstrapData(): Promise<void>                // GET /api/claude_cli/bootstrap, persists into globalConfig.{clientDataCache, additionalModelOptionsCache}
```

From `src/services/api/filesApi.ts`:
```ts
type File = { fileId: string; relativePath: string }
type FilesApiConfig = { oauthToken: string; baseUrl?: string; sessionId: string }
downloadFile(fileId, config): Promise<Buffer>
downloadAndSaveFile(attachment, config): Promise<DownloadResult>
downloadSessionFiles(files, config, concurrency=5): Promise<DownloadResult[]>
uploadFile(filePath, relativePath, config, opts?): Promise<UploadResult>
uploadSessionFiles(files, config, concurrency=5): Promise<UploadResult[]>
listFilesCreatedAfter(afterCreatedAt, config): Promise<FileMetadata[]>
parseFileSpecs(fileSpecs: string[]): File[]
buildDownloadPath(basePath, sessionId, relativePath): string | null
```

`Options` shape (`claude.ts:676-707`): `{ getToolPermissionContext, model, toolChoice?, isNonInteractiveSession, extraToolSchemas?, maxOutputTokensOverride?, fallbackModel?, onStreamingFallback?, querySource, agents, allowedAgentTypes?, hasAppendSystemPrompt, fetchOverride?, enablePromptCaching?, skipCacheWrite?, temperatureOverride?, effortValue?, mcpTools, hasPendingMcpServers?, queryTracking?, agentId?, outputFormat?, fastMode?, advisorModel?, addNotification?, taskBudget?:{total,remaining?} }`.

## 4. Data Model & State

### Core types

- `NonNullableUsage` (re-exported via `src/entrypoints/sdk/sdkUtilityTypes.js`) — see `EMPTY_USAGE` in §6.3 for the shape.
- `RetryContext` (`withRetry.ts:120-125`): `{ maxTokensOverride?; model; thinkingConfig; fastMode? }`. Mutated across attempts: `fastMode` flips off after 429/529 cooldown; `maxTokensOverride` set after 400 prompt-too-long context overflow.
- `PromptStateSnapshot` (`promptCacheBreakDetection.ts:227-241`): `{ system; toolSchemas; querySource; model; agentId?; fastMode?; globalCacheStrategy?; betas?; autoModeActive?; isUsingOverage?; cachedMCEnabled?; effortValue?; extraBodyParams? }`.
- `BetaMessageStreamParams` is owned by the SDK; the build call in `paramsFromContext` (`claude.ts:1538-1729`) returns: `{ model: normalizeModelStringForAPI(...), messages, system, tools, tool_choice?, betas?, metadata, max_tokens, thinking?, temperature?, context_management?, output_config?, speed?, ...extraBodyParams }`.

### Session-stable latches (in `bootstrap/state.js` — owned there; consumer here)

`afkModeHeaderLatched`, `cacheEditingHeaderLatched`, `fastModeHeaderLatched`, `thinkingClearLatched`, `promptCache1hAllowlist`, `promptCache1hEligible`, `lastApiCompletionTimestamp`, `lastMainRequestId`. Latch policy: once first sent for the session, **the beta header keeps being sent** so mid-session toggles (overage flip, cooldown, AFK toggle) do not change the server-side cache key (`claude.ts:1405-1456, 1655-1689`). Latches are cleared on `/clear`, `/compact`.

### Per-source cache-break tracking
`Map<key, PreviousState>` keyed by `agentId || querySource` (with `querySource==='compact'` aliased to `'repl_main_thread'`). Capped at `MAX_TRACKED_SOURCES = 10`; eldest evicted (`promptCacheBreakDetection.ts:101-115, 296-326`).

### Usage state machine
Anthropic streaming sends **cumulative** input-side totals on `message_start` and **may send 0** on `message_delta` for those fields. `updateUsage` (see §6.5) preserves prior input-side values when the new value is null/zero; `output_tokens` always uses the latest non-undefined value (`claude.ts:2916-2987`).

### Concurrency
- Generator-based; one `for await` loop per attempt; `releaseStreamResources()` mandatory in `finally` to free native TLS/socket buffers (`claude.ts:1519-1526, 2807-2891`).
- `cleanupStream` aborts via `stream.controller.abort()` if not already aborted (`claude.ts:2898-2912`).
- Stream idle watchdog: `STREAM_IDLE_TIMEOUT_MS = parseInt(env.CLAUDE_STREAM_IDLE_TIMEOUT_MS) || 90_000`, warning at `STREAM_IDLE_TIMEOUT_MS / 2`, only enabled if `CLAUDE_ENABLE_STREAM_WATCHDOG` truthy (`claude.ts:1874-1928`).
- Stall (gap-between-events) threshold: `STALL_THRESHOLD_MS = 30_000` (logs only) (`claude.ts:1936`).

## 5. Algorithm / Control Flow

### 5.1 Top-level streaming pipeline (`queryModel`)

```
queryModel(messages, systemPrompt, thinkingConfig, tools, signal, options):
  if !subscriber and isNonCustomOpusModel(model)
       and getDynamicConfig_BLOCKS_ON_INIT('tengu-off-switch').activated:
     yield error(CUSTOM_OFF_SWITCH_MESSAGE); return                     # claude.ts:1031-1049
  previousRequestId  = scan messages backwards for last assistant.requestId
  resolvedModel      = bedrock+inference-profile? backing model : model  # 1057-1062
  isAgenticQuery     = querySource startsWith repl_main_thread/agent: || ==='sdk'|hook_agent|verification_agent
  betas              = getMergedBetas(model, {isAgenticQuery})           # utils/betas.ts
  if isAdvisorEnabled(): betas.push(ADVISOR_BETA_HEADER)
  resolve advisorModel (experiment may override)
  useToolSearch      = await isToolSearchEnabled(model, tools, perm, agents, 'query')
  deferredToolNames  = { isDeferredTool(t) for t in tools }   if useToolSearch
  if useToolSearch and deferredToolNames empty and !hasPendingMcpServers: useToolSearch=false
  filteredTools      = filter ToolSearchTool out unless useToolSearch; otherwise filter deferred not in extractDiscoveredToolNames(messages)
  if useToolSearch and provider!=='bedrock': betas.push(getToolSearchBetaHeader())
  if feature(CACHED_MICROCOMPACT): cachedMCEnabled = isCachedMicrocompactEnabled() && isModelSupportedForCacheEditing(model); cacheEditingBetaHeader=CACHE_EDITING_BETA_HEADER
  useGlobalCacheFeature = shouldUseGlobalCacheScope()
  needsToolBasedCacheMarker = useGlobalCacheFeature && filteredTools has live MCP (not deferred)
  if useGlobalCacheFeature: betas push PROMPT_CACHING_SCOPE_BETA_HEADER (idempotent)
  globalCacheStrategy = useGlobalCacheFeature ? (needsToolBasedCacheMarker ? 'none' : 'system_prompt') : 'none'
  toolSchemas = await Promise.all(filteredTools map toolToAPISchema(...defer_loading=willDefer(tool)))
  messagesForAPI = normalizeMessagesForAPI(messages, filteredTools)
  if !useToolSearch: strip tool_reference (user) and 'caller' (assistant) from messagesForAPI
  messagesForAPI = ensureToolResultPairing(messagesForAPI)
  if !betas.includes(ADVISOR_BETA_HEADER): stripAdvisorBlocks
  messagesForAPI = stripExcessMediaItems(messagesForAPI, API_MAX_MEDIA_PER_REQUEST)
  fingerprint = computeFingerprintFromMessages(messagesForAPI)
  if useToolSearch and !isDeferredToolsDeltaEnabled():
     messagesForAPI prepend synthetic <available-deferred-tools>...</available-deferred-tools> user message (isMeta:true)
  systemPrompt = [getAttributionHeader(fingerprint), getCLISyspromptPrefix({...}), ...systemPrompt,
                  ADVISOR_TOOL_INSTRUCTIONS?, CHROME_TOOL_SEARCH_INSTRUCTIONS?].filter(Boolean)
  enablePromptCaching = options.enablePromptCaching ?? getPromptCachingEnabled(model)
  system  = buildSystemPromptBlocks(systemPrompt, enablePromptCaching, {skipGlobalCacheForSystemPrompt: needsToolBasedCacheMarker, querySource})
  isFastMode = fastMode supported AND available AND !cooldown AND options.fastMode
  Latch flip-once: afkHeaderLatched/fastModeHeaderLatched/cacheEditingHeaderLatched/thinkingClearLatched (if conditions hold)
  effort = resolveAppliedEffort(model, options.effortValue)
  if feature(PROMPT_CACHE_BREAK_DETECTION): recordPromptState({system, toolSchemas without defer_loading, ...})
  llmSpan = startLLMRequestSpan(model, newContext?, messagesForAPI, isFastMode)
  consumedCacheEdits = cachedMCEnabled ? consumePendingCacheEdits() : null
  consumedPinnedEdits = cachedMCEnabled ? getPinnedCacheEdits() : []
  paramsFromContext(retryContext) -> BetaMessageStreamParams (see §5.4)
  log scalars synchronously then fire-and-forget logAPIQuery(...)
  generator = withRetry(
        () => getAnthropicClient({ maxRetries:0, model, fetchOverride, source:querySource }),
        async (anthropic, attempt, context) => {
           attemptNumber = attempt; isFastModeRequest = context.fastMode
           start = Date.now(); attemptStartTimes.push(start)
           params = paramsFromContext(context); maxOutputTokens = params.max_tokens
           clientRequestId = (provider==='firstParty' && isFirstPartyAnthropicBaseUrl()) ? randomUUID() : undefined
           result = await anthropic.beta.messages
                      .create({...params, stream:true}, {signal, headers:{x-client-request-id:clientRequestId}})
                      .withResponse()
           # NOTE: only two error branches at this site are handled with bespoke logic:
           #   (a) APIError(status===404)  → wrapped CannotRetryError, caught by outer
           #        `is404StreamCreationError` branch (claude.ts:2612-2618), triggers
           #        non-streaming fallback.
           #   (b) APIUserAbortError       → handled in the streamingError catch (claude.ts:2434-2462).
           # Any *other* throw during stream creation (e.g. SDK TypeError on malformed
           # response body before headers complete, transport-level Errors that aren't
           # APIError, AggregateError) is **not** specially-cased: it propagates up to
           # the outermost `catch errorFromRetry` at claude.ts:2738+, where
           # `getAssistantMessageFromError` is yielded and `classifyAPIError` maps it to
           # `'unknown'`. Reimplementer MUST NOT add an early throw at this site.
           streamRequestId = result.request_id; streamResponse = result.response
           return result.data
        },
        { model, fallbackModel, thinkingConfig, fastMode, signal, querySource })

  drain generator: yield SystemAPIErrorMessage values; final value = stream

  reset state; resetStreamIdleTimer(); startSessionActivity('api_call')
  for await part in stream:
     resetStreamIdleTimer()
     detect stall (>30s gap)
     dispatch on part.type (see §5.2)
     yield {type:'stream_event', event:part, ttftMs?}
  clearStreamIdleTimers()
  if streamIdleAborted: throw 'Stream idle timeout - no chunks received'
  if !partialMessage || (newMessages.length==0 && !stopReason): throw 'Stream ended without receiving any events'
  if feature(PROMPT_CACHE_BREAK_DETECTION): void checkResponseForCacheBreak(...)
  if streamResponse: extractQuotaStatusFromHeaders(streamResponse.headers); responseHeaders=streamResponse.headers
catch streamingError:
  clearStreamIdleTimers()
  if streamingError instanceof APIUserAbortError:
     if signal.aborted: throw streamingError                            # genuine user abort
     else: throw new APIConnectionTimeoutError({message:'Request timed out'})
  if disableFallback (env CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK || GB tengu_disable_streaming_to_non_streaming_fallback):
     throw streamingError
  didFallBackToNonStreaming = true; options.onStreamingFallback?.()
  result = yield* executeNonStreamingRequest(..., initialConsecutive529Errors: is529Error(streamingError) ? 1 : 0)
  push AssistantMessage; fallbackMessage=m; yield m
catch errorFromRetry (outermost):
  if FallbackTriggeredError: rethrow                                    # query.ts handles model switch
  if 404 stream-creation CannotRetryError: ... fall back to non-streaming as above; re-catch FallbackTriggeredError
  else: extractQuotaStatusFromError; logAPIError; if APIUserAbortError return; yield getAssistantMessageFromError; return
finally:
  stopSessionActivity('api_call'); releaseStreamResources()
  if fallbackMessage: usage = updateUsage(EMPTY_USAGE, fallbackMessage.message.usage); stopReason = fallbackMessage.message.stop_reason; cost track
if cachedMCEnabled: markToolsSentToAPIState()
if streamRequestId && !getAgentContext() && (querySource startsWith repl_main_thread || ==='sdk'): setLastMainRequestId(streamRequestId)
fire-and-forget logAPISuccessAndDuration(...)
```

Cite span: `claude.ts:1017-2892`.

### 5.2 SSE event handler (verbatim event-name set)

The stream event-name set used by `for await (const part of stream)` switches on `part.type ∈ { "message_start", "content_block_start", "content_block_delta", "content_block_stop", "message_delta", "message_stop" }` (`claude.ts:1979-2297`). Errors flowing as a `'error'` SSE event are mapped to thrown `APIError` by the SDK before reaching this loop.

| Event | Action |
|---|---|
| `message_start` | `partialMessage = part.message`; `ttftMs = Date.now()-start`; `usage = updateUsage(usage, part.message?.usage)`; if `USER_TYPE==='ant'` capture `research`. |
| `content_block_start` | switch `part.content_block.type`: `tool_use` → `{...content_block, input:''}`; `server_tool_use` → `{...content_block, input:'' as object}`; if name==='advisor' set `isAdvisorInProgress=true`, log `tengu_advisor_tool_call`. `text` → init `{...content_block, text:''}`. `thinking` → init `{...content_block, thinking:'', signature:''}`. default: `{...content_block}`; `advisor_tool_result` resets `isAdvisorInProgress=false`. |
| `content_block_delta` | dispatch on `delta.type`: `connector_text_delta` (only `feature('CONNECTOR_TEXT')`) appends to `connector_text`; `citations_delta` (TODO); `input_json_delta` requires `tool_use`/`server_tool_use` and string-typed input; appends `partial_json` (no partial JSON parsing — saves O(n²)). `text_delta` requires `text` block; appends `delta.text`. `signature_delta` writes `signature` on `connector_text` (CONNECTOR_TEXT) or `thinking`. `thinking_delta` requires `thinking`; appends. Mismatch types log `tengu_streaming_error` and throw. ANT: capture `research` if present. |
| `content_block_stop` | normalize the single block via `normalizeContentFromAPI([contentBlock], tools, agentId)`; build `AssistantMessage{message:{...partialMessage, content:[normalized]}, requestId:streamRequestId, type:'assistant', uuid:randomUUID(), timestamp:new Date().toISOString(), research?, advisorModel?}`; push to `newMessages`; `yield m`. |
| `message_delta` | `usage = updateUsage(usage, part.usage)`; ANT capture `research`, write back to all `newMessages`. **Direct-mutate** (`lastMsg.message.usage = usage; lastMsg.message.stop_reason = stopReason`) NOT object replacement (transcript queue holds reference). `costUSDForPart = calculateUSDCost(resolvedModel, usage); costUSD += addToTotalSessionCost(...)`. If `getErrorMessageIfRefusal` non-null, yield it. If `stopReason==='max_tokens'`: log `tengu_max_tokens_reached`; yield `{content: 'API Error: Claude\'s response exceeded the {maxOutputTokens} output token maximum. To configure this behavior, set the CLAUDE_CODE_MAX_OUTPUT_TOKENS environment variable.', apiError:'max_output_tokens', error:'max_output_tokens'}`. If `stopReason==='model_context_window_exceeded'`: log; yield `{content: 'API Error: The model has reached its context window limit.', apiError:'max_output_tokens', error:'max_output_tokens'}`. |
| `message_stop` | no-op. |

After every event, `yield {type:'stream_event', event:part, ttftMs?}` (only `message_start` includes `ttftMs`).

### 5.3 Streaming → non-streaming fallback

Triggered by any `streamingError` not blocked by env/GrowthBook kill-switch (`claude.ts:2469-2569`) or by 404 thrown during `.withResponse()` (`2607-2666`). Calls `executeNonStreamingRequest`, which wraps `withRetry(getAnthropicClient(maxRetries:0), async (anthropic, attempt, context) => anthropic.beta.messages.create(adjustParamsForNonStreaming(params, MAX_NON_STREAMING_TOKENS), {signal, timeout: getNonstreamingFallbackTimeoutMs()}))`. Per-attempt timeout: `parseInt(env.API_TIMEOUT_MS)` if set, else `120_000` if `CLAUDE_CODE_REMOTE`, else `300_000` (`claude.ts:807-811`). `MAX_NON_STREAMING_TOKENS = 64_000` (`claude.ts:3354`). On streaming-529, the fallback receives `initialConsecutive529Errors:1` so the consecutive-529 budget is consistent across modes (`claude.ts:2559`).

### 5.4 paramsFromContext (request body assembly) — pseudocode

```
paramsFromContext(retryContext):
  betasParams = [...betas]
  if !contains(CONTEXT_1M_BETA_HEADER) and getSonnet1mExpTreatmentEnabled(retryContext.model): push CONTEXT_1M
  bedrockBetas = (provider==='bedrock')
                 ? [...getBedrockExtraBodyParamsBetas(retryContext.model), ...(toolSearchHeader ? [toolSearchHeader] : [])]
                 : []
  extraBodyParams = getExtraBodyParams(bedrockBetas)        # CLAUDE_CODE_EXTRA_BODY parsed JSON object + anti_distillation + anthropic_beta merge
  outputConfig = { ...(extraBodyParams.output_config ?? {}) }
  configureEffortParams(effort, outputConfig, extraBodyParams, betasParams, model)
  configureTaskBudgetParams(options.taskBudget, outputConfig, betasParams)
  if options.outputFormat and !('format' in outputConfig):
      outputConfig.format = options.outputFormat
      if modelSupportsStructuredOutputs(model): push STRUCTURED_OUTPUTS_BETA_HEADER (idempotent)
  maxOutputTokens = retryContext.maxTokensOverride ?? options.maxOutputTokensOverride ?? getMaxOutputTokensForModel(model)
  hasThinking = thinkingConfig.type !== 'disabled' && !env.CLAUDE_CODE_DISABLE_THINKING
  if hasThinking and modelSupportsThinking(model):
       if !env.CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING and modelSupportsAdaptiveThinking(model):
           thinking = { type:'adaptive' }
       else:
           thinkingBudget = (thinkingConfig.type==='enabled' && thinkingConfig.budgetTokens) ?? getMaxThinkingTokensForModel(model)
           thinkingBudget = min(maxOutputTokens-1, thinkingBudget)
           thinking = { budget_tokens: thinkingBudget, type:'enabled' }
  contextManagement = getAPIContextManagement({ hasThinking, isRedactThinkingActive: betas.includes(REDACT_THINKING_BETA_HEADER), clearAllThinking: thinkingClearLatched })
  speed = (isFastModeForRetry) ? 'fast' : undefined
  if fastModeHeaderLatched and !contains(FAST_MODE_BETA_HEADER): push FAST_MODE
  if afkHeaderLatched and shouldIncludeFirstPartyOnlyBetas() and isAgenticQuery and !contains(AFK_MODE_BETA_HEADER): push AFK_MODE
  useCachedMC = cachedMCEnabled and provider==='firstParty' and querySource==='repl_main_thread'
  if cacheEditingHeaderLatched and provider==='firstParty' and querySource==='repl_main_thread' and !contains(cacheEditingBetaHeader): push cacheEditingBetaHeader
  temperature = !hasThinking ? (options.temperatureOverride ?? 1) : undefined        # API requires temperature:1 when thinking on (we omit)
  return {
    model: normalizeModelStringForAPI(options.model),
    messages: addCacheBreakpoints(messagesForAPI, enablePromptCaching, querySource, useCachedMC, consumedCacheEdits, consumedPinnedEdits, options.skipCacheWrite),
    system, tools: allTools, tool_choice: options.toolChoice,
    ...(useBetas && { betas: betasParams }),
    metadata: getAPIMetadata(),
    max_tokens: maxOutputTokens,
    thinking,
    ...(temperature !== undefined && { temperature }),
    ...(contextManagement && useBetas && betasParams.includes(CONTEXT_MANAGEMENT_BETA_HEADER) && { context_management: contextManagement }),
    ...extraBodyParams,
    ...(Object.keys(outputConfig).length > 0 && { output_config: outputConfig }),
    ...(speed !== undefined && { speed }),
  }
```

Cite span: `claude.ts:1538-1729`.

### 5.5 Prompt-cache breakpoint placement (`addCacheBreakpoints`) — pseudocode

```
addCacheBreakpoints(messages, enablePromptCaching, querySource?, useCachedMC=false, newCacheEdits?, pinnedEdits?, skipCacheWrite=false):
  logEvent('tengu_api_cache_breakpoints', { totalMessageCount, cachingEnabled, skipCacheWrite })

  # Exactly one message-level cache_control marker per request.
  # Default: last message. Fork (skipCacheWrite=true): second-to-last.
  markerIndex = skipCacheWrite ? messages.length - 2 : messages.length - 1
  result = messages.map((msg, i) =>
      msg.type === 'user'
        ? userMessageToMessageParam(msg, i===markerIndex, enablePromptCaching, querySource)
        : assistantMessageToMessageParam(msg, i===markerIndex, enablePromptCaching, querySource))
  if !useCachedMC: return result

  seenDeleteRefs = Set()
  dedup(block) = { ...block, edits: block.edits.filter(e => add seenDeleteRefs e.cache_reference) }
  for pinned in (pinnedEdits ?? []):
     msg = result[pinned.userMessageIndex]
     if msg.role === 'user': insertBlockAfterToolResults(msg.content, dedup(pinned.block))
  if newCacheEdits and result.length>0:
     deduped = dedup(newCacheEdits)
     for i = result.length-1 downTo 0:
        if result[i].role === 'user':
           insertBlockAfterToolResults(result[i].content, deduped); pinCacheEdits(i, newCacheEdits); log; break
  if enablePromptCaching:
     # find last index that has a cache_control marker
     lastCCMsg = max i s.t. result[i].content has block with cache_control
     # add cache_reference = tool_use_id to every tool_result block strictly before lastCCMsg
     for i in [0, lastCCMsg):
        if result[i].role !== 'user' or content not array: continue
        clone-on-first-write; for each tool_result block: msg.content[j] = {...block, cache_reference: block.tool_use_id}
  return result
```

`userMessageToMessageParam` / `assistantMessageToMessageParam` apply cache_control to the **last block** of the message; for assistants the marker is **NOT** placed on `thinking` / `redacted_thinking` / (CONNECTOR_TEXT) `connector_text` blocks (`claude.ts:588-674`).

`buildSystemPromptBlocks` (`claude.ts:3213-3237`) uses `splitSysPromptPrefix` to compute per-block `cacheScope`. When `enablePromptCaching && block.cacheScope !== null`, attaches `cache_control: getCacheControl({scope, querySource})`. **Comment is normative**: "IMPORTANT: Do not add any more blocks for caching or you will get a 400" — the system block layout and number of cache markers is fixed.

`getCacheControl({scope?, querySource?})` (`claude.ts:358-374`): always `{type:'ephemeral'}`; if `should1hCacheTTL(querySource)` then `ttl:'1h'`; if `scope==='global'` then `scope:'global'`. `should1hCacheTTL` gates on (a) Bedrock + `ENABLE_PROMPT_CACHING_1H_BEDROCK`, OR (b) `(USER_TYPE==='ant') OR (subscriber AND !overage)` with `querySource` matched against the GrowthBook `tengu_prompt_cache_1h_config.allowlist` (supports trailing `*`); both eligibility and allowlist are **session-latched** to avoid cache busts (`claude.ts:393-434`).

### 5.6 `withRetry` — retry algorithm (pseudocode)

```
withRetry(getClient, operation, options):
  maxRetries = options.maxRetries ?? getDefaultMaxRetries()
  retryContext = { model, thinkingConfig, fastMode? }
  client=null; consecutive529Errors=options.initialConsecutive529Errors??0; lastError; persistentAttempt=0

  for attempt = 1..maxRetries+1:
     if signal.aborted: throw APIUserAbortError
     wasFastModeActive = isFastModeEnabled() ? (retryContext.fastMode && !isFastModeCooldown()) : false
     try:
        if USER_TYPE==='ant': mockError = checkMockRateLimitError(model, wasFastModeActive); if mockError throw
        isStaleConnection = isStaleConnectionError(lastError)                    # ECONNRESET/EPIPE
        if isStaleConnection and GB tengu_disable_keepalive_on_econnreset: disableKeepAlive()
        if client==null OR (lastError.status===401) OR isOAuthTokenRevoked(lastError) OR isBedrockAuthError(lastError) OR isVertexAuthError(lastError) OR isStaleConnection:
            if 401-token-expired or 403-revoked: handleOAuth401Error(currentAccessToken)
            client = await getClient()
        return await operation(client, attempt, retryContext)
     catch error:
        lastError = error
        # FAST-MODE PATH (suppressed in persistent retry mode)
        if wasFastModeActive and !isPersistentRetryEnabled() and APIError(429|529):
           if header anthropic-ratelimit-unified-overage-disabled-reason: handleFastModeOverageRejection; retryContext.fastMode=false; continue
           retryAfterMs = getRetryAfterMs(error)
           if retryAfterMs!==null and retryAfterMs<SHORT_RETRY_THRESHOLD_MS: sleep(retryAfterMs); continue
           cooldownMs = max(retryAfterMs ?? DEFAULT_FAST_MODE_FALLBACK_HOLD_MS, MIN_COOLDOWN_MS)
           triggerFastModeCooldown(Date.now()+cooldownMs, is529Error(error)?'overloaded':'rate_limit')
           if isFastModeEnabled(): retryContext.fastMode=false
           continue
        if wasFastModeActive and isFastModeNotEnabledError(error):
           handleFastModeRejectedByAPI; retryContext.fastMode=false; continue

        # 529 GATE — non-foreground sources never retry (drop)
        if is529Error(error) and !shouldRetry529(querySource):
           logEvent('tengu_api_529_background_dropped'); throw new CannotRetryError(error, retryContext)

        # 529 BUDGET — Opus or all-models override
        if is529Error(error) and (FALLBACK_FOR_ALL_PRIMARY_MODELS or (!subscriber and isNonCustomOpusModel(model))):
           consecutive529Errors++
           if consecutive529Errors >= MAX_529_RETRIES (3):
              if options.fallbackModel:
                 log; throw new FallbackTriggeredError(originalModel, fallbackModel)
              if USER_TYPE==='external' and !IS_SANDBOX and !persistent:
                 log; throw new CannotRetryError(new Error(REPEATED_529_ERROR_MESSAGE), retryContext)

        persistent = isPersistentRetryEnabled() and isTransientCapacityError(error)
        if attempt > maxRetries and !persistent: throw new CannotRetryError(error, retryContext)
        handledCloudAuthError = handleAwsCredentialError(error) || handleGcpCredentialError(error)
        if !handledCloudAuthError and (!APIError or !shouldRetry(error)): throw new CannotRetryError(error, retryContext)

        # 400 CONTEXT-OVERFLOW (legacy — extended-context-window beta now returns model_context_window_exceeded stop_reason)
        if APIError and parseMaxTokensContextOverflowError(error) → {inputTokens, contextLimit}:
           safetyBuffer = 1000
           availableContext = max(0, contextLimit - inputTokens - safetyBuffer)
           if availableContext < FLOOR_OUTPUT_TOKENS (3000): throw error
           minRequired = (thinkingConfig.type==='enabled' ? thinkingConfig.budgetTokens : 0) + 1
           retryContext.maxTokensOverride = max(FLOOR_OUTPUT_TOKENS, availableContext, minRequired)
           logEvent('tengu_max_tokens_context_overflow_adjustment'); continue

        retryAfter = getRetryAfter(error)              # honored if present (no jitter, bypasses maxDelayMs in getRetryDelay)
        if persistent and APIError.status===429:
           persistentAttempt++
           resetDelay = getRateLimitResetDelayMs(error)              # anthropic-ratelimit-unified-reset → unix-sec*1000-Date.now(), capped at PERSISTENT_RESET_CAP_MS
           delayMs = resetDelay ?? min(getRetryDelay(persistentAttempt, retryAfter, PERSISTENT_MAX_BACKOFF_MS), PERSISTENT_RESET_CAP_MS)
        elif persistent:
           persistentAttempt++
           delayMs = min(getRetryDelay(persistentAttempt, retryAfter, PERSISTENT_MAX_BACKOFF_MS), PERSISTENT_RESET_CAP_MS)
        else:
           delayMs = getRetryDelay(attempt, retryAfter)              # default maxDelayMs=32_000
        logEvent('tengu_api_retry', { attempt: persistent?persistentAttempt:attempt, delayMs, error.message, status, provider })
        if persistent:
           if delayMs > 60_000: logEvent('tengu_api_persistent_retry_wait')
           remaining = delayMs
           while remaining > 0:
              if signal.aborted: throw APIUserAbortError
              if APIError: yield createSystemAPIErrorMessage(error, remaining, persistentAttempt, maxRetries)
              chunk = min(remaining, HEARTBEAT_INTERVAL_MS)         # 30_000
              await sleep(chunk, signal, {abortError})
              remaining -= chunk
           if attempt >= maxRetries: attempt = maxRetries           # clamp so for-loop never terminates
        else:
           if APIError: yield createSystemAPIErrorMessage(error, delayMs, attempt, maxRetries)
           await sleep(delayMs, signal, {abortError})
  throw new CannotRetryError(lastError, retryContext)
```

`shouldRetry(error)` decision tree (`withRetry.ts:696-787`):
1. mock-rate-limit error (`isMockRateLimitError`) → `false`.
2. persistent + transient capacity (429/529) → `true` (bypasses subscriber gates and `x-should-retry`).
3. CCR (`CLAUDE_CODE_REMOTE`) and status ∈ {401,403} → `true`.
4. message includes `"type":"overloaded_error"` → `true`.
5. `parseMaxTokensContextOverflowError(error)` truthy → `true`.
6. `x-should-retry: 'true'` → if non-subscriber OR enterprise, `true`.
7. `x-should-retry: 'false'` → respect (`false`) UNLESS ANT and 5xx.
8. `APIConnectionError` → `true`.
9. status 408 → `true`; status 409 → `true`.
10. status 429 → `!subscriber || enterprise`.
11. status 401 → `clearApiKeyHelperCache()`; `true`.
12. `isOAuthTokenRevokedError` → `true`.
13. status >= 500 → `true`. Else `false`.

`getRetryDelay(attempt, retryAfterHeader?, maxDelayMs=32000)` (`withRetry.ts:530-548`):
```
if retryAfterHeader: secs = parseInt(...,10); if !isNaN: return secs*1000     # honored as-is, NO jitter, NO maxDelayMs cap
baseDelay = min(BASE_DELAY_MS * 2^(attempt-1), maxDelayMs)
jitter    = Math.random() * 0.25 * baseDelay
return baseDelay + jitter
```

### 5.7 Bootstrap

`fetchBootstrapData` (`bootstrap.ts:42-141`): only on first-party, when `!isEssentialTrafficOnly()`, with usable OAuth (`hasProfileScope()`) **or** API key. GETs `${BASE_API_URL}/api/claude_cli/bootstrap` with `User-Agent: getClaudeCodeUserAgent()`, timeout 5000ms; uses `withOAuth401Retry` so refresh-and-retry handles a stale token. Validates response with `bootstrapResponseSchema` (`{client_data, additional_model_options:[{model,name,description}→{value:model,label:name,description}]}`). Persists into `globalConfig.{clientDataCache, additionalModelOptionsCache}` only if `lodash.isEqual` finds diffs.

### 5.8 Files API

Headers: `Authorization: Bearer ${oauthToken}; anthropic-version: 2023-06-01; anthropic-beta: 'files-api-2025-04-14,oauth-2025-04-20'` (`filesApi.ts:27`). Endpoints: `GET /v1/files/${fileId}/content`; `POST /v1/files` (multipart with boundary `----FormBoundary${randomUUID()}`, parts: `file` + `purpose=user_data`); `GET /v1/files?after_created_at=...&after_id=...`. Constants: `MAX_RETRIES=3`, `BASE_DELAY_MS=500`, `MAX_FILE_SIZE_BYTES=500*1024*1024`, download timeout 60_000ms, upload timeout 120_000ms, list timeout 60_000ms. Backoff: `BASE_DELAY_MS * 2^(attempt-1)` (no jitter). Path traversal: rejects normalized path that startsWith `..`; strips redundant `${basePath}/${sessionId}/uploads/` and `/uploads/` prefixes; final path: `${basePath}/${sessionId}/uploads/${cleanPath}`. Concurrency cap default 5.

### 5.9 Prompt-cache break detection (two-phase)

Phase 1 (`recordPromptState`, `claude.ts:1471-1486`, `promptCacheBreakDetection.ts:247-430`): tracks per-key state, computes hashes (`Bun.hash` if available else `djb2Hash`) over `system` (cache-control stripped), `tools` (cache-control stripped), per-tool, plus `cacheControlHash` over `system.map(b => b.cache_control ?? null)`, plus tracking of `model`, `fastMode`, `globalCacheStrategy`, sorted `betas`, `autoModeActive`, `isUsingOverage`, `cachedMCEnabled`, `effortValue`, `extraBodyHash`. Differences populate `pendingChanges`. Phase 2 (`checkResponseForCacheBreak`, `claude.ts:2382-2392`) fires after stream end with the actual `cache_read_input_tokens` / `cache_creation_input_tokens`; flags a break only if drop > `MIN_CACHE_MISS_TOKENS=2_000` AND `cacheReadTokens < prevCacheRead*0.95`. Tracked sources: `repl_main_thread*`, `sdk`, `agent:custom`, `agent:default`, `agent:builtin`; `compact` aliases to `repl_main_thread`. Excludes `haiku` models.

## 6. Verbatim Assets

### 6.1 Beta header constants (verbatim, `src/constants/betas.ts`)

```ts
import { feature } from 'bun:bundle'

export const CLAUDE_CODE_20250219_BETA_HEADER = 'claude-code-20250219'
export const INTERLEAVED_THINKING_BETA_HEADER =
  'interleaved-thinking-2025-05-14'
export const CONTEXT_1M_BETA_HEADER = 'context-1m-2025-08-07'
export const CONTEXT_MANAGEMENT_BETA_HEADER = 'context-management-2025-06-27'
export const STRUCTURED_OUTPUTS_BETA_HEADER = 'structured-outputs-2025-12-15'
export const WEB_SEARCH_BETA_HEADER = 'web-search-2025-03-05'
// Tool search beta headers differ by provider:
// - Claude API / Foundry: advanced-tool-use-2025-11-20
// - Vertex AI / Bedrock: tool-search-tool-2025-10-19
export const TOOL_SEARCH_BETA_HEADER_1P = 'advanced-tool-use-2025-11-20'
export const TOOL_SEARCH_BETA_HEADER_3P = 'tool-search-tool-2025-10-19'
export const EFFORT_BETA_HEADER = 'effort-2025-11-24'
export const TASK_BUDGETS_BETA_HEADER = 'task-budgets-2026-03-13'
export const PROMPT_CACHING_SCOPE_BETA_HEADER =
  'prompt-caching-scope-2026-01-05'
export const FAST_MODE_BETA_HEADER = 'fast-mode-2026-02-01'
export const REDACT_THINKING_BETA_HEADER = 'redact-thinking-2026-02-12'
export const TOKEN_EFFICIENT_TOOLS_BETA_HEADER =
  'token-efficient-tools-2026-03-28'
export const SUMMARIZE_CONNECTOR_TEXT_BETA_HEADER = feature('CONNECTOR_TEXT')
  ? 'summarize-connector-text-2026-03-13'
  : ''
export const AFK_MODE_BETA_HEADER = feature('TRANSCRIPT_CLASSIFIER')
  ? 'afk-mode-2026-01-31'
  : ''
export const CLI_INTERNAL_BETA_HEADER =
  process.env.USER_TYPE === 'ant' ? 'cli-internal-2026-02-09' : ''
export const ADVISOR_BETA_HEADER = 'advisor-tool-2026-03-01'

export const BEDROCK_EXTRA_PARAMS_HEADERS = new Set([
  INTERLEAVED_THINKING_BETA_HEADER,
  CONTEXT_1M_BETA_HEADER,
  TOOL_SEARCH_BETA_HEADER_3P,
])

export const VERTEX_COUNT_TOKENS_ALLOWED_BETAS = new Set([
  CLAUDE_CODE_20250219_BETA_HEADER,
  INTERLEAVED_THINKING_BETA_HEADER,
  CONTEXT_MANAGEMENT_BETA_HEADER,
])
```

Other beta headers used by this layer but defined elsewhere: `CACHE_EDITING_BETA_HEADER` (loaded dynamically when `feature('CACHED_MICROCOMPACT')` is on, `claude.ts:1190-1205`); Files API beta: `'files-api-2025-04-14,oauth-2025-04-20'` (`filesApi.ts:27`); OAuth bootstrap beta: `OAUTH_BETA_HEADER` from `src/constants/oauth.js` (`bootstrap.ts:75`).

### 6.2 Retry constants (verbatim, `withRetry.ts`)

| Constant | Value | Cite |
|---|---|---|
| `DEFAULT_MAX_RETRIES` | `10` | `:52` |
| `FLOOR_OUTPUT_TOKENS` | `3000` | `:53` |
| `MAX_529_RETRIES` | `3` | `:54` |
| `BASE_DELAY_MS` (exported) | `500` | `:55` |
| `PERSISTENT_MAX_BACKOFF_MS` | `5 * 60 * 1000` (300_000) | `:96` |
| `PERSISTENT_RESET_CAP_MS` | `6 * 60 * 60 * 1000` (21_600_000) | `:97` |
| `HEARTBEAT_INTERVAL_MS` | `30_000` | `:98` |
| `DEFAULT_FAST_MODE_FALLBACK_HOLD_MS` | `30 * 60 * 1000` | `:799` |
| `SHORT_RETRY_THRESHOLD_MS` | `20 * 1000` | `:800` |
| `MIN_COOLDOWN_MS` | `10 * 60 * 1000` | `:801` |
| `getRetryDelay` jitter | `Math.random() * 0.25 * baseDelay` | `:546` |
| `getRetryDelay` default `maxDelayMs` | `32000` | `:533` |

`FOREGROUND_529_RETRY_SOURCES` (`withRetry.ts:62-82`):

```ts
new Set<QuerySource>([
  'repl_main_thread',
  'repl_main_thread:outputStyle:custom',
  'repl_main_thread:outputStyle:Explanatory',
  'repl_main_thread:outputStyle:Learning',
  'sdk',
  'agent:custom',
  'agent:default',
  'agent:builtin',
  'compact',
  'hook_agent',
  'hook_prompt',
  'verification_agent',
  'side_question',
  'auto_mode',
  ...(feature('BASH_CLASSIFIER') ? (['bash_classifier'] as const) : []),
])
```

`shouldRetry529(querySource)`: `querySource === undefined || FOREGROUND_529_RETRY_SOURCES.has(querySource)` (untagged → retry).

### 6.3 `EMPTY_USAGE` (verbatim, `emptyUsage.ts:8-22`)

```ts
export const EMPTY_USAGE: Readonly<NonNullableUsage> = {
  input_tokens: 0,
  cache_creation_input_tokens: 0,
  cache_read_input_tokens: 0,
  output_tokens: 0,
  server_tool_use: { web_search_requests: 0, web_fetch_requests: 0 },
  service_tier: 'standard',
  cache_creation: {
    ephemeral_1h_input_tokens: 0,
    ephemeral_5m_input_tokens: 0,
  },
  inference_geo: '',
  iterations: [],
  speed: 'standard',
}
```

### 6.4 Retry-classification regexes / conditions

- `parseMaxTokensContextOverflowError` (`withRetry.ts:550-595`): status===400 AND message includes ``input length and `max_tokens` exceed context limit``; regex ``/input length and `max_tokens` exceed context limit: (\d+) \+ (\d+) > (\d+)/``. Returns `{inputTokens, maxTokens, contextLimit}`.
- `is529Error` (`withRetry.ts:610-621`): `APIError && (status===529 || message.includes('"type":"overloaded_error"'))`.
- `isFastModeNotEnabledError` (`withRetry.ts:600-608`): `APIError && status===400 && message.includes('Fast mode is not enabled')`.
- `isOAuthTokenRevokedError` (`withRetry.ts:623-629`): `APIError && status===403 && message.includes('OAuth token has been revoked')`.
- `isStaleConnectionError` (`withRetry.ts:112-118`): `APIConnectionError` AND `extractConnectionErrorDetails(error).code ∈ {ECONNRESET, EPIPE}`.
- `isBedrockAuthError` (`withRetry.ts:631-644`): `CLAUDE_CODE_USE_BEDROCK` AND (`isAwsCredentialsProviderError(error)` OR `(APIError && status===403)`).
- `isGoogleAuthLibraryCredentialError` (`withRetry.ts:660-668`): `Error` whose message contains any of `'Could not load the default credentials'`, `'Could not refresh access token'`, `'invalid_grant'`.
- `isVertexAuthError` (`withRetry.ts:670-682`): `CLAUDE_CODE_USE_VERTEX` AND (Google-auth-lib creds error OR `(APIError && status===401)`).
- `isMediaSizeError(raw)` (`errors.ts:133-139`): `(raw.includes('image exceeds') && raw.includes('maximum')) || (raw.includes('image dimensions exceed') && raw.includes('many-image')) || /maximum of \d+ PDF pages/.test(raw)`.
- `parsePromptTooLongTokenCounts` (`errors.ts:85-96`): regex `/prompt is too long[^0-9]*(\d+)\s*tokens?\s*>\s*(\d+)/i`.
- Rate-limit headers parsed in `errors.ts:471-516`: `anthropic-ratelimit-unified-representative-claim` (`'five_hour'|'seven_day'|'seven_day_opus'`), `anthropic-ratelimit-unified-overage-status` (`'allowed'|'allowed_warning'|'rejected'`), `anthropic-ratelimit-unified-reset` (Number → `limits.resetsAt`), `anthropic-ratelimit-unified-overage-reset`, `anthropic-ratelimit-unified-overage-disabled-reason`.
- Rate-limit reset header: `anthropic-ratelimit-unified-reset` (Number unix-sec → `delayMs = resetUnixSec*1000 - Date.now()`, clamped to `PERSISTENT_RESET_CAP_MS`) (`withRetry.ts:814-822`).
- 429 inner-message extraction: `error.message.replace(/^429\s+/, '')` then `/"message"\s*:\s*"([^"]*)"/` (`errors.ts:551-557`).
- Tool-use ID regex: `/toolu_[a-zA-Z0-9]+/` (`errors.ts:676`).

### 6.5 `categorizeRetryableAPIError` (verbatim, `errors.ts:1163-1182`)

```ts
export function categorizeRetryableAPIError(
  error: APIError,
): SDKAssistantMessageError {
  if (
    error.status === 529 ||
    error.message?.includes('"type":"overloaded_error"')
  ) {
    return 'rate_limit'
  }
  if (error.status === 429) {
    return 'rate_limit'
  }
  if (error.status === 401 || error.status === 403) {
    return 'authentication_failed'
  }
  if (error.status !== undefined && error.status >= 408) {
    return 'server_error'
  }
  return 'unknown'
}
```

### 6.6 `classifyAPIError` mapping (verbatim, `errors.ts:962-1161`)

Branch order (first match wins):
1. `Error.message === 'Request was aborted.'` → `'aborted'`
2. `APIConnectionTimeoutError` OR (`APIConnectionError` and message has `'timeout'` ci) → `'api_timeout'`
3. message includes `REPEATED_529_ERROR_MESSAGE` (`'Repeated 529 Overloaded errors'`) → `'repeated_529'`
4. message includes `CUSTOM_OFF_SWITCH_MESSAGE` (`'Opus is experiencing high load, please use /model to switch to Sonnet'`) → `'capacity_off_switch'`
5. status 429 → `'rate_limit'`
6. status 529 OR message includes `'"type":"overloaded_error"'` → `'server_overload'`
7. message includes `'prompt is too long'` (ci) → `'prompt_too_long'`
8. `/maximum of \d+ PDF pages/` → `'pdf_too_large'`
9. message includes `'The PDF specified is password protected'` → `'pdf_password_protected'`
10. status 400 + `'image exceeds'` + `'maximum'` → `'image_too_large'`
11. status 400 + `'image dimensions exceed'` + `'many-image'` → `'image_too_large'`
12. status 400 + `` '`tool_use` ids were found without `tool_result` blocks immediately after' `` → `'tool_use_mismatch'`
13. status 400 + `` 'unexpected `tool_use_id` found in `tool_result`' `` → `'unexpected_tool_result'`
14. status 400 + `` '`tool_use` ids must be unique' `` → `'duplicate_tool_use_id'`
15. status 400 + `'invalid model name'` (ci) → `'invalid_model'`
16. message includes `'credit balance is too low'` (ci) → `'credit_balance_low'`
17. message includes `'x-api-key'` (ci) → `'invalid_api_key'`
18. status 403 + `'OAuth token has been revoked'` → `'token_revoked'`
19. (status 401|403) + `'OAuth authentication is currently not allowed for this organization'` → `'oauth_org_not_allowed'`
20. status 401|403 → `'auth_error'`
21. `CLAUDE_CODE_USE_BEDROCK` + `Error.message.includes('model id')` (ci) → `'bedrock_model_access'`
22. `APIError`: status >= 500 → `'server_error'`; else status >= 400 → `'client_error'`
23. `APIConnectionError`: SSL via `extractConnectionErrorDetails().isSSLError` → `'ssl_cert_error'`; else `'connection_error'`
24. else `'unknown'`

### 6.7 User-facing error strings (verbatim, all from `errors.ts`)

```ts
const API_ERROR_MESSAGE_PREFIX = 'API Error'
const PROMPT_TOO_LONG_ERROR_MESSAGE = 'Prompt is too long'
const CREDIT_BALANCE_TOO_LOW_ERROR_MESSAGE = 'Credit balance is too low'
const INVALID_API_KEY_ERROR_MESSAGE = 'Not logged in · Please run /login'
const INVALID_API_KEY_ERROR_MESSAGE_EXTERNAL = 'Invalid API key · Fix external API key'
const ORG_DISABLED_ERROR_MESSAGE_ENV_KEY_WITH_OAUTH =
  'Your ANTHROPIC_API_KEY belongs to a disabled organization · Unset the environment variable to use your subscription instead'
const ORG_DISABLED_ERROR_MESSAGE_ENV_KEY =
  'Your ANTHROPIC_API_KEY belongs to a disabled organization · Update or unset the environment variable'
const TOKEN_REVOKED_ERROR_MESSAGE = 'OAuth token revoked · Please run /login'
const CCR_AUTH_ERROR_MESSAGE = 'Authentication error · This may be a temporary network issue, please try again'
const REPEATED_529_ERROR_MESSAGE = 'Repeated 529 Overloaded errors'
const CUSTOM_OFF_SWITCH_MESSAGE = 'Opus is experiencing high load, please use /model to switch to Sonnet'
const API_TIMEOUT_ERROR_MESSAGE = 'Request timed out'
const OAUTH_ORG_NOT_ALLOWED_ERROR_MESSAGE =
  'Your account does not have access to Claude Code. Please run /login.'
```

Mode-conditional strings (`getIsNonInteractiveSession()` flips first vs second):
- PDF too large: `'PDF too large (max ${API_PDF_MAX_PAGES} pages, ${formatFileSize(PDF_TARGET_RAW_SIZE)}). Try reading the file a different way (e.g., extract text with pdftotext).'` ↔ `'… Double press esc to go back and try again, or use pdftotext to convert to text first.'`
- PDF password-protected: `'PDF is password protected. Try using a CLI tool to extract or convert the PDF.'` ↔ `'PDF is password protected. Please double press esc to edit your message and try again.'`
- PDF invalid: `'The PDF file was not valid. Try converting it to text first (e.g., pdftotext).'` ↔ `'The PDF file was not valid. Double press esc to go back and try again with a different file.'`
- Image too large: `'Image was too large. Try resizing the image or using a different approach.'` ↔ `'Image was too large. Double press esc to go back and try again with a smaller image.'`
- Request too large: `'Request too large (max ${formatFileSize(PDF_TARGET_RAW_SIZE)}). Try with a smaller file.'` ↔ `'… Double press esc to go back and try with a smaller file.'`
- Token revoked: `'Your account does not have access to Claude. Please login again or contact your administrator.'` ↔ `TOKEN_REVOKED_ERROR_MESSAGE`
- OAuth org not allowed: `'Your organization does not have access to Claude. Please login again or contact your administrator.'` ↔ `OAUTH_ORG_NOT_ALLOWED_ERROR_MESSAGE`

Other verbatim strings emitted by this layer:
- 1M-context entitlement 429: `'API Error: Extra usage is required for 1M context · enable extra usage at claude.ai/settings/usage, or use --model to switch to standard context'` ↔ `'… run /extra-usage to enable, or /model to switch to standard context'` (`errors.ts:540-547`).
- Generic 429 fallback: `'API Error: Request rejected (429) · ${detail || "this may be a temporary capacity issue — check status.anthropic.com"}'` (`errors.ts:553-557`).
- Many-image dimension: `'An image in the conversation exceeds the dimension limit for many-image requests (2000px). Start a new session with fewer images.'` ↔ `'… Run /compact to remove old images from context, or start a new session.'` (`errors.ts:632-635`).
- Auto-mode beta rejection: `'Auto mode is unavailable for your plan'` (`errors.ts:651-654`).
- Tool-use mismatch (ANT): `` `API Error: 400 ${error.message}\n\nRun /share and post the JSON file to ${MACRO.FEEDBACK_CHANNEL}.[ Then, use /rewind to recover the conversation.]` ``; non-ANT: `'API Error: 400 due to tool use concurrency issues.[ Run /rewind to recover the conversation.]'` (`errors.ts:687-705`).
- Duplicate tool_use ID: `` `API Error: 400 duplicate tool_use ID in conversation history.[ Run /rewind to recover the conversation.]` `` (`errors.ts:719-732`).
- Subscriber-Opus 400: `'Claude Opus is not available with the Claude Pro plan. If you have updated your subscription plan recently, run /logout and /login for the plan to take effect.'` (`errors.ts:743-748`).
- ANT invalid-model: `` `[ANT-ONLY] Your org isn't gated into the \`${model}\` model. Either run \`claude\` with \`ANTHROPIC_MODEL=${getDefaultMainLoopModelSetting()}\`{ or share your orgId (${orgId}) in ${MACRO.FEEDBACK_CHANNEL} for help getting access. | or reach out in ${MACRO.FEEDBACK_CHANNEL} for help getting access.}` `` (`errors.ts:760-770`).
- 401/403 generic: `` `Failed to authenticate. ${API_ERROR_MESSAGE_PREFIX}: ${error.message}` `` ↔ `` `Please run /login · ${API_ERROR_MESSAGE_PREFIX}: ${error.message}` `` (`errors.ts:879-882`).
- Bedrock model access: `` `${API_ERROR_MESSAGE_PREFIX} (${model}): ${error.message}. Try ${switchCmd} to switch to ${fallbackSuggestion}.` `` else `` `… Run ${switchCmd} to pick a different model.` `` (`errors.ts:894-898`).
- 404 model: `` `The model ${model} is not available on your ${getAPIProvider()} deployment. Try ${switchCmd} to switch to ${fallbackSuggestion}, or ask your admin to enable this model.` `` else `` `There's an issue with the selected model (${model}). It may not exist or you may not have access to it. Run ${switchCmd} to pick a different model.` `` (`errors.ts:908-913`).
- Refusal stop_reason: `` `${API_ERROR_MESSAGE_PREFIX}: Claude Code is unable to respond to this request, which appears to violate our Usage Policy (https://www.anthropic.com/legal/aup). [Try rephrasing the request or attempting a different approach. | Please double press esc to edit your last message or start a new session for Claude Code to assist with a different task.]` `` (+ `' If you are seeing this refusal repeatedly, try running /model claude-sonnet-4-20250514 to switch models.'` when `model !== 'claude-sonnet-4-20250514'`) (`errors.ts:1184-1207`).
- Connection error / SSL hint strings (`errorUtils.ts:99, 200-260`): `` `SSL certificate error (${details.code}). If you are behind a corporate proxy or TLS-intercepting firewall, set NODE_EXTRA_CA_CERTS to your CA bundle path, or ask IT to allowlist *.anthropic.com. Run /doctor for details.` ``; per-code SSL messages: `'Unable to connect to API: SSL certificate verification failed. Check your proxy or corporate SSL certificates'`, `'Unable to connect to API: SSL certificate has expired'`, `'Unable to connect to API: SSL certificate has been revoked'`, `'Unable to connect to API: Self-signed certificate detected. …'`, `'Unable to connect to API: SSL certificate hostname mismatch'`, `'Unable to connect to API: SSL certificate is not yet valid'`, `'Unable to connect to API: SSL error (${code})'`. Generic timeout: `'Request timed out. Check your internet connection and proxy settings'`. Generic connection error: `'Unable to connect to API. Check your internet connection'` or `'Unable to connect to API (${connectionDetails.code})'`.
- max_output_tokens (`claude.ts:2271-2273`): `` `${API_ERROR_MESSAGE_PREFIX}: Claude's response exceeded the ${maxOutputTokens} output token maximum. To configure this behavior, set the CLAUDE_CODE_MAX_OUTPUT_TOKENS environment variable.` ``.
- model_context_window_exceeded (`claude.ts:2287-2291`): `` `${API_ERROR_MESSAGE_PREFIX}: The model has reached its context window limit.` ``.
- `FallbackTriggeredError.message` (`withRetry.ts:165`): `` `Model fallback triggered: ${originalModel} -> ${fallbackModel}` ``.

### 6.8 `updateUsage` (verbatim, `claude.ts:2924-2987`)

Reproduced verbatim — see source listing in §5.2 above. Key invariants:

- For `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`: keep prior value if `partUsage.x === null || partUsage.x <= 0`.
- For `output_tokens`: `partUsage.output_tokens ?? usage.output_tokens` (uses latest non-undefined).
- `server_tool_use.{web_search_requests,web_fetch_requests}`: `?? prior`.
- `service_tier`: keep prior (always). **REIMPLEMENTER HAZARD — intentional asymmetry with `accumulateUsage`:** within a single message (this function), `service_tier` is *frozen* at `message_start` and never updated by mid-stream `message_delta` events — the rationale is to avoid mid-stream churn polluting the per-message tier reading. Across messages (`accumulateUsage` §6.9), the *most recent* `service_tier` wins. The two functions deliberately disagree; do NOT "fix" `updateUsage` to track latest, and do NOT "fix" `accumulateUsage` to keep prior. Same pattern applies to `inference_geo` (kept prior here, latest in `accumulateUsage`).
- `cache_creation.{ephemeral_1h_input_tokens, ephemeral_5m_input_tokens}`: `(partUsage as BetaUsage).cache_creation?.x ?? prior`.
- `feature('CACHED_MICROCOMPACT')` only: `cache_deleted_input_tokens` follows the same `>0` guard as token fields, default 0.
- `inference_geo`: keep prior (always).
- `iterations`: `partUsage.iterations ?? prior`.
- `speed`: `(partUsage as BetaUsage).speed ?? prior`.

### 6.9 `accumulateUsage` (verbatim, `claude.ts:2993-3038`)

```ts
return {
  input_tokens: totalUsage.input_tokens + messageUsage.input_tokens,
  cache_creation_input_tokens:
    totalUsage.cache_creation_input_tokens + messageUsage.cache_creation_input_tokens,
  cache_read_input_tokens:
    totalUsage.cache_read_input_tokens + messageUsage.cache_read_input_tokens,
  output_tokens: totalUsage.output_tokens + messageUsage.output_tokens,
  server_tool_use: {
    web_search_requests:
      totalUsage.server_tool_use.web_search_requests + messageUsage.server_tool_use.web_search_requests,
    web_fetch_requests:
      totalUsage.server_tool_use.web_fetch_requests + messageUsage.server_tool_use.web_fetch_requests,
  },
  service_tier: messageUsage.service_tier, // Use the most recent service tier
  cache_creation: {
    ephemeral_1h_input_tokens:
      totalUsage.cache_creation.ephemeral_1h_input_tokens + messageUsage.cache_creation.ephemeral_1h_input_tokens,
    ephemeral_5m_input_tokens:
      totalUsage.cache_creation.ephemeral_5m_input_tokens + messageUsage.cache_creation.ephemeral_5m_input_tokens,
  },
  ...(feature('CACHED_MICROCOMPACT') ? { cache_deleted_input_tokens: (totalUsage.cache_deleted_input_tokens ?? 0) + (messageUsage.cache_deleted_input_tokens ?? 0) } : {}),
  inference_geo: messageUsage.inference_geo, // Use the most recent
  iterations: messageUsage.iterations,       // Use the most recent
  speed: messageUsage.speed,                 // Use the most recent
}
```

**REIMPLEMENTER HAZARD — intentional asymmetry with `updateUsage`:** `service_tier`, `inference_geo`, `iterations`, `speed` use the *most-recent* (`messageUsage.x`) value here. In `updateUsage` (§6.8), `service_tier` and `inference_geo` are *kept prior* (frozen at `message_start`); only `iterations` and `speed` use `?? prior` (latest non-undefined). The two-stage policy is deliberate: freeze tier reading within a message to avoid mid-stream churn, but propagate the latest tier across messages so the session reflects the active tier. Do NOT unify these.

### 6.10 Default headers on every Anthropic client (`client.ts:104-129`)

```
defaultHeaders = {
  'x-app': 'cli',
  'User-Agent': getUserAgent(),
  'X-Claude-Code-Session-Id': getSessionId(),
  ...customHeaders,                                          # parsed from process.env.ANTHROPIC_CUSTOM_HEADERS, "Name: Value" newline-separated
  ['x-claude-remote-container-id'?]: process.env.CLAUDE_CODE_CONTAINER_ID,
  ['x-claude-remote-session-id'?]:   process.env.CLAUDE_CODE_REMOTE_SESSION_ID,
  ['x-client-app'?]:                 process.env.CLAUDE_AGENT_SDK_CLIENT_APP,
  ['x-anthropic-additional-protection'?]: 'true' if env.CLAUDE_CODE_ADDITIONAL_PROTECTION truthy,
  ['Authorization'?]: 'Bearer ${ANTHROPIC_AUTH_TOKEN || apiKeyHelper(...)}'  if !subscriber,  # configureApiKeyHeaders
}
```

Per-request: `x-client-request-id: randomUUID()` (only first-party + `isFirstPartyAnthropicBaseUrl()`); pre-set values are preserved (`client.ts:368-388`).

Client `ARGS`: `{ defaultHeaders, maxRetries, timeout: parseInt(env.API_TIMEOUT_MS || String(600*1000), 10), dangerouslyAllowBrowser: true, fetchOptions: getProxyFetchOptions({forAnthropicAPI:true}), fetch?: resolvedFetch }` (`client.ts:141-152`).

Provider switch (mutually exclusive, in order):
1. `CLAUDE_CODE_USE_BEDROCK` truthy → dynamic `import('@anthropic-ai/bedrock-sdk')`. AWS region: small-fast-model gets `ANTHROPIC_SMALL_FAST_MODEL_AWS_REGION` if set, else `getAWSRegion()`. Auth: `AWS_BEARER_TOKEN_BEDROCK` → `Authorization: Bearer …` and `skipAuth=true`; else `refreshAndGetAwsCredentials()` for keys/session token; or `CLAUDE_CODE_SKIP_BEDROCK_AUTH` → `skipAuth=true`. Returns `AnthropicBedrock`.
2. `CLAUDE_CODE_USE_FOUNDRY` truthy → dynamic `import('@anthropic-ai/foundry-sdk')`. If `ANTHROPIC_FOUNDRY_API_KEY` unset: either mock (`CLAUDE_CODE_SKIP_FOUNDRY_AUTH`) or `getBearerTokenProvider(new DefaultAzureCredential(), 'https://cognitiveservices.azure.com/.default')`. Returns `AnthropicFoundry`.
3. `CLAUDE_CODE_USE_VERTEX` truthy → `refreshGcpCredentialsIfNeeded()` (unless `CLAUDE_CODE_SKIP_VERTEX_AUTH`); dynamic `import('@anthropic-ai/vertex-sdk', 'google-auth-library')`. `region: getVertexRegionForModel(model)`. `googleAuth`: real or mock. Project-ID fallback uses `ANTHROPIC_VERTEX_PROJECT_ID` only if no env-var/keyfile present. Returns `AnthropicVertex`.
4. else first-party `Anthropic` constructor: `apiKey: subscriber ? null : (apiKey || getAnthropicApiKey())`; `authToken: subscriber ? getClaudeAIOAuthTokens()?.accessToken : undefined`; staging override `baseURL: getOauthConfig().BASE_API_URL` only if `USER_TYPE==='ant' && USE_STAGING_OAUTH`.

### 6.11 Critical literal constants

| Constant | Value | Cite |
|---|---|---|
| Default API timeout | `parseInt(env.API_TIMEOUT_MS || '600000', 10)` (= 10 min) | `client.ts:144` |
| Non-streaming fallback timeout (CCR) | `120_000` ms | `claude.ts:810` |
| Non-streaming fallback timeout (default) | `300_000` ms | `claude.ts:810` |
| `MAX_NON_STREAMING_TOKENS` | `64_000` | `claude.ts:3354` |
| Stream idle watchdog default | `90_000` ms (env override `CLAUDE_STREAM_IDLE_TIMEOUT_MS`) | `claude.ts:1877-1878` |
| Stream stall log threshold | `30_000` ms | `claude.ts:1936` |
| Cache-break min token drop | `MIN_CACHE_MISS_TOKENS = 2_000` | `promptCacheBreakDetection.ts:120` |
| Cache TTL probes | `CACHE_TTL_5MIN_MS = 5*60_000`; `CACHE_TTL_1HOUR_MS = 60*60_000` | `promptCacheBreakDetection.ts:125-126` |
| `MAX_TRACKED_SOURCES` | `10` | `promptCacheBreakDetection.ts:107` |
| Max media per request | `API_MAX_MEDIA_PER_REQUEST` (`constants/apiLimits.ts`) | `claude.ts:1313` |
| Files API `MAX_FILE_SIZE_BYTES` | `500 * 1024 * 1024` | `filesApi.ts:82` |
| Files API `MAX_RETRIES` | `3` | `filesApi.ts:80` |
| Files API `BASE_DELAY_MS` | `500` | `filesApi.ts:81` |
| Files API download/list timeout | `60_000` ms | `filesApi.ts:152, 649` |
| Files API upload timeout | `120_000` ms | `filesApi.ts:466` |
| Bootstrap fetch timeout | `5000` ms | `bootstrap.ts:91` |
| Files API beta header | `'files-api-2025-04-14,oauth-2025-04-20'` | `filesApi.ts:27` |
| `ANTHROPIC_VERSION` | `'2023-06-01'` | `filesApi.ts:28` |

## 7. Side Effects & I/O

- **Network**: HTTPS to `${ANTHROPIC_BASE_URL || provider-default}` for `POST /v1/beta/messages` (streaming + non-streaming), `GET /api/claude_cli/bootstrap`, `GET/POST/LIST /v1/files`. Provider switching adds AWS Bedrock, Vertex AI, Azure Foundry endpoints.
- **Filesystem**: `globalConfig` writes (bootstrap cache); cache-break diff files at `${getClaudeTempDir()}/cache-break-${random4}.diff`; Files API saves into `${cwd}/${sessionId}/uploads/${cleanPath}`; VCR cassettes under `${getClaudeConfigHomeDir()}/...` (when gating allows).
- **Environment variables consumed**: see §2 table.
- **External binaries**: none directly (the SDKs handle network); proxy via `getProxyFetchOptions({forAnthropicAPI:true})`.
- **Process / signals**: none (no `spawn`, no signal handlers); `AbortSignal` is the cancellation channel.
- **Session activity**: `startSessionActivity('api_call')` / `stopSessionActivity('api_call')` around the for-await loop (`claude.ts:1931, 2809`).
- **Trust boundaries**: `CLAUDE_CODE_EXTRA_BODY` is parsed JSON spread into request body — explicit user trust (`claude.ts:272-313`). `CLAUDE_CODE_EXTRA_METADATA` similarly (`claude.ts:506-516`). `ANTHROPIC_CUSTOM_HEADERS` injects raw HTTP headers (`client.ts:330-353`). `apiKeyHelper` is invoked only on non-subscriber paths (`client.ts:135-137`, `configureApiKeyHeaders`).

## 8. Feature Flags & Variants

| Flag / env / role | Behavior on | Behavior off | Cite |
|---|---|---|---|
| `feature('ANTI_DISTILLATION_CC')` | injects `anti_distillation: ['fake_tools']` extra-body if 1P CLI + `tengu_anti_distill_fake_tool_injection` | absent | `claude.ts:303-313` |
| `feature('CONNECTOR_TEXT')` | exports `SUMMARIZE_CONNECTOR_TEXT_BETA_HEADER`; handles `connector_text` blocks/deltas; counts `connectorTextBlockCount` | empty header; no connector_text branch | `betas.ts:23-25`; `claude.ts:661-662, 2067-2081, 661-665, 663` |
| `feature('TRANSCRIPT_CLASSIFIER')` | `AFK_MODE_BETA_HEADER='afk-mode-2026-01-31'`; AFK header latched; auto-mode active branch | empty header | `betas.ts:26-28`; `claude.ts:1413-1423` |
| `feature('CACHED_MICROCOMPACT')` | imports `cachedMicrocompact` lazily; loads `CACHE_EDITING_BETA_HEADER`; supports `cache_edits` blocks via `addCacheBreakpoints`; persists `cache_deleted_input_tokens` in usage; logs `cacheDeletedInputTokens` analytics; `markToolsSentToAPIState()` post-success | none of those paths active; no cache_edits | `claude.ts:1190-1205, 2970-2982, 3022-3033, 3108-3163, 2834-2836`; `logging.ts:557-566` |
| `feature('PROMPT_CACHE_BREAK_DETECTION')` | `recordPromptState` + `checkResponseForCacheBreak` instrumented | no-op | `claude.ts:1460-1486, 2382-2392` |
| `feature('UNATTENDED_RETRY')` | when `CLAUDE_CODE_UNATTENDED_RETRY` truthy, persistent retry mode active (chunked sleeps with heartbeats, ignores `x-should-retry:false`, no max-attempts) | normal retry | `withRetry.ts:100-104` |
| `feature('NATIVE_CLIENT_ATTESTATION')` | attribution adds `' cch=00000;'` placeholder | no `cch` segment | `constants/system.ts:64,82` |
| `feature('BASH_CLASSIFIER')` | `'bash_classifier'` in foreground 529 retry sources | excluded (DCE) | `withRetry.ts:81` |
| `process.env.USER_TYPE === 'ant'` | mock-rate-limit checks; ANT-only error strings; ANT-only `cli-internal-2026-02-09` beta; ANT-only thinking output in tracing; ANT-only `research` block capture; staging OAuth `baseURL`; ANT 5xx bypasses `x-should-retry:false`; VCR `FORCE_VCR` available | external user paths | many (see §2) |
| `process.env.CLAUDE_CODE_REMOTE` | 401/403 retried; non-streaming fallback timeout 120s; CCR_AUTH_ERROR_MESSAGE for `x-api-key`/auth surfacing; bypass `disableFallback` | first-party paths | `withRetry.ts:712-717`; `claude.ts:807-811`; `errors.ts:217-219, 818-822, 870-874` |
| `process.env.FALLBACK_FOR_ALL_PRIMARY_MODELS` | 529 budget engages on ALL models, not just non-custom Opus | only non-custom Opus | `withRetry.ts:330-333` |
| `process.env.CLAUDE_CODE_DISABLE_THINKING` | `hasThinking=false` | thinking enabled per model | `claude.ts:1597-1598` |
| `process.env.CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING` | use budget-based thinking even on adaptive-capable models | adaptive thinking when supported | `claude.ts:1606-1607` |
| `process.env.CLAUDE_CODE_MAX_OUTPUT_TOKENS` | `validateBoundedIntEnvVar(default, upperLimit)` — env override | model default (or `CAPPED_DEFAULT_MAX_TOKENS` when `tengu_otk_slot_v1`) | `claude.ts:3399-3418` |
| `process.env.CLAUDE_CODE_MAX_RETRIES` | `parseInt(...,10)` | `DEFAULT_MAX_RETRIES=10` | `withRetry.ts:789-793` |
| `process.env.CLAUDE_CODE_DISABLE_NONSTREAMING_FALLBACK` / `tengu_disable_streaming_to_non_streaming_fallback` | propagate streaming error | mid-stream fallback to non-streaming | `claude.ts:2469-2474` |
| `process.env.CLAUDE_CODE_ENTRYPOINT === 'cli'` | gates `ANTI_DISTILLATION_CC` | extra-body `anti_distillation` not added | `claude.ts:304` |

Sticky-on latches (cache-stable; cleared on `/clear`, `/compact`): AFK mode header, fast-mode header, cache-editing header, thinking-clear (latched once `Date.now() - lastApiCompletionTimestamp > CACHE_TTL_1HOUR_MS` for an agentic query). Per-call gates remain (`isAgenticQuery`, provider, `querySource==='repl_main_thread'`).

## 9. Error Handling & Edge Cases

- **User abort**: `APIUserAbortError` with `signal.aborted===true` → re-throw; with `!signal.aborted` → translate to `APIConnectionTimeoutError({message:'Request timed out'})` (`claude.ts:2434-2462`). On user abort, no error message is yielded — interruption message handled by `query.ts`.
- **Stream-watchdog abort**: throws `'Stream idle timeout - no chunks received'`; goes through normal fallback path (`claude.ts:2310-2335`).
- **Empty stream / partial stream**: throws `'Stream ended without receiving any events'` if `!partialMessage` OR (`newMessages.length===0 && !stopReason`). Avoids false-positive on legitimate end_turn after structured-output tool use (`claude.ts:2350-2364`).
- **`max_tokens` recovery**: yields error message, surfaces `apiError:'max_output_tokens'`; `query.ts` retries with escalated `max_output_tokens`.
- **`model_context_window_exceeded` recovery**: same `apiError:'max_output_tokens'` path; reuses recovery wiring.
- **400 prompt-too-long**: `getAssistantMessageFromError` returns content `PROMPT_TOO_LONG_ERROR_MESSAGE` and stores raw API error string in `errorDetails`; `getPromptTooLongTokenGap` parses `(\d+) tokens > (\d+)` for reactive compact (`errors.ts:560-573, 85-118`).
- **400 context-overflow** (legacy 400): `withRetry` adjusts `retryContext.maxTokensOverride` to `max(FLOOR_OUTPUT_TOKENS=3000, contextLimit-inputTokens-1000, thinkingBudget+1)` and `continue`s (`withRetry.ts:388-426`).
- **413**: yields request-too-large error.
- **404**: special handling for stream-creation 404 → non-streaming fallback (`claude.ts:2607-2666`).
- **Connection errors**: classified via `extractConnectionErrorDetails`; SSL codes mapped to user-facing strings; SSL hint exported via `getSSLErrorHint`.
- **Fast-mode failure modes**: 429/529 with short `Retry-After` → wait-and-retry preserving fast-mode (cache-friendly); long/unknown → `triggerFastModeCooldown` (min 10 min), drop fast-mode for retry. Header `anthropic-ratelimit-unified-overage-disabled-reason` → `handleFastModeOverageRejection` (permanent disable). `'Fast mode is not enabled'` 400 → `handleFastModeRejectedByAPI` (permanent disable).
- **529 cascade**: after `MAX_529_RETRIES=3` consecutive 529s on Opus (or all models when `FALLBACK_FOR_ALL_PRIMARY_MODELS`): if `options.fallbackModel` set → throw `FallbackTriggeredError` (query.ts switches model, retries cleanly with fresh state); else if external+!sandbox+!persistent → `CannotRetryError(REPEATED_529_ERROR_MESSAGE)`.
- **Non-foreground 529**: `shouldRetry529 === false` → drop with `tengu_api_529_background_dropped`, throw `CannotRetryError`. Capacity-cascade-amplification protection.
- **Auth errors**: 401 clears `apiKeyHelper` cache; 401-token-expired or 403-revoked OAuth triggers `handleOAuth401Error(failedAccessToken)` and forces `getClient()` again. CCR mode: 401/403 retried as transient.
- **`ECONNRESET` / `EPIPE`**: `disableKeepAlive()` if GrowthBook `tengu_disable_keepalive_on_econnreset`; force `getClient()` again.
- **Generator early-termination**: `releaseStreamResources()` in `finally` cancels `streamResponse.body` and aborts the stream controller — required to free native TLS/socket buffers (GH #32920).
- **Fallback message cost**: tracked outside `message_delta` path (in `finally`) via `usage = updateUsage(EMPTY_USAGE, fallbackUsage); calculateUSDCost; addToTotalSessionCost` (`claude.ts:2820-2830`).

## 10. Telemetry & Observability

Emitted analytics events (each via `logEvent(name, AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS-typed payload)`):

`tengu_api_query`, `tengu_api_success`, `tengu_api_error`, `tengu_api_retry`, `tengu_api_persistent_retry_wait`, `tengu_api_529_background_dropped`, `tengu_api_opus_fallback_triggered`, `tengu_api_custom_529_overloaded_error`, `tengu_api_cache_breakpoints`, `tengu_api_before_normalize`, `tengu_api_after_normalize`, `tengu_max_tokens_reached`, `tengu_max_tokens_context_overflow_adjustment`, `tengu_context_window_exceeded`, `tengu_off_switch_query`, `tengu_refusal_api_response`, `tengu_streaming_error`, `tengu_streaming_stall`, `tengu_streaming_stall_summary`, `tengu_streaming_idle_timeout`, `tengu_streaming_fallback_to_non_streaming`, `tengu_nonstreaming_fallback_started`, `tengu_nonstreaming_fallback_error`, `tengu_stream_no_events`, `tengu_stream_loop_exited_after_watchdog`, `tengu_advisor_tool_call`, `tengu_advisor_tool_interrupted`, `tengu_tool_use_tool_result_mismatch_error`, `tengu_unexpected_tool_result`, `tengu_duplicate_tool_use_id`, `tengu_prompt_cache_break`, `tengu_teleport_first_message_success`, `tengu_teleport_first_message_error`, `tengu_file_upload_failed`, `tengu_file_list_failed`.

OTLP events: `api_request`, `api_error` (`logging.ts:368-375, 717-727`).

OpenTelemetry tracing: `startLLMRequestSpan` (begin) / `endLLMRequestSpan` (success or failure) when `isBetaTracingEnabled()`. Captures retry attempts via `attemptStartTimes[]`, `requestSetupMs = start - startIncludingRetries`, optional `modelOutput` (all users), `thinkingOutput` (ANT only), `hasToolCall`, `ttftMs`. Spans are explicitly threaded so parallel agents don't cross-bind.

Diagnostic-only logs (no PII): `cli_nonstreaming_fallback_error`, `cli_nonstreaming_fallback_started`, `cli_streaming_idle_warning`, `cli_streaming_idle_timeout`, `cli_stream_loop_exited_after_watchdog_clean`, `cli_stream_loop_exited_after_watchdog_error`.

Gateway detection (`logging.ts:107-138`): header-prefix sniffing for `litellm`, `helicone`, `portkey`, `cloudflare-ai-gateway`, `kong`, `braintrust`; URL-suffix sniffing for `databricks` (`*.cloud.databricks.com`, `*.azuredatabricks.net`, `*.gcp.databricks.com`).

`x-client-request-id` (`client.ts:356, 1813-1816`): generated client-side for first-party (so timeouts, which return no server request-id, can still be looked up by API team). Logged on every API request and on errors.

## 11. Reimplementation Checklist

A reimplementer must preserve:
1. SSE event-name set and dispatch order (§5.2). `message_delta` mutates the last assistant message's `usage` and `stop_reason` IN-PLACE (transcript queue holds reference).
2. `updateUsage` `>0`-guard for input-side fields; `??`-fallback for output_tokens and other fields. `accumulateUsage` sum semantics with most-recent overlay for `service_tier`/`speed`/`iterations`/`inference_geo`.
3. Retry algorithm: `BASE_DELAY_MS=500` ms, jitter `Math.random()*0.25*baseDelay`, default `maxDelayMs=32_000` ms (32 s), `Retry-After` honored verbatim (no jitter, no maxDelayMs cap), `MAX_529_RETRIES=3`, `DEFAULT_MAX_RETRIES=10`, `FLOOR_OUTPUT_TOKENS=3000`, persistent-mode `PERSISTENT_MAX_BACKOFF_MS=300_000` ms (5 min), `PERSISTENT_RESET_CAP_MS=21_600_000` ms (6 h), `HEARTBEAT_INTERVAL_MS=30_000` ms (30 s), `MIN_COOLDOWN_MS=600_000` ms (= `10 * 60 * 1000`, 10 min — verbatim `withRetry.ts:801`), `SHORT_RETRY_THRESHOLD_MS=20_000` ms (20 s), `DEFAULT_FAST_MODE_FALLBACK_HOLD_MS=1_800_000` ms (= `30 * 60 * 1000`, 30 min — verbatim `withRetry.ts:799`). All literal underscores match source; the `(N min)` annotations are reader aids and not in source.
4. `shouldRetry` decision tree exact ordering (mock → persistent transient → CCR-401/403 → overloaded → context-overflow → x-should-retry → APIConnectionError → 408/409 → 429-subscriber-gate → 401-clear-cache → OAuth-revoked-403 → 5xx).
5. `FOREGROUND_529_RETRY_SOURCES` exact set; `shouldRetry529(undefined)===true`.
6. `is529Error` covers both status===529 and message-contains `'"type":"overloaded_error"'` (SDK can drop status in streaming).
7. `categorizeRetryableAPIError` exact branch order (529/overloaded → 429 → 401/403 → ≥408 → unknown).
8. `classifyAPIError` exact 24-branch ordering (§6.6).
9. Error-message strings (§6.7) verbatim, including `' · '` separators and conditional non-interactive variants.
10. Beta header values verbatim and the latch policy: once the header has been first sent in the session, keep sending it until `/clear` or `/compact` (cache-stability invariant).
11. Default headers on every client (`x-app:'cli'`, `User-Agent`, `X-Claude-Code-Session-Id`, plus optional remote/protection/clientApp headers). `x-client-request-id` is per-request, first-party only, may be pre-set by caller.
12. Provider switching order: Bedrock > Foundry > Vertex > first-party. Each provider's auth error subset (Bedrock `CredentialsProviderError | 403`; Vertex `google-auth-library` cred messages | 401) bypasses `shouldRetry` to force a client refresh.
13. `addCacheBreakpoints` exactly-one-marker invariant; marker on last message default; second-to-last for `skipCacheWrite=true`; assistant marker NOT placed on `thinking`/`redacted_thinking`/`connector_text` last-block. Tool-result `cache_reference` populated for ALL tool_results strictly before the last `cache_control` marker.
14. `getCacheControl` defaults: `{type:'ephemeral'}` always; `ttl:'1h'` only if eligibility AND allowlist match, both latched.
15. `paramsFromContext` keys and ordering: `model, messages, system, tools, tool_choice?, betas?, metadata, max_tokens, thinking, temperature?, context_management?, ...extraBodyParams, output_config?, speed?`. **Deliberate**: `...extraBodyParams` is spread *before* `output_config` and `speed` so user-provided `CLAUDE_CODE_EXTRA_BODY` cannot shadow them. Reordering this object literal is a security regression — preserve the spread position.
16. Thinking mode: adaptive on capable models unless `CLAUDE_CODE_DISABLE_ADAPTIVE_THINKING`; budget = `min(maxOutputTokens-1, getMaxThinkingTokensForModel(model) | thinkingConfig.budgetTokens)`. Temperature OMITTED when thinking enabled; otherwise `options.temperatureOverride ?? 1`.
17. Mid-stream → non-streaming fallback wiring: per-attempt timeout (300s default, 120s in CCR), `MAX_NON_STREAMING_TOKENS=64_000`, `adjustParamsForNonStreaming` clamps `thinking.budget_tokens < max_tokens`. 529 carries forward via `initialConsecutive529Errors:1`.
18. `releaseStreamResources()` MUST run in `finally` regardless of how generator exits.
19. `FallbackTriggeredError` must propagate to `query.ts` (caller does the model switch); never swallowed in error-yield paths.
20. Files API: BETA header `'files-api-2025-04-14,oauth-2025-04-20'`; `anthropic-version: '2023-06-01'`; multipart `purpose=user_data`; max 500MB; download path traversal protection; concurrency cap default 5.
21. Bootstrap: only first-party + `!isEssentialTrafficOnly()`; OAuth (with `hasProfileScope()`) preferred, API key fallback; `withOAuth401Retry`; persist on diff only.

Spec is complete when:
- Re-implementer can build a behaviorally-equivalent layer driving the Anthropic SDK with byte-identical request bodies (modulo timestamps, randomized client-request-id), identical SSE handling, identical retry timing distributions (within jitter), and identical user-facing error strings.
- All beta headers, env vars, feature flags, and ANT branches enumerated in §2 §6 §8 are honored.
- All telemetry events fire at the same call sites.

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited.

1. ~~**Files API base URL precedence**~~ — **NOTE Phase 9.7**: behavior at `filesApi.ts:32-38` documented as observed. Bedrock/Vertex env-var routing is guarded upstream by `getCloudProvider()` checks (spec 25 / 27 territory); `ANTHROPIC_BASE_URL` is reserved for Anthropic-direct routes. No mis-route risk identified in source review.
2. ~~**`MACRO.FEEDBACK_CHANNEL` / `MACRO.BUILD_TIME`**~~ — **DEFERRED**: bundler-injected globals; values not source-derivable. Recorded in spec 00 §13 as known-unfalsifiable build-macro injections.
3. ~~**`CACHE_EDITING_BETA_HEADER` value**~~ — **DEFERRED (missing-leaked-source)**: dynamic import target `src/constants/betas.ts` does not contain this literal in the leaked tree; likely DCE'd alongside `feature('CACHED_MICROCOMPACT')`. Recorded as missing.
4. ~~**`getMergedBetas` precise composition**~~ — **NOTE Phase 9.7**: `utils/betas.ts:397-428` is consumer-internal; spec 22 documents the consumer interface. The composition is `[...modelBetas, ...agenticOverlays, ...customHeaderBetas]` deduped — readable from the function body. Not blocking.
5. ~~**`getMaxThinkingTokensForModel` / `getModelMaxOutputTokens` / `CAPPED_DEFAULT_MAX_TOKENS`**~~ — **RESOLVED Phase 9.7**: model-registry helpers now enumerated in spec 42a §3 (Phase 10b cataloged the 17-file directory). Spec 06 consumes for cost calculation.
6. ~~**`getInferenceProfileBackingModel`**~~ — **RESOLVED Phase 9.7**: implementation in `src/utils/model/bedrock.ts` enumerated in spec 42a §3.
7. ~~**VCR cassette naming and fixture format**~~ — **NOTE Phase 9.7**: spec 42 §5.7 (and §6) documents VCR fixture key derivation and dehydration regexes; full fixture format is sampling-level documented, sufficient for understanding the gating.
8. ~~**`claudeAiLimits.ts` quota structure**~~ — **NOTE Phase 9.7**: spec 27 (service-policy) §3 enumerates `ClaudeAILimits`, `OverageDisabledReason`, `getRateLimitErrorMessage`. Verbatim message bodies in `src/services/rateLimitMessages.ts` deferred to spec 27 §6 inline-asset section per Phase 9.6 routing.
9. ~~**`output_config.format` shape**~~ — **DEFERRED**: SDK type `BetaJSONOutputFormat` is opaque from leak (no SDK source in tree). Recorded in spec 00 §13 as SDK-opaque known-unfalsifiable.
10. ~~**Stream `'error'` SSE event type**~~ — **DEFERRED**: actual on-the-wire event name (`error` vs `message_error`) not directly observable in our source — only the SDK-thrown error path is visible. SDK-internal contract; not source-derivable.
