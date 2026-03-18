//! Unit tests for `zhenfa_router/rpc` module.

use super::*;

use crate::link_graph::{LinkGraphConfidenceLevel, LinkGraphRetrievalMode};

#[test]
fn normalize_limit_clamps_range() {
    assert_eq!(normalize_limit(None), DEFAULT_SEARCH_LIMIT);
    assert_eq!(normalize_limit(Some(0)), 1);
    assert_eq!(normalize_limit(Some(3)), 3);
    assert_eq!(
        normalize_limit(Some(MAX_SEARCH_LIMIT + 10)),
        MAX_SEARCH_LIMIT
    );
}

#[test]
fn render_markdown_includes_hits() {
    let payload = LinkGraphPlannedSearchPayload {
        query: "router".to_string(),
        options: crate::link_graph::LinkGraphSearchOptions::default(),
        hits: vec![crate::link_graph::LinkGraphDisplayHit {
            stem: "alpha".to_string(),
            title: "Alpha Note".to_string(),
            path: "notes/alpha.md".to_string(),
            doc_type: None,
            tags: Vec::new(),
            score: 0.9,
            best_section: "Design".to_string(),
            match_reason: String::new(),
        }],
        hit_count: 1,
        section_hit_count: 1,
        requested_mode: LinkGraphRetrievalMode::Hybrid,
        selected_mode: LinkGraphRetrievalMode::Hybrid,
        reason: "graph_sufficient".to_string(),
        graph_hit_count: 1,
        source_hint_count: 1,
        graph_confidence_score: 0.9,
        graph_confidence_level: LinkGraphConfidenceLevel::High,
        retrieval_plan: None,
        results: vec![],
        provisional_suggestions: vec![],
        provisional_error: None,
        promoted_overlay: None,
        ccs_audit: None,
    };

    let rendered = render_markdown(&payload);
    assert!(rendered.contains("Wendao Search Results"));
    assert!(rendered.contains("Alpha Note"));
    assert!(rendered.contains("section: Design"));
}
