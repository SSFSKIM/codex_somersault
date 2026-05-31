# Proposal 03 — Port the LSP client into Codex as `codex-lsp`

> Status: draft spec, implementation-ready
> Audience: Rust engineer implementing the port
> Source-of-truth: Codex Rust workspace at `codex-rs/`; Claude Code TS reference at `Claude Code Src/`

## 1. Summary & motivation

Codex has **zero** Language Server Protocol support today — `grep -rni lsp` across
`codex-rs/config/src` and `codex-rs/core/src` returns nothing relevant (only false
positives like `elapsed`/`collapse`). There is no `lsp-types` dependency, no
JSON-RPC-over-stdio client, no model-facing code-intelligence tool. The gap is total.

This proposal ports Claude Code's three-layer LSP stack (transport → server instance →
routing manager) plus its passive diagnostic registry and model-facing `LspTool` into a
**new optional crate `codex-lsp`** at `codex-rs/ext/lsp-client/`. v1 buys the model four
high-value code-intelligence operations — diagnostics, go-to-definition, hover, references —
backed by real language servers (rust-analyzer, typescript-language-server, pyright, gopls),
plus automatic document sync so the server sees the model's edits.

**Why now:** the extension-tier API (`codex-extension-api`) and the `ext/` crate convention
already exist and are exercised by `ext/web-search`, `ext/memories`, `ext/image-generation`
(`codex-rs/ext/`). That machinery is exactly what an optional subsystem like LSP needs: a
`ToolContributor` injects model-visible tools into the per-turn registry **without editing
core tool registration** (`codex-rs/app-server/src/extensions.rs:31-36`). The only thing
missing is a place to hang the long-lived server processes and the one-line doc-sync hook in
the apply-patch runtime. Both are bounded.

**Intentional divergence from the TS reference:** Claude Code sources LSP server configs
*exclusively* from plugins (`Claude Code Src/src/services/lsp/config.ts`). v1 instead sources
them from `~/.codex/config.toml` under an `[lsp_servers]` table, mirroring the existing
`[mcp_servers]` pattern (`codex-rs/config/src/config_toml.rs:246-250`). Plugin-based config is
a future phase.

## 2. Source-of-truth behavior (Claude Code)

All citations below are into `/Users/new/Documents/GitHub/codex_somersault/Claude Code Src`.

### Three transport/lifecycle layers
- **Transport — `src/services/lsp/LSPClient.ts`.** Spawns the server binary as a child with
  stdio pipes; builds a `vscode-jsonrpc` `MessageConnection` over `StreamMessageReader`/
  `StreamMessageWriter`. Waits for the OS `spawn` event before touching streams (avoids ENOENT
  races). Consumes stderr to `tracing::debug`-equivalent. `shutdown()` = `shutdown` request +
  `exit` notification + process kill.
- **Lifecycle — `src/services/lsp/LSPServerInstance.ts`.** State machine
  stopped → starting → running | error → stopping → stopped. `initialize` declares UTF-16
  position encoding, `synchronization.didSave: true`, and explicit capabilities for
  publishDiagnostics/hover/definition/references/documentSymbol/callHierarchy but **NOT**
  `workspace/configuration` (avoids servers demanding config we can't provide). Sends
  `initialized` after `InitializeResult`. `sendRequest` retries up to **3** times with backoff
  **[500ms, 1s, 2s]** on JSON-RPC error **-32801** (ContentModified — rust-analyzer emits this
  during indexing). Startup-timeout race via `Promise.race`.
- **Routing — `src/services/lsp/LSPServerManager.ts`.** Builds `extensionMap: ext → serverName[]`
  (**first-registered wins** on conflict, `LSPServerManager.ts:201`). Lazy start
  (`ensureServerStarted`). `openedFiles: uri → serverName` dedups `didOpen`. `changeFile` sends
  a **full-content** `didChange` (single change entry, `TextDocumentSyncKind.Full`,
  `LSPServerManager.ts:291-331`); falls through to `openFile` if the file isn't open yet.
  Registers a `workspace/configuration` handler returning `null[]` (typescript-language-server
  sends this unconditionally).

### Document-sync wiring
`src/tools/FileWriteTool/FileWriteTool.ts` and `src/tools/FileEditTool/FileEditTool.ts`: after
writing to disk, call `clearDeliveredDiagnosticsForFile(uri)` → `changeFile(path, content)` →
`saveFile(path)`, fire-and-forget.

### Passive diagnostics
- `src/services/lsp/passiveFeedback.ts` registers a `textDocument/publishDiagnostics` handler on
  every server after init; maps numeric severity 1-4 → `Error`/`Warning`/`Info`/`Hint`; **drops
  empty diagnostic arrays** (server clearing diagnostics).
- `src/services/lsp/LSPDiagnosticRegistry.ts` stores UUID-keyed pending batches; on drain,
  dedups within-batch and across-turn (LRU keyed by `fileUri → Set<diagnosticKey>`), sorts by
  severity (Error first), caps **10/file** then **30 total**, marks sent.

### Model-facing tool
`src/tools/LSPTool/LSPTool.ts`: 9 operations; input `{operation, filePath, line, character}`
with **1-based** line/character converted to 0-based. Lazy `didOpen` on first access (rejects
files > **10 MB**). incomingCalls/outgoingCalls = two-step RPC (`prepareCallHierarchy` then
`callHierarchy/{incoming,outgoing}Calls`). Location results filtered via `git check-ignore` in
batches of 50. Gated by `ENABLE_LSP_TOOL=1`. Canonical config schema (NOT `types.ts`) is
`src/utils/plugins/schemas.ts:708-787`.

### Key constants (all from the research pack, verified against cited TS files)
| Constant | Value | Source |
|---|---|---|
| `MAX_DIAGNOSTICS_PER_FILE` | 10 | `LSPDiagnosticRegistry.ts:42` |
| `MAX_TOTAL_DIAGNOSTICS` | 30 | `LSPDiagnosticRegistry.ts:43` |
| `MAX_DELIVERED_FILES` (LRU) | 500 | `LSPDiagnosticRegistry.ts:46` |
| `LSP_ERROR_CONTENT_MODIFIED` | -32801 | `LSPServerInstance.ts:17` |
| `MAX_RETRIES_FOR_TRANSIENT_ERRORS` | 3 | `LSPServerInstance.ts:22` |
| `RETRY_BASE_DELAY_MS` | 500 (×2^n) | `LSPServerInstance.ts:27` |
| `MAX_LSP_FILE_SIZE_BYTES` | 10_000_000 | `LSPTool.ts:53` |
| `DEFAULT_MAX_RESTARTS` | 3 | `LSPServerInstance.ts:142,313` |
| `GIT_CHECK_IGNORE_BATCH_SIZE` | 50 | `LSPTool.ts:581` |
| position encoding | utf-16 | `LSPServerInstance.ts:234` |
| didOpen/didChange version | constant `1` | `LSPServerManager.ts:291-331` |

**Not implemented in TS (do NOT implement in v1):** `shutdownTimeout`, `restartOnCrash` both
throw "not yet implemented" (`LSPServerInstance.ts:96-103`). Validate-and-reject if set.

## 3. Target placement in Codex

**New crate: package `codex-lsp`, lib `codex_lsp`, directory `codex-rs/ext/lsp-client/`.**

The `ext/` directory is the established home for optional subsystems
(`codex-rs/ext/{web-search,memories,image-generation,goal,guardian}`). LSP is an optional
capability, not a core agent primitive, so it belongs there.

**codex-core avoidance rationale (fork prime directive):**
- The `LspManager` lifecycle, transport, server instances, diagnostic registry, the `LspTool`,
  and the `ToolContributor` all live in `codex-rs/ext/lsp-client/`. None of this touches
  `codex-core`.
- Model-tool injection requires **zero** core edits: an extension's `ToolContributor::tools()`
  is collected and routed via `extension_tool_executors`
  (`codex-rs/core/src/tools/router.rs:226`) — the registration path used by `ext/web-search`
  and `ext/memories`. We register through
  `codex-rs/app-server/src/extensions.rs:31-36`, the existing install seam.
- The **only** core edit is a single doc-sync call in the apply-patch runtime
  (`codex-rs/core/src/tools/runtimes/apply_patch.rs`). This is unavoidable because the changed
  file contents needed for `textDocument/didChange` live in the `AppliedPatchDelta` that core
  owns; the extension lifecycle hooks (`ToolFinishInput`) deliberately do **not** carry tool
  payloads or applied deltas (`codex-rs/ext/extension-api/src/contributors/tool_lifecycle.rs:65-82` —
  it carries session/thread/turn stores, ids, tool name, source, and `ToolCallOutcome`,
  but no patch content or applied delta). See §10-D for the chosen mechanism that keeps even this edit to a
  thin, optional, upstream-safe shape.

**Why `LspManager` can be held in `ExtensionData`:** `ExtensionData` stores
`Arc<dyn Any + Send + Sync>` (`codex-rs/ext/extension-api/src/state.rs:8`), so an
`Arc<LspManager>` (which owns spawned `tokio::process::Child` handles) can be inserted at
`on_thread_start` and borrowed in `ToolContributor::tools()` — exactly the
`MemoriesExtensionConfig` pattern (`codex-rs/ext/memories/src/extension.rs:72-111`).

## 4. Architecture

### Module layout (each file < 500 LoC)
```
codex-rs/ext/lsp-client/
  Cargo.toml            # package codex-lsp, lib codex_lsp
  BUILD.bazel           # two-line codex_rust_crate stanza
  src/
    lib.rs              # re-exports: LspManager, LspTool, install(); module decls
    config.rs           # ResolvedLspServerConfig (runtime view of config-toml type)
    transport.rs        # LspTransport: spawn + Content-Length framing + request/notify
    instance.rs         # LspServerInstance: state machine, init, retry, restart
    manager.rs          # LspServerManager: routing, lazy start, doc sync, shutdown
    diagnostics.rs      # DiagnosticRegistry + publishDiagnostics handler
    tool.rs             # LspTool: ToolExecutor<ToolCall> impl
    format.rs           # human-readable formatting of LSP results
    extension.rs        # LspExtension: ThreadLifecycle + Config + Tool contributors; install()
    *_tests.rs          # co-located unit tests (pretty_assertions)
```

### Config type (lives in `codex-config`, see §6)
```rust
// codex-rs/config/src/lsp_config.rs  (new file)
use std::collections::HashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One configured language server, keyed by a user-chosen name under
/// `[lsp_servers.<name>]` in config.toml. Mirrors the `[mcp_servers]` raw/validated
/// split (`codex-rs/config/src/mcp_types.rs`).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct LspServerConfig {
    /// Executable to spawn (e.g. "rust-analyzer", "typescript-language-server").
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// REQUIRED, non-empty. e.g. {".ts" = "typescript", ".tsx" = "typescriptreact"}.
    /// Keys are file extensions WITH the leading dot, lowercased at load time.
    pub extension_to_language: HashMap<String, String>,
    /// Defaults to the session cwd when absent.
    #[serde(default)]
    pub workspace_folder: Option<String>,
    /// Forwarded verbatim as `initialize.initializationOptions`.
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,
    /// No timeout when absent (matches TS default).
    #[serde(default)]
    pub startup_timeout_ms: Option<u64>,
    /// Crash-recovery restart cap; default 3.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
}

const fn default_max_restarts() -> u32 { 3 }
```
The crate carries a `ResolvedLspServerConfig` (config.rs) that holds the validated runtime view
(absolute `workspace_folder`, lowercased extensions) so the transport never re-validates.

### Transport (transport.rs)
```rust
use lsp_types::{InitializeParams, InitializeResult, ServerCapabilities};
use serde_json::Value;
use tokio::process::Child;

/// Async JSON-RPC client over a child process's stdio using LSP Content-Length framing.
/// Hand-rolled framer (~120 LoC): a writer task drains an mpsc of outgoing frames into
/// child stdin; a reader task parses `Content-Length` headers from stdout and routes by
/// id into a pending-request map, or dispatches notifications/server-requests to handlers.
pub(crate) struct LspTransport {
    child: Child,
    outgoing: tokio::sync::mpsc::Sender<Vec<u8>>,
    pending: Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, JsonRpcError>>>>>,
    next_id: AtomicI64,
    reader_task: tokio::task::JoinHandle<()>,
    writer_task: tokio::task::JoinHandle<()>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum JsonRpcError {
    #[error("json-rpc error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("transport closed")]
    Closed,
}

impl LspTransport {
    /// Spawns `command` and starts reader/writer tasks. `on_exit` fires with the exit
    /// status when the process terminates (drives crash recovery in the instance layer).
    pub(crate) async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &std::path::Path,
        on_exit: impl FnOnce(std::process::ExitStatus) + Send + 'static,
    ) -> std::io::Result<Self>;

    /// Sends a request and awaits the typed result.
    pub(crate) async fn request<P: Serialize, R: DeserializeOwned>(
        &self, method: &str, params: P,
    ) -> Result<R, JsonRpcError>;

    /// Sends a notification (no response expected).
    pub(crate) async fn notify<P: Serialize>(&self, method: &str, params: P)
        -> Result<(), JsonRpcError>;

    /// Registers a handler for inbound server-push notifications (e.g. publishDiagnostics).
    pub(crate) fn on_notification(
        &self, method: &str, handler: Arc<dyn Fn(Value) + Send + Sync>,
    );

    /// Registers a server->client request responder (e.g. workspace/configuration -> null[]).
    pub(crate) fn on_request(
        &self, method: &str, responder: Arc<dyn Fn(Value) -> Value + Send + Sync>,
    );

    /// shutdown request + exit notification + kill (SIGTERM then SIGKILL after 2s).
    pub(crate) async fn shutdown(self);
}
```
> Library choice: depend on `lsp-types` (crates.io, type-safe protocol structs) and hand-roll
> the framer rather than pull `async-lsp` (~5 transitive deps). See §10-A.

### Server instance (instance.rs)
```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LspServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

pub(crate) struct LspServerInstance {
    name: String,
    config: ResolvedLspServerConfig,
    state: Mutex<LspServerState>,
    transport: Mutex<Option<Arc<LspTransport>>>,
    capabilities: Mutex<Option<ServerCapabilities>>,
    crash_recovery_count: AtomicU32,   // unexpected exits; reset to 0 on successful start
    /// Handlers registered before the connection is live; re-applied after each (re)start.
    notification_handlers: Mutex<HashMap<String, Arc<dyn Fn(Value) + Send + Sync>>>,
}

impl LspServerInstance {
    pub(crate) async fn start(&self) -> Result<(), LspError>;     // idempotent; respects max_restarts
    pub(crate) async fn stop(&self) -> Result<(), LspError>;
    pub(crate) fn state(&self) -> LspServerState;
    pub(crate) fn is_healthy(&self) -> bool;                      // state == Running

    /// ContentModified(-32801) retry: up to 3 attempts, sleep 500ms*2^n between.
    pub(crate) async fn request<R: DeserializeOwned>(
        &self, method: &str, params: Value,
    ) -> Result<R, LspError>;

    pub(crate) async fn notify(&self, method: &str, params: Value) -> Result<(), LspError>;

    /// Stores the handler and, if the connection is live, registers it immediately.
    pub(crate) fn on_notification(&self, method: &str, handler: Arc<dyn Fn(Value) + Send + Sync>);
}
```

### Routing manager (manager.rs)
```rust
/// Owns all server instances; routes by file extension; tracks open documents.
/// One per thread; held as Arc<LspManager> in thread-scoped ExtensionData.
pub struct LspManager {
    servers: HashMap<String, Arc<LspServerInstance>>,   // keyed by server name
    extension_map: HashMap<String, String>,             // ".ts" -> server name (first wins)
    opened_files: Mutex<HashMap<String, String>>,       // file:// uri -> server name
    diagnostics: Arc<DiagnosticRegistry>,
    cwd: std::path::PathBuf,
}

impl LspManager {
    /// Validates configs (command non-empty, extension_to_language non-empty; log+skip on
    /// error), builds the extension map (first wins), creates instances. Does NOT start them.
    pub fn new(configs: HashMap<String, LspServerConfig>, cwd: std::path::PathBuf) -> Arc<Self>;

    pub fn diagnostics(&self) -> Arc<DiagnosticRegistry>;

    /// Lazy start: starts the routed server if Stopped/Error, returns it.
    async fn ensure_started(&self, file_path: &Path) -> Option<Arc<LspServerInstance>>;

    pub async fn open_file(&self, file_path: &Path, content: &str);
    pub async fn change_file(&self, file_path: &Path, content: &str);  // falls through to open
    pub async fn save_file(&self, file_path: &Path);
    pub async fn close_file(&self, file_path: &Path);
    pub fn is_file_open(&self, file_path: &Path) -> bool;

    /// Routes by extension, ensures started, issues the request.
    pub async fn request<R: DeserializeOwned>(
        &self, file_path: &Path, method: &str, params: Value,
    ) -> Option<Result<R, LspError>>;

    /// Drives the apply-patch doc-sync. Maps AppliedPatchFileChange variants:
    ///   Add -> open_file; Update -> change_file+save_file; Delete -> close_file.
    /// Clears delivered diagnostics for each touched uri first. Fire-and-forget per file.
    pub async fn notify_file_changes(&self, changes: &[FileChange]);

    pub async fn shutdown(&self);  // stop all Running/Error servers, clear maps
}

/// Minimal, dependency-free view of one committed change passed in from core.
pub struct FileChange {
    pub path: std::path::PathBuf,
    pub kind: FileChangeKind,
}
pub enum FileChangeKind { Added(String), Updated(String), Deleted }
```

### Diagnostic registry (diagnostics.rs)
```rust
pub struct DiagnosticRegistry {
    pending: Mutex<HashMap<uuid::Uuid, PendingDiagnostic>>,
    delivered: Mutex<LruCache<String, HashSet<String>>>,  // file_uri -> {diagnostic_key}; cap 500; use lru 0.16 API: LruCache::new(NonZeroUsize::new(MAX_DELIVERED_FILES).unwrap())
}

pub struct DiagnosticFile {
    pub uri: String,
    pub diagnostics: Vec<DiagnosticEntry>,
}
pub struct DiagnosticEntry {
    pub severity: DiagnosticSeverity,   // Error/Warning/Info/Hint
    pub message: String,
    pub range: lsp_types::Range,
    pub source: Option<String>,
    pub code: Option<String>,
}

impl DiagnosticRegistry {
    pub fn register_pending(&self, server_name: &str, files: Vec<DiagnosticFile>);
    /// Drain unsent; cross-turn + within-batch dedup; severity sort (Error first);
    /// cap 10/file then 30 total; mark sent; update delivered LRU.
    pub fn check_for_diagnostics(&self) -> Vec<DiagnosticFile>;
    pub fn clear_delivered_for_file(&self, file_uri: &str);
}
```

### Model-facing tool (tool.rs)
```rust
use codex_extension_api::{ToolCall, ToolExecutor, ToolName, ToolOutput, ToolSpec, FunctionCallError};
use codex_tools::ToolExposure;  // ToolExposure is NOT re-exported by codex_extension_api; import directly from codex_tools (codex-rs/tools/src/lib.rs:94). codex-tools must be a direct dep in Cargo.toml.

pub(crate) struct LspTool {
    manager: Arc<LspManager>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspToolInput {
    operation: LspOperation,
    file_path: String,
    line: u32,        // 1-based
    character: u32,   // 1-based
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum LspOperation {
    GoToDefinition, FindReferences, Hover, DocumentSymbol,
    WorkspaceSymbol, GoToImplementation, PrepareCallHierarchy,
    IncomingCalls, OutgoingCalls,
}

#[async_trait::async_trait]
impl ToolExecutor<ToolCall> for LspTool {
    fn tool_name(&self) -> ToolName { ToolName::namespaced("lsp", "query") }
    fn spec(&self) -> ToolSpec { /* ToolSpec::Namespace { Function(ResponsesApiTool {..}) } */ }
    fn exposure(&self) -> ToolExposure { ToolExposure::Direct }
    fn supports_parallel_tool_calls(&self) -> bool { true }   // read-only
    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError>;
}
```
> v1 exposes all 9 operations through one namespaced tool (matching the TS single-tool shape).
> The four mandated by v1 scope (diagnostics-via-registry + definition + hover + references)
> are first-class; the remaining five (documentSymbol, workspaceSymbol, implementation, and the
> two call-hierarchy ops) ride along for free since the dispatch is uniform. Diagnostics are
> delivered passively via the registry (see §7), not through this tool's request path.

### Data-flow diagram
```
config.toml [lsp_servers]
        │ (codex-config deserialize)
        ▼
ConfigToml.lsp_servers ── Config ──► LspExtension::on_thread_start
                                          │ LspManager::new(configs, cwd)
                                          ▼
                              thread_store.insert(Arc<LspManager>)
                                          │
          ┌───────────────────────────────┼───────────────────────────────┐
          ▼                                ▼                                ▼
  ToolContributor::tools()        apply_patch runtime              publishDiagnostics
  reads Arc<LspManager>           (core, ONE call)                 (server push)
          │                                │                                │
          ▼                                ▼                                ▼
     LspTool (model)            notify_file_changes(changes)      DiagnosticRegistry
          │ request()                 │ didOpen/didChange/didSave        .register_pending
          ▼                           ▼                                   │
   LspManager.request ──► LspServerInstance.request ──► LspTransport ──► language server
          ▲                                                                │
          └──────────────── after turn: check_for_diagnostics() ◄──────────┘
                            (injected as turn item / tool attachment)
```

## 5. Integration points (exact files in `codex-rs/`)

| File | Line | Change |
|---|---|---|
| `codex-rs/Cargo.toml` | `2` (`members`), after `51` (`ext/web-search`) | Add `"ext/lsp-client"` to `[workspace.members]`. Add `codex-lsp = { path = "ext/lsp-client" }` near `185` in `[workspace.dependencies]`. Add `lsp-types = "0.97"` to `[workspace.dependencies]`. **Do not add `lru`** — `lru = "0.16.3"` is already present at `Cargo.toml:314`; `codex-lsp` should reference it as `lru = { workspace = true }`. |
| `codex-rs/ext/lsp-client/Cargo.toml` | new | `package.name = "codex-lsp"`, `lib.name = "codex_lsp"`. Deps: `codex-extension-api`, `codex-tools` (direct dep required — `ToolExposure` is exported from `codex_tools::ToolExposure` at `codex-rs/tools/src/lib.rs:94` and is **not** re-exported by `codex-extension-api`), `codex-protocol`, `codex-config`, `async-trait`, `serde`, `serde_json`, `schemars`, `thiserror`, `tokio` (features `io-util,process,sync,macros,rt`), `tracing`, `lsp-types`, `lru = { workspace = true }`, `uuid`, `url`. Dev-deps: `pretty_assertions`. Model verbatim on `codex-rs/ext/web-search/Cargo.toml`. |
| `codex-rs/ext/lsp-client/BUILD.bazel` | new | `load("//:defs.bzl", "codex_rust_crate")` + `codex_rust_crate(name = "lsp-client", crate_name = "codex_lsp")`. No `compile_data` unless `include_str!` is used for the tool description (if so, add it — AGENTS.md include_str rule). Template: `codex-rs/ext/web-search/BUILD.bazel`. |
| `codex-rs/config/src/lsp_config.rs` | new | `LspServerConfig` type (see §4). |
| `codex-rs/config/src/lib.rs` | — | `mod lsp_config; pub use lsp_config::LspServerConfig;` (match how `mcp_types` is re-exported). |
| `codex-rs/config/src/config_toml.rs` | `250` (after `mcp_servers`) | Add field: `#[serde(default)] pub lsp_servers: HashMap<String, LspServerConfig>,` with a doc comment. (Unlike `mcp_servers`, no custom `schema_with` is needed because `LspServerConfig` derives `JsonSchema` directly.) Then propagate into the runtime `Config` struct following the `mcp_servers` path so `LspExtension` can read it. |
| `codex-rs/app-server/src/extensions.rs` | between lines `35` and `36` (after `codex_image_generation_extension::install`, before `Arc::new(builder.build())`) | `codex_lsp::install(&mut builder);` — the install seam (no args needed; reads `Config` in `on_thread_start`). |
| `codex-rs/core/src/tools/runtimes/apply_patch.rs` | after `self.committed_delta.append(delta)` (~`246`) | The doc-sync hook (see §10-D). **Two changes required:** (1) rename `_ctx` → `ctx` at line 224 (currently suppressed as unused); (2) add one call to `crate::lsp_sync::notify_if_present(ctx.session.services.thread_extension_data, &changes)`. The helper reads `Arc<dyn ApplyPatchDocSync>` (trait in `codex-extension-api`) from thread store — avoids a circular dep. The delta's `AppliedPatchChange { path, change }` (`codex-rs/apply-patch/src/lib.rs:228-248`) carries `Add{content}` / `Update{new_content}` / `Delete{content}` — exactly what didOpen/didChange/didClose need; no disk re-read. |

**Citations of the machinery being reused (no edits needed):**
- `codex-rs/ext/extension-api/src/contributors.rs:123-134` — `ToolContributor::tools()` trait (declaration at 123, `fn tools()` signature at 125-129).
- `codex-rs/ext/extension-api/src/contributors.rs:47-65` — `ThreadLifecycleContributor::on_thread_start`.
- `codex-rs/ext/extension-api/src/state.rs:32-63` — `ExtensionData::get/insert` (holds `Arc<LspManager>`).
- `codex-rs/tools/src/tool_executor.rs:40-59` — `ToolExecutor<Invocation>` (already `#[async_trait]`).
- `codex-rs/apply-patch/src/lib.rs:187-248` — `AppliedPatchDelta::changes()` + `AppliedPatchChange`.

## 6. Config & schema changes

1. **New type** `codex-rs/config/src/lsp_config.rs` (`LspServerConfig`, `#[derive(JsonSchema)]`).
2. **New field** `lsp_servers: HashMap<String, LspServerConfig>` on `ConfigToml`
   (`config_toml.rs:250`) with `#[serde(default)]`, plus the runtime `Config` propagation.
3. **Regenerate the config schema in the same change** (AGENTS.md line 38):
   ```bash
   just write-config-schema    # regenerates codex-rs/core/config.schema.json
   ```
   Commit the regenerated `codex-rs/core/config.schema.json`.
4. **No hooks-schema / app-server-schema change in v1.** The tool is injected via the extension
   `ToolContributor` and surfaces through the existing tool list; no new app-server RPC types are
   added. (If a future phase adds an `lsp/*` app-server method or an LSP TUI surface, run
   `just write-app-server-schema` then.)
5. **Bazel reconciliation (deps changed):** after editing `codex-rs/Cargo.toml`:
   ```bash
   just bazel-lock-update && just bazel-lock-check   # run from repo root
   ```
   Commit the `MODULE.bazel.lock` update. The new `BUILD.bazel` must list any `include_str!`
   inputs under `compile_data` or Bazel breaks even when Cargo passes (AGENTS.md include_str rule).

Example user config:
```toml
[lsp_servers.rust]
command = "rust-analyzer"
extension_to_language = { ".rs" = "rust" }

[lsp_servers.typescript]
command = "typescript-language-server"
args = ["--stdio"]
extension_to_language = { ".ts" = "typescript", ".tsx" = "typescriptreact" }
startup_timeout_ms = 30000
```

## 7. Implementation workflow (phased)

### Phase 1 — v1 (minimal, upstream-mergeable)
Delivers: config-driven multi-server spawn, per-extension routing, basic lifecycle, document
sync wired to file edits, diagnostics registry, and an `LspTool` exposing diagnostics +
go-to-definition + hover + references (plus the other 5 ops for free).

1. **Crate bootstrap.** Create `ext/lsp-client/{Cargo.toml,BUILD.bazel,src/lib.rs}`. Add to
   workspace members + dependencies (incl. `lsp-types`). Empty `pub fn install(_: &mut
   ExtensionRegistryBuilder<Config>) {}`.
   - *Checkpoint:* `just fix -p codex-lsp` and `cargo build -p codex-lsp` succeed;
     `just bazel-lock-check` passes.
2. **Config type + ConfigToml field + schema.** Add `LspServerConfig`, wire `lsp_servers`, run
   `just write-config-schema`.
   - *Checkpoint:* a unit test in `config` deserializes the example TOML above into the expected
     struct; `git diff codex-rs/core/config.schema.json` shows the new `lsp_servers` block.
3. **Transport.** Implement `transport.rs` (spawn + Content-Length framer + request/notify/
   handlers + shutdown).
   - *Checkpoint:* `transport_tests.rs` drives a **mock stdio echo server** (a tiny script, or
     an in-process `tokio::io::duplex` fake child) and asserts a round-trip request/response and
     a delivered notification. `just test -p codex-lsp transport`.
4. **Instance.** `instance.rs`: state machine, `initialize` with the exact capability block
   (UTF-16, didSave, no workspace/configuration), `initialized`, ContentModified retry
   [500/1000/2000], crash-recovery counter, `workspace/configuration → null[]` responder,
   handler re-application after restart.
   - *Checkpoint:* `instance_tests.rs` with a mock server asserts: state transitions
     stopped→running; a `-32801` response triggers exactly 3 attempts with the right backoff
     (use a paused tokio clock); `workspace/configuration` is answered with `null[]`.
5. **Diagnostic registry.** `diagnostics.rs`: severity mapping (1-4 → enum), within-batch +
   cross-turn dedup, severity sort, caps 10/30, LRU 500, `clear_delivered_for_file`.
   - *Checkpoint:* table-driven `diagnostics_tests.rs`: empty publishDiagnostics dropped;
     dedup across two `check_for_diagnostics` calls; caps enforced; Error sorts before Warning.
6. **Manager.** `manager.rs`: validation/skip, first-wins extension map, lazy start, open/change/
   save/close with `openedFiles` dedup and full-content `didChange`, `request` routing,
   `notify_file_changes` mapping Add/Update/Delete, `shutdown`. Register the
   `publishDiagnostics` handler on each instance feeding the registry.
   - *Checkpoint:* `manager_tests.rs` with two mock servers: routing picks the right server by
     extension; first-wins on conflicting extensions; `change_file` on an unopened file falls
     through to `open_file`; `notify_file_changes([Update])` emits didChange then didSave.
7. **Tool + formatters.** `tool.rs` + `format.rs`: input parsing, 1→0-based conversion, lazy
   open (10 MB guard), 9-op dispatch, two-step call hierarchy, `git check-ignore` batch-50
   filtering (skip when not a git repo / exit 128), human-readable output via `JsonToolOutput`
   or a custom `ToolOutput`.
   - *Checkpoint:* `tool_tests.rs`: hover/definition/references against a mock server return
     formatted strings; line/character off-by-one verified (input 1,1 → request 0,0); a 10 MB+
     file is rejected with a size error.
8. **Extension wiring.** `extension.rs`: `LspExtension` implements `ThreadLifecycleContributor`
   (builds `Arc<LspManager>` from `Config.lsp_servers`; also stores it as
   `Arc<dyn ApplyPatchDocSync>` in thread store so `apply_patch.rs` can reach it without a
   circular dep; skips creation when `session_source == SessionSource::Exec` or another
   non-interactive source), `ConfigContributor` (rebuild on config change), and
   `ToolContributor` (emit `LspTool` reading the manager from the store). `install()` registers
   all three. Add `codex_lsp::install(&mut builder)` between lines 35 and 36 of
   `app-server/src/extensions.rs` (before `Arc::new(builder.build())`). Also add the
   `ApplyPatchDocSync` trait to `codex-extension-api` in the same step.
   - *Checkpoint:* an `extension_tests.rs` analogous to
     `ext/web-search/src/extension.rs:152-179` asserts the tool is contributed when
     `lsp_servers` is non-empty and absent when empty.
9. **Core doc-sync hook.** Add the two changes in
   `core/src/tools/runtimes/apply_patch.rs` (§10-D mechanism): rename `_ctx` → `ctx`, then
   add the `notify_if_present` call. Headless/exec sessions must NOT start servers — gate in
   `LspExtension::on_thread_start` by checking `input.session_source`:
   `ThreadStartInput.session_source` (`codex-rs/ext/extension-api/src/contributors/thread_lifecycle.rs:9`)
   already carries `&SessionSource`; skip `LspManager` creation and store insertion when
   `*input.session_source == SessionSource::Exec` (or any non-interactive source). The
   apply-patch hook is then a no-op automatically (no manager in store).
   - *Checkpoint:* `just test -p codex-core` (apply-patch runtime tests still pass; the hook is a
     no-op when no manager is present). Manual smoke: `just c` with a `[lsp_servers.rust]` config
     in a Rust repo, ask the model to hover a symbol; confirm a real rust-analyzer response.
10. **Diagnostics delivery.** After each turn, call `registry.check_for_diagnostics()` and inject
    results. v1 minimal: surface them as part of the `LspTool` not needed — instead inject as a
    developer/system turn item via the extension `ContextContributor` or a turn-lifecycle hook.
    (Pick the injection point that matches where Codex polls between turns — see §10-E.)
    - *Checkpoint:* integration test (below) shows diagnostics injected exactly once per change.

**Phase 1 done-definition:** `just fmt && just fix -p codex-lsp && just test -p codex-lsp`
green; `just test -p codex-core` green; `just write-config-schema` produces no further diff;
`just bazel-lock-check` clean.

### Phase 2 — Robustness
- Startup-timeout race (config `startup_timeout_ms`).
- Crash recovery on in-flight request failure (process exit → state Error → restart next use,
  respecting `max_restarts`).
- Inspect `InitializeResult.capabilities.position_encoding` and adapt offsets if a server
  rejects UTF-16 (the TS harness never validates this — Rust should, per the gotcha).
- Per-document version counter to enable `TextDocumentSyncKind.Incremental`.

### Phase 3 — Parity & reach
- Plugin-sourced LSP configs (the TS canonical source) layered on top of config.toml.
- TUI surface for LSP server status (would require `insta` snapshots).
- `lsp/*` app-server RPC methods (would require `just write-app-server-schema`).
- Windows UNC-path skip + `windows-sandbox` interplay.

## 8. Test strategy

- **Unit (nextest, co-located `*_tests.rs`)** in `codex-lsp`, run with `just test -p codex-lsp`:
  - `transport_tests.rs` — round-trip + notification dispatch against a `tokio::io::duplex`
    fake child or a tiny mock-server script; framer header parsing edge cases.
  - `instance_tests.rs` — state transitions; ContentModified backoff (paused tokio clock);
    `workspace/configuration → null[]`.
  - `diagnostics_tests.rs` — severity map, dedup, caps (10/30), LRU eviction at 500, empty-drop.
  - `manager_tests.rs` — routing, first-wins, change-falls-through-to-open, didChange+didSave.
  - `tool_tests.rs` — 1→0-based conversion, lazy-open 10 MB guard, two-step call hierarchy,
    git-ignore filter exit-code handling, formatter output (`pretty_assertions::assert_eq`).
  - `extension_tests.rs` — `ToolContributor` emits/omits the tool based on `lsp_servers`.
- **Config (nextest)** in `codex-config`, `just test -p codex-config`: TOML round-trip for
  `LspServerConfig`; rejection of unknown fields and of `shutdownTimeout`/`restartOnCrash`.
- **Core (nextest)** `just test -p codex-core`: apply-patch runtime tests still pass with the
  doc-sync hook (no-op when no manager); a test that, given a stub manager in the thread store,
  a successful apply triggers `notify_file_changes` with the expected `FileChange` kinds.
- **Integration with mock language server:** a fixture binary/script that speaks LSP over stdio
  (responds to `initialize`, returns canned hover/definition, pushes one `publishDiagnostics`).
  Drive the full `LspManager` + `LspTool` path end-to-end; assert the diagnostics registry
  delivers exactly once and that the tool returns the formatted hover. This avoids depending on
  rust-analyzer in CI.
- **No `insta` snapshots in v1** (no TUI surface). Add them only if Phase 3 adds a TUI panel
  (AGENTS.md UI-snapshot rule).
- After Rust changes: `just fmt`, then `just test -p codex-lsp` / `-p codex-config`, then
  `just fix -p codex-lsp`. Run full `just test` only if `core`/`config`/`protocol` change, and
  ask first (per CLAUDE.md).

## 9. Upstream-merge safety analysis

**Footprint.** New code is 100% in a new crate `ext/lsp-client/` plus a new
`config/src/lsp_config.rs`. Edits to pre-existing upstream files are exactly five, all additive:
- `codex-rs/Cargo.toml` — append one member + two workspace deps (`codex-lsp`, `lsp-types`; `lru` already present at line 314, no new entry needed).
- `codex-rs/config/src/config_toml.rs:250` — one `#[serde(default)]` field (additive; the
  surrounding struct rarely sees positional churn).
- `codex-rs/config/src/lib.rs` — one `mod`+`pub use` line.
- `codex-rs/app-server/src/extensions.rs` — one `install()` call inserted between lines 35 and
  36 (before `Arc::new(builder.build())`), in a *fork-local glue* function already listing
  `memories`/`web-search`/`image-generation`.
- `codex-rs/core/src/tools/runtimes/apply_patch.rs` — **two** logical changes: rename `_ctx` →
  `ctx` (required to access the parameter, currently unused at line 224), plus one delegating
  call to `crate::lsp_sync::notify_if_present(ctx.session.services.thread_extension_data, ...)`.
  All logic lives in the new fork-local module `codex-core/src/lsp_sync.rs`.
- `codex-rs/ext/extension-api/src/contributors.rs` (or a new file in that crate) — one new
  `pub trait ApplyPatchDocSync` (additive; `codex-extension-api` is a fork-local crate, not
  upstream).

**Conflict risk on hot upstream files.** The only genuinely hot upstream file touched is
`apply_patch.rs`. Mitigation: keep the diff to two statements (parameter rename + one call) that
delegate into the fork-local helper; the logic lives in a new module `lsp_sync.rs`.
`config_toml.rs` is large but the field addition is at the tail of the struct's field list;
serde field order is irrelevant, so a merge can place it anywhere.

**Isolation strategy.** All behavior lives behind the extension `ToolContributor`/lifecycle
traits, which are themselves fork additions (`ext/extension-api`). The core hook is a no-op
unless an `Arc<LspManager>` is present in the thread store, so upstream code paths are unchanged
when `[lsp_servers]` is empty. No upstream trait signatures change. No `CODEX_SANDBOX_*` code is
touched (AGENTS.md line 15) — LSP servers are spawned directly via `tokio::process::Command`
with the config-provided `command`/`args`/`env`/`cwd`, outside sandbox env-var paths.

## 10. Open design decisions & tradeoffs

**A. LSP transport library.**
- Options: (1) hand-rolled Content-Length framer over `tokio` + `lsp-types`; (2) `async-lsp`
  crate (tower-service client, ~5 transitive deps); (3) `lsp-server` (sync, designed for
  *server* side).
- Recommendation: **(1) hand-rolled framer + `lsp-types`.** The framer is ~120 LoC, keeps the
  dep surface minimal (Bazel-friendly), and avoids tower-service shape mismatch. `lsp-types`
  gives type-safe params/results without owning the IO.

**B. Where the `LspManager` lives.**
- Options: (1) thread-scoped `ExtensionData` only; (2) a dedicated `Services.lsp_manager:
  Option<Arc<LspManager>>` field on `core` `Session`; (3) a process-global singleton (TS shape).
- Recommendation: **(1) thread-scoped `ExtensionData`.** It needs zero core struct edits
  (`state.rs` already stores `Arc<dyn Any>`), matches the `MemoriesExtensionConfig` precedent,
  and scopes servers to a thread (correct for multi-thread sessions). The apply-patch hook reads
  the manager out of the thread store rather than a core field.

**C. One namespaced tool vs. many.**
- Options: (1) single `lsp/query` tool with an `operation` enum (TS shape); (2) one tool per
  operation (`lsp_hover`, `lsp_definition`, …).
- Recommendation: **(1) single tool.** Mirrors the TS source-of-truth, keeps the model's tool
  list small, and the uniform dispatch makes adding ops trivial. The four v1-mandated ops are
  documented prominently in the tool description.

**D. The core doc-sync hook mechanism.**

> **Blocker resolved:** `apply_patch.rs` lives in `codex-core`, which cannot import
> `codex_lsp::LspManager` — that would create a `codex-lsp ← codex-core ← codex-lsp` cycle
> that Cargo rejects. The fix is a thin trait in `codex-extension-api` (which `codex-core`
> already depends on at `codex-rs/core/Cargo.toml:38`).

- Options: (1a) **trait object via `codex-extension-api`** — define
  `trait ApplyPatchDocSync: Send + Sync { fn notify_patch_changes(&self, changes: &[FileChange]); }`
  in `codex-rs/ext/extension-api/src/contributors.rs` (or a new file in that crate);
  `LspManager` implements it in `codex-lsp`; `apply_patch.rs` reads
  `Arc<dyn ApplyPatchDocSync>` from `ExtensionData` — **no cycle**; (1b) read
  `Arc<LspManager>` directly — rejected, creates a compile cycle; (2) add a
  `ToolLifecycleContributor` — rejected, `ToolFinishInput` carries no payload/delta
  (`ext/extension-api/src/contributors/tool_lifecycle.rs:65-82`); (3) add a callback param
  to `codex-apply-patch::apply_patch` — rejected, edits an upstream crate.
- Recommendation: **(1a).** Define `ApplyPatchDocSync` in `codex-extension-api` (one additive
  file or a new `pub trait` in `contributors.rs`). `LspExtension::on_thread_start` inserts
  `Arc<LspManager>` as `Arc<dyn ApplyPatchDocSync>` into `thread_store`. The helper in
  `apply_patch.rs` becomes:
  ```rust
  // in a fork-local module codex-core/src/lsp_sync.rs
  pub(crate) fn notify_if_present(thread_store: &ExtensionData, changes: &[AppliedPatchChange]) {
      if let Some(sync) = thread_store.get::<dyn ApplyPatchDocSync>() {
          sync.notify_patch_changes(&map_to_file_changes(changes));
      }
  }
  ```
  The diff on `apply_patch.rs` is **two** logical changes (not one): rename `_ctx` → `ctx` to
  stop suppressing the unused-variable warning (the parameter is currently
  `_ctx: &ToolCtx` at `apply_patch.rs:224`), then add one call to `notify_if_present`. Both
  changes are required; count the diff as two lines on the hot file.
  This adds one fifth upstream touch-point (`codex-extension-api`), but the edit is
  additive (a new trait) and `codex-extension-api` is a fork-local crate.

**E. Diagnostics injection point.**
- Options: (1) `ContextContributor` injecting a prompt fragment before the next turn; (2) a
  turn-lifecycle hook emitting a synthetic event; (3) attach to the `LspTool` output.
- Recommendation: **(1) `ContextContributor`** for v1 (the memories extension already uses this
  exact path, `ext/memories/src/extension.rs:49-70`), polling `check_for_diagnostics()` at
  prompt assembly. Revisit if it should instead be a between-turns event to match the TS
  `checkForLSPDiagnostics()` poll site. Flag as the one behavior whose exact placement should be
  confirmed against where Codex naturally drains between-turn state.

## 11. Risks & mitigations

- **Process leaks on crash/exit.** Spawned servers could outlive the thread. Mitigation:
  `LspManager::shutdown()` on `on_thread_stop` (kill via SIGTERM→SIGKILL); never start servers
  in headless/exec mode.
- **Blocking the agent loop.** A slow/hung server could stall doc-sync or tool calls. Mitigation:
  doc-sync is fire-and-forget (spawned, not awaited inline); tool requests honor
  `startup_timeout_ms` (Phase 2) and the ContentModified retry caps total wait at ~3.5s.
- **Position-encoding mismatch.** We declare UTF-16 but some servers negotiate UTF-8.
  Mitigation: v1 declares UTF-16 (universal default); Phase 2 inspects
  `InitializeResult.capabilities.position_encoding` and adapts.
- **Sandbox interaction.** Spawning arbitrary `command` from config sidesteps sandboxing.
  Mitigation: spawn directly via `tokio::process::Command` outside the `CODEX_SANDBOX_*` paths
  (which we must not touch); document that `[lsp_servers]` commands run with the user's trust,
  like `[mcp_servers]`.
- **Bazel/Cargo drift.** New deps break Bazel if `MODULE.bazel.lock` isn't synced or
  `include_str!` inputs aren't in `compile_data`. Mitigation: run `just bazel-lock-update &&
  just bazel-lock-check`; keep the tool description inline (no `include_str!`) in v1, or list it
  in `BUILD.bazel`.
- **Config-schema drift.** Forgetting `just write-config-schema` lands a stale schema.
  Mitigation: run it in the same change; CI diff catches it.
- **First-wins extension routing is silent.** Two servers claiming `.ts` — the second is ignored.
  Mitigation: log the conflict at `warn`; document the rule. No priority system in v1.

## 12. Out of scope / future work (deferred parity items)

- Plugin-sourced LSP configs (`.lsp.json` / `manifest.lspServers`) — the TS canonical source.
- Startup-timeout enforcement and crash recovery on in-flight failures (Phase 2).
- Incremental `didChange` with a per-document version counter (v1 is Full sync, version=1).
- `shutdownTimeout` / `restartOnCrash` config fields — unimplemented in TS; v1 validates and
  rejects them.
- TUI surface for server status (needs `insta` snapshots) and `lsp/*` app-server RPC methods
  (needs `just write-app-server-schema`).
- Windows UNC-path handling and `windows-sandbox` interplay.
- Negotiated position-encoding (UTF-8) support.
- `reinitialize` on plugin refresh (TS `reinitializeLspServerManager`).
