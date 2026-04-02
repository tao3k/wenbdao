use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::gateway::studio::types::UiProjectConfig;
use crate::link_graph::LinkGraphAttachmentKind;
use crate::search_plane::attachment::build::plan_attachment_build;
use crate::search_plane::attachment::search_attachment_hits;
use crate::search_plane::cache::SearchPlaneCache;
use crate::search_plane::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
    SearchPlaneService,
};

#[test]
fn plan_attachment_build_only_reparses_changed_notes() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path();
    std::fs::create_dir_all(project_root.join("docs"))
        .unwrap_or_else(|error| panic!("create docs: {error}"));
    std::fs::write(
        project_root.join("docs/alpha.md"),
        "# Alpha\n\n![Topology](assets/topology.png)\n",
    )
    .unwrap_or_else(|error| panic!("write alpha note: {error}"));
    std::fs::write(
        project_root.join("docs/beta.md"),
        "# Beta\n\n![Avatar](images/avatar.jpg)\n",
    )
    .unwrap_or_else(|error| panic!("write beta note: {error}"));
    let projects = vec![UiProjectConfig {
        name: "kernel".to_string(),
        root: ".".to_string(),
        dirs: vec!["docs".to_string()],
    }];

    let first = plan_attachment_build(project_root, project_root, &projects, None, BTreeMap::new());
    assert_eq!(first.base_epoch, None);
    assert!(
        first
            .changed_hits
            .iter()
            .any(|hit| hit.source_path == "docs/alpha.md" && hit.attachment_name == "topology.png")
    );
    assert!(
        first
            .changed_hits
            .iter()
            .any(|hit| hit.source_path == "docs/beta.md" && hit.attachment_name == "avatar.jpg")
    );

    std::thread::sleep(Duration::from_millis(5));
    std::fs::write(
        project_root.join("docs/alpha.md"),
        "# Alpha\n\n![Diagram](assets/diagram.png)\n",
    )
    .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));

    let second = plan_attachment_build(
        project_root,
        project_root,
        &projects,
        Some(7),
        first.file_fingerprints.clone(),
    );
    assert_eq!(second.base_epoch, Some(7));
    assert_eq!(
        second.replaced_paths,
        BTreeSet::from(["docs/alpha.md".to_string()])
    );
    assert!(
        second
            .changed_hits
            .iter()
            .all(|hit| hit.source_path == "docs/alpha.md")
    );
    assert!(
        second
            .changed_hits
            .iter()
            .any(|hit| hit.attachment_name == "diagram.png")
    );
    assert!(
        second
            .changed_hits
            .iter()
            .all(|hit| hit.attachment_name != "avatar.jpg"),
        "unchanged note attachments must not be reparsed into the changed set"
    );
}

#[tokio::test]
async fn attachment_incremental_refresh_reuses_unchanged_rows() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path().join("workspace");
    let storage_root = temp_dir.path().join("search_plane");
    std::fs::create_dir_all(project_root.join("docs"))
        .unwrap_or_else(|error| panic!("create docs: {error}"));
    std::fs::write(
        project_root.join("docs/alpha.md"),
        "# Alpha\n\n![Topology](assets/topology.png)\n",
    )
    .unwrap_or_else(|error| panic!("write alpha note: {error}"));
    std::fs::write(
        project_root.join("docs/beta.md"),
        "# Beta\n\n![Avatar](images/avatar.jpg)\n",
    )
    .unwrap_or_else(|error| panic!("write beta note: {error}"));
    let projects = vec![UiProjectConfig {
        name: "kernel".to_string(),
        root: ".".to_string(),
        dirs: vec!["docs".to_string()],
    }];
    let keyspace = SearchManifestKeyspace::new("xiuxian:test:search_plane:attachment-incremental");
    let cache = SearchPlaneCache::for_tests(keyspace.clone());
    let service = SearchPlaneService::with_runtime(
        project_root.clone(),
        storage_root,
        keyspace,
        SearchMaintenancePolicy::default(),
        cache,
    );

    super::ensure_attachment_index_started(
        &service,
        project_root.as_path(),
        project_root.as_path(),
        &projects,
    );
    wait_for_attachment_ready(&service, None).await;

    let initial_avatar = search_attachment_hits(&service, "avatar", 10, &[], &[], false)
        .await
        .unwrap_or_else(|error| panic!("query avatar: {error}"));
    assert_eq!(initial_avatar.len(), 1);
    let initial_topology = search_attachment_hits(&service, "topology", 10, &[], &[], false)
        .await
        .unwrap_or_else(|error| panic!("query topology: {error}"));
    assert_eq!(initial_topology.len(), 1);

    std::fs::write(
        project_root.join("docs/alpha.md"),
        "# Alpha\n\n![Diagram](assets/diagram.png)\n",
    )
    .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));
    super::ensure_attachment_index_started(
        &service,
        project_root.as_path(),
        project_root.as_path(),
        &projects,
    );
    wait_for_attachment_ready(&service, Some(1)).await;

    let avatar = search_attachment_hits(&service, "avatar", 10, &[], &[], false)
        .await
        .unwrap_or_else(|error| panic!("query avatar after refresh: {error}"));
    assert_eq!(avatar.len(), 1);
    let diagram = search_attachment_hits(&service, "diagram", 10, &[], &[], false)
        .await
        .unwrap_or_else(|error| panic!("query diagram after refresh: {error}"));
    assert_eq!(diagram.len(), 1);
    assert_eq!(diagram[0].kind, "image");
    let topology = search_attachment_hits(
        &service,
        "topology",
        10,
        &[],
        &[LinkGraphAttachmentKind::Image],
        false,
    )
    .await
    .unwrap_or_else(|error| panic!("query topology after refresh: {error}"));
    assert!(topology.is_empty());
    let active_epoch = service
        .coordinator()
        .status_for(SearchCorpusKind::Attachment)
        .active_epoch
        .unwrap_or_else(|| panic!("attachment active epoch"));
    assert!(
        service
            .local_epoch_parquet_path(SearchCorpusKind::Attachment, active_epoch)
            .exists(),
        "missing attachment parquet export"
    );
}

async fn wait_for_attachment_ready(service: &SearchPlaneService, previous_epoch: Option<u64>) {
    for _ in 0..100 {
        let status = service
            .coordinator()
            .status_for(SearchCorpusKind::Attachment);
        if status.phase == SearchPlanePhase::Ready
            && status.active_epoch.is_some()
            && previous_epoch.is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("attachment build did not reach ready state");
}
