use std::collections::BTreeMap;
use std::path::Path;

use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::local_symbol::build::LocalSymbolBuildPlan;
use crate::search_plane::local_symbol::build::partitions::build_partition_plans;
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, fingerprint_symbol_projects, scan_symbol_project_files,
};

const LOCAL_SYMBOL_EXTRACTOR_VERSION: u32 = 1;

pub(crate) fn plan_local_symbol_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> LocalSymbolBuildPlan {
    let scanned_files = scan_symbol_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    LOCAL_SYMBOL_EXTRACTOR_VERSION,
                    SearchCorpusKind::LocalSymbol.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return LocalSymbolBuildPlan {
            base_epoch: None,
            file_fingerprints,
            partitions: build_partition_plans(project_root, scanned_files.as_slice()),
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
    let mut partitions = build_partition_plans(project_root, changed_files.as_slice());
    for file in &changed_files {
        partitions
            .entry(file.partition_id.clone())
            .or_default()
            .replaced_paths
            .insert(file.normalized_path.clone());
    }
    for (path, previous_fingerprint) in &previous_fingerprints {
        let current_fingerprint = file_fingerprints.get(path.as_str());
        if current_fingerprint.is_none() {
            if let Some(partition_id) = previous_fingerprint.partition_id.as_deref() {
                partitions
                    .entry(partition_id.to_string())
                    .or_default()
                    .replaced_paths
                    .insert(path.clone());
            }
            continue;
        }

        if let Some(current_fingerprint) = current_fingerprint
            && current_fingerprint.partition_id != previous_fingerprint.partition_id
            && let Some(partition_id) = previous_fingerprint.partition_id.as_deref()
        {
            partitions
                .entry(partition_id.to_string())
                .or_default()
                .replaced_paths
                .insert(path.clone());
        }
    }

    LocalSymbolBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        partitions,
    }
}

pub(crate) fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    fingerprint_symbol_projects(project_root, config_root, projects)
}
