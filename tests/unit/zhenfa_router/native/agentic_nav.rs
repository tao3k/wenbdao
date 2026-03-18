//! Unit tests for `agentic_nav` module.

use super::*;

#[test]
fn test_xml_escape() {
    assert_eq!(xml_escape("a<b>c&d"), "a&lt;b&gt;c&amp;d");
    assert_eq!(xml_escape("\"quoted\""), "&quot;quoted&quot;");
    assert_eq!(xml_escape("'single'"), "&apos;single&apos;");
    assert_eq!(xml_escape("normal text"), "normal text");
}

#[test]
fn test_generate_navigation_hint_invalid_anchor() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Invalid anchor should return orphaned hint
    let hit = SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#missing".to_string(),
            vector_score: 0.8,
        },
        is_valid: false,
        doc_id: "doc.md".to_string(),
        anchor: "missing".to_string(),
        structural_path: None,
        reranked_score: 0.3,
    };

    let navigation_hint = generate_navigation_hint(&hit);
    assert!(
        navigation_hint.contains("Orphaned anchor"),
        "Expected orphaned hint for invalid anchor, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("content may have changed"),
        "Hint should mention content change, got: {navigation_hint}"
    );
}

#[test]
fn test_generate_navigation_hint_root_level() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Root level (depth 0) should return overview hint
    let hit = SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#root".to_string(),
            vector_score: 0.9,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "root".to_string(),
        structural_path: Some(vec![]), // Empty path = root
        reranked_score: 0.95,
    };

    let navigation_hint = generate_navigation_hint(&hit);
    assert!(
        navigation_hint.contains("Root-level"),
        "Expected root-level hint, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("high-level overview"),
        "Hint should mention overview, got: {navigation_hint}"
    );
}

#[test]
fn test_generate_navigation_hint_top_level() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Top level (depth 1) should return entry point hint
    let hit = SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#intro".to_string(),
            vector_score: 0.9,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "intro".to_string(),
        structural_path: Some(vec!["Introduction".to_string()]),
        reranked_score: 0.95,
    };

    let navigation_hint = generate_navigation_hint(&hit);
    assert!(
        navigation_hint.contains("Top-level section"),
        "Expected top-level hint, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("good entry point"),
        "Hint should mention entry point, got: {navigation_hint}"
    );
}

#[test]
fn test_generate_navigation_hint_nested_moderate() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Nested section at depth 2 should return implementation details hint
    let hit = SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#storage".to_string(),
            vector_score: 0.85,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "storage".to_string(),
        structural_path: Some(vec!["Architecture".to_string(), "Storage".to_string()]),
        reranked_score: 0.90,
    };

    let navigation_hint = generate_navigation_hint(&hit);
    assert!(
        navigation_hint.contains("Nested section"),
        "Expected nested hint, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("depth 2"),
        "Hint should mention depth, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("implementation details"),
        "Hint should mention details, got: {navigation_hint}"
    );
}

#[test]
fn test_generate_navigation_hint_nested_deep() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Deeply nested section (depth 3) should return specific details hint
    let hit = SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#engine".to_string(),
            vector_score: 0.8,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "engine".to_string(),
        structural_path: Some(vec![
            "Architecture".to_string(),
            "Engine".to_string(),
            "Core".to_string(),
        ]),
        reranked_score: 0.85,
    };

    let navigation_hint = generate_navigation_hint(&hit);
    assert!(
        navigation_hint.contains("depth 3"),
        "Hint should mention depth 3, got: {navigation_hint}"
    );
}

#[test]
fn test_generate_navigation_hint_deeply_nested() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Very deeply nested section (depth 4+) should return highly specific hint
    let hit = SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#deep".to_string(),
            vector_score: 0.75,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "deep".to_string(),
        structural_path: Some(vec![
            "Level1".to_string(),
            "Level2".to_string(),
            "Level3".to_string(),
            "Level4".to_string(),
            "Level5".to_string(),
        ]),
        reranked_score: 0.80,
    };

    let navigation_hint = generate_navigation_hint(&hit);
    assert!(
        navigation_hint.contains("Deeply nested"),
        "Expected deeply nested hint, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("highly specific"),
        "Hint should mention specificity, got: {navigation_hint}"
    );
    assert!(
        navigation_hint.contains("parent context"),
        "Hint should mention context, got: {navigation_hint}"
    );
}

#[test]
fn test_render_agentic_nav_result_basic() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    let validated = vec![SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#intro".to_string(),
            vector_score: 0.9,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "intro".to_string(),
        structural_path: Some(vec!["Introduction".to_string()]),
        reranked_score: 0.95,
    }];

    let xml = render_agentic_nav_result("test query", &validated, 10);

    assert!(xml.contains("<query>test query</query>"));
    assert!(xml.contains("<anchor_id>doc.md#intro</anchor_id>"));
    assert!(xml.contains("<is_valid>true</is_valid>"));
    assert!(xml.contains("<score>0.9500</score>"));
    assert!(xml.contains("<total_found>1</total_found>"));
}

#[test]
fn test_render_agentic_nav_result_with_navigation_hint() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    let validated = vec![SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#intro".to_string(),
            vector_score: 0.9,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "intro".to_string(),
        structural_path: Some(vec!["Introduction".to_string()]),
        reranked_score: 0.95,
    }];

    let xml = render_agentic_nav_result("test query", &validated, 10);

    // Verify navigation_hint element is present
    assert!(
        xml.contains("<navigation_hint>"),
        "XML should contain navigation_hint element"
    );
    assert!(
        xml.contains("</navigation_hint>"),
        "XML should contain closing navigation_hint tag"
    );
    // For depth 1, should mention entry point
    assert!(
        xml.contains("entry point"),
        "XML should contain entry point hint for depth 1, got: {xml}"
    );
}

#[test]
fn test_render_agentic_nav_result_with_invalid_anchor_hint() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    let validated = vec![SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#missing".to_string(),
            vector_score: 0.8,
        },
        is_valid: false,
        doc_id: "doc.md".to_string(),
        anchor: "missing".to_string(),
        structural_path: None,
        reranked_score: 0.3,
    }];

    let xml = render_agentic_nav_result("test query", &validated, 10);

    // Verify navigation_hint contains orphaned message
    assert!(
        xml.contains("<navigation_hint>"),
        "XML should contain navigation_hint element"
    );
    assert!(
        xml.contains("Orphaned anchor"),
        "XML should contain orphaned anchor hint, got: {xml}"
    );
}

#[test]
fn test_render_agentic_nav_result_with_structural_path() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    let validated = vec![SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#storage".to_string(),
            vector_score: 0.85,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "storage".to_string(),
        structural_path: Some(vec!["Architecture".to_string(), "Storage".to_string()]),
        reranked_score: 0.90,
    }];

    let xml = render_agentic_nav_result("storage systems", &validated, 10);

    // Verify structural_path is rendered
    assert!(
        xml.contains("<structural_path>"),
        "XML should contain structural_path element"
    );
    assert!(
        xml.contains("<segment>Architecture</segment>"),
        "XML should contain Architecture segment, got: {xml}"
    );
    assert!(
        xml.contains("<segment>Storage</segment>"),
        "XML should contain Storage segment, got: {xml}"
    );
}

#[test]
fn test_render_agentic_nav_result_limit() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    // Create 5 hits but limit to 2
    let validated: Vec<SkeletonValidatedHit> = (0..5)
        .map(|i| SkeletonValidatedHit {
            hit: QuantumAnchorHit {
                anchor_id: format!("doc.md#section{i}"),
                vector_score: 0.9 - (f64::from(i) * 0.1),
            },
            is_valid: true,
            doc_id: "doc.md".to_string(),
            anchor: format!("section{i}"),
            structural_path: Some(vec![format!("Section {}", i)]),
            reranked_score: 0.95 - (f64::from(i) * 0.1),
        })
        .collect();

    let xml = render_agentic_nav_result("test query", &validated, 2);

    // Should contain exactly 2 candidates
    assert!(
        xml.contains("<total_found>5</total_found>"),
        "XML should report 5 total found"
    );
    // Count candidate tags - should be 2
    let candidate_count = xml.matches("<candidate>").count();
    assert_eq!(
        candidate_count, 2,
        "Should have exactly 2 candidates, got {candidate_count}"
    );
}

#[test]
fn test_render_agentic_nav_result_xml_escapes_query() {
    use crate::link_graph::addressing::SkeletonValidatedHit;

    let validated = vec![SkeletonValidatedHit {
        hit: QuantumAnchorHit {
            anchor_id: "doc.md#test".to_string(),
            vector_score: 0.9,
        },
        is_valid: true,
        doc_id: "doc.md".to_string(),
        anchor: "test".to_string(),
        structural_path: Some(vec!["Test".to_string()]),
        reranked_score: 0.95,
    }];

    // Query with special characters
    let xml = render_agentic_nav_result("test <query> & \"data\"", &validated, 10);

    // Should be properly escaped
    assert!(
        xml.contains("&lt;query&gt;"),
        "XML should escape angle brackets"
    );
    assert!(xml.contains("&amp;"), "XML should escape ampersand");
    assert!(xml.contains("&quot;"), "XML should escape quotes");
}
