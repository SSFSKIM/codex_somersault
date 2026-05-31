//! The model-facing `lsp/query` tool.
//!
//! A single namespaced tool exposes nine operations through an `operation` enum (matching the TS
//! single-tool shape). Input line/character are **1-based** and converted to 0-based for the LSP
//! wire. Files are lazily opened on first access (rejecting files over 10 MB). Location results are
//! filtered through `git check-ignore` so ignored files (e.g. vendored deps) are omitted.
//!
//! Mirrors `Claude Code Src/src/tools/LSPTool/LSPTool.ts`.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use codex_extension_api::FunctionCallError;
use codex_extension_api::JsonToolOutput;
use codex_extension_api::ResponsesApiTool;
use codex_extension_api::ToolCall;
use codex_extension_api::ToolExecutor;
use codex_extension_api::ToolName;
use codex_extension_api::ToolOutput;
use codex_extension_api::ToolSpec;
use codex_extension_api::parse_tool_input_schema;
use codex_tools::ResponsesApiNamespace;
use codex_tools::ResponsesApiNamespaceTool;
use codex_tools::ToolExposure;
use codex_tools::default_namespace_description;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use crate::format;
use crate::format::Location;
use crate::instance::path_to_uri;
use crate::manager::LspManager;
use crate::schema::input_schema_for;

/// Files larger than this are rejected from LSP analysis (matches the TS `MAX_LSP_FILE_SIZE_BYTES`).
const MAX_LSP_FILE_SIZE_BYTES: u64 = 10_000_000;
/// Paths per `git check-ignore` invocation.
const GIT_CHECK_IGNORE_BATCH_SIZE: usize = 50;
const LSP_NAMESPACE: &str = "lsp";
const LSP_TOOL_NAME: &str = "query";

pub(crate) struct LspTool {
    manager: Arc<LspManager>,
}

impl LspTool {
    pub(crate) fn new(manager: Arc<LspManager>) -> Self {
        Self { manager }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LspToolInput {
    /// Which language-server operation to run.
    operation: LspOperation,
    /// Absolute or cwd-relative path of the file to query.
    file_path: String,
    /// 1-based line of the symbol of interest.
    #[serde(default = "default_one")]
    line: u32,
    /// 1-based character (column) of the symbol of interest.
    #[serde(default = "default_one")]
    character: u32,
}

fn default_one() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum LspOperation {
    GoToDefinition,
    FindReferences,
    Hover,
    DocumentSymbol,
    WorkspaceSymbol,
    GoToImplementation,
    PrepareCallHierarchy,
    IncomingCalls,
    OutgoingCalls,
}

#[async_trait]
impl ToolExecutor<ToolCall> for LspTool {
    fn tool_name(&self) -> ToolName {
        ToolName::namespaced(LSP_NAMESPACE, LSP_TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        let tool = ResponsesApiTool {
            name: LSP_TOOL_NAME.to_string(),
            description: TOOL_DESCRIPTION.to_string(),
            strict: false,
            defer_loading: None,
            parameters: parse_tool_input_schema(&input_schema_for::<LspToolInput>())
                .expect("LspToolInput schema should parse"),
            output_schema: None,
        };
        ToolSpec::Namespace(ResponsesApiNamespace {
            name: LSP_NAMESPACE.to_string(),
            description: default_namespace_description(LSP_NAMESPACE),
            tools: vec![ResponsesApiNamespaceTool::Function(tool)],
        })
    }

    fn exposure(&self) -> ToolExposure {
        ToolExposure::Direct
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        true // read-only queries
    }

    async fn handle(&self, call: ToolCall) -> Result<Box<dyn ToolOutput>, FunctionCallError> {
        let input: LspToolInput = parse_args(&call)?;
        let formatted = self.run(input).await?;
        Ok(Box::new(JsonToolOutput::new(Value::String(formatted))))
    }
}

impl LspTool {
    /// Runs one parsed operation and returns the formatted, model-facing text. Split from
    /// [`ToolExecutor::handle`] so tests can exercise dispatch without building a full `ToolCall`.
    async fn run(&self, input: LspToolInput) -> Result<String, FunctionCallError> {
        let path = self.manager.resolve_path(&input.file_path);
        self.ensure_open(&path).await?;

        // 1-based (model) → 0-based (LSP wire).
        let position = json!({
            "line": input.line.saturating_sub(1),
            "character": input.character.saturating_sub(1),
        });
        let text_document = json!({ "uri": path_to_uri(&path) });

        let formatted = match input.operation {
            LspOperation::Hover => {
                let v = self
                    .request(&path, "textDocument/hover", json!({"textDocument": text_document, "position": position}))
                    .await?;
                format::format_hover(&v)
            }
            LspOperation::GoToDefinition => {
                self.locations(&path, "textDocument/definition", &text_document, &position, "definition")
                    .await?
            }
            LspOperation::GoToImplementation => {
                self.locations(&path, "textDocument/implementation", &text_document, &position, "implementation")
                    .await?
            }
            LspOperation::FindReferences => {
                let params = json!({
                    "textDocument": text_document,
                    "position": position,
                    "context": {"includeDeclaration": true},
                });
                let v = self.request(&path, "textDocument/references", params).await?;
                let locations = self.filter_ignored(format::parse_locations(&v)).await;
                format::format_locations(&locations, "reference")
            }
            LspOperation::DocumentSymbol => {
                let v = self
                    .request(&path, "textDocument/documentSymbol", json!({"textDocument": text_document}))
                    .await?;
                format::format_document_symbols(&v)
            }
            LspOperation::WorkspaceSymbol => {
                // v1 ride-along: query all symbols (the single-tool input carries no query string).
                let v = self.request(&path, "workspace/symbol", json!({"query": ""})).await?;
                format::format_workspace_symbols(&v)
            }
            LspOperation::PrepareCallHierarchy => {
                let v = self
                    .request(&path, "textDocument/prepareCallHierarchy", json!({"textDocument": text_document, "position": position}))
                    .await?;
                format::format_call_hierarchy_items(&v)
            }
            LspOperation::IncomingCalls => {
                self.call_hierarchy(&path, &text_document, &position, CallDirection::Incoming).await?
            }
            LspOperation::OutgoingCalls => {
                self.call_hierarchy(&path, &text_document, &position, CallDirection::Outgoing).await?
            }
        };

        Ok(formatted)
    }

    /// Lazily opens the file (rejecting >10 MB) so the server has its contents.
    async fn ensure_open(&self, path: &Path) -> Result<(), FunctionCallError> {
        if self.manager.is_file_open(path).await {
            return Ok(());
        }
        let meta = tokio::fs::metadata(path).await.map_err(|e| {
            FunctionCallError::RespondToModel(format!("cannot stat {}: {e}", path.display()))
        })?;
        if meta.len() > MAX_LSP_FILE_SIZE_BYTES {
            return Err(FunctionCallError::RespondToModel(format!(
                "File too large for LSP analysis ({} bytes exceeds {MAX_LSP_FILE_SIZE_BYTES} byte limit)",
                meta.len()
            )));
        }
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            FunctionCallError::RespondToModel(format!("cannot read {}: {e}", path.display()))
        })?;
        self.manager.open_file(path, &content).await;
        Ok(())
    }

    async fn request(
        &self,
        path: &Path,
        method: &str,
        params: Value,
    ) -> Result<Value, FunctionCallError> {
        match self.manager.request_value(path, method, params).await {
            Some(Ok(value)) => Ok(value),
            Some(Err(err)) => {
                Err(FunctionCallError::RespondToModel(format!("lsp request `{method}` failed: {err}")))
            }
            None => Err(FunctionCallError::RespondToModel(format!(
                "no language server is configured for {}",
                path.display()
            ))),
        }
    }

    async fn locations(
        &self,
        path: &Path,
        method: &str,
        text_document: &Value,
        position: &Value,
        label: &str,
    ) -> Result<String, FunctionCallError> {
        let v = self
            .request(path, method, json!({"textDocument": text_document, "position": position}))
            .await?;
        let locations = self.filter_ignored(format::parse_locations(&v)).await;
        Ok(format::format_locations(&locations, label))
    }

    /// Two-step call hierarchy: `prepareCallHierarchy` then incoming/outgoing on the first item.
    async fn call_hierarchy(
        &self,
        path: &Path,
        text_document: &Value,
        position: &Value,
        direction: CallDirection,
    ) -> Result<String, FunctionCallError> {
        let items = self
            .request(path, "textDocument/prepareCallHierarchy", json!({"textDocument": text_document, "position": position}))
            .await?;
        let Some(first) = items.as_array().and_then(|a| a.first()).cloned() else {
            return Ok("No call hierarchy items found.".to_string());
        };
        let (method, _) = direction.method();
        let result = self.request(path, method, json!({"item": first})).await?;
        Ok(match direction {
            CallDirection::Incoming => format::format_incoming_calls(&result),
            CallDirection::Outgoing => format::format_outgoing_calls(&result),
        })
    }

    /// Drops locations whose files are git-ignored (best-effort: returns all on non-git repos or
    /// when `git` is unavailable). Paths are checked in batches of 50.
    async fn filter_ignored(&self, locations: Vec<Location>) -> Vec<Location> {
        let paths: Vec<(usize, PathBuf)> = locations
            .iter()
            .enumerate()
            .filter_map(|(i, loc)| uri_to_path(&loc.uri).map(|p| (i, p)))
            .collect();
        if paths.is_empty() {
            return locations;
        }

        let mut ignored_indices = std::collections::HashSet::new();
        for batch in paths.chunks(GIT_CHECK_IGNORE_BATCH_SIZE) {
            let mut cmd = tokio::process::Command::new("git");
            cmd.arg("-C").arg(self.manager.cwd()).arg("check-ignore");
            for (_, p) in batch {
                cmd.arg(p);
            }
            let Ok(output) = cmd.output().await else {
                return locations; // git missing → don't filter
            };
            // Exit 128 = not a git repo; treat as "nothing ignored".
            if output.status.code() == Some(128) {
                return locations;
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            let ignored: std::collections::HashSet<String> =
                stdout.lines().map(|l| l.trim().to_string()).collect();
            for (i, p) in batch {
                if ignored.contains(&p.to_string_lossy().to_string()) {
                    ignored_indices.insert(*i);
                }
            }
        }

        locations
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !ignored_indices.contains(i))
            .map(|(_, loc)| loc)
            .collect()
    }
}

#[derive(Clone, Copy)]
enum CallDirection {
    Incoming,
    Outgoing,
}

impl CallDirection {
    fn method(self) -> (&'static str, &'static str) {
        match self {
            CallDirection::Incoming => ("callHierarchy/incomingCalls", "from"),
            CallDirection::Outgoing => ("callHierarchy/outgoingCalls", "to"),
        }
    }
}

/// Converts a `file://` URI to a filesystem path.
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    url::Url::parse(uri).ok()?.to_file_path().ok()
}

/// Parses the tool-call arguments into `LspToolInput`.
fn parse_args(call: &ToolCall) -> Result<LspToolInput, FunctionCallError> {
    let arguments = call.function_arguments()?;
    serde_json::from_str(arguments)
        .map_err(|e| FunctionCallError::RespondToModel(format!("invalid lsp tool arguments: {e}")))
}

const TOOL_DESCRIPTION: &str = "\
Query a language server for code intelligence about a file. Operations:
- goToDefinition: where the symbol at (line, character) is defined.
- findReferences: all references to the symbol at (line, character).
- hover: type/doc information for the symbol at (line, character).
- goToImplementation: implementations of the symbol at (line, character).
- documentSymbol: the symbol outline of the file.
- workspaceSymbol: symbols across the workspace.
- prepareCallHierarchy / incomingCalls / outgoingCalls: call hierarchy for the symbol.
`line` and `character` are 1-based. Diagnostics are surfaced automatically after edits and are not \
requested through this tool.";

#[cfg(test)]
#[path = "tool_tests.rs"]
mod tool_tests;
