//! Passive diagnostic registry.
//!
//! Language servers push `textDocument/publishDiagnostics` notifications continuously. This
//! registry batches them (UUID-keyed), then on drain ([`DiagnosticRegistry::check_for_diagnostics`])
//! deduplicates within the batch and across turns, sorts by severity (errors first), and caps
//! volume (10 per file, 30 total) before they are surfaced to the model exactly once.
//!
//! Mirrors `Claude Code Src/src/services/lsp/{LSPDiagnosticRegistry,passiveFeedback}.ts`.

use std::collections::HashMap;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::Mutex;

use lru::LruCache;
use uuid::Uuid;

/// Per-file diagnostics kept after a file is truncated.
pub const MAX_DIAGNOSTICS_PER_FILE: usize = 10;
/// Total diagnostics surfaced across all files in one drain.
pub const MAX_TOTAL_DIAGNOSTICS: usize = 30;
/// LRU capacity (files tracked for cross-turn deduplication).
pub const MAX_DELIVERED_FILES: usize = 500;

/// Severity, ordered so `Error < Warning < Info < Hint` (errors sort first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

impl DiagnosticSeverity {
    /// Maps the LSP numeric severity (1..=4) to our enum; anything else defaults to `Error`,
    /// matching the TS `mapLSPSeverity`.
    pub fn from_lsp(value: Option<i64>) -> Self {
        match value {
            Some(1) => Self::Error,
            Some(2) => Self::Warning,
            Some(3) => Self::Info,
            Some(4) => Self::Hint,
            _ => Self::Error,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Info => "Info",
            Self::Hint => "Hint",
        }
    }
}

/// One diagnostic, normalized from the LSP payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticEntry {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub range: lsp_types::Range,
    pub source: Option<String>,
    pub code: Option<String>,
}

impl DiagnosticEntry {
    /// Stable dedup key over (message, severity, range, source, code), matching the TS
    /// `createDiagnosticKey`. `None` source/code serialize to JSON `null`.
    fn dedup_key(&self) -> String {
        serde_json::json!({
            "message": self.message,
            "severity": self.severity.label(),
            "range": self.range,
            "source": self.source,
            "code": self.code,
        })
        .to_string()
    }
}

/// Diagnostics for a single file URI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticFile {
    pub uri: String,
    pub diagnostics: Vec<DiagnosticEntry>,
}

struct PendingDiagnostic {
    files: Vec<DiagnosticFile>,
}

/// Stores pending diagnostic batches and the cross-turn delivered-key LRU.
pub struct DiagnosticRegistry {
    pending: Mutex<HashMap<Uuid, PendingDiagnostic>>,
    delivered: Mutex<LruCache<String, HashSet<String>>>,
}

impl Default for DiagnosticRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticRegistry {
    pub fn new() -> Self {
        let cap = NonZeroUsize::new(MAX_DELIVERED_FILES).expect("non-zero LRU capacity");
        Self {
            pending: Mutex::new(HashMap::new()),
            delivered: Mutex::new(LruCache::new(cap)),
        }
    }

    /// Registers a batch of files for later draining. Files with no diagnostics are dropped
    /// (a server clearing diagnostics), and an all-empty batch is not registered.
    pub fn register_pending(&self, files: Vec<DiagnosticFile>) {
        let files: Vec<DiagnosticFile> = files
            .into_iter()
            .filter(|f| !f.diagnostics.is_empty())
            .collect();
        if files.is_empty() {
            return;
        }
        self.pending
            .lock()
            .unwrap()
            .insert(Uuid::new_v4(), PendingDiagnostic { files });
    }

    /// Drains all pending batches and returns the diagnostics to surface this turn:
    /// within-batch + cross-turn dedup, severity sort (errors first), per-file cap then total cap.
    pub fn check_for_diagnostics(&self) -> Vec<DiagnosticFile> {
        let drained: Vec<PendingDiagnostic> = {
            let mut pending = self.pending.lock().unwrap();
            pending.drain().map(|(_, v)| v).collect()
        };

        // Merge by URI, preserving first-seen order across batches.
        let mut order: Vec<String> = Vec::new();
        let mut by_uri: HashMap<String, Vec<DiagnosticEntry>> = HashMap::new();
        for batch in drained {
            for file in batch.files {
                let entry = by_uri.entry(file.uri.clone()).or_insert_with(|| {
                    order.push(file.uri.clone());
                    Vec::new()
                });
                entry.extend(file.diagnostics);
            }
        }

        let mut delivered = self.delivered.lock().unwrap();
        let mut result: Vec<DiagnosticFile> = Vec::new();
        let mut total = 0usize;

        for uri in order {
            if total >= MAX_TOTAL_DIAGNOSTICS {
                break;
            }
            let mut diags = by_uri.remove(&uri).unwrap_or_default();

            // Dedup within batch and against previously delivered keys for this URI.
            let seen = delivered.get_or_insert_mut(uri.clone(), HashSet::new);
            let mut batch_seen: HashSet<String> = HashSet::new();
            diags.retain(|d| {
                let key = d.dedup_key();
                if seen.contains(&key) || !batch_seen.insert(key) {
                    return false;
                }
                true
            });
            if diags.is_empty() {
                continue;
            }

            // Severity sort (errors first), then per-file cap.
            diags.sort_by(|a, b| a.severity.cmp(&b.severity));
            diags.truncate(MAX_DIAGNOSTICS_PER_FILE);

            // Apply the remaining total budget.
            let remaining = MAX_TOTAL_DIAGNOSTICS - total;
            diags.truncate(remaining);
            if diags.is_empty() {
                continue;
            }

            // Mark the kept diagnostics as delivered.
            for d in &diags {
                seen.insert(d.dedup_key());
            }
            total += diags.len();
            result.push(DiagnosticFile {
                uri,
                diagnostics: diags,
            });
        }

        result
    }

    /// Forgets the delivered keys for a URI so its diagnostics will be re-surfaced after the file
    /// changes (called from the doc-sync path before `didChange`).
    pub fn clear_delivered_for_file(&self, file_uri: &str) {
        self.delivered.lock().unwrap().pop(file_uri);
    }
}

#[cfg(test)]
#[path = "diagnostics_tests.rs"]
mod diagnostics_tests;
