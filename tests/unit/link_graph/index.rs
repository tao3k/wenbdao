use super::*;

#[test]
fn test_symbol_cache_stats_empty() {
    let stats = SymbolCacheStats {
        unique_symbols: 0,
        total_references: 0,
    };
    assert_eq!(stats.unique_symbols, 0);
    assert_eq!(stats.total_references, 0);
}

#[test]
fn test_symbol_cache_stats_with_data() {
    let stats = SymbolCacheStats {
        unique_symbols: 10,
        total_references: 25,
    };
    assert_eq!(stats.unique_symbols, 10);
    assert_eq!(stats.total_references, 25);
}

#[test]
fn test_symbol_ref_serialization() {
    let symbol_ref = SymbolRef {
        doc_id: "docs/api".to_string(),
        node_id: "docs/api#section-1".to_string(),
        pattern: "fn process_data($$$)".to_string(),
        language: "rust".to_string(),
        line_number: Some(42),
        scope: Some("src/api/**".to_string()),
    };

    let Ok(json) = serde_json::to_string(&symbol_ref) else {
        panic!("symbol reference serialization should succeed");
    };
    assert!(json.contains("process_data"));
    assert!(json.contains("rust"));
    assert!(json.contains("src/api"));

    let Ok(deserialized) = serde_json::from_str::<SymbolRef>(&json) else {
        panic!("symbol reference deserialization should succeed");
    };
    assert_eq!(deserialized.doc_id, "docs/api");
    assert_eq!(deserialized.line_number, Some(42));
    assert_eq!(deserialized.scope, Some("src/api/**".to_string()));
}

#[test]
fn test_symbol_ref_serialization_no_scope() {
    let symbol_ref = SymbolRef {
        doc_id: "docs/api".to_string(),
        node_id: "docs/api#section-1".to_string(),
        pattern: "fn process_data($$$)".to_string(),
        language: "rust".to_string(),
        line_number: Some(42),
        scope: None,
    };

    let Ok(json) = serde_json::to_string(&symbol_ref) else {
        panic!("symbol reference serialization should succeed");
    };
    let Ok(deserialized) = serde_json::from_str::<SymbolRef>(&json) else {
        panic!("symbol reference deserialization should succeed");
    };
    assert!(deserialized.scope.is_none());
}
