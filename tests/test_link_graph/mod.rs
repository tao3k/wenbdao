//! Integration tests for `LinkGraph` parsing, retrieval, and cache behaviors.

use redis::Connection;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use xiuxian_wendao::link_graph::{
    LinkGraphAttachmentKind, LinkGraphDirection, LinkGraphEdgeType, LinkGraphIndex,
    LinkGraphLinkFilter, LinkGraphMatchStrategy, LinkGraphPprSubgraphMode, LinkGraphRelatedFilter,
    LinkGraphRelatedPprOptions, LinkGraphScope, LinkGraphSearchFilters, LinkGraphSearchOptions,
    LinkGraphSemanticDocumentScope, LinkGraphSortField, LinkGraphSortOrder, LinkGraphSortTerm,
    parse_search_query,
};
use xiuxian_wendao::{
    LinkGraphSaliencyPolicy, compute_link_graph_saliency, valkey_saliency_get_with_valkey,
};

use build_scope_fixture_support::{
    BuildScopeFixture, assert_build_scope_fixture, docs_snapshot, stats_and_toc_snapshot,
};
use cache_build_fixture_support::{
    CacheBuildFixture, assert_cache_build_fixture, cache_hits_snapshot, cache_stats_snapshot,
    read_cache_build_fixture, saliency_state_snapshot,
};
use graph_navigation_fixture_support::{
    NavigationFixture, assert_graph_navigation_fixture, navigation_surface_snapshot,
    related_diagnostics_snapshot,
};
use markdown_attachments_fixture_support::{
    AttachmentFixture, assert_markdown_attachment_fixture, attachment_hits_snapshot,
    stats_and_neighbors_snapshot,
};
use page_index_fixture_support::{
    PageIndexFixture, assert_page_index_fixture, page_index_tree_snapshot, read_page_index_fixture,
    semantic_documents_snapshot,
};
use quantum_fixture_support::{
    assert_quantum_fixture, build_hybrid_fixture, contexts_snapshot,
    default_quantum_fusion_options, page_index_fallback_snapshot,
};
use refresh_fixture_support::{
    RefreshFixture, assert_refresh_fixture, read_refresh_fixture, refresh_hits_snapshot,
    refresh_mode_label, refresh_sequence_snapshot, stats_snapshot,
};
use search_core_fixture_support::{
    SearchCoreFixture, assert_search_core_fixture, direct_id_snapshot, hits_snapshot,
    planned_payload_snapshot, stats_and_hits_snapshot,
};
use search_filters_fixture_support::{
    SearchFilterFixture, assert_search_filter_fixture, ordered_hit_paths,
};
use search_match_fixture_support::{
    SearchMatchFixture, assert_search_match_fixture, hits_outline_snapshot, parsed_query_snapshot,
};
use semantic_policy_fixture_support::{
    SemanticPolicyFixture, assert_semantic_policy_fixture, parsed_semantic_policy_snapshot,
    planned_payload_semantic_policy_snapshot,
};
use tree_scope_fixture_support::{
    TreeScopeFixture, assert_tree_scope_fixture, ordered_section_labels, per_path_counts_snapshot,
    tree_hit_outline_snapshot,
};

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn sort_term(field: LinkGraphSortField, order: LinkGraphSortOrder) -> LinkGraphSortTerm {
    LinkGraphSortTerm { field, order }
}

fn valkey_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    let client = redis::Client::open("redis://127.0.0.1:6379/0")?;
    let conn = client.get_connection()?;
    Ok(conn)
}

fn clear_cache_keys(prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    if !keys.is_empty() {
        redis::cmd("DEL").arg(keys).query::<()>(&mut conn)?;
    }
    Ok(())
}

fn count_cache_keys(prefix: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    Ok(keys.len())
}

fn unique_cache_prefix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_nanos())
        .unwrap_or(0);
    format!("omni:test:link_graph:{nanos}")
}

mod batch_quantum_scorer;
mod build_scope;
mod build_scope_fixture_support;
mod cache_build;
mod cache_build_fixture_support;
mod graph_navigation;
mod graph_navigation_fixture_support;
mod markdown_attachments;
mod markdown_attachments_fixture_support;
mod page_index;
mod page_index_fixture_support;
mod quantum_anchor_batch;
mod quantum_fixture_support;
mod quantum_fusion;
mod query_parsing;
mod refresh;
mod refresh_fixture_support;
mod search_core;
mod search_core_fixture_support;
mod search_filters;
mod search_filters_fixture_support;
mod search_match_fixture_support;
mod search_match_strategies;
mod semantic_ignition;
mod semantic_policy;
mod semantic_policy_fixture_support;
mod tree_scope_filters;
mod tree_scope_fixture_support;
