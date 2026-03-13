use std::sync::Arc;

use super::types::AssetRequest;
use crate::skill_vfs::{SkillVfsError, SkillVfsResolver};

impl AssetRequest {
    /// Resolve the asset and return UTF-8 text using the provided resolver.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when URI resolution fails or file read fails.
    pub fn read_utf8(&self, resolver: &SkillVfsResolver) -> Result<String, SkillVfsError> {
        let uri = self.uri();
        resolver.read_utf8(uri).map(|text| text.trim().to_string())
    }

    /// Resolve the asset and return shared UTF-8 text using the provided resolver.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when URI resolution fails or file read fails.
    pub fn read_utf8_shared(&self, resolver: &SkillVfsResolver) -> Result<Arc<str>, SkillVfsError> {
        resolver.read_utf8_shared(self.uri())
    }

    /// Convenience wrapper to read text and trim it.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when URI resolution fails or file read fails.
    pub fn read_trimmed(&self, resolver: &SkillVfsResolver) -> Result<String, SkillVfsError> {
        self.read_utf8(resolver).map(|text| text.trim().to_string())
    }
}
