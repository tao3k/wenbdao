#[path = "runtime_config/constants.rs"]
mod constants;
#[path = "runtime_config/models.rs"]
mod models;
#[path = "runtime_config/resolve/mod.rs"]
mod resolve;
#[path = "runtime_config/settings/mod.rs"]
mod settings;

pub(crate) use constants::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;
pub(crate) use models::LinkGraphCacheRuntimeConfig;
pub use models::LinkGraphIndexRuntimeConfig;
pub use resolve::resolve_link_graph_index_runtime;
pub(crate) use resolve::{
    resolve_link_graph_agentic_runtime, resolve_link_graph_cache_runtime,
    resolve_link_graph_related_runtime, resolve_link_graph_retrieval_policy_runtime,
};
pub use settings::{set_link_graph_config_home_override, set_link_graph_wendao_config_override};
