//! Per-server lifecycle: spawn → initialize handshake → request/notify with transient-error retry.
//!
//! State machine: `Stopped → Starting → Running → Stopping → Stopped`, with any failure moving to
//! `Error`. The `initialize` request declares UTF-16 positions, `didSave` sync, and explicit
//! diagnostic/hover/definition/references/documentSymbol/callHierarchy capabilities — but **not**
//! `workspace/configuration` (we cannot serve config). A `workspace/configuration` responder still
//! answers `null[]` because some servers (typescript-language-server) request it unconditionally.
//!
//! Mirrors `Claude Code Src/src/services/lsp/LSPServerInstance.ts`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::Value;
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::ResolvedLspServerConfig;
use crate::transport::JsonRpcError;
use crate::transport::LspTransport;

/// JSON-RPC error code rust-analyzer emits while indexing; the request should be retried.
const LSP_ERROR_CONTENT_MODIFIED: i64 = -32801;
/// Number of retries for `ContentModified` (3 retries → 4 total attempts, sleeps 500/1000/2000ms).
const MAX_RETRIES_FOR_TRANSIENT_ERRORS: u32 = 3;
/// Base backoff; the nth retry waits `RETRY_BASE_DELAY_MS * 2^n`.
const RETRY_BASE_DELAY_MS: u64 = 500;

type NotificationHandler = Arc<dyn Fn(Value) + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LspServerState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LspError {
    #[error("lsp server `{0}` is not running")]
    NotRunning(String),
    #[error("failed to spawn lsp server `{name}`: {source}")]
    Spawn {
        name: String,
        source: std::io::Error,
    },
    #[error("lsp request `{method}` failed: {message}")]
    Rpc {
        method: String,
        code: Option<i64>,
        message: String,
    },
    #[error("lsp server `{0}` exceeded its restart limit")]
    MaxRestarts(String),
}

pub(crate) struct LspServerInstance {
    config: ResolvedLspServerConfig,
    state: Mutex<LspServerState>,
    transport: Mutex<Option<Arc<LspTransport>>>,
    capabilities: Mutex<Option<Value>>,
    /// Unexpected exits observed; reset to 0 on a successful start.
    crash_recovery_count: AtomicU32,
    /// Handlers stored before start and re-applied on each `connect`. A `std::sync::Mutex` keeps
    /// registration synchronous (handlers are registered before the server starts).
    notification_handlers: std::sync::Mutex<HashMap<String, NotificationHandler>>,
}

impl LspServerInstance {
    pub(crate) fn new(config: ResolvedLspServerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(LspServerState::Stopped),
            transport: Mutex::new(None),
            capabilities: Mutex::new(None),
            crash_recovery_count: AtomicU32::new(0),
            notification_handlers: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.config.name
    }

    pub(crate) async fn state(&self) -> LspServerState {
        self.state.lock().await.clone()
    }

    pub(crate) async fn is_healthy(&self) -> bool {
        matches!(*self.state.lock().await, LspServerState::Running)
    }

    /// Stores a notification handler, applied to the transport on the next `connect`. Registration
    /// is expected before the server starts (e.g. the manager's `publishDiagnostics` handler).
    pub(crate) fn on_notification(&self, method: &str, handler: NotificationHandler) {
        self.notification_handlers
            .lock()
            .unwrap()
            .insert(method.to_string(), handler);
    }

    /// Idempotent start: spawns the server (respecting the restart cap) and runs the handshake.
    pub(crate) async fn start(&self) -> Result<(), LspError> {
        if self.is_healthy().await {
            return Ok(());
        }
        if self.crash_recovery_count.load(Ordering::Relaxed) > self.config.max_restarts {
            let msg = format!("server `{}` exceeded restart limit", self.config.name);
            *self.state.lock().await = LspServerState::Error(msg);
            return Err(LspError::MaxRestarts(self.config.name.clone()));
        }

        *self.state.lock().await = LspServerState::Starting;
        let transport = LspTransport::spawn(
            &self.config.command,
            &self.config.args,
            &self.config.env,
            &self.config.workspace_folder,
        )
        .await
        .map_err(|source| LspError::Spawn {
            name: self.config.name.clone(),
            source,
        })?;
        let transport = Arc::new(transport);

        self.connect(transport).await
    }

    /// Runs the initialize/initialized handshake on `transport` and installs handlers. Shared by
    /// [`Self::start`] and the test seam (`pub(crate)` so tests can inject a mock transport).
    pub(crate) async fn connect(&self, transport: Arc<LspTransport>) -> Result<(), LspError> {
        // `workspace/configuration` → one null per requested item (we serve no config).
        transport.on_request(
            "workspace/configuration",
            Arc::new(|params: Value| {
                let n = params
                    .get("items")
                    .and_then(Value::as_array)
                    .map(|a| a.len())
                    .unwrap_or(0);
                Value::Array(vec![Value::Null; n])
            }),
        );
        // Re-apply previously registered notification handlers (e.g. publishDiagnostics). The lock
        // is held only across synchronous `transport.on_notification` calls (no await inside).
        {
            let handlers = self.notification_handlers.lock().unwrap();
            for (method, handler) in handlers.iter() {
                transport.on_notification(method, handler.clone());
            }
        }

        let init_result = transport
            .request_value("initialize", self.initialize_params())
            .await
            .map_err(|e| self.rpc_error("initialize", e))?;
        *self.capabilities.lock().await = init_result.get("capabilities").cloned();

        transport
            .notify("initialized", json!({}))
            .await
            .map_err(|e| self.rpc_error("initialized", e))?;

        *self.transport.lock().await = Some(transport);
        *self.state.lock().await = LspServerState::Running;
        self.crash_recovery_count.store(0, Ordering::Relaxed);
        Ok(())
    }

    /// Sends a request, retrying up to 3 times on `ContentModified` with 500/1000/2000ms backoff.
    pub(crate) async fn request_value(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, LspError> {
        let transport = self
            .transport
            .lock()
            .await
            .clone()
            .ok_or_else(|| LspError::NotRunning(self.config.name.clone()))?;

        let mut attempt = 0u32;
        loop {
            match transport.request_value(method, params.clone()).await {
                Ok(value) => return Ok(value),
                Err(err)
                    if err.code() == Some(LSP_ERROR_CONTENT_MODIFIED)
                        && attempt < MAX_RETRIES_FOR_TRANSIENT_ERRORS =>
                {
                    let delay = RETRY_BASE_DELAY_MS * 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                Err(err) => return Err(self.rpc_error(method, err)),
            }
        }
    }

    /// Sends a notification (fire-and-forget; no retry).
    pub(crate) async fn notify(&self, method: &str, params: Value) -> Result<(), LspError> {
        let transport = self
            .transport
            .lock()
            .await
            .clone()
            .ok_or_else(|| LspError::NotRunning(self.config.name.clone()))?;
        transport
            .notify(method, params)
            .await
            .map_err(|e| self.rpc_error(method, e))
    }

    /// Stops the server: shutdown request + exit notification + kill.
    pub(crate) async fn stop(&self) {
        *self.state.lock().await = LspServerState::Stopping;
        if let Some(transport) = self.transport.lock().await.take() {
            transport.shutdown().await;
        }
        *self.capabilities.lock().await = None;
        *self.state.lock().await = LspServerState::Stopped;
    }

    fn rpc_error(&self, method: &str, err: JsonRpcError) -> LspError {
        LspError::Rpc {
            method: method.to_string(),
            code: err.code(),
            message: err.to_string(),
        }
    }

    /// The `initialize` params: UTF-16 positions, `didSave`, explicit feature capabilities, and
    /// crucially **no** `workspace/configuration`. Built as raw JSON to match the TS shape exactly.
    fn initialize_params(&self) -> Value {
        let workspace_uri = path_to_uri(&self.config.workspace_folder);
        json!({
            "processId": std::process::id(),
            "initializationOptions": self.config.initialization_options.clone().unwrap_or(json!({})),
            "rootUri": workspace_uri,
            "workspaceFolders": [{
                "uri": workspace_uri,
                "name": self.config.workspace_folder
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "workspace".to_string()),
            }],
            "capabilities": {
                "workspace": {
                    "configuration": false,
                    "workspaceFolders": false,
                },
                "textDocument": {
                    "synchronization": {
                        "dynamicRegistration": false,
                        "willSave": false,
                        "willSaveWaitUntil": false,
                        "didSave": true,
                    },
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "tagSupport": { "valueSet": [1, 2] },
                        "versionSupport": false,
                        "codeDescriptionSupport": true,
                        "dataSupport": false,
                    },
                    "hover": {
                        "dynamicRegistration": false,
                        "contentFormat": ["markdown", "plaintext"],
                    },
                    "definition": { "dynamicRegistration": false, "linkSupport": true },
                    "references": { "dynamicRegistration": false },
                    "implementation": { "dynamicRegistration": false, "linkSupport": true },
                    "documentSymbol": {
                        "dynamicRegistration": false,
                        "hierarchicalDocumentSymbolSupport": true,
                    },
                    "callHierarchy": { "dynamicRegistration": false },
                },
                "general": { "positionEncodings": ["utf-16"] },
            },
        })
    }
}

/// Best-effort `file://` URI for an absolute path (enough for local workspace folders).
pub(crate) fn path_to_uri(path: &std::path::Path) -> String {
    url::Url::from_file_path(path)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| format!("file://{}", path.display()))
}

#[cfg(test)]
#[path = "instance_tests.rs"]
mod instance_tests;
