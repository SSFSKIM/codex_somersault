//! Validated runtime view of an [`LspServerConfig`].
//!
//! The on-disk [`codex_config::LspServerConfig`] is normalized once at manager construction into a
//! [`ResolvedLspServerConfig`] (absolute workspace folder, lowercased extension keys with a leading
//! dot) so the transport and instance layers never re-validate.

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use codex_config::LspServerConfig;

/// A validated, ready-to-spawn language server configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedLspServerConfig {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) env: HashMap<String, String>,
    /// Lowercased extension (with leading dot) → LSP language id.
    pub(crate) extension_to_language: HashMap<String, String>,
    /// Absolute workspace root sent to the server.
    pub(crate) workspace_folder: PathBuf,
    pub(crate) initialization_options: Option<serde_json::Value>,
    pub(crate) startup_timeout_ms: Option<u64>,
    pub(crate) max_restarts: u32,
}

impl ResolvedLspServerConfig {
    /// Validates and normalizes `raw`, defaulting the workspace folder to `cwd`.
    ///
    /// Returns `Err` with a human-readable reason when the command is empty or no extensions are
    /// mapped; the manager logs and skips such servers rather than failing the whole session.
    pub(crate) fn resolve(name: &str, raw: &LspServerConfig, cwd: &Path) -> Result<Self, String> {
        if raw.command.trim().is_empty() {
            return Err(format!("lsp server `{name}` has an empty command"));
        }
        if raw.extension_to_language.is_empty() {
            return Err(format!(
                "lsp server `{name}` has an empty extension_to_language map"
            ));
        }

        let extension_to_language = raw
            .extension_to_language
            .iter()
            .map(|(ext, lang)| (normalize_extension(ext), lang.clone()))
            .collect();

        let workspace_folder = match &raw.workspace_folder {
            Some(folder) if !folder.trim().is_empty() => {
                let path = PathBuf::from(folder);
                if path.is_absolute() {
                    path
                } else {
                    cwd.join(path)
                }
            }
            _ => cwd.to_path_buf(),
        };

        Ok(Self {
            name: name.to_string(),
            command: raw.command.clone(),
            args: raw.args.clone(),
            env: raw.env.clone(),
            extension_to_language,
            workspace_folder,
            initialization_options: raw.initialization_options.clone(),
            startup_timeout_ms: raw.startup_timeout_ms,
            max_restarts: raw.max_restarts,
        })
    }
}

/// Lowercases an extension and ensures a single leading dot (`"RS"` → `".rs"`, `".TS"` → `".ts"`).
fn normalize_extension(ext: &str) -> String {
    let lowered = ext.trim().to_ascii_lowercase();
    if lowered.starts_with('.') {
        lowered
    } else {
        format!(".{lowered}")
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
