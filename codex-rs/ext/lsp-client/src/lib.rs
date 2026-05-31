//! `codex-lsp` — optional Language Server Protocol client extension for Codex.
//!
//! This crate ports Claude Code's three-layer LSP stack (transport → server instance →
//! routing manager), its passive diagnostic registry, and a model-facing `lsp/query` tool
//! into a self-contained extension crate. It is wired into the agent through the existing
//! extension machinery (see [`install`]) so that `codex-core` stays almost untouched.
//!
//! Design: `docs/proposals/03-lsp-crate.md`.

mod config;
mod diagnostics;
mod format;
mod instance;
mod manager;
mod schema;
mod tool;
mod transport;

#[cfg(test)]
mod test_support;

pub use manager::LspManager;
pub use manager::ManagerDocSync;

use codex_core::config::Config;
use codex_extension_api::ExtensionRegistryBuilder;

/// Registers the LSP extension on the host's extension registry.
///
/// This is the single install seam called from `app-server/src/extensions.rs`. It reads the
/// `[lsp_servers]` config at thread start and, for interactive sessions, builds a thread-scoped
/// `LspManager` that the contributed `lsp/query` tool and the apply-patch doc-sync hook share.
///
/// The body is filled in once the extension contributors land (build sequence step 9); the
/// signature is intentionally argument-free because everything it needs is read from `Config`
/// in `on_thread_start`.
pub fn install(_registry: &mut ExtensionRegistryBuilder<Config>) {}
