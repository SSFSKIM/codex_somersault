use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use super::*;
use crate::config::ResolvedLspServerConfig;
use crate::test_support::connected_instance;
use codex_config::LspServerConfig;
use pretty_assertions::assert_eq;
use serde_json::json;

fn raw_config(command: &str, exts: &[(&str, &str)]) -> LspServerConfig {
    LspServerConfig {
        command: command.to_string(),
        args: Vec::new(),
        env: HashMap::new(),
        extension_to_language: exts
            .iter()
            .map(|(e, l)| (e.to_string(), l.to_string()))
            .collect(),
        workspace_folder: None,
        initialization_options: None,
        startup_timeout_ms: None,
        max_restarts: 3,
    }
}

fn resolved(name: &str, exts: &[(&str, &str)]) -> ResolvedLspServerConfig {
    ResolvedLspServerConfig::resolve(name, &raw_config("unused", exts), Path::new("/work")).unwrap()
}

async fn next_method(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<serde_json::Value>,
) -> serde_json::Value {
    tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("frame should arrive")
        .expect("channel open")
}

#[tokio::test]
async fn routes_by_extension_with_first_registered_winning() {
    let mut configs = HashMap::new();
    // Both claim `.ts`; sorted-name order makes `alpha` win. `beta` uniquely owns `.js`.
    configs.insert(
        "alpha".to_string(),
        raw_config("a-ls", &[(".ts", "typescript")]),
    );
    configs.insert(
        "beta".to_string(),
        raw_config("b-ls", &[(".ts", "typescript"), (".js", "javascript")]),
    );
    let manager = LspManager::new(configs, std::path::PathBuf::from("/work"));

    assert_eq!(
        manager.server_for(Path::new("/x.ts")).unwrap().name(),
        "alpha"
    );
    assert_eq!(
        manager.server_for(Path::new("/x.js")).unwrap().name(),
        "beta"
    );
    assert!(manager.server_for(Path::new("/x.py")).is_none());
}

#[tokio::test]
async fn open_then_update_emits_didchange_then_didsave() {
    let (instance, mut rx) = connected_instance(resolved("rust", &[(".rs", "rust")])).await;
    let manager = LspManager::new_for_test(
        HashMap::from([("rust".to_string(), instance)]),
        HashMap::from([(".rs".to_string(), "rust".to_string())]),
        HashMap::from([(".rs".to_string(), "rust".to_string())]),
        std::path::PathBuf::from("/work"),
    );

    let path = Path::new("/work/a.rs");
    manager.open_file(path, "fn main() {}").await;
    assert_eq!(next_method(&mut rx).await["method"], "textDocument/didOpen");
    assert!(manager.is_file_open(path).await);

    manager
        .notify_file_changes(&[FileChange {
            path: path.to_path_buf(),
            kind: FileChangeKind::Updated("fn main() { /* edited */ }".to_string()),
        }])
        .await;

    let change = next_method(&mut rx).await;
    assert_eq!(change["method"], "textDocument/didChange");
    assert_eq!(
        change["params"]["contentChanges"][0]["text"],
        "fn main() { /* edited */ }"
    );
    assert_eq!(next_method(&mut rx).await["method"], "textDocument/didSave");
}

#[tokio::test]
async fn change_on_unopened_file_falls_through_to_open() {
    let (instance, mut rx) = connected_instance(resolved("rust", &[(".rs", "rust")])).await;
    let manager = LspManager::new_for_test(
        HashMap::from([("rust".to_string(), instance)]),
        HashMap::from([(".rs".to_string(), "rust".to_string())]),
        HashMap::from([(".rs".to_string(), "rust".to_string())]),
        std::path::PathBuf::from("/work"),
    );

    manager
        .change_file(Path::new("/work/b.rs"), "content")
        .await;
    assert_eq!(next_method(&mut rx).await["method"], "textDocument/didOpen");
}

#[test]
fn parses_publish_diagnostics_and_drops_empty() {
    assert!(
        parse_publish_diagnostics(&json!({"uri": "file:///a.rs", "diagnostics": []})).is_none()
    );

    let file = parse_publish_diagnostics(&json!({
        "uri": "file:///a.rs",
        "diagnostics": [{
            "severity": 1,
            "message": "boom",
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}},
            "source": "rustc",
            "code": "E0001"
        }]
    }))
    .expect("non-empty diagnostics parse");
    assert_eq!(file.uri, "file:///a.rs");
    assert_eq!(file.diagnostics[0].severity, DiagnosticSeverity::Error);
    assert_eq!(file.diagnostics[0].code.as_deref(), Some("E0001"));
}

#[test]
fn numeric_diagnostic_code_is_stringified() {
    let file = parse_publish_diagnostics(&json!({
        "uri": "file:///a.rs",
        "diagnostics": [{
            "severity": 2,
            "message": "warn",
            "range": {"start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 2}},
            "code": 42
        }]
    }))
    .unwrap();
    assert_eq!(file.diagnostics[0].code.as_deref(), Some("42"));
}
