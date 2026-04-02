use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::gateway::studio::search::source_index::build_ast_hits_for_file;
use crate::gateway::studio::types::AstSearchHit;
use crate::search_plane::ProjectScannedFile;
use crate::search_plane::local_symbol::build::LocalSymbolPartitionBuildPlan;

pub(crate) fn build_partition_plans(
    project_root: &Path,
    files: &[ProjectScannedFile],
) -> BTreeMap<String, LocalSymbolPartitionBuildPlan> {
    let mut files_by_partition = BTreeMap::<String, Vec<ProjectScannedFile>>::new();
    for file in files {
        files_by_partition
            .entry(file.partition_id.clone())
            .or_default()
            .push(file.clone());
    }

    files_by_partition
        .into_iter()
        .map(|(partition_id, partition_files)| {
            (
                partition_id,
                LocalSymbolPartitionBuildPlan {
                    replaced_paths: BTreeSet::new(),
                    changed_hits: build_hits_for_files(project_root, partition_files.as_slice()),
                },
            )
        })
        .collect()
}

fn build_hits_for_files(project_root: &Path, files: &[ProjectScannedFile]) -> Vec<AstSearchHit> {
    let mut hits = Vec::new();
    for file in files {
        let mut file_hits = build_ast_hits_for_file(
            project_root,
            file.scan_root.as_path(),
            file.absolute_path.as_path(),
        );
        for hit in &mut file_hits {
            if file.project_name.is_some() {
                hit.project_name.clone_from(&file.project_name);
                hit.navigation_target
                    .project_name
                    .clone_from(&file.project_name);
            }
            if file.root_label.is_some() {
                hit.root_label.clone_from(&file.root_label);
                hit.navigation_target
                    .root_label
                    .clone_from(&file.root_label);
            }
        }
        hits.extend(file_hits);
    }
    hits
}
