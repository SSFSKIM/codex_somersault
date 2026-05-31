//! Human-readable formatting of LSP results.
//!
//! Results are parsed from generic `serde_json::Value` (rather than strict `lsp_types` structs) so
//! the formatters tolerate server-to-server variation (e.g. `Location` vs `LocationLink`, single vs
//! array, `DocumentSymbol` vs `SymbolInformation`). Positions are rendered 1-based for humans.

use serde_json::Value;

/// A source location flattened from `Location` / `LocationLink` for filtering and display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Location {
    pub(crate) uri: String,
    /// 0-based line (LSP wire value).
    pub(crate) line: u32,
    /// 0-based character (LSP wire value).
    pub(crate) character: u32,
}

impl Location {
    /// `file:///a.rs:LINE:COL`, rendered 1-based.
    fn display(&self) -> String {
        format!("{}:{}:{}", self.uri, self.line + 1, self.character + 1)
    }
}

/// Parses a definition/references/implementation result into flat [`Location`]s, accepting a single
/// object, an array, `Location`, or `LocationLink` (`targetUri`/`targetRange`).
pub(crate) fn parse_locations(value: &Value) -> Vec<Location> {
    match value {
        Value::Array(items) => items.iter().filter_map(parse_one_location).collect(),
        Value::Null => Vec::new(),
        other => parse_one_location(other).into_iter().collect(),
    }
}

fn parse_one_location(value: &Value) -> Option<Location> {
    // LocationLink uses targetUri/targetRange; Location uses uri/range.
    let (uri, range) = if let Some(uri) = value.get("targetUri").and_then(Value::as_str) {
        (uri, value.get("targetRange"))
    } else {
        (value.get("uri").and_then(Value::as_str)?, value.get("range"))
    };
    let (line, character) = range_start(range);
    Some(Location { uri: uri.to_string(), line, character })
}

/// Extracts the 0-based `(line, character)` of a range's start, defaulting to `(0, 0)`.
fn range_start(range: Option<&Value>) -> (u32, u32) {
    let start = range.and_then(|r| r.get("start"));
    let line = start.and_then(|s| s.get("line")).and_then(Value::as_u64).unwrap_or(0) as u32;
    let character =
        start.and_then(|s| s.get("character")).and_then(Value::as_u64).unwrap_or(0) as u32;
    (line, character)
}

/// Renders a list of locations, one per line, or an empty-result message.
pub(crate) fn format_locations(locations: &[Location], empty_label: &str) -> String {
    if locations.is_empty() {
        return format!("No {empty_label} found.");
    }
    let mut out = format!("{} result(s):\n", locations.len());
    for loc in locations {
        out.push_str(&format!("  {}\n", loc.display()));
    }
    out.trim_end().to_string()
}

/// Formats a `textDocument/hover` result, extracting markdown/plaintext from the various
/// `contents` shapes (`MarkupContent`, `MarkedString`, arrays, plain strings).
pub(crate) fn format_hover(value: &Value) -> String {
    let Some(contents) = value.get("contents") else {
        return "No hover information.".to_string();
    };
    let text = hover_contents_to_string(contents);
    if text.trim().is_empty() {
        "No hover information.".to_string()
    } else {
        text
    }
}

fn hover_contents_to_string(contents: &Value) -> String {
    match contents {
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            items.iter().map(hover_contents_to_string).collect::<Vec<_>>().join("\n")
        }
        Value::Object(map) => {
            // MarkupContent { kind, value } or MarkedString { language, value }.
            if let Some(Value::String(v)) = map.get("value") {
                v.clone()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// Formats `textDocument/documentSymbol` (hierarchical `DocumentSymbol[]` or flat
/// `SymbolInformation[]`).
pub(crate) fn format_document_symbols(value: &Value) -> String {
    let Some(items) = value.as_array() else {
        return "No symbols found.".to_string();
    };
    if items.is_empty() {
        return "No symbols found.".to_string();
    }
    let mut out = String::new();
    for item in items {
        format_symbol(item, 0, &mut out);
    }
    out.trim_end().to_string()
}

fn format_symbol(item: &Value, depth: usize, out: &mut String) {
    let name = item.get("name").and_then(Value::as_str).unwrap_or("<anonymous>");
    let kind = symbol_kind_name(item.get("kind").and_then(Value::as_u64));
    // DocumentSymbol has `range`; SymbolInformation has `location.range`.
    let range = item
        .get("range")
        .or_else(|| item.get("location").and_then(|l| l.get("range")));
    let (line, _) = range_start(range);
    out.push_str(&format!("{}{kind} {name} (line {})\n", "  ".repeat(depth), line + 1));
    if let Some(children) = item.get("children").and_then(Value::as_array) {
        for child in children {
            format_symbol(child, depth + 1, out);
        }
    }
}

/// Formats `workspace/symbol` (`SymbolInformation[]`).
pub(crate) fn format_workspace_symbols(value: &Value) -> String {
    let Some(items) = value.as_array() else {
        return "No symbols found.".to_string();
    };
    if items.is_empty() {
        return "No symbols found.".to_string();
    }
    let mut out = format!("{} symbol(s):\n", items.len());
    for item in items {
        let name = item.get("name").and_then(Value::as_str).unwrap_or("<anonymous>");
        let kind = symbol_kind_name(item.get("kind").and_then(Value::as_u64));
        let loc = item
            .get("location")
            .and_then(parse_one_location)
            .map(|l| l.display())
            .unwrap_or_default();
        out.push_str(&format!("  {kind} {name} — {loc}\n"));
    }
    out.trim_end().to_string()
}

/// Formats `textDocument/prepareCallHierarchy` (`CallHierarchyItem[]`).
pub(crate) fn format_call_hierarchy_items(value: &Value) -> String {
    let Some(items) = value.as_array() else {
        return "No call hierarchy items found.".to_string();
    };
    if items.is_empty() {
        return "No call hierarchy items found.".to_string();
    }
    let mut out = format!("{} item(s):\n", items.len());
    for item in items {
        out.push_str(&format!("  {}\n", call_hierarchy_item_line(item)));
    }
    out.trim_end().to_string()
}

/// Formats `callHierarchy/incomingCalls` (`CallHierarchyIncomingCall[]`, `from` items).
pub(crate) fn format_incoming_calls(value: &Value) -> String {
    format_calls(value, "from", "incoming call")
}

/// Formats `callHierarchy/outgoingCalls` (`CallHierarchyOutgoingCall[]`, `to` items).
pub(crate) fn format_outgoing_calls(value: &Value) -> String {
    format_calls(value, "to", "outgoing call")
}

fn format_calls(value: &Value, item_key: &str, label: &str) -> String {
    let Some(items) = value.as_array() else {
        return format!("No {label}s found.");
    };
    if items.is_empty() {
        return format!("No {label}s found.");
    }
    let mut out = format!("{} {label}(s):\n", items.len());
    for call in items {
        if let Some(item) = call.get(item_key) {
            out.push_str(&format!("  {}\n", call_hierarchy_item_line(item)));
        }
    }
    out.trim_end().to_string()
}

fn call_hierarchy_item_line(item: &Value) -> String {
    let name = item.get("name").and_then(Value::as_str).unwrap_or("<anonymous>");
    let kind = symbol_kind_name(item.get("kind").and_then(Value::as_u64));
    let uri = item.get("uri").and_then(Value::as_str).unwrap_or("");
    let (line, _) = range_start(item.get("range"));
    format!("{kind} {name} — {uri}:{}", line + 1)
}

/// Maps the LSP `SymbolKind` enum (1..=26) to a short name.
fn symbol_kind_name(kind: Option<u64>) -> &'static str {
    match kind {
        Some(1) => "File",
        Some(2) => "Module",
        Some(3) => "Namespace",
        Some(4) => "Package",
        Some(5) => "Class",
        Some(6) => "Method",
        Some(7) => "Property",
        Some(8) => "Field",
        Some(9) => "Constructor",
        Some(10) => "Enum",
        Some(11) => "Interface",
        Some(12) => "Function",
        Some(13) => "Variable",
        Some(14) => "Constant",
        Some(15) => "String",
        Some(16) => "Number",
        Some(17) => "Boolean",
        Some(18) => "Array",
        Some(19) => "Object",
        Some(20) => "Key",
        Some(21) => "Null",
        Some(22) => "EnumMember",
        Some(23) => "Struct",
        Some(24) => "Event",
        Some(25) => "Operator",
        Some(26) => "TypeParameter",
        _ => "Symbol",
    }
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod format_tests;
