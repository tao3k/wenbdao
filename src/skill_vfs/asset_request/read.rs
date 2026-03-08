use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use super::types::AssetRequest;
use crate::skill_vfs::SkillVfsError;
use crate::skill_vfs::zhixing::embedded_resource_text_from_wendao_uri;

static STRIPPED_BODY_CACHE: OnceLock<RwLock<HashMap<String, Arc<str>>>> = OnceLock::new();

impl AssetRequest {
    /// Reads asset text using the caller-provided resolver callback.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when the callback
    /// returns `None` for this URI.
    pub fn read_utf8_with<F>(&self, resolver: F) -> Result<String, SkillVfsError>
    where
        F: Fn(&str) -> Option<String>,
    {
        resolver(self.uri())
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| SkillVfsError::EmbeddedAssetNotFound {
                uri: self.uri().to_string(),
            })
    }

    /// Reads UTF-8 asset text through Wendao's built-in embedded resolver.
    ///
    /// This method currently supports embedded skill assets that are resolvable
    /// by Wendao internal registries.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when no embedded asset
    /// exists for this URI.
    pub fn read_utf8(&self) -> Result<String, SkillVfsError> {
        embedded_resource_text_from_wendao_uri(self.uri())
            .map(str::to_string)
            .ok_or_else(|| SkillVfsError::EmbeddedAssetNotFound {
                uri: self.uri().to_string(),
            })
    }

    /// Reads UTF-8 asset text through Wendao's built-in embedded resolver and
    /// returns a shared immutable string.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when no embedded asset
    /// exists for this URI.
    pub fn read_utf8_shared(&self) -> Result<Arc<str>, SkillVfsError> {
        embedded_resource_text_from_wendao_uri(self.uri())
            .map(Arc::<str>::from)
            .ok_or_else(|| SkillVfsError::EmbeddedAssetNotFound {
                uri: self.uri().to_string(),
            })
    }

    /// Reads asset text and trims outer whitespace.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when resolver lookup
    /// fails or yields empty content.
    pub fn read_stripped_body_with<F>(&self, resolver: F) -> Result<String, SkillVfsError>
    where
        F: Fn(&str) -> Option<String>,
    {
        self.read_stripped_body_with_shared(resolver)
            .map(|text| text.as_ref().to_string())
    }

    /// Reads and strips one asset through Wendao's built-in embedded resolver.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when no embedded asset
    /// exists for this URI.
    pub fn read_stripped_body(&self) -> Result<String, SkillVfsError> {
        self.read_stripped_body_shared()
            .map(|text| text.as_ref().to_string())
    }

    /// Reads and strips one asset using the caller-provided resolver callback
    /// and returns a shared immutable string.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when resolver lookup
    /// fails or yields empty content.
    pub fn read_stripped_body_with_shared<F>(&self, resolver: F) -> Result<Arc<str>, SkillVfsError>
    where
        F: Fn(&str) -> Option<String>,
    {
        self.read_utf8_with(resolver).map(|text| {
            let stripped = text.trim();
            Arc::<str>::from(stripped)
        })
    }

    /// Reads and strips one asset through Wendao's built-in embedded resolver
    /// with process-level `Arc<str>` cache.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError::EmbeddedAssetNotFound`] when no embedded asset
    /// exists for this URI.
    pub fn read_stripped_body_shared(&self) -> Result<Arc<str>, SkillVfsError> {
        let cache = stripped_body_cache();
        if let Some(hit) = cache
            .read()
            .ok()
            .and_then(|entries| entries.get(self.uri()).cloned())
        {
            return Ok(hit);
        }

        let stripped = embedded_resource_text_from_wendao_uri(self.uri())
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(Arc::<str>::from)
            .ok_or_else(|| SkillVfsError::EmbeddedAssetNotFound {
                uri: self.uri().to_string(),
            })?;

        if let Ok(mut entries) = cache.write() {
            entries.insert(self.uri().to_string(), Arc::clone(&stripped));
        }
        Ok(stripped)
    }
}

fn stripped_body_cache() -> &'static RwLock<HashMap<String, Arc<str>>> {
    STRIPPED_BODY_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}
