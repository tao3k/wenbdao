use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::attachment::build::AttachmentBuildPlan;
use crate::search_plane::attachment::build::extract::build_attachment_hits_for_files;
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, fingerprint_note_projects, scan_note_project_files,
};

const ATTACHMENT_EXTRACTOR_VERSION: u32 = 1;

pub(crate) fn plan_attachment_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> AttachmentBuildPlan {
    let scanned_files = scan_note_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    ATTACHMENT_EXTRACTOR_VERSION,
                    SearchCorpusKind::Attachment.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return AttachmentBuildPlan {
            base_epoch: None,
            file_fingerprints,
            replaced_paths: BTreeSet::new(),
            changed_hits: build_attachment_hits_for_files(
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

    AttachmentBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        replaced_paths,
        changed_hits: build_attachment_hits_for_files(
            project_root,
            config_root,
            projects,
            &changed_files,
        ),
    }
}

pub(crate) fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    fingerprint_note_projects(project_root, config_root, projects)
}
