//! Tests for the JSON-RPC framer, driven against an in-process `tokio::io::duplex` fake server.

use super::*;
use pretty_assertions::assert_eq;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::DuplexStream;

/// Reads one `Content-Length`-framed JSON message from a duplex stream.
async fn read_frame(stream: &mut DuplexStream) -> Value {
    let mut header = Vec::new();
    // Read byte-by-byte until the `\r\n\r\n` header terminator.
    loop {
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await.expect("header byte");
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = String::from_utf8(header).expect("utf8 header");
    let len: usize = header
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|r| r.trim().to_string())
        })
        .expect("content-length header")
        .parse()
        .expect("length");
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await.expect("body");
    serde_json::from_slice(&body).expect("json body")
}

/// Writes one `Content-Length`-framed JSON message to a duplex stream.
async fn write_frame(stream: &mut DuplexStream, value: &Value) {
    stream
        .write_all(&encode_frame(value))
        .await
        .expect("write frame");
    stream.flush().await.expect("flush");
}

/// Wires a transport's client streams to a mock-server's streams via two duplex pipes.
/// Returns `(transport, server_reader, server_writer)`.
fn wired() -> (LspTransport, DuplexStream, DuplexStream) {
    let (client_writer, server_reader) = tokio::io::duplex(8192);
    let (server_writer, client_reader) = tokio::io::duplex(8192);
    let transport = LspTransport::connect(client_reader, client_writer, None);
    (transport, server_reader, server_writer)
}

#[tokio::test]
async fn round_trips_a_request_and_response() {
    let (transport, mut server_reader, mut server_writer) = wired();

    // Mock server: read the request, echo its id back in a result.
    let server = tokio::spawn(async move {
        let req = read_frame(&mut server_reader).await;
        assert_eq!(req["method"], "initialize");
        let id = req["id"].clone();
        write_frame(
            &mut server_writer,
            &json!({"jsonrpc": "2.0", "id": id, "result": {"ok": true}}),
        )
        .await;
    });

    let result: Value = transport
        .request("initialize", json!({"processId": 1}))
        .await
        .expect("request should succeed");
    assert_eq!(result, json!({"ok": true}));
    server.await.unwrap();
}

#[tokio::test]
async fn surfaces_rpc_errors_with_code() {
    let (transport, mut server_reader, mut server_writer) = wired();
    tokio::spawn(async move {
        let req = read_frame(&mut server_reader).await;
        let id = req["id"].clone();
        write_frame(
            &mut server_writer,
            &json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32801, "message": "content modified"}}),
        )
        .await;
    });

    let err = transport
        .request_value("textDocument/hover", json!({}))
        .await
        .expect_err("should surface server error");
    assert_eq!(err.code(), Some(-32801));
}

#[tokio::test]
async fn dispatches_a_server_notification_to_a_handler() {
    let (transport, mut _server_reader, mut server_writer) = wired();
    let (tx, rx) = std::sync::mpsc::channel();
    transport.on_notification(
        "textDocument/publishDiagnostics",
        Arc::new(move |params| {
            tx.send(params).expect("forward params");
        }),
    );

    write_frame(
        &mut server_writer,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": "file:///a.rs", "diagnostics": []}
        }),
    )
    .await;

    let params =
        tokio::task::spawn_blocking(move || rx.recv_timeout(std::time::Duration::from_secs(2)))
            .await
            .unwrap()
            .expect("handler should receive the notification");
    assert_eq!(params["uri"], "file:///a.rs");
}

#[tokio::test]
async fn answers_server_to_client_requests_with_a_responder() {
    let (transport, mut server_reader, mut server_writer) = wired();
    transport.on_request(
        "workspace/configuration",
        Arc::new(|params: Value| {
            // Mirror the TS behavior: one null per requested item.
            let items = params
                .get("items")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            Value::Array(vec![Value::Null; items])
        }),
    );

    // Server asks the client for configuration; expect a null[] of matching length back.
    write_frame(
        &mut server_writer,
        &json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "workspace/configuration",
            "params": {"items": [{"section": "rust"}, {"section": "ts"}]}
        }),
    )
    .await;

    let response = read_frame(&mut server_reader).await;
    assert_eq!(response["id"], 99);
    assert_eq!(response["result"], json!([Value::Null, Value::Null]));
}
