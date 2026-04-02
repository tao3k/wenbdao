use crate::analyzers::{RepoBacklinkItem, RepoSymbolKind, SymbolSearchHit};
use crate::gateway::studio::search::handlers::code_search::{
    helpers::symbol_search_hit_to_search_hit,
    query::{
        collect_repo_search_targets, infer_repo_hint_from_query, parse_code_search_query,
        repo_search_parallelism, repo_search_result_limits, repo_wide_code_search_timeout,
    },
};
use crate::gateway::studio::search::handlers::test_prelude::SearchQuery;
use crate::search_plane::{RepoSearchAvailability, RepoSearchPublicationState};

#[test]
fn parse_code_search_query_extracts_repo_lang_and_kind_filters() {
    let parsed = parse_code_search_query("repo:sciml lang:julia kind:function reexport", None);
    assert_eq!(parsed.query, "reexport");
    assert_eq!(parsed.repo.as_deref(), Some("sciml"));
    assert_eq!(parsed.languages, vec!["julia".to_string()]);
    assert_eq!(parsed.kinds, vec!["function".to_string()]);
}

#[test]
fn repo_wide_code_search_timeout_applies_only_without_repo_hint() {
    assert_eq!(repo_wide_code_search_timeout(Some("valid")), None,);
    assert_eq!(
        repo_wide_code_search_timeout(None),
        Some(std::time::Duration::from_secs(5)),
    );
}

#[test]
fn repo_wide_code_search_scopes_per_repo_limits() {
    assert_eq!(
        repo_search_result_limits(Some("valid"), 20),
        crate::gateway::studio::search::handlers::code_search::query::RepoSearchResultLimits {
            entity_limit: 20,
            content_limit: 20,
        }
    );
    assert_eq!(
        repo_search_result_limits(None, 20),
        crate::gateway::studio::search::handlers::code_search::query::RepoSearchResultLimits {
            entity_limit: 12,
            content_limit: 4,
        }
    );
    assert_eq!(
        repo_search_result_limits(None, 3),
        crate::gateway::studio::search::handlers::code_search::query::RepoSearchResultLimits {
            entity_limit: 3,
            content_limit: 3,
        }
    );
}

#[test]
fn parse_code_search_query_preserves_repo_identifier_case() {
    let parsed = parse_code_search_query(
        "repo:DifferentialEquations.jl using ModelingToolkit",
        Some("SciMLBase.jl"),
    );

    assert_eq!(parsed.query, "using ModelingToolkit");
    assert_eq!(parsed.repo.as_deref(), Some("DifferentialEquations.jl"));
}

#[test]
fn infer_repo_hint_from_query_matches_unique_normalized_repo_seed() {
    let parsed = parse_code_search_query("SciMLBase", None);
    let inferred = infer_repo_hint_from_query(
        &parsed,
        [
            "SciMLBase.jl",
            "DifferentialEquations.jl",
            "ModelingToolkit.jl",
        ],
    );

    assert_eq!(inferred.as_deref(), Some("SciMLBase.jl"));
}

#[test]
fn infer_repo_hint_from_query_ignores_ambiguous_normalized_repo_seed() {
    let parsed = parse_code_search_query("SciMLBase", None);
    let inferred = infer_repo_hint_from_query(&parsed, ["SciMLBase.jl", "scimlbase"]);

    assert_eq!(inferred, None);
}

#[test]
fn collect_repo_search_targets_preserves_repo_order_and_partitions_by_availability() {
    let publication_states = std::collections::BTreeMap::from([
        (
            "ready".to_string(),
            RepoSearchPublicationState {
                entity_published: true,
                content_published: false,
                availability: RepoSearchAvailability::Searchable,
            },
        ),
        (
            "pending".to_string(),
            RepoSearchPublicationState {
                entity_published: false,
                content_published: false,
                availability: RepoSearchAvailability::Pending,
            },
        ),
        (
            "skipped".to_string(),
            RepoSearchPublicationState {
                entity_published: false,
                content_published: false,
                availability: RepoSearchAvailability::Skipped,
            },
        ),
    ]);

    let dispatch = collect_repo_search_targets(
        vec![
            "ready".to_string(),
            "pending".to_string(),
            "skipped".to_string(),
            "implicit-pending".to_string(),
        ],
        &publication_states,
    );

    assert_eq!(
        dispatch
            .searchable_repos
            .into_iter()
            .map(|target| target.repo_id)
            .collect::<Vec<_>>(),
        vec!["ready".to_string()]
    );
    assert_eq!(
        dispatch.pending_repos,
        vec!["pending".to_string(), "implicit-pending".to_string()]
    );
    assert_eq!(dispatch.skipped_repos, vec!["skipped".to_string()]);
}

#[test]
fn repo_search_parallelism_reuses_search_plane_read_budget() {
    let studio = crate::gateway::studio::search::handlers::tests::test_studio_state();
    let expected_budget = studio.search_plane.repo_search_read_concurrency_limit;

    assert_eq!(
        repo_search_parallelism(&studio.search_plane, usize::MAX),
        expected_budget
    );
    assert_eq!(repo_search_parallelism(&studio.search_plane, 2), 2);
    assert_eq!(repo_search_parallelism(&studio.search_plane, 0), 1);
}

#[test]
fn search_query_deserializes_query_alias() {
    let query: SearchQuery = serde_json::from_value(serde_json::json!({
        "query": "reexport",
        "intent": "code_search",
    }))
    .unwrap_or_else(|error| panic!("query alias should deserialize: {error}"));

    assert_eq!(query.q.as_deref(), Some("reexport"));
    assert_eq!(query.intent.as_deref(), Some("code_search"));
}

#[test]
fn symbol_search_hit_to_search_hit_preserves_backend_metadata() {
    let hit = symbol_search_hit_to_search_hit(
        "sciml",
        SymbolSearchHit {
            symbol: crate::analyzers::SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:reexport".to_string(),
                module_id: Some("module:BaseModelica".to_string()),
                name: "reexport".to_string(),
                qualified_name: "BaseModelica.reexport".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/BaseModelica.jl".to_string(),
                line_start: Some(7),
                line_end: Some(9),
                signature: Some("reexport()".to_string()),
                audit_status: Some("verified".to_string()),
                verification_state: Some("verified".to_string()),
                attributes: std::collections::BTreeMap::new(),
            },
            score: Some(0.8),
            rank: Some(1),
            saliency_score: Some(0.9),
            hierarchical_uri: Some("repo://sciml/symbol/reexport".to_string()),
            hierarchy: Some(vec!["src".to_string(), "BaseModelica.jl".to_string()]),
            implicit_backlinks: Some(vec!["doc:readme".to_string()]),
            implicit_backlink_items: Some(vec![RepoBacklinkItem {
                id: "doc:readme".to_string(),
                title: Some("README".to_string()),
                path: Some("README.md".to_string()),
                kind: Some("documents".to_string()),
            }]),
            projection_page_ids: Some(vec!["projection:1".to_string()]),
            audit_status: Some("verified".to_string()),
            verification_state: Some("verified".to_string()),
        },
    );

    assert_eq!(hit.doc_type.as_deref(), Some("symbol"));
    assert!(hit.tags.iter().any(|tag| tag == "lang:julia"));
    assert!(hit.tags.iter().any(|tag| tag == "kind:function"));
    assert!((hit.score - 0.9).abs() < f64::EPSILON);
    assert_eq!(
        hit.navigation_target.and_then(|target| target.project_name),
        Some("sciml".to_string())
    );
    assert_eq!(hit.audit_status.as_deref(), Some("verified"));
}
