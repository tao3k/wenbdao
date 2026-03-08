mod build;
mod preload;
mod semantic;

use std::collections::HashMap;
use std::path::PathBuf;

use preload::{preload_reference_dir, semantic_resource_uri_key};

/// One mounted semantic namespace in skill VFS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillNamespaceMount {
    /// Semantic namespace from SKILL frontmatter `name`.
    pub semantic_name: String,
    /// Descriptor path that declared the namespace.
    pub skill_doc: PathBuf,
    /// Relative resource root (`references/`) for this namespace.
    pub references_dir: PathBuf,
}

/// In-memory semantic namespace index built from skill roots.
#[derive(Debug, Clone, Default)]
pub struct SkillNamespaceIndex {
    pub(in crate::skill_vfs::index) mounts_by_name: HashMap<String, Vec<SkillNamespaceMount>>,
    pub(in crate::skill_vfs::index) paths_by_uri: HashMap<String, PathBuf>,
}

impl SkillNamespaceIndex {
    /// Resolve all mounts by semantic namespace (case-insensitive).
    #[must_use]
    pub fn mounts_for(&self, semantic_name: &str) -> Option<&[SkillNamespaceMount]> {
        let key = semantic_name.trim().to_ascii_lowercase();
        self.mounts_by_name.get(&key).map(Vec::as_slice)
    }

    /// Returns total number of indexed semantic namespaces.
    #[must_use]
    pub fn namespace_count(&self) -> usize {
        self.mounts_by_name.len()
    }

    /// Resolve one concrete path from parsed semantic URI.
    #[must_use]
    pub fn path_for_uri(&self, uri: &super::WendaoResourceUri) -> Option<&PathBuf> {
        let key = semantic_resource_uri_key(uri.semantic_name(), uri.entity_name());
        self.paths_by_uri.get(key.as_str())
    }

    pub(in crate::skill_vfs::index) fn preload_references_for_semantic(
        &mut self,
        semantic_name: &str,
    ) {
        let Some(mounts) = self.mounts_by_name.get(semantic_name) else {
            return;
        };
        let references_roots = mounts
            .iter()
            .map(|mount| mount.references_dir.clone())
            .collect::<Vec<_>>();
        for references_dir in references_roots {
            preload_reference_dir(self, semantic_name, references_dir.as_path());
        }
    }
}
