use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use super::*;
use crate::config::ResolvedLspServerConfig;
use crate::instance::LspServerInstance;
use crate::manager::LspManager;
use crate::test_support::answer_handshake;
use crate::test_support::read_frame;
use crate::test_support::wire;
use crate::test_support::write_frame;
use codex_config::LspServerConfig;
use pretty_assertions::assert_eq;

fn rust_config(cwd: &Path) -> ResolvedLspServerConfig {
    let raw = LspServerConfig {
        command: "unused".to_string(),
        args: Vec::new(),
        env: HashMap::new(),
        extension_to_language: HashMap::from([(".rs".to_string(), "rust".to_string())]),
        workspace_folder: None,
        initialization_options: None,
        startup_timeout_ms: None,
        max_restarts: 3,
    };
    ResolvedLspServerConfig::resolve("rust", &raw, cwd).unwrap()
}

/// Builds an [`LspTool`] backed by a mock server that answers `hover`/`definition` (recording the
/// hover position) and `null` for anything else.
async fn tool_with_mock(
    cwd: &Path,
    def_uri: String,
    captured_position: Arc<StdMutex<Option<serde_json::Value>>>,
) -> LspTool {
    let wired = wire();
    let transport = wired.transport;
    let mut reader = wired.server_reader;
    let mut writer = wired.server_writer;

    tokio::spawn(async move {
        answer_handshake(&mut reader, &mut writer).await;
        while let Some(frame) = read_frame(&mut reader).await {
            let id = frame.get("id").cloned();
            let method = frame.get("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
            match method.as_str() {
                "textDocument/hover" => {
                    *captured_position.lock().unwrap() = Some(frame["params"]["position"].clone());
                    if let Some(id) = id {
                        write_frame(
                            &mut writer,
                            &serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {"contents": {"kind": "markdown", "value": "hovered: T"}}}),
                        )
                        .await;
                    }
                }
                "textDocument/definition" => {
                    if let Some(id) = id {
                        write_frame(
                            &mut writer,
                            &serde_json::json!({"jsonrpc": "2.0", "id": id, "result": [{"uri": def_uri, "range": {"start": {"line": 4, "character": 2}, "end": {"line": 4, "character": 6}}}]}),
                        )
                        .await;
                    }
                }
                _ => {
                    if let Some(id) = id {
                        write_frame(&mut writer, &serde_json::json!({"jsonrpc": "2.0", "id": id, "result": null})).await;
                    }
                }
            }
        }
    });

    let instance = Arc::new(LspServerInstance::new(rust_config(cwd)));
    instance.connect(transport).await.expect("mock handshake");
    let manager = LspManager::new_for_test(
        HashMap::from([("rust".to_string(), instance)]),
        HashMap::from([(".rs".to_string(), "rust".to_string())]),
        HashMap::from([(".rs".to_string(), "rust".to_string())]),
        cwd.to_path_buf(),
    );
    LspTool::new(manager)
}

#[tokio::test]
async fn hover_converts_one_based_to_zero_based_and_formats() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.rs");
    tokio::fs::write(&file, "fn main() {}\n").await.unwrap();

    let captured = Arc::new(StdMutex::new(None));
    let tool = tool_with_mock(dir.path(), String::new(), captured.clone()).await;

    let out = tool
        .run(LspToolInput {
            operation: LspOperation::Hover,
            file_path: file.to_string_lossy().into_owned(),
            line: 2,
            character: 3,
        })
        .await
        .expect("hover succeeds");

    assert!(out.contains("hovered: T"), "{out}");
    // Input (2, 3) 1-based → wire (1, 2) 0-based.
    assert_eq!(
        captured.lock().unwrap().clone().unwrap(),
        serde_json::json!({"line": 1, "character": 2})
    );
}

#[tokio::test]
async fn definition_formats_a_location() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("a.rs");
    tokio::fs::write(&file, "fn main() {}\n").await.unwrap();
    let def_uri = format!("file://{}", file.display());

    let tool = tool_with_mock(dir.path(), def_uri, Arc::new(StdMutex::new(None))).await;
    let out = tool
        .run(LspToolInput {
            operation: LspOperation::GoToDefinition,
            file_path: file.to_string_lossy().into_owned(),
            line: 1,
            character: 1,
        })
        .await
        .expect("definition succeeds");

    // 0-based (4, 2) renders 1-based (5, 3).
    assert!(out.contains(":5:3"), "{out}");
}

#[tokio::test]
async fn rejects_files_over_the_size_limit() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("big.rs");
    let handle = std::fs::File::create(&file).unwrap();
    handle.set_len(MAX_LSP_FILE_SIZE_BYTES + 1).unwrap();
    drop(handle);

    let tool = tool_with_mock(dir.path(), String::new(), Arc::new(StdMutex::new(None))).await;
    let err = tool
        .run(LspToolInput {
            operation: LspOperation::Hover,
            file_path: file.to_string_lossy().into_owned(),
            line: 1,
            character: 1,
        })
        .await
        .expect_err("oversized file is rejected");

    match err {
        FunctionCallError::RespondToModel(msg) => assert!(msg.contains("too large"), "{msg}"),
        other => panic!("unexpected error: {other:?}"),
    }
}
