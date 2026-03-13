use std::path::PathBuf;

use walkdir::WalkDir;
use xiuxian_skills::SkillScanner;

use super::semantic::{is_skill_descriptor, parse_semantic_name_from_skill_doc};
use super::{SkillNamespaceIndex, SkillNamespaceMount};
use crate::skill_vfs::SkillVfsError;

impl SkillNamespaceIndex {
    /// Build namespace index by scanning skill descriptor files under roots.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when descriptor I/O or frontmatter parsing fails.
    pub fn build_from_roots(roots: &[PathBuf]) -> Result<Self, SkillVfsError> {
        Self::build_from_roots_with_internal_flag(roots, false)
    }

    /// Build namespace index with an explicit internal-skill flag for reference URIs.
    ///
    /// # Errors
    /// Returns [`SkillVfsError`] when descriptor I/O or frontmatter parsing fails.
    pub fn build_from_roots_with_internal_flag(
        roots: &[PathBuf],
        is_internal: bool,
    ) -> Result<Self, SkillVfsError> {
        let mut index = Self::default();
        let scanner = SkillScanner::new();

        for root in roots {
            if !root.exists() || !root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
                if !entry.file_type().is_file() {
                    continue;
                }
                let skill_doc = entry.into_path();
                if !is_skill_descriptor(skill_doc.as_path()) {
                    continue;
                }
                let Some(semantic_name) =
                    parse_semantic_name_from_skill_doc(skill_doc.as_path(), &scanner)?
                else {
                    continue;
                };
                let references_dir = skill_doc
                    .parent()
                    .map_or_else(PathBuf::new, |parent| parent.join("references"));
                index
                    .mounts_by_name
                    .entry(semantic_name.clone())
                    .or_default()
                    .push(SkillNamespaceMount {
                        semantic_name: semantic_name.clone(),
                        skill_doc,
                        references_dir,
                    });

                // When building with internal flag, we need to pass it to preloader
                index.preload_references_for_semantic_with_internal_flag(
                    &semantic_name,
                    is_internal,
                );
            }
        }

        Ok(index)
    }
}
