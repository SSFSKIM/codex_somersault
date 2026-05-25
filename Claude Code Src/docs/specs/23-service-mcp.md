# 23 — MCP Service Specification

> Lifecycle, transports, auth/approval, retry, and tool-call invariants for the MCP (Model Context Protocol) service.
>
> Adjacent specs (cited, **not** redocumented): **16** (MCP/LSP tool surface — `MCPTool`, `ListMcpResourcesTool`, `ReadMcpResourceTool`, `McpAuthTool`, `LSPTool` schemas/prompts/UI), **22** (Anthropic SDK / api), **25** (OAuth flow itself — `performMCPOAuthFlow`, `ClaudeAuthProvider`, XAA, keychain), **28** (plugin loader; plugin-provided MCP servers), **34** (IDE bridge — `sse-ide`, `ws-ide` connection consumers).

---

## 1. Purpose & Scope

The MCP service owns the **lifecycle and transport plumbing** for third-party Model Context Protocol servers configured via `.mcp.json`, user/project/local/enterprise/managed settings, dynamic flags, plugin manifests, and claude.ai connectors. It hands the harness three things per server: a `client` (MCP SDK `Client`) ready for `tools/call`, a typed `MCPServerConnection` discriminated-union state, and lazily-fetched tools/resources/prompts (the latter surfaces as `Command`s and as `mcp__server__tool` `Tool`s through spec 16).

**In scope:**

- All files under `src/services/mcp/` plus `src/services/mcpServerApproval.tsx`.
- Server lifecycle: discovery, batched connect (`pMap` concurrency split local/remote), connection-cache memoization (`getServerCacheKey`), ack of `notifications/*/list_changed`, automatic reconnection with exponential backoff (`MAX_RECONNECT_ATTEMPTS = 5`, base 1 s, cap 30 s), session-expiry recovery, manual `reconnectMcpServer` / `toggleMcpServer`.
- Transports: `stdio` (with optional `CLAUDE_CODE_SHELL_PREFIX`), `sse`, `http` (Streamable HTTP), `ws`, `sse-ide`, `ws-ide`, `sdk` (in-process via `SdkControlClientTransport`), `claudeai-proxy`, plus in-process linked pairs for Chicago MCP / claude-in-chrome.
- Auth/approval surfaces: `MCPServerApprovalDialog` and `MCPServerMultiselectDialog` for project `.mcp.json` servers, `getProjectMcpServerStatus`, OAuth integration via `ClaudeAuthProvider` and the `mcp-needs-auth-cache.json` 15-min TTL gate (full OAuth flow → spec 25).
- `MCPServerConnection` and config schemas (verbatim).
- Tool-name invariants the service emits/consumes: `mcp__<server>__<tool>` prefix, `CLAUDE_AGENT_SDK_MCP_NO_PREFIX` skip path, `mcpInfo`, `getToolNameForPermissionCheck`.
- `ensureConnectedClient`, `fetchToolsForClient` / `fetchResourcesForClient` / `fetchCommandsForClient` (LRU 20, keyed by server name), `callMCPToolWithUrlElicitationRetry`, `callMCPTool`.
- McpAuth race-and-swap: `tools.ts` precedes the real connect; `McpAuthTool` (spec 16 §3.4) launches `performMCPOAuthFlow` and on success runs `clearMcpAuthCache()` + `reconnectMcpServerImpl()` and merges new tools/commands/resources into `appState.mcp` after stripping the prefix.
- Error wrapping: `McpAuthError`, `McpSessionExpiredError`, `McpToolCallError_…`, `TelemetrySafeError_…`. JSON-RPC code mapping `-32000` (Connection closed), `-32001` (Session not found / Request timeout), `-32042` (URL elicitation required).
- Telemetry: every `tengu_mcp_*` event emitted by the service.
- Flag deltas owned here: `CHICAGO_MCP` and `MCP_SKILLS` callsites inside `services/mcp/`.

**Out of scope (refer by spec #):**

- MCP tool surface (template, prompt, UI, schemas, classifyForCollapse) → 16.
- Anthropic SDK transport, file API, retries → 22.
- OAuth provider implementation, XAA, keychain layout → 25.
- Plugin manifest loading and `getPluginMcpServers` → 28.
- IDE connection consumer (auth-token format, lockfile, IDE bridge JWT) → 34.
- `query.ts` `CHICAGO_MCP` cleanup (computer-use after-turn cleanup): the service exposes only the in-process Computer Use server connection; the cleanup hook lives in `query.ts:1033,1489` and is owned by spec 04.

---

## 2. Source Map

### 2.1 In-scope files (full read unless marked)

| Path | Lines | Coverage | Purpose |
|---|---|---|---|
| `src/services/mcp/types.ts` | 1–259 | full | `MCPServerConnection`, transport configs, `McpJsonConfig`, `MCPCliState`, OAuth/XAA config schemas. |
| `src/services/mcp/client.ts` | 1–3349 | full | Connect/cleanup, transport wiring (stdio/sse/http/ws/sdk/claudeai-proxy/sse-ide/ws-ide/in-process), auth-cache, fetch caches, reconnect impl, batched discovery, tool-call retry, MCP-result transformation. |
| `src/services/mcp/auth.ts` | 1–~2200 | full read of public surface (1–700 sampled in-spec, residual is OAuth/XAA implementation owned by 25) | `wrapFetchWithStepUpDetection`, `ClaudeAuthProvider`, `hasMcpDiscoveryButNoToken`, `performMCPOAuthFlow`, `revokeServerTokens`, `getServerKey`, lockfile retry. |
| `src/services/mcp/elicitationHandler.ts` | 1–313 | full | `registerElicitationHandler`, `runElicitationHooks`, `runElicitationResultHooks`. |
| `src/services/mcp/utils.ts` | 1–576 | full | `commandBelongsToServer`, `excludeStalePluginClients`, `hashMcpConfig`, `getProjectMcpServerStatus`, `getMcpServerScopeFromToolName`, `extractAgentMcpServers`, `getLoggingSafeMcpBaseUrl`, scope/transport ensure helpers. |
| `src/services/mcp/mcpStringUtils.ts` | 1–107 | full | `mcpInfoFromString`, `getMcpPrefix`, `buildMcpToolName`, `getToolNameForPermissionCheck`, `getMcpDisplayName`, `extractMcpToolDisplayName`. |
| `src/services/mcp/normalization.ts` | 1–24 | full | `normalizeNameForMCP`. |
| `src/services/mcp/config.ts` | 1–1579 | full | Scope filesystem layer, dedup (`dedupPluginMcpServers`, `dedupClaudeAiMcpServers`, `unwrapCcrProxyUrl`, `getMcpServerSignature`), allow/deny policy, `addMcpConfig` / `removeMcpConfig`, env var expansion, claude.ai fetch entry hooks. |
| `src/services/mcp/useManageMCPConnections.ts` | 1–1142 | full | Hook driving connect lifecycle, batched AppState updates, list-changed handlers, exponential-backoff reconnect, channel notifications, `reconnectMcpServer` / `toggleMcpServer`. |
| `src/services/mcp/MCPConnectionManager.tsx` | 1–72 | full | React provider exposing `useMcpReconnect` / `useMcpToggleEnabled`. |
| `src/services/mcp/elicitationHandler.ts` | 1–313 | full | (above) |
| `src/services/mcp/InProcessTransport.ts` | 1–~70 | full | `InProcessTransport`, `createLinkedTransportPair`. |
| `src/services/mcp/SdkControlTransport.ts` | sampled | grep | `SdkControlClientTransport` for `sdk` servers (consumed by `setupSdkMcpClients`, called from `entrypoints/sdk` — spec 01). |
| `src/services/mcp/channelNotification.ts`, `channelPermissions.ts`, `channelAllowlist.ts` | sampled | grep | KAIROS channel wiring; subset behavior described in §8. |
| `src/services/mcp/claudeai.ts` | sampled | grep | `markClaudeAiMcpConnected`, `clearClaudeAIMcpConfigsCache`, `fetchClaudeAIMcpConfigsIfEligible`. |
| `src/services/mcp/headersHelper.ts` | sampled | grep | `getMcpServerHeaders` (combines static + `headersHelper` script output). |
| `src/services/mcp/envExpansion.ts` | sampled | grep | `expandEnvVarsInString`. |
| `src/services/mcp/officialRegistry.ts`, `oauthPort.ts`, `xaa.ts`, `xaaIdpLogin.ts` | sampled | grep | OAuth-flow internals owned by spec 25; the service references them. |
| `src/services/mcp/vscodeSdkMcp.ts` | sampled | grep | ANT-only IDE-bridge plumbing (gated by `process.env.USER_TYPE === 'ant'` at `vscodeSdkMcp.ts:44`); fires `tengu_vscode_*` events and reads `tengu_vscode_review_upsell` / `tengu_vscode_onboarding` Statsig gates. **Owned by spec 34** (IDE bridge), not spec 25. Service references it only to dispatch `file_updated` notifications. |
| `src/services/mcpServerApproval.tsx` | 1–40 | full | Renders `MCPServerApprovalDialog` / `MCPServerMultiselectDialog` for pending project servers. |
| `src/components/MCPServerApprovalDialog.tsx`, `MCPServerMultiselectDialog.tsx`, `MCPServerDialogCopy.tsx` | sampled | grep | Approval UI strings (verbatim in §6.5). |
| `src/components/mcp/ElicitationDialog.tsx` | 1–1168 | sampled | grep | UI consumer of the §5.6 elicitation-queue contract (`{params, signal, waitingState, respond, onWaitingDismiss}`). Renders form-mode and url-mode elicitations dequeued from `appState.mcp.elicitation.queue`. **Owned by spec 37** (UI shell); spec 23 owns the queue payload shape only — modifying the queue contract here breaks this dialog. |

### 2.2 Cross-cutting feature-flag / env gates

- `feature('CHICAGO_MCP')` — `src/services/mcp/client.ts:241,245,926,1983`, `src/services/mcp/config.ts:641,1512`. Gates the in-process Computer-Use MCP server (`isComputerUseMCPServer`), its tool overrides via `getComputerUseMCPToolOverrides`, and a default-disabled built-in server (`COMPUTER_USE_MCP_SERVER_NAME`) added via `enabledMcpServers` (opt-in, vs the usual `disabledMcpServers` opt-out).
- `feature('MCP_SKILLS')` — `src/services/mcp/client.ts:117,1392,1670,2174,2348`, `useManageMCPConnections.ts:22,684,718,723,729`, plus `commands.ts:550` (consumed by `getMcpSkillCommands`, owned by spec 20). Gates the `fetchMcpSkillsForClient` import and: cache invalidation for skills on `prompts/list_changed` and `resources/list_changed`, eager skills fetch in `getMcpToolsCommandsAndResources`, and propagation into command list.
- `feature('EXPERIMENTAL_SKILL_SEARCH')` — `useManageMCPConnections.ts:27`. Gates `clearSkillIndexCache?.()` calls when MCP skills change.
- `feature('KAIROS') || feature('KAIROS_CHANNELS')` — `useManageMCPConnections.ts:172,180,473`. Gates channel-notification handler registration and channel-permission callbacks; channel surface itself is owned by 32.
- Env: `MCP_TOOL_TIMEOUT`, `MCP_TIMEOUT`, `MCP_SERVER_CONNECTION_BATCH_SIZE`, `MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE`, `CLAUDE_AGENT_SDK_MCP_NO_PREFIX`, `CLAUDE_CODE_SHELL_PREFIX`, `ENABLE_MCP_LARGE_OUTPUT_FILES`, `CLAUDE_CODE_ENABLE_XAA`, `USER_TYPE === 'ant'` (gates stdio command-basename inclusion in `tengu_mcp_servers`).

### 2.3 Imports from / Imported by

**Imports from:** `@modelcontextprotocol/sdk/*` (Client, transports, types, auth), `src/Tool.ts` types, `src/state/AppState`, `src/utils/auth.ts`, `src/utils/secureStorage`, `src/utils/lockfile`, `src/utils/proxy`, `src/utils/mtls`, `src/utils/mcpWebSocketTransport`, `src/utils/sanitization` (`recursivelySanitizeUnicode`), `src/utils/mcpOutputStorage` (`persistBinaryContent`, `getBinaryBlobSavedMessage`), `src/utils/mcpValidation`, `src/utils/toolResultStorage`, `src/utils/imageResizer`, `src/utils/sleep`, `src/utils/cleanupRegistry`, `src/utils/codeIndexing`, analytics service (`logEvent`), constants (`getOauthConfig`, `MCP_CLIENT_METADATA_URL`, `PRODUCT_URL`).

**Imported by:** `src/main.tsx` and SDK entrypoint (boot — spec 01); `src/tools.ts` (registry filter); `src/tools/MCPTool/*` and `src/tools/{ListMcpResources,ReadMcpResource,McpAuth}Tool/*` (spec 16); `src/commands/mcp/*` (slash commands — spec 21); `src/components/mcp/*` (UI surface — spec 37 once filed); `src/coordinator/*` and `tools/shared/spawnMultiAgent.ts` (spec 30) for child-agent inheritance; `src/print.ts` and `src/entrypoints/sdk/*` (`setupSdkMcpClients`).

### 2.4 Source-coverage inventory (pre-write)

The service is dense; this spec inlines the **load-bearing** verbatim assets only. Anything not inlined below is summarized with `path:line-range` and the bytes of the assertion live in source.

| Asset | Where in this spec |
|---|---|
| `MCPServerConnection`, `ConfigScope`, transport-config Zod schemas | §3, §6.1 |
| `MCPCliState`, `SerializedTool`, `SerializedClient` | §6.1 |
| Reconnect / batch / cache / timeout / retry constants | §6.4 |
| Error class set + JSON-RPC code map | §6.6 |
| `tengu_mcp_*` analytics inventory | §10 |
| Connect dispatch table | §5.2 |
| Tool name invariants (prefix, skip-prefix, `mcpInfo`, permission-check name) | §3.5, §6.2 |
| Approval-dialog copy + select labels | §6.5 |
| URL elicitation retry pseudocode (`callMCPToolWithUrlElicitationRetry`) | §5.6 |
| Channel-blocked toast strings | §6.5 |
| Reconnect onclose + exponential-backoff pseudocode | §5.5 |
| Approval flow vs OAuth race-and-swap | §5.7 |
| Telemetry-safe error wrapping rules | §5.4, §6.6 |

---

## 3. Public Interface (Contract)

The MCP service surface decomposes into five blocks. Internal helpers (`callMCPTool`, `getMcpAuthCache`, `processBatched`, `revokeToken`, etc.) are not part of the contract; do not call them from outside `services/mcp/`.

### 3.1 Lifecycle entrypoints

```ts
// client.ts
export const connectToServer: ((
  name: string,
  serverRef: ScopedMcpServerConfig,
  serverStats?: { totalServers; stdioCount; sseCount; httpCount; sseIdeCount; wsIdeCount },
) => Promise<MCPServerConnection>) & { cache: Map<string, Promise<MCPServerConnection>> }   // memoized via lodash memoize, key = getServerCacheKey

export function getServerCacheKey(name: string, serverRef: ScopedMcpServerConfig): string   // `${name}-${jsonStringify(serverRef)}`
export async function clearServerCache(name: string, serverRef: ScopedMcpServerConfig): Promise<void>
export async function ensureConnectedClient(client: ConnectedMCPServer): Promise<ConnectedMCPServer>   // SDK servers returned as-is

export async function getMcpToolsCommandsAndResources(
  onConnectionAttempt: (params: {
    client: MCPServerConnection
    tools: Tool[]
    commands: Command[]
    resources?: ServerResource[]
  }) => void,
  mcpConfigs?: Record<string, ScopedMcpServerConfig>,
): Promise<void>

export function prefetchAllMcpResources(
  mcpConfigs: Record<string, ScopedMcpServerConfig>,
): Promise<{ clients: MCPServerConnection[]; tools: Tool[]; commands: Command[] }>

export async function reconnectMcpServerImpl(
  name: string, config: ScopedMcpServerConfig,
): Promise<{ client: MCPServerConnection; tools: Tool[]; commands: Command[]; resources?: ServerResource[] }>

export async function setupSdkMcpClients(
  sdkMcpConfigs: Record<string, McpSdkServerConfig>,
  sendMcpMessage: (serverName: string, message: JSONRPCMessage) => Promise<JSONRPCMessage>,
): Promise<{ clients: MCPServerConnection[]; tools: Tool[] }>

export function areMcpConfigsEqual(a: ScopedMcpServerConfig, b: ScopedMcpServerConfig): boolean
export function getMcpServerConnectionBatchSize(): number   // `MCP_SERVER_CONNECTION_BATCH_SIZE` || 3
```

The hook `useManageMCPConnections(dynamicMcpConfig?, isStrictMcpConfig=false)` is the **only sanctioned consumer** for the React tree. `MCPConnectionManager` exposes `useMcpReconnect()` and `useMcpToggleEnabled()`. UI components must not call `reconnectMcpServerImpl` directly.

### 3.2 Discovery / fetch caches

```ts
// memoizeWithLRU(fn, keyFn=client.name, MCP_FETCH_CACHE_SIZE=20)
export const fetchToolsForClient: (c: MCPServerConnection) => Promise<Tool[]>     // returns [] if !connected or no tools capability
export const fetchResourcesForClient: (c: MCPServerConnection) => Promise<ServerResource[]>
export const fetchCommandsForClient: (c: MCPServerConnection) => Promise<Command[]>   // converts MCP prompts to `Command{type:'prompt', isMcp, source:'mcp'}`

// Each has `.cache.delete(name)` invalidation. onclose clears all four (skills only when MCP_SKILLS).
```

Tool conversion is described in spec 16 §3.1 and §3.6; this service guarantees `mcpInfo === { serverName: client.name, toolName: tool.name }` (raw, *unnormalized*) and `isMcp: true` on every emitted tool, plus a one-shot per-tool `checkPermissions()` returning `{behavior:'passthrough', message:'MCPTool requires permission.', suggestions:[{type:'addRules', rules:[{toolName: fullyQualifiedName, ruleContent: undefined}], behavior:'allow', destination:'localSettings'}]}` (`client.ts:1814–1832`).

### 3.3 Tool invocation

```ts
export async function callMCPToolWithUrlElicitationRetry(opts: {
  client: ConnectedMCPServer
  clientConnection: MCPServerConnection
  tool: string
  args: Record<string, unknown>
  meta?: Record<string, unknown>
  signal: AbortSignal
  setAppState: (f: (prev: AppState) => AppState) => void
  onProgress?: (data: MCPProgress) => void
  callToolFn?: typeof callMCPTool      // injectable for tests
  handleElicitation?: (serverName: string, params: ElicitRequestURLParams, signal: AbortSignal) => Promise<ElicitResult>
}): Promise<{ content: MCPToolResult; _meta?: Record<string, unknown>; structuredContent?: Record<string, unknown> }>

export async function callIdeRpc(toolName: string, args: Record<string, unknown>, client: ConnectedMCPServer): Promise<string | ContentBlockParam[] | undefined>
```

Internal `callMCPTool` (`client.ts:3029–3245`) is the single dispatch primitive: it sets up a 30 s heartbeat `logMCPDebug`, races `client.callTool(...)` with a cooperative `MCP_TOOL_TIMEOUT` timer, processes the result through `processMCPResult`, and translates errors into `McpAuthError` / `McpSessionExpiredError` / `McpToolCallError_…`.

### 3.4 Auth / approval

```ts
// utils.ts
export function getProjectMcpServerStatus(serverName: string): 'approved' | 'rejected' | 'pending'
export function getMcpServerScopeFromToolName(toolName: string): ConfigScope | null

// mcpServerApproval.tsx
export async function handleMcpjsonServerApprovals(root: Root): Promise<void>   // single-server → MCPServerApprovalDialog; multiple → MCPServerMultiselectDialog
```

OAuth is driven by `auth.ts` (full surface owned by **spec 25**); this service's outward contract is:

```ts
// auth.ts
export class ClaudeAuthProvider implements OAuthClientProvider {…}
export class AuthenticationCancelledError extends Error {…}
export function wrapFetchWithStepUpDetection(inner: FetchLike, p: ClaudeAuthProvider): FetchLike
export function hasMcpDiscoveryButNoToken(name, cfg): boolean   // “probed but no token” gate
export function getServerKey(name, cfg): string                 // `${name}|${sha256(type|url|headers).slice(0,16)}`
export async function performMCPOAuthFlow(
  serverName, serverConfig, onAuthorizationUrl,
  abortSignal?, options?: { skipBrowserOpen?: boolean; onWaitingForCallback?: (submit: (cb: string) => void) => void },
): Promise<void>
export async function revokeServerTokens(name, cfg, opts?: { preserveStepUpState?: boolean }): Promise<void>
export function clearServerTokensFromLocalStorage(name, cfg): void

// client.ts
export class McpAuthError extends Error {…}                     // 401 / re-auth required
export class McpToolCallError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS extends TelemetrySafeError_…
export function clearMcpAuthCache(): void                       // wipes mcp-needs-auth-cache.json
export function isMcpSessionExpiredError(error: Error): boolean // 404 + JSON-RPC -32001
export function createClaudeAiProxyFetch(inner: FetchLike): FetchLike
export function wrapFetchWithTimeout(inner: FetchLike): FetchLike   // 60 s POSTs, infinite GETs (SSE)
```

### 3.5 Naming policy invariants

The service guarantees:

1. **Default tool name** is `mcp__${normalizeNameForMCP(serverName)}__${normalizeNameForMCP(toolName)}` (`mcpStringUtils.ts:50–52`).
2. **Skip-prefix mode** is gated narrowly: `client.config.type === 'sdk' && isEnvTruthy(process.env.CLAUDE_AGENT_SDK_MCP_NO_PREFIX)`. Even when set, `mcpInfo` is still populated. Comment at `client.ts:1771–1773` (verbatim in §6.2).
3. **Permission-check name** is `getToolNameForPermissionCheck(tool) = tool.mcpInfo ? buildMcpToolName(...) : tool.name` (`mcpStringUtils.ts:60–67`). Spec 09 must use this; spec 16 §3.6 mirrors it.
4. **Server discriminator**: `tool.name?.startsWith('mcp__') || tool.isMcp === true` (`utils.ts:246`).
5. **IDE allowlist**: `ALLOWED_IDE_TOOLS = ['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']` filters `mcp__ide__*` (`client.ts:568–571`).
6. **Server-name normalization**: regex `/[^a-zA-Z0-9_-]/g → '_'`; for `claude.ai ` prefix names, additionally collapse `_+` to `_` and strip leading/trailing `_` (`normalization.ts:17–22`).

### 3.6 Reconnect/toggle (React surface)

`useMcpReconnect()(serverName)` calls `reconnectMcpServerImpl(serverName, client.config)`, cancels any active backoff timer, and feeds the result into `onConnectionAttempt`. `useMcpToggleEnabled()(serverName)` persists the new disabled state to project config first, then either calls `clearServerCache` and emits `disabled` or starts a fresh connect. Both surfaces are wrapped by `MCPConnectionManager.tsx`.

---

## 4. Data Model & State

### 4.1 `MCPServerConnection` (verbatim — `services/mcp/types.ts:180–226`)

```ts
export type ConnectedMCPServer = {
  client: Client
  name: string
  type: 'connected'
  capabilities: ServerCapabilities
  serverInfo?: { name: string; version: string }
  instructions?: string
  config: ScopedMcpServerConfig
  cleanup: () => Promise<void>
}

export type FailedMCPServer = {
  name: string
  type: 'failed'
  config: ScopedMcpServerConfig
  error?: string
}

export type NeedsAuthMCPServer = {
  name: string
  type: 'needs-auth'
  config: ScopedMcpServerConfig
}

export type PendingMCPServer = {
  name: string
  type: 'pending'
  config: ScopedMcpServerConfig
  reconnectAttempt?: number
  maxReconnectAttempts?: number
}

export type DisabledMCPServer = {
  name: string
  type: 'disabled'
  config: ScopedMcpServerConfig
}

export type MCPServerConnection =
  | ConnectedMCPServer
  | FailedMCPServer
  | NeedsAuthMCPServer
  | PendingMCPServer
  | DisabledMCPServer

export type ServerResource = Resource & { server: string }
```

State machine (per server name):

```
                                 isMcpServerDisabled?
                                       │
                                  yes  │  no
                                  ▼    ▼
                              disabled  pending
                                  │      │  (initialized in useEffect)
                                  │      ▼
                                  │   connectToServer() in pMap batch
                                  │      │
                                  │   ┌──┴────────────────┬───────────────┐
                                  │   │                   │               │
                                  ▼   ▼                   ▼               ▼
                              disabled connected      needs-auth       failed
                                          │              │
                              onclose ────┤              │ /mcp → McpAuthTool.call
                              (clearServerCache,         │   → performMCPOAuthFlow
                               cache deletes)            │   → reconnectMcpServerImpl
                                          │              ▼
                            isMcpServerDisabled?     connected ↻ (or stays needs-auth)
                            (disk re-read; AppState
                             may be stale — see §5.5)
                              ┌───┴───┐
                          yes │       │ no & remote (sse/http/ws/claudeai-proxy)
                              ▼       ▼
                          stay      pending(reconnectAttempt++)
                          disabled  exponential backoff (1s, 2s, 4s, 8s, 16s capped at 30s)
                                          │
                                 success → connected   max 5 attempts → failed
```

### 4.2 `MCPCliState` (verbatim — `types.ts:232–258`)

```ts
export interface SerializedTool {
  name: string
  description: string
  inputJSONSchema?: { [x: string]: unknown; type: 'object'; properties?: { [x: string]: unknown } }
  isMcp?: boolean
  originalToolName?: string
}
export interface SerializedClient {
  name: string
  type: 'connected' | 'failed' | 'needs-auth' | 'pending' | 'disabled'
  capabilities?: ServerCapabilities
}
export interface MCPCliState {
  clients: SerializedClient[]
  configs: Record<string, ScopedMcpServerConfig>
  tools: SerializedTool[]
  resources: Record<string, ServerResource[]>
  normalizedNames?: Record<string, string>
}
```

### 4.3 In-memory caches

| Cache | Scope | Key | Size / TTL | Invalidation |
|---|---|---|---|---|
| `connectToServer.cache` | module | `${name}-${jsonStringify(serverRef)}` | unbounded (lodash `memoize`) | `clearServerCache(name, ref)`; `client.onclose` (deletes by `getServerCacheKey`) |
| `fetchToolsForClient.cache` | module | `client.name` | LRU 20 | `client.onclose`; `tools/list_changed`; `clearServerCache` |
| `fetchResourcesForClient.cache` | module | `client.name` | LRU 20 | `client.onclose`; `resources/list_changed`; `clearServerCache` |
| `fetchCommandsForClient.cache` | module | `client.name` | LRU 20 | `client.onclose`; `prompts/list_changed`; `resources/list_changed` (when `MCP_SKILLS`); `clearServerCache` |
| `fetchMcpSkillsForClient.cache` | module (when `MCP_SKILLS`) | `client.name` | LRU 20 | `client.onclose`; `resources/list_changed`; `clearServerCache` |
| `authCachePromise` | module | path: `~/.claude/mcp-needs-auth-cache.json` | 15 min TTL per entry | `setMcpAuthCacheEntry(name)` (write); `clearMcpAuthCache()`; on each write the in-memory promise is cleared so next read sees fresh data |
| `reconnectTimersRef` | hook (`useManageMCPConnections`) | `serverName → NodeJS.Timeout` | per session | cancellation in `reconnectMcpServer`, `toggleMcpServer`, `initializeServersAsPending` (stale plugin removal), and unmount |
| `pendingUpdatesRef` | hook | array | flush every `MCP_BATCH_FLUSH_MS = 16` ms | `flushPendingUpdates` |
| `doesEnterpriseMcpConfigExist` | module | none | `lodash memoize` | none (module-lifetime) |
| `getMcpAuthCache` | module | none | first-read | nulled on every write or `clearMcpAuthCache` |
| `writeChain` | module | none | promise chain | best-effort serialization for `mcp-needs-auth-cache.json` writes |

### 4.4 OAuth tokens / discovery state

Persisted in the OS-secure-storage `mcpOAuth` blob keyed by `getServerKey(name, cfg) = \`${name}|${sha256(type|url|headers).slice(0,16)}\`` (`auth.ts:325–341`). Layout owned by spec 25.

---

## 5. Algorithm / Control Flow

### 5.1 `getMcpToolsCommandsAndResources` (top-level connect orchestrator, `client.ts:2226–2403`)

```
allConfigEntries = Object.entries(mcpConfigs ?? (await getAllMcpConfigs()).servers)
disabled = []; configEntries = []
for entry in allConfigEntries:
  if isMcpServerDisabled(entry.name):
    onConnectionAttempt({ client: { name, type:'disabled', config }, tools: [], commands: [] })
  else:
    configEntries.push(entry)

stats = transport-counts over configEntries
localServers  = filter(isLocalMcpServer)        # type === 'stdio' or undefined or 'sdk'
remoteServers = !isLocalMcpServer

processServer(name, config):
  if isMcpServerDisabled(name): emit('disabled'); return
  if (config.type ∈ {claudeai-proxy, http, sse}) and (await isMcpAuthCached(name)
      or ((http|sse) and hasMcpDiscoveryButNoToken(name, config))):
    onConnectionAttempt({ client: needs-auth, tools: [createMcpAuthTool(name, config)], commands: [] })
    return
  client = await connectToServer(name, config, stats)        # memoized
  if client.type !== 'connected':
    emit({client, tools: type==='needs-auth' ? [createMcpAuthTool(name,config)] : [], commands: []})
    return
  if config.type === 'claudeai-proxy': markClaudeAiMcpConnected(name)
  supportsResources = !!client.capabilities?.resources
  [tools, mcpCommands, mcpSkills, resources] = await Promise.all([
    fetchToolsForClient(client),
    fetchCommandsForClient(client),
    feature('MCP_SKILLS') && supportsResources ? fetchMcpSkillsForClient(client) : [],
    supportsResources ? fetchResourcesForClient(client) : [],
  ])
  resourceTools = (supportsResources && !alreadyAdded) ? [ListMcpResourcesTool, ReadMcpResourceTool] : []
  emit({client, tools: [...tools, ...resourceTools], commands: [...mcpCommands, ...mcpSkills],
        resources: resources.length ? resources : undefined})

await Promise.all([
  pMap(localServers,  processServer, {concurrency: getMcpServerConnectionBatchSize()       /* 3 default */}),
  pMap(remoteServers, processServer, {concurrency: getRemoteMcpServerConnectionBatchSize() /* 20 default */}),
])
```

`processBatched` is `pMap`. Pre-2026-03 the implementation ran fixed-size sequential batches; the comment at `client.ts:2212–2217` records that one slow server in batch N used to hold up batch N+1 even with idle slots. `pMap` releases each slot as soon as its server completes.

### 5.2 `connectToServer` transport dispatch (`client.ts:595–1639`)

| Branch | Trigger | Transport | Notes |
|---|---|---|---|
| 1 | `serverRef.type === 'sse'` | `SSEClientTransport` | `authProvider = new ClaudeAuthProvider(name, ref)`; static + `headersHelper` headers; `fetch = wrapFetchWithTimeout(wrapFetchWithStepUpDetection(createFetchWithInit(), authProvider))`; `eventSourceInit.fetch` is **un-timeouted** (long-lived stream) and applies auth headers explicitly. |
| 2 | `'sse-ide'` | `SSEClientTransport` | No auth provider (TODO: lockfile-token); proxy options if any. |
| 3 | `'ws-ide'` | `WebSocketTransport` over Bun `WebSocket` or `ws`; headers `User-Agent` + optional `X-Claude-Code-Ide-Authorization`. |
| 4 | `'ws'` | `WebSocketTransport`; static + helper headers + `Authorization: Bearer ${sessionIngressToken}` if present. Authorization redacted in logs. |
| 5 | `'http'` | `StreamableHTTPClientTransport` | `authProvider`, fresh-timeout fetch wrapper, optional `X-Claude-Code-Ide-Authorization`-style session ingress (omitted if `authProvider.tokens()` is set). Pre-test logs DNS hint for loopback. |
| 6 | `'sdk'` | throws `'SDK servers should be handled in print.ts'`; routed via `setupSdkMcpClients`. |
| 7 | `'claudeai-proxy'` | `StreamableHTTPClientTransport` to `${MCP_PROXY_URL}${MCP_PROXY_PATH.replace('{server_id}', id)}`; `fetch = wrapFetchWithTimeout(createClaudeAiProxyFetch(globalThis.fetch))` (handles 401 by force-refreshing claude.ai OAuth and retrying). Header `X-Mcp-Client-Session-Id: ${getSessionId()}`. |
| 8 | `(stdio | undef) && isClaudeInChromeMCPServer(name)` | in-process via `createLinkedTransportPair()`; spawns `createClaudeForChromeMcpServer(createChromeContext(env))`. |
| 9 | `feature('CHICAGO_MCP') && (stdio | undef) && isComputerUseMCPServer(name)` | in-process via `createLinkedTransportPair()`; spawns `createComputerUseMcpServerForCli()`. Tool overrides applied later in `fetchToolsForClient` by `getComputerUseMCPToolOverrides(tool.name)`. |
| 10 | `(stdio | undef)` else | `StdioClientTransport({command, args, env})` | `command = process.env.CLAUDE_CODE_SHELL_PREFIX || serverRef.command`; if shell prefix is set, `args = [[command, ...args].join(' ')]`. `env = {...subprocessEnv(), ...serverRef.env}`. `stderr: 'pipe'` (handler caps at 64 MB). |
| ELSE | unknown `type` | throws `Unsupported server type: ${type}`. |

After transport creation:

1. Stderr listener attached to capture failed-connect output (capped 64 MB).
2. `Client` instantiated with metadata `{name:'claude-code', title:'Claude Code', version:MACRO.VERSION??'unknown', description:"Anthropic's agentic coding tool", websiteUrl:PRODUCT_URL}` and capabilities `{roots:{}, elicitation:{}}` (the comment at `client.ts:996–999` notes Spring AI / Java MCP SDK refuses any `{form:{},url:{}}` content here).
3. `client.setRequestHandler(ListRootsRequestSchema, async () => ({roots:[{uri:\`file://${getOriginalCwd()}\`}]}))`.
4. `Promise.race([client.connect(transport), timeoutPromise(getConnectionTimeoutMs() = MCP_TIMEOUT || 30_000)])`. On timeout the transport is closed and a `TelemetrySafeError_…` is rejected.
5. SSE/HTTP-specific catches: `UnauthorizedError → handleRemoteAuthFailure(name, ref, transportType)` → emits `tengu_mcp_server_needs_auth`, calls `setMcpAuthCacheEntry(name)`, and returns `{name,type:'needs-auth',config}`. claude.ai-proxy catches `code === 401` and routes through `handleRemoteAuthFailure(...,'claudeai-proxy')`.
6. On success: `client.getServerCapabilities()`, `getServerVersion()`, `getInstructions()`. Instructions truncated at `MAX_MCP_DESCRIPTION_LENGTH = 2048` with suffix `'… [truncated]'`.
7. Default elicitation handler (`{action:'cancel'}`) is registered to swallow elicitations during initialization until `registerElicitationHandler` overwrites it from `useManageMCPConnections.onConnectionAttempt`.
8. Bridged onerror/onclose installed (see §5.3); cleanup callback registered with `cleanupRegistry`. Returns `ConnectedMCPServer{name, client, type:'connected', capabilities ?? {}, serverInfo, instructions, config, cleanup: wrappedCleanup}`.

### 5.3 onerror / onclose bridging (`client.ts:1216–1402`)

```
client.onerror = (error):
  log uptime + transportType
  pattern-match error.message → ECONNRESET / ETIMEDOUT / EPIPE / EHOSTUNREACH / ECONNREFUSED / ESRCH / spawn → debug log

  if transport ∈ {http, claudeai-proxy} and isMcpSessionExpiredError(error):    # 404 + JSON-RPC -32001
    closeTransportAndRejectPending('session expired'); originalOnerror(error); return

  if transport ∈ {sse, http, claudeai-proxy}:
    if 'Maximum reconnection attempts' in error.message:
      closeTransportAndRejectPending('SSE reconnection exhausted'); originalOnerror(error); return
    if isTerminalConnectionError(message):                                       # ECONNRESET/ETIMEDOUT/EPIPE/...
      consecutiveConnectionErrors++
      if consecutiveConnectionErrors >= MAX_ERRORS_BEFORE_RECONNECT (=3):
        consecutiveConnectionErrors = 0
        closeTransportAndRejectPending('max consecutive terminal errors')
    else:
      consecutiveConnectionErrors = 0
  originalOnerror?(error)

client.onclose = ():
  log uptime + dirty/clean
  fetchToolsForClient.cache.delete(name)
  fetchResourcesForClient.cache.delete(name)
  fetchCommandsForClient.cache.delete(name)
  if MCP_SKILLS: fetchMcpSkillsForClient.cache.delete(name)
  connectToServer.cache.delete(getServerCacheKey(name, serverRef))
  originalOnclose?()
```

`closeTransportAndRejectPending` is guarded by `hasTriggeredClose`; calling `client.close()` (not `client.onclose?.()`) ensures the SDK's `_onclose()` rejects all pending request handlers so hung `callTool()` promises fail with `McpError -32000 "Connection closed"` instead of hanging.

### 5.4 Tool call (`fetchToolsForClient` per-tool `call` body, `client.ts:1833–1971`)

```
toolUseId = extractToolUseId(parentMessage)
meta = toolUseId ? {'claudecode/toolUseId': toolUseId} : {}
emit progress(status='started') if onProgress && toolUseId
startTime = Date.now()
MAX_SESSION_RETRIES = 1
for attempt in 0,1,…:
  try:
    connectedClient = await ensureConnectedClient(client)
    mcpResult = await callMCPToolWithUrlElicitationRetry({client: connectedClient, clientConnection: client,
      tool: tool.name, args, meta, signal: ctx.abortController.signal,
      setAppState: ctx.setAppState,
      onProgress: progressData → onProgress({toolUseID, data: progressData}),
      handleElicitation: ctx.handleElicitation })
    emit progress(status='completed', elapsedTimeMs)
    return { data: mcpResult.content,
             ...((mcpResult._meta || mcpResult.structuredContent) && {
               mcpMeta: { ...(mcpResult._meta && {_meta}), ...(structuredContent && {structuredContent}) }})
           }
  catch error:
    if error instanceof McpSessionExpiredError && attempt < MAX_SESSION_RETRIES:
      logMCPDebug; continue   # cache cleared inside callMCPTool's session-expired branch
    emit progress(status='failed')
    # Telemetry-safe wrap (verbatim rules)
    if error instanceof Error and !(error instanceof TelemetrySafeError_…):
      const n = error.constructor.name
      if n === 'Error':       throw new TelemetrySafeError_…(error.message, error.message.slice(0, 200))
      if n === 'McpError' and typeof error.code === 'number':
                              throw new TelemetrySafeError_…(error.message, `McpError ${error.code}`)
    throw error
```

`callMCPTool` (internal, `client.ts:3029–3245`) does:

1. 30 s heartbeat `setInterval` `logMCPDebug`.
2. `Promise.race([client.callTool({name:tool, arguments:args, _meta:meta}, CallToolResultSchema, {signal, timeout: getMcpToolTimeoutMs(), onprogress: sdkProgress → onProgress({type:'mcp_progress', status:'progress', serverName, toolName, progress, total, progressMessage})}), timeoutPromise])`.
3. If `result.isError`: extract `text` of first content block (else `String(result.error)`), throw `McpToolCallError_…(errorDetails, 'MCP tool returned error', _meta?)`.
4. `processMCPResult(result, tool, name)` (see §5.8).
5. On 401 / `UnauthorizedError`: emit `tengu_mcp_tool_call_auth_error`; throw `McpAuthError(name, 'MCP server "${name}" requires re-authorization (token expired)')`.
6. On `isMcpSessionExpiredError(e)` OR (`-32000 'Connection closed'` AND `config.type ∈ {http, claudeai-proxy}`): emit `tengu_mcp_session_expired`; `await clearServerCache(name, config)`; throw `McpSessionExpiredError(name)`.
7. `AbortError` → return `{content: undefined}` (no logspew).

### 5.5 Reconnect with exponential backoff (`useManageMCPConnections.ts:354–467`)

```
configType = client.config.type ?? 'stdio'
clearServerCache(client.name, client.config)   # ignores errors
if isMcpServerDisabled(client.name): return    # check disk state, AppState may be stale
if configType in {stdio, sdk}: updateServer({...client, type:'failed'}); return

cancel any existing reconnectTimersRef[name]
for attempt in 1..MAX_RECONNECT_ATTEMPTS (=5):
  if isMcpServerDisabled(name): return
  updateServer({...client, type:'pending', reconnectAttempt: attempt, maxReconnectAttempts: 5})
  result = await reconnectMcpServerImpl(name, client.config)
  if result.client.type === 'connected':
    onConnectionAttempt(result); return
  if attempt === 5: onConnectionAttempt(result); return
  backoffMs = min(1000 * 2^(attempt-1), 30000)   # 1, 2, 4, 8, 16 (capped 30) → next attempt is the 5th
  reconnectTimersRef[name] = setTimeout(resolve, backoffMs); await
```

Initial server-list build (`initializeServersAsPending`):

1. `existingConfigs = isStrictMcpConfig ? {} : (await getClaudeCodeMcpConfigs(dynamicMcpConfig)).servers`
2. `configs = {...existingConfigs, ...dynamicMcpConfig}`
3. `excludeStalePluginClients(prevState.mcp, configs)` removes:
   - `scope === 'dynamic'` clients whose names are no longer in `configs` (plugin disabled), AND
   - any client whose `hashMcpConfig(c.config) !== hashMcpConfig(configs[c.name])` (config edited).
4. For each stale client, cancel its reconnect timer and `clearServerCache` if it was `'connected'`. Comments at `useManageMCPConnections.ts:790–812` enumerate the three hazards: pending reconnect with old config, onclose firing reconnectWithBackoff with old config, and `clearServerCache` calling `connectToServer` for never-connected entries.
5. New client names appended as `'pending'` (or `'disabled'` per `isMcpServerDisabled`).
6. Effect runs on `[isStrictMcpConfig, dynamicMcpConfig, setAppState, sessionId, _pluginReconnectKey]`.

### 5.6 URL elicitation retry (`callMCPToolWithUrlElicitationRetry`, `client.ts:2813–3027`)

```
MAX_URL_ELICITATION_RETRIES = 3
for attempt in 0..∞:
  try: return await callToolFn({client, tool, args, meta, signal, onProgress})
  catch error:
    if !(error instanceof McpError) or error.code !== ErrorCode.UrlElicitationRequired (-32042): throw
    if attempt >= 3: throw
    rawElicitations = error.data?.elicitations || []
    elicitations = filter rawElicitations to objects with mode='url' && string url + elicitationId + message
    if elicitations.length === 0: throw

    for each elicitation:
      hookResponse = await runElicitationHooks(serverName, elicitation, signal)
      if hookResponse:
        if hookResponse.action !== 'accept':
          return { content: `URL elicitation was ${decline?'declined':action+'ed'} by a hook. The tool "${tool}" could not complete because it requires the user to open a URL.` }
        continue
      userResult = handleElicitation
        ? await handleElicitation(serverName, elicitation, signal)        # print/SDK mode
        : await new Promise(resolve =>                                    # REPL mode
            setAppState(prev => ({...prev, elicitation:{queue:[...prev.elicitation.queue, {
              serverName, requestId: `error-elicit-${id}`, params: elicitation, signal,
              waitingState: { actionLabel: 'Retry now', showCancel: true },
              respond: result => result.action==='accept' ? noop : (signal.removeEventListener('abort',onAbort), resolve(result)),
              onWaitingDismiss: action => action==='retry' ? resolve({action:'accept'}) : resolve({action:'cancel'}),
            }]}})))
      finalResult = await runElicitationResultHooks(serverName, userResult, signal, 'url', id)
      if finalResult.action !== 'accept':
        return { content: `URL elicitation was ${decline?'declined':action+'ed'} by the user. The tool "${tool}" could not complete because it requires the user to open a URL.` }
    # loop back, retry callToolFn
```

The completion notification (`ElicitationCompleteNotificationSchema`) is handled at registration in `elicitationHandler.ts:175–207` and toggles `completed: true` on the matching queue entry; the dialog resolves on dismiss.

### 5.7 McpAuthTool race-and-swap (cross-cite spec 16 §3.4)

The pseudo-tool `mcp__<server>__authenticate` (created by `createMcpAuthTool` and added to the per-server tool list whenever `connectToServer` returns `'needs-auth'` — `client.ts:2316–2320, 2331`) executes `performMCPOAuthFlow(serverName, sseOrHttpConfig, onAuthorizationUrl, signal, {skipBrowserOpen:true})`. The implementation **races URL capture against flow completion**: if the URL arrives first, returns `auth_url` with the URL embedded; the flow continues in the background, on success fires `clearMcpAuthCache()` then `reconnectMcpServerImpl` and merges results into `appState.mcp` after `reject`-ing any prior tool/command starting with `getMcpPrefix(serverName)`. The pseudo-tool itself disappears because its name shares the prefix.

### 5.8 `processMCPResult` / `transformMCPResult` (`client.ts:2632–2799`)

```
{content, type, schema} = transformMCPResult(result, tool, name)
  # type: 'toolResult' | 'structuredContent' | 'contentArray'
  # 'toolResult'        ← legacy pre-spec field, content = String(result.toolResult)
  # 'structuredContent' ← jsonStringify(result.structuredContent), schema = inferCompactSchema(...)
  # 'contentArray'      ← Promise.all(map(transformResultContent(item, name))).flat(), schema = inferCompactSchema(...)
  # else → throw TelemetrySafeError_… 'MCP tool unexpected response format'

if name === 'ide': return content                            # IDE bypass — not sent to model
if !mcpContentNeedsTruncation(content): return content
sizeEstimateTokens = getContentSizeEstimate(content)

if isEnvDefinedFalsy(ENABLE_MCP_LARGE_OUTPUT_FILES):
  emit tengu_mcp_large_result_handled{outcome:'truncated', reason:'env_disabled', sizeEstimateTokens}
  return await truncateMcpContentIfNeeded(content)

if !content: return content
if contentContainsImages(content):
  emit ...{reason:'contains_images'}; return await truncateMcpContentIfNeeded(content)

persistId = `mcp-${normalizeNameForMCP(name)}-${normalizeNameForMCP(tool)}-${Date.now()}`
contentStr = (typeof content === 'string') ? content : jsonStringify(content, null, 2)
persistResult = await persistToolResult(contentStr, persistId)

if isPersistError(persistResult):
  emit ...{outcome:'truncated', reason:'persist_failed'}
  return `Error: result (${len.toLocaleString()} characters) exceeds maximum allowed tokens. Failed to save output to file: ${persistResult.error}. If this MCP server provides pagination or filtering tools, use them to retrieve specific portions of the data.`

emit tengu_mcp_large_result_handled{outcome:'persisted', reason:'file_saved', sizeEstimateTokens, persistedSizeChars}
return getLargeOutputInstructions(persistResult.filepath, persistResult.originalSize, getFormatDescription(type, schema))
```

`transformResultContent(content, server)` (`client.ts:2478–2591`) maps MCP content types to Anthropic SDK `ContentBlockParam`s: `text` → text block; `audio` (base64) → `persistBlobToTextBlock(...)`; `image` → resized via `maybeResizeAndDownsampleImageBuffer`; `resource` (`text` | `blob`) → text block prefixed with `[Resource from <server> at <uri>] `, with image blobs resized inline and other blobs persisted; `resource_link` → `[Resource link: ${name}] ${uri}` (+ description if present).

### 5.9 fetchToolsForClient — search-hint and alwaysLoad (`client.ts:1779–1786`)

`searchHint`: `tool._meta?.['anthropic/searchHint']` collapsed via `replace(/\s+/g, ' ').trim()`; falsy ⇒ `undefined`. Comment at line 1776 calls out that newlines must be stripped because `formatDeferredToolLine` joins on `'\n'`. `alwaysLoad`: `tool._meta?.['anthropic/alwaysLoad'] === true`.

### 5.10 fetchCommandsForClient — MCP prompts as Commands (`client.ts:2033–2107`)

Builds `Command{type:'prompt', name: 'mcp__'+normalizeNameForMCP(serverName)+'__'+prompt.name, description: prompt.description ?? '', hasUserSpecifiedDescription: !!prompt.description, contentLength: 0, isEnabled: ()=>true, isHidden: false, isMcp: true, progressMessage: 'running', userFacingName: () => '${serverName}:${prompt.name} (MCP)', argNames, source:'mcp', getPromptForCommand(args)}`. `getPromptForCommand`: parses args by space, calls `client.getPrompt({name:prompt.name, arguments: zipObject(argNames, argsArray)})`, then `transformResultContent` over each message and flattens. Errors logged via `logMCPError` and rethrown.

`fetchMcpSkillsForClient` (gated by `MCP_SKILLS`, owned by `src/skills/mcpSkills.ts`) discovers `skill://` resources and builds `Command{loadedFrom:'mcp', source:'mcp'}` skills alongside prompts.

### 5.11 Approval flow (`mcpServerApproval.tsx`)

```
projectServers = getMcpConfigsByScope('project').servers
pending = filter where getProjectMcpServerStatus(name) === 'pending'
if pending.length === 0: return
if pending.length === 1: render <MCPServerApprovalDialog serverName={pending[0]} onDone={resolve} />
else:                    render <MCPServerMultiselectDialog serverNames={pending} onDone={resolve} />
```

Both dialogs persist via `updateSettingsForSource('localSettings', …)`:

| Choice | Single dialog | Multiselect dialog |
|---|---|---|
| approve one | `enabledMcpjsonServers ∋ name` | `enabledMcpjsonServers ∪ approvedServers` |
| approve all (single only, `'yes_all'`) | also `enableAllProjectMcpServers: true` | n/a |
| reject one | `disabledMcpjsonServers ∋ name` | `disabledMcpjsonServers ∪ rejectedServers` |
| Esc | route through `'no'` | `disabledMcpjsonServers ∪ serverNames` |

Analytics: `tengu_mcp_dialog_choice {choice}` for single, `tengu_mcp_multidialog_choice {approved, rejected}` for multi.

`getProjectMcpServerStatus` (`utils.ts:351–406`) decision tree:

```
norm = normalizeNameForMCP(name)
if settings.disabledMcpjsonServers ∋ norm: return 'rejected'
if settings.enabledMcpjsonServers ∋ norm OR settings.enableAllProjectMcpServers: return 'approved'
if hasSkipDangerousModePermissionPrompt() and isSettingSourceEnabled('projectSettings'): return 'approved'
   # SECURITY: explicitly does NOT consider projectSettings or sessionBypassPermissionsMode
if getIsNonInteractiveSession() and isSettingSourceEnabled('projectSettings'): return 'approved'
return 'pending'
```

### 5.12 Channel-notification gate (`useManageMCPConnections.ts:469–614`, gated by `KAIROS` / `KAIROS_CHANNELS`)

`gateChannelServer(name, capabilities, pluginSource) → { action: 'register'|'skip', kind?, reason? }` decides whether to install:

- `notifications/claude/channel` notification handler → `enqueue({mode:'prompt', value:wrapChannelMessage(...), priority:'next', isMeta:true, origin:{kind:'channel', server:name}, skipSlashCommands:true})`;
- `notifications/claude/channel/permission` handler — only registered if `client.capabilities.experimental['claude/channel/permission']` is declared — resolves pending channel-permission promises via `channelPermCallbacksRef.current.resolve(request_id, behavior, name)`.

Skip kinds emit `tengu_mcp_channel_gate{registered, skip_kind, entry_kind, is_dev, plugin}` (capability-miss is suppressed). The four user-visible toast strings are inlined verbatim in §6.5.

### 5.13 Dedup and policy (`config.ts`)

- `getMcpServerSignature(config)`: `'stdio:${jsonStringify([command, ...args])}'` for stdio, `'url:${unwrapCcrProxyUrl(url)}'` for remote, `null` for sdk.
- `unwrapCcrProxyUrl(url)` reads `mcp_url` query param when path matches one of `['/v2/session_ingress/shttp/mcp/', '/v2/ccr-sessions/']`.
- `dedupPluginMcpServers(pluginServers, manualServers)`: drop plugin server when its signature matches any manually-configured server (manual wins) or any earlier plugin server (first-loaded wins). Disabled / policy-blocked manual servers are excluded from dedup targets so neither a manual disabled nor its plugin twin gets silently disabled.
- `dedupClaudeAiMcpServers(claudeAiServers, manualServers)`: claude.ai connector dropped when its URL signature matches an enabled manual server.
- `isMcpServerAllowedByPolicy(name, cfg)`: deny list (name | command | url-glob) takes precedence; allow list semantics — empty array blocks all; if any `serverCommand`/`serverUrl` entries exist, *the matching transport class must hit one of them*; name-based fallback otherwise. URL globs: `*` ⇒ `.*`, anchored.
- `shouldAllowManagedMcpServersOnly()`: if `policySettings.allowManagedMcpServersOnly === true`, allowlist sources from policySettings only; denylist always merges all sources.
- `doesEnterpriseMcpConfigExist()` (memoized): when true, only enterprise `managed-mcp.json` servers run; user/project/local/plugin/claude.ai are dropped.

### 5.14 Cleanup escalation for stdio (`client.ts:1404–1570`)

```
SIGINT → wait 100 ms (poll process.kill(pid, 0) every 50 ms)
  if still alive: SIGTERM → wait 400 ms
    if still alive: SIGKILL
total budget 600 ms (failsafe). All errors swallowed; logMCPDebug only.
```

In-process servers (`isClaudeInChromeMCPServer`, `isComputerUseMCPServer`) call `inProcessServer.close()` then `client.close()`. Stderr listener removed before signaling. Cleanup is registered with `registerCleanup` so harness shutdown invokes it.

---

## 6. Verbatim Assets

### 6.1 Config Zod schemas (verbatim — `services/mcp/types.ts:10–135`)

```ts
export const ConfigScopeSchema = lazySchema(() =>
  z.enum([
    'local',
    'user',
    'project',
    'dynamic',
    'enterprise',
    'claudeai',
    'managed',
  ]),
)

export const TransportSchema = lazySchema(() =>
  z.enum(['stdio', 'sse', 'sse-ide', 'http', 'ws', 'sdk']),
)

export const McpStdioServerConfigSchema = lazySchema(() =>
  z.object({
    type: z.literal('stdio').optional(),     // Optional for backwards compatibility
    command: z.string().min(1, 'Command cannot be empty'),
    args: z.array(z.string()).default([]),
    env: z.record(z.string(), z.string()).optional(),
  }),
)

const McpXaaConfigSchema = lazySchema(() => z.boolean())
const McpOAuthConfigSchema = lazySchema(() =>
  z.object({
    clientId: z.string().optional(),
    callbackPort: z.number().int().positive().optional(),
    authServerMetadataUrl: z
      .string()
      .url()
      .startsWith('https://', { message: 'authServerMetadataUrl must use https://' })
      .optional(),
    xaa: McpXaaConfigSchema().optional(),
  }),
)

export const McpSSEServerConfigSchema = lazySchema(() =>
  z.object({
    type: z.literal('sse'),
    url: z.string(),
    headers: z.record(z.string(), z.string()).optional(),
    headersHelper: z.string().optional(),
    oauth: McpOAuthConfigSchema().optional(),
  }),
)

export const McpSSEIDEServerConfigSchema = lazySchema(() =>
  z.object({
    type: z.literal('sse-ide'),
    url: z.string(),
    ideName: z.string(),
    ideRunningInWindows: z.boolean().optional(),
  }),
)

export const McpWebSocketIDEServerConfigSchema = lazySchema(() =>
  z.object({
    type: z.literal('ws-ide'),
    url: z.string(),
    ideName: z.string(),
    authToken: z.string().optional(),
    ideRunningInWindows: z.boolean().optional(),
  }),
)

export const McpHTTPServerConfigSchema = lazySchema(() =>
  z.object({
    type: z.literal('http'),
    url: z.string(),
    headers: z.record(z.string(), z.string()).optional(),
    headersHelper: z.string().optional(),
    oauth: McpOAuthConfigSchema().optional(),
  }),
)

export const McpWebSocketServerConfigSchema = lazySchema(() =>
  z.object({
    type: z.literal('ws'),
    url: z.string(),
    headers: z.record(z.string(), z.string()).optional(),
    headersHelper: z.string().optional(),
  }),
)

export const McpSdkServerConfigSchema = lazySchema(() =>
  z.object({ type: z.literal('sdk'), name: z.string() }),
)

export const McpClaudeAIProxyServerConfigSchema = lazySchema(() =>
  z.object({ type: z.literal('claudeai-proxy'), url: z.string(), id: z.string() }),
)

export const McpServerConfigSchema = lazySchema(() =>
  z.union([
    McpStdioServerConfigSchema(),
    McpSSEServerConfigSchema(),
    McpSSEIDEServerConfigSchema(),
    McpWebSocketIDEServerConfigSchema(),
    McpHTTPServerConfigSchema(),
    McpWebSocketServerConfigSchema(),
    McpSdkServerConfigSchema(),
    McpClaudeAIProxyServerConfigSchema(),
  ]),
)

export type ScopedMcpServerConfig = McpServerConfig & {
  scope: ConfigScope
  pluginSource?: string
}

export const McpJsonConfigSchema = lazySchema(() =>
  z.object({ mcpServers: z.record(z.string(), McpServerConfigSchema()) }),
)
```

`MCPServerConnection` and `MCPCliState` are inlined in §4.1, §4.2.

### 6.2 Skip-prefix mode invariant (verbatim — `client.ts:1768–1832`)

```ts
const fullyQualifiedName = buildMcpToolName(client.name, tool.name)
return {
  ...MCPTool,
  // In skip-prefix mode, use the original name for model invocation so MCP tools
  // can override builtins by name. mcpInfo is used for permission checking.
  name: skipPrefix ? tool.name : fullyQualifiedName,
  mcpInfo: { serverName: client.name, toolName: tool.name },
  isMcp: true,
  …
}
```

`getToolNameForPermissionCheck` (verbatim — `mcpStringUtils.ts:60–67`):

```ts
export function getToolNameForPermissionCheck(tool: {
  name: string
  mcpInfo?: { serverName: string; toolName: string }
}): string {
  return tool.mcpInfo
    ? buildMcpToolName(tool.mcpInfo.serverName, tool.mcpInfo.toolName)
    : tool.name
}
```

`normalizeNameForMCP` (verbatim — `normalization.ts:17–22`):

```ts
export function normalizeNameForMCP(name: string): string {
  let normalized = name.replace(/[^a-zA-Z0-9_-]/g, '_')
  if (name.startsWith('claude.ai ')) {
    normalized = normalized.replace(/_+/g, '_').replace(/^_|_$/g, '')
  }
  return normalized
}
```

### 6.3 Streamable-HTTP fetch invariant (verbatim — `client.ts:466–550`)

`MCP_STREAMABLE_HTTP_ACCEPT = 'application/json, text/event-stream'`. POSTs without `accept` are stamped with this value (defense against runtime/agent header drops); GETs are exempt because in MCP transports they're long-lived SSE streams. Per-request fresh `AbortController`+`setTimeout(MCP_REQUEST_TIMEOUT_MS=60000)` (chosen over `AbortSignal.timeout()` to avoid Bun lazy-GC native-memory leak — comment at line 514).

### 6.4 Constants table

| Name | Value | File:Line |
|---|---|---|
| `DEFAULT_MCP_TOOL_TIMEOUT_MS` | `100_000_000` (~27.8 h) | `client.ts:211` |
| `MCP_TOOL_TIMEOUT` env override | parseInt | `client.ts:226` |
| `MAX_MCP_DESCRIPTION_LENGTH` | `2048` | `client.ts:218` |
| `MCP_AUTH_CACHE_TTL_MS` | `15 * 60 * 1000` | `client.ts:257` |
| `getMcpAuthCachePath()` | `${getClaudeConfigHomeDir()}/mcp-needs-auth-cache.json` | `client.ts:261–263` |
| `getConnectionTimeoutMs()` | `MCP_TIMEOUT` || `30000` | `client.ts:456–458` |
| `MCP_REQUEST_TIMEOUT_MS` | `60000` | `client.ts:463` |
| `MCP_STREAMABLE_HTTP_ACCEPT` | `'application/json, text/event-stream'` | `client.ts:471` |
| `getMcpServerConnectionBatchSize()` | `MCP_SERVER_CONNECTION_BATCH_SIZE` || `3` | `client.ts:552–554` |
| `getRemoteMcpServerConnectionBatchSize()` | `MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE` || `20` | `client.ts:556–561` |
| `ALLOWED_IDE_TOOLS` | `['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']` | `client.ts:568` |
| `MAX_ERRORS_BEFORE_RECONNECT` | `3` | `client.ts:1228` |
| `MAX_SESSION_RETRIES` | `1` | `client.ts:1859` |
| `MAX_URL_ELICITATION_RETRIES` | `3` | `client.ts:2850` |
| `MCP_FETCH_CACHE_SIZE` | `20` | `client.ts:1726` |
| `MAX_RECONNECT_ATTEMPTS` | `5` | `useManageMCPConnections.ts:88` |
| `INITIAL_BACKOFF_MS` | `1000` | `useManageMCPConnections.ts:89` |
| `MAX_BACKOFF_MS` | `30000` | `useManageMCPConnections.ts:90` |
| `MCP_BATCH_FLUSH_MS` | `16` | `useManageMCPConnections.ts:207` |
| `AUTH_REQUEST_TIMEOUT_MS` | `30000` | `auth.ts:65` |
| `MAX_LOCK_RETRIES` | `5` | `auth.ts:94` |
| `MAX_FLUSH_… SIGINT/SIGTERM/SIGKILL escalation` | 100 / 400 / 500 ms (failsafe 600 ms); stderr cap 64 MiB | `client.ts:1430–1565` |
| Streaming/HTTP `requestInit.timeoutMs` log key | `MCP_REQUEST_TIMEOUT_MS` | `client.ts:857` |

### 6.5 Approval prompts (verbatim)

`MCPServerApprovalDialog` (`components/MCPServerApprovalDialog.tsx`):

- Title: `\`New MCP server found in .mcp.json: ${serverName}\``
- Color: `'warning'`
- Choices:
  - `{ label: 'Use this and all future MCP servers in this project', value: 'yes_all' }`
  - `{ label: 'Use this MCP server', value: 'yes' }`
  - `{ label: 'Continue without using this MCP server', value: 'no' }`
- Cancel = `'no'`. (Plus `<MCPServerDialogCopy />` body — the legal/security blurb shipped separately; out-of-scope to inline here.)

`MCPServerMultiselectDialog`:

- Title: `\`${serverNames.length} new MCP servers found in .mcp.json\``
- Subtitle: `'Select any you wish to enable.'`
- Color: `'warning'`
- Footer hints: `Space=select`, `Enter=confirm`, `Esc → reject all`.
- Cancel/Esc rejects ALL pending servers.

Channel-blocked toast strings (verbatim — `useManageMCPConnections.ts:597–610`):

- `disabled` → `'Channels are not currently available'`
- `auth` → `'Channels require claude.ai authentication · run /login'`
- `policy` → `'Channels are not enabled for your org · have an administrator set channelsEnabled: true in managed settings'`
- `marketplace` / `allowlist` → `gate.reason` verbatim
- key: `\`channels-blocked-${gate.kind}\``, priority `'high'`, color `'warning'`, `timeoutMs: 12000`

### 6.6 Error class set + JSON-RPC code map

```ts
// client.ts
export class McpAuthError extends Error { serverName: string; constructor(name, msg) { name = 'McpAuthError' } }
class McpSessionExpiredError extends Error { constructor(name) { super(`MCP server "${name}" session expired`); name = 'McpSessionExpiredError' } }
export class McpToolCallError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS extends TelemetrySafeError_…
{ readonly mcpMeta?: { _meta?: Record<string, unknown> }; constructor(message, telemetryMessage, mcpMeta?) }

// auth.ts
export class AuthenticationCancelledError extends Error
```

Detection helpers:

```ts
// client.ts:193–206
export function isMcpSessionExpiredError(error: Error): boolean {
  const httpStatus = 'code' in error ? (error as Error & { code?: number }).code : undefined
  if (httpStatus !== 404) return false
  return error.message.includes('"code":-32001') || error.message.includes('"code": -32001')
}
```

JSON-RPC codes used by the service:

| Code | Meaning | Source comment |
|---|---|---|
| `-32000` | `Connection closed` (MCP SDK derived after transport closes) | `client.ts:1234–1239`, `client.ts:1955–1960` |
| `-32001` | `Session not found` (HTTP 404) | `client.ts:189–192`, `client.ts:1955–1960` |
| `ErrorCode.UrlElicitationRequired` (`-32042`) | URL elicitation required | `client.ts:2862–2867` |
| `401` | OAuth/token expired (StreamableHTTPError `code` or `UnauthorizedError`) | `client.ts:1106, 1121, 1136, 3198–3208` |

Telemetry-safe wrap: any caught `Error` whose constructor name is `'Error'` is rewrapped as `TelemetrySafeError_…(message, message.slice(0, 200))`; `'McpError'` with numeric `code` is rewrapped with telemetry message `\`McpError ${code}\`` (`client.ts:1941–1967`).

Error string set (verbatim, in addition to the channel/elicitation copy above):

- `\`MCP server "${name}" connection timed out after ${getConnectionTimeoutMs()}ms\``
- `\`MCP server "${name}" tool "${tool}" timed out after ${Math.floor(timeoutMs/1000)}s\``
- `\`MCP server "${name}" requires re-authorization (token expired)\``
- `\`MCP server "${name}" session expired\``
- `\`MCP server "${name}" is not connected\`` (`ensureConnectedClient`)
- `'SDK servers should be handled in print.ts'`
- `\`Unsupported server type: ${serverRef.type}\``
- `'No claude.ai OAuth token found'`
- `\`Server "${name}" not found. Available servers: ${csv}\`` (cross-cite spec 16; thrown by `ListMcpResourcesTool` / `ReadMcpResourceTool`)
- `\`Server "${name}" is not connected\``
- `\`Server "${name}" does not support resources\``
- `\`MCP server "${name}" tool "${tool}": unexpected response format\``
- `\`URL elicitation was ${decline?'declined':action+'ed'} by a hook. The tool "${tool}" could not complete because it requires the user to open a URL.\``
- `\`URL elicitation was ${decline?'declined':action+'ed'} by the user. The tool "${tool}" could not complete because it requires the user to open a URL.\``
- `'authServerMetadataUrl must use https:// (got: ${configuredMetadataUrl})'`
- `\`HTTP ${response.status} fetching configured auth server metadata from ${configuredMetadataUrl}\``
- `\`MCP server "${name}" connection timed out after ${getConnectionTimeoutMs()}ms\``
- `'Authentication was cancelled'`

---

## 7. Side Effects & I/O

- **Filesystem:** `~/.claude/mcp-needs-auth-cache.json` (read/write); `<cwd>/.mcp.json` (atomic temp+rename, preserve mode); user/local config via `saveGlobalConfig` / `saveCurrentProjectConfig`; `getEnterpriseMcpFilePath() → managed-mcp.json` (read-only here); blob persistence via `persistBinaryContent` and `persistToolResult` for resource reads / large tool outputs.
- **Process:** `StdioClientTransport` spawns child processes; cleanup escalates SIGINT/SIGTERM/SIGKILL with 600 ms total budget; `process.kill(pid, 0)` polled to detect exit.
- **Network:** SSE/HTTP/WebSocket via SDK transports; OAuth metadata discovery (RFC 9728, RFC 8414) via `discoverOauthServerInfo` / `discoverAuthorizationServerMetadata`; revocation per RFC 7009; XAA cross-app-access exchanges per RFC 8693 + RFC 7523; auth fetch wrappers each force `Accept: application/json, text/event-stream` on POSTs.
- **Browser:** `performMCPOAuthFlow` may launch a browser via `openBrowser` (when not `skipBrowserOpen`); `McpAuthTool` always sets `skipBrowserOpen: true` (the URL is delivered to the model instead).
- **Env consumed:** `MCP_TIMEOUT`, `MCP_TOOL_TIMEOUT`, `MCP_SERVER_CONNECTION_BATCH_SIZE`, `MCP_REMOTE_SERVER_CONNECTION_BATCH_SIZE`, `CLAUDE_AGENT_SDK_MCP_NO_PREFIX`, `CLAUDE_CODE_SHELL_PREFIX`, `CLAUDE_CODE_ENABLE_XAA`, `ENABLE_MCP_LARGE_OUTPUT_FILES`, `USER_TYPE`, plus indirectly all that `subprocessEnv()` exports (spawned MCP server inherits user env).
- **External binaries:** any `command` configured per stdio server (e.g. `node`, `python`, `docker`, `npx` — Windows requires `cmd /c npx` wrapper, validation warning at `config.ts:1351–1369`).
- **Trust boundaries:**
  - Project `.mcp.json` requires interactive approval (`MCPServerApprovalDialog`) unless `enableAllProjectMcpServers` is set. Bypass-permissions and non-interactive sessions auto-approve only when `projectSettings` source is enabled and the user explicitly opted in (security comment at `utils.ts:377–404`).
  - Enterprise `managed-mcp.json` has exclusive control: when present, user/project/local/plugin/claude.ai configs are silently dropped.
  - `policySettings.allowedMcpServers` / `deniedMcpServers` filter both name- and content- (command, URL-glob) based; deny absolute precedence; `allowManagedMcpServersOnly` restricts allowlist sources.
  - `isRestrictedToPluginOnly('mcp')` blocks user/project/local — only enterprise + plugin.
  - SDK-type servers are exempt from policy gating (CLI never spawns them — see `config.ts:540–544`).
  - `claudeai-proxy` requires a logged-in claude.ai OAuth token; 401 retry forces a token refresh once.

---

## 8. Feature Flags & Variants

| Flag | Where | On | Off |
|---|---|---|---|
| `CHICAGO_MCP` | `client.ts:241,245,926,1983`; `config.ts:641,1512,1519` | `isComputerUseMCPServer(name)` import + check; in-process Computer-Use server (linked transport pair); `getComputerUseMCPToolOverrides(tool.name)` applied to the per-tool override; reserved-name block in `addMcpConfig`; `COMPUTER_USE_MCP_SERVER_NAME` becomes a default-disabled built-in (opt-in via `enabledMcpServers`). The cleanup-after-turn hook is in `query.ts:1033,1489` (owned by spec 04). | All branches stripped (`isComputerUseMCPServer === undefined`). Adding a server with the reserved name is allowed (no extra check). No default-disabled built-in. |
| `MCP_SKILLS` | `client.ts:117,1392,1670,2174,2348`; `useManageMCPConnections.ts:22,684,718,723,729` | `fetchMcpSkillsForClient` imported and runs in parallel with `fetchToolsForClient`/`fetchCommandsForClient` whenever the server has `resources`; also invalidated on `prompts/list_changed` and `resources/list_changed`; `cache.delete(name)` on `onclose` and `clearServerCache`. Skills appear inside `appState.mcp.commands` and `commands.ts:550`'s `getMcpSkillCommands` returns them. | `null` import; no skills fetched; `prompts/list_changed` only invalidates `fetchCommandsForClient`; `resources/list_changed` only invalidates `fetchResourcesForClient`. `commands.ts:550` returns `[]`. |
| `EXPERIMENTAL_SKILL_SEARCH` | `useManageMCPConnections.ts:27,693,738` | `clearSkillIndexCache()` called when MCP skills change (after list_changed on prompts/resources) | no-op |
| `KAIROS` / `KAIROS_CHANNELS` | `useManageMCPConnections.ts:172,180,473` | Channel notification handler installed on register, removed on skip; channel permission callbacks installed in AppState (additionally gated by `isChannelPermissionRelayEnabled()` GrowthBook); analytics + warning toasts | All channel-related branches dropped; servers connect normally without channel handlers |

ANT-only behavior:

- `useManageMCPConnections.ts:988–1007`: when `process.env.USER_TYPE === 'ant'`, enabled stdio servers contribute `basename(command)` to a sorted CSV emitted as `tengu_mcp_servers.stdio_commands` — used to correlate stdio MCP servers (e.g. `rust-analyzer`) with RSS/FPS metrics.
- `vscodeSdkMcp.ts:44`: `if (process.env.USER_TYPE !== 'ant' || !vscodeMcpClient) return` gates the entire VS Code SDK MCP bridge (file-update notifications, `tengu_vscode_*` events, `tengu_vscode_review_upsell` / `tengu_vscode_onboarding` Statsig gates). The producing side (IDE-bridge JWT, lockfile, auth-token format) is owned by spec 34 — this service only dispatches `file_updated` notifications when an ANT-tagged `claude-vscode` SDK client is connected.
- No other `USER_TYPE === 'ant'` branches are present in `services/mcp/`; all other ANT-vs-prod gating is via `feature(...)` flags.

---

## 9. Error Handling & Edge Cases

- **Connect timeout** (`getConnectionTimeoutMs()`): transport closed, in-process server (if any) closed, rejected with `TelemetrySafeError_…('MCP server "${name}" connection timed out after ${ms}ms', 'MCP connection timeout')`. Stats logged via `tengu_mcp_server_connection_failed`.
- **Connect 401 (sse/http/claudeai-proxy):** caught explicitly, returns `'needs-auth'`; auth-cache stamped to skip re-probing for 15 min.
- **Connect failure (other):** `'failed'` with `error: errorMessage(error)`; `tengu_mcp_server_connection_failed` includes per-transport counts and the elapsed `connectionDurationMs`.
- **Stderr noise:** captured via `'data'` listener, capped at 64 MiB to prevent unbounded growth; logged to `logMCPError` once on success or once on failure path. Listener removed in cleanup.
- **`isError: true` from `tools/call`:** translated to `McpToolCallError_…(errorDetails, 'MCP tool returned error', _meta?)`.
- **HTTP/claudeai-proxy session expiry:** detected by `404 + JSON-RPC -32001`; closes transport, rejects pending requests, clears caches; one transparent retry via `MAX_SESSION_RETRIES = 1`.
- **HTTP `-32000 'Connection closed'` after transport close:** treated as session expiry on those transports, same recovery.
- **SSE reconnect exhaustion:** SDK error `'Maximum reconnection attempts'` → `closeTransportAndRejectPending('SSE reconnection exhausted')`. Pending tool calls reject; AppState reconnect loop kicks in.
- **`MAX_ERRORS_BEFORE_RECONNECT = 3` consecutive terminal errors** (`ECONNRESET`/`ETIMEDOUT`/`EPIPE`/`EHOSTUNREACH`/`ECONNREFUSED`/`Body Timeout Error`/`terminated`/`SSE stream disconnected`/`Failed to reconnect SSE stream`) → close.
- **Reconnection (remote only):** 5 attempts; non-remote (`stdio`, `sdk`) immediately `failed`. Cancelled by toggle/disable, manual reconnect, plugin-reload, unmount.
- **Tool timeout (`MCP_TOOL_TIMEOUT` || ~27.8 h):** cooperative race; the SDK's `timeout` is also passed (`callTool` `timeout: timeoutMs`).
- **AbortError** (Ctrl+C): `callMCPTool` returns `{content: undefined}` — no error logspew. The outer loop in `MCPTool.call` does not re-throw.
- **Telemetry-safe wrapping:** any `Error` whose constructor name is `'Error'` is rewrapped to avoid leaking arbitrary messages into telemetry; `'McpError'` is rewrapped with code suffix.
- **Approval ESC:** rejects (single) or rejects-all (multi). `pending.length === 0` early returns without rendering anything.
- **Stale plugin servers / config edits:** `excludeStalePluginClients` removes from AppState; cleanup is fire-and-forget but guarded — only `'connected'` clients trigger `clearServerCache` to avoid spinning up a real connect just to tear it down (comment at `useManageMCPConnections.ts:797–812`).
- **OAuth concurrency:** `mcp-needs-auth-cache.json` writes are serialized via a single `writeChain` promise. The `claudeAiProxy` 401 retry passes the *exact* token sent (not a re-read) into `handleOAuth401Error` to avoid a same-as-keychain false negative.
- **Step-up auth (403):** `wrapFetchWithStepUpDetection` (auth.ts:1354) intercepts 403 with `WWW-Authenticate` `error="insufficient_scope"`, persists the requested `scope`, and triggers re-auth.
- **DCR / token refresh failure:** `revokeServerTokens` is best-effort; local tokens are cleared regardless. `preserveStepUpState` keeps `scope` + `discoveryState` so the next consent flow doesn't re-probe.

---

## 10. Telemetry & Observability

Logging primitives (from `utils/log.js`): `logMCPDebug(serverName, message)` — `--debug` only; `logMCPError(serverName, errorOrString)` — always.

Analytics (`logEvent`, owned by spec 26). Every event the service emits:

> **Redaction policy (defer to spec 09):** all string values cast into `tengu_mcp_*` event payloads — including `mcpServerBaseUrl`, `transportType`, `serverVersion`, `failureStage`, `entry_kind`, `type`, `outcome`, `reason`, plus the `serverName` strings inside `TelemetrySafeError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` — are wrapped via the `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` cast (~25 occurrences across `client.ts` and `auth.ts`, e.g. `auth.ts:828, 838, 880, 1245, 1323`; `client.ts:177, 1061, 1698, 2702, 2746`). The `_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` marker is the type-system declaration that the field has been audited as redaction-safe. **Spec 09 owns the redaction contract**; spec 23 inherits it verbatim.


| Event | Where | Fields |
|---|---|---|
| `tengu_mcp_servers` | `useManageMCPConnections.ts:997` (after both phases load) | `enterprise, global, project, user, plugin, claudeai`; ANT-only `stdio_commands` (sorted CSV of stdio command basenames) |
| `tengu_mcp_server_connection_succeeded` | `client.ts:1583` | `connectionDurationMs, transportType, totalServers, stdioCount, sseCount, httpCount, sseIdeCount, wsIdeCount, mcpServerBaseUrl?` |
| `tengu_mcp_server_connection_failed` | `client.ts:1607` | same as above |
| `tengu_mcp_server_needs_auth` | `client.ts:345` | `transportType ∈ {'sse','http','claudeai-proxy'}, mcpServerBaseUrl?` |
| `tengu_mcp_ide_server_connection_succeeded` / `_failed` | `client.ts:1143, 1201` | `connectionDurationMs, serverVersion?` |
| `tengu_mcp_tools_commands_loaded` | `client.ts:2448` | `tools_count, commands_count, commands_metadata_length` |
| `tengu_mcp_list_changed` | `useManageMCPConnections.ts:638, 651, 675, 713` | `type ∈ {'tools','prompts','resources'}, previousCount?, newCount?`. **No separate `type:'skills'` event:** when `MCP_SKILLS` is on, skill cache is invalidated alongside `prompts/list_changed` and `resources/list_changed` (see §8) but no distinct skills-typed event is emitted — `useManageMCPConnections.ts:638,651,675,713` only ever set `type` to one of the three above. |
| `tengu_mcp_elicitation_shown` / `_response` | `elicitationHandler.ts:85, 101, 140` | `mode ∈ {'form','url'}, action?` |
| `tengu_mcp_large_result_handled` | `client.ts:2742, 2762, 2779, 2786` | `outcome, reason, sizeEstimateTokens, persistedSizeChars?` |
| `tengu_mcp_tool_call_auth_error` | `client.ts:3203` | `{}` |
| `tengu_mcp_session_expired` | `client.ts:3228` | `{}` |
| `tengu_mcp_claudeai_proxy_401` | `client.ts:403` | `tokenChanged` |
| `tengu_mcp_dialog_choice` / `tengu_mcp_multidialog_choice` | dialog components | `choice ∈ {'yes','yes_all','no'}` / `{approved, rejected}` |
| `tengu_mcp_channel_gate` / `tengu_mcp_channel_message` | `useManageMCPConnections.ts:493, 515` | `registered, skip_kind, entry_kind, is_dev, plugin?, content_length, meta_key_count?` |
| `tengu_mcp_oauth_flow_start` / `_success` / `_failure` / `tengu_mcp_oauth_refresh_failure` | `auth.ts` (owned by spec 25 for full schema; emitted on the MCP service path) | per `auth.ts` |
| `tengu_builtin_mcp_toggle` | `config.ts:1572` | `serverName, enabled` (only when default-disabled built-in changes state) |
| `tengu_code_indexing_tool_used` | `client.ts:3163` | `tool, source:'mcp', success` (when `detectCodeIndexingFromMcpServerName(name)` returns a tool) |

Heartbeat: every 30 s during `callMCPTool`, `logMCPDebug` emits `Tool '${tool}' still running (${seconds}s elapsed)`.

---

## 11. Reimplementation Checklist

A bit-exact reimplementation must preserve:

- The exact transport dispatch order in §5.2, including the in-process branches for `claude-in-chrome` (always-on) and Computer-Use (`CHICAGO_MCP` flag).
- `client.connect` capability bag `{roots:{}, elicitation:{}}` — no `{form:{},url:{}}` payload (Spring AI / Java SDK refuses unknown elicitation properties).
- `setRequestHandler(ListRootsRequestSchema, ...)` returning `[{uri:\`file://${getOriginalCwd()}\`}]` exactly.
- A no-op `{action:'cancel'}` elicitation handler that runs *before* the real handler is registered post-`onConnectionAttempt`.
- The 60 s `MCP_REQUEST_TIMEOUT_MS` POST timeout but **no** GET timeout (long-lived SSE stream); always-set `Accept: application/json, text/event-stream` on POSTs.
- 30 s connection timeout (`MCP_TIMEOUT`); 27.8 h tool timeout default (`MCP_TOOL_TIMEOUT`); 30 s OAuth-request timeout (`AUTH_REQUEST_TIMEOUT_MS`).
- `MAX_SESSION_RETRIES = 1` for `McpSessionExpiredError` only; **no** retry for any other error class.
- `MAX_URL_ELICITATION_RETRIES = 3`; URL elicitations validated for `mode: 'url'` + string `url`/`elicitationId`/`message`.
- Exponential reconnect backoff: 1, 2, 4, 8, 16 s capped at 30 s; 5 attempts max; only for remote transports; cancellable; honours mid-attempt disable.
- Connection-cache memo key is `${name}-${jsonStringify(serverRef)}` (full config including scope is part of the key for `connectToServer` cache; **scope-stripped hash** for `hashMcpConfig` change detection).
- LRU=20 fetch caches keyed by `client.name` (stable across reconnects); invalidated by `onclose`, list-changed notifications, and `clearServerCache`.
- Per-tool override of `name`, `mcpInfo`, `isMcp:true`, `searchHint`, `alwaysLoad`, `description/prompt`, `isConcurrencySafe/isReadOnly/isDestructive/isOpenWorld`, `inputJSONSchema`, `checkPermissions`, `call`, `userFacingName`, `toAutoClassifierInput`, plus the Chrome- and Computer-Use overrides when applicable.
- `mcpInfo` always present even in `CLAUDE_AGENT_SDK_MCP_NO_PREFIX` skip-prefix mode (only valid when `config.type === 'sdk'`).
- Approval-dialog choice strings + persistence via `enabledMcpjsonServers` / `disabledMcpjsonServers` / `enableAllProjectMcpServers` in `localSettings`.
- `getProjectMcpServerStatus` decision tree exact order; `hasSkipDangerousModePermissionPrompt` MUST be read from non-`projectSettings` sources only.
- pMap concurrency split: 3 (local) and 20 (remote) with env overrides.
- `processBatched = pMap` (not the legacy fixed-batch sequential loop).
- Stdio cleanup escalation budget (100/400/500/600 ms) and SIGINT→SIGTERM→SIGKILL order.
- Stderr 64 MiB cap; redact `Authorization` headers in all log lines.
- MCP description / instructions truncation at `MAX_MCP_DESCRIPTION_LENGTH = 2048` with suffix `'… [truncated]'`.
- 15-min `mcp-needs-auth-cache.json` TTL; in-memory `authCachePromise` cleared on every write; `clearMcpAuthCache()` unlinks the file (best-effort).
- `unwrapCcrProxyUrl` triggers on path markers `'/v2/session_ingress/shttp/mcp/'`, `'/v2/ccr-sessions/'`.
- Dedup signature scheme (stdio: cmd+args; remote: unwrapped URL; sdk: `null`).
- Plugin-MCP key namespace `plugin:<plugin>:<serverName>` and the disabled/policy-blocked split in `getClaudeCodeMcpConfigs`.
- Enterprise exclusivity: `doesEnterpriseMcpConfigExist()` short-circuits all other sources.
- `feature('CHICAGO_MCP')` and `feature('MCP_SKILLS')` deltas exactly as in §8.
- `isMcpSessionExpiredError`: 404 + substring match on `'"code":-32001'` or `'"code": -32001'`.
- Telemetry-safe error wrapping rules (constructor-name based, with `McpError` code preservation).
- Channel-blocked toast strings and the `register/skip` action dispatch.
- The exact analytics event names and payload keys in §10.

---

## 12. Open Questions / Unknowns

1. **`channelNotification.ts` / `channelPermissions.ts` / `channelAllowlist.ts` shapes.** This spec describes the call sites (`useManageMCPConnections.ts`) but defers the channel state machine + `gateChannelServer` decision tree to spec 32 (`KAIROS*` modes) — they'll need a cross-cite once 32 is filed. Filed here so 32's owner sees the dependency.
2. **`SdkControlClientTransport` wire format.** `setupSdkMcpClients` is in scope; the underlying transport's per-message routing is summarized but the `sendMcpMessage` callback contract is owned by `entrypoints/sdk` (spec 01). When 35 (remote-server / SDK V2) is filed, that spec should formalize the message envelope.
3. **`mcpSkills.ts` (`src/skills/`).** The MCP_SKILLS gate references `fetchMcpSkillsForClient` and `clearSkillIndexCache`; their implementations live under `src/skills/` (spec 17). This spec captures the call sites only; resource-URI scheme `skill://` and the skill→`Command` mapping are owned by 17.
4. **`vscodeSdkMcp.ts` and `bridge/`.** The IDE-side bridge that surfaces `sse-ide` / `ws-ide` (auth-token format, lockfile location, JWT) is partially seen here (`X-Claude-Code-Ide-Authorization`, `serverRef.authToken`, `ideRunningInWindows`) but the producing side belongs in **spec 34**. `vscodeSdkMcp.ts` itself is **ANT-only** (gated by `process.env.USER_TYPE === 'ant'` at `vscodeSdkMcp.ts:44`) and fires `tengu_vscode_*` events / reads `tengu_vscode_review_upsell` and `tengu_vscode_onboarding` Statsig gates — also owned by 34, not 25.
5. **`claudeai.ts` (claude.ai connectors).** `markClaudeAiMcpConnected`, `clearClaudeAIMcpConfigsCache`, `fetchClaudeAIMcpConfigsIfEligible`, and `dedupClaudeAiMcpServers` are exercised here. The remote-fetch HTTP details belong adjacent to spec 25 (OAuth).
6. **`McpAuthOutput` user-visible strings.** The pseudo-tool's `unsupported`/`auth_url`/`error` template wording is owned by spec 16 §3.4. This spec records only the side effects (`clearMcpAuthCache`, `reconnectMcpServerImpl`, AppState merge).
7. **`auth.ts:700+` (XAA, lock-file, refresh).** Sampled for the public surface only. Full XAA sequence (RFC 8693 + RFC 7523), DCR client registration, refresh-lock semantics, and `revokeServerTokens` decision tree are owned by spec 25.
8. **`MCPServerDialogCopy.tsx` body text.** Verbatim copy not inlined here (file was sampled by grep). Spec 37 (UI shell) should record it once filed; it does not change MCP behavior.
