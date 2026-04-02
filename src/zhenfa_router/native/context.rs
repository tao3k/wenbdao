use std::sync::Arc;

use xiuxian_zhenfa::{ZhenfaContext, ZhenfaError};

use crate::{AssetRequest, LinkGraphIndex, SkillVfsResolver, WendaoAssetHandle};

/// Typed extension accessors for Wendao native tools.
pub trait WendaoContextExt {
    /// Resolve the injected immutable `LinkGraph` index from zhenfa context.
    ///
    /// # Errors
    /// Returns execution error when the index is not present in context.
    fn link_graph_index(&self) -> Result<Arc<LinkGraphIndex>, ZhenfaError>;

    /// Resolve the injected semantic skill VFS resolver from zhenfa context.
    ///
    /// # Errors
    /// Returns execution error when resolver is not present in context.
    fn vfs(&self) -> Result<Arc<SkillVfsResolver>, ZhenfaError>;

    /// Builds one skill-scoped asset request.
    ///
    /// # Errors
    /// Returns execution error when semantic URI mapping arguments are invalid.
    fn skill_asset(
        &self,
        semantic_name: &str,
        relative_path: &str,
    ) -> Result<AssetRequest, ZhenfaError>;
}

impl WendaoContextExt for ZhenfaContext {
    fn link_graph_index(&self) -> Result<Arc<LinkGraphIndex>, ZhenfaError> {
        self.get_extension::<LinkGraphIndex>().ok_or_else(|| {
            ZhenfaError::execution("missing LinkGraphIndex in zhenfa context extensions")
        })
    }

    fn vfs(&self) -> Result<Arc<SkillVfsResolver>, ZhenfaError> {
        self.get_extension::<SkillVfsResolver>().ok_or_else(|| {
            ZhenfaError::execution("missing SkillVfsResolver in zhenfa context extensions")
        })
    }

    fn skill_asset(
        &self,
        semantic_name: &str,
        relative_path: &str,
    ) -> Result<AssetRequest, ZhenfaError> {
        WendaoAssetHandle::skill_reference_asset(semantic_name, relative_path).map_err(|error| {
            ZhenfaError::invalid_arguments(format!(
                "invalid skill asset mapping (`{semantic_name}`, `{relative_path}`): {error}"
            ))
        })
    }
}
