//! Configuration for language servers Codex can drive, parsed from the `[lsp_servers]` table in
//! `config.toml`. Mirrors the `[mcp_servers]` shape, keyed by a user-chosen server name.
//!
//! This is the raw, on-disk view. The `codex-lsp` crate derives a validated runtime view
//! (absolute workspace folder, lowercased extension keys) from it so the transport never has to
//! re-validate.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

/// One configured language server, keyed by name under `[lsp_servers.<name>]`.
///
/// Unknown fields are rejected so that not-yet-implemented knobs (`shutdown_timeout_ms`,
/// `restart_on_crash`) and typos surface as config errors rather than being silently ignored.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct LspServerConfig {
    /// Executable to spawn (e.g. `"rust-analyzer"`, `"typescript-language-server"`).
    pub command: String,

    /// Arguments passed to the executable (e.g. `["--stdio"]`).
    #[serde(default)]
    pub args: Vec<String>,

    /// Extra environment variables for the spawned server process.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// REQUIRED, non-empty: maps a file extension (with leading dot, e.g. `".ts"`) to the LSP
    /// language id (e.g. `"typescript"`). Extensions are lowercased when the runtime view is built.
    pub extension_to_language: HashMap<String, String>,

    /// Workspace root sent to the server. Defaults to the session cwd when absent.
    #[serde(default)]
    pub workspace_folder: Option<String>,

    /// Forwarded verbatim as `initialize.initializationOptions`.
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,

    /// Maximum time to wait for `initialize`. No timeout when absent (matches the TS default).
    #[serde(default)]
    pub startup_timeout_ms: Option<u64>,

    /// Crash-recovery restart cap before the server is left in an error state. Defaults to 3.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,
}

const fn default_max_restarts() -> u32 {
    3
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserializes_minimal_config_with_defaults() {
        let cfg: LspServerConfig = toml::from_str(
            r#"
                command = "rust-analyzer"
                extension_to_language = { ".rs" = "rust" }
            "#,
        )
        .expect("minimal lsp server config should deserialize");

        assert_eq!(cfg.command, "rust-analyzer");
        assert!(cfg.args.is_empty());
        assert!(cfg.env.is_empty());
        assert_eq!(
            cfg.extension_to_language,
            HashMap::from([(".rs".to_string(), "rust".to_string())])
        );
        assert_eq!(cfg.workspace_folder, None);
        assert_eq!(cfg.initialization_options, None);
        assert_eq!(cfg.startup_timeout_ms, None);
        assert_eq!(cfg.max_restarts, 3);
    }

    #[test]
    fn deserializes_full_config() {
        let cfg: LspServerConfig = toml::from_str(
            r#"
                command = "typescript-language-server"
                args = ["--stdio"]
                env = { NODE_OPTIONS = "--max-old-space-size=4096" }
                extension_to_language = { ".ts" = "typescript", ".tsx" = "typescriptreact" }
                workspace_folder = "/work"
                startup_timeout_ms = 30000
                max_restarts = 5
            "#,
        )
        .expect("full lsp server config should deserialize");

        assert_eq!(cfg.args, vec!["--stdio".to_string()]);
        assert_eq!(cfg.workspace_folder.as_deref(), Some("/work"));
        assert_eq!(cfg.startup_timeout_ms, Some(30000));
        assert_eq!(cfg.max_restarts, 5);
        assert_eq!(cfg.extension_to_language.len(), 2);
    }

    #[test]
    fn rejects_unimplemented_fields() {
        // `shutdown_timeout_ms` / `restart_on_crash` are deferred (Phase 2) and must be rejected
        // rather than silently ignored.
        let err = toml::from_str::<LspServerConfig>(
            r#"
                command = "rust-analyzer"
                extension_to_language = { ".rs" = "rust" }
                restart_on_crash = true
            "#,
        )
        .expect_err("unknown field should be rejected");
        assert!(
            err.to_string().contains("restart_on_crash")
                || err.to_string().contains("unknown field"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn requires_command_and_extension_map() {
        toml::from_str::<LspServerConfig>(r#"extension_to_language = { ".rs" = "rust" }"#)
            .expect_err("missing command should fail");
        toml::from_str::<LspServerConfig>(r#"command = "rust-analyzer""#)
            .expect_err("missing extension_to_language should fail");
    }
}
