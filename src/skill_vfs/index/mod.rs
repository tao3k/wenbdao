mod build;
mod preload;
mod semantic;

use std::collections::HashMap;
use std::path::PathBuf;

use preload::{preload_reference_dir_with_internal_flag, semantic_resource_uri_key};

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
        let key = semantic_resource_uri_key(
            uri.semantic_name(),
            uri.entity_name(),
            uri.is_internal_skill(),
        );
        self.paths_by_uri.get(key.as_str())
    }

    /// Index a single root directory into the namespace.
    ///
    /// # Errors
    /// Returns [`SkillVfsError`] if descriptor scanning or parsing fails.
    pub fn index_root(
        &mut self,
        root: &std::path::Path,
        is_internal: bool,
    ) -> Result<(), super::SkillVfsError> {
        let other = Self::build_from_roots_with_internal_flag(&[root.to_path_buf()], is_internal)?;
        for (name, mounts) in other.mounts_by_name {
            self.mounts_by_name.entry(name).or_default().extend(mounts);
        }
        for (uri, path) in other.paths_by_uri {
            self.paths_by_uri.insert(uri, path);
        }
        Ok(())
    }

    /// Return all unique semantic resource URIs currently indexed.
    #[must_use]
    pub fn all_uris(&self) -> Vec<String> {
        self.paths_by_uri.keys().cloned().collect()
    }

    /// Internal helper to preload references with a specific scheme flag.
    pub(in crate::skill_vfs::index) fn preload_references_for_semantic_with_internal_flag(
        &mut self,
        semantic_name: &str,
        is_internal: bool,
    ) {
        let Some(mounts) = self.mounts_by_name.get(semantic_name) else {
            return;
        };
        let references_roots = mounts
            .iter()
            .map(|mount| mount.references_dir.clone())
            .collect::<Vec<_>>();
        for references_dir in references_roots {
            preload_reference_dir_with_internal_flag(
                self,
                semantic_name,
                references_dir.as_path(),
                is_internal,
            );
        }
    }
}
