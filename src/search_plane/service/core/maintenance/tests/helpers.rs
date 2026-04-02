use std::path::PathBuf;

use crate::search_plane::coordinator::SearchCompactionReason;
use crate::search_plane::service::core::{
    RepoCompactionTask, RepoMaintenanceTask, RepoPrewarmTask, SearchPlaneService,
};
use crate::search_plane::{SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace};

pub(crate) fn make_service(temp_dir: &tempfile::TempDir, keyspace: &str) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new(keyspace),
        SearchMaintenancePolicy::default(),
    )
}

pub(crate) fn make_prewarm_task(
    corpus: SearchCorpusKind,
    repo_id: &str,
    table_name: &str,
    projected_columns: &[&str],
) -> RepoMaintenanceTask {
    RepoMaintenanceTask::Prewarm(RepoPrewarmTask {
        corpus,
        repo_id: repo_id.to_string(),
        table_name: table_name.to_string(),
        projected_columns: projected_columns
            .iter()
            .map(|column| column.to_string())
            .collect(),
    })
}

pub(crate) fn make_compaction_task(
    corpus: SearchCorpusKind,
    repo_id: &str,
    publication_id: &str,
    table_name: &str,
    row_count: u64,
    reason: SearchCompactionReason,
) -> RepoMaintenanceTask {
    RepoMaintenanceTask::Compaction(RepoCompactionTask {
        corpus,
        repo_id: repo_id.to_string(),
        publication_id: publication_id.to_string(),
        table_name: table_name.to_string(),
        row_count,
        reason,
    })
}
