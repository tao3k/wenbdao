//! Package-top harness for xml-lite unit tests.

#![cfg(feature = "zhenfa-router")]

use xiuxian_wendao::link_graph::{
    LinkGraphConfidenceLevel, LinkGraphDisplayHit, LinkGraphPlannedSearchPayload,
    LinkGraphRetrievalMode, LinkGraphSearchOptions,
};
use xiuxian_wendao::zhenfa_router::render_xml_lite_hits;

fn hit(path: &str, title: &str, doc_type: Option<&str>, tags: Vec<&str>) -> LinkGraphDisplayHit {
    LinkGraphDisplayHit {
        stem: "entry".to_string(),
        title: title.to_string(),
        path: path.to_string(),
        doc_type: doc_type.map(str::to_string),
        tags: tags.into_iter().map(str::to_string).collect(),
        score: 0.9,
        best_section: String::new(),
        match_reason: String::new(),
    }
}

fn render_hit_type(display_hit: LinkGraphDisplayHit) -> String {
    let payload = LinkGraphPlannedSearchPayload {
        query: "q".to_string(),
        options: LinkGraphSearchOptions::default(),
        hits: vec![display_hit],
        hit_count: 1,
        section_hit_count: 0,
        requested_mode: LinkGraphRetrievalMode::Hybrid,
        selected_mode: LinkGraphRetrievalMode::Hybrid,
        reason: String::new(),
        graph_hit_count: 0,
        source_hint_count: 0,
        graph_confidence_score: 0.0,
        graph_confidence_level: LinkGraphConfidenceLevel::None,
        retrieval_plan: None,
        results: Vec::new(),
        provisional_suggestions: Vec::new(),
        provisional_error: None,
        promoted_overlay: None,
    };
    render_xml_lite_hits(&payload)
}

#[test]
fn infer_hit_type_prefers_doc_type_mapping_over_tags_and_path() {
    let rendered = render_hit_type(hit(
        "agenda/today.md",
        "Today",
        Some("journal"),
        vec!["agenda"],
    ));
    assert!(rendered.contains("type=\"journal\""));
}

#[test]
fn infer_hit_type_accepts_namespaced_doc_type_values() {
    let rendered = render_hit_type(hit(
        "notes/entry.md",
        "Entry",
        Some("zhixing/agenda"),
        vec![],
    ));
    assert!(rendered.contains("type=\"agenda\""));
}

#[test]
fn infer_hit_type_prefers_tag_mapping_over_path() {
    let rendered = render_hit_type(hit("notes/entry.md", "Entry", None, vec!["journal"]));
    assert!(rendered.contains("type=\"journal\""));
}

#[test]
fn infer_hit_type_uses_path_when_metadata_is_missing() {
    let rendered = render_hit_type(hit("agenda/today.md", "Today", None, vec![]));
    assert!(rendered.contains("type=\"agenda\""));
}

#[test]
fn infer_hit_type_marks_non_markdown_as_attachment() {
    let rendered = render_hit_type(hit("assets/diagram.png", "Diagram", None, vec![]));
    assert!(rendered.contains("type=\"attachment\""));
}
