use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::*;
use crate::config::ResolvedLspServerConfig;
use crate::test_support::answer_handshake;
use crate::test_support::read_frame;
use crate::test_support::wire;
use crate::test_support::write_frame;
use pretty_assertions::assert_eq;
use serde_json::json;

fn test_config() -> ResolvedLspServerConfig {
    ResolvedLspServerConfig {
        name: "mock".to_string(),
        command: "unused".to_string(),
        args: Vec::new(),
        env: HashMap::new(),
        extension_to_language: HashMap::from([(".rs".to_string(), "rust".to_string())]),
        workspace_folder: PathBuf::from("/work"),
        initialization_options: None,
        startup_timeout_ms: None,
        max_restarts: 3,
    }
}

#[tokio::test]
async fn handshake_reaches_running_and_answers_workspace_configuration() {
    let wired = wire();
    let transport = wired.transport;
    let mut server_reader = wired.server_reader;
    let mut server_writer = wired.server_writer;

    let server = tokio::spawn(async move {
        answer_handshake(&mut server_reader, &mut server_writer).await;
        // Server demands configuration unconditionally; expect a null[] of matching length.
        write_frame(
            &mut server_writer,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "workspace/configuration",
                "params": {"items": [{"section": "rust"}]}
            }),
        )
        .await;
        read_frame(&mut server_reader).await.expect("config reply")
    });

    let instance = LspServerInstance::new(test_config());
    instance
        .connect(transport)
        .await
        .expect("handshake should succeed");
    assert_eq!(instance.state().await, LspServerState::Running);
    assert!(instance.is_healthy().await);

    let reply = server.await.unwrap();
    assert_eq!(reply["result"], json!([serde_json::Value::Null]));
}

#[tokio::test(start_paused = true)]
async fn retries_content_modified_three_times_with_backoff() {
    let wired = wire();
    let transport = wired.transport;
    let mut server_reader = wired.server_reader;
    let mut server_writer = wired.server_writer;

    let server = tokio::spawn(async move {
        answer_handshake(&mut server_reader, &mut server_writer).await;
        let mut count = 0u32;
        while let Some(req) = read_frame(&mut server_reader).await {
            if req["method"] == "textDocument/hover" {
                count += 1;
                if count <= 3 {
                    write_frame(
                        &mut server_writer,
                        &json!({"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32801, "message": "content modified"}}),
                    )
                    .await;
                } else {
                    write_frame(
                        &mut server_writer,
                        &json!({"jsonrpc": "2.0", "id": req["id"], "result": {"ok": true}}),
                    )
                    .await;
                    return count;
                }
            }
        }
        count
    });

    let instance = Arc::new(LspServerInstance::new(test_config()));
    instance.connect(transport).await.unwrap();

    let start = tokio::time::Instant::now();
    let result = instance
        .request_value("textDocument/hover", json!({}))
        .await
        .expect("succeeds after retries");
    let elapsed = start.elapsed();

    assert_eq!(result, json!({"ok": true}));
    // Backoff 500 + 1000 + 2000 = 3500ms across exactly 3 retries.
    assert_eq!(elapsed, std::time::Duration::from_millis(3500));
    assert_eq!(server.await.unwrap(), 4, "1 initial + 3 retries");
}

#[tokio::test]
async fn request_without_connection_reports_not_running() {
    let instance = LspServerInstance::new(test_config());
    let err = instance
        .request_value("textDocument/hover", json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, LspError::NotRunning(_)));
}
