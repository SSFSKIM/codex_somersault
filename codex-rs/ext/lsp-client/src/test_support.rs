//! Shared test helpers: `Content-Length` framing over `tokio::io::duplex` and a scriptable mock
//! language server, reused by the instance/manager/tool unit tests.

#![cfg(test)]

use std::sync::Arc;

use serde_json::Value;
use serde_json::json;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::DuplexStream;

use crate::transport::LspTransport;

/// Encodes one JSON value as a `Content-Length`-framed LSP message.
pub(crate) fn encode_frame(value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).expect("serialize frame");
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(&body);
    frame
}

/// Reads one framed JSON message from a duplex stream (returns `None` at EOF).
pub(crate) async fn read_frame(stream: &mut DuplexStream) -> Option<Value> {
    let mut header = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        if stream.read_exact(&mut byte).await.is_err() {
            return None;
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header = String::from_utf8(header).ok()?;
    let len: usize = header
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|r| r.trim().to_string())
        })?
        .parse()
        .ok()?;
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await.ok()?;
    serde_json::from_slice(&body).ok()
}

/// Writes one framed JSON message to a duplex stream.
pub(crate) async fn write_frame(stream: &mut DuplexStream, value: &Value) {
    stream
        .write_all(&encode_frame(value))
        .await
        .expect("write frame");
    stream.flush().await.expect("flush frame");
}

/// A client transport wired to a mock server's two stream halves.
pub(crate) struct Wired {
    pub(crate) transport: Arc<LspTransport>,
    pub(crate) server_reader: DuplexStream,
    pub(crate) server_writer: DuplexStream,
}

/// Builds a client [`LspTransport`] connected to a mock server over two duplex pipes.
pub(crate) fn wire() -> Wired {
    let (client_writer, server_reader) = tokio::io::duplex(64 * 1024);
    let (server_writer, client_reader) = tokio::io::duplex(64 * 1024);
    let transport = Arc::new(LspTransport::connect(client_reader, client_writer, None));
    Wired {
        transport,
        server_reader,
        server_writer,
    }
}

/// Replies to an `initialize` request with empty capabilities and consumes the `initialized`
/// notification, leaving the server ready to handle feature requests.
pub(crate) async fn answer_handshake(reader: &mut DuplexStream, writer: &mut DuplexStream) {
    let init = read_frame(reader).await.expect("initialize request");
    assert_eq!(init["method"], "initialize");
    write_frame(
        writer,
        &json!({"jsonrpc": "2.0", "id": init["id"], "result": {"capabilities": {}}}),
    )
    .await;
    let initialized = read_frame(reader).await.expect("initialized notification");
    assert_eq!(initialized["method"], "initialized");
}

/// Builds an [`LspServerInstance`] connected to a mock server that answers the handshake and then
/// forwards every subsequently received frame to the returned channel. Used by manager/tool tests
/// to assert which notifications/requests the manager emits.
pub(crate) async fn connected_instance(
    config: crate::config::ResolvedLspServerConfig,
) -> (
    Arc<crate::instance::LspServerInstance>,
    tokio::sync::mpsc::UnboundedReceiver<Value>,
) {
    let wired = wire();
    let transport = wired.transport;
    let mut server_reader = wired.server_reader;
    let mut server_writer = wired.server_writer;
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let server = tokio::spawn(async move {
        answer_handshake(&mut server_reader, &mut server_writer).await;
        while let Some(frame) = read_frame(&mut server_reader).await {
            if tx.send(frame).is_err() {
                break;
            }
        }
    });

    let instance = Arc::new(crate::instance::LspServerInstance::new(config));
    instance
        .connect(transport)
        .await
        .expect("mock handshake should succeed");
    // The server task keeps running to record later frames; detach it.
    drop(server);
    (instance, rx)
}
