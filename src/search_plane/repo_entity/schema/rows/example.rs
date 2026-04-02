use xiuxian_vector::VectorStoreError;

use crate::analyzers::ExampleRecord;
use crate::analyzers::service::{
    backlinks_for, hierarchy_segments_from_path, projection_pages_for, record_hierarchical_uri,
    related_modules_for_example, related_symbols_for_example,
};
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::repo_entity::schema::definitions::{ENTITY_KIND_EXAMPLE, RepoEntityRow};
use crate::search_plane::repo_entity::schema::helpers::{
    infer_code_language, map_backlink_items, repo_entity_tags, repo_navigation_target,
    serialize_backlink_items_json, serialize_hit_json,
};
use crate::search_plane::repo_entity::schema::rows::RepoEntityContext;

pub(crate) fn build_example_row(
    context: &RepoEntityContext<'_>,
    example: &ExampleRecord,
) -> Result<RepoEntityRow, VectorStoreError> {
    let example_id = example.example_id.clone();
    let path = example.path.clone();
    let language = infer_code_language(path.as_str());
    let summary = example.summary.clone().unwrap_or_default();
    let hierarchy = hierarchy_segments_from_path(path.as_str());
    let related_symbols = related_symbols_for_example(
        example_id.as_str(),
        &context.example_relations,
        &context.analysis.symbols,
    );
    let related_modules = related_modules_for_example(
        example_id.as_str(),
        &context.example_relations,
        &context.analysis.modules,
    );
    let related_symbols_text = related_symbols.join(" ");
    let related_modules_text = related_modules.join(" ");
    let (implicit_backlinks, implicit_backlink_items) =
        backlinks_for(example_id.as_str(), &context.backlink_lookup);
    let saliency_score = context
        .saliency_map
        .get(example_id.as_str())
        .copied()
        .unwrap_or(0.0);
    let projection_page_ids =
        projection_pages_for(example_id.as_str(), &context.projection_lookup).unwrap_or_default();
    let hierarchical_uri = record_hierarchical_uri(
        context.repo_id,
        context.ecosystem,
        "examples",
        path.as_str(),
        example_id.as_str(),
    );
    let hit = SearchHit {
        stem: example.title.clone(),
        title: Some(example.title.clone()),
        path: path.clone(),
        doc_type: Some(ENTITY_KIND_EXAMPLE.to_string()),
        tags: repo_entity_tags(
            context.repo_id,
            ENTITY_KIND_EXAMPLE,
            language.clone(),
            Some("example"),
            None,
        ),
        score: saliency_score,
        best_section: example.summary.clone(),
        match_reason: Some("repo_example_search".to_string()),
        hierarchical_uri: Some(hierarchical_uri.clone()),
        hierarchy: hierarchy.clone(),
        saliency_score: Some(saliency_score),
        audit_status: None,
        verification_state: None,
        implicit_backlinks,
        implicit_backlink_items: map_backlink_items(implicit_backlink_items),
        navigation_target: Some(repo_navigation_target(
            context.repo_id,
            path.as_str(),
            Some(1),
            None,
        )),
    };
    Ok(RepoEntityRow {
        id: example_id,
        entity_kind: ENTITY_KIND_EXAMPLE.to_string(),
        name: example.title.clone(),
        name_folded: example.title.to_ascii_lowercase(),
        qualified_name: example.title.clone(),
        qualified_name_folded: example.title.to_ascii_lowercase(),
        path: path.clone(),
        path_folded: path.to_ascii_lowercase(),
        language: language.unwrap_or_default(),
        symbol_kind: "example".to_string(),
        module_id: None,
        signature: None,
        signature_folded: String::new(),
        summary: example.summary.clone(),
        summary_folded: summary.to_ascii_lowercase(),
        related_symbols_folded: related_symbols.join("\n"),
        related_modules_folded: related_modules.join("\n"),
        line_start: Some(1),
        line_end: None,
        audit_status: None,
        verification_state: None,
        attributes_json: None,
        hierarchical_uri: Some(hierarchical_uri),
        hierarchy: hierarchy.clone().unwrap_or_default(),
        implicit_backlinks: hit.implicit_backlinks.clone().unwrap_or_default(),
        implicit_backlink_items_json: serialize_backlink_items_json(
            hit.implicit_backlink_items.as_ref(),
        )?,
        projection_page_ids,
        saliency_score,
        search_text: [
            example.title.as_str(),
            summary.as_str(),
            related_symbols_text.as_str(),
            related_modules_text.as_str(),
            path.as_str(),
        ]
        .join(" "),
        hit_json: serialize_hit_json(&hit)?,
    })
}
