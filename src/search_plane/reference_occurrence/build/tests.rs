use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::cache::SearchPlaneCache;
use crate::search_plane::reference_occurrence::build::{
    ensure_reference_occurrence_index_started, fingerprint_projects,
    plan_reference_occurrence_build,
};
use crate::search_plane::reference_occurrence::search_reference_occurrences;
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
    SearchPlaneService,
};

async fn wait_for_reference_occurrence_ready(
    service: &SearchPlaneService,
    previous_epoch: Option<u64>,
) {
    for _ in 0..100 {
        let status = service
            .coordinator()
            .status_for(SearchCorpusKind::ReferenceOccurrence);
        if status.phase == SearchPlanePhase::Ready
            && status.active_epoch.is_some()
            && previous_epoch.is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("reference occurrence build did not reach ready state");
}

#[test]
fn plan_reference_occurrence_build_only_reparses_changed_files() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path();
    std::fs::create_dir_all(project_root.join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    std::fs::write(
        project_root.join("src/lib.rs"),
        "fn alpha() {}\nfn use_alpha() { alpha(); }\n",
    )
    .unwrap_or_else(|error| panic!("write lib: {error}"));
    std::fs::write(
        project_root.join("src/extra.rs"),
        "fn gamma() {}\nfn use_gamma() { gamma(); }\n",
    )
    .unwrap_or_else(|error| panic!("write extra: {error}"));
    let projects = vec![UiProjectConfig {
        name: "demo".to_string(),
        root: ".".to_string(),
        dirs: vec![".".to_string()],
    }];

    let first = plan_reference_occurrence_build(
        project_root,
        project_root,
        &projects,
        None,
        BTreeMap::new(),
    );
    assert_eq!(first.base_epoch, None);
    assert!(
        first
            .changed_hits
            .iter()
            .any(|hit| hit.path == "src/lib.rs" && hit.name == "alpha")
    );
    assert!(
        first
            .changed_hits
            .iter()
            .any(|hit| hit.path == "src/extra.rs" && hit.name == "gamma")
    );

    std::thread::sleep(Duration::from_millis(5));
    std::fs::write(
        project_root.join("src/lib.rs"),
        "fn beta() {}\nfn use_beta() { beta(); }\n",
    )
    .unwrap_or_else(|error| panic!("rewrite lib: {error}"));

    let second = plan_reference_occurrence_build(
        project_root,
        project_root,
        &projects,
        Some(7),
        first.file_fingerprints.clone(),
    );
    assert_eq!(second.base_epoch, Some(7));
    assert_eq!(
        second.replaced_paths,
        BTreeSet::from(["src/lib.rs".to_string()])
    );
    assert!(
        second
            .changed_hits
            .iter()
            .all(|hit| hit.path == "src/lib.rs")
    );
    assert!(
        second.changed_hits.iter().any(|hit| hit.name == "beta"),
        "changed-file rebuild must include the updated token"
    );
    assert!(
        second.changed_hits.iter().all(|hit| hit.name != "gamma"),
        "unchanged file rows must not be reparsed into the changed set"
    );
}

#[tokio::test]
async fn reference_occurrence_incremental_refresh_reuses_unchanged_rows() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path().join("workspace");
    let storage_root = temp_dir.path().join("search_plane");
    std::fs::create_dir_all(project_root.join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    std::fs::write(
        project_root.join("src/lib.rs"),
        "fn alpha() {}\nfn use_alpha() { alpha(); }\n",
    )
    .unwrap_or_else(|error| panic!("write lib: {error}"));
    std::fs::write(
        project_root.join("src/extra.rs"),
        "fn gamma() {}\nfn use_gamma() { gamma(); }\n",
    )
    .unwrap_or_else(|error| panic!("write extra: {error}"));
    let projects = vec![UiProjectConfig {
        name: "demo".to_string(),
        root: ".".to_string(),
        dirs: vec![".".to_string()],
    }];
    let keyspace =
        SearchManifestKeyspace::new("xiuxian:test:search_plane:reference-occurrence-incremental");
    let cache = SearchPlaneCache::for_tests(keyspace.clone());
    let service = SearchPlaneService::with_runtime(
        project_root.clone(),
        storage_root,
        keyspace,
        SearchMaintenancePolicy::default(),
        cache,
    );

    ensure_reference_occurrence_index_started(
        &service,
        project_root.as_path(),
        project_root.as_path(),
        &projects,
    );
    wait_for_reference_occurrence_ready(&service, None).await;

    let initial_gamma = search_reference_occurrences(&service, "gamma", 10)
        .await
        .unwrap_or_else(|error| panic!("query gamma: {error}"));
    assert_eq!(initial_gamma.len(), 1);
    let initial_alpha = search_reference_occurrences(&service, "alpha", 10)
        .await
        .unwrap_or_else(|error| panic!("query alpha: {error}"));
    assert_eq!(initial_alpha.len(), 1);

    std::fs::write(
        project_root.join("src/lib.rs"),
        "fn beta() {}\nfn use_beta() { beta(); }\n",
    )
    .unwrap_or_else(|error| panic!("rewrite lib: {error}"));
    ensure_reference_occurrence_index_started(
        &service,
        project_root.as_path(),
        project_root.as_path(),
        &projects,
    );
    wait_for_reference_occurrence_ready(&service, Some(1)).await;

    let gamma = search_reference_occurrences(&service, "gamma", 10)
        .await
        .unwrap_or_else(|error| panic!("query gamma after refresh: {error}"));
    assert_eq!(gamma.len(), 1);
    let beta = search_reference_occurrences(&service, "beta", 10)
        .await
        .unwrap_or_else(|error| panic!("query beta after refresh: {error}"));
    assert_eq!(beta.len(), 1);
    let alpha = search_reference_occurrences(&service, "alpha", 10)
        .await
        .unwrap_or_else(|error| panic!("query alpha after refresh: {error}"));
    assert!(alpha.is_empty());
    let active_epoch = service
        .coordinator()
        .status_for(SearchCorpusKind::ReferenceOccurrence)
        .active_epoch
        .unwrap_or_else(|| panic!("reference occurrence active epoch"));
    assert!(
        service
            .local_epoch_parquet_path(SearchCorpusKind::ReferenceOccurrence, active_epoch)
            .exists(),
        "missing reference occurrence parquet export"
    );
}

#[test]
fn fingerprint_projects_changes_when_scanned_file_metadata_changes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path();
    std::fs::create_dir_all(project_root.join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    std::fs::create_dir_all(project_root.join("node_modules/pkg"))
        .unwrap_or_else(|error| panic!("create skipped dir: {error}"));
    std::fs::write(
        project_root.join("src/lib.rs"),
        "fn alpha() {}\nfn use_alpha() { alpha(); }\n",
    )
    .unwrap_or_else(|error| panic!("write rust source: {error}"));
    std::fs::write(
        project_root.join("node_modules/pkg/index.js"),
        "ignored();\n",
    )
    .unwrap_or_else(|error| panic!("write skipped file: {error}"));

    let projects = vec![UiProjectConfig {
        name: "demo".to_string(),
        root: ".".to_string(),
        dirs: vec![".".to_string()],
    }];

    let first = fingerprint_projects(project_root, project_root, &projects);
    std::fs::write(
        project_root.join("node_modules/pkg/index.js"),
        "ignored-again();\n",
    )
    .unwrap_or_else(|error| panic!("rewrite skipped file: {error}"));
    let after_skipped_change = fingerprint_projects(project_root, project_root, &projects);
    assert_eq!(first, after_skipped_change);

    std::fs::write(
        project_root.join("src/lib.rs"),
        "fn beta() {}\nfn use_beta() { beta(); }\n",
    )
    .unwrap_or_else(|error| panic!("rewrite rust source: {error}"));
    let second = fingerprint_projects(project_root, project_root, &projects);
    assert_ne!(first, second);
}
