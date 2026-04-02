use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use super::{
    RepositoryAnalysisCacheKey, RepositorySearchArtifacts, RepositorySearchQueryCacheKey,
    build_repository_analysis_cache_key, load_cached_repository_search_artifacts,
    load_cached_repository_search_result, store_cached_repository_search_artifacts,
    store_cached_repository_search_result,
};
use crate::analyzers::config::{
    RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy,
};
use crate::git::checkout::{
    LocalCheckoutMetadata, ResolvedRepositorySource, ResolvedRepositorySourceKind,
};
use crate::search::{FuzzySearchOptions, SearchDocumentIndex};

fn ok_or_panic<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: std::fmt::Display,
{
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

fn some_or_panic<T>(value: Option<T>, context: &str) -> T {
    value.unwrap_or_else(|| panic!("{context}"))
}

fn sample_analysis_key(repo_id: &str) -> RepositoryAnalysisCacheKey {
    RepositoryAnalysisCacheKey {
        repo_id: repo_id.to_string(),
        checkout_root: format!("/virtual/{repo_id}"),
        checkout_revision: Some("rev-1".to_string()),
        mirror_revision: Some("mirror-1".to_string()),
        tracking_revision: Some("tracking-1".to_string()),
        plugin_ids: vec!["plugin-a".to_string()],
    }
}

fn empty_artifacts() -> RepositorySearchArtifacts {
    RepositorySearchArtifacts {
        module_index: SearchDocumentIndex::new(),
        symbol_index: SearchDocumentIndex::new(),
        example_index: SearchDocumentIndex::new(),
        projected_page_index: SearchDocumentIndex::new(),
        modules_by_id: BTreeMap::default(),
        symbols_by_id: BTreeMap::default(),
        examples_by_id: BTreeMap::default(),
        example_metadata: BTreeMap::default(),
        projected_pages_by_id: HashMap::default(),
        projected_pages: Vec::new(),
    }
}

#[test]
fn build_repository_analysis_cache_key_sorts_and_deduplicates_plugin_ids() {
    let repository = RegisteredRepository {
        id: "repo-cache-key".to_string(),
        path: Some(PathBuf::from("/tmp/repo-cache-key")),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![
            RepositoryPluginConfig::Id("plugin-z".to_string()),
            RepositoryPluginConfig::Id("plugin-a".to_string()),
            RepositoryPluginConfig::Id("plugin-z".to_string()),
        ],
    };
    let source = ResolvedRepositorySource {
        checkout_root: PathBuf::from("/tmp/repo-cache-key"),
        mirror_root: None,
        mirror_revision: Some("mirror-1".to_string()),
        tracking_revision: Some("tracking-1".to_string()),
        last_fetched_at: None,
        drift_state: crate::analyzers::query::RepoSyncDriftState::NotApplicable,
        mirror_state: crate::git::checkout::RepositoryLifecycleState::NotApplicable,
        checkout_state: crate::git::checkout::RepositoryLifecycleState::Validated,
        source_kind: ResolvedRepositorySourceKind::LocalCheckout,
    };
    let metadata = Some(LocalCheckoutMetadata {
        revision: Some("rev-1".to_string()),
        remote_url: None,
    });

    let key = build_repository_analysis_cache_key(&repository, &source, metadata.as_ref());

    assert_eq!(
        key.plugin_ids,
        vec!["plugin-a".to_string(), "plugin-z".to_string()]
    );
    assert_eq!(key.checkout_revision, Some("rev-1".to_string()));
}

#[test]
fn repository_search_artifacts_cache_roundtrip_uses_analysis_identity() {
    let key = sample_analysis_key("artifact-cache-roundtrip");
    let stored = ok_or_panic(
        store_cached_repository_search_artifacts(key.clone(), empty_artifacts()),
        "artifact cache store should succeed",
    );
    let loaded = some_or_panic(
        ok_or_panic(
            load_cached_repository_search_artifacts(&key),
            "artifact cache load should succeed",
        ),
        "stored artifacts should be present",
    );

    assert!(std::sync::Arc::ptr_eq(&stored, &loaded));
}

#[test]
fn repository_search_query_cache_isolated_by_endpoint_and_filter() {
    let analysis_key = sample_analysis_key("query-cache-isolation");
    let options = FuzzySearchOptions::document_search();
    let module_key = RepositorySearchQueryCacheKey::new(
        &analysis_key,
        "repo.module-search",
        "solve",
        None,
        options,
        10,
    );
    let projected_key = RepositorySearchQueryCacheKey::new(
        &analysis_key,
        "repo.projected-page-search",
        "solve",
        Some("reference".to_string()),
        options,
        10,
    );

    ok_or_panic(
        store_cached_repository_search_result(module_key.clone(), &vec!["module"]),
        "query cache store should succeed",
    );
    ok_or_panic(
        store_cached_repository_search_result(projected_key.clone(), &vec!["projected"]),
        "query cache store should succeed",
    );

    let module_value: Vec<String> = some_or_panic(
        ok_or_panic(
            load_cached_repository_search_result(&module_key),
            "query cache load should succeed",
        ),
        "module cached value should exist",
    );
    let projected_value: Vec<String> = some_or_panic(
        ok_or_panic(
            load_cached_repository_search_result(&projected_key),
            "query cache load should succeed",
        ),
        "projected cached value should exist",
    );

    assert_eq!(module_value, vec!["module".to_string()]);
    assert_eq!(projected_value, vec!["projected".to_string()]);
}
