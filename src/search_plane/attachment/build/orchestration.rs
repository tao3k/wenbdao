use std::path::Path;

use tokio::runtime::Handle;

use crate::gateway::studio::types::UiProjectConfig;
#[cfg(test)]
use crate::search_plane::attachment::build::AttachmentBuildError;
use crate::search_plane::attachment::build::{
    fingerprint_projects, plan_attachment_build, write_attachment_epoch,
};
use crate::search_plane::attachment::schema::projected_columns_with_hit_json;
use crate::search_plane::{BeginBuildDecision, SearchCorpusKind, SearchPlaneService};

pub(crate) fn ensure_attachment_index_started(
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
        SearchCorpusKind::Attachment,
        fingerprint,
        SearchCorpusKind::Attachment.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let active_epoch = service.corpus_active_epoch(SearchCorpusKind::Attachment);
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::Attachment)
                .await;
            let build: Result<_, tokio::task::JoinError> = tokio::task::spawn_blocking(move || {
                plan_attachment_build(
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
                    let write = write_attachment_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service
                            .coordinator()
                            .fail_build(&lease, format!("attachment epoch write failed: {error}"));
                        return;
                    }
                    let write = write.unwrap_or_else(|_| unreachable!());
                    let prewarm_columns = projected_columns_with_hit_json();
                    if let Err(error) = service
                        .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                        .await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("attachment epoch prewarm failed: {error}"),
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
                                SearchCorpusKind::Attachment,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("attachment background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for attachment index build",
        );
    }
}

#[cfg(test)]
pub(crate) async fn publish_attachments_from_projects(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    fingerprint: &str,
) -> Result<(), AttachmentBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::Attachment,
        fingerprint,
        SearchCorpusKind::Attachment.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Ok(());
        }
    };
    let plan = plan_attachment_build(
        project_root,
        config_root,
        projects,
        None,
        std::collections::BTreeMap::new(),
    );
    match write_attachment_epoch(service, &lease, &plan).await {
        Ok(write) => {
            let prewarm_columns = projected_columns_with_hit_json();
            service
                .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                .await?;
            service.publish_ready_and_maintain(&lease, write.row_count, write.fragment_count);
            Ok(())
        }
        Err(error) => {
            service
                .coordinator()
                .fail_build(&lease, format!("attachment epoch write failed: {error}"));
            Err(AttachmentBuildError::Storage(error))
        }
    }
}
