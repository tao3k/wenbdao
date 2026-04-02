use crate::search_plane::SearchManifestKeyspace;

use super::config::SearchPlaneCacheConfig;
#[cfg(test)]
use super::tests::TestCacheShadow;
#[cfg(test)]
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub(crate) struct SearchPlaneCache {
    pub(crate) client: Option<redis::Client>,
    pub(crate) config: SearchPlaneCacheConfig,
    pub(crate) keyspace: SearchManifestKeyspace,
    #[cfg(test)]
    pub(crate) shadow: Arc<RwLock<TestCacheShadow>>,
}
