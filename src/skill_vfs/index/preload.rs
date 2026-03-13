use std::path::Path;

use walkdir::WalkDir;

use super::SkillNamespaceIndex;

pub(super) fn preload_reference_dir_with_internal_flag(
    index: &mut SkillNamespaceIndex,
    semantic_name: &str,
    references_dir: &Path,
    is_internal: bool,
) {
    if !references_dir.exists() || !references_dir.is_dir() {
        return;
    }

    for entry in WalkDir::new(references_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let Ok(relative) = path.strip_prefix(references_dir) else {
            continue;
        };
        let entity_name = relative.to_string_lossy().replace('\\', "/");
        let key = semantic_resource_uri_key(semantic_name, &entity_name, is_internal);
        index.paths_by_uri.insert(key, path);
    }
}

pub(super) fn semantic_resource_uri_key(
    semantic_name: &str,
    entity_name: &str,
    is_internal: bool,
) -> String {
    let scheme = if is_internal {
        "wendao://skills-internal"
    } else {
        "wendao://skills"
    };
    format!(
        "{}/{}/references/{}",
        scheme,
        semantic_name.trim().to_ascii_lowercase(),
        entity_name.trim_start_matches('/')
    )
}
