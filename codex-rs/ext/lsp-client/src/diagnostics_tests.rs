use super::*;
use lsp_types::Position;
use lsp_types::Range;
use pretty_assertions::assert_eq;

fn range(line: u32) -> Range {
    Range {
        start: Position { line, character: 0 },
        end: Position { line, character: 1 },
    }
}

fn diag(severity: DiagnosticSeverity, message: &str, line: u32) -> DiagnosticEntry {
    DiagnosticEntry {
        severity,
        message: message.to_string(),
        range: range(line),
        source: Some("test".to_string()),
        code: None,
    }
}

fn file(uri: &str, diagnostics: Vec<DiagnosticEntry>) -> DiagnosticFile {
    DiagnosticFile {
        uri: uri.to_string(),
        diagnostics,
    }
}

#[test]
fn drops_empty_diagnostic_batches() {
    let reg = DiagnosticRegistry::new();
    reg.register_pending(vec![file("file:///a.rs", vec![])]);
    assert!(reg.check_for_diagnostics().is_empty());
}

#[test]
fn dedups_across_turns() {
    let reg = DiagnosticRegistry::new();
    let d = diag(DiagnosticSeverity::Error, "boom", 1);

    reg.register_pending(vec![file("file:///a.rs", vec![d.clone()])]);
    let first = reg.check_for_diagnostics();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].diagnostics.len(), 1);

    // Same diagnostic re-published next turn: already delivered, so nothing surfaces.
    reg.register_pending(vec![file("file:///a.rs", vec![d])]);
    assert!(reg.check_for_diagnostics().is_empty());
}

#[test]
fn dedups_within_a_batch() {
    let reg = DiagnosticRegistry::new();
    let d = diag(DiagnosticSeverity::Warning, "dup", 2);
    reg.register_pending(vec![file("file:///a.rs", vec![d.clone(), d])]);
    let out = reg.check_for_diagnostics();
    assert_eq!(out[0].diagnostics.len(), 1);
}

#[test]
fn clear_delivered_allows_resurfacing() {
    let reg = DiagnosticRegistry::new();
    let d = diag(DiagnosticSeverity::Error, "boom", 1);
    reg.register_pending(vec![file("file:///a.rs", vec![d.clone()])]);
    assert_eq!(reg.check_for_diagnostics().len(), 1);

    reg.clear_delivered_for_file("file:///a.rs");
    reg.register_pending(vec![file("file:///a.rs", vec![d])]);
    assert_eq!(
        reg.check_for_diagnostics().len(),
        1,
        "should re-surface after clear"
    );
}

#[test]
fn sorts_errors_before_warnings() {
    let reg = DiagnosticRegistry::new();
    reg.register_pending(vec![file(
        "file:///a.rs",
        vec![
            diag(DiagnosticSeverity::Warning, "w", 1),
            diag(DiagnosticSeverity::Error, "e", 2),
            diag(DiagnosticSeverity::Hint, "h", 3),
        ],
    )]);
    let out = reg.check_for_diagnostics();
    let severities: Vec<_> = out[0].diagnostics.iter().map(|d| d.severity).collect();
    assert_eq!(
        severities,
        vec![
            DiagnosticSeverity::Error,
            DiagnosticSeverity::Warning,
            DiagnosticSeverity::Hint
        ]
    );
}

#[test]
fn caps_per_file_at_ten_keeping_most_severe() {
    let reg = DiagnosticRegistry::new();
    let mut diags = Vec::new();
    // 5 hints then 8 errors on distinct lines (distinct dedup keys).
    for line in 0..5 {
        diags.push(diag(DiagnosticSeverity::Hint, "h", line));
    }
    for line in 5..13 {
        diags.push(diag(DiagnosticSeverity::Error, "e", line));
    }
    reg.register_pending(vec![file("file:///a.rs", diags)]);
    let out = reg.check_for_diagnostics();
    assert_eq!(out[0].diagnostics.len(), MAX_DIAGNOSTICS_PER_FILE);
    // All 8 errors must be kept (most severe), filling the rest with hints.
    let errors = out[0]
        .diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .count();
    assert_eq!(errors, 8);
}

#[test]
fn caps_total_at_thirty() {
    let reg = DiagnosticRegistry::new();
    // 5 files × 10 errors = 50, but total is capped at 30.
    let mut files = Vec::new();
    for f in 0..5 {
        let diags = (0..10)
            .map(|line| diag(DiagnosticSeverity::Error, "e", f * 100 + line))
            .collect();
        files.push(file(&format!("file:///{f}.rs"), diags));
    }
    reg.register_pending(files);
    let out = reg.check_for_diagnostics();
    let total: usize = out.iter().map(|f| f.diagnostics.len()).sum();
    assert_eq!(total, MAX_TOTAL_DIAGNOSTICS);
}
