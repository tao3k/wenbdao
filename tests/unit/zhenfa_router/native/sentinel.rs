use super::*;

#[test]
fn test_semantic_drift_signal_summary() {
    let mut signal = SemanticDriftSignal::new("src/lib.rs", "lib");
    signal.add_affected_doc(AffectedDoc::new(
        "docs/api",
        "fn lib_init($$$)",
        "rust",
        "node-1",
    ));
    signal.update_confidence(DriftConfidence::High);

    let summary = signal.summary();
    assert!(summary.contains("lib"));
    assert!(summary.contains("docs/api"));
}

#[test]
fn test_semantic_drift_signal_serialization() {
    let mut signal = SemanticDriftSignal::new("src/lib.rs", "lib");
    signal.add_affected_doc(AffectedDoc::new(
        "docs/api",
        "fn lib_init($$$)",
        "rust",
        "node-1",
    ));

    let json = signal.to_streaming_payload();
    assert!(json.contains("lib"));
    assert!(json.contains("docs/api"));
}

#[test]
fn test_drift_confidence_levels() {
    assert_eq!(DriftConfidence::High, DriftConfidence::High);
    assert_ne!(DriftConfidence::High, DriftConfidence::Low);
}

#[test]
fn test_affected_doc_builder() {
    let doc = AffectedDoc::new("docs/test", "pattern", "rust", "node-1").with_line(42);

    assert_eq!(doc.doc_id, "docs/test");
    assert_eq!(doc.matching_pattern, "pattern");
    assert_eq!(doc.language, "rust");
    assert_eq!(doc.line_number, Some(42));
    assert_eq!(doc.node_id, "node-1");
}

#[test]
fn test_is_source_code() {
    assert!(is_source_code(Path::new("src/lib.rs")));
    assert!(is_source_code(Path::new("app/main.py")));
    assert!(is_source_code(Path::new("ui/index.ts")));
    assert!(is_source_code(Path::new("web/app.js")));
    assert!(!is_source_code(Path::new("docs/README.md")));
    assert!(!is_source_code(Path::new("config.toml")));
}

#[test]
fn test_is_ignorable_path() {
    assert!(is_ignorable_path(Path::new(".git/config")));
    assert!(is_ignorable_path(Path::new("target/debug/app")));
    assert!(!is_ignorable_path(Path::new("src/lib.rs")));
}

// =========================================================================
// ObservationSignal Tests
// =========================================================================

#[test]
fn test_observation_signal_stale_from_drift() {
    let mut drift = SemanticDriftSignal::new("src/lib.rs", "lib");
    drift.add_affected_doc(AffectedDoc::new(
        "docs/api",
        "fn lib_init($$$)",
        "rust",
        "node-1",
    ));
    drift.update_confidence(DriftConfidence::High);

    let signals = ObservationSignal::stale_from_drift(&drift);
    assert_eq!(signals.len(), 1);

    match &signals[0] {
        ObservationSignal::Stale {
            doc_id,
            observation,
            trigger_source,
            confidence,
        } => {
            assert_eq!(doc_id, "docs/api");
            assert_eq!(observation.pattern, "fn lib_init($$$)");
            assert_eq!(observation.language, "rust");
            assert_eq!(*trigger_source, "src/lib.rs");
            assert_eq!(*confidence, DriftConfidence::High);
        }
        _ => panic!("Expected Stale signal"),
    }
}

#[test]
fn test_observation_signal_to_status_message() {
    let signal = ObservationSignal::Stale {
        doc_id: "docs/api".to_string(),
        observation: ObservationRef {
            pattern: "fn test()".to_string(),
            language: "rust".to_string(),
            line_number: 42,
            node_id: "node-1".to_string(),
        },
        trigger_source: "src/lib.rs".to_string(),
        confidence: DriftConfidence::High,
    };

    let msg = signal.to_status_message();
    assert!(msg.contains("Stale"));
    assert!(msg.contains("docs/api"));
    assert!(msg.contains("fn test()"));
    assert!(msg.contains("High"));
}

#[test]
fn test_observation_signal_requires_attention() {
    let high_stale = ObservationSignal::Stale {
        doc_id: "docs/api".to_string(),
        observation: ObservationRef {
            pattern: "fn test()".to_string(),
            language: "rust".to_string(),
            line_number: 1,
            node_id: "n1".to_string(),
        },
        trigger_source: "src/lib.rs".to_string(),
        confidence: DriftConfidence::High,
    };
    assert!(high_stale.requires_attention());

    let low_stale = ObservationSignal::Stale {
        doc_id: "docs/api".to_string(),
        observation: ObservationRef {
            pattern: "fn test()".to_string(),
            language: "rust".to_string(),
            line_number: 1,
            node_id: "n1".to_string(),
        },
        trigger_source: "src/lib.rs".to_string(),
        confidence: DriftConfidence::Low,
    };
    assert!(!low_stale.requires_attention());

    let broken = ObservationSignal::Broken {
        doc_id: "docs/api".to_string(),
        observation: ObservationRef {
            pattern: "fn test()".to_string(),
            language: "rust".to_string(),
            line_number: 1,
            node_id: "n1".to_string(),
        },
        error: "Pattern not found".to_string(),
    };
    assert!(broken.requires_attention());
}

#[test]
fn test_observation_bus_emit() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut bus = ObservationBus::new();
    assert!(!bus.is_connected());

    bus.connect(tx);
    assert!(bus.is_connected());

    let signal = ObservationSignal::Stale {
        doc_id: "docs/api".to_string(),
        observation: ObservationRef {
            pattern: "fn test()".to_string(),
            language: "rust".to_string(),
            line_number: 1,
            node_id: "n1".to_string(),
        },
        trigger_source: "src/lib.rs".to_string(),
        confidence: DriftConfidence::High,
    };

    let id = bus.emit(signal);
    assert!(id.is_some());

    let received = rx.try_recv();
    assert!(received.is_ok());
}

#[test]
fn test_observation_bus_emit_drift_signals() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let mut bus = ObservationBus::new();
    bus.connect(tx);

    let mut drift = SemanticDriftSignal::new("src/lib.rs", "lib");
    drift.add_affected_doc(AffectedDoc::new("docs/a", "p1", "rust", "n1"));
    drift.add_affected_doc(AffectedDoc::new("docs/b", "p2", "rust", "n2"));

    let ids = bus.emit_drift_signals(&drift);
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_signals_to_status_batch() {
    let signals = vec![
        ObservationSignal::Stale {
            doc_id: "docs/a".to_string(),
            observation: ObservationRef {
                pattern: "fn a()".to_string(),
                language: "rust".to_string(),
                line_number: 1,
                node_id: "n1".to_string(),
            },
            trigger_source: "src/a.rs".to_string(),
            confidence: DriftConfidence::High,
        },
        ObservationSignal::Broken {
            doc_id: "docs/b".to_string(),
            observation: ObservationRef {
                pattern: "fn b()".to_string(),
                language: "rust".to_string(),
                line_number: 2,
                node_id: "n2".to_string(),
            },
            error: "Not found".to_string(),
        },
    ];

    let batch = signals_to_status_batch(&signals);
    assert!(batch.contains("Observation Signal Batch"));
    assert!(batch.contains("2 signal(s)"));
    assert!(batch.contains("2 require immediate attention"));
}

// =========================================================================
// Audit Recommendation Function Tests
// =========================================================================

#[test]
fn test_is_high_noise_file() {
    // High noise files
    assert!(is_high_noise_file(Path::new("src/mod.rs")));
    assert!(is_high_noise_file(Path::new("src/lib.rs")));
    assert!(is_high_noise_file(Path::new("bin/main.rs")));
    assert!(is_high_noise_file(Path::new("prelude.rs")));
    assert!(is_high_noise_file(Path::new("types.rs")));
    assert!(is_high_noise_file(Path::new("error.rs")));
    assert!(is_high_noise_file(Path::new("utils.rs")));

    // Regular source files
    assert!(!is_high_noise_file(Path::new("src/parser.rs")));
    assert!(!is_high_noise_file(Path::new("src/sentinel.rs")));
    assert!(!is_high_noise_file(Path::new("app/models/user.rs")));
}

#[test]
fn test_extract_pattern_symbols_function() {
    let symbols = extract_pattern_symbols("fn process_data($$$)");
    assert_eq!(symbols, vec!["process_data"]);

    let symbols = extract_pattern_symbols("async fn fetch_user(id: u32) -> Result<User, Error>");
    assert!(symbols.contains(&"fetch_user".to_string()));
}

#[test]
fn test_extract_pattern_symbols_struct() {
    let symbols = extract_pattern_symbols("struct User { $$$ }");
    assert_eq!(symbols, vec!["User"]);

    let symbols = extract_pattern_symbols("struct HttpRequest { method: String, path: String }");
    assert!(symbols.contains(&"HttpRequest".to_string()));
}

#[test]
fn test_extract_pattern_symbols_class() {
    let symbols = extract_pattern_symbols("class UserProfile { $$$ }");
    assert_eq!(symbols, vec!["UserProfile"]);
}

#[test]
fn test_extract_pattern_symbols_enum() {
    let symbols = extract_pattern_symbols("enum Status { $$$ }");
    assert_eq!(symbols, vec!["Status"]);
}

#[test]
fn test_extract_pattern_symbols_trait() {
    let symbols = extract_pattern_symbols("trait Handler { $$$ }");
    assert_eq!(symbols, vec!["Handler"]);
}

#[test]
fn test_extract_pattern_symbols_impl() {
    let symbols = extract_pattern_symbols("impl User { $$$ }");
    assert!(symbols.contains(&"User".to_string()));

    let symbols = extract_pattern_symbols("impl Display for User { $$$ }");
    assert!(symbols.contains(&"User".to_string()));
}

#[test]
fn test_extract_pattern_symbols_multiple() {
    // Pattern with function name - note: return types not currently extracted
    let symbols = extract_pattern_symbols("fn create_user() -> User { $$$ }");
    assert!(symbols.contains(&"create_user".to_string()));
    // Note: 'User' in return type is not extracted - only explicit struct/enum/class keywords

    // Pattern with explicit struct and function
    let symbols = extract_pattern_symbols("struct User { } fn create_user() { $$$ }");
    assert!(symbols.contains(&"User".to_string()));
    assert!(symbols.contains(&"create_user".to_string()));
}

#[test]
fn test_extract_pattern_symbols_empty() {
    let symbols = extract_pattern_symbols("$$$");
    assert!(symbols.is_empty());

    let symbols = extract_pattern_symbols("// just a comment");
    assert!(symbols.is_empty());
}

#[test]
fn test_verify_file_stable_with_temp_file() {
    use std::io::Write;

    // Create a temp file with content
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("xiuxian_test_stable.rs");

    let Ok(mut file) = std::fs::File::create(&temp_path) else {
        panic!("failed to create temporary sentinel stability file");
    };
    assert!(file.write_all(b"fn main() {}").is_ok());
    drop(file);

    assert!(verify_file_stable(&temp_path));

    // Cleanup
    std::fs::remove_file(&temp_path).ok();
}

#[test]
fn test_verify_file_stable_nonexistent() {
    assert!(!verify_file_stable(Path::new("/nonexistent/file.rs")));
}

#[test]
fn test_compute_file_hash_with_temp_file() {
    use std::io::Write;

    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("xiuxian_test_hash.txt");

    let Ok(mut file) = std::fs::File::create(&temp_path) else {
        panic!("failed to create temporary sentinel hash file");
    };
    assert!(file.write_all(b"test content for hashing").is_ok());
    drop(file);

    let hash = compute_file_hash(&temp_path);
    assert!(hash.is_some());
    let Some(hash) = hash else {
        panic!("hash should exist for the temporary file");
    };
    assert_eq!(hash.len(), 64); // Blake3 hex length

    // Same content should produce same hash
    let Some(hash2) = compute_file_hash(&temp_path) else {
        panic!("hash should exist for the same temporary file");
    };
    assert_eq!(hash, hash2);

    // Cleanup
    std::fs::remove_file(&temp_path).ok();
}

#[test]
fn test_compute_file_hash_nonexistent() {
    let hash = compute_file_hash(Path::new("/nonexistent/file.rs"));
    assert!(hash.is_none());
}

// =========================================================================
// Symbol Cache Helper Tests
// =========================================================================

#[test]
fn test_to_pascal_case() {
    assert_eq!(to_pascal_case("user_handler"), "UserHandler");
    assert_eq!(to_pascal_case("process_data"), "ProcessData");
    assert_eq!(to_pascal_case("single"), "Single");
    assert_eq!(to_pascal_case(""), "");
    assert_eq!(to_pascal_case("a_b_c"), "ABC");
}

// =========================================================================
// DriftConfidence Ordering Tests (Phase 7)
// =========================================================================

#[test]
fn test_drift_confidence_ordering() {
    // Verify ordering: Low < Medium < High
    assert!(DriftConfidence::Low < DriftConfidence::Medium);
    assert!(DriftConfidence::Medium < DriftConfidence::High);
    assert!(DriftConfidence::Low < DriftConfidence::High);

    // Verify reverse
    assert!(DriftConfidence::High > DriftConfidence::Medium);
    assert!(DriftConfidence::Medium > DriftConfidence::Low);

    // Verify equality
    assert_eq!(DriftConfidence::Low, DriftConfidence::Low);
    assert_eq!(DriftConfidence::Medium, DriftConfidence::Medium);
    assert_eq!(DriftConfidence::High, DriftConfidence::High);

    // Verify >= and <=
    assert!(DriftConfidence::Medium >= DriftConfidence::Low);
    assert!(DriftConfidence::High >= DriftConfidence::Medium);
    assert!(DriftConfidence::High >= DriftConfidence::High);
}

#[test]
fn test_drift_confidence_threshold_filtering() {
    // Simulate ForwardNotifier threshold filtering logic
    let threshold = DriftConfidence::Medium;

    // Low confidence should be filtered out
    assert!(DriftConfidence::Low < threshold);

    // Medium and High should pass
    assert!(DriftConfidence::Medium >= threshold);
    assert!(DriftConfidence::High >= threshold);
}

#[test]
fn test_drift_confidence_auto_fix_threshold() {
    // Simulate auto-fix threshold (typically higher than notification)
    let auto_fix_threshold = DriftConfidence::High;

    // Only High confidence should trigger auto-fix
    assert!(DriftConfidence::Low < auto_fix_threshold);
    assert!(DriftConfidence::Medium < auto_fix_threshold);
    assert!(DriftConfidence::High >= auto_fix_threshold);
}

// =========================================================================
// Phase 7.6: Scope Filtering Tests
// =========================================================================

#[test]
fn test_matches_scope_filter_no_scope() {
    // No scope should match all files
    assert!(matches_scope_filter("src/api/handler.rs", None));
    assert!(matches_scope_filter("any/path/file.rs", None));
}

#[test]
fn test_matches_scope_filter_with_scope() {
    // Scope should match only matching paths
    assert!(matches_scope_filter(
        "src/api/handler.rs",
        Some("src/api/**")
    ));
    assert!(!matches_scope_filter(
        "src/db/handler.rs",
        Some("src/api/**")
    ));
}

#[test]
fn test_matches_scope_filter_double_star() {
    // ** should match any depth
    assert!(matches_scope_filter(
        "deep/nested/path/file.rs",
        Some("**/*.rs")
    ));
    assert!(matches_scope_filter("lib.rs", Some("**/*.rs")));
    assert!(!matches_scope_filter("lib.py", Some("**/*.rs")));
}

#[test]
fn test_matches_scope_filter_package_specific() {
    // Package-specific scope
    assert!(matches_scope_filter(
        "packages/core/src/lib.rs",
        Some("packages/core/**")
    ));
    assert!(!matches_scope_filter(
        "packages/api/src/lib.rs",
        Some("packages/core/**")
    ));
}
