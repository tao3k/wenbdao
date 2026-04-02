use std::path::Path;

use tokio::runtime::Handle;

use crate::gateway::studio::types::UiProjectConfig;
#[cfg(test)]
use crate::search_plane::local_symbol::build::LocalSymbolBuildError;
use crate::search_plane::local_symbol::build::{
    fingerprint_projects, plan_local_symbol_build, write_local_symbol_epoch,
};
use crate::search_plane::{BeginBuildDecision, SearchCorpusKind, SearchPlaneService};

pub(crate) fn ensure_local_symbol_index_started(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) {
    if projects.is_empty() {
        return;
    }

    let fingerprint = fingerprint_projects(project_root, config_root, projects);
    let decision = service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        fingerprint,
        SearchCorpusKind::LocalSymbol.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let active_epoch = service
        .corpus_active_epoch(SearchCorpusKind::LocalSymbol)
        .filter(|epoch| {
            service.local_epoch_has_partition_tables(SearchCorpusKind::LocalSymbol, *epoch)
        });
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::LocalSymbol)
                .await;
            let build: Result<_, tokio::task::JoinError> = tokio::task::spawn_blocking(move || {
                plan_local_symbol_build(
                    build_project_root.as_path(),
                    build_config_root.as_path(),
                    &build_projects,
                    active_epoch,
                    previous_fingerprints,
                )
            })
            .await;

            match build {
                Ok(plan) => {
                    service.coordinator().update_progress(&lease, 0.3);
                    let write = write_local_symbol_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service.coordinator().fail_build(
                            &lease,
                            format!("local symbol epoch write failed: {error}"),
                        );
                        return;
                    }
                    let write = write.unwrap_or_else(|_| unreachable!());
                    let prewarm_columns =
                        crate::search_plane::local_symbol::schema::projected_columns();
                    if let Err(error) = service
                        .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                        .await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("local symbol epoch prewarm failed: {error}"),
                        );
                        return;
                    }
                    service.coordinator().update_progress(&lease, 0.9);
                    if service.publish_ready_and_maintain(
                        &lease,
                        write.row_count,
                        write.fragment_count,
                    ) {
                        service
                            .set_corpus_file_fingerprints(
                                SearchCorpusKind::LocalSymbol,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("local symbol background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for local symbol index build",
        );
    }
}

#[cfg(test)]
pub(crate) async fn publish_local_symbol_hits(
    service: &SearchPlaneService,
    fingerprint: &str,
    hits: &[crate::gateway::studio::types::AstSearchHit],
) -> Result<(), LocalSymbolBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        fingerprint,
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Err(LocalSymbolBuildError::BuildRejected(
                fingerprint.to_string(),
            ));
        }
    };

    let plan = crate::search_plane::local_symbol::build::LocalSymbolBuildPlan {
        base_epoch: None,
        file_fingerprints: std::collections::BTreeMap::new(),
        partitions: std::collections::BTreeMap::from([(
            "manual".to_string(),
            crate::search_plane::local_symbol::build::LocalSymbolPartitionBuildPlan {
                replaced_paths: std::collections::BTreeSet::new(),
                changed_hits: hits.to_vec(),
            },
        )]),
    };

    match write_local_symbol_epoch(service, &lease, &plan).await {
        Ok(write) => {
            let prewarm_columns = crate::search_plane::local_symbol::schema::projected_columns();
            service
                .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                .await?;
            service.publish_ready_and_maintain(&lease, write.row_count, write.fragment_count);
            Ok(())
        }
        Err(error) => {
            service
                .coordinator()
                .fail_build(&lease, format!("local symbol epoch write failed: {error}"));
            Err(LocalSymbolBuildError::Storage(error))
        }
    }
}
