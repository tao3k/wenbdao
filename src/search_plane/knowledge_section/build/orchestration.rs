use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::knowledge_section::build::paths::fingerprint_projects;
use crate::search_plane::knowledge_section::build::rows::build_knowledge_section_rows_for_files;
use crate::search_plane::knowledge_section::build::types::KnowledgeSectionBuildPlan;
use crate::search_plane::knowledge_section::build::write::write_knowledge_section_epoch;
use crate::search_plane::knowledge_section::schema::projected_columns;
use crate::search_plane::{
    BeginBuildDecision, SearchCorpusKind, SearchFileFingerprint, SearchPlaneService,
    scan_note_project_files,
};
use tokio::runtime::Handle;

const KNOWLEDGE_SECTION_EXTRACTOR_VERSION: u32 = 1;

pub(crate) fn ensure_knowledge_section_index_started(
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
        SearchCorpusKind::KnowledgeSection,
        fingerprint,
        SearchCorpusKind::KnowledgeSection.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let active_epoch = service.corpus_active_epoch(SearchCorpusKind::KnowledgeSection);
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::KnowledgeSection)
                .await;
            let build: Result<KnowledgeSectionBuildPlan, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    plan_knowledge_section_build(
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
                    let write = write_knowledge_section_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service.coordinator().fail_build(
                            &lease,
                            format!("knowledge section epoch write failed: {error}"),
                        );
                        return;
                    }
                    let write = write.unwrap_or_else(|_| unreachable!());
                    let prewarm_columns = projected_columns();
                    if let Err(error) = service
                        .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                        .await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("knowledge section epoch prewarm failed: {error}"),
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
                                SearchCorpusKind::KnowledgeSection,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("knowledge section background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for knowledge section build",
        );
    }
}

pub(super) fn plan_knowledge_section_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> KnowledgeSectionBuildPlan {
    let scanned_files = scan_note_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    KNOWLEDGE_SECTION_EXTRACTOR_VERSION,
                    SearchCorpusKind::KnowledgeSection.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return KnowledgeSectionBuildPlan {
            base_epoch: None,
            file_fingerprints,
            replaced_paths: BTreeSet::new(),
            changed_rows: build_knowledge_section_rows_for_files(
                project_root,
                config_root,
                projects,
                &scanned_files,
            ),
        };
    }

    let changed_files = scanned_files
        .iter()
        .filter(|file| {
            previous_fingerprints.get(file.normalized_path.as_str())
                != file_fingerprints.get(file.normalized_path.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut replaced_paths = changed_files
        .iter()
        .map(|file| file.normalized_path.clone())
        .collect::<BTreeSet<_>>();
    for path in previous_fingerprints.keys() {
        if !file_fingerprints.contains_key(path) {
            replaced_paths.insert(path.clone());
        }
    }

    KnowledgeSectionBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        replaced_paths,
        changed_rows: build_knowledge_section_rows_for_files(
            project_root,
            config_root,
            projects,
            &changed_files,
        ),
    }
}
