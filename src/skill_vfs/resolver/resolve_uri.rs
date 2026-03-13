use std::path::PathBuf;

use super::super::{SkillVfsError, WendaoResourceUri};
use super::core::SkillVfsResolver;

impl SkillVfsResolver {
    /// Resolve one semantic URI to concrete file path.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when URI parsing fails, namespace is unknown,
    /// or no matching reference document exists.
    pub fn resolve_path(&self, uri: &str) -> Result<PathBuf, SkillVfsError> {
        let parsed = WendaoResourceUri::parse(uri)?;
        self.resolve_parsed_uri(&parsed)
    }

    /// Resolve one parsed URI to concrete file path.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when namespace is unknown or resource is missing.
    pub fn resolve_parsed_uri(&self, uri: &WendaoResourceUri) -> Result<PathBuf, SkillVfsError> {
        if uri.is_internal_skill() {
            // Internal skills are usually inside a folder named after semantic_name
            // then references/entity_name.
            for root in &self.internal_roots {
                let candidate = root
                    .join(uri.semantic_name())
                    .join("references")
                    .join(uri.entity_name());
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }

        let Some(path) = self.index.path_for_uri(uri).cloned() else {
            let Some(_mounts) = self.index.mounts_for(uri.semantic_name()) else {
                return Err(SkillVfsError::UnknownSemanticSkill {
                    semantic_name: uri.semantic_name().to_string(),
                });
            };
            return Err(SkillVfsError::ResourceNotFound {
                semantic_name: uri.semantic_name().to_string(),
                entity_name: uri.entity_name().to_string(),
            });
        };

        Ok(path)
    }
}
