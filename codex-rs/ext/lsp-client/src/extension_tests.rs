use std::collections::HashMap;
use std::path::PathBuf;

use super::*;
use codex_config::LspServerConfig;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::ToolName;
use pretty_assertions::assert_eq;

fn manager() -> Arc<LspManager> {
    let mut configs = HashMap::new();
    configs.insert(
        "rust".to_string(),
        LspServerConfig {
            command: "rust-analyzer".to_string(),
            args: Vec::new(),
            env: HashMap::new(),
            extension_to_language: HashMap::from([(".rs".to_string(), "rust".to_string())]),
            workspace_folder: None,
            initialization_options: None,
            startup_timeout_ms: None,
            max_restarts: 3,
        },
    );
    LspManager::new(configs, PathBuf::from("/work"))
}

#[tokio::test]
async fn contributes_lsp_query_tool_when_state_present() {
    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();

    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("11111111-1111-4111-8111-111111111111");
    thread_store.insert(LspState { manager: manager() });

    let names: Vec<ToolName> = registry
        .tool_contributors()
        .iter()
        .flat_map(|c| c.tools(&session_store, &thread_store))
        .map(|t| t.tool_name())
        .collect();

    assert_eq!(names, vec![ToolName::namespaced("lsp", "query")]);
}

#[tokio::test]
async fn contributes_nothing_without_state() {
    let mut builder = ExtensionRegistryBuilder::<Config>::new();
    install(&mut builder);
    let registry = builder.build();

    let session_store = ExtensionData::new("session");
    let thread_store = ExtensionData::new("11111111-1111-4111-8111-111111111111");

    let count = registry
        .tool_contributors()
        .iter()
        .flat_map(|c| c.tools(&session_store, &thread_store))
        .count();
    assert_eq!(count, 0);
}
