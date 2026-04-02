use std::path::{Path, PathBuf};

use crate::link_graph::index::build::assemble::types::BuildInputs;
use crate::link_graph::index::build::constants::DEFAULT_EXCLUDED_DIR_NAMES;
use crate::link_graph::index::build::filters::{
    merge_excluded_dirs, normalize_include_dir, should_skip_entry,
};
use crate::link_graph::parser::is_supported_note;
use walkdir::WalkDir;

pub(crate) fn prepare_build_inputs(
    root_dir: &Path,
    include_dirs: &[String],
    excluded_dirs: &[String],
) -> Result<BuildInputs, String> {
    let root = root_dir
        .canonicalize()
        .map_err(|e| format!("invalid notebook root '{}': {e}", root_dir.display()))?;
    if !root.is_dir() {
        return Err(format!(
            "notebook root is not a directory: {}",
            root.display()
        ));
    }

    let normalized_include_dirs: Vec<String> = include_dirs
        .iter()
        .filter_map(|path| normalize_include_dir(path))
        .collect();
    let normalized_excluded_dirs: Vec<String> =
        merge_excluded_dirs(excluded_dirs, DEFAULT_EXCLUDED_DIR_NAMES);

    Ok(BuildInputs {
        root,
        included: normalized_include_dirs.iter().cloned().collect(),
        excluded: normalized_excluded_dirs.iter().cloned().collect(),
        normalized_include_dirs,
        normalized_excluded_dirs,
    })
}

pub(crate) fn collect_candidate_paths(inputs: &BuildInputs) -> Vec<PathBuf> {
    let mut candidate_paths = Vec::new();
    for entry in WalkDir::new(&inputs.root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !should_skip_entry(
                entry.path(),
                entry.file_type().is_dir(),
                &inputs.root,
                &inputs.included,
                &inputs.excluded,
            )
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file() || !is_supported_note(path) {
            continue;
        }
        candidate_paths.push(path.to_path_buf());
    }
    candidate_paths
}
