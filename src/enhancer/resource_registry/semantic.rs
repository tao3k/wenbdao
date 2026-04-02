use std::path::Path;

use crate::enhancer::resource_registry::scan::{is_wendao_uri, normalize_registry_key};

pub(crate) fn semantic_lift_target(
    target: &str,
    source_path: &str,
    semantic_skill_name: Option<&str>,
) -> String {
    if is_wendao_uri(target) {
        return target.to_string();
    }
    let Some(semantic_skill_name) = semantic_skill_name else {
        return target.to_string();
    };

    let source_parent = Path::new(source_path).parent();
    let Some(source_parent) = source_parent else {
        return target.to_string();
    };
    let references_dir = source_parent.join("references");
    let target_path = Path::new(target);
    let Ok(relative_entity) = target_path.strip_prefix(references_dir.as_path()) else {
        return target.to_string();
    };
    let normalized_entity = normalize_registry_key(relative_entity.to_string_lossy().as_ref());
    if normalized_entity.is_empty() {
        return target.to_string();
    }
    format!("wendao://skills/{semantic_skill_name}/references/{normalized_entity}")
}
