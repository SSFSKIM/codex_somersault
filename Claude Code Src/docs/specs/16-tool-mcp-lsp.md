# 16 — MCP & LSP Tools

> Multi-tool spec covering the **MCP family** (`MCPTool`, `ListMcpResourcesTool`, `ReadMcpResourceTool`, `McpAuthTool`) and `LSPTool`. Per-tool sub-headings are nested under each canonical section.
>
> Adjacent specs (do not redocument): **08** (registry & ToolDef), **09** (permissions), **23** (MCP server lifecycle, transports, OAuth, callMCPToolWithUrlElicitationRetry), **24** (LSP server manager), **28** (plugin manifest loading), **37** (UI shell). This spec cites their public APIs but does not re-derive them.

---

## 1. Purpose & Scope

This spec specifies the in-process tool surface used to expose two external "open-world" subsystems to the model:

1. **MCP (Model Context Protocol)** — third-party servers that ship arbitrary tools/resources/prompts. Claude Code surfaces them as a *family* of tools:
   - `MCPTool` — a *template* tool whose fields are mostly overridden per-tool by `mcpClient.ts`. It is never registered directly; instead `fetchToolsForClient` clones it once per remote MCP tool and rewrites `name`, `description`, `prompt`, `inputJSONSchema`, `call`, `userFacingName`, `isConcurrencySafe`, `isReadOnly`, `isDestructive`, `isOpenWorld`, `searchHint`, `alwaysLoad`, `toAutoClassifierInput`, `isSearchOrReadCommand`, `mcpInfo`, `checkPermissions` (`tools/MCPTool/MCPTool.ts:27-77`, `services/mcp/client.ts:1743-1995`).
   - `ListMcpResourcesTool` — list MCP `resources` across all (or one) connected servers (`tools/ListMcpResourcesTool/ListMcpResourcesTool.ts:40-123`).
   - `ReadMcpResourceTool` — read one resource by `(server, uri)`, persist binary blobs to disk (`tools/ReadMcpResourceTool/ReadMcpResourceTool.ts:49-158`).
   - `McpAuthTool` — *pseudo-tool* synthesized by `createMcpAuthTool(serverName, config)` for installed-but-unauthenticated servers (`tools/McpAuthTool/McpAuthTool.ts:49-215`). Replaces the server's missing real tools until OAuth completes; uses prefix `mcp__<server>__authenticate`.
2. **LSP (Language Server Protocol)** — `LSPTool` exposes nine LSP operations behind a single discriminated-union schema (`tools/LSPTool/LSPTool.ts:127-422`). Gated by `process.env.ENABLE_LSP_TOOL` (`tools.ts:224`); registered only when truthy.

**In scope.** Per-tool input/output schemas, prompts, permission behavior, render functions, the MCP naming policy (`mcp__server__tool` vs unprefixed under `CLAUDE_AGENT_SDK_MCP_NO_PREFIX`), the `mcpInfo` shape, `MCP_RICH_OUTPUT` rendering variants, MCP UI heuristics for unwrap/flatten/Slack-send, classification of MCP tools as search/read for collapse, blob persistence on resource read, OAuth start-and-swap pattern in `McpAuthTool`, and LSP-specific behaviors: 1-based↔0-based conversion, two-step call-hierarchy, gitignore filtering, file-size guard, formatter output strings, symbol-at-position UI hint.

**Out of scope.** Permission framework (→ 09), tool registry & `buildTool` semantics (→ 08), MCP transports/auth flow internals/`callMCPToolWithUrlElicitationRetry`/`ensureConnectedClient`/`fetchResourcesForClient`/`reconnectMcpServerImpl`/elicitation (→ 23), LSP server manager / `getLspServerManager`/`waitForInitialization`/`isLspConnected`/`openFile`/`sendRequest`/`isFileOpen` (→ 24), plugin-loaded MCP servers (→ 28), shell rendering primitives (→ 37). This spec **cites** those APIs by signature only.

---

## 2. Source Map

### 2.1 In-scope files (bit-exact targets)

| Path | Lines | Purpose |
|---|---|---|
| `src/tools/MCPTool/MCPTool.ts` | 1-77 | Template `ToolDef`. All overridable methods set to placeholders. |
| `src/tools/MCPTool/prompt.ts` | 1-4 | `PROMPT = ''` and `DESCRIPTION = ''` (overridden in client). |
| `src/tools/MCPTool/UI.tsx` | 1-403 | `renderToolUseMessage` / `renderToolUseProgressMessage` / `renderToolResultMessage` + `MCP_RICH_OUTPUT` rich path (`MCPTextOutput`, `tryUnwrapTextPayload`, `tryFlattenJson`, `trySlackSendCompact`). |
| `src/tools/MCPTool/classifyForCollapse.ts` | 1-605 | `classifyMcpToolForCollapse(serverName, toolName) → { isSearch, isRead }`. Two static allowlists keyed on snake-cased tool names. |
| `src/tools/ListMcpResourcesTool/ListMcpResourcesTool.ts` | 1-123 | Tool definition. |
| `src/tools/ListMcpResourcesTool/prompt.ts` | 1-21 | Constants + DESCRIPTION + PROMPT. |
| `src/tools/ListMcpResourcesTool/UI.tsx` | 1-28 | renderToolUseMessage / renderToolResultMessage (JSON pretty-print fallback to `(No resources found)`). |
| `src/tools/ReadMcpResourceTool/ReadMcpResourceTool.ts` | 1-158 | Tool definition with binary-blob persistence. |
| `src/tools/ReadMcpResourceTool/prompt.ts` | 1-16 | DESCRIPTION + PROMPT. |
| `src/tools/ReadMcpResourceTool/UI.tsx` | 1-36 | renderToolUseMessage / userFacingName / renderToolResultMessage. |
| `src/tools/McpAuthTool/McpAuthTool.ts` | 1-215 | `createMcpAuthTool(serverName, config)` factory; not registered globally. |
| `src/tools/LSPTool/LSPTool.ts` | 1-860 | `LSPTool` build, schemas (compat + dispatch via `lspToolInputSchema`), file-existence + UNC bypass, two-step call hierarchy, gitignore filter via `git check-ignore` batches of 50, formatting dispatch. |
| `src/tools/LSPTool/prompt.ts` | 1-21 | `LSP_TOOL_NAME = 'LSP'`, full DESCRIPTION. |
| `src/tools/LSPTool/schemas.ts` | 1-216 | `lspToolInputSchema` (Zod discriminated union with 9 variants) + `isValidLSPOperation` type guard. |
| `src/tools/LSPTool/UI.tsx` | 1-227 | OPERATION_LABELS + LSPResultSummary (collapsed Ctrl+O / verbose tree) + renderToolUseMessage with symbol context + renderToolUseErrorMessage. |
| `src/tools/LSPTool/formatters.ts` | 1-593 | All operation-specific result→string formatters (and SymbolKind→string mapping). |
| `src/tools/LSPTool/symbolContext.ts` | 1-91 | Sync `getSymbolAtPosition(filePath, line, character)`. Reads first 64 KiB only. Symbol regex `/[\w$'!]+|[+\-*/%&|^~<>=]+/g`. |

### 2.2 Cross-spec references (cited, not redocumented)

- Naming policy primitives — `services/mcp/mcpStringUtils.ts:39-67` (`getMcpPrefix`, `buildMcpToolName`, `getToolNameForPermissionCheck`, `mcpInfoFromString`, `getMcpDisplayName`, `extractMcpToolDisplayName`).
- Per-tool override loop — `services/mcp/client.ts:1743-1995` (`fetchToolsForClient`).
- MCP description truncation constant — `services/mcp/client.ts:218` (`MAX_MCP_DESCRIPTION_LENGTH = 2048`).
- IDE allowlist constant — `services/mcp/client.ts:568` (`ALLOWED_IDE_TOOLS = ['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']`).
- Skip-prefix gate — `services/mcp/client.ts:1760-1773` (`CLAUDE_AGENT_SDK_MCP_NO_PREFIX` only takes effect when `client.config.type === 'sdk'`).
- Registry conditionals — `tools.ts:224` (`ENABLE_LSP_TOOL`), `tools.ts:245-246` (Mcp resource tools), `tools.ts:301-307` (special-tool filter set).
- ToolDef discriminator fields — `Tool.ts:436-455` (`isMcp`, `isLsp`, `mcpInfo`, `alwaysLoad`).

### 2.3 Source-coverage inventory (pre-write)

Tracked files: 17. Tracked behaviors and where this spec inlines them:

| Asset | Tool | Where in this spec |
|---|---|---|
| Full prompt | MCPTool | §6.1 |
| Full prompt | ListMcpResourcesTool | §6.2 |
| Full prompt | ReadMcpResourceTool | §6.3 |
| Full prompt | McpAuthTool | §6.4 (template form) |
| Full prompt | LSPTool | §6.5 |
| Full input Zod | All five | §6.6–§6.10 |
| Full output Zod | LSPTool, List, Read | §6.6, §6.8, §6.9 |
| MCP_RICH_OUTPUT delta | MCPTool | §6.11 |
| Error strings | All | §6.12 |
| Constants table | All | §6.13 |
| Search/Read collapse allowlists | MCPTool | referenced (too long to inline; see §3.6 for invariant) |

---

## 3. Public Interface (Contract)

### 3.1 MCPTool (template)

```ts
buildTool({
  isMcp: true,
  isOpenWorld() { return false },          // overridden
  name: 'mcp',                              // overridden to mcp__<server>__<tool>
  maxResultSizeChars: 100_000,
  description() { return DESCRIPTION },     // overridden
  prompt()      { return PROMPT },          // overridden
  inputSchema:   z.object({}).passthrough(),  // overridden via inputJSONSchema
  outputSchema:  z.string().describe('MCP tool execution result'),
  call() { return { data: '' } },           // overridden
  checkPermissions(): { behavior: 'passthrough', message: 'MCPTool requires permission.' },
  renderToolUseMessage, renderToolUseProgressMessage, renderToolResultMessage,
  userFacingName: () => 'mcp',              // overridden
  isResultTruncated(output): boolean        // = isOutputLineTruncated(output)
  mapToolResultToToolResultBlockParam(content, toolUseID) { … }
})
```
(`tools/MCPTool/MCPTool.ts:27-77`)

The fields actually shipped per-call come from `fetchToolsForClient` (`services/mcp/client.ts:1766-1989`); this spec documents the **template** and the **client-side override contract**, not the discovery loop.

**`mcpInfo` invariant.** Always `{ serverName, toolName }` using *unnormalized* names as received from the server (`Tool.ts:451-455`, `client.ts:1774`). Used both as the discriminator (`isMcp === true || name.startsWith('mcp__')`, `services/mcp/utils.ts:246`) and as the canonical permission-check key (`getToolNameForPermissionCheck`, `mcpStringUtils.ts:60-67`).

### 3.2 ListMcpResourcesTool

- `name = 'ListMcpResourcesTool'`, `userFacingName = 'listMcpResources'`.
- `searchHint = 'list resources from connected MCP servers'`.
- `shouldDefer = true` (always loaded into the deferred-tool list, never sent in the initial prompt).
- `isConcurrencySafe = true`, `isReadOnly = true`.
- `maxResultSizeChars = 100_000`.
- Input: `{ server?: string }`.
- Output: `Array<{ uri, name, mimeType?, description?, server }>`.
- `toAutoClassifierInput(input) = input.server ?? ''`.
- Empty result is mapped to a placeholder string: `'No resources found. MCP servers may still provide tools even if they have no resources.'` (`ListMcpResourcesTool.ts:108-122`).

### 3.3 ReadMcpResourceTool

- `name = 'ReadMcpResourceTool'`, `userFacingName = 'readMcpResource'`.
- `searchHint = 'read a specific MCP resource by URI'`.
- `shouldDefer = true`, `isConcurrencySafe = true`, `isReadOnly = true`, `maxResultSizeChars = 100_000`.
- Input: `{ server: string, uri: string }`.
- Output: `{ contents: Array<{ uri: string, mimeType?: string, text?: string, blobSavedTo?: string }> }`.
- Binary blob handling: any `blob` field is base64-decoded, persisted via `persistBinaryContent` to a path with id `mcp-resource-${Date.now()}-${i}-${rand6}`; the entry is rewritten to expose `blobSavedTo` plus a human-readable text marker (`getBinaryBlobSavedMessage(filepath, mimeType, size, "[Resource from <server> at <uri>] ")`). On persistence error, only `text: \`Binary content could not be saved to disk: ${err}\`` is emitted (`ReadMcpResourceTool.ts:106-138`).
- `toAutoClassifierInput(input) = \`${input.server} ${input.uri}\``.

### 3.4 McpAuthTool (factory)

`createMcpAuthTool(serverName, config: ScopedMcpServerConfig) → Tool<{}, McpAuthOutput>`.

- `McpAuthOutput = { status: 'auth_url' | 'unsupported' | 'error', message: string, authUrl?: string }`.
- `name = buildMcpToolName(serverName, 'authenticate')` → `mcp__<server>__authenticate`.
- `mcpInfo = { serverName, toolName: 'authenticate' }`.
- `userFacingName = \`${serverName} - authenticate (MCP)\``.
- `maxResultSizeChars = 10_000` (MCPTool default 100_000).
- `isMcp = true`, `isConcurrencySafe = false`, `isReadOnly = false`.
- `checkPermissions(input) → { behavior: 'allow', updatedInput: input }` (no prompt — this is the *gate* to gain real tools).
- `toAutoClassifierInput() = serverName`.
- `renderToolUseMessage()` → `'Authenticate <serverName> MCP server'`.

**Branching** (`McpAuthTool.ts:85-205`):
- `config.type === 'claudeai-proxy'` → returns `unsupported`, points the user at `/mcp`.
- `config.type !∈ {'sse','http'}` → returns `unsupported` with the same redirect.
- Otherwise: starts `performMCPOAuthFlow(serverName, sseOrHttpConfig, onAuthorizationUrl, signal, { skipBrowserOpen: true })`. **Races** capture-of-URL against flow completion; if URL arrives first, returns `auth_url` with the URL embedded in the message. Background continuation: on flow success runs `clearMcpAuthCache()` then `reconnectMcpServerImpl(serverName, config)` then mutates `appState.mcp` via `setAppState` — replacing any tool/command starting with `getMcpPrefix(serverName)` and merging in `result.tools`, `result.commands`, and per-server `result.resources`. The pseudo-tool is removed automatically because its name shares the prefix.

### 3.5 LSPTool

- `name = 'LSP'`, `userFacingName = 'LSP'`, `searchHint = 'code intelligence (definitions, references, symbols, hover)'`.
- `isLsp = true`, `isReadOnly = true`, `isConcurrencySafe = true`, `shouldDefer = true`.
- `maxResultSizeChars = 100_000`. File size cap `MAX_LSP_FILE_SIZE_BYTES = 10_000_000`.
- `isEnabled() = isLspConnected()` (delegates to LSP service — spec 24).
- `getPath({ filePath }) = expandPath(filePath)` (used by permission filter to scope read rules to a specific path).
- `checkPermissions` delegates to `checkReadPermissionForTool(LSPTool, input, appState.toolPermissionContext)` (spec 09).
- Input/Output schemas: §6.7. Validation runs the discriminated-union variant (`lspToolInputSchema().safeParse`) for type quality, then verifies the file exists and is a regular file. UNC paths (`startsWith('\\\\')` or `'//'`) skip filesystem access entirely (NTLM-leak hardening) and short-circuit to `{ result: true }`.
- `validateInput` error codes: `1 = ENOENT`, `2 = not a regular file`, `3 = invalid input (zod)`, `4 = stat failure (other)`. (`LSPTool.ts:158-208`.)

### 3.6 MCP tool naming policy

- **Default** (`mcpInfo` set, name normalized): `mcp__<normalized-server>__<normalized-tool>`. `getMcpPrefix(server) = mcp__${normalizeNameForMCP(server)}__` (`mcpStringUtils.ts:39-41`); `buildMcpToolName(server, tool) = ${getMcpPrefix(server)}${normalizeNameForMCP(tool)}` (`mcpStringUtils.ts:50-52`).
- **Skip-prefix mode**: `client.config.type === 'sdk' && isEnvTruthy(process.env.CLAUDE_AGENT_SDK_MCP_NO_PREFIX)` causes `name = tool.name` (raw), but **`mcpInfo` is still set**. Comment at `services/mcp/client.ts:1771-1773`: *"In skip-prefix mode, use the original name for model invocation so MCP tools can override builtins by name. mcpInfo is used for permission checking."* This means an unprefixed MCP tool can shadow a builtin by name; permission rules should target `getToolNameForPermissionCheck(tool)`, which returns the prefixed form via `mcpInfo`.
- **MCP discriminator** (used across the codebase): `tool.name?.startsWith('mcp__') || tool.isMcp === true` (`services/mcp/utils.ts:246`). Both checks needed because of skip-prefix mode.
- **IDE allowlist**: `ALLOWED_IDE_TOOLS = ['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']` filters `mcp__ide__*` to those two (`services/mcp/client.ts:568-571`).
- **Search/read collapse**: `classifyMcpToolForCollapse(serverName, toolName) → { isSearch, isRead }` matches against two large *static* allowlists keyed on the normalized tool name only (snake_case, lowercased — `normalize: replace ([a-z])([A-Z]) → $1_$2; replace - → _; toLowerCase()`). `_serverName` is intentionally unused; comment: *"Tool names are stable across installs (even when the server name varies, e.g., 'slack' vs 'claude_ai_Slack')"* (`classifyForCollapse.ts:1-11, 588-604`). Unknown names return both `false` (conservative non-collapse). Allowlists are too long to inline; full enumerations are at `classifyForCollapse.ts:14-139` (SEARCH) and `:142-586` (READ).

---

## 4. Data Model & State

### 4.1 MCPTool runtime data

- **Per-call state.** None held in tool itself; `parentMessage` provides `toolUseId`; `meta` becomes `{ 'claudecode/toolUseId': toolUseId }` and is forwarded to MCP `tools/call` (`client.ts:1840-1843`). Retry state: `MAX_SESSION_RETRIES = 1`; only `McpSessionExpiredError` triggers retry.
- **Result shape**. Either (a) raw `string`, or (b) `MCPToolResult` array (`{ type: 'text', text } | { type: 'image', … } | …`). Plus optional `mcpMeta = { _meta?, structuredContent? }` returned alongside `data` (`client.ts:1898-1908`).
- **Progress events.** `MCPProgress` discriminated union with `status ∈ { 'started', 'completed', 'failed', other }` plus `serverName`, `toolName`, optional `elapsedTimeMs`, optional `progress`/`total`/`progressMessage` for in-flight ticks. Re-exported via `tools/MCPTool/MCPTool.ts:24-25` from `types/tools.ts`.
- **Telemetry-safe error wrapping.** Any non-`TelemetrySafeError_…` `Error` whose constructor is `'Error'` gets re-thrown as `TelemetrySafeError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS(message, message.slice(0,200))`; `'McpError'` with numeric `code` is re-thrown with code prefix `\`McpError ${error.code}\`` (`client.ts:1941-1967`). JSON-RPC error codes called out in comment: `-32000 ConnectionClosed`, `-32001 RequestTimeout`.

### 4.2 ListMcpResourcesTool

- Reads `context.options.mcpClients`, filters by name if `input.server` is set; throws `Server "<name>" not found. Available servers: <csv>` if filter is non-empty and matches nothing.
- Per client: skips when `client.type !== 'connected'`. Calls `ensureConnectedClient(client)` (no-op when healthy, fresh connect after `onclose`). Then `fetchResourcesForClient(fresh)` — LRU-cached by server name, invalidated by `onclose` and `resources/list_changed` (per comment at `ListMcpResourcesTool.ts:79-83`).
- Per-server failure isolated via try/catch → `[]`; one server's reconnect failure does **not** sink the aggregate.
- `isResultTruncated = isOutputLineTruncated(jsonStringify(output))`.

### 4.3 ReadMcpResourceTool

- Throws (caller turns it into a `tool_use_error`) when:
  - server name not found: `Server "<name>" not found. Available servers: <csv>`,
  - server present but `client.type !== 'connected'`: `Server "<name>" is not connected`,
  - server lacks `capabilities.resources`: `Server "<name>" does not support resources`.
- Calls MCP `resources/read` with `ReadResourceResultSchema`.
- Persists each `blob` content with id pattern `mcp-resource-<epochMs>-<index>-<rand36×6>`.

### 4.4 McpAuthTool

- Local closures: `resolveAuthUrl?: (url) => void`, `authUrlPromise: Promise<string>`, `controller = new AbortController()`.
- Background side-effects mutate `appState.mcp` after OAuth completes; uses lodash-es `reject` to remove anything starting with `getMcpPrefix(serverName)` from `tools` and `commands` before merging in the new arrays. `resources` becomes `{ ...prev.mcp.resources, [serverName]: result.resources }` only if `result.resources` is set.
- `oauthPromise.catch(err)` logs via `logMCPError(serverName, ...)` but does not propagate.

### 4.5 LSPTool

- Per-call: `absolutePath = expandPath(input.filePath)`; `cwd = getCwd()`; `manager = getLspServerManager()`. Awaits `waitForInitialization()` if `getInitializationStatus().status === 'pending'`.
- File-open invariant: if `!manager.isFileOpen(absolutePath)`, opens via `fs/promises open()`, stats, refuses if size > 10 MB (returns `File too large for LSP analysis (<N>MB exceeds 10MB limit)`), reads UTF-8, calls `manager.openFile(absolutePath, fileContent)`, then closes the handle.
- LSP method dispatch (`getMethodAndParams`, `LSPTool.ts:427-513`) — table:

| `operation` | LSP method | params shape |
|---|---|---|
| `goToDefinition` | `textDocument/definition` | `{ textDocument: { uri }, position }` |
| `findReferences` | `textDocument/references` | `{ textDocument, position, context: { includeDeclaration: true } }` |
| `hover` | `textDocument/hover` | `{ textDocument, position }` |
| `documentSymbol` | `textDocument/documentSymbol` | `{ textDocument }` |
| `workspaceSymbol` | `workspace/symbol` | `{ query: '' }` (always empty — returns all) |
| `goToImplementation` | `textDocument/implementation` | `{ textDocument, position }` |
| `prepareCallHierarchy` | `textDocument/prepareCallHierarchy` | `{ textDocument, position }` |
| `incomingCalls` | `textDocument/prepareCallHierarchy` (step 1) → `callHierarchy/incomingCalls` (step 2 with `{ item: callItems[0] }`) | — |
| `outgoingCalls` | same, then `callHierarchy/outgoingCalls` | — |

- Position conversion: `position = { line: input.line - 1, character: input.character - 1 }` (1-based input, 0-based wire).
- URI: `pathToFileURL(absolutePath).href`.
- **Gitignore filtering**: applies *only* to `findReferences`, `goToDefinition`, `goToImplementation`, `workspaceSymbol`. Uses `git check-ignore` in batches of 50 paths with 5 s timeout, `preserveOutputOnError: false`. Exit code 0 + stdout → ignored set; cwd-rooted (`LSPTool.ts:556-611`). For `workspaceSymbol`, `SymbolInformation[]` is filtered by URIs of remaining `Location`s; otherwise `Location | LocationLink` items are checked via `toLocation()` adapter.
- `LocationLink` adapter: `{ uri: targetUri, range: targetSelectionRange ?? targetRange }` (`LSPTool.ts:622-631`).

---

## 5. Algorithm / Control Flow

### 5.1 MCPTool.call (per-tool, post-override)

Pseudocode:
```
toolUseId = extractToolUseId(parentMessage)
meta = toolUseId ? { 'claudecode/toolUseId': toolUseId } : {}
emit progress(status='started', serverName, toolName) if onProgress && toolUseId
startTime = now()
for attempt in 0,1:
  try:
    connectedClient = await ensureConnectedClient(client)
    mcpResult = await callMCPToolWithUrlElicitationRetry({client:connectedClient, clientConnection:client,
                  tool:tool.name, args, meta, signal:context.abortController.signal,
                  setAppState, onProgress: progress→relay-with-toolUseID, handleElicitation })
    emit progress(status='completed', elapsedTimeMs)
    return { data: mcpResult.content,
             ...(mcpResult._meta || mcpResult.structuredContent
                  ? { mcpMeta: { ...(_meta && {_meta}), ...(structuredContent && {structuredContent}) } }
                  : {}) }
  catch error:
    if McpSessionExpiredError && attempt < MAX_SESSION_RETRIES (=1): logMCPDebug; continue
    emit progress(status='failed', elapsedTimeMs)
    wrap-into-TelemetrySafeError if (error.constructor.name === 'Error' || 'McpError'+numeric code)
    throw
```

### 5.2 LSPTool.call

```
absolutePath = expandPath(input.filePath); cwd = getCwd()
if getInitializationStatus().status == 'pending': await waitForInitialization()
manager = getLspServerManager()
if !manager: log + return Output{ result:'LSP server manager not initialized…', operation, filePath }

(method, params) = getMethodAndParams(input, absolutePath)

if !manager.isFileOpen(absolutePath):
  handle = await fs/promises.open(absolutePath, 'r')
  try:
    if (await handle.stat()).size > 10MB: return Output{ result:'File too large…', … }
    fileContent = await handle.readFile('utf-8')
    await manager.openFile(absolutePath, fileContent)
  finally: handle.close()

result = await manager.sendRequest(absolutePath, method, params)
if result === undefined: log + return Output{ result:`No LSP server available for file type: ${ext}`, … }

if input.operation in {incomingCalls, outgoingCalls}:
  callItems = result as CallHierarchyItem[]
  if !callItems || empty: return Output{ result:'No call hierarchy item found at this position', resultCount:0, fileCount:0 }
  callMethod = operation==='incomingCalls' ? 'callHierarchy/incomingCalls' : 'callHierarchy/outgoingCalls'
  result = await manager.sendRequest(absolutePath, callMethod, { item: callItems[0] })
  // result==undefined falls through to formatter for graceful empty handling

if Array.isArray(result) and operation in {findReferences, goToDefinition, goToImplementation, workspaceSymbol}:
  apply filterGitIgnoredLocations(locations, cwd) — see §4.5

(formatted, resultCount, fileCount) = formatResult(operation, result, cwd)
return Output{ operation, result: formatted, filePath, resultCount, fileCount }
```

`mapToolResultToToolResultBlockParam(output, toolUseID)` returns `{ tool_use_id, type:'tool_result', content: output.result }` — i.e. only the `formatted` string is sent to the model; `resultCount`/`fileCount` are UI-only.

### 5.3 LSPTool symbol-context UI augmentation

In `renderToolUseMessage(input, { verbose })` (`LSPTool/UI.tsx:163-199`):
- For position-based ops (`goToDefinition | findReferences | hover | goToImplementation`) with all three of `filePath`, `line`, `character` defined:
  - Convert from 1-based to 0-based and call `getSymbolAtPosition(filePath, line-1, character-1)` (sync, reads first 64 KiB).
  - If symbol found: render `operation: "<op>", symbol: "<symbol>", in: "<displayPath>"` (verbose mode keeps full path; otherwise `getDisplayPath`).
  - Else: `operation: "<op>", file: "<displayPath>", position: <line>:<char>`.
- For `documentSymbol`/`workspaceSymbol`: `operation: "<op>"` plus optional `file: "<displayPath>"`.

`getSymbolAtPosition` reads at most `MAX_READ_BYTES = 64 * 1024`. If `bytesRead === MAX_READ_BYTES` and the requested line is the last index of the split, returns `null` (last line may be truncated mid-line). Symbol regex matches: identifiers (`\w$'!`), and operators (`+\-*/%&|^~<>=`). Result truncated to 30 chars.

### 5.4 LSPTool result-summary rendering

`renderToolResultMessage(output, _, { verbose })` (`LSPTool/UI.tsx:212-226`):
- If `output.resultCount !== undefined && output.fileCount !== undefined`: delegate to `LSPResultSummary` (collapsed view with `<CtrlOToExpand />` when `resultCount > 0`; verbose view shows the full `output.result` indented under a `⎿` glyph).
- Else: fallback `<MessageResponse><Text>{output.result}</Text></MessageResponse>`.

OPERATION_LABELS (singular/plural; hover has `special: 'available'` rendered as *Hover info available*):

| operation | singular | plural | special |
|---|---|---|---|
| goToDefinition | definition | definitions | — |
| findReferences | reference | references | — |
| documentSymbol | symbol | symbols | — |
| workspaceSymbol | symbol | symbols | — |
| hover | hover info | hover info | available |
| goToImplementation | implementation | implementations | — |
| prepareCallHierarchy | call item | call items | — |
| incomingCalls | caller | callers | — |
| outgoingCalls | callee | callees | — |

### 5.5 LSPTool error message rendering

`renderToolUseErrorMessage(result, { verbose })` (`LSPTool/UI.tsx:200-211`): if `!verbose && typeof result === 'string' && extractTag(result, 'tool_use_error')` → renders `<Text color="error">LSP operation failed</Text>`. Otherwise delegates to `<FallbackToolUseErrorMessage />`.

### 5.6 MCPTool rendering pipeline (always-on path)

`renderToolUseMessage(input, { verbose })`: `Object.keys(input).length === 0` → empty string. Otherwise comma-joined `${key}: ${jsonStringify(value)}`. Under `MCP_RICH_OUTPUT && !verbose`, each rendered value > 80 chars is truncated to `slice(0, 80).trimEnd() + '…'` (`MCPTool/UI.tsx:46-56`).

`renderToolUseProgressMessage`: pulls `progressMessagesForMessage.at(-1)`. If absent or `progress === undefined` → `<Text dimColor>Running…</Text>`. With `total > 0` → `<ProgressBar ratio width={20} />` plus rounded `${percentage}%`. Otherwise → `progressMessage ?? \`Processing… ${progress}\``.

`renderToolResultMessage(output, _, { verbose, input })`:
1. `!verbose` → try `trySlackSendCompact(output, input)`. If non-null, render single-line `Sent a message to <Ansi>{createHyperlink(url, channel)}</Ansi>`.
2. `estimatedTokens = getContentSizeEstimate(mcpOutput)`; if `> 10_000` (`MCP_OUTPUT_WARNING_THRESHOLD_TOKENS`) prepend warning row: `${figures.warning} Large MCP response (~<formatNumber(tokens)> tokens), this can fill up context quickly`.
3. Branch by output shape:
   - `Array.isArray(mcpOutput)`: per item, `image` blocks → `<Text>[Image]</Text>` placeholder; text/other blocks → either `<MCPTextOutput …>` (rich) or `<OutputLine …>` (plain).
   - falsy → `<Text dimColor>(No content)</Text>`.
   - else (string) → rich vs plain.

### 5.7 MCP_RICH_OUTPUT delta (the only feature-flag variant in scope)

Cited at `tools/MCPTool/UI.tsx:51, 125, 139` (`feature('MCP_RICH_OUTPUT')`). When **off**, `OutputLine(content, verbose)` is used everywhere and `renderToolUseMessage` does **not** truncate values to 80 chars. When **on**:

1. **`renderToolUseMessage`** truncates each value to `MAX_INPUT_VALUE_CHARS = 80` (suffix `'…'`) when `!verbose`.
2. **`MCPTextOutput`** is used in place of `OutputLine` for text blocks. It tries three strategies in order (`MCPTool/UI.tsx:159-252, 267-362`):
   1. **Unwrap-text-payload** (`tryUnwrapTextPayload`): JSON object with up to 4 keys, ≤ 200_000 chars total, exactly one *dominant* string (length > `UNWRAP_MIN_STRING_LEN = 200` OR contains `\n` and length > 50) and small scalar siblings — render `<MessageResponse><Box flexDirection="column">{extras dim-joined by ' · '}{<OutputLine content=body verbose linkifyUrls />}</Box></MessageResponse>`. Reject if two big strings, or any sibling string > 150 chars, or any nested object/array.
   2. **Flatten-JSON** (`tryFlattenJson`): JSON object, ≤ `MAX_FLAT_JSON_CHARS = 5_000`, ≤ `MAX_FLAT_JSON_KEYS = 12`, every value is scalar or a tiny nested object whose `jsonStringify(value).length ≤ 120`. Render as `key: value` rows aligned to `maxKeyWidth`, value passed through `linkifyUrlsInText` inside `<Ansi>`.
   3. **Fallthrough**: `<OutputLine content verbose linkifyUrls />`.
3. Slack short-circuit (always on, but uses `linkifyUrlsInText` style only under rich path) hits before token-warning logic when `!verbose` (`MCPTool/UI.tsx:99-109`).

Rich vs plain are otherwise identical (warning header, image placeholder, `(No content)` fallback). Constants: `MCP_OUTPUT_WARNING_THRESHOLD_TOKENS = 10_000`; `MAX_INPUT_VALUE_CHARS = 80`; `MAX_FLAT_JSON_KEYS = 12`; `MAX_FLAT_JSON_CHARS = 5_000`; `MAX_JSON_PARSE_CHARS = 200_000`; `UNWRAP_MIN_STRING_LEN = 200`. Slack archives regex: `/^https:\/\/[a-z0-9-]+\.slack\.com\/archives\/([A-Z0-9]+)\/p\d+$/`.

### 5.8 ListMcpResourcesTool.call

```
{ server: targetServer } = input
clientsToProcess = targetServer ? mcpClients.filter(c => c.name === targetServer) : mcpClients
if targetServer && empty: throw `Server "${targetServer}" not found. Available servers: ${csv}`
results = await Promise.all(clientsToProcess.map(async client => {
  if client.type !== 'connected' return []
  try: fresh = await ensureConnectedClient(client); return await fetchResourcesForClient(fresh)
  except: logMCPError(client.name, err); return []
}))
return { data: results.flat() }
```

### 5.9 ReadMcpResourceTool.call

```
client = mcpClients.find(c => c.name === serverName)
if !client: throw `Server "${serverName}" not found. Available servers: ${csv}`
if client.type !== 'connected': throw `Server "${serverName}" is not connected`
if !client.capabilities?.resources: throw `Server "${serverName}" does not support resources`
connectedClient = await ensureConnectedClient(client)
result = await connectedClient.client.request({ method:'resources/read', params:{ uri }}, ReadResourceResultSchema)
contents = await Promise.all(result.contents.map(async (c, i) => {
  if 'text' in c: return { uri:c.uri, mimeType:c.mimeType, text:c.text }
  if !('blob' in c) || typeof c.blob !== 'string': return { uri:c.uri, mimeType:c.mimeType }
  persistId = `mcp-resource-${Date.now()}-${i}-${rand6}`
  persisted = await persistBinaryContent(Buffer.from(c.blob, 'base64'), c.mimeType, persistId)
  if 'error' in persisted: return { uri, mimeType, text: `Binary content could not be saved to disk: ${persisted.error}` }
  return { uri, mimeType, blobSavedTo: persisted.filepath,
           text: getBinaryBlobSavedMessage(filepath, mimeType, size, `[Resource from ${serverName} at ${c.uri}] `) }
}))
return { data: { contents } }
```

### 5.10 McpAuthTool.call

See §3.4 for branching. Pseudocode for the OAuth path:
```
authUrlPromise = new Promise<string>(res => resolveAuthUrl = res)
controller = new AbortController()
oauthPromise = performMCPOAuthFlow(serverName, sseOrHttpConfig, u => resolveAuthUrl?.(u), controller.signal, { skipBrowserOpen: true })
// Background: do NOT await before returning
oauthPromise.then(async () => {
  clearMcpAuthCache()
  result = await reconnectMcpServerImpl(serverName, config)
  prefix = getMcpPrefix(serverName)
  setAppState(prev => ({ ...prev, mcp: { ...prev.mcp,
    clients:  prev.mcp.clients.map(c => c.name === serverName ? result.client : c),
    tools:    [...reject(prev.mcp.tools,    t => t.name?.startsWith(prefix)), ...result.tools],
    commands: [...reject(prev.mcp.commands, c => c.name?.startsWith(prefix)), ...result.commands],
    resources: result.resources ? { ...prev.mcp.resources, [serverName]: result.resources } : prev.mcp.resources,
  }}))
  logMCPDebug(serverName, `OAuth complete, reconnected with ${result.tools.length} tool(s)`)
}).catch(err => logMCPError(serverName, `OAuth flow failed after tool-triggered start: ${errorMessage(err)}`))
try:
  authUrl = await Promise.race([authUrlPromise, oauthPromise.then(() => null)])
  if authUrl: return { data: { status: 'auth_url', authUrl, message: <see §6.12> }}
  return { data: { status: 'auth_url', message: 'Authentication completed silently for <server>. The server\'s tools should now be available.' }}
catch err:
  return { data: { status: 'error', message: 'Failed to start OAuth flow for <server>: <err>. Ask the user to run /mcp and authenticate manually.' }}
```

Note (`McpAuthTool.ts:175-180`): the *silent* path is described as occurring *e.g. XAA with cached IdP token*.

---

## 6. Verbatim Assets

### 6.1 MCPTool prompt + description

```ts
// src/tools/MCPTool/prompt.ts:1-4
// Actual prompt and description are overridden in mcpClient.ts
export const PROMPT = ''
export const DESCRIPTION = ''
```
Per-tool description at runtime is `tool.description ?? ''` from the MCP server, capped to `MAX_MCP_DESCRIPTION_LENGTH = 2048` chars (suffix `'… [truncated]'`) inside `prompt()` (`services/mcp/client.ts:1789-1794`). `searchHint` comes from `tool._meta['anthropic/searchHint']` (whitespace collapsed, `.trim() || undefined`); `alwaysLoad = tool._meta['anthropic/alwaysLoad'] === true`.

### 6.2 ListMcpResourcesTool prompts

```ts
// src/tools/ListMcpResourcesTool/prompt.ts
export const LIST_MCP_RESOURCES_TOOL_NAME = 'ListMcpResourcesTool'

export const DESCRIPTION = `
Lists available resources from configured MCP servers.
Each resource object includes a 'server' field indicating which server it's from.

Usage examples:
- List all resources from all servers: \`listMcpResources\`
- List resources from a specific server: \`listMcpResources({ server: "myserver" })\`
`

export const PROMPT = `
List available resources from configured MCP servers.
Each returned resource will include all standard MCP resource fields plus a 'server' field 
indicating which server the resource belongs to.

Parameters:
- server (optional): The name of a specific MCP server to get resources from. If not provided,
  resources from all servers will be returned.
`
```

### 6.3 ReadMcpResourceTool prompts

```ts
// src/tools/ReadMcpResourceTool/prompt.ts
export const DESCRIPTION = `
Reads a specific resource from an MCP server.
- server: The name of the MCP server to read from
- uri: The URI of the resource to read

Usage examples:
- Read a resource from a server: \`readMcpResource({ server: "myserver", uri: "my-resource-uri" })\`
`

export const PROMPT = `
Reads a specific resource from an MCP server, identified by server name and resource URI.

Parameters:
- server (required): The name of the MCP server from which to read the resource
- uri (required): The URI of the resource to read
`
```

### 6.4 McpAuthTool description (template)

`description` and `prompt` are computed per-server (`McpAuthTool.ts:53-77`):

```ts
const url = getConfigUrl(config)
const transport = config.type ?? 'stdio'
const location = url ? `${transport} at ${url}` : transport
const description =
  `The \`${serverName}\` MCP server (${location}) is installed but requires authentication. ` +
  `Call this tool to start the OAuth flow — you'll receive an authorization URL to share with the user. ` +
  `Once the user completes authorization in their browser, the server's real tools will become available automatically.`
```

### 6.5 LSPTool prompt

```ts
// src/tools/LSPTool/prompt.ts
export const LSP_TOOL_NAME = 'LSP' as const

export const DESCRIPTION = `Interact with Language Server Protocol (LSP) servers to get code intelligence features.

Supported operations:
- goToDefinition: Find where a symbol is defined
- findReferences: Find all references to a symbol
- hover: Get hover information (documentation, type info) for a symbol
- documentSymbol: Get all symbols (functions, classes, variables) in a document
- workspaceSymbol: Search for symbols across the entire workspace
- goToImplementation: Find implementations of an interface or abstract method
- prepareCallHierarchy: Get call hierarchy item at a position (functions/methods)
- incomingCalls: Find all functions/methods that call the function at a position
- outgoingCalls: Find all functions/methods called by the function at a position

All operations require:
- filePath: The file to operate on
- line: The line number (1-based, as shown in editors)
- character: The character offset (1-based, as shown in editors)

Note: LSP servers must be configured for the file type. If no server is available, an error will be returned.`
```
`prompt()` returns `DESCRIPTION` (`LSPTool.ts:218-220`).

### 6.6 MCPTool input/output schemas

```ts
// src/tools/MCPTool/MCPTool.ts:13-22
export const inputSchema  = lazySchema(() => z.object({}).passthrough())
export const outputSchema = lazySchema(() => z.string().describe('MCP tool execution result'))
```
At runtime, `inputJSONSchema` is overwritten with `tool.inputSchema as Tool['inputJSONSchema']` from the MCP server (`client.ts:1813`).

### 6.7 LSPTool schemas

**Compatibility input** (`LSPTool.ts:59-86`):
```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    operation: z.enum([
      'goToDefinition','findReferences','hover','documentSymbol','workspaceSymbol',
      'goToImplementation','prepareCallHierarchy','incomingCalls','outgoingCalls',
    ]).describe('The LSP operation to perform'),
    filePath: z.string().describe('The absolute or relative path to the file'),
    line: z.number().int().positive().describe('The line number (1-based, as shown in editors)'),
    character: z.number().int().positive().describe('The character offset (1-based, as shown in editors)'),
  }),
)
```

**Validation input** (discriminated union, `LSPTool/schemas.ts:8-191`): each of the 9 operations has a `z.strictObject` with `operation: z.literal(<op>)`, `filePath: z.string()`, `line: z.number().int().positive()`, `character: z.number().int().positive()` — same shape across all 9. Combined via `z.discriminatedUnion('operation', [goToDefinitionSchema, findReferencesSchema, hoverSchema, documentSymbolSchema, workspaceSymbolSchema, goToImplementationSchema, prepareCallHierarchySchema, incomingCallsSchema, outgoingCallsSchema])`.

**Output** (`LSPTool.ts:89-122`):
```ts
const outputSchema = lazySchema(() =>
  z.object({
    operation: z.enum([...same 9 strings...]).describe('The LSP operation that was performed'),
    result: z.string().describe('The formatted result of the LSP operation'),
    filePath: z.string().describe('The file path the operation was performed on'),
    resultCount: z.number().int().nonnegative().optional().describe('Number of results (definitions, references, symbols)'),
    fileCount:   z.number().int().nonnegative().optional().describe('Number of files containing results'),
  }),
)
```

**Type guard** (`schemas.ts:201-215`): `isValidLSPOperation(operation: string)` — returns true iff in the same 9-tuple array.

### 6.8 ListMcpResourcesTool schemas

```ts
// src/tools/ListMcpResourcesTool/ListMcpResourcesTool.ts:15-36
const inputSchema = lazySchema(() =>
  z.object({
    server: z.string().optional().describe('Optional server name to filter resources by'),
  }),
)

const outputSchema = lazySchema(() =>
  z.array(
    z.object({
      uri: z.string().describe('Resource URI'),
      name: z.string().describe('Resource name'),
      mimeType: z.string().optional().describe('MIME type of the resource'),
      description: z.string().optional().describe('Resource description'),
      server: z.string().describe('Server that provides this resource'),
    }),
  ),
)
```

### 6.9 ReadMcpResourceTool schemas

```ts
// src/tools/ReadMcpResourceTool/ReadMcpResourceTool.ts:22-45
export const inputSchema = lazySchema(() =>
  z.object({
    server: z.string().describe('The MCP server name'),
    uri:    z.string().describe('The resource URI to read'),
  }),
)

export const outputSchema = lazySchema(() =>
  z.object({
    contents: z.array(
      z.object({
        uri: z.string().describe('Resource URI'),
        mimeType: z.string().optional().describe('MIME type of the content'),
        text:     z.string().optional().describe('Text content of the resource'),
        blobSavedTo: z.string().optional().describe('Path where binary blob content was saved'),
      }),
    ),
  }),
)
```

### 6.10 McpAuthTool input schema

```ts
// src/tools/McpAuthTool/McpAuthTool.ts:23-24
const inputSchema = lazySchema(() => z.object({}))
```
Output type: `McpAuthOutput = { status: 'auth_url' | 'unsupported' | 'error'; message: string; authUrl?: string }`.

### 6.11 MCP_RICH_OUTPUT UI variant deltas

Three feature-gated branch points in `tools/MCPTool/UI.tsx`:

| Line | Off | On |
|---|---|---|
| 51 | (no truncation in tool-use header) | truncate value > 80 chars to `slice(0, 80).trimEnd() + '…'` when `!verbose` |
| 125 | `<OutputLine content={textContent} verbose />` for each text block | `<MCPTextOutput content verbose />` (3-strategy unwrap → flatten → fallthrough) |
| 139 | `<OutputLine content={mcpOutput} verbose />` for non-array string output | `<MCPTextOutput content verbose />` |

Strategy precedence inside `MCPTextOutput`: (1) `tryUnwrapTextPayload`; (2) `tryFlattenJson`; (3) `<OutputLine linkifyUrls />`. Slack send compaction is upstream of MCP_RICH_OUTPUT (always tried when `!verbose`).

### 6.12 User-facing error / status strings

**LSPTool** (formatters.ts + LSPTool.ts):
- `'No definition found. This may occur if the cursor is not on a symbol, or if the definition is in an external library not indexed by the LSP server.'`
- `'No references found. This may occur if the symbol has no usages, or if the LSP server has not fully indexed the workspace.'`
- `'No hover information available. This may occur if the cursor is not on a symbol, or if the LSP server has not fully indexed the file.'`
- `'No symbols found in document. This may occur if the file is empty, not supported by the LSP server, or if the server has not fully indexed the file.'`
- `'No symbols found in workspace. This may occur if the workspace is empty, or if the LSP server has not finished indexing the project.'`
- `'No call hierarchy item found at this position'`
- `'No incoming calls found (nothing calls this function)'`
- `'No outgoing calls found (this function calls nothing)'`
- `'<unknown location>'` (URI fallback in formatters.ts)
- Found-N templates: `Defined in <path>:<line>:<col>`, `Found <n> definitions:\n  <list>`, `Found 1 reference:\n  <loc>`, `Found <n> references across <m> files:`, `Hover info at <line>:<col>:\n\n<content>`, `Document symbols:`, `Found <n> symbol(s) in workspace:`, `Call hierarchy item: <item>`, `Found <n> call hierarchy items:`, `Found <n> incoming call(s):`, `Found <n> outgoing call(s):` (plural via `plural()` helper).
- LSPTool.ts errors: `'LSP server manager not initialized. This may indicate a startup issue.'`, `'File too large for LSP analysis (<N>MB exceeds 10MB limit)'`, `'No LSP server available for file type: <ext>'`, `'No call hierarchy item found at this position'` (also returned from LSPTool.ts), `'Error performing <operation>: <errorMessage>'`. ValidateInput: `'Invalid input: <zod.error.message>'`, `'File does not exist: <filePath>'`, `'Cannot access file: <filePath>. <err.message>'`, `'Path is not a file: <filePath>'`. UI error: `'LSP operation failed'` (collapsed) or fallback component (verbose).
- LSPTool result-summary text fragments: `'Found '`, `' across '`, `' files'`, `'Hover info available'`.

**ListMcpResourcesTool**: throws `\`Server "${targetServer}" not found. Available servers: ${csv}\``. Empty-result `tool_result` content: `'No resources found. MCP servers may still provide tools even if they have no resources.'`. UI placeholder: `'(No resources found)'`.

**ReadMcpResourceTool**: throws `\`Server "${serverName}" not found. Available servers: ${csv}\``, `\`Server "${serverName}" is not connected\``, `\`Server "${serverName}" does not support resources\``. Per-blob fallback text: `\`Binary content could not be saved to disk: ${err}\``. Successful blob text: rendered via `getBinaryBlobSavedMessage(filepath, mimeType, size, \`[Resource from ${serverName} at ${c.uri}] \`)`. UI placeholder: `'(No content)'`.

**McpAuthTool**:
- Claude.ai-proxy unsupported: `\`This is a claude.ai MCP connector. Ask the user to run /mcp and select "${serverName}" to authenticate.\``.
- Other-transport unsupported: `\`Server "${serverName}" uses ${transport} transport which does not support OAuth from this tool. Ask the user to run /mcp and authenticate manually.\``.
- Got URL: `\`Ask the user to open this URL in their browser to authorize the ${serverName} MCP server:\n\n${authUrl}\n\nOnce they complete the flow, the server's tools will become available automatically.\``.
- Silent success: `\`Authentication completed silently for ${serverName}. The server's tools should now be available.\``.
- Error: `\`Failed to start OAuth flow for ${serverName}: ${errorMessage(err)}. Ask the user to run /mcp and authenticate manually.\``.

**MCPTool runtime**: `'MCPTool requires permission.'` (template + override both); permission suggestion `{ type:'addRules', rules:[{ toolName: fullyQualifiedName, ruleContent: undefined }], behavior:'allow', destination:'localSettings' }` (`client.ts:1814-1832`). Large-output warning: `\`${figures.warning} Large MCP response (~${formatNumber(tokens)} tokens), this can fill up context quickly\``. UI placeholders: `'Running…'`, `'Processing… <progress>'`, `'(No content)'`, `'[Image]'`. Slack: `<Text>Sent a message to <Ansi>{createHyperlink(url, channel)}</Ansi></Text>`. UserFacingName: `\`${client.name} - ${tool.annotations?.title ?? tool.name} (MCP)\`` (`client.ts:1972-1976`); McpAuthTool variant: `\`${serverName} - authenticate (MCP)\``. Description-truncation suffix: `'… [truncated]'`.

### 6.13 Constants table

| Constant | Value | Where | Purpose |
|---|---|---|---|
| `MAX_MCP_DESCRIPTION_LENGTH` | `2048` | `services/mcp/client.ts:218` | MCP per-tool prompt cap |
| `MCP_OUTPUT_WARNING_THRESHOLD_TOKENS` | `10_000` | `tools/MCPTool/UI.tsx:21` | UI warning threshold |
| `MAX_INPUT_VALUE_CHARS` | `80` | `tools/MCPTool/UI.tsx:26` | Header value truncation under MCP_RICH_OUTPUT |
| `MAX_FLAT_JSON_KEYS` | `12` | `tools/MCPTool/UI.tsx:30` | Flat-JSON UI bound |
| `MAX_FLAT_JSON_CHARS` | `5_000` | `tools/MCPTool/UI.tsx:33` | Flat-JSON UI bound |
| `MAX_JSON_PARSE_CHARS` | `200_000` | `tools/MCPTool/UI.tsx:36` | Parse safety bound |
| `UNWRAP_MIN_STRING_LEN` | `200` | `tools/MCPTool/UI.tsx:40` | Dominant-payload threshold |
| `MAX_SESSION_RETRIES` | `1` | `services/mcp/client.ts:1859` | MCP session-retry cap |
| `MCP_FETCH_CACHE_SIZE` | `20` | `services/mcp/client.ts:1726` | LRU cap (per-server) |
| `MAX_LSP_FILE_SIZE_BYTES` | `10_000_000` | `tools/LSPTool/LSPTool.ts:53` | LSP file cap (10 MB) |
| `BATCH_SIZE` (gitignore) | `50` | `tools/LSPTool/LSPTool.ts:580` | `git check-ignore` batch |
| `git check-ignore` timeout | `5_000` ms | `tools/LSPTool/LSPTool.ts:589` | per-batch timeout |
| `MAX_READ_BYTES` (symbol context) | `64 * 1024` | `tools/LSPTool/symbolContext.ts:6` | UI sync read cap |
| `maxResultSizeChars` (MCPTool) | `100_000` | `tools/MCPTool/MCPTool.ts:35` | persist-to-disk threshold |
| `maxResultSizeChars` (List/Read/LSP) | `100_000` | each tool file | same |
| `maxResultSizeChars` (McpAuthTool) | `10_000` | `tools/McpAuthTool/McpAuthTool.ts:71` | low because messages are short |
| Symbol char cap (UI) | `30` | `symbolContext.ts:73` (`truncate(symbol, 30)`) | UI hint length |
| Slack archives regex | `/^https:\/\/[a-z0-9-]+\.slack\.com\/archives\/([A-Z0-9]+)\/p\d+$/` | `MCPTool/UI.tsx:363` | Slack-send detection |
| MCP prefix template | `mcp__${normalizeNameForMCP(server)}__` | `mcpStringUtils.ts:39-41` | naming policy |
| IDE allowlist | `['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']` | `client.ts:568` | filter `mcp__ide__*` to two |
| LSP tool name | `'LSP'` | `LSPTool/prompt.ts:1` | registry name |
| `LIST_MCP_RESOURCES_TOOL_NAME` | `'ListMcpResourcesTool'` | `ListMcpResourcesTool/prompt.ts:1` | registry name |

### 6.14 Env variables

| Variable | Type | Effect | Where |
|---|---|---|---|
| `ENABLE_LSP_TOOL` | `isEnvTruthy` | If truthy, `LSPTool` is appended to base tool list | `tools.ts:224` |
| `CLAUDE_AGENT_SDK_MCP_NO_PREFIX` | `isEnvTruthy` | Combined with `client.config.type === 'sdk'`, MCP tool `name` becomes raw (unprefixed); `mcpInfo` still set so permission checks use the prefixed form | `services/mcp/client.ts:1760-1773` |

`MCP_RICH_OUTPUT` is a **bundle-time** flag (`feature('MCP_RICH_OUTPUT')`), not env. Three sites listed in §6.11.

---

## 7. Side Effects & I/O

- **Filesystem.** `LSPTool.call` opens, stats, and reads files via `fs/promises.open` (UTF-8, full body) when LSP needs them — gated by `manager.isFileOpen(absolutePath)`. `LSPTool.validateInput` calls `getFsImplementation().stat`. `getSymbolAtPosition` does a synchronous read of the first 64 KiB. `ReadMcpResourceTool` writes binary blobs via `persistBinaryContent` to a path produced by `mcpOutputStorage` (out of scope here).
- **Subprocess.** `LSPTool` invokes `git check-ignore` via `execFileNoThrowWithCwd('git', ['check-ignore', ...batch], { cwd, preserveOutputOnError:false, timeout:5_000 })` per batch of 50.
- **Network.** All MCP I/O is mediated by the MCP SDK client (`spec 23`). MCPTool itself does not open sockets; per-call MCP requests go through `connectedClient.client.request(...)` (Read) or `callMCPToolWithUrlElicitationRetry({...})` (MCPTool override).
- **State mutation.** `McpAuthTool` mutates `appState.mcp` (clients/tools/commands/resources) via the `setAppState` capability injected on `context`. `MCPTool` override may call `context.handleElicitation` (spec 23) and `context.setAppState` indirectly through `callMCPToolWithUrlElicitationRetry`.
- **Logging.** `logMCPDebug`, `logMCPError`, `logForDebugging`, `logError` (`LSPTool` writes diagnostic errors for malformed LSP responses, undefined URIs, and stat failures).
- **Process env.** Read-only consumption of `ENABLE_LSP_TOOL` and `CLAUDE_AGENT_SDK_MCP_NO_PREFIX`.

---

## 8. Feature Flags & Variants

| Flag | Type | Where | Variant |
|---|---|---|---|
| `MCP_RICH_OUTPUT` | bundle (`feature(...)`) | `tools/MCPTool/UI.tsx:51, 125, 139` | Enables `MCPTextOutput` (unwrap/flatten) and 80-char header value truncation. See §5.7, §6.11. |
| `process.env.ENABLE_LSP_TOOL` | env truthy | `tools.ts:224` | Controls registration of `LSPTool`. When falsy, the tool is absent from `getAllBaseTools()`; `isEnabled()` (which checks `isLspConnected()`) is the second gate. |
| `process.env.CLAUDE_AGENT_SDK_MCP_NO_PREFIX` | env truthy + `client.config.type === 'sdk'` | `services/mcp/client.ts:1760-1773` | Switches MCP tool naming from `mcp__server__tool` to raw `tool.name` (only for SDK servers). `mcpInfo` is preserved so permissions still use the prefixed key. |
| `feature('CHICAGO_MCP')` | bundle | `services/mcp/client.ts:1983-1987` | Computer-use MCP overrides applied (server-specific). Out of scope here — body of overrides lives elsewhere. |
| `isClaudeInChromeMCPServer(client.name)` (computed) | runtime | `services/mcp/client.ts:1977-1982` | Per-server tool-rendering overrides for Claude-in-Chrome MCP. |

---

## 9. Error Handling & Edge Cases

- **MCPTool / per-tool `call`**: `McpSessionExpiredError` triggers exactly **one** retry (`MAX_SESSION_RETRIES = 1`). Other errors are wrapped (`TelemetrySafeError_…`) when their constructor name is `'Error'` or `'McpError'` with numeric `code` — original message kept; classification short-string used for telemetry. Comments document JSON-RPC codes `-32000 ConnectionClosed`, `-32001 RequestTimeout`.
- **MCPTool UI**: `(No content)` for falsy output; `[Image]` for image blocks; large-output warning at >10k tokens. `MCPTextOutput` falls all the way through to `OutputLine` if no JSON heuristic matches.
- **ListMcpResourcesTool**: target-server-not-found → throw with available list. Per-server reconnect failure swallowed (logged) and treated as `[]`. `(No resources found)` UI placeholder when output is empty/falsy.
- **ReadMcpResourceTool**: three named throws (server-not-found / not-connected / no-resource-capability). Blob persistence failure folded into the per-content `text` payload, never bubbles out. UI `(No content)` if `output.contents` empty.
- **McpAuthTool**: `claudeai-proxy` and non-sse/non-http transports return `unsupported` rather than throwing. OAuth race may resolve with `null` (silent success). Background continuation `.catch(...)` swallows + logs.
- **LSPTool validateInput**: skips fs ops for UNC (`\\\\` or `//`). On `ENOENT` returns `errorCode:1`. Other stat errors logged + `errorCode:4`. Non-file stat → `errorCode:2`. Zod failure → `errorCode:3`.
- **LSPTool call**: missing `manager` → human-readable result, no throw. `result === undefined` from sendRequest → human-readable "No LSP server available for file type" message. Top-level `try/catch` wraps any exception into `\`Error performing ${operation}: ${err.message}\``.
- **LSPTool formatters**: defensive filtering of locations/symbols with undefined `uri` (via `logForDebugging`); empty results return human-readable strings (see §6.12). `formatUri` decodes percent-encoded paths, strips Windows `/C:/` leading slash, falls back to un-decoded path on `decodeURIComponent` throw.
- **LSPTool gitignore**: shells `git check-ignore`; exit code 128 (not a git repo) means "no ignored paths" (no entries added to set). Batches of 50 prevent argv overflow.

---

## 10. Telemetry & Observability

- MCPTool emits `MCPProgress` events (`status: 'started' | 'completed' | 'failed' | …`) tagged with `serverName`, `toolName`, and (on terminal) `elapsedTimeMs`. `meta` carrying `claudecode/toolUseId` is propagated to the MCP server on every `tools/call`.
- LSPTool logs LSP-related anomalies via `logForDebugging` (level `'warn'`) and `logError`: `"No LSP server available for file type … for operation … on file …"`, `"LSP server returned undefined for <method> on <file>"`, `"LSP server returned <n> location(s) with undefined URI for <op> on <cwd>"`, `"LSP server manager not initialized when tool was called"`, `"LSP tool request failed for <op> on <file>: <msg>"`, `"Failed to access file stats for LSP operation on <file>: <msg>"`, plus formatter-side warnings for malformed `from`/`to` fields on call hierarchy entries.
- `getSymbolAtPosition` logs sync-read failures (`Symbol extraction failed for <file>:<line>:<char>: <msg>`) at warn level — this is a UI nicety only and does not bubble.
- ListMcpResourcesTool: per-server failure logged via `logMCPError(client.name, errorMessage)`.
- McpAuthTool: `logMCPDebug` on OAuth completion (`OAuth complete, reconnected with N tool(s)`); `logMCPError` on flow failure (`OAuth flow failed after tool-triggered start: <msg>`).

---

## 11. Reimplementation Checklist

A reviewer porting this spec must verify all of:

1. [ ] `MCPTool` is a *template* — registered tools are clones produced by `fetchToolsForClient`; the unmodified template is never registered for direct invocation.
2. [ ] `mcpInfo: { serverName, toolName }` is set on every MCP-derived tool, including unprefixed (skip-prefix) tools and `McpAuthTool` clones.
3. [ ] Default name = `mcp__${normalizeNameForMCP(server)}__${normalizeNameForMCP(tool)}`. Skip-prefix mode (`client.config.type === 'sdk' && CLAUDE_AGENT_SDK_MCP_NO_PREFIX` truthy) leaves name = raw `tool.name`.
4. [ ] `mcp__ide__*` tools filtered to the two-element `ALLOWED_IDE_TOOLS` allowlist.
5. [ ] MCP description capped at `MAX_MCP_DESCRIPTION_LENGTH = 2048` chars with suffix `'… [truncated]'`.
6. [ ] Per-tool `searchHint` from `_meta['anthropic/searchHint']` (whitespace collapsed, empty→undefined). `alwaysLoad` from `_meta['anthropic/alwaysLoad'] === true`.
7. [ ] Per-tool `isConcurrencySafe` / `isReadOnly` mirror `tool.annotations?.readOnlyHint ?? false`; `isDestructive` from `destructiveHint ?? false`; `isOpenWorld` from `openWorldHint ?? false`.
8. [ ] `MCPTool.call` retries exactly once on `McpSessionExpiredError`. Other `Error`s with constructor `'Error'` or `'McpError'` get wrapped with telemetry-safe codes.
9. [ ] Progress events emitted on start, completion (with `elapsedTimeMs`), and failure (with `elapsedTimeMs`).
10. [ ] `MCPTool` UI fallbacks: `(No content)` for falsy, `[Image]` for image blocks, warning at > 10k tokens.
11. [ ] `MCP_RICH_OUTPUT` adds: 80-char header truncation in `renderToolUseMessage`, and `MCPTextOutput` (unwrap → flatten → fallthrough) replacing `OutputLine` for text content.
12. [ ] Slack send compaction precedes both warning and content rendering when `!verbose`.
13. [ ] `classifyMcpToolForCollapse` matches normalized tool name only (snake_case + lowercase), independent of server. Unknown → both false.
14. [ ] `ListMcpResourcesTool`: `shouldDefer = true`; per-client failures isolated; empty → fixed string `'No resources found. MCP servers may still provide tools even if they have no resources.'`.
15. [ ] `ReadMcpResourceTool`: throws three specific server-state errors; binary blobs persisted as `mcp-resource-<epoch>-<i>-<rand6>` and replaced with `blobSavedTo` + human marker.
16. [ ] `McpAuthTool`: `name = mcp__<server>__authenticate`; `checkPermissions → allow`; URL emitted via race vs flow completion; on completion `clearMcpAuthCache → reconnectMcpServerImpl → setAppState` (replace prefix-matching tools/commands, add resources). Background errors logged but never thrown.
17. [ ] `LSPTool`: gated behind `ENABLE_LSP_TOOL` (env) AND `isLspConnected()` (runtime via `isEnabled`).
18. [ ] LSP input is 1-based; `position` sent to LSP is `(line-1, character-1)`.
19. [ ] LSP file-size cap = 10 MB (`MAX_LSP_FILE_SIZE_BYTES`). UNC paths skip fs in `validateInput`.
20. [ ] LSP `incomingCalls`/`outgoingCalls` are two-step: `prepareCallHierarchy` → `callHierarchy/{incoming,outgoing}Calls` with `{ item: callItems[0] }`.
21. [ ] LSP gitignore filtering: applies to `findReferences`/`goToDefinition`/`goToImplementation`/`workspaceSymbol`; `git check-ignore` in batches of 50, 5 s timeout.
22. [ ] LSPTool `mapToolResultToToolResultBlockParam` returns only `output.result` (the formatted string) — `resultCount`/`fileCount` are UI-only.
23. [ ] LSPTool UI: hover special label `'available'`; collapsed view uses `<CtrlOToExpand />` when `resultCount > 0`; verbose view uses `⎿` glyph; symbol-context UI hint uses sync 64 KiB read with regex `/[\w$'!]+|[+\-*/%&|^~<>=]+/g` and 30-char cap; collapsed-error renders `'LSP operation failed'` via tag detection.
24. [ ] LSP formatters: SymbolKind table (1..26) per `formatters.ts:273-301`; URI normalization strips `file://`, drops `/C:/` leading slash on Windows, decodes URI components with try/catch fallback, prefers relative paths *only* when shorter and not starting with `'../../'`; backslashes always normalized to forward slashes for display.
25. [ ] All listed user-facing error strings (§6.12) preserved verbatim, including punctuation and casing.

---

## 12. Open Questions / Unknowns

1. **`isOpenWorld()` returns `false` on the MCPTool template** (`MCPTool.ts:30-32`) but the template comment is `Overridden in mcpClient.ts with the real MCP tool name + args`. I confirmed the override (`client.ts:1807-1809`) reads `tool.annotations?.openWorldHint ?? false`. Servers that don't set `openWorldHint` therefore default to *closed-world*, which conflicts with the intuition that arbitrary external MCP tools should be considered open-world. Behavior is bit-exact; intent unclear. Out of scope for fix — flagged for spec 23.
2. **Skip-prefix interaction with permissions.** When `CLAUDE_AGENT_SDK_MCP_NO_PREFIX` is set, the model invokes a tool by raw name (e.g. `Write`) but permission checks key on the *prefixed* `mcpInfo` form. Whether existing rule-matching handlers (spec 09) do the right thing for unprefixed names is asserted by code comments but not directly verifiable from this scope.
3. **`workspaceSymbol` always sends `query: ''`** (`LSPTool.ts:474-476`). Comment says "Empty query returns all symbols," but most LSP servers treat empty queries differently (some return nothing). Unclear whether the input `filePath`/`line`/`character` should be used to scope the query. Out of scope for behavioral change; documented as-is.
4. **`extractMcpToolDisplayName` and `getMcpDisplayName`** appear unused inside the in-scope tools but are exported from `mcpStringUtils.ts`. Their callers (likely UI menus) are spec 37 / 28.
5. **`'No call hierarchy item found at this position'` appears twice** — once in `LSPTool.ts:310` (early-return) and once in `formatters.ts:460` (via `formatPrepareCallHierarchyResult`). Strings are identical; both paths reachable. Confirmed bit-exact.
6. **`logError` for "Failed to access file stats…"** uses `new Error(...)` wrapping rather than the original. This is intentional (per surrounding pattern) but loses stack provenance.
7. **`McpAuthTool.maxResultSizeChars = 10_000`** (vs the family default `100_000`). Likely because OAuth messages are short — but if a server returns an unusually long error message it could be persisted to disk, which seems unhelpful for an error path. Out of scope; flagged.
8. **`MCP_FETCH_CACHE_SIZE = 20`** — server names are stable, but a workspace with > 20 MCP servers would thrash the LRU. Documented constant; no behavioral consequence within in-scope tools.
9. **LSPTool comment at `LSPTool.ts:299-334`** says "For incomingCalls and outgoingCalls, we need a two-step process" but `getMethodAndParams` at `:494-511` already returns `prepareCallHierarchy` for both — meaning the first request is always `prepareCallHierarchy`, and the second is dispatched in-line via `callMethod`. This is bit-exact and intentional; flagged in case future readers find the structure surprising.
