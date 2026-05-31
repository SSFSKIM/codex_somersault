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
mod extension;
mod format;
mod instance;
mod manager;
mod schema;
mod tool;
mod transport;

#[cfg(test)]
mod test_support;

pub use extension::install;
pub use manager::LspManager;
pub use manager::ManagerDocSync;
