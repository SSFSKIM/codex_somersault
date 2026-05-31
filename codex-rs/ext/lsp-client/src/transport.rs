//! Async JSON-RPC client over a child process's stdio using LSP `Content-Length` framing.
//!
//! The framer is split from process spawning so it can be tested against an in-process
//! [`tokio::io::duplex`] pair: [`LspTransport::connect`] runs the reader/writer tasks over any
//! async byte streams, while [`LspTransport::spawn`] wires a real child's stdio into `connect`.
//!
//! Wire shape (one frame): `Content-Length: <N>\r\n\r\n<N bytes of JSON>`. The reader routes by
//! JSON-RPC `id` into a pending-request map, dispatches id-less messages to notification handlers,
//! and answers server→client requests via registered responders (e.g. `workspace/configuration`).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde_json::json;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Error from a JSON-RPC request: either a protocol error object from the server, or a closed
/// transport (process gone / streams ended).
#[derive(Debug, thiserror::Error)]
pub(crate) enum JsonRpcError {
    #[error("json-rpc error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("transport closed")]
    Closed,
}

impl JsonRpcError {
    pub(crate) fn code(&self) -> Option<i64> {
        match self {
            JsonRpcError::Rpc { code, .. } => Some(*code),
            JsonRpcError::Closed => None,
        }
    }
}

type NotificationHandler = Arc<dyn Fn(Value) + Send + Sync>;
type RequestResponder = Arc<dyn Fn(Value) -> Value + Send + Sync>;
type PendingMap = Arc<std::sync::Mutex<HashMap<i64, oneshot::Sender<Result<Value, JsonRpcError>>>>>;
type Handlers = Arc<std::sync::Mutex<HashMap<String, NotificationHandler>>>;
type Responders = Arc<std::sync::Mutex<HashMap<String, RequestResponder>>>;

pub(crate) struct LspTransport {
    child: Mutex<Option<Child>>,
    outgoing: mpsc::Sender<Vec<u8>>,
    pending: PendingMap,
    notification_handlers: Handlers,
    request_responders: Responders,
    next_id: AtomicI64,
    reader_task: JoinHandle<()>,
    writer_task: JoinHandle<()>,
}

impl LspTransport {
    /// Spawns `command` with piped stdio and starts the framer.
    ///
    /// Process-exit observation is intentionally implicit in v1: when the child dies its streams
    /// close, the reader loop ends and fails all in-flight requests with [`JsonRpcError::Closed`],
    /// which the instance layer treats as "server gone". Explicit exit-status callbacks and
    /// restart-on-crash are Phase 2 (see `docs/proposals/03-lsp-crate.md` §7).
    pub(crate) async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: &std::path::Path,
    ) -> std::io::Result<Self> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .envs(env)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        let mut child = cmd.spawn()?;

        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");
        if let Some(stderr) = child.stderr.take() {
            spawn_stderr_drain(command.to_string(), stderr);
        }

        Ok(Self::connect(stdout, stdin, Some(child)))
    }

    /// Starts the reader/writer framer over arbitrary async byte streams. Used by [`Self::spawn`]
    /// and directly by tests (with [`tokio::io::duplex`]). When `child` is `None`,
    /// [`Self::shutdown`] just tears down the tasks (no process to kill).
    pub(crate) fn connect<R, W>(reader: R, writer: W, child: Option<Child>) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let pending: PendingMap = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let notification_handlers: Handlers = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let request_responders: Responders = Arc::new(std::sync::Mutex::new(HashMap::new()));
        let (outgoing, outgoing_rx) = mpsc::channel::<Vec<u8>>(64);

        let writer_task = tokio::spawn(writer_loop(writer, outgoing_rx));
        let reader_task = tokio::spawn(reader_loop(
            BufReader::new(reader),
            pending.clone(),
            notification_handlers.clone(),
            request_responders.clone(),
            outgoing.clone(),
        ));

        Self {
            child: Mutex::new(child),
            outgoing,
            pending,
            notification_handlers,
            request_responders,
            next_id: AtomicI64::new(1),
            reader_task,
            writer_task,
        }
    }

    /// Sends a request and awaits the typed result.
    pub(crate) async fn request<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R, JsonRpcError> {
        let value = self
            .request_value(method, serde_json::to_value(params).unwrap_or(Value::Null))
            .await?;
        serde_json::from_value(value).map_err(|e| JsonRpcError::Rpc {
            code: -32603,
            message: format!("failed to deserialize result for {method}: {e}"),
        })
    }

    /// Sends a request whose params are already a [`Value`] and returns the raw result value.
    pub(crate) async fn request_value(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, JsonRpcError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            pending.insert(id, tx);
        }
        let frame = encode_frame(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        if self.outgoing.send(frame).await.is_err() {
            self.pending.lock().unwrap().remove(&id);
            return Err(JsonRpcError::Closed);
        }
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(JsonRpcError::Closed),
        }
    }

    /// Sends a notification (no response expected).
    pub(crate) async fn notify<P: Serialize>(
        &self,
        method: &str,
        params: P,
    ) -> Result<(), JsonRpcError> {
        let frame = encode_frame(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": serde_json::to_value(params).unwrap_or(Value::Null),
        }));
        self.outgoing
            .send(frame)
            .await
            .map_err(|_| JsonRpcError::Closed)
    }

    /// Registers a handler for inbound server-push notifications (e.g. `publishDiagnostics`).
    pub(crate) fn on_notification(&self, method: &str, handler: NotificationHandler) {
        self.notification_handlers
            .lock()
            .unwrap()
            .insert(method.to_string(), handler);
    }

    /// Registers a responder for server→client requests (e.g. `workspace/configuration` → null[]).
    pub(crate) fn on_request(&self, method: &str, responder: RequestResponder) {
        self.request_responders
            .lock()
            .unwrap()
            .insert(method.to_string(), responder);
    }

    /// `shutdown` request + `exit` notification + kill the child (best-effort).
    ///
    /// Takes `&self` so the transport can be held in an `Arc` (shared across concurrent requests in
    /// the instance layer) and still be torn down.
    pub(crate) async fn shutdown(&self) {
        // Politely ask the server to shut down, ignoring errors (it may already be gone).
        let _ = self.request_value("shutdown", Value::Null).await;
        let _ = self.notify("exit", Value::Null).await;

        if let Some(mut child) = self.child.lock().await.take() {
            // SIGTERM (start_kill) then a bounded wait, then SIGKILL via kill_on_drop on drop.
            let _ = child.start_kill();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), child.wait()).await;
        }
        self.reader_task.abort();
        self.writer_task.abort();
    }
}

/// Encodes one JSON value as a `Content-Length`-framed LSP message.
fn encode_frame(value: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(value).unwrap_or_default();
    let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    frame.extend_from_slice(&body);
    frame
}

/// Drains outgoing frames into the child's stdin.
async fn writer_loop<W: AsyncWrite + Unpin>(mut writer: W, mut rx: mpsc::Receiver<Vec<u8>>) {
    while let Some(frame) = rx.recv().await {
        if writer.write_all(&frame).await.is_err() {
            break;
        }
        if writer.flush().await.is_err() {
            break;
        }
    }
}

/// Reads `Content-Length`-framed messages and routes them.
async fn reader_loop<R: AsyncRead + Unpin>(
    mut reader: BufReader<R>,
    pending: PendingMap,
    notification_handlers: Handlers,
    request_responders: Responders,
    outgoing: mpsc::Sender<Vec<u8>>,
) {
    loop {
        let len = match read_content_length(&mut reader).await {
            Some(len) => len,
            None => break, // streams closed
        };
        let mut body = vec![0u8; len];
        if reader.read_exact(&mut body).await.is_err() {
            break;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&body) else {
            tracing::debug!("lsp: dropping unparseable frame");
            continue;
        };
        dispatch_message(
            message,
            &pending,
            &notification_handlers,
            &request_responders,
            &outgoing,
        )
        .await;
    }
    // Streams closed: fail all in-flight requests so callers don't hang.
    let mut pending = pending.lock().unwrap();
    for (_, tx) in pending.drain() {
        let _ = tx.send(Err(JsonRpcError::Closed));
    }
}

/// Routes one decoded message to a pending response, a notification handler, or a request responder.
async fn dispatch_message(
    message: Value,
    pending: &PendingMap,
    notification_handlers: &Handlers,
    request_responders: &Responders,
    outgoing: &mpsc::Sender<Vec<u8>>,
) {
    let has_method = message.get("method").is_some();
    let id = message.get("id").and_then(Value::as_i64);

    if !has_method {
        // Response to one of our requests.
        if let Some(id) = id
            && let Some(tx) = pending.lock().unwrap().remove(&id)
        {
            if let Some(err) = message.get("error") {
                let code = err.get("code").and_then(Value::as_i64).unwrap_or(-32603);
                let msg = err
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
                    .to_string();
                let _ = tx.send(Err(JsonRpcError::Rpc { code, message: msg }));
            } else {
                let result = message.get("result").cloned().unwrap_or(Value::Null);
                let _ = tx.send(Ok(result));
            }
        }
        return;
    }

    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    if let Some(id) = id {
        // Server→client request: answer with the registered responder (or null).
        let responder = request_responders.lock().unwrap().get(&method).cloned();
        let result = responder.map(|r| r(params)).unwrap_or(Value::Null);
        let frame = encode_frame(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result,
        }));
        let _ = outgoing.send(frame).await;
    } else {
        // Server→client notification.
        let handler = notification_handlers.lock().unwrap().get(&method).cloned();
        if let Some(handler) = handler {
            handler(params);
        }
    }
}

/// Parses headers up to the blank line and returns the `Content-Length`, or `None` at EOF.
async fn read_content_length<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> Option<usize> {
    use tokio::io::AsyncBufReadExt;
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await.ok()?;
        if n == 0 {
            return None; // EOF
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // End of headers.
            return content_length;
        }
        if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
}

/// Spawns a task that drains a child's stderr to `tracing::debug`.
fn spawn_stderr_drain(server: String, stderr: tokio::process::ChildStderr) {
    tokio::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if !line.trim().is_empty() {
                tracing::debug!(server = %server, "lsp stderr: {line}");
            }
        }
    });
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod transport_tests;
