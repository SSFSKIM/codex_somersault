use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn parses_location_array_and_location_link() {
    let single = json!({
        "uri": "file:///a.rs",
        "range": {"start": {"line": 4, "character": 2}, "end": {"line": 4, "character": 6}}
    });
    assert_eq!(
        parse_locations(&single),
        vec![Location {
            uri: "file:///a.rs".into(),
            line: 4,
            character: 2
        }]
    );

    let array = json!([
        single,
        {"targetUri": "file:///b.rs", "targetRange": {"start": {"line": 0, "character": 0}}}
    ]);
    let locs = parse_locations(&array);
    assert_eq!(locs.len(), 2);
    assert_eq!(locs[1].uri, "file:///b.rs");

    assert!(parse_locations(&json!(null)).is_empty());
}

#[test]
fn formats_locations_one_based() {
    let locs = vec![Location {
        uri: "file:///a.rs".into(),
        line: 4,
        character: 2,
    }];
    // 0-based (4, 2) renders 1-based (5, 3).
    assert!(format_locations(&locs, "definition").contains("file:///a.rs:5:3"));
    assert_eq!(format_locations(&[], "reference"), "No reference found.");
}

#[test]
fn formats_hover_variants() {
    assert_eq!(
        format_hover(&json!({"contents": {"kind": "markdown", "value": "`fn foo()`"}})),
        "`fn foo()`"
    );
    assert_eq!(format_hover(&json!({"contents": "plain"})), "plain");
    assert_eq!(
        format_hover(&json!({"contents": ["a", {"value": "b"}]})),
        "a\nb"
    );
    assert_eq!(format_hover(&json!({})), "No hover information.");
}

#[test]
fn formats_document_symbols_hierarchically() {
    let v = json!([{
        "name": "Foo",
        "kind": 5,
        "range": {"start": {"line": 0, "character": 0}},
        "children": [{
            "name": "bar",
            "kind": 6,
            "range": {"start": {"line": 2, "character": 4}}
        }]
    }]);
    let out = format_document_symbols(&v);
    assert!(out.contains("Class Foo (line 1)"), "{out}");
    assert!(out.contains("Method bar (line 3)"), "{out}");
}

#[test]
fn formats_incoming_calls() {
    let v = json!([{
        "from": {
            "name": "caller",
            "kind": 12,
            "uri": "file:///c.rs",
            "range": {"start": {"line": 9, "character": 0}}
        }
    }]);
    let out = format_incoming_calls(&v);
    assert!(out.contains("Function caller"), "{out}");
    assert!(out.contains("file:///c.rs:10"), "{out}");
}
