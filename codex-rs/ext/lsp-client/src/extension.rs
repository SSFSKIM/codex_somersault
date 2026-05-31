//! Extension wiring: builds a thread-scoped [`LspManager`] from `[lsp_servers]` at thread start
//! (for interactive sessions only), contributes the `lsp/query` tool, injects pushed diagnostics
//! as a developer-policy prompt fragment each turn, and tears the manager down at thread stop.
//!
//! The manager is also exposed as an [`ApplyPatchDocSyncHandle`] so the apply-patch runtime in
//! `codex-core` can forward committed edits without depending on this crate.

use std::sync::Arc;

use codex_core::config::Config;
use codex_extension_api::ApplyPatchDocSyncHandle;
use codex_extension_api::ContextContributor;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::PromptFragment;
use codex_extension_api::ThreadLifecycleContributor;
use codex_extension_api::ThreadStartInput;
use codex_extension_api::ThreadStopInput;
use codex_extension_api::ToolCall;
use codex_extension_api::ToolContributor;
use codex_extension_api::ToolExecutor;
use codex_protocol::protocol::SessionSource;

use crate::format;
use crate::manager::LspManager;
use crate::manager::ManagerDocSync;
use crate::tool::LspTool;

/// Thread-scoped LSP state stored in `ExtensionData` and shared by the tool, the diagnostics
/// injector, and (via [`ApplyPatchDocSyncHandle`]) the apply-patch runtime.
struct LspState {
    manager: Arc<LspManager>,
}

#[derive(Clone)]
struct LspExtension;

impl LspExtension {
    /// Builds the manager from `config.lsp_servers`, or `None` when no servers are configured.
    fn build_state(config: &Config) -> Option<LspState> {
        if config.lsp_servers.is_empty() {
            return None;
        }
        let cwd = config.cwd.as_path().to_path_buf();
        let manager = LspManager::new(config.lsp_servers.clone(), cwd);
        Some(LspState { manager })
    }
}

#[async_trait::async_trait]
impl ThreadLifecycleContributor<Config> for LspExtension {
    async fn on_thread_start(&self, input: ThreadStartInput<'_, Config>) {
        // Never spawn language servers in headless/exec sessions.
        if matches!(input.session_source, SessionSource::Exec) {
            return;
        }
        let Some(state) = LspExtension::build_state(input.config) else {
            return;
        };
        // Expose the manager to the apply-patch doc-sync seam, then store the thread state.
        let doc_sync = ApplyPatchDocSyncHandle(Arc::new(ManagerDocSync(state.manager.clone())));
        input.thread_store.insert(doc_sync);
        input.thread_store.insert(state);
    }

    async fn on_thread_stop(&self, input: ThreadStopInput<'_>) {
        if let Some(state) = input.thread_store.get::<LspState>() {
            state.manager.shutdown().await;
        }
    }
}

impl ToolContributor for LspExtension {
    fn tools(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
        let Some(state) = thread_store.get::<LspState>() else {
            return Vec::new();
        };
        vec![Arc::new(LspTool::new(state.manager.clone()))]
    }
}

impl ContextContributor for LspExtension {
    fn contribute<'a>(
        &'a self,
        _session_store: &'a ExtensionData,
        thread_store: &'a ExtensionData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<PromptFragment>> + Send + 'a>> {
        Box::pin(async move {
            let Some(state) = thread_store.get::<LspState>() else {
                return Vec::new();
            };
            let files = state.manager.diagnostics().check_for_diagnostics();
            if files.is_empty() {
                return Vec::new();
            }
            vec![PromptFragment::developer_policy(
                format::format_diagnostics(&files),
            )]
        })
    }
}

/// Installs the LSP extension. Reads `[lsp_servers]` from `Config` at thread start; contributes no
/// tools and injects no diagnostics when unconfigured or in non-interactive sessions.
pub fn install(registry: &mut ExtensionRegistryBuilder<Config>) {
    let extension = Arc::new(LspExtension);
    registry.thread_lifecycle_contributor(extension.clone());
    registry.tool_contributor(extension.clone());
    registry.prompt_contributor(extension);
}

#[cfg(test)]
#[path = "extension_tests.rs"]
mod extension_tests;
