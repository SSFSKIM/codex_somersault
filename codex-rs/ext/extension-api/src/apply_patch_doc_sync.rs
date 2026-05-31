//! Doc-sync seam between the apply-patch runtime in `codex-core` and an optional LSP extension.
//!
//! `codex-core` cannot depend on `codex-lsp` (that would form a dependency cycle, since
//! `codex-lsp` depends on `codex-core`). Instead, the apply-patch runtime reaches the LSP
//! manager through this trait, which lives in `codex-extension-api` — a crate both sides already
//! depend on. The extension stores an [`ApplyPatchDocSyncHandle`] in the thread-scoped
//! `ExtensionData` at thread start; the apply-patch runtime reads it back (a no-op when absent)
//! and forwards committed file changes so the language server's open documents stay in sync.

use std::path::PathBuf;
use std::sync::Arc;

/// One committed file change, projected into a dependency-free shape for the doc-sync seam.
///
/// This mirrors the subset of `codex_apply_patch::AppliedPatchFileChange` that document
/// synchronization needs, without pulling the apply-patch crate into `codex-extension-api`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    /// Absolute path of the file that was changed.
    pub path: PathBuf,
    /// What happened to the file and (for adds/updates) its full new contents.
    pub kind: FileChangeKind,
}

/// The kind of change committed to a file, carrying full contents for adds and updates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileChangeKind {
    /// File was created with the given full contents (maps to `textDocument/didOpen`).
    Added(String),
    /// File was modified; carries the full new contents (maps to `didChange` + `didSave`).
    Updated(String),
    /// File was deleted (maps to `textDocument/didClose`).
    Deleted,
}

/// Receiver for committed apply-patch changes. Implemented by the LSP manager in `codex-lsp`.
///
/// Implementations must be cheap and non-blocking: the apply-patch runtime calls this on its hot
/// path, so any real I/O (sending `didChange`/`didSave` to a language server) should be
/// fire-and-forget (e.g. spawned onto a runtime), never awaited inline.
pub trait ApplyPatchDocSync: Send + Sync {
    /// Notify the document-sync subsystem about a batch of committed file changes.
    fn notify_patch_changes(&self, changes: &[FileChange]);
}

/// Concrete, `Sized` newtype wrapper so the handle can be stored in and retrieved from
/// [`crate::ExtensionData`].
///
/// `ExtensionData::get::<T>` downcasts an `Arc<dyn Any + Send + Sync>`, which requires a concrete
/// `Sized` key type — a bare `dyn ApplyPatchDocSync` cannot be used as that key. The extension
/// inserts `ApplyPatchDocSyncHandle(manager)` at thread start; the apply-patch runtime reads it
/// back via `get::<ApplyPatchDocSyncHandle>()`.
#[derive(Clone)]
pub struct ApplyPatchDocSyncHandle(pub Arc<dyn ApplyPatchDocSync>);

impl std::fmt::Debug for ApplyPatchDocSyncHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApplyPatchDocSyncHandle(..)")
    }
}
