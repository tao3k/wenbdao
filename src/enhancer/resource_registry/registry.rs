use std::collections::HashMap;

use include_dir::Dir;

use crate::enhancer::markdown_config::{
    extract_markdown_config_blocks, extract_markdown_config_link_targets_by_id,
};

use super::scan::{
    collect_embedded_markdown_files, is_wendao_uri, normalize_registry_key,
    semantic_skill_name_from_descriptor,
};
use super::semantic::semantic_lift_target;
use super::types::{
    MissingEmbeddedLink, WendaoResourceFile, WendaoResourceLinkTarget, WendaoResourceRegistry,
    WendaoResourceRegistryError,
};

pub(super) fn build_from_embedded(
    embedded: &Dir<'_>,
) -> Result<WendaoResourceRegistry, WendaoResourceRegistryError> {
    let mut registry = WendaoResourceRegistry::new();
    let mut markdown_files = Vec::new();
    collect_embedded_markdown_files(embedded, &mut markdown_files);
    markdown_files.sort_by(
        |left: &&include_dir::File<'_>, right: &&include_dir::File<'_>| {
            left.path().cmp(right.path())
        },
    );

    let mut missing_links: Vec<MissingEmbeddedLink> = Vec::new();

    for file in markdown_files {
        let relative_path = normalize_registry_key(file.path().to_string_lossy().as_ref());
        let Some(markdown) = file.contents_utf8() else {
            return Err(WendaoResourceRegistryError::InvalidUtf8 {
                path: relative_path,
            });
        };

        registry
            .config_index
            .extend(extract_markdown_config_blocks(markdown));

        let semantic_skill_name =
            semantic_skill_name_from_descriptor(relative_path.as_str(), markdown);
        let raw_link_targets = extract_markdown_config_link_targets_by_id(markdown, &relative_path);
        let mut link_targets_by_id: HashMap<String, Vec<WendaoResourceLinkTarget>> = HashMap::new();
        for (id, targets) in &raw_link_targets {
            let lifted = targets
                .iter()
                .map(|target| WendaoResourceLinkTarget {
                    target_path: semantic_lift_target(
                        target.target.as_str(),
                        relative_path.as_str(),
                        semantic_skill_name.as_deref(),
                    ),
                    reference_type: target.reference_type.clone(),
                })
                .collect::<Vec<_>>();
            link_targets_by_id.insert(id.clone(), lifted);
        }

        let mut links_by_id: HashMap<String, Vec<String>> = HashMap::new();
        for (id, targets) in &raw_link_targets {
            let lifted = targets
                .iter()
                .map(|target| {
                    semantic_lift_target(
                        target.target.as_str(),
                        relative_path.as_str(),
                        semantic_skill_name.as_deref(),
                    )
                })
                .collect::<Vec<_>>();
            links_by_id.insert(id.clone(), lifted);
        }

        for (id, targets) in &raw_link_targets {
            for target in targets {
                if is_wendao_uri(target.target.as_str()) {
                    continue;
                }
                if embedded.get_file(target.target.as_str()).is_none() {
                    missing_links.push(MissingEmbeddedLink {
                        source_path: relative_path.clone(),
                        id: id.clone(),
                        target_path: target.target.clone(),
                    });
                }
            }
        }

        registry.files_by_path.insert(
            relative_path.clone(),
            WendaoResourceFile {
                path: relative_path,
                links_by_id,
                link_targets_by_id,
            },
        );
    }

    if missing_links.is_empty() {
        Ok(registry)
    } else {
        Err(WendaoResourceRegistryError::MissingLinkedResources {
            count: missing_links.len(),
            missing: missing_links,
        })
    }
}
