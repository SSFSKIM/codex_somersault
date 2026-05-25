# 24 — LSP Service Specification

> Owns `src/services/lsp/`. The LSP server manager singleton, per-language LSP server lifecycle, JSON-RPC client wrapper, diagnostic pipeline (passive `publishDiagnostics` → attachment registry), and file-sync notifications consumed by the file tools.

Adjacent (do NOT redocument):
- LSPTool surface (`src/tools/LSPTool/`) → spec 16.
- `FileWriteTool` / `FileEditTool` callsites that write/save/clear LSP diagnostics → spec 11.
- IDE LSP integration via the bridge → spec 34.
- UI rendering of diagnostics → spec 37.

---

## 1. Purpose & Scope

The LSP service manages a singleton **LSP server manager** that owns N language-server child processes (one per plugin-supplied configuration), routes per-file requests/notifications to the right server based on file extension, opens/changes/saves files, captures `textDocument/publishDiagnostics` notifications passively into an attachment registry, deduplicates diagnostics across batches and across turns, and exposes a small public surface to: (a) `LSPTool` (gated by `ENABLE_LSP_TOOL` at `src/tools.ts:224`), (b) `FileWriteTool` / `FileEditTool` (sync writes + clear delivered diagnostics), (c) `useLspInitializationNotification` (UI), (d) the plugin refresh path (re-init when plugins change).

LSP servers are **only** supplied via plugins — there is no user/project setting path (`src/services/lsp/config.ts:9-13`). The manager is a *passive* diagnostic source: it never blocks turns; diagnostics arrive via the attachment registry on the next query.

In scope: `src/services/lsp/{manager.ts,LSPServerManager.ts,LSPServerInstance.ts,LSPClient.ts,LSPDiagnosticRegistry.ts,passiveFeedback.ts,config.ts}`.
Out of scope: see "Adjacent" above; gitignore filtering / 10 MB cap / UNC bypass / 50-batch / 5 s timeout (per the §16 finding) live in **the LSPTool layer**, not in this manager — see §12.

---

## 2. Source Map

Source coverage (every owned file, fully read):

| File | Lines | Coverage |
|---|---:|---|
| `src/services/lsp/manager.ts` | 290 | full |
| `src/services/lsp/LSPServerManager.ts` | 421 | full |
| `src/services/lsp/LSPServerInstance.ts` | 512 | full |
| `src/services/lsp/LSPClient.ts` | 448 | full |
| `src/services/lsp/LSPDiagnosticRegistry.ts` | 387 | full |
| `src/services/lsp/passiveFeedback.ts` | 329 | full |
| `src/services/lsp/config.ts` | 80 | full |

**Missing from leak (referenced but absent):**
- `src/services/lsp/types.ts` — imported by `LSPServerManager.ts:11`, `LSPServerInstance.ts:10`, `config.ts:7`. Declares `LspServerState` and `ScopedLspServerConfig`. Recorded in §12. Field set is recoverable from usage in instance.ts (see §4).

**Imports from (upstream):**
- `src/utils/plugins/lspPluginIntegration.ts` (`getPluginLspServers`) — `config.ts:5`
- `src/utils/plugins/pluginLoader.ts` (`loadAllPluginsCacheOnly`) — `config.ts:6`
- `src/utils/{cwd,debug,errors,log,sleep,subprocessEnv,slowOperations}.ts`
- `src/utils/envUtils.ts` (`isBareMode`) — `manager.ts:2`
- `src/services/diagnosticTracking.ts` (type `DiagnosticFile`) — `LSPDiagnosticRegistry.ts:7`, `passiveFeedback.ts:7`
- `vscode-jsonrpc/node.js`, `vscode-languageserver-protocol`, `lru-cache`, `child_process`, `crypto`, `url`, `path`

**Imported by (downstream callsites):**
- `src/main.tsx:2321` — `initializeLspServerManager()` at startup
- `src/entrypoints/init.ts:189` — `registerCleanup(shutdownLspServerManager)`
- `src/utils/plugins/refresh.ts:145` — `reinitializeLspServerManager()` after plugin reload
- `src/hooks/useManagePlugins.ts:145` — `reinitializeLspServerManager()`
- `src/tools/LSPTool/LSPTool.ts:17-19,138,232,236` — `getLspServerManager`, `isLspConnected`, `waitForInitialization`
- `src/tools/FileWriteTool/FileWriteTool.ts:308,311` — `getLspServerManager`, `clearDeliveredDiagnosticsForFile`
- `src/tools/FileEditTool/FileEditTool.ts:494,497` — same pair
- `src/hooks/notifs/useLspInitializationNotification.tsx:97` — `getLspServerManager`

**Feature flags / env gates touching this service:**
- `ENABLE_LSP_TOOL` (env, truthy) — adds `LSPTool` at `src/tools.ts:224`. Manager itself is **not** gated by this; LSP can be initialized while the tool surface is hidden (file tools still call `saveFile`).
- `isBareMode()` (env: `--bare` / `SIMPLE`) — short-circuits `initializeLspServerManager` (`manager.ts:148-150`).
- No `feature(...)`, no `USER_TYPE === 'ant'` gate in `src/services/lsp/`.

---

## 3. Public Interface (Contract)

### 3.1 Module-level singleton (`manager.ts`)

```ts
function getLspServerManager(): LSPServerManager | undefined
function getInitializationStatus():
  | { status: 'not-started' }
  | { status: 'pending' }
  | { status: 'success' }
  | { status: 'failed'; error: Error }
function isLspConnected(): boolean
function waitForInitialization(): Promise<void>
function initializeLspServerManager(): void
function reinitializeLspServerManager(): void
function shutdownLspServerManager(): Promise<void>
function _resetLspManagerForTesting(): void
```

`getLspServerManager` returns `undefined` when state is `'failed'` or before the first `initialize…` call (`manager.ts:63-69`). `isLspConnected` returns `true` iff initialization did not fail, the singleton has ≥1 server, and ≥1 server is **not** in `'error'` state (`manager.ts:100-110`).

### 3.2 `LSPServerManager` (`LSPServerManager.ts:16-43`, verbatim)

```ts
export type LSPServerManager = {
  initialize(): Promise<void>
  shutdown(): Promise<void>
  getServerForFile(filePath: string): LSPServerInstance | undefined
  ensureServerStarted(filePath: string): Promise<LSPServerInstance | undefined>
  sendRequest<T>(filePath: string, method: string, params: unknown): Promise<T | undefined>
  getAllServers(): Map<string, LSPServerInstance>
  openFile(filePath: string, content: string): Promise<void>
  changeFile(filePath: string, content: string): Promise<void>
  saveFile(filePath: string): Promise<void>
  closeFile(filePath: string): Promise<void>
  isFileOpen(filePath: string): boolean
}
```

### 3.3 `LSPServerInstance` (`LSPServerInstance.ts:33-65`, verbatim)

```ts
export type LSPServerInstance = {
  readonly name: string
  readonly config: ScopedLspServerConfig
  readonly state: LspServerState
  readonly startTime: Date | undefined
  readonly lastError: Error | undefined
  readonly restartCount: number
  start(): Promise<void>
  stop(): Promise<void>
  restart(): Promise<void>
  isHealthy(): boolean
  sendRequest<T>(method: string, params: unknown): Promise<T>
  sendNotification(method: string, params: unknown): Promise<void>
  onNotification(method: string, handler: (params: unknown) => void): void
  onRequest<TParams, TResult>(
    method: string,
    handler: (params: TParams) => TResult | Promise<TResult>,
  ): void
}
```

### 3.4 `LSPClient` (`LSPClient.ts:21-41`, verbatim)

```ts
export type LSPClient = {
  readonly capabilities: ServerCapabilities | undefined
  readonly isInitialized: boolean
  start: (
    command: string,
    args: string[],
    options?: { env?: Record<string, string>; cwd?: string },
  ) => Promise<void>
  initialize: (params: InitializeParams) => Promise<InitializeResult>
  sendRequest: <TResult>(method: string, params: unknown) => Promise<TResult>
  sendNotification: (method: string, params: unknown) => Promise<void>
  onNotification: (method: string, handler: (params: unknown) => void) => void
  onRequest: <TParams, TResult>(
    method: string,
    handler: (params: TParams) => TResult | Promise<TResult>,
  ) => void
  stop: () => Promise<void>
}
```

### 3.5 Diagnostic registry (`LSPDiagnosticRegistry.ts`)

```ts
export type PendingLSPDiagnostic = {
  serverName: string
  files: DiagnosticFile[]
  timestamp: number
  attachmentSent: boolean
}
function registerPendingLSPDiagnostic({ serverName, files }): void
function checkForLSPDiagnostics(): Array<{ serverName: string; files: DiagnosticFile[] }>
function clearAllLSPDiagnostics(): void
function resetAllLSPDiagnosticState(): void
function clearDeliveredDiagnosticsForFile(fileUri: string): void
function getPendingLSPDiagnosticCount(): number
```

### 3.6 Passive feedback (`passiveFeedback.ts`)

```ts
function formatDiagnosticsForAttachment(params: PublishDiagnosticsParams): DiagnosticFile[]
function registerLSPNotificationHandlers(manager: LSPServerManager): HandlerRegistrationResult
type HandlerRegistrationResult = {
  totalServers: number
  successCount: number
  registrationErrors: Array<{ serverName: string; error: string }>
  diagnosticFailures: Map<string, { count: number; lastError: string }>
}
```

### 3.7 Config (`config.ts:15-17`)

```ts
export async function getAllLspServers(): Promise<{
  servers: Record<string, ScopedLspServerConfig>
}>
```

---

## 4. Data Model & State

### 4.1 Module-scope state (`manager.ts:14-40`)

```ts
type InitializationState = 'not-started' | 'pending' | 'success' | 'failed'
let lspManagerInstance: LSPServerManager | undefined
let initializationState: InitializationState = 'not-started'
let initializationError: Error | undefined
let initializationGeneration = 0          // invalidates stale init promises
let initializationPromise: Promise<void> | undefined
```

### 4.2 `ScopedLspServerConfig` (recovered from usage; declaring `types.ts` is **missing from leak**)

Fields actually read by `LSPServerInstance.ts` and `LSPServerManager.ts`:

| Field | Type | Citation | Notes |
|---|---|---|---|
| `command` | `string` | mgr `:92` | required |
| `args` | `string[]` (opt) | inst `:158` | defaults to `[]` |
| `env` | `Record<string,string>` (opt) | inst `:159` | merged on top of `subprocessEnv()` |
| `workspaceFolder` | `string` (opt) | inst `:160,164` | falls back to `getCwd()` |
| `extensionToLanguage` | `Record<string,string>` | mgr `:97-104,107,286` | required, non-empty |
| `initializationOptions` | `unknown` (opt) | inst `:174` | passed to LSP `initialize` |
| `startupTimeout` | `number` (ms) (opt) | inst `:240-247` | wraps `initialize` in `withTimeout` |
| `maxRestarts` | `number` (opt) | inst `:142,313` | default `3` |
| `restartOnCrash` | unimplemented | inst `:95-99` | THROWS if set |
| `shutdownTimeout` | unimplemented | inst `:100-104` | THROWS if set |

### 4.3 `LspServerState` (recovered from usage)

```ts
type LspServerState = 'stopped' | 'starting' | 'running' | 'stopping' | 'error'
```

State transitions (`LSPServerInstance.ts`):
- `stopped → starting → running` (on `start()` success, `:154 → :250`)
- `running → stopping → stopped` (on `stop()` success, `:280 → :282`)
- `any → error` (on any failure, with `lastError` set; `:259, :286`)
- `error → starting` (retry path, capped by `maxRestarts`, `:143-150`)

### 4.4 `LSPServerManager` closure state (`LSPServerManager.ts:60-64`)

```ts
const servers: Map<string, LSPServerInstance>          // serverName → instance
const extensionMap: Map<string, string[]>              // .ext (lowercase) → [serverName,...]
const openedFiles: Map<string, string>                 // fileURI → serverName
```

Extension dispatch picks the **first** registered server when multiple handle the same extension (`LSPServerManager.ts:200-207`). Two distinct ordering layers must not be conflated:

1. **Same-named server collisions** in `getAllLspServers` (`config.ts:45-49`): `Object.assign(allServers, scopedServers)` is called per plugin in `Promise.all`-resolved order, so a **later** plugin overwrites an earlier plugin's config for the *same* scoped server name (later-wins).
2. **Different-named servers competing for the same extension** in `LSPServerManager.initialize` (`:89-117`): `Object.entries(serverConfigs)` is iterated in insertion order and each server's extensions are pushed onto `extensionMap[ext]` — so for *different* server names, the **earlier**-iterated server wins dispatch (first-wins).

Reimplementers must preserve both: later-wins for same-key config-dict overrides, first-wins for cross-server extension dispatch.

**Notebook (`.ipynb`) dispatch** is **not** filtered by this manager. `getServerForFile` keys purely off `path.extname(filePath).toLowerCase()` — if a plugin registers `.ipynb` in `extensionToLanguage`, notebook paths route to that server. `.ipynb` exclusion (if desired) is the **caller layer's** responsibility — see §9.6 / §12.8 and cross-link to spec 11 (file write/edit tools) and spec 16 (LSPTool).

### 4.5 `LSPDiagnosticRegistry` state (`LSPDiagnosticRegistry.ts:42-56`)

```ts
const MAX_DIAGNOSTICS_PER_FILE = 10
const MAX_TOTAL_DIAGNOSTICS = 30
const MAX_DELIVERED_FILES = 500

const pendingDiagnostics = new Map<string /* uuid */, PendingLSPDiagnostic>()
const deliveredDiagnostics = new LRUCache<string /* fileUri */, Set<string /* dedupKey */>>(
  { max: MAX_DELIVERED_FILES },
)
```

### 4.6 `LSPClient` per-instance state (`LSPClient.ts:55-71`)

```ts
let process: ChildProcess | undefined
let connection: MessageConnection | undefined
let capabilities: ServerCapabilities | undefined
let isInitialized = false
let startFailed = false
let startError: Error | undefined
let isStopping = false
const pendingHandlers: Array<{method, handler}>          // queued before listen()
const pendingRequestHandlers: Array<{method, handler}>
```

---

## 5. Algorithm / Control Flow

### 5.1 `initializeLspServerManager()` (`manager.ts:145-208`)

```
if isBareMode(): return
if lspManagerInstance && state != 'failed': return       # idempotent
if state == 'failed': clear instance/error               # retry path
lspManagerInstance = createLSPServerManager()
state = 'pending'
gen = ++initializationGeneration
initializationPromise = lspManagerInstance.initialize()
  .then(() =>
    if gen == initializationGeneration:
      state = 'success'
      registerLSPNotificationHandlers(lspManagerInstance))
  .catch(err =>
    if gen == initializationGeneration:
      state = 'failed'; initializationError = err
      lspManagerInstance = undefined
      logError(err))
```

`reinitializeLspServerManager()` (`:226-253`): if state is `'not-started'` returns immediately (does **not** kick off a first init). Otherwise fires-and-forgets `lspManagerInstance.shutdown()`, clears instance, resets state to `'not-started'`, and calls `initializeLspServerManager()`. Generation counter invalidates any in-flight init.

`shutdownLspServerManager()` (`:267-289`): no-op if not initialized; awaits `manager.shutdown()`; in `finally` always clears instance, state, error, promise, and increments generation. **Errors are logged, never propagated.**

`waitForInitialization()` (`:121-133`): returns immediately if `'success'` or `'failed'` or `'not-started'`; awaits `initializationPromise` only if `'pending'`.

### 5.2 `LSPServerManager.initialize()` (`LSPServerManager.ts:71-148`)

```
serverConfigs = await getAllLspServers()
for [serverName, config] in serverConfigs:
  validate config.command exists                     # else throw inside per-server try
  validate config.extensionToLanguage non-empty
  for ext in keys(extensionToLanguage):
    extensionMap[ext.toLowerCase()].push(serverName)
  instance = createLSPServerInstance(serverName, config)
  servers[serverName] = instance
  instance.onRequest('workspace/configuration', params =>
    params.items.map(_ => null))                     # always answer null
# per-server failure does NOT abort init — continue with others
```

### 5.3 `LSPServerInstance.start()` (`LSPServerInstance.ts:135-264`)

```
if state in {running, starting}: return
maxRestarts = config.maxRestarts ?? 3
if state == 'error' and crashRecoveryCount > maxRestarts:
  throw "LSP server '{name}' exceeded max crash recovery attempts ({maxRestarts})"
state = 'starting'
await client.start(command, args ?? [], { env, cwd: workspaceFolder })
workspaceFolder = config.workspaceFolder || getCwd()
workspaceUri = pathToFileURL(workspaceFolder).href
init = client.initialize(InitializeParams)           # see §6.2 verbatim
if config.startupTimeout != null:
  await withTimeout(init, startupTimeout,
    "LSP server '{name}' timed out after {ms}ms during initialization")
else:
  await init
state = 'running'; startTime = now; crashRecoveryCount = 0
catch err:
  client.stop().catch(() => {})                      # cleanup spawned process
  state = 'error'; lastError = err; logError(err); throw
```

### 5.4 `LSPServerInstance.sendRequest` retry loop (`:355-410`)

```
if !isHealthy(): throw "Cannot send request to LSP server '{name}': server is {state}{...last error...}"
for attempt in 0..MAX_RETRIES_FOR_TRANSIENT_ERRORS:    # MAX = 3, total 4 attempts
  try return await client.sendRequest(method, params)
  catch err:
    code = err.code   # duck-typed (jsonrpc version skew tolerated)
    if code === -32801 (LSP_ERROR_CONTENT_MODIFIED) and attempt < MAX:
      delay = 500 * 2^attempt                          # 500ms, 1000ms, 2000ms
      await sleep(delay); continue
    break
throw "LSP request '{method}' failed for server '{name}': {lastError.message ?? 'unknown error'}"
```

### 5.5 `restart()` (`:300-331`)

```
try await stop() catch e: throw "Failed to stop LSP server '{name}' during restart: ..."
# only reached if stop() resolved successfully:
restartCount++
if restartCount > maxRestarts: throw "Max restart attempts ({maxRestarts}) exceeded for server '{name}'"
try await start() catch e: throw "Failed to start LSP server '{name}' during restart (attempt {n}/{max}): ..."
```

**Ordering invariant**: `restartCount` is incremented **only after `stop()` resolves successfully**. A failed `stop()` re-throws *without* bumping the counter, so subsequent `restart()` attempts following a stuck-shutdown are not penalized against `maxRestarts`. Reimplementers must not move the increment above the `stop()` await or into the catch path.

`restartCount` and `crashRecoveryCount` are **separate counters** — manual `restart()` and crash-recovery share `maxRestarts` but track independently.

### 5.6 File sync routines (`LSPServerManager.ts:270-405`)

`openFile(path, content)`:
```
server = await ensureServerStarted(path); if !server: return
fileUri = pathToFileURL(resolve(path)).href
if openedFiles[fileUri] == server.name: return        # idempotent
ext = path.extname(path).toLowerCase()
languageId = server.config.extensionToLanguage[ext] || 'plaintext'
await server.sendNotification('textDocument/didOpen', {
  textDocument: { uri: fileUri, languageId, version: 1, text: content }
})
openedFiles[fileUri] = server.name
```

`changeFile(path, content)`:
```
server = getServerForFile(path)
if !server || server.state != 'running': return openFile(path, content)
fileUri = pathToFileURL(resolve(path)).href
if openedFiles[fileUri] != server.name: return openFile(path, content)   # didOpen first
await server.sendNotification('textDocument/didChange', {
  textDocument: { uri: fileUri, version: 1 },          # version is hard-coded 1
  contentChanges: [{ text: content }]                  # full-document sync
})
```

`saveFile(path)`:
```
server = getServerForFile(path)
if !server || server.state != 'running': return
await server.sendNotification('textDocument/didSave', {
  textDocument: { uri: pathToFileURL(resolve(path)).href }
})
```

`closeFile(path)`: same shape; on success deletes `openedFiles[fileUri]`. Comment notes "Currently available but not yet integrated with compact flow" (`:373-374`).

`isFileOpen(path)`: pure check on `openedFiles`.

### 5.7 LSPClient connection bootstrap (`LSPClient.ts:88-254`)

```
process = spawn(command, args, {
  stdio: ['pipe','pipe','pipe'],
  env: { ...subprocessEnv(), ...options?.env },
  cwd: options?.cwd,
  windowsHide: true,
})
if !process.stdout || !process.stdin: throw "LSP server process stdio not available"
# CRITICAL: wait for 'spawn' before using streams — ENOENT fires async
await new Promise((resolve, reject) => {
  process.once('spawn', () => { cleanup(); resolve() })
  process.once('error', err => { cleanup(); reject(err) })
})
process.stderr.on('data', buf => logForDebugging(`[LSP SERVER {name}] {buf.trim()}`))
process.on('error', err =>
  if !isStopping: startFailed = true; startError = err;
                  logError("LSP server {name} failed to start: {msg}"))
process.on('exit', (code, _signal) =>
  if code !== 0 and code !== null and !isStopping:
    isInitialized = false; startFailed = false; startError = undefined
    crashError = "LSP server {name} crashed with exit code {code}"
    logError(crashError); onCrash?.(crashError))
process.stdin.on('error', err =>
  if !isStopping: logForDebugging("LSP server {name} stdin error: {msg}"))
reader = new StreamMessageReader(process.stdout)
writer = new StreamMessageWriter(process.stdin)
connection = createMessageConnection(reader, writer)
connection.onError(([err,_,_]) =>
  if !isStopping: startFailed = true; startError = err;
                  logError("LSP server {name} connection error: {msg}"))
connection.onClose(() => if !isStopping: isInitialized = false; logForDebugging(...))
connection.listen()
connection.trace(Trace.Verbose, { log: m => logForDebugging("[LSP PROTOCOL {name}] {m}") })
  .catch(err => logForDebugging("Failed to enable tracing for {name}: {msg}"))
for {method,handler} in pendingHandlers: connection.onNotification(method, handler)
pendingHandlers.length = 0
for {method,handler} in pendingRequestHandlers: connection.onRequest(method, handler)
pendingRequestHandlers.length = 0
```

`initialize(params)`: sends `'initialize'` request, captures `result.capabilities`, sends `'initialized'` notification, sets `isInitialized = true`.

`stop()`: sets `isStopping = true`, sends `'shutdown'` request and `'exit'` notification (errors recorded as `shutdownError` but cleanup continues), `connection.dispose()`, removes all event listeners, `process.kill()`. Resets `isInitialized = false`, `capabilities = undefined`, `isStopping = false`. **Re-throws** `shutdownError` after cleanup.

**`stop()` side-effect on failure (undocumented elsewhere)** (`LSPClient.ts:431-436`): when `shutdownError` is captured, the cleanup block sets `startFailed = true; startError = shutdownError` *before* the re-throw. The comment "Don't reset startFailed - preserve error state for diagnostics" intentionally leaves prior `startFailed` state intact, but a fresh shutdown error overwrites it. Because `LSPServerInstance` retains the same `client` closure across its lifetime (`LSPServerInstance.ts:121-125` — the client is constructed once and not reset between `start()`/`stop()` cycles), a subsequent `instance.start()` on the same instance will spawn a new process but **inherit the poisoned `startFailed`/`startError`**. Reimplementers must mirror this behavior verbatim or explicitly reset `startFailed` at the top of `start()` — the leaked source does neither.

Crash propagation: `onCrash` callback (passed by `LSPServerInstance`) flips that instance to `state = 'error'` and increments `crashRecoveryCount`, so the next `ensureServerStarted` call will attempt to restart (`LSPServerInstance.ts:118-125`).

### 5.8 Diagnostic delivery pipeline (`passiveFeedback.ts` + `LSPDiagnosticRegistry.ts`)

```
# server side: textDocument/publishDiagnostics arrives
handler(params):
  validate params is {uri, diagnostics}
  diagnosticFiles = formatDiagnosticsForAttachment(params)
    # uri: if startsWith('file://') -> fileURLToPath; else pass-through
    # severity: 1→Error, 2→Warning, 3→Info, 4→Hint, default→Error
    # range: 0-based line/character preserved verbatim from LSP
    # code: stringified if present, else undefined
  if diagnosticFiles[0].diagnostics.length == 0: skip
  registerPendingLSPDiagnostic({ serverName, files: diagnosticFiles })
    # diagnosticId = randomUUID(); pendingDiagnostics[id] = {...}

# attachment delivery (called per-turn by checkForLSPDiagnostics):
allFiles = [unsent.files...]
for file in allFiles:                                  # within-batch + cross-turn dedup
  for diag in file.diagnostics:
    key = jsonStringify({message, severity, range, source||null, code||null})
    if seenInBatch[file.uri].has(key) or
       deliveredDiagnostics[file.uri]?.has(key): skip
    seenInBatch.add(key); dedupedFile.diagnostics.push(diag)
drop files where dedupedFile.diagnostics empty
mark all sent diagnostics as attachmentSent and DELETE from pendingDiagnostics
for file in dedupedFiles:
  sort by severityToNumber asc                         # Error<Warning<Info<Hint
  if length > MAX_DIAGNOSTICS_PER_FILE (10):
    truncate to 10
  remaining = MAX_TOTAL_DIAGNOSTICS (30) - runningTotal
  if length > remaining: truncate to remaining
  runningTotal += length
filter empty files
for file/diag delivered: deliveredDiagnostics[uri].add(key)   # LRU max=500 files
return [{ serverName: joined, files: dedupedFiles }] (or [] if 0)
```

`clearDeliveredDiagnosticsForFile(fileUri)` (`:372-379`): called by `FileWriteTool.ts:311` and `FileEditTool.ts:497` after a successful write/edit so re-emitted diagnostics for the just-edited file are not suppressed by the cross-turn dedup cache.

### 5.9 Coordinate convention

All ranges from `publishDiagnostics` are passed through verbatim — LSP uses **0-based line and character**; `formatDiagnosticsForAttachment` preserves them as-is (`passiveFeedback.ts:75-85`). Any 1-based↔0-based conversion happens **upstream of this service** (in LSPTool / file tools) — no conversion is performed here.

### 5.10 Server-binary discovery / path resolution

There is **no PATH search, no per-language binary detection, and no version probing inside `src/services/lsp/`.** The launcher's `command`, `args`, `env`, and `workspaceFolder` come **only** from the plugin-supplied `ScopedLspServerConfig` and are passed to Node's `spawn(command, args, {...})` directly (`LSPClient.ts:98-104`). Dispatch is purely by lower-cased extension (`LSPServerManager.ts:108-117, 192-207`). Discovery upstream of this service lives in `src/utils/plugins/lspPluginIntegration.ts` (out of scope for this spec).

---

## 6. Verbatim Assets

### 6.1 Constants & timing

| Constant | Value | Citation |
|---|---|---|
| `LSP_ERROR_CONTENT_MODIFIED` | `-32801` | `LSPServerInstance.ts:17` |
| `MAX_RETRIES_FOR_TRANSIENT_ERRORS` | `3` (4 attempts total) | `LSPServerInstance.ts:22` |
| `RETRY_BASE_DELAY_MS` | `500` (delays 500/1000/2000 ms) | `LSPServerInstance.ts:28` |
| Default `maxRestarts` | `3` | `LSPServerInstance.ts:142, :313` |
| `MAX_DIAGNOSTICS_PER_FILE` | `10` | `LSPDiagnosticRegistry.ts:42` |
| `MAX_TOTAL_DIAGNOSTICS` | `30` | `LSPDiagnosticRegistry.ts:43` |
| `MAX_DELIVERED_FILES` (LRU) | `500` | `LSPDiagnosticRegistry.ts:46, :54` |
| Severity numeric ordering | Error=1, Warning=2, Info=3, Hint=4 | `LSPDiagnosticRegistry.ts:91-103` |
| `version` field on didOpen / didChange | hard-coded `1` | `LSPServerManager.ts:293, :330` |
| Default `languageId` | `'plaintext'` | `LSPServerManager.ts:286` |
| Failure-warning threshold | `>= 3` consecutive | `passiveFeedback.ts:240, :266` |
| `startupTimeout` | only applied if `config.startupTimeout !== undefined` | `LSPServerInstance.ts:240-247` |
| Debounce / throttle / batch=50 / 5 s timeout / 10 MB cap / UNC bypass | **not present in `src/services/lsp/`** — handled by LSPTool/file layer; see §12 | — |

### 6.2 LSP `InitializeParams` (verbatim, `LSPServerInstance.ts:167-237`)

```ts
const initParams: InitializeParams = {
  processId: process.pid,
  initializationOptions: config.initializationOptions ?? {},
  workspaceFolders: [
    { uri: workspaceUri, name: path.basename(workspaceFolder) },
  ],
  rootPath: workspaceFolder,                 // deprecated LSP 3.8 — kept for some servers
  rootUri: workspaceUri,                     // deprecated LSP 3.16 — typescript-language-server
  capabilities: {
    workspace: {
      configuration: false,                  // we don't implement workspace/configuration
      workspaceFolders: false,               // we don't handle didChangeWorkspaceFolders
    },
    textDocument: {
      synchronization: {
        dynamicRegistration: false,
        willSave: false,
        willSaveWaitUntil: false,
        didSave: true,
      },
      publishDiagnostics: {
        relatedInformation: true,
        tagSupport: { valueSet: [1, 2] },     // Unnecessary, Deprecated
        versionSupport: false,
        codeDescriptionSupport: true,
        dataSupport: false,
      },
      hover: {
        dynamicRegistration: false,
        contentFormat: ['markdown', 'plaintext'],
      },
      definition: { dynamicRegistration: false, linkSupport: true },
      references: { dynamicRegistration: false },
      documentSymbol: {
        dynamicRegistration: false,
        hierarchicalDocumentSymbolSupport: true,
      },
      callHierarchy: { dynamicRegistration: false },
    },
    general: { positionEncodings: ['utf-16'] },
  },
}
```

### 6.3 Per-method request/notification shapes (verbatim, this service)

| Method | Direction | Shape sent | Citation |
|---|---|---|---|
| `initialize` | C→S, request | §6.2 above | `LSPServerInstance.ts:239` via `client.initialize` |
| `initialized` | C→S, notify | `{}` | `LSPClient.ts:272` |
| `shutdown` | C→S, request | `{}` | `LSPClient.ts:382` |
| `exit` | C→S, notify | `{}` | `LSPClient.ts:383` |
| `textDocument/didOpen` | C→S, notify | `{ textDocument: { uri, languageId, version: 1, text: content } }` | `LSPServerManager.ts:289-296` |
| `textDocument/didChange` | C→S, notify | `{ textDocument: { uri, version: 1 }, contentChanges: [{ text: content }] }` | `LSPServerManager.ts:327-333` |
| `textDocument/didSave` | C→S, notify | `{ textDocument: { uri } }` | `LSPServerManager.ts:354-358` |
| `textDocument/didClose` | C→S, notify | `{ textDocument: { uri } }` | `LSPServerManager.ts:384-388` |
| `workspace/configuration` | S→C, request | reply: `params.items.map(() => null)` | `LSPServerManager.ts:125-135` |
| `textDocument/publishDiagnostics` | S→C, notify | handled by `formatDiagnosticsForAttachment` | `passiveFeedback.ts:161-278` |
| `$/setTrace` | C→S, notify | sent via `connection.trace(Trace.Verbose, …)` | `LSPClient.ts:216-226` |

### 6.4 Server-binary launch contract (verbatim, `LSPClient.ts:98-104`)

```ts
process = spawn(command, args, {
  stdio: ['pipe', 'pipe', 'pipe'],
  env: { ...subprocessEnv(), ...options?.env },
  cwd: options?.cwd,
  windowsHide: true,
})
```

Rules:
- `command` and `args` come from `ScopedLspServerConfig` unmodified.
- `cwd` = `config.workspaceFolder` (or undefined → process default).
- env merge order: `subprocessEnv()` first, then `config.env` overrides.
- `windowsHide: true` is unconditional.
- After `spawn()`, **must** await the `'spawn'` event before using stdio (§5.7) — otherwise ENOENT becomes an unhandled rejection.

### 6.5 Error string set (verbatim)

LSP server manager / instance:
- `"LSP server '{name}': restartOnCrash is not yet implemented. Remove this field from the configuration."` (`LSPServerInstance.ts:97`)
- `"LSP server '{name}': shutdownTimeout is not yet implemented. Remove this field from the configuration."` (`:102`)
- `"LSP server '{name}' exceeded max crash recovery attempts ({maxRestarts})"` (`:144-146`)
- `"LSP server '{name}' timed out after {ms}ms during initialization"` (`:244`)
- `"Cannot send request to LSP server '{name}': server is {state}{, last error: {msg}}"` (`:357-360`)
- `"LSP request '{method}' failed for server '{name}': {message ?? 'unknown error'}"` (`:405-407`)
- `"Cannot send notification to LSP server '{name}': server is {state}"` (`:421`)
- `"LSP notification '{method}' failed for server '{name}': {msg}"` (`:431`)
- `"Failed to stop LSP server '{name}' during restart: {msg}"` (`:304`)
- `"Max restart attempts ({maxRestarts}) exceeded for server '{name}'"` (`:315`)
- `"Failed to start LSP server '{name}' during restart (attempt {n}/{max}): {msg}"` (`:325-327`)
- `"Server {name} missing required 'command' field"` (`LSPServerManager.ts:93`)
- `"Server {name} missing required 'extensionToLanguage' field"` (`:101`)
- `"Failed to initialize LSP server {name}: {msg}"` (`:140`)
- `"Failed to start LSP server for file {filePath}: {msg}"` (`:227`)
- `"LSP request failed for file {filePath}, method '{method}': {msg}"` (`:257`)
- `"Failed to load LSP server configuration: {msg}"` (`:83`)
- `"Failed to stop {n} LSP server(s): {joined}"` (`:179`)
- `"Failed to sync file open {path}: {msg}"` (`:303`)
- `"Failed to sync file change {path}: {msg}"` (`:336`)
- `"Failed to sync file save {path}: {msg}"` (`:361`)
- `"Failed to sync file close {path}: {msg}"` (`:393`)

LSP client:
- `"LSP server process stdio not available"` (`LSPClient.ts:107`)
- `"LSP server {name} failed to start: {msg}"` (`:150-151, :249-250`) — also default `"LSP server {serverName} failed to start"` if no error captured (`:75`)
- `"LSP server {name} crashed with exit code {code}"` (`:161-163`)
- `"LSP server {name} connection error: {msg}"` (`:194`)
- `"LSP server {name} initialize failed: {msg}"` (`:282`)
- `"LSP server {name} request {method} failed: {msg}"` (`:309`)
- `"LSP server {name} notification {method} failed: {msg}"` (`:329`)
- `"LSP server {name} stop failed: {msg}"` (`:388`)
- `"LSP client not started"` (`:258, :294, :318`)
- `"LSP server not initialized"` (`:300`)

Passive feedback / dedup:
- `"LSP server {name} sent invalid diagnostic params (missing uri or diagnostics)"` (`passiveFeedback.ts:175-177`)
- `"Server instance is null/undefined"` (`:147`)
- `"Server instance has no onNotification method"` (`:148`)
- `"Failed to register diagnostics for {n} LSP server(s): {failedServers}"` (`:307`)
- `"Failed to deduplicate diagnostic in {uri}: {msg}. Diagnostic message: {first 100 chars}"` (`LSPDiagnosticRegistry.ts:170-174`)
- `"Failed to deduplicate LSP diagnostics: {msg}"` (`:223`)
- `"Failed to track delivered diagnostic in {uri}: {msg}. Diagnostic message: {first 100 chars}"` (`:303-308`)
- `"Failed to convert URI to file path: {uri}. Error: {msg}. Using original URI as fallback."` (`passiveFeedback.ts:55-57`)

---

## 7. Side Effects & I/O

| Side effect | Location | Notes |
|---|---|---|
| Spawn child process per LSP server | `LSPClient.ts:98-104` | stdio piped, `windowsHide: true`, env merged |
| Read child stderr | `LSPClient.ts:134-141` | logged as `[LSP SERVER {name}] {output}` debug |
| `connection.trace(Trace.Verbose, …)` | `LSPClient.ts:216-226` | sends `$/setTrace`; failure swallowed (logged only) |
| `process.kill()` on stop | `LSPClient.ts:418` | wrapped in try/catch — already-dead is fine |
| Filesystem path resolution | `pathToFileURL(path.resolve(filePath))` everywhere file paths cross to LSP | URIs use `file://` scheme |
| Plugin loader read | `loadAllPluginsCacheOnly()` via `config.ts:21` | cache-only — no fresh disk read |
| Env vars consumed | `subprocessEnv()` (base for child env), `isBareMode()` reading `--bare`/`SIMPLE`, `ENABLE_LSP_TOOL` (read at `tools.ts:224`) | none read directly inside the manager |
| Network | none in this service |
| Trust boundaries | LSP server is treated as semi-trusted: stderr logged, JSON-RPC messages parsed by `vscode-jsonrpc`, `workspace/configuration` requests answered with `null` so server cannot pull config it didn't ship with | no permission gate at this layer (LSP is read-only diagnostics + tool-routed requests) |

`pendingHandlers` and `pendingRequestHandlers` queues (`LSPClient.ts:64-71`) let callers register handlers **before** `start()` finishes; they're flushed once `connection.listen()` is called. `LSPServerManager.initialize` exploits this to attach `workspace/configuration` early (`:125-135`); `passiveFeedback.registerLSPNotificationHandlers` likewise attaches `textDocument/publishDiagnostics` to all servers, including any whose process hasn't started yet.

---

## 8. Feature Flags & Variants

- **No `feature(...)` gates** in `src/services/lsp/`.
- **No `USER_TYPE === 'ant'` gates** in `src/services/lsp/`.
- **`isBareMode()`** (`manager.ts:148-150`) — `--bare` / `SIMPLE` short-circuits initialization. Comment: *"--bare / SIMPLE: no LSP. LSP is for editor integration (diagnostics, hover, go-to-def in the REPL). Scripted -p calls have no use for it."*
- **`ENABLE_LSP_TOOL`** (env, truthy) — gates `LSPTool` registration at `src/tools.ts:224`. Manager runs regardless; affects only the explicit tool surface (spec 16). File-tool callsites (`saveFile`, `clearDeliveredDiagnosticsForFile`) still fire when this is unset.
- **Plugin-only configuration** — there is no user/project-settings path. Disabling all plugins effectively disables LSP.

---

## 9. Error Handling & Edge Cases

- **Per-server init failure isolated** (`LSPServerManager.ts:136-144`): a thrown error initializing one server does not abort the manager; the failed server is dropped from `servers`/`extensionMap`, others continue.
- **Manager init failure**: state set to `'failed'`, error captured in `initializationError`, instance cleared. `getLspServerManager()` returns `undefined` until a `reinitializeLspServerManager()` call retries.
- **Crash recovery**: `process.on('exit', code !== 0 && code !== null && !isStopping)` triggers `onCrash` (`LSPClient.ts:156-167`), bumping `crashRecoveryCount`. Next `ensureServerStarted` attempts restart; capped by `maxRestarts` (default 3).
- **`ContentModified` (-32801)**: retried up to 3 times with 500/1000/2000 ms backoff (§5.4). All other JSON-RPC errors fail immediately. Duck-typed via `(error as { code?: number }).code` to tolerate dual `vscode-jsonrpc` versions in the dependency tree (`LSPServerInstance.ts:380-382`).
- **`startupTimeout`**: `withTimeout` (`LSPServerInstance.ts:499-511`) clears its own timer in `finally` to avoid orphaned `setTimeout` callbacks; a winning init promise still cleans up.
- **Spawn-vs-stream race**: explicit `await` for `'spawn'` event before using stdio — comment "*This is CRITICAL: spawn() returns immediately, but the 'error' event (e.g., ENOENT for command not found) fires asynchronously*" (`LSPClient.ts:110-131`).
- **`connection.onError` / `onClose`** registered **before** `connection.listen()` to avoid unhandled rejections on early failures (`LSPClient.ts:185-207`).
- **`isStopping` flag** suppresses spurious error logging during intentional shutdown so `stop()` can send `shutdown`+`exit` and tear down without flooding logs (`LSPClient.ts:62, :144-178, :376`).
- **Notifications are fire-and-forget at the client layer** — `LSPClient.sendNotification` logs failures but does not throw (`:316-335`). `LSPServerInstance.sendNotification` *does* throw (`:431-435`), and `LSPServerManager` re-throws to its callers (`:303-309, :336-342, :361-367, :393-399`).
- **Stale-init invalidation**: `initializationGeneration` is bumped on every `initialize…`, `shutdown…`, and `_resetLspManagerForTesting`. Resolution of an in-flight `initialize` is silently dropped if the captured generation no longer matches (`manager.ts:184, :196`).
- **Validation of unimplemented config fields**: `restartOnCrash` and `shutdownTimeout` throw immediately (`LSPServerInstance.ts:95-104`).
- **Diagnostic dedup robustness**: per-diagnostic `try/catch` around `createDiagnosticKey` so a malformed diagnostic doesn't drop an entire batch — falls through to "include anyway" (`LSPDiagnosticRegistry.ts:166-178, :297-310`).
- **Empty-diagnostics skip**: `passiveFeedback.ts:194-204` skips `publishDiagnostics` with zero diagnostics so the registry is not polluted with empty notifications.
- **3-strikes warning**: per-server failure counter in `passiveFeedback.ts:230-247, :256-274` emits a `WARNING:` debug log after 3 consecutive failures and resets on success.
- **`getServerForFile` priority**: when multiple **different-named** servers register the same extension, the server **iterated first** in `Object.entries(serverConfigs)` wins dispatch (`LSPServerManager.ts:200-202`). This is **distinct from** `Object.assign` later-wins for *same-named* config-dict overrides in `config.ts:45-49`. See §4.4 for the disambiguation.
- **Graceful-close zombie state** (`LSPClient.ts:200-207` vs `:156-167`): `connection.onClose` does **not** invoke `onCrash` — only `process.on('exit', code !== 0 && code !== null && !isStopping)` does. If a server cleanly closes its stdout (graceful exit code 0, or stdout closed without process exit) without `isStopping` set, `connection.onClose` fires and sets `isInitialized = false`, but `LSPServerInstance.state` stays `'running'` and `crashRecoveryCount` is **not** bumped. The next `sendRequest` then fails `isHealthy()` (which AND-checks `state === 'running' && client.isInitialized`, `:339`), throws "server is running, last error: …" with no `lastError` populated, and never schedules a restart. This is a known source-side defect — flagged for `BUGS-IN-SOURCE.md`.
- **Notebook (`.ipynb`) handling** is the caller's responsibility (§4.4, §5.6, §5.10). The manager does not filter notebooks. `FileWriteTool.ts:308-320` calls `clearDeliveredDiagnosticsForFile` and `saveFile` **unconditionally** for any successfully-written path; `NotebookEditTool.ts` has **zero LSP imports** (no `saveFile`, no `clearDeliveredDiagnosticsForFile`). Consequences if a plugin registers `.ipynb`: (a) `FileWriteTool` writes to `.ipynb` will fire `saveFile` for a path that was never `didOpen`'d (the manager's `saveFile` no-ops via `state !== 'running'`/missing entry, so this is benign by accident, not by design); (b) `NotebookEditTool` edits on the same `.ipynb` path do not invoke `clearDeliveredDiagnosticsForFile`, so cross-turn dedup will suppress re-emitted diagnostics until LRU eviction. Spec 11 owns the fix (gate on extension or add notebook-path equivalents); spec 16 owns the LSPTool input filter.

---

## 10. Telemetry & Observability

All observability via `logForDebugging` (debug log) and `logError` (structured error). No OpenTelemetry spans, no analytics events, no metrics.

Notable log points:
- `[LSP MANAGER] initializeLspServerManager() called` / `Created manager instance, state=pending` / `Starting async initialization (generation N)` / `LSP server manager initialized successfully` (`manager.ts:151-186`)
- `[LSP SERVER MANAGER] getAllLspServers returned N server(s)` (`LSPServerManager.ts:78`)
- `LSP manager initialized with N servers` (`:147`)
- `[LSP SERVER {name}] {stderr}` (`LSPClient.ts:138`)
- `[LSP PROTOCOL {name}] {trace}` (`LSPClient.ts:219`)
- `LSP client started for {name}`, `LSP server {name} initialized`, `LSP server {name} connection closed`, `LSP client stopped for {name}` (`LSPClient.ts:246, :275, :205, :438`)
- Diagnostic counts on every `checkForLSPDiagnostics`: `Deduplication removed N`, `Volume limiting removed N`, `Delivering N file(s) with M diagnostic(s) from K server(s)` (`LSPDiagnosticRegistry.ts:251-253, :284-287, :327-329`)
- `WARNING: LSP diagnostic handler for {name} has failed N times consecutively…` (`passiveFeedback.ts:240-247, :266-274`)
- Aggregate registration result returned by `registerLSPNotificationHandlers` (`HandlerRegistrationResult`, `passiveFeedback.ts:104-114`).

---

## 11. Reimplementation Checklist

- Module-scope singleton with the four-state machine (`not-started`/`pending`/`success`/`failed`) and a generation counter that invalidates stale `initialize()` resolutions.
- `isBareMode()` short-circuits initialization; `ENABLE_LSP_TOOL` is *not* checked here.
- Manager closure state: `servers`, `extensionMap` (lowercase ext → array), `openedFiles` (URI → server name).
- Per-server `initialize` validates `command` and non-empty `extensionToLanguage`; per-server failure does not abort init.
- Always register a `workspace/configuration` reverse-handler returning `params.items.map(() => null)`.
- `LSPServerInstance` state machine `stopped/starting/running/stopping/error`; `start()` + `stop()` idempotent for already-in-target states; `restart()` increments `restartCount` **only after `stop()` resolves successfully** (a thrown `stop()` re-throws without bumping); crash path increments `crashRecoveryCount`; both share `maxRestarts ?? 3`.
- `client.stop()` side-effect: on captured `shutdownError`, sets `startFailed = true; startError = shutdownError` before re-throw (`LSPClient.ts:431-436`). The closure-captured client persists across `LSPServerInstance` `start/stop` cycles and is **not reset** by the instance — replicate the side-effect verbatim or reset `startFailed` at the top of `start()`.
- Extension dispatch ordering: **same-named** server config-dict collisions are later-wins (`Object.assign` in `config.ts:45-49`); **different-named** servers competing for the same extension resolve to the earlier-iterated entry of `Object.entries(serverConfigs)` (first-wins in `extensionMap`).
- `.ipynb` is **not** filtered by this manager. Reimplementers of spec 11/16 must apply the carve-out (skip `saveFile`/`clearDeliveredDiagnosticsForFile` for `.ipynb` in `FileWriteTool` and `FileEditTool`; or add the equivalent calls to `NotebookEditTool`); spec 24's contract is "extension dispatch is purely lower-cased `path.extname`."
- `connection.onClose` does not invoke `onCrash`; only `process.on('exit', code !== 0 && !isStopping)` does. Document the graceful-close zombie state; do not introduce an `onClose → onCrash` bridge without first deciding whether a clean stdout-close should auto-restart.
- `InitializeParams` exactly as in §6.2 — including the deprecated `rootPath`/`rootUri`, the `tagSupport.valueSet: [1, 2]`, `positionEncodings: ['utf-16']`, `configuration: false` and `workspaceFolders: false`.
- `version: 1` hard-coded on every didOpen/didChange (no versioning state).
- Full-document sync in `didChange` (`contentChanges: [{ text: content }]`).
- `languageId` from `extensionToLanguage[ext.toLowerCase()] || 'plaintext'`.
- `pathToFileURL(path.resolve(filePath))` consistently used as URI source.
- Spawn flow: pipe stdio, merge `subprocessEnv()` then `config.env`, `windowsHide: true`, **await `'spawn'` event before using stdio**, then attach exit/error/stdin-error handlers, register `connection.onError`/`onClose` **before** `connection.listen()`, then enable `Trace.Verbose`.
- `sendRequest` retry: 4 attempts total on `code === -32801`, 500/1000/2000 ms backoff (`500 * 2^attempt`), duck-typed code check.
- `withTimeout` clears its timer in `finally`.
- Stop sequence: set `isStopping`, send `shutdown` request + `exit` notification (failures captured but cleanup continues), `connection.dispose()`, remove all listeners, `process.kill()`, reset state, re-throw shutdown error.
- Diagnostic registry: `randomUUID` keys for pending; `LRUCache(max:500)` for delivered; dedup key = `jsonStringify({message, severity, range, source||null, code||null})`; deliveries sorted by severity (`Error<Warning<Info<Hint`); cap `MAX_DIAGNOSTICS_PER_FILE=10`, `MAX_TOTAL_DIAGNOSTICS=30`; mark sent then delete from pending; record into delivered LRU; per-key `try/catch` falls through to "include anyway".
- `clearDeliveredDiagnosticsForFile(fileUri)` exists and must be called by file write/edit tools (spec 11) post-write.
- `passiveFeedback.registerLSPNotificationHandlers` validates each instance has `onNotification`; isolates per-server failures; tracks consecutive failures and emits `WARNING:` after ≥3.
- `formatDiagnosticsForAttachment` preserves LSP 0-based line/character verbatim; severity map 1→Error/2→Warning/3→Info/4→Hint with default `'Error'`; `code` stringified.
- LSP servers are plugin-only (no settings path).
- All public surface from §3 returned exactly (closure-based factories, no classes).

Spec is complete when: the manager singleton, the per-server lifecycle, the JSON-RPC client, the diagnostic registry, and the passive `publishDiagnostics` handler can be rebuilt against the cited line ranges to produce byte-equivalent JSON-RPC traffic for `initialize`, `initialized`, `textDocument/{didOpen,didChange,didSave,didClose,publishDiagnostics}`, `workspace/configuration` reply, `shutdown`, `exit`, and `$/setTrace`, with identical retry/backoff, identical dedup keys, and identical error strings (§6.5).

---

## 12. Open Questions / Unknowns

1. **`src/services/lsp/types.ts` is missing from the leaked tree** (imported by `LSPServerManager.ts:11`, `LSPServerInstance.ts:10`, `config.ts:7`). `ScopedLspServerConfig` and `LspServerState` were reconstructed from usage (§4.2, §4.3); any field never read by the in-scope files (e.g. plugin-scoped metadata) is invisible to this spec. Resolution: reads of `getPluginLspServers` (spec 28) and `lspPluginIntegration.ts` may surface additional fields.
2. **Gitignore filtering, batch=50, 5 s per-batch timeout, 10 MB file cap, UNC bypass** — the dispatch prompt cites these as "per 16 finding". They are **not present in `src/services/lsp/`**: no `.gitignore` parser, no 10 MB constant, no UNC string handling, no 5 s constant. They live above this layer (LSPTool — spec 16, or the file-read pipeline — spec 11). This spec accurately documents the LSP service surface; those constants are *not* an unresolved gap of the manager.
3. **Per-language LSP binary detection / PATH resolution** is **not done in this service** (§5.10). Discovery lives in `src/utils/plugins/lspPluginIntegration.ts` (owned by spec 28). The launcher here is a thin `spawn(command, args, …)` over the plugin-supplied config.
4. **Debounce / throttle on diagnostic delivery** — the registry has dedup + volume capping but no time-window debounce; servers' own debouncing dictates frequency. If a per-turn debounce is required by a downstream consumer, it sits on top of `checkForLSPDiagnostics`, not inside this service.
5. **`closeFile` is implemented but unused** (`LSPServerManager.ts:373-374` comment: *"Currently available but not yet integrated with compact flow"*). Compact integration (spec 07) is the natural caller.
6. **`workspace/didChangeWatchedFiles`, `workspace/symbol`, `textDocument/codeAction`, etc.** — not declared in `capabilities` and never sent by this service; if a server unilaterally sends them, the request hits the `workspace/configuration → null` shortcut only for that one method. Other reverse-requests will produce JSON-RPC "method not found" replies from `vscode-jsonrpc`'s default handler. No specific handling here.
7. **`startupTimeout` is opt-in only** — a misbehaving LSP server with no `startupTimeout` configured can hang `initialize()` indefinitely, blocking `state` in `'starting'`. There is no global default. Spec 28 may want to enforce a default in plugin loading.
8. **Notebook (`.ipynb`) handling is caller-layer** (per §4.4, §9). `LSPServerManager` performs no notebook carve-out — dispatch is purely `path.extname(filePath).toLowerCase()`. The §9.6 direction "exclude `.ipynb` from LSP" is therefore a **spec 11 + spec 16** requirement, not a spec 24 requirement. Cross-spec rollups: (a) spec 11 (`FileWriteTool`, `FileEditTool`) must gate `saveFile` / `clearDeliveredDiagnosticsForFile` on a non-`.ipynb` extension predicate, *or* `NotebookEditTool` must add the equivalent calls for the underlying `.ipynb` path so cross-turn dedup state stays consistent; (b) spec 16 (LSPTool) must filter `.ipynb` at input validation / `preparePermissionMatcher`. Currently neither is done — see Phase 9.6c findings on specs 11/16.
9. **`connection.onClose` does not call `onCrash`** (`LSPClient.ts:200-207` vs `:156-167`). The graceful-close zombie state — server closes stdout cleanly, `isInitialized` flips to false, but `LSPServerInstance.state` stays `'running'` because no exit-code-≠-0 fires — is a real defect: subsequent `sendRequest` fails `isHealthy()` with no `lastError` and `crashRecoveryCount` never increments, so `ensureServerStarted` does not auto-restart. Candidate for `BUGS-IN-SOURCE.md`. Fix would either bridge `onClose` to `onCrash` (with a guard against double-fire when `process.on('exit')` follows shortly after) or reset state in `onClose`.
