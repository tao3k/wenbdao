use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::cache::SearchPlaneCache;
use crate::search_plane::knowledge_section::search_knowledge_sections;
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
    SearchPlaneService,
};

use super::orchestration::{ensure_knowledge_section_index_started, plan_knowledge_section_build};
use super::paths::fingerprint_projects;

#[test]
fn plan_knowledge_section_build_only_reparses_changed_notes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path();
    std::fs::create_dir_all(project_root.join("notes"))
        .unwrap_or_else(|error| panic!("create notes: {error}"));
    std::fs::write(
        project_root.join("notes/alpha.md"),
        "# Alpha\n\nAlpha body.\n\n## Overview\n\nAlpha section.\n",
    )
    .unwrap_or_else(|error| panic!("write alpha note: {error}"));
    std::fs::write(
        project_root.join("notes/gamma.md"),
        "# Gamma\n\nGamma body.\n\n## Overview\n\nGamma section.\n",
    )
    .unwrap_or_else(|error| panic!("write gamma note: {error}"));
    let projects = vec![UiProjectConfig {
        name: "notes".to_string(),
        root: ".".to_string(),
        dirs: vec![".".to_string()],
    }];

    let first =
        plan_knowledge_section_build(project_root, project_root, &projects, None, BTreeMap::new());
    assert_eq!(first.base_epoch, None);
    assert!(
        first
            .changed_rows
            .iter()
            .any(|row| row.path == "notes/alpha.md")
    );
    assert!(
        first
            .changed_rows
            .iter()
            .any(|row| row.path == "notes/gamma.md")
    );

    std::thread::sleep(Duration::from_millis(5));
    std::fs::write(
        project_root.join("notes/alpha.md"),
        "# Beta\n\nBeta body.\n\n## Overview\n\nBeta section.\n",
    )
    .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));

    let second = plan_knowledge_section_build(
        project_root,
        project_root,
        &projects,
        Some(7),
        first.file_fingerprints.clone(),
    );
    assert_eq!(second.base_epoch, Some(7));
    assert_eq!(
        second.replaced_paths,
        BTreeSet::from(["notes/alpha.md".to_string()])
    );
    assert!(
        second
            .changed_rows
            .iter()
            .all(|row| row.path == "notes/alpha.md")
    );
    assert!(
        second
            .changed_rows
            .iter()
            .any(|row| row.search_text.contains("Beta"))
    );
    assert!(
        second
            .changed_rows
            .iter()
            .all(|row| !row.path.contains("gamma")),
        "unchanged note rows must not be reparsed into the changed set"
    );
}

#[tokio::test]
async fn knowledge_section_incremental_refresh_reuses_unchanged_rows() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path().join("workspace");
    let storage_root = temp_dir.path().join("search_plane");
    std::fs::create_dir_all(project_root.join("notes"))
        .unwrap_or_else(|error| panic!("create notes: {error}"));
    std::fs::write(
        project_root.join("notes/alpha.md"),
        "# Alpha\n\nAlpha body.\n\n## Overview\n\nAlpha section.\n",
    )
    .unwrap_or_else(|error| panic!("write alpha note: {error}"));
    std::fs::write(
        project_root.join("notes/gamma.md"),
        "# Gamma\n\nGamma body.\n\n## Overview\n\nGamma section.\n",
    )
    .unwrap_or_else(|error| panic!("write gamma note: {error}"));
    let projects = vec![UiProjectConfig {
        name: "notes".to_string(),
        root: ".".to_string(),
        dirs: vec![".".to_string()],
    }];
    let keyspace =
        SearchManifestKeyspace::new("xiuxian:test:search_plane:knowledge-section-incremental");
    let cache = SearchPlaneCache::for_tests(keyspace.clone());
    let service = SearchPlaneService::with_runtime(
        project_root.clone(),
        storage_root,
        keyspace,
        SearchMaintenancePolicy::default(),
        cache,
    );

    ensure_knowledge_section_index_started(
        &service,
        project_root.as_path(),
        project_root.as_path(),
        &projects,
    );
    wait_for_knowledge_section_ready(&service, None).await;

    let initial_gamma = search_knowledge_sections(&service, "Gamma body", 10)
        .await
        .unwrap_or_else(|error| panic!("query gamma: {error}"));
    assert_eq!(initial_gamma.len(), 1);
    let initial_alpha = search_knowledge_sections(&service, "Alpha body", 10)
        .await
        .unwrap_or_else(|error| panic!("query alpha: {error}"));
    assert_eq!(initial_alpha.len(), 1);

    std::fs::write(
        project_root.join("notes/alpha.md"),
        "# Beta\n\nBeta body.\n\n## Overview\n\nBeta section.\n",
    )
    .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));
    ensure_knowledge_section_index_started(
        &service,
        project_root.as_path(),
        project_root.as_path(),
        &projects,
    );
    wait_for_knowledge_section_ready(&service, Some(1)).await;

    let gamma = search_knowledge_sections(&service, "Gamma body", 10)
        .await
        .unwrap_or_else(|error| panic!("query gamma after refresh: {error}"));
    assert_eq!(gamma.len(), 1);
    let beta = search_knowledge_sections(&service, "Beta body", 10)
        .await
        .unwrap_or_else(|error| panic!("query beta after refresh: {error}"));
    assert_eq!(beta.len(), 1);
    let alpha = search_knowledge_sections(&service, "Alpha body", 10)
        .await
        .unwrap_or_else(|error| panic!("query alpha after refresh: {error}"));
    assert!(alpha.is_empty());
    let active_epoch = service
        .coordinator()
        .status_for(SearchCorpusKind::KnowledgeSection)
        .active_epoch
        .unwrap_or_else(|| panic!("knowledge section active epoch"));
    assert!(
        service
            .local_epoch_parquet_path(SearchCorpusKind::KnowledgeSection, active_epoch)
            .exists(),
        "missing knowledge section parquet export"
    );
}

async fn wait_for_knowledge_section_ready(
    service: &SearchPlaneService,
    previous_epoch: Option<u64>,
) {
    for _ in 0..100 {
        let status = service
            .coordinator()
            .status_for(SearchCorpusKind::KnowledgeSection);
        if status.phase == SearchPlanePhase::Ready
            && status.active_epoch.is_some()
            && previous_epoch.is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("knowledge section build did not reach ready state");
}

#[test]
fn fingerprint_projects_changes_when_scanned_note_metadata_changes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path();
    std::fs::create_dir_all(project_root.join("notes"))
        .unwrap_or_else(|error| panic!("create notes: {error}"));
    std::fs::create_dir_all(project_root.join("node_modules/pkg"))
        .unwrap_or_else(|error| panic!("create skipped dir: {error}"));
    std::fs::write(
        project_root.join("notes/alpha.md"),
        "# Alpha\n\nAlpha body.\n",
    )
    .unwrap_or_else(|error| panic!("write note: {error}"));
    std::fs::write(
        project_root.join("node_modules/pkg/ignored.md"),
        "# Ignored\n",
    )
    .unwrap_or_else(|error| panic!("write skipped file: {error}"));

    let projects = vec![UiProjectConfig {
        name: "notes".to_string(),
        root: ".".to_string(),
        dirs: vec![".".to_string()],
    }];

    let first = fingerprint_projects(project_root, project_root, &projects);
    std::fs::write(
        project_root.join("node_modules/pkg/ignored.md"),
        "# Still Ignored\n",
    )
    .unwrap_or_else(|error| panic!("rewrite skipped note: {error}"));
    let after_skipped_change = fingerprint_projects(project_root, project_root, &projects);
    assert_eq!(first, after_skipped_change);

    std::fs::write(
        project_root.join("notes/alpha.md"),
        "# Beta\n\nBeta body.\n",
    )
    .unwrap_or_else(|error| panic!("rewrite note: {error}"));
    let second = fingerprint_projects(project_root, project_root, &projects);
    assert_ne!(first, second);
}
