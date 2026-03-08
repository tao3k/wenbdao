use std::path::Path;

use walkdir::WalkDir;

use super::SkillNamespaceIndex;

pub(in crate::skill_vfs::index) fn preload_reference_dir(
    index: &mut SkillNamespaceIndex,
    semantic_name: &str,
    references_dir: &Path,
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
        let absolute = entry.into_path();
        let Ok(relative) = absolute.strip_prefix(references_dir) else {
            continue;
        };
        let Some(relative_entity) = normalize_relative_entity_path(relative) else {
            continue;
        };
        let uri_key = semantic_resource_uri_key(semantic_name, relative_entity.as_str());
        if index.paths_by_uri.contains_key(uri_key.as_str()) {
            continue;
        }
        index.paths_by_uri.insert(uri_key, absolute);
    }
}

fn normalize_relative_entity_path(path: &Path) -> Option<String> {
    let rendered = path.to_string_lossy().replace('\\', "/");
    let trimmed = rendered.trim_matches('/');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(in crate::skill_vfs::index) fn semantic_resource_uri_key(
    semantic_name: &str,
    entity_name: &str,
) -> String {
    format!(
        "wendao://skills/{}/references/{}",
        semantic_name.trim().to_ascii_lowercase(),
        entity_name.trim().trim_matches('/')
    )
}
