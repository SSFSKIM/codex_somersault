//! Fork-local glue forwarding committed apply-patch changes to the optional LSP document-sync
//! seam. Keeping this in its own module holds the edit to `apply_patch.rs` to two lines and avoids
//! a `codex-lsp ← codex-core` dependency cycle: the manager is reached through the
//! [`ApplyPatchDocSyncHandle`] trait object stored in thread-scoped `ExtensionData`, not by name.

use codex_apply_patch::AppliedPatchChange;
use codex_apply_patch::AppliedPatchFileChange;
use codex_extension_api::ApplyPatchDocSyncHandle;
use codex_extension_api::ExtensionData;
use codex_extension_api::FileChange;
use codex_extension_api::FileChangeKind;

/// If an LSP doc-sync handle is present in the thread store, forward the committed changes to it.
/// A no-op (and zero cost beyond a map lookup) when no LSP extension is active for the thread.
pub(crate) fn notify_if_present(thread_store: &ExtensionData, changes: &[AppliedPatchChange]) {
    let Some(handle) = thread_store.get::<ApplyPatchDocSyncHandle>() else {
        return;
    };
    let mapped = map_changes(changes);
    if mapped.is_empty() {
        return;
    }
    handle.0.notify_patch_changes(&mapped);
}

/// Projects committed apply-patch changes onto the dependency-free [`FileChange`] shape:
/// Add → Added(content), Update → Updated(new_content), Delete → Deleted.
fn map_changes(changes: &[AppliedPatchChange]) -> Vec<FileChange> {
    changes
        .iter()
        .map(|change| {
            let kind = match &change.change {
                AppliedPatchFileChange::Add { content, .. } => {
                    FileChangeKind::Added(content.clone())
                }
                AppliedPatchFileChange::Update { new_content, .. } => {
                    FileChangeKind::Updated(new_content.clone())
                }
                AppliedPatchFileChange::Delete { .. } => FileChangeKind::Deleted,
            };
            FileChange {
                path: change.path.clone(),
                kind,
            }
        })
        .collect()
}
