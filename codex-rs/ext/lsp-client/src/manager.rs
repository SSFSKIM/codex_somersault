//! Routing manager: owns all server instances, routes files to servers by extension (first-wins),
//! tracks open documents, drives document sync, and feeds pushed diagnostics into the registry.
//!
//! One `LspManager` is created per thread and held as `Arc<LspManager>` in thread-scoped
//! `ExtensionData`. It also implements [`ApplyPatchDocSync`] so the apply-patch runtime can forward
//! committed edits without `codex-core` depending on this crate.
//!
//! Mirrors `Claude Code Src/src/services/lsp/LSPServerManager.ts`.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use codex_config::LspServerConfig;
use codex_extension_api::ApplyPatchDocSync;
use codex_extension_api::FileChange;
use codex_extension_api::FileChangeKind;
use serde_json::Value;
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::ResolvedLspServerConfig;
use crate::diagnostics::DiagnosticEntry;
use crate::diagnostics::DiagnosticFile;
use crate::diagnostics::DiagnosticRegistry;
use crate::diagnostics::DiagnosticSeverity;
use crate::instance::LspError;
use crate::instance::LspServerInstance;
use crate::instance::path_to_uri;

/// Document version sent with every `didOpen`/`didChange` (v1 uses full-content sync, version 1).
const DOC_VERSION: i64 = 1;

pub struct LspManager {
    servers: HashMap<String, Arc<LspServerInstance>>,
    /// Lowercased extension (with dot) → server name. First registration wins on conflict.
    extension_map: HashMap<String, String>,
    /// Lowercased extension (with dot) → LSP language id, aligned with `extension_map`.
    languages: HashMap<String, String>,
    /// `file://` URI → server name, deduplicating `didOpen`.
    opened_files: Mutex<HashMap<String, String>>,
    diagnostics: Arc<DiagnosticRegistry>,
    cwd: PathBuf,
}

impl LspManager {
    /// Validates configs (logging and skipping invalid ones), builds the first-wins extension map,
    /// creates instances, and wires each server's `publishDiagnostics` handler into the registry.
    /// Servers are **not** started here (lazy start on first use).
    pub fn new(configs: HashMap<String, LspServerConfig>, cwd: PathBuf) -> Arc<Self> {
        let diagnostics = Arc::new(DiagnosticRegistry::new());
        let mut servers = HashMap::new();
        let mut extension_map: HashMap<String, String> = HashMap::new();
        let mut languages: HashMap<String, String> = HashMap::new();

        // Deterministic order so "first registered wins" is stable across runs.
        let mut names: Vec<&String> = configs.keys().collect();
        names.sort();

        for name in names {
            let raw = &configs[name];
            let resolved = match ResolvedLspServerConfig::resolve(name, raw, &cwd) {
                Ok(r) => r,
                Err(reason) => {
                    tracing::warn!("skipping lsp server: {reason}");
                    continue;
                }
            };

            for (ext, lang) in &resolved.extension_to_language {
                if let Some(existing) = extension_map.get(ext) {
                    tracing::warn!(
                        "lsp extension `{ext}` already handled by `{existing}`; ignoring `{name}`"
                    );
                    continue;
                }
                extension_map.insert(ext.clone(), name.clone());
                languages.insert(ext.clone(), lang.clone());
            }

            let instance = Arc::new(LspServerInstance::new(resolved));
            servers.insert(name.clone(), instance);
        }

        let manager = Self {
            servers,
            extension_map,
            languages,
            opened_files: Mutex::new(HashMap::new()),
            diagnostics,
            cwd,
        };
        manager.register_diagnostic_handlers();
        Arc::new(manager)
    }

    /// Registers a `publishDiagnostics` handler on every server, feeding the shared registry.
    /// Handlers are stored synchronously and applied when each server connects.
    fn register_diagnostic_handlers(&self) {
        for instance in self.servers.values() {
            let registry = self.diagnostics.clone();
            let handler = Arc::new(move |params: Value| {
                if let Some(file) = parse_publish_diagnostics(&params) {
                    registry.register_pending(vec![file]);
                }
            });
            instance.on_notification("textDocument/publishDiagnostics", handler);
        }
    }

    pub fn diagnostics(&self) -> Arc<DiagnosticRegistry> {
        self.diagnostics.clone()
    }

    /// Routes a path to its server (by extension) without starting it.
    fn server_for(&self, file_path: &Path) -> Option<Arc<LspServerInstance>> {
        let ext = file_extension(file_path)?;
        let name = self.extension_map.get(&ext)?;
        self.servers.get(name).cloned()
    }

    /// Lazy start: routes the file to a server, starts it if needed, and returns it.
    async fn ensure_started(&self, file_path: &Path) -> Option<Arc<LspServerInstance>> {
        let instance = self.server_for(file_path)?;
        if !instance.is_healthy().await
            && let Err(err) = instance.start().await
        {
            tracing::warn!("failed to start lsp server `{}`: {err}", instance.name());
            return None;
        }
        Some(instance)
    }

    pub async fn is_file_open(&self, file_path: &Path) -> bool {
        let uri = path_to_uri(file_path);
        self.opened_files.lock().await.contains_key(&uri)
    }

    /// Sends `didOpen` for a file (deduplicated per server).
    pub async fn open_file(&self, file_path: &Path, content: &str) {
        let Some(instance) = self.ensure_started(file_path).await else {
            return;
        };
        let uri = path_to_uri(file_path);
        {
            let opened = self.opened_files.lock().await;
            if opened.get(&uri) == Some(&instance.name().to_string()) {
                return;
            }
        }
        let Some(language_id) = self.language_for(file_path) else {
            return;
        };
        let params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": DOC_VERSION,
                "text": content,
            }
        });
        if let Err(err) = instance.notify("textDocument/didOpen", params).await {
            tracing::debug!("didOpen failed: {err}");
            return;
        }
        self.opened_files
            .lock()
            .await
            .insert(uri, instance.name().to_string());
    }

    /// Sends a full-content `didChange`, falling through to `open_file` if the file isn't open yet.
    pub async fn change_file(&self, file_path: &Path, content: &str) {
        if !self.is_file_open(file_path).await {
            self.open_file(file_path, content).await;
            return;
        }
        let Some(instance) = self.server_for(file_path) else {
            return;
        };
        let uri = path_to_uri(file_path);
        let params = json!({
            "textDocument": { "uri": uri, "version": DOC_VERSION },
            "contentChanges": [{ "text": content }],
        });
        if let Err(err) = instance.notify("textDocument/didChange", params).await {
            tracing::debug!("didChange failed: {err}");
        }
    }

    /// Sends `didSave` for an open file.
    pub async fn save_file(&self, file_path: &Path) {
        if !self.is_file_open(file_path).await {
            return;
        }
        let Some(instance) = self.server_for(file_path) else {
            return;
        };
        let uri = path_to_uri(file_path);
        let params = json!({ "textDocument": { "uri": uri } });
        if let Err(err) = instance.notify("textDocument/didSave", params).await {
            tracing::debug!("didSave failed: {err}");
        }
    }

    /// Sends `didClose` and forgets the open-file mapping.
    pub async fn close_file(&self, file_path: &Path) {
        let uri = path_to_uri(file_path);
        let server_name = self.opened_files.lock().await.remove(&uri);
        if let Some(name) = server_name
            && let Some(instance) = self.servers.get(&name)
        {
            let params = json!({ "textDocument": { "uri": uri } });
            if let Err(err) = instance.notify("textDocument/didClose", params).await {
                tracing::debug!("didClose failed: {err}");
            }
        }
    }

    /// Routes a request by extension, ensures the server is started, and issues it.
    pub(crate) async fn request_value(
        &self,
        file_path: &Path,
        method: &str,
        params: Value,
    ) -> Option<Result<Value, LspError>> {
        let instance = self.ensure_started(file_path).await?;
        Some(instance.request_value(method, params).await)
    }

    /// Drives apply-patch document sync: Add→open, Update→change+save, Delete→close. Clears
    /// delivered diagnostics for each touched URI first so post-edit diagnostics re-surface.
    pub async fn notify_file_changes(&self, changes: &[FileChange]) {
        for change in changes {
            let uri = path_to_uri(&change.path);
            self.diagnostics.clear_delivered_for_file(&uri);
            match &change.kind {
                FileChangeKind::Added(content) => self.open_file(&change.path, content).await,
                FileChangeKind::Updated(content) => {
                    self.change_file(&change.path, content).await;
                    self.save_file(&change.path).await;
                }
                FileChangeKind::Deleted => self.close_file(&change.path).await,
            }
        }
    }

    /// Stops every running server and clears the open-file map.
    pub async fn shutdown(&self) {
        for instance in self.servers.values() {
            instance.stop().await;
        }
        self.opened_files.lock().await.clear();
    }

    fn language_for(&self, file_path: &Path) -> Option<String> {
        let ext = file_extension(file_path)?;
        self.languages.get(&ext).cloned()
    }

    /// The session working directory, used to resolve relative tool paths and as the git root for
    /// ignore filtering.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// Resolves a possibly-relative tool path against the session cwd.
    pub fn resolve_path(&self, raw: &str) -> PathBuf {
        let path = PathBuf::from(raw);
        if path.is_absolute() {
            path
        } else {
            self.cwd.join(path)
        }
    }

    /// Test seam: builds a manager from already-constructed (and typically mock-connected)
    /// instances and explicit routing maps, bypassing config resolution and process spawning.
    #[cfg(test)]
    pub(crate) fn new_for_test(
        servers: HashMap<String, Arc<LspServerInstance>>,
        extension_map: HashMap<String, String>,
        languages: HashMap<String, String>,
        cwd: PathBuf,
    ) -> Arc<Self> {
        let manager = Self {
            servers,
            extension_map,
            languages,
            opened_files: Mutex::new(HashMap::new()),
            diagnostics: Arc::new(DiagnosticRegistry::new()),
            cwd,
        };
        manager.register_diagnostic_handlers();
        Arc::new(manager)
    }
}

/// Owns an `Arc<LspManager>` so the synchronous [`ApplyPatchDocSync`] seam can spawn
/// fire-and-forget document-sync tasks. The extension stores this (not the bare manager) as the
/// `ApplyPatchDocSyncHandle`, because the trait method only receives `&self` and cannot recover an
/// owned `Arc<LspManager>` to move into a task.
#[derive(Clone)]
pub struct ManagerDocSync(pub Arc<LspManager>);

impl ApplyPatchDocSync for ManagerDocSync {
    fn notify_patch_changes(&self, changes: &[FileChange]) {
        let manager = self.0.clone();
        let changes = changes.to_vec();
        tokio::spawn(async move {
            manager.notify_file_changes(&changes).await;
        });
    }
}

/// Extracts a lowercased extension with a leading dot (`/a/b.RS` → `.rs`).
fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_ascii_lowercase()))
}

/// Parses a `publishDiagnostics` payload into a [`DiagnosticFile`], dropping empty arrays.
fn parse_publish_diagnostics(params: &Value) -> Option<DiagnosticFile> {
    let uri = params.get("uri").and_then(Value::as_str)?.to_string();
    let raw = params.get("diagnostics").and_then(Value::as_array)?;
    if raw.is_empty() {
        return None; // server cleared diagnostics
    }
    let diagnostics: Vec<DiagnosticEntry> = raw
        .iter()
        .filter_map(|d| {
            let range = serde_json::from_value(d.get("range")?.clone()).ok()?;
            Some(DiagnosticEntry {
                severity: DiagnosticSeverity::from_lsp(d.get("severity").and_then(Value::as_i64)),
                message: d
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                range,
                source: d.get("source").and_then(Value::as_str).map(str::to_string),
                code: d.get("code").map(stringify_code),
            })
        })
        .collect();
    if diagnostics.is_empty() {
        return None;
    }
    Some(DiagnosticFile { uri, diagnostics })
}

/// LSP `code` may be a string or a number; normalize to a string for display/dedup.
fn stringify_code(code: &Value) -> String {
    match code {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod manager_tests;
