# 13 — Web Tools (WebFetch & WebSearch) Specification

> The two network-touching tools shipped in the static registry. WebFetch performs an HTTPS GET, runs a domain preflight against `api.anthropic.com`, converts HTML → markdown via Turndown, and applies a user prompt with Haiku. WebSearch is a thin streaming wrapper around the Anthropic server-tool `web_search_20250305`. Anchor for the network-tool half of specs 10–19. Read 08 (registry/Tool interface) and 09 (permissions) before this.

---

## 1. Purpose & Scope

### IN scope
- `src/tools/WebFetchTool/` (5 files, total ~720 lines):
  - `WebFetchTool.ts` — `buildTool` definition, redirect message synthesis, preapproved-host fast path, prompt application orchestration.
  - `utils.ts` — caches (`URL_CACHE`, `DOMAIN_CHECK_CACHE`), `validateURL`, `checkDomainBlocklist` preflight, `getWithPermittedRedirects` axios wrapper, `isPermittedRedirect`, `getURLMarkdownContent` (the main fetch/decode/cache pipeline), `applyPromptToMarkdown` (Haiku call), error classes, ANT-only telemetry hook (`utils.ts:400`).
  - `prompt.ts` — `WEB_FETCH_TOOL_NAME`, `DESCRIPTION`, `makeSecondaryModelPrompt`.
  - `preapproved.ts` — `PREAPPROVED_HOSTS` (~80 entries), `HOSTNAME_ONLY` / `PATH_PREFIXES` split, `isPreapprovedHost`.
  - `UI.tsx` — `renderToolUseMessage`, `renderToolUseProgressMessage`, `renderToolResultMessage`, `getToolUseSummary`.
- `src/tools/WebSearchTool/` (3 files, ~530 lines):
  - `WebSearchTool.ts` — `buildTool` definition, `makeToolSchema` (server-tool schema), provider gating in `isEnabled`, streaming consumption of `server_tool_use` / `web_search_tool_result` / `text` blocks via `queryModelWithStreaming`, progress emission, `makeOutputFromSearchResponse` block-merge state machine, `mapToolResultToToolResultBlockParam` formatter.
  - `prompt.ts` — `WEB_SEARCH_TOOL_NAME`, `getWebSearchPrompt`.
  - `UI.tsx` — `renderToolUseMessage`, progress UI dispatch, `renderToolResultMessage`.
- All feature flags **including ANT paths** (the only ANT branch is the `tengu_web_fetch_host` analytics call at `utils.ts:400-405`; no `feature(...)` gates on either tool).

### OUT of scope
- Permission decision tree (rule lookup, `behavior:'allow'|'deny'|'ask'|'passthrough'` semantics) → 09.
- Tool registry assembly, `buildTool` factory mechanics, `ToolUseContext` shape → 08.
- Anthropic server-tool wiring inside `queryModelWithStreaming`, `usage.server_tool_use.web_search_requests` accounting, model-cost computation → 22 (cited, not redocumented).
- Analytics event router → 26.
- Per-MCP web-fetch alternatives (referenced in the prompt) → 16.

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/tools/WebFetchTool/WebFetchTool.ts` | 319 | Tool definition; preapproved-host short-circuit in `checkPermissions`; redirect message synthesis; binary-content append; Haiku-or-passthrough decision |
| `src/tools/WebFetchTool/utils.ts` | 531 | `URL_CACHE` (LRU 50MB, 15min TTL); `DOMAIN_CHECK_CACHE` (128 entries, 5min TTL); `validateURL`; `checkDomainBlocklist` (calls `api.anthropic.com/api/web/domain_info`); `getWithPermittedRedirects`; redirect-loop & egress-block detection; Turndown lazy singleton; `MAX_HTTP_CONTENT_LENGTH` 10MB; `FETCH_TIMEOUT_MS` 60s; `DOMAIN_CHECK_TIMEOUT_MS` 10s; `MAX_REDIRECTS` 10; `MAX_URL_LENGTH` 2000; `MAX_MARKDOWN_LENGTH` 100k; `applyPromptToMarkdown` (Haiku call); ANT-only `tengu_web_fetch_host` event |
| `src/tools/WebFetchTool/prompt.ts` | 46 | `WEB_FETCH_TOOL_NAME = 'WebFetch'`; `DESCRIPTION` body; `makeSecondaryModelPrompt(content, prompt, isPreapprovedDomain)` — branched guidelines |
| `src/tools/WebFetchTool/preapproved.ts` | 167 | `PREAPPROVED_HOSTS` Set (~80 entries: Anthropic, languages, frameworks, clouds); module-load split into `HOSTNAME_ONLY` and `PATH_PREFIXES`; `isPreapprovedHost(host, path)` with segment-boundary enforcement |
| `src/tools/WebFetchTool/UI.tsx` | 71 | Tool-use line, progress (`Fetching…`), result message (`Received <size> (<code> <text>)`), summary truncated to `TOOL_SUMMARY_MAX_LENGTH` |
| `src/tools/WebSearchTool/WebSearchTool.ts` | 436 | Tool definition; `makeToolSchema` → `BetaWebSearchTool20250305` with `max_uses: 8`; provider gate (`firstParty`, `vertex` Claude-4+, `foundry`); streaming consumption with `tengu_plum_vx3` Haiku flag; per-server-tool-use progress; `makeOutputFromSearchResponse` block merger; `mapToolResultToToolResultBlockParam` formatter with mandatory "Sources:" reminder |
| `src/tools/WebSearchTool/prompt.ts` | 34 | `WEB_SEARCH_TOOL_NAME = 'WebSearch'`; `getWebSearchPrompt()` injects `getLocalMonthYear()` |
| `src/tools/WebSearchTool/UI.tsx` | 100 | Verbose query/domain echo; progress dispatch on `query_update` / `search_results_received`; "Did N searches in Xs" summary |

### 2.2 Source coverage inventory

| Source | Read fully | Notes |
|---|---|---|
| `WebFetchTool/WebFetchTool.ts` | ✅ | All 319 lines |
| `WebFetchTool/utils.ts` | ✅ | All 531 lines |
| `WebFetchTool/prompt.ts` | ✅ | |
| `WebFetchTool/preapproved.ts` | ✅ | |
| `WebFetchTool/UI.tsx` | ✅ | |
| `WebSearchTool/WebSearchTool.ts` | ✅ | All 436 lines |
| `WebSearchTool/prompt.ts` | ✅ | |
| `WebSearchTool/UI.tsx` | ✅ | |
| `src/tools.ts:11,55,207,209` | grep | Static registry imports; no feature gate |
| `src/utils/messages.ts:367` | grep | `EMPTY_USAGE` carries `server_tool_use: { web_search_requests: 0, web_fetch_requests: 0 }` — confirms server-tool accounting |
| `src/cost-tracker.ts:271`, `src/utils/modelCost.ts:139` | grep | Server-tool counter is read here for cost computation (owned by 06/22) |
| `src/main.tsx:2509,4527,4550,4576` | grep | `skipWebFetchPreflight` plumbed from settings into runtime options |
| `src/utils/settings/types.ts:649` | grep | Schema for `skipWebFetchPreflight` (owned by 02) |

### 2.3 Imports from

WebFetchTool imports: `zod/v4`; `axios`; `lru-cache`; `../../Tool.js`; `../../types/permissions.js`; `../../utils/format.js`; `../../utils/lazySchema.js`; `../../utils/permissions/PermissionResult.js`; `../../utils/permissions/permissions.js` (`getRuleByContentsForTool`); `../../services/analytics/index.js` (`logEvent`, `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS`); `../../services/api/claude.js` (`queryHaiku`); `../../utils/errors.js` (`AbortError`); `../../utils/http.js` (`getWebFetchUserAgent`); `../../utils/log.js` (`logError`); `../../utils/mcpOutputStorage.js` (`isBinaryContentType`, `persistBinaryContent`); `../../utils/settings/settings.js` (`getSettings_DEPRECATED`); `../../utils/systemPromptType.js` (`asSystemPrompt`); dynamic `import('turndown')`.

WebSearchTool imports: `@anthropic-ai/sdk/resources/beta/messages/messages.mjs` types (`BetaContentBlock`, `BetaWebSearchTool20250305`); `zod/v4`; `../../Tool.js`; `../../services/analytics/growthbook.js` (`getFeatureValue_CACHED_MAY_BE_STALE`); `../../services/api/claude.js` (`queryModelWithStreaming`); `../../utils/lazySchema.js`; `../../utils/log.js`; `../../utils/messages.js` (`createUserMessage`); `../../utils/model/model.js` (`getMainLoopModel`, `getSmallFastModel`); `../../utils/model/providers.js` (`getAPIProvider`); `../../utils/slowOperations.js` (`jsonParse`, `jsonStringify`); `../../utils/systemPromptType.js`; `../../types/tools.js` (`WebSearchProgress`).

### 2.4 Imported by

Both tools are imported only by `src/tools.ts:11` and `src/tools.ts:55` and listed unconditionally in the base tool array (`tools.ts:207,209`). No feature flag, no ANT gate at registration. The `EMPTY_USAGE` entry at `src/utils/messages.ts:367` and the cost path at `src/cost-tracker.ts:271` consume `server_tool_use.web_search_requests` produced by the server-side execution of WebSearch.

---

## 3. Public Interface (Contract)

### 3.1 WebFetch

| Field | Value | Source |
|---|---|---|
| `name` | `'WebFetch'` | `prompt.ts:1` |
| `searchHint` | `'fetch and extract content from a URL'` | `WebFetchTool.ts:68` |
| `maxResultSizeChars` | `100_000` (persistence threshold) | `WebFetchTool.ts:70` |
| `shouldDefer` | `true` | `WebFetchTool.ts:71` |
| `userFacingName()` | `'Fetch'` | `WebFetchTool.ts:81-83` |
| `isConcurrencySafe()` | `true` | `WebFetchTool.ts:95-97` |
| `isReadOnly()` | `true` | `WebFetchTool.ts:98-100` |
| `toAutoClassifierInput(input)` | `${url}: ${prompt}` if prompt else `url` | `WebFetchTool.ts:101-103` |
| Permission rule content | `domain:<hostname>` (else `input:<input.toString()>`) | `WebFetchTool.ts:50-64` |
| Permission suggestions | `addRules → localSettings, behavior:'allow', toolName:'WebFetch', ruleContent:'domain:<host>'` | `WebFetchTool.ts:309-318` |

Input schema (`z.strictObject`): `{ url: string().url(), prompt: string() }`.

Output: `{ bytes:number; code:number; codeText:string; result:string; durationMs:number; url:string }`.

### 3.2 WebSearch

| Field | Value | Source |
|---|---|---|
| `name` | `'WebSearch'` | `prompt.ts:3` |
| `searchHint` | `'search the web for current information'` | `WebSearchTool.ts:154` |
| `maxResultSizeChars` | `100_000` | `WebSearchTool.ts:155` |
| `shouldDefer` | `true` | `WebSearchTool.ts:156` |
| `userFacingName()` | `'Web Search'` | `WebSearchTool.ts:160-162` |
| `isConcurrencySafe()` | `true` | `WebSearchTool.ts:200-202` |
| `isReadOnly()` | `true` | `WebSearchTool.ts:203-205` |
| `isEnabled()` | provider gate: `firstParty` ✅; `vertex` ✅ iff model contains `claude-opus-4`/`claude-sonnet-4`/`claude-haiku-4`; `foundry` ✅; else ❌ | `WebSearchTool.ts:168-193` |
| `extractSearchText()` | `''` (intentional no-op — UI shows only chrome) | `WebSearchTool.ts:229-234` |
| `toAutoClassifierInput(input)` | `input.query` | `WebSearchTool.ts:206-208` |
| Permission | `behavior:'passthrough'`; suggestion adds tool-wide allow rule (no rule content) | `WebSearchTool.ts:209-222` |

Input schema (`z.strictObject`): `{ query: string().min(2); allowed_domains?: string[]; blocked_domains?: string[] }`.

Output: `{ query:string; results:(SearchResult|string)[]; durationSeconds:number }` where `SearchResult = { tool_use_id:string; content:{ title:string; url:string }[] }`.

Progress (`WebSearchProgress`): `{ type:'query_update', query }` and `{ type:'search_results_received', resultCount, query }`. Source: `WebSearchTool.ts:344-355, 375-385`.

### 3.3 Validation

WebFetch (`WebFetchTool.ts:191-204`): rejects unparseable URL with `errorCode:1`, message `Error: Invalid URL "<url>". The URL provided could not be parsed.`, `meta.reason='invalid_url'`.

WebSearch (`WebSearchTool.ts:235-253`): rejects empty query (`errorCode:1`, `Error: Missing query`); rejects co-specified `allowed_domains` and `blocked_domains` (`errorCode:2`, `Error: Cannot specify both allowed_domains and blocked_domains in the same request`).

> **Note (BUGS-IN-SOURCE candidate, see §X / `BUGS-IN-SOURCE.md`):** the `!query.length` branch at `WebSearchTool.ts:237-242` is **unreachable**. The Zod schema enforces `z.string().min(2)` (`WebSearchTool.ts:27`), and Zod parsing runs *before* `validateInput`. Empty/single-char queries fail with the Zod-generated message ("Too small: expected string to have ≥2 characters"), never reaching this validator. The `Error: Missing query` string is dead code. Spec faithfully reproduces source but flags this discrepancy.

> **Cross-spec note (server enforcement):** WebSearch's `allowed_domains` / `blocked_domains` are **not** evaluated client-side. They are passed verbatim into the `web_search_20250305` server-tool schema (§6.15) and enforced by the Anthropic API. Unlike WebFetch's `domain:<host>` permission rules (which run through 09's local rule machinery), there is no client-side filter for WebSearch URLs — `behavior:'passthrough'` (§5.8, §11.24) means the network call lives entirely server-side and the client never sees individual result URLs at decision time.

---

## 4. Data Model & State

### 4.1 Caches (process-global, WebFetch only)

| Cache | Key | Value | Capacity | TTL | Source |
|---|---|---|---|---|---|
| `URL_CACHE` | original `url` (pre-upgrade, pre-redirect) | `CacheEntry` | `maxSize: 50 * 1024 * 1024` bytes (LRU sized by `contentBytes`, clamped `Math.max(1, contentBytes)`) | `15 * 60 * 1000` ms (15min) | `utils.ts:63-69, 480` |
| `DOMAIN_CHECK_CACHE` | hostname | `true` (only `'allowed'` cached) | `max: 128` | `5 * 60 * 1000` ms (5min, deliberately shorter than URL TTL) | `utils.ts:75-78` |

`CacheEntry` shape (`utils.ts:51-59`): `{ bytes, code, codeText, content, contentType, persistedPath?, persistedSize? }`.

`clearWebFetchCache()` (`utils.ts:80-83`) clears both caches.

### 4.2 Turndown lazy singleton

`turndownServicePromise` (`utils.ts:90-97`) is a module-scoped `Promise<Turndown>` initialized on first HTML decode. `import('turndown')` is awaited then `new Turndown()` constructed once. Comment-cited reasons: defers a ~1.4MB `@mixmark-io/domino` import; the 15 turndown-rule constructors run once; `.turndown()` is stateless.

### 4.3 Settings

`skipWebFetchPreflight` (boolean, read via `getSettings_DEPRECATED()` at `utils.ts:386-387`): when truthy, skips the `checkDomainBlocklist` round-trip entirely. Intended for enterprise environments that block outbound `api.anthropic.com`. Schema owner: 02 (`src/utils/settings/types.ts:649`).

### 4.4 WebSearch state machine state

Per-call locals in `call()` (`WebSearchTool.ts:293-298`):
- `allContentBlocks: BetaContentBlock[]` — accumulator for final `makeOutputFromSearchResponse`.
- `currentToolUseId: string|null` — id of the in-flight `server_tool_use` block.
- `currentToolUseJson: string` — JSON delta accumulator; regex-scanned for `"query":"…"` to emit progress before the block completes.
- `progressCounter: number`.
- `toolUseQueries: Map<id,string>`.

---

## 5. Algorithm / Control Flow

### 5.1 WebFetch — `call({ url, prompt }, ctx)` (pseudocode)

```
start = Date.now()
response = await getURLMarkdownContent(url, abortController)
if response.type === 'redirect':
    statusText = { 301:'Moved Permanently', 308:'Permanent Redirect',
                   307:'Temporary Redirect', else:'Found' }[response.statusCode]
    msg = synth_redirect_message(response, prompt)   // §6.5
    return { bytes: byteLength(msg), code, codeText: statusText, result: msg, durationMs, url }

{ content, bytes, code, codeText, contentType, persistedPath, persistedSize } = response
isPreapproved = isPreapprovedUrl(url)
if isPreapproved && contentType.includes('text/markdown') && content.length < MAX_MARKDOWN_LENGTH:
    result = content                                 // bypass Haiku for trusted markdown ≤ 100k
else:
    result = await applyPromptToMarkdown(prompt, content, signal, isNonInteractive, isPreapproved)
if persistedPath:
    result += `\n\n[Binary content (${contentType}, ${formatFileSize(persistedSize ?? bytes)}) also saved to ${persistedPath}]`
return { bytes, code, codeText, result, durationMs: Date.now()-start, url }
```
Source: `WebFetchTool.ts:208-298`.

### 5.2 `getURLMarkdownContent(url, abortController)` (pseudocode)

```
if !validateURL(url): throw Error('Invalid URL')
if URL_CACHE.has(url): return cached entry as FetchedContent     // shape preserved exactly

parsed = new URL(url)
if parsed.protocol === 'http:': parsed.protocol = 'https:'; upgradedUrl = parsed.toString()

try:
    if !settings.skipWebFetchPreflight:
        switch await checkDomainBlocklist(parsed.hostname):
            case 'allowed':       break
            case 'blocked':       throw new DomainBlockedError(host)
            case 'check_failed':  throw new DomainCheckFailedError(host)
    if process.env.USER_TYPE === 'ant':
        logEvent('tengu_web_fetch_host', { hostname })           // ANT-ONLY
catch e:
    if e instanceof DomainBlockedError|DomainCheckFailedError: throw  // user-facing, no logError
    logError(e)                                                       // others swallowed

response = await getWithPermittedRedirects(upgradedUrl, signal, isPermittedRedirect)
if isRedirectInfo(response): return response                          // bubble redirect to caller

rawBuffer = Buffer.from(response.data)
response.data = null                                                  // free axios ArrayBuffer copy
contentType = response.headers['content-type'] ?? ''

if isBinaryContentType(contentType):
    persistId = `webfetch-${Date.now()}-${random base36 [2..8)}`
    r = await persistBinaryContent(rawBuffer, contentType, persistId)
    if !('error' in r): persistedPath=r.filepath; persistedSize=r.size
                                                                      // fall through to UTF-8 decode
bytes = rawBuffer.length
htmlContent = rawBuffer.toString('utf-8')

if contentType.includes('text/html'):
    markdownContent = (await getTurndownService()).turndown(htmlContent)
    contentBytes    = Buffer.byteLength(markdownContent)
else:
    markdownContent = htmlContent
    contentBytes    = bytes                                           // skip O(n) re-scan

entry = { bytes, code: response.status, codeText: response.statusText,
          content: markdownContent, contentType, persistedPath, persistedSize }
URL_CACHE.set(url, entry, { size: Math.max(1, contentBytes) })        // key = ORIGINAL url
return entry
```
Source: `utils.ts:347-482`. Note: cache is keyed under the **pre-upgrade pre-redirect** URL (comment, `utils.ts:469`).

### 5.3 `checkDomainBlocklist(domain)` (pseudocode)

```
if DOMAIN_CHECK_CACHE.has(domain): return { status:'allowed' }
try:
    r = await axios.get(`https://api.anthropic.com/api/web/domain_info?domain=${encodeURIComponent(domain)}`,
                        { timeout: DOMAIN_CHECK_TIMEOUT_MS /* 10_000 */ })
    if r.status === 200:
        if r.data.can_fetch === true:
            DOMAIN_CHECK_CACHE.set(domain, true)
            return { status:'allowed' }
        return { status:'blocked' }
    return { status:'check_failed', error: Error(`Domain check returned status ${r.status}`) }
catch e:
    logError(e); return { status:'check_failed', error: e }
```
Source: `utils.ts:176-203`. Only `allowed` is cached; `blocked` and `check_failed` re-check next attempt.

### 5.4 `getWithPermittedRedirects(url, signal, redirectChecker, depth=0)` (pseudocode)

```
if depth > MAX_REDIRECTS /* 10 */: throw Error(`Too many redirects (exceeded ${MAX_REDIRECTS})`)
try:
    return await axios.get(url, {
        signal, timeout: FETCH_TIMEOUT_MS /* 60_000 */,
        maxRedirects: 0,                                       // we drive redirects ourselves
        responseType: 'arraybuffer',
        maxContentLength: MAX_HTTP_CONTENT_LENGTH /* 10MB */,
        headers: { Accept: 'text/markdown, text/html, */*', 'User-Agent': getWebFetchUserAgent() }
    })
catch error:
    if axios.isAxiosError(error) && error.response.status in {301,302,307,308}:
        loc = error.response.headers.location
        if !loc: throw Error('Redirect missing Location header')
        redirectUrl = new URL(loc, url).toString()             // resolve relative
        if redirectChecker(url, redirectUrl):
            return getWithPermittedRedirects(redirectUrl, signal, redirectChecker, depth+1)
        return { type:'redirect', originalUrl:url, redirectUrl, statusCode: error.response.status }
    if axios.isAxiosError(error) && error.response.status === 403
       && error.response.headers['x-proxy-error'] === 'blocked-by-allowlist':
        throw new EgressBlockedError(new URL(url).hostname)
    throw error
```
Source: `utils.ts:262-329`. PSR cited inline (`utils.ts:249-254`): never auto-follow cross-host redirects.

### 5.5 `isPermittedRedirect(originalUrl, redirectUrl)`

Returns `true` iff: same protocol, same port, no creds on redirect, and `stripWww(host)` matches between original and redirect (where `stripWww = h => h.replace(/^www\./,'')`). Source: `utils.ts:212-243`.

### 5.6 `validateURL(url)`

`true` iff: `url.length ≤ MAX_URL_LENGTH (2000)`; `new URL(url)` succeeds; no `username` or `password`; hostname has at least 2 dot-separated parts. Source: `utils.ts:139-169`.

### 5.7 `applyPromptToMarkdown(prompt, content, signal, isNonInteractive, isPreapproved)`

```
truncated = content.length > MAX_MARKDOWN_LENGTH /* 100_000 */
            ? content.slice(0, MAX_MARKDOWN_LENGTH) + '\n\n[Content truncated due to length...]'
            : content
modelPrompt = makeSecondaryModelPrompt(truncated, prompt, isPreapproved)
asst = await queryHaiku({ systemPrompt: asSystemPrompt([]), userPrompt: modelPrompt, signal,
        options: { querySource:'web_fetch_apply', agents:[], isNonInteractiveSession:isNonInteractive,
                   hasAppendSystemPrompt:false, mcpTools:[] } })
if signal.aborted: throw new AbortError()                  // forces is_error tool_use → red dot
content = asst.message.content
if content.length > 0 && 'text' in content[0]: return content[0].text
return 'No response from model'
```
Source: `utils.ts:484-530`. Truncation indicator is appended **inside** the secondary prompt, not in the tool result.

### 5.8 `checkPermissions` (WebFetch only)

```
1. parse URL; if isPreapprovedHost(host, path): return { behavior:'allow', updatedInput, decisionReason:{type:'other', reason:'Preapproved host'} }
2. ruleContent = `domain:${host}` (or fallback `input:<input.toString()>` on parse failure)
3. lookup deny rule → return { behavior:'deny', message:`WebFetch denied access to ${ruleContent}.`, decisionReason:{type:'rule', rule:denyRule} }
4. lookup ask rule → return { behavior:'ask', message:`Claude requested permissions to use WebFetch, but you haven't granted it yet.`, decisionReason:{type:'rule', rule:askRule}, suggestions: addRules(allow, localSettings, [{toolName:'WebFetch', ruleContent}]) }
5. lookup allow rule → return { behavior:'allow', updatedInput, decisionReason:{type:'rule', rule:allowRule} }
6. default: return { behavior:'ask', message:..., suggestions: ... }
```
Source: `WebFetchTool.ts:104-180`. See 09 for full decision-tree semantics.

`checkPermissions` for WebSearch is `behavior:'passthrough'` with a tool-wide allow suggestion — the network call happens server-side (08/09 own the passthrough semantics).

### 5.9 WebSearch — `call(input, ctx)` (pseudocode)

```
startTime = performance.now()
userMessage = createUserMessage({ content: 'Perform a web search for the query: ' + query })
toolSchema   = makeToolSchema(input)                        // { type:'web_search_20250305', name:'web_search', allowed_domains, blocked_domains, max_uses: 8 }
useHaiku     = getFeatureValue_CACHED_MAY_BE_STALE('tengu_plum_vx3', false)

stream = queryModelWithStreaming({
  messages: [userMessage],
  systemPrompt: asSystemPrompt(['You are an assistant for performing a web search tool use']),
  thinkingConfig: useHaiku ? {type:'disabled'} : ctx.options.thinkingConfig,
  tools: [],
  signal: ctx.abortController.signal,
  options: {
    getToolPermissionContext: async () => appState.toolPermissionContext,
    model:       useHaiku ? getSmallFastModel() : ctx.options.mainLoopModel,
    toolChoice:  useHaiku ? { type:'tool', name:'web_search' } : undefined,
    isNonInteractiveSession, hasAppendSystemPrompt: !!ctx.options.appendSystemPrompt,
    extraToolSchemas: [toolSchema],
    querySource: 'web_search_tool',
    agents:      ctx.options.agentDefinitions.activeAgents,
    mcpTools:    [],
    agentId:     ctx.agentId,
    effortValue: appState.effortValue,
  },
})

for await event in stream:
  if event.type === 'assistant': allContentBlocks.push(...event.message.content); continue

  // server_tool_use start
  if event.type === 'stream_event' && event.event.type === 'content_block_start'
     && event.event.content_block.type === 'server_tool_use':
        currentToolUseId = block.id; currentToolUseJson = ''; continue

  // accumulate JSON deltas, attempt to extract "query" early
  if currentToolUseId && event.event.type === 'content_block_delta'
     && delta.type === 'input_json_delta' && delta.partial_json:
        currentToolUseJson += delta.partial_json
        m = currentToolUseJson.match(/"query"\s*:\s*"((?:[^"\\]|\\.)*)"/)
        if m && m[1]:
            q = jsonParse('"' + m[1] + '"')
            if toolUseQueries.get(currentToolUseId) !== q:
                toolUseQueries.set(currentToolUseId, q); progressCounter++
                onProgress({ toolUseID: `search-progress-${progressCounter}`,
                             data: { type:'query_update', query:q } })

  // results received
  if event.event.type === 'content_block_start'
     && event.event.content_block.type === 'web_search_tool_result':
        toolUseId = block.tool_use_id; actualQuery = toolUseQueries.get(toolUseId) || query
        progressCounter++
        onProgress({ toolUseID: toolUseId || `search-progress-${progressCounter}`,
                     data: { type:'search_results_received',
                             resultCount: Array.isArray(block.content) ? block.content.length : 0,
                             query: actualQuery } })

durationSeconds = (performance.now()-startTime)/1000
return { data: makeOutputFromSearchResponse(allContentBlocks, query, durationSeconds) }
```
Source: `WebSearchTool.ts:254-399`. The `extraToolSchemas` mechanism is owned by 22.

### 5.10 `makeOutputFromSearchResponse(blocks, query, durationSeconds)` block-merge state machine

States: `inText: bool` (start `true`); `textAcc: string`. For each block:
- `server_tool_use`: if `inText`: flush `textAcc.trim()` if non-empty into `results`; reset; `inText=false`. Continue (skip block).
- `web_search_tool_result`:
  - if `!Array.isArray(block.content)` (error envelope): `errorMessage = 'Web search error: ' + block.content.error_code'`; `logError(...)`; `results.push(errorMessage)`; continue.
  - else: `results.push({ tool_use_id: block.tool_use_id, content: block.content.map(r=>({title:r.title, url:r.url})) })`.
- `text`:
  - if `inText`: `textAcc += block.text`.
  - else: `inText=true; textAcc = block.text` (replace, not concatenate).

After loop: `if textAcc.length: results.push(textAcc.trim())`.

Returns `{ query, results, durationSeconds }`.

Source: `WebSearchTool.ts:86-150`.

### 5.11 `mapToolResultToToolResultBlockParam` (WebSearch)

```
out = `Web search results for query: "${query}"\n\n`
for r in (results ?? []):
    if r == null: continue
    if typeof r === 'string': out += r + '\n\n'                   // text summary
    else:
        if r.content?.length > 0: out += `Links: ${jsonStringify(r.content)}\n\n`
        else:                     out += 'No links found.\n\n'
out += '\nREMINDER: You MUST include the sources above in your response to the user using markdown hyperlinks.'
return { tool_use_id, type:'tool_result', content: out.trim() }
```
Source: `WebSearchTool.ts:401-434`.

### 5.12 `isPreapprovedHost(host, path)` (preapproved.ts)

Module load-time split (`preapproved.ts:136-152`): for each entry in `PREAPPROVED_HOSTS`, if it has no `/`, push to `HOSTNAME_ONLY: Set<string>`; otherwise split at first `/` and push `path` into `PATH_PREFIXES: Map<host, string[]>`.

Lookup: `HOSTNAME_ONLY.has(host) ⇒ true`; else for each prefix `p` registered for `host`: match iff `pathname === p` OR `pathname.startsWith(p + '/')` (segment-boundary enforcement). Source: `preapproved.ts:154-166`. The latter blocks `/anthropics-evil/...` from matching `/anthropics`.

---

## 6. Verbatim Assets

### 6.1 WebFetch input schema

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    url: z.string().url().describe('The URL to fetch content from'),
    prompt: z.string().describe('The prompt to run on the fetched content'),
  }),
)
```
Source: `WebFetchTool.ts:24-29`.

### 6.2 WebFetch output schema

```ts
const outputSchema = lazySchema(() =>
  z.object({
    bytes: z.number().describe('Size of the fetched content in bytes'),
    code: z.number().describe('HTTP response code'),
    codeText: z.string().describe('HTTP response code text'),
    result: z
      .string()
      .describe('Processed result from applying the prompt to the content'),
    durationMs: z
      .number()
      .describe('Time taken to fetch and process the content'),
    url: z.string().describe('The URL that was fetched'),
  }),
)
```
Source: `WebFetchTool.ts:32-45`.

### 6.3 WebSearch input schema

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    query: z.string().min(2).describe('The search query to use'),
    allowed_domains: z
      .array(z.string())
      .optional()
      .describe('Only include search results from these domains'),
    blocked_domains: z
      .array(z.string())
      .optional()
      .describe('Never include search results from these domains'),
  }),
)
```
Source: `WebSearchTool.ts:25-37`.

### 6.4 WebSearch output schema

```ts
const searchHitSchema = z.object({
  title: z.string().describe('The title of the search result'),
  url: z.string().describe('The URL of the search result'),
})
return z.object({
  tool_use_id: z.string().describe('ID of the tool use'),
  content: z.array(searchHitSchema).describe('Array of search hits'),
})

const outputSchema = lazySchema(() =>
  z.object({
    query: z.string().describe('The search query that was executed'),
    results: z
      .array(z.union([searchResultSchema(), z.string()]))
      .describe('Search results and/or text commentary from the model'),
    durationSeconds: z
      .number()
      .describe('Time taken to complete the search operation'),
  }),
)
```
Source: `WebSearchTool.ts:42-66`.

### 6.5 WebFetch system-prompt prefix and DESCRIPTION (verbatim)

The `prompt()` member always emits this prefix joined to `DESCRIPTION` (`WebFetchTool.ts:181-190`; comment cites prompt-cache flicker reasoning):

```
IMPORTANT: WebFetch WILL FAIL for authenticated or private URLs. Before using this tool, check if the URL points to an authenticated service (e.g. Google Docs, Confluence, Jira, GitHub). If so, look for a specialized MCP tool that provides authenticated access.
```

`DESCRIPTION` body verbatim (`prompt.ts:3-21`):

```
- Fetches content from a specified URL and processes it using an AI model
- Takes a URL and a prompt as input
- Fetches the URL content, converts HTML to markdown
- Processes the content with the prompt using a small, fast model
- Returns the model's response about the content
- Use this tool when you need to retrieve and analyze web content

Usage notes:
  - IMPORTANT: If an MCP-provided web fetch tool is available, prefer using that tool instead of this one, as it may have fewer restrictions.
  - The URL must be a fully-formed valid URL
  - HTTP URLs will be automatically upgraded to HTTPS
  - The prompt should describe what information you want to extract from the page
  - This tool is read-only and does not modify any files
  - Results may be summarized if the content is very large
  - Includes a self-cleaning 15-minute cache for faster responses when repeatedly accessing the same URL
  - When a URL redirects to a different host, the tool will inform you and provide the redirect URL in a special format. You should then make a new WebFetch request with the redirect URL to fetch the content.
  - For GitHub URLs, prefer using the gh CLI via Bash instead (e.g., gh pr view, gh issue view, gh api).
```

### 6.6 WebFetch secondary-model prompt (verbatim)

```ts
export function makeSecondaryModelPrompt(
  markdownContent: string,
  prompt: string,
  isPreapprovedDomain: boolean,
): string {
  const guidelines = isPreapprovedDomain
    ? `Provide a concise response based on the content above. Include relevant details, code examples, and documentation excerpts as needed.`
    : `Provide a concise response based only on the content above. In your response:
 - Enforce a strict 125-character maximum for quotes from any source document. Open Source Software is ok as long as we respect the license.
 - Use quotation marks for exact language from articles; any language outside of the quotation should never be word-for-word the same.
 - You are not a lawyer and never comment on the legality of your own prompts and responses.
 - Never produce or reproduce exact song lyrics.`

  return `
Web page content:
---
${markdownContent}
---

${prompt}

${guidelines}
`
}
```
Source: `prompt.ts:23-46`.

### 6.7 WebFetch redirect-message template (verbatim)

```
REDIRECT DETECTED: The URL redirects to a different host.

Original URL: ${response.originalUrl}
Redirect URL: ${response.redirectUrl}
Status: ${response.statusCode} ${statusText}

To complete your request, I need to fetch content from the redirected URL. Please use WebFetch again with these parameters:
- url: "${response.redirectUrl}"
- prompt: "${prompt}"
```
`statusText` mapping: `301:'Moved Permanently'`, `308:'Permanent Redirect'`, `307:'Temporary Redirect'`, else `'Found'`. Source: `WebFetchTool.ts:217-235`.

Binary-content append (`WebFetchTool.ts:283-285`):
```
\n\n[Binary content (${contentType}, ${formatFileSize(persistedSize ?? bytes)}) also saved to ${persistedPath}]
```

Truncation marker inside Haiku prompt (`utils.ts:493-496`):
```
\n\n[Content truncated due to length...]
```

### 6.8 WebSearch system prompt (verbatim, with `${currentMonthYear}` substitution)

```
- Allows Claude to search the web and use the results to inform responses
- Provides up-to-date information for current events and recent data
- Returns search result information formatted as search result blocks, including links as markdown hyperlinks
- Use this tool for accessing information beyond Claude's knowledge cutoff
- Searches are performed automatically within a single API call

CRITICAL REQUIREMENT - You MUST follow this:
  - After answering the user's question, you MUST include a "Sources:" section at the end of your response
  - In the Sources section, list all relevant URLs from the search results as markdown hyperlinks: [Title](URL)
  - This is MANDATORY - never skip including sources in your response
  - Example format:

    [Your answer here]

    Sources:
    - [Source Title 1](https://example.com/1)
    - [Source Title 2](https://example.com/2)

Usage notes:
  - Domain filtering is supported to include or block specific websites
  - Web search is only available in the US

IMPORTANT - Use the correct year in search queries:
  - The current month is ${currentMonthYear}. You MUST use this year when searching for recent information, documentation, or current events.
  - Example: If the user asks for "latest React docs", search for "React documentation" with the current year, NOT last year
```
Source: `WebSearchTool/prompt.ts:7-33`.

### 6.9 WebSearch result formatter trailer (verbatim)

```
\nREMINDER: You MUST include the sources above in your response to the user using markdown hyperlinks.
```
Source: `WebSearchTool.ts:426-427`.

### 6.10 HTML→markdown extraction algorithm (pseudocode, faithful to source)

```
// Lazy singleton (utils.ts:90-97)
turndownServicePromise ??= import('turndown').then(m => new ((m as { default: TurndownCtor }).default)())

// In getURLMarkdownContent (utils.ts:454-466):
if contentType.includes('text/html'):
    markdownContent = (await turndownServicePromise).turndown(htmlContent)
    contentBytes    = Buffer.byteLength(markdownContent)
else:
    markdownContent = htmlContent              // raw bytes' decoded text
    contentBytes    = bytes                    // skip O(n) re-scan; comment cites U+FFFD as negligible
```

Truncation logic for the secondary-model prompt (`utils.ts:491-496`):
```
truncatedContent = markdownContent.length > MAX_MARKDOWN_LENGTH
    ? markdownContent.slice(0, MAX_MARKDOWN_LENGTH) + '\n\n[Content truncated due to length...]'
    : markdownContent
```

### 6.11 Cache key + TTL constants (verbatim)

```ts
const CACHE_TTL_MS = 15 * 60 * 1000        // 15 minutes
const MAX_CACHE_SIZE_BYTES = 50 * 1024 * 1024  // 50MB
const URL_CACHE = new LRUCache<string, CacheEntry>({
  maxSize: MAX_CACHE_SIZE_BYTES,
  ttl: CACHE_TTL_MS,
})
const DOMAIN_CHECK_CACHE = new LRUCache<string, true>({
  max: 128,
  ttl: 5 * 60 * 1000, // 5 minutes — shorter than URL_CACHE TTL
})
```
Source: `utils.ts:63-78`. URL key = original (pre-upgrade, pre-redirect) string. Domain key = hostname.

### 6.12 User-facing error strings (verbatim)

| Error | String | Source |
|---|---|---|
| `DomainBlockedError` | `Claude Code is unable to fetch from ${domain}` | `utils.ts:21-26` |
| `DomainCheckFailedError` | `Unable to verify if domain ${domain} is safe to fetch. This may be due to network restrictions or enterprise security policies blocking claude.ai.` | `utils.ts:28-35` |
| `EgressBlockedError.message` | JSON: `{"error_type":"EGRESS_BLOCKED","domain":"…","message":"Access to ${domain} is blocked by the network egress proxy."}` | `utils.ts:37-48` |
| Too-many-redirects | `Too many redirects (exceeded ${MAX_REDIRECTS})` (=10) | `utils.ts:268-269` |
| Missing redirect Location | `Redirect missing Location header` | `utils.ts:290-291` |
| Validation: invalid URL | `Error: Invalid URL "${url}". The URL provided could not be parsed.` (errorCode 1, `meta.reason='invalid_url'`) | `WebFetchTool.ts:196-201` |
| Validation: missing query (WebSearch) | `Error: Missing query` (errorCode 1) | `WebSearchTool.ts:238-242` |
| Validation: domain conflict (WebSearch) | `Error: Cannot specify both allowed_domains and blocked_domains in the same request` (errorCode 2) | `WebSearchTool.ts:244-250` |
| Permission deny (WebFetch) | `${WebFetchTool.name} denied access to ${ruleContent}.` | `WebFetchTool.ts:131-140` |
| Permission ask / default (WebFetch) | `Claude requested permissions to use ${WebFetchTool.name}, but you haven't granted it yet.` | `WebFetchTool.ts:147-179` |
| Permission passthrough (WebSearch) | `WebSearchTool requires permission.` | `WebSearchTool.ts:209-222` |
| Internal: invalid URL (utils) | `Invalid URL` (thrown when `validateURL` returns false) | `utils.ts:351-353` |
| Internal: domain check non-200 | `Domain check returned status ${r.status}` | `utils.ts:194-198` |
| Web search server error | `Web search error: ${block.content.error_code}` | `WebSearchTool.ts:117-121` |
| Empty Haiku response | `No response from model` | `utils.ts:529` |

### 6.13 Constants table

| Constant | Value | Source |
|---|---|---|
| `MAX_URL_LENGTH` | `2000` | `utils.ts:106` |
| `MAX_HTTP_CONTENT_LENGTH` | `10 * 1024 * 1024` (10MB) | `utils.ts:112` |
| `FETCH_TIMEOUT_MS` | `60_000` (60s) | `utils.ts:116` |
| `DOMAIN_CHECK_TIMEOUT_MS` | `10_000` (10s) | `utils.ts:119` |
| `MAX_REDIRECTS` | `10` | `utils.ts:125` |
| `MAX_MARKDOWN_LENGTH` | `100_000` | `utils.ts:128` |
| `CACHE_TTL_MS` | `15 * 60 * 1000` | `utils.ts:63` |
| `MAX_CACHE_SIZE_BYTES` | `50 * 1024 * 1024` | `utils.ts:64` |
| `DOMAIN_CHECK_CACHE.max` | `128` | `utils.ts:76` |
| `DOMAIN_CHECK_CACHE.ttl` | `5 * 60 * 1000` | `utils.ts:77` |
| WebFetch `maxResultSizeChars` | `100_000` | `WebFetchTool.ts:70` |
| WebSearch `maxResultSizeChars` | `100_000` | `WebSearchTool.ts:155` |
| WebSearch server-tool `max_uses` | `8` | `WebSearchTool.ts:82` |
| WebSearch input min query length | `2` | `WebSearchTool.ts:27` |
| Server-tool name (WebSearch) | `'web_search_20250305'` (`name:'web_search'`) | `WebSearchTool.ts:78-83` |
| WebFetch axios `Accept` header | `text/markdown, text/html, */*` | `utils.ts:279` |
| Egress-block proxy header trigger | `x-proxy-error: blocked-by-allowlist` on HTTP 403 | `utils.ts:317-322` |
| WebSearch `extractSearchText()` | `''` (intentional) | `WebSearchTool.ts:229-234` |
| Haiku flag for WebSearch | `tengu_plum_vx3` (default `false`) | `WebSearchTool.ts:262-265` |
| Haiku `querySource` (WebFetch) | `'web_fetch_apply'` | `utils.ts:507` |
| `querySource` (WebSearch) | `'web_search_tool'` | `WebSearchTool.ts:285` |
| Domain blocklist endpoint | `https://api.anthropic.com/api/web/domain_info?domain=<host>` | `utils.ts:184` |

### 6.14 Allowed-domain rules (preapproved-host policy)

Match policy: case-sensitive Set lookup (`HOSTNAME_ONLY`) OR exact-or-segment-prefix match (`PATH_PREFIXES`). The Set has ~80 entries with one path-scoped entry: `github.com/anthropics`. Verbatim list source: `preapproved.ts:14-131`. The full Set is reproduced here for completeness:

```
Anthropic: platform.claude.com, code.claude.com, modelcontextprotocol.io,
           github.com/anthropics, agentskills.io
Languages: docs.python.org, en.cppreference.com, docs.oracle.com,
           learn.microsoft.com, developer.mozilla.org, go.dev, pkg.go.dev,
           www.php.net, docs.swift.org, kotlinlang.org, ruby-doc.org,
           doc.rust-lang.org, www.typescriptlang.org
Web/JS:    react.dev, angular.io, vuejs.org, nextjs.org, expressjs.com,
           nodejs.org, bun.sh, jquery.com, getbootstrap.com, tailwindcss.com,
           d3js.org, threejs.org, redux.js.org, webpack.js.org, jestjs.io,
           reactrouter.com
Python:    docs.djangoproject.com, flask.palletsprojects.com,
           fastapi.tiangolo.com, pandas.pydata.org, numpy.org,
           www.tensorflow.org, pytorch.org, scikit-learn.org,
           matplotlib.org, requests.readthedocs.io, jupyter.org
PHP:       laravel.com, symfony.com, wordpress.org
Java:      docs.spring.io, hibernate.org, tomcat.apache.org, gradle.org,
           maven.apache.org
.NET/C#:   asp.net, dotnet.microsoft.com, nuget.org, blazor.net
Mobile:    reactnative.dev, docs.flutter.dev, developer.apple.com,
           developer.android.com
ML:        keras.io, spark.apache.org, huggingface.co, www.kaggle.com
DBs:       www.mongodb.com, redis.io, www.postgresql.org, dev.mysql.com,
           www.sqlite.org, graphql.org, prisma.io
Cloud:     docs.aws.amazon.com, cloud.google.com, learn.microsoft.com,
           kubernetes.io, www.docker.com, www.terraform.io,
           www.ansible.com, vercel.com/docs, docs.netlify.com,
           devcenter.heroku.com
Test:      cypress.io, selenium.dev
Game:      docs.unity.com, docs.unrealengine.com
Tools:     git-scm.com, nginx.org, httpd.apache.org
```
(`learn.microsoft.com` appears twice in the source Set; Set semantics deduplicate.)

### 6.15 WebSearch tool schema (server-tool wiring)

```ts
function makeToolSchema(input: Input): BetaWebSearchTool20250305 {
  return {
    type: 'web_search_20250305',
    name: 'web_search',
    allowed_domains: input.allowed_domains,
    blocked_domains: input.blocked_domains,
    max_uses: 8, // Hardcoded to 8 searches maximum
  }
}
```
Source: `WebSearchTool.ts:76-84`. Schema is passed via `extraToolSchemas` to `queryModelWithStreaming`. Server-side execution accounting via `usage.server_tool_use.web_search_requests` is owned by 22 / 06.

---

## 7. Side Effects & I/O

| Effect | Tool | Notes |
|---|---|---|
| Outbound HTTPS GET to arbitrary host (axios, arraybuffer) | WebFetch | Driven by `getWithPermittedRedirects`; `maxRedirects:0` in axios; redirects re-entered manually |
| Outbound HTTPS GET to `api.anthropic.com/api/web/domain_info` | WebFetch | Skippable via `skipWebFetchPreflight` setting |
| Disk write of binary content via `persistBinaryContent(rawBuffer, contentType, persistId)` | WebFetch | Path returned via `persistedPath`; appended to result string |
| Anthropic API call to Haiku via `queryHaiku` (`querySource:'web_fetch_apply'`) | WebFetch | Always issued unless preapproved+markdown+≤100k fast path |
| Anthropic API call via `queryModelWithStreaming` (`querySource:'web_search_tool'`) | WebSearch | Either main-loop model or small/fast model (per `tengu_plum_vx3`); the API performs the actual web search server-side |
| Process-global LRU caches | WebFetch | `URL_CACHE`, `DOMAIN_CHECK_CACHE`; cleared by `clearWebFetchCache()` |
| `logEvent('tengu_web_fetch_host', { hostname })` | WebFetch (ANT-only) | `process.env.USER_TYPE === 'ant'`; `utils.ts:400-405` |
| `logError(...)` | both | On preflight non-200, redirect parse failures, server-side web_search errors, generic catches |

---

## 8. Feature Flags & Variants

| Gate | Value | Effect | Source |
|---|---|---|---|
| `process.env.USER_TYPE === 'ant'` | runtime env | Emits `tengu_web_fetch_host` analytics on each fetched hostname; **no** ANT-only behavioral delta in fetch logic itself | `utils.ts:400-405` |
| `getSettings_DEPRECATED().skipWebFetchPreflight` | settings boolean | Skip `checkDomainBlocklist` round-trip; preserves `DomainBlockedError` semantics by leaving them never to fire | `utils.ts:386-387` |
| `getFeatureValue_CACHED_MAY_BE_STALE('tengu_plum_vx3', false)` | GrowthBook flag | When `true`: WebSearch uses small/fast model with `thinkingConfig:{type:'disabled'}` and `toolChoice:{type:'tool', name:'web_search'}` | `WebSearchTool.ts:262-281` |
| `getAPIProvider()` | runtime | WebSearch enable: `firstParty` always; `vertex` iff model name contains `claude-opus-4`/`claude-sonnet-4`/`claude-haiku-4`; `foundry` always; else disabled | `WebSearchTool.ts:168-193` |
| `bun:bundle feature(...)` | build-time | **None** for either tool. Both are statically registered (`tools.ts:11,55,207,209`). | `src/tools.ts` |
| `verbose` UI flag | runtime | WebFetch: shows `prompt` and full `result` body; WebSearch: appends domain echo `, only allowing/blocking domains: …` | `WebFetchTool/UI.tsx:24-26, 45-55`; `WebSearchTool/UI.tsx:45-52` |

---

## 9. Error Handling & Edge Cases

- **Invalid URL (validateURL false)**: `getURLMarkdownContent` throws `Error('Invalid URL')`. Validation step earlier returns errorCode-1 to the registry before `call()` even runs. (`utils.ts:351-353`, `WebFetchTool.ts:191-204`).
- **HTTP→HTTPS upgrade**: protocol upgrade is silent; the cache key still uses the **pre-upgrade** url (`utils.ts:376-379, 469`). The same caveat extends to **all** same-origin redirect normalizations permitted by `isPermittedRedirect` (§5.5): a request that arrives via a `www.example.com` → `example.com` (or vice-versa) redirect chain caches under the *original* URL only — a subsequent direct hit to the post-strip target URL misses cache and re-fetches. There is no second `URL_CACHE.set` under the upgraded or final-redirect URL anywhere in `getURLMarkdownContent`.
- **Cross-host redirect**: returned as a `RedirectInfo` to caller and surfaced as a non-error tool result containing the user-facing redirect template (§6.7); statusCode in the output object reflects the 3xx.
- **Same-host redirect**: recursively followed up to `MAX_REDIRECTS=10`; `FETCH_TIMEOUT_MS=60s` resets per hop (comment `utils.ts:121-125`).
- **Egress proxy block**: HTTP 403 + `X-Proxy-Error: blocked-by-allowlist` ⇒ `EgressBlockedError`. (`utils.ts:317-324`).
- **Domain blocklist failures**: `DomainBlockedError` and `DomainCheckFailedError` re-thrown by the outer `getURLMarkdownContent` catch without an additional `logError` (`utils.ts:407-413`). **Correction:** the underlying axios/network error is nevertheless logged once *inside* `checkDomainBlocklist` itself at `utils.ts:200` (`logError(e); return { status:'check_failed', error: e }`) before the wrapper synthesizes `DomainCheckFailedError`. So a domain-check failure DOES emit one `logError` per attempt — the wrapper just doesn't double-log. The "expected user-facing failure" comment at `utils.ts:411` describes the wrapper-level behavior, not the inner axios catch. **BUGS-IN-SOURCE candidate (§X):** this means transient network blips on `api.anthropic.com/api/web/domain_info` pollute telemetry once per fetch attempt even though the user-facing error is "expected".
- **HTTP 4xx/5xx other than 3xx/403-egress**: re-thrown raw (`utils.ts:327`).
- **Binary content + decode**: PDFs are persisted to disk **and** passed through utf-8 decode → Haiku, because the decoded ASCII PDF dictionary keys give Haiku enough structure for a summary (comment `utils.ts:439-444`). Cache `size` for `URL_CACHE.set` is the markdown byte count; binary persistence size is **not** counted into cache eviction. **Important:** the binary-content branch (`isBinaryContentType(contentType)` at `utils.ts:442`) and the HTML-decode branch (`contentType.includes('text/html')` at `utils.ts:456`) are **independent** boolean conditions, not mutually exclusive. A response whose `Content-Type` matches both predicates (e.g., a server quirk emitting `Content-Type: text/html` on a binary body, or a future MIME judged binary that also contains `text/html`) will be persisted *and* sent through Turndown. Spec's earlier wording could be read as implying mutual exclusivity; it is not enforced anywhere in the source.
- **Empty HTTP body**: `Math.max(1, contentBytes)` clamp is required because `lru-cache` rejects 0-size entries (`utils.ts:480`).
- **AbortSignal during Haiku**: `applyPromptToMarkdown` checks `signal.aborted` after `queryHaiku` and throws `AbortError()` so the surrounding tool call surfaces as `is_error` (red dot) — comment cites the rationale (`utils.ts:518-521`).
- **WebSearch API web_search error envelope**: `block.content` is non-Array; mapped to a `Web search error: <error_code>` string in `results[]`, `logError` is called with that message, but the surrounding stream is consumed normally and other blocks still appended.
- **WebSearch null entries in `results`**: `mapToolResultToToolResultBlockParam` skips `null`/`undefined` entries (the comment cites JSON round-tripping via compaction/transcript).
- **WebSearch text-block ordering**: when the model emits `text` after a `web_search_tool_result`, the state machine **replaces** the accumulator (`textAcc = block.text`) rather than concatenating, then continues to extend on subsequent contiguous `text` blocks (`WebSearchTool.ts:131-138`).
- **Both `allowed_domains` and `blocked_domains` set**: Validation rejects pre-call (errorCode 2). Once accepted, only one of them is non-undefined on the schema sent server-side.
- **Empty/no preapproved hostname split table**: `PATH_PREFIXES` matches by exact equality OR `startsWith(p + '/')`, never bare `startsWith(p)` (segment-boundary).

---

## 10. Telemetry & Observability

- **WebFetch ANT-only event**: `logEvent('tengu_web_fetch_host', { hostname })` (`utils.ts:400-405`). Emitted only when the preflight succeeded (i.e., before `getWithPermittedRedirects`). Data routing/PII handling owned by 26.
- **WebFetch logError surface**: preflight check non-200, **preflight catch (`utils.ts:200` — emits one `logError` per failed `api.anthropic.com/api/web/domain_info` attempt; subsequently surfaces as `DomainCheckFailedError` to the user without re-logging at the wrapper)**, redirect-parse failure (caught at outer try), generic non-domain errors.
- **WebSearch logError**: server-tool result error envelope `block.content.error_code`.
- **Cost / token usage**: WebSearch contributes to `usage.server_tool_use.web_search_requests`. Read at `cost-tracker.ts:271` and `modelCost.ts:139`. Owned by 06/22.
- **Progress messages (WebSearch only)**: `query_update` (regex-extracted from streaming JSON deltas), `search_results_received` (on each `web_search_tool_result` block).
- **WebSearch `extractSearchText()` is `''`**: deliberate to prevent phantom matches when the heuristic indexer scans rendered tool output (the rendered chrome is "Did N searches in Xs", which carries no result content; comment `WebSearchTool.ts:229-234`).
- **`AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` cast**: explicit type-cast at the analytics call site to satisfy the analytics module's typed-payload guard (utils.ts:402-404).

---

## 11. Reimplementation Checklist

1. **Schemas**: lazy `z.strictObject` for both tools; ensure `query.min(2)` for WebSearch; emit identical `.describe()` strings (used in tool API schema).
2. **Constants**: copy every constant in §6.13 verbatim. Cache TTLs and the 50MB sizing must be byte-accurate; `MAX_REDIRECTS=10`, `MAX_HTTP_CONTENT_LENGTH=10MB`, `FETCH_TIMEOUT_MS=60s`, `DOMAIN_CHECK_TIMEOUT_MS=10s`, `MAX_MARKDOWN_LENGTH=100k`, `MAX_URL_LENGTH=2000`.
3. **Caches**: two LRUs with distinct policies (URL = `maxSize`-by-bytes; domain = `max`-by-count). Only cache `'allowed'` domain results. Key URL_CACHE on the **original** input URL.
4. **validateURL**: all of: length≤2000, parseable, no creds, hostname has ≥2 dot-parts.
5. **HTTP fetch**: arraybuffer responseType, `Accept: text/markdown, text/html, */*`, `User-Agent` from `getWebFetchUserAgent()`, `maxRedirects:0`, `maxContentLength=10MB`, signal threading.
6. **Redirect handling**: `isPermittedRedirect` (same proto, same port, no creds, hosts equal modulo `^www\.`); recurse up to 10 hops; emit `RedirectInfo` to caller for cross-host. Egress block detection on 403 + `X-Proxy-Error: blocked-by-allowlist`.
7. **Preflight**: hit `https://api.anthropic.com/api/web/domain_info?domain=<encoded host>` with 10s timeout; `r.data.can_fetch === true` ⇒ allowed. Cache only allowed. Honor `skipWebFetchPreflight`.
8. **HTTP→HTTPS upgrade**: silent; cache key uses pre-upgrade URL.
9. **Buffer ownership**: null `response.data` after copy to `rawBuffer` to release axios's ArrayBuffer (`utils.ts:430-432`).
10. **Binary content**: detect via `isBinaryContentType`; persist via `persistBinaryContent`; **always** also decode utf-8 and pass through Haiku; append `[Binary content (...) also saved to ...]` to the result string.
11. **HTML→markdown**: lazy-loaded `turndown` singleton; only invoked when `contentType.includes('text/html')`. For non-HTML, `markdownContent = htmlContent` and `contentBytes = bytes` (no re-scan).
12. **Cache size**: clamp `Math.max(1, contentBytes)`.
13. **Preapproved fast path**: `isPreapprovedUrl(url) && contentType.includes('text/markdown') && content.length < MAX_MARKDOWN_LENGTH` ⇒ return raw `content` as the result, bypass Haiku.
14. **Secondary-model prompt**: branched guidelines on `isPreapprovedDomain`. Truncate at 100k with `\n\n[Content truncated due to length...]`.
15. **AbortError on signal.aborted** post-Haiku to force `is_error` tool_use.
16. **WebFetch permission**: preapproved-host short-circuit returns `behavior:'allow'` with `decisionReason:{type:'other', reason:'Preapproved host'}`; otherwise check deny → ask → allow rules keyed on `domain:<host>`; default to `ask` with allow-rule suggestion.
17. **WebFetch system prompt**: emit the IMPORTANT-prefix unconditionally (caching reason cited in source comment).
18. **WebSearch tool schema**: `web_search_20250305`, `name:'web_search'`, `max_uses:8`. Pass via `extraToolSchemas`.
19. **WebSearch enablement gate**: provider matrix per §3.2.
20. **WebSearch streaming consumption**: track `currentToolUseId`; accumulate `partial_json`; regex `"query"\s*:\s*"((?:[^"\\]|\\.)*)"`; `jsonParse('"' + match + '"')`; emit `query_update` on first match per id; emit `search_results_received` on `web_search_tool_result` block start.
21. **WebSearch block-merge state machine**: per §5.10, including the **replace-not-concat** behavior on text-after-non-text.
22. **WebSearch error envelope**: non-Array `content` ⇒ `Web search error: <error_code>` pushed; `logError` called.
23. **WebSearch result formatter**: prefix `Web search results for query: "${query}"\n\n`; per-entry: skip null; string ⇒ `r + '\n\n'`; SearchResult ⇒ `Links: ${jsonStringify(r.content)}\n\n` or `No links found.\n\n`; trailer `\nREMINDER: You MUST include the sources above in your response to the user using markdown hyperlinks.`; `out.trim()`.
24. **Permission-passthrough for WebSearch**; the network call lives behind the API provider — there is no client-side rule check.

---

## 12. Open Questions / Unknowns

- **Where does `tools/utils.ts`-level shared web logic live?** Confirmed: there is **no** shared utility module across the two tools — the WebFetch dir's `utils.ts` is internal to WebFetchTool. WebSearch contains no analog.
- **`learn.microsoft.com` duplicate entry** in `PREAPPROVED_HOSTS` (lines 26 and 110): Set semantics deduplicate; documented for fidelity but no behavioral impact.
- **`http://` upgrade and the cache key** (utils.ts:469 comment): noted that the cache stores under the *original* (un-upgraded, un-redirected) URL. A second request to the http URL hits cache; a subsequent request to the upgraded https URL would re-fetch. This is intentional per the comment.
- **Server-tool `web_fetch_requests` accounting**: `EMPTY_USAGE` in `utils/messages.ts:367` includes a `web_fetch_requests` counter alongside `web_search_requests`, but the WebFetch tool runs entirely client-side and never increments a server-side counter. Implication: there is a separate server-side web-fetch capability (likely a beta tool) **not used here**. Out-of-scope for this spec; flagged for 22.
- **WebSearch `useHaiku` model selection**: when `tengu_plum_vx3` is true, `getSmallFastModel()` is used regardless of `provider`. Whether the `vertex`/`foundry` provider matrix in `isEnabled()` accommodates the small/fast model correctly is not validated by source we can read; if a non-WebSearch-supporting small model is selected, the server-tool call would fail. Flagged for 22.
- **`createUserMessage` content format**: passed a plain string `'Perform a web search for the query: ' + query`; depends on `createUserMessage` accepting an unblocked string-body; semantics owned by 04 / `utils/messages.ts`.
- **Tool description's "Web search is only available in the US"** is a static prompt assertion (`prompt.ts:28`); the underlying server enforcement is owned by 22.
- **`getLocalMonthYear()`**: imports `from 'src/constants/common.js'`. We did not read its implementation; the prompt template uses its return value verbatim into `${currentMonthYear}`.

---

## §X — BUGS-IN-SOURCE Candidates (cross-link)

The following items are flagged as bugs in the source (not spec defects) and are tracked in `docs/specs/BUGS-IN-SOURCE.md`:

1. **WebSearch `Error: Missing query` validator branch is dead** (`WebSearchTool.ts:237-242` vs schema `:27`). Zod `.min(2)` preempts; the `!query.length` arm is unreachable. Fix: drop the validator branch or weaken the schema to `min(0)` if the post-Zod message is intentional. See §3.3 note.
2. **`logError` double-attribution on `DomainCheckFailedError`** (`utils.ts:200` + `utils.ts:407-413`). The inner `checkDomainBlocklist` catch logs the underlying axios error before returning `check_failed`; the outer wrapper then throws `DomainCheckFailedError` as a "user-facing expected" failure. Net effect: each failed preflight pollutes telemetry once even when the user-visible message is by-design. See §9, §10.
3. **`URL_CACHE` clamp `Math.max(1, contentBytes)`** (`utils.ts:480`). The 1-byte floor on empty bodies is asymmetric vs eviction accounting (50MB cap can hold 50M empty entries before eviction; TTL caps actual exposure). Likely benign but flagged for fidelity.
