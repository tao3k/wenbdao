use std::path::PathBuf;

use crate::gateway::studio::types::{AttachmentSearchHit, StudioNavigationTarget};
use crate::search_plane::attachment::query::scoring::{compare_candidates, retained_window};
use crate::search_plane::attachment::query::search::search_attachment_hits;
use crate::search_plane::attachment::query::types::AttachmentCandidate;
use crate::search_plane::attachment::schema::{
    attachment_batches, attachment_schema, search_text_column,
};
use crate::search_plane::ranking::trim_ranked_vec;
use crate::search_plane::{
    BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneService,
};

#[test]
fn trim_candidates_keeps_highest_ranked_attachment_hits() {
    let mut candidates = vec![
        AttachmentCandidate {
            id: "zeta".to_string(),
            score: 0.4,
            source_path: "docs/zeta.md".to_string(),
            attachment_path: "assets/zeta.png".to_string(),
        },
        AttachmentCandidate {
            id: "beta".to_string(),
            score: 0.9,
            source_path: "docs/beta.md".to_string(),
            attachment_path: "assets/beta.png".to_string(),
        },
        AttachmentCandidate {
            id: "alpha".to_string(),
            score: 0.9,
            source_path: "docs/alpha.md".to_string(),
            attachment_path: "assets/alpha.png".to_string(),
        },
    ];

    trim_ranked_vec(&mut candidates, 2, compare_candidates);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].attachment_path, "assets/alpha.png");
    assert_eq!(candidates[1].attachment_path, "assets/beta.png");
}

#[test]
fn retained_window_scales_with_limit() {
    assert_eq!(retained_window(0).target, 32);
    assert_eq!(retained_window(8).target, 32);
    assert_eq!(retained_window(32).target, 64);
}

fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:attachment"),
        SearchMaintenancePolicy::default(),
    )
}

fn sample_hit(
    name: &str,
    source_path: &str,
    attachment_path: &str,
    kind: &str,
) -> AttachmentSearchHit {
    AttachmentSearchHit {
        name: name.to_string(),
        path: source_path.to_string(),
        source_id: source_path.trim_end_matches(".md").to_string(),
        source_stem: "alpha".to_string(),
        source_title: "Alpha".to_string(),
        source_path: source_path.to_string(),
        attachment_id: format!("att://{source_path}/{attachment_path}"),
        attachment_path: attachment_path.to_string(),
        attachment_name: name.to_string(),
        attachment_ext: attachment_path
            .split('.')
            .next_back()
            .unwrap_or_default()
            .to_string(),
        kind: kind.to_string(),
        navigation_target: StudioNavigationTarget {
            path: source_path.to_string(),
            category: "doc".to_string(),
            project_name: None,
            root_label: None,
            line: None,
            line_end: None,
            column: None,
        },
        score: 0.0,
        vision_snippet: None,
    }
}

#[tokio::test]
async fn attachment_query_reads_hits_from_published_epoch() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::Attachment,
        "fp-1",
        SearchCorpusKind::Attachment.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let hits = vec![
        sample_hit(
            "topology.png",
            "docs/alpha.md",
            "assets/topology.png",
            "image",
        ),
        sample_hit("spec.pdf", "docs/alpha.md", "files/spec.pdf", "pdf"),
    ];
    let store = service
        .open_store(SearchCorpusKind::Attachment)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            attachment_schema(),
            attachment_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .create_inverted_index(table_name.as_str(), search_text_column(), None)
        .await
        .unwrap_or_else(|error| panic!("create inverted index: {error}"));
    crate::search_plane::attachment::build::export_attachment_epoch_parquet(&service, lease.epoch)
        .await
        .unwrap_or_else(|error| panic!("export attachment parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);

    let results = search_attachment_hits(&service, "topology", 5, &[], &[], false)
        .await
        .unwrap_or_else(|error| panic!("query should succeed: {error}"));
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].attachment_name, "topology.png");
    assert!(results[0].score > 0.0);
}
