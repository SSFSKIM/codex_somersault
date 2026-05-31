//! Real-language-server smoke test. Ignored by default (requires `rust-analyzer` on PATH and is
//! slow). Run explicitly with:
//!
//! ```bash
//! cargo test -p codex-lsp --features '' -- --ignored real_rust_analyzer_hover
//! ```
//!
//! Drives the full `LspManager → LspServerInstance → LspTransport` stack against an actual
//! `rust-analyzer` process indexing a tiny fixture crate, and asserts a real hover response.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use codex_config::LspServerConfig;
use serde_json::Value;
use serde_json::json;

use crate::config::ResolvedLspServerConfig;
use crate::instance::LspServerInstance;
use crate::instance::path_to_uri;

const FIXTURE_LIB: &str = "\
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn caller() -> i32 {
    add(1, 2)
}
";

fn write_fixture_crate(dir: &Path) -> PathBuf {
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create src");
    let lib = src.join("lib.rs");
    std::fs::write(&lib, FIXTURE_LIB).expect("write lib.rs");
    lib
}

fn rust_config(cwd: &Path) -> HashMap<String, LspServerConfig> {
    // The repo pins a Rust toolchain (rust-toolchain.toml) that may lack the rust-analyzer
    // rustup component; force the spawned server to the toolchain that has it. In a normal user
    // environment `rust-analyzer` resolves directly and this env override is unnecessary — it also
    // exercises the `[lsp_servers.*].env` config field.
    let env = match std::env::var("CODEX_LSP_SMOKE_TOOLCHAIN") {
        Ok(tc) => HashMap::from([("RUSTUP_TOOLCHAIN".to_string(), tc)]),
        Err(_) => HashMap::from([("RUSTUP_TOOLCHAIN".to_string(), "stable".to_string())]),
    };
    HashMap::from([(
        "rust".to_string(),
        LspServerConfig {
            command: "rust-analyzer".to_string(),
            args: Vec::new(),
            env,
            extension_to_language: HashMap::from([(".rs".to_string(), "rust".to_string())]),
            workspace_folder: Some(cwd.to_string_lossy().into_owned()),
            initialization_options: None,
            startup_timeout_ms: None,
            max_restarts: 3,
        },
    )])
}

/// Extracts hover text from a `textDocument/hover` result, if any.
fn hover_text(value: &Value) -> Option<String> {
    let contents = value.get("contents")?;
    let text = match contents {
        Value::String(s) => s.clone(),
        Value::Object(map) => map.get("value").and_then(Value::as_str)?.to_string(),
        _ => return None,
    };
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[tokio::test]
#[ignore = "diagnostic: drives tokio::process + manual framing against rust-analyzer"]
async fn tokio_spawn_rust_analyzer_responds() {
    use tokio::io::AsyncBufReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::io::BufReader;

    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_crate(dir.path());

    let toolchain =
        std::env::var("CODEX_LSP_SMOKE_TOOLCHAIN").unwrap_or_else(|_| "stable".to_string());
    let mut child = tokio::process::Command::new("rust-analyzer")
        .current_dir(dir.path())
        .env("RUSTUP_TOOLCHAIN", toolchain)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn rust-analyzer");

    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    let body = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":12345,"rootUri":null,"capabilities":{}}})
        .to_string();
    let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    stdin
        .write_all(frame.as_bytes())
        .await
        .expect("write frame");
    stdin.flush().await.expect("flush");

    let mut line = String::new();
    let read = tokio::time::timeout(Duration::from_secs(15), stdout.read_line(&mut line)).await;
    eprintln!("read result: {read:?}; first line: {line:?}");
    child.start_kill().ok();
    assert!(
        line.to_ascii_lowercase().contains("content-length"),
        "expected a framed response"
    );
}

#[tokio::test]
#[ignore = "requires a real rust-analyzer on PATH; slow (spawns + indexes a crate)"]
async fn real_rust_analyzer_hover() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lib = write_fixture_crate(dir.path());

    let configs = rust_config(dir.path());
    let raw = configs.get("rust").expect("config present");
    let resolved =
        ResolvedLspServerConfig::resolve("rust", raw, dir.path()).expect("resolve config");
    let instance = LspServerInstance::new(resolved);

    // Start the real rust-analyzer process and run the initialize handshake. Surfaces the exact
    // LspError (Spawn / Rpc / Closed) if anything goes wrong.
    instance
        .start()
        .await
        .expect("rust-analyzer should start and initialize");

    // didOpen the fixture file so the server has its contents.
    let uri = path_to_uri(&lib);
    instance
        .notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": FIXTURE_LIB,
                }
            }),
        )
        .await
        .expect("didOpen");

    // Hover over `add` in `pub fn add(...)` — 0-based line 0, character 7.
    let params = json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 7 },
    });

    // rust-analyzer returns null / ContentModified until it finishes indexing; poll up to 90s.
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut last: Option<String> = None;
    while Instant::now() < deadline {
        match instance
            .request_value("textDocument/hover", params.clone())
            .await
        {
            Ok(value) => {
                if let Some(text) = hover_text(&value) {
                    last = Some(text);
                    break;
                }
            }
            Err(err) => eprintln!("hover transient: {err}"),
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    instance.stop().await;

    let hover = last.expect("rust-analyzer should return a non-empty hover within 90s");
    eprintln!("rust-analyzer hover:\n{hover}");
    assert!(
        hover.contains("add") && hover.contains("i32"),
        "hover should describe `fn add(a: i32, b: i32) -> i32`, got:\n{hover}"
    );
}
