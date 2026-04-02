use redis::AsyncConnectionConfig;

use crate::search_plane::SearchManifestKeyspace;
use crate::valkey_common::resolve_optional_client_from_env;

use super::config::SearchPlaneCacheConfig;
#[cfg(test)]
use super::tests::TestCacheShadow;
use super::types::SearchPlaneCache;
#[cfg(test)]
use std::sync::{Arc, RwLock};

const SEARCH_PLANE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL";
const KNOWLEDGE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL";
const VALKEY_URL_ENV: &str = "VALKEY_URL";
const REDIS_URL_ENV: &str = "REDIS_URL";

impl SearchPlaneCache {
    pub(crate) fn from_env(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(
            resolve_valkey_client(),
            SearchPlaneCacheConfig::from_env(),
            keyspace,
        )
    }

    pub(crate) fn disabled(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(None, SearchPlaneCacheConfig::default(), keyspace)
    }

    #[cfg(test)]
    pub(crate) fn for_tests(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(
            Some(
                redis::Client::open("redis://127.0.0.1/")
                    .unwrap_or_else(|error| panic!("client: {error}")),
            ),
            SearchPlaneCacheConfig::default(),
            keyspace,
        )
    }

    fn new(
        client: Option<redis::Client>,
        config: SearchPlaneCacheConfig,
        keyspace: SearchManifestKeyspace,
    ) -> Self {
        Self {
            client,
            config,
            keyspace,
            #[cfg(test)]
            shadow: Arc::new(RwLock::new(TestCacheShadow::default())),
        }
    }

    pub(crate) fn async_connection_config(&self) -> AsyncConnectionConfig {
        AsyncConnectionConfig::new()
            .set_connection_timeout(Some(self.config.connection_timeout))
            .set_response_timeout(Some(self.config.response_timeout))
    }
}

fn resolve_valkey_client() -> Option<redis::Client> {
    resolve_optional_client_from_env(&[
        SEARCH_PLANE_VALKEY_URL_ENV,
        KNOWLEDGE_VALKEY_URL_ENV,
        VALKEY_URL_ENV,
        REDIS_URL_ENV,
    ])
}
