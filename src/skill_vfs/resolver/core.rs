use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use include_dir::Dir;

use super::super::{SkillNamespaceIndex, SkillVfsError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::skill_vfs::resolver) struct EmbeddedSemanticMount {
    pub(in crate::skill_vfs::resolver) crate_id: String,
    pub(in crate::skill_vfs::resolver) references_dir: PathBuf,
}

/// Semantic and internal-skill resource resolver for `wendao://...` addresses.
#[derive(Debug, Clone, Default)]
pub struct SkillVfsResolver {
    pub(in crate::skill_vfs::resolver) index: SkillNamespaceIndex,
    pub(in crate::skill_vfs::resolver) mounts: HashMap<String, &'static Dir<'static>>,
    pub(in crate::skill_vfs::resolver) embedded_mounts_by_semantic:
        HashMap<String, Vec<EmbeddedSemanticMount>>,
    pub(in crate::skill_vfs::resolver) internal_roots: Vec<PathBuf>,
    pub(in crate::skill_vfs::resolver) content_cache: Arc<DashMap<String, Arc<str>>>,
}

impl SkillVfsResolver {
    /// Build resolver by scanning one or more semantic skill roots.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace indexing fails.
    pub fn from_roots(roots: &[PathBuf]) -> Result<Self, SkillVfsError> {
        Ok(Self {
            index: SkillNamespaceIndex::build_from_roots(roots)?,
            mounts: HashMap::new(),
            embedded_mounts_by_semantic: HashMap::new(),
            internal_roots: Vec::new(),
            content_cache: Arc::new(DashMap::new()),
        })
    }

    /// Build resolver by scanning roots and mounting internal skill roots.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace indexing fails.
    pub fn from_roots_with_internal(
        roots: &[PathBuf],
        internal_roots: &[PathBuf],
    ) -> Result<Self, SkillVfsError> {
        Self::from_roots(roots).map(|resolver| resolver.with_internal_roots(internal_roots))
    }

    /// Build resolver by scanning semantic roots and enabling embedded mounts.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace indexing fails.
    pub fn from_roots_with_embedded(roots: &[PathBuf]) -> Result<Self, SkillVfsError> {
        Self::from_roots(roots).map(Self::mount_embedded_dir)
    }

    /// Build resolver by scanning roots, mounting internal skill roots, and enabling embedded mounts.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace indexing fails.
    pub fn from_roots_with_embedded_and_internal(
        roots: &[PathBuf],
        internal_roots: &[PathBuf],
    ) -> Result<Self, SkillVfsError> {
        Self::from_roots_with_internal(roots, internal_roots).map(Self::mount_embedded_dir)
    }

    /// Mount internal skill roots for `wendao://skills-internal/...` lookups.
    #[must_use]
    pub fn with_internal_roots(mut self, roots: &[PathBuf]) -> Self {
        self.internal_roots = normalize_internal_roots(roots);
        self
    }

    /// Access the underlying semantic namespace index.
    #[must_use]
    pub fn index(&self) -> &SkillNamespaceIndex {
        &self.index
    }

    /// Access the mounted internal skill roots.
    #[must_use]
    pub fn internal_roots(&self) -> &[PathBuf] {
        &self.internal_roots
    }
}

fn normalize_internal_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut normalized = Vec::new();
    for root in roots {
        if !root.exists() || !root.is_dir() {
            continue;
        }
        if !normalized.contains(root) {
            normalized.push(root.clone());
        }
    }
    normalized
}
