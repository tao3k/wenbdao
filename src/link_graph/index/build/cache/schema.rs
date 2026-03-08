use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::OnceLock;

const LINK_GRAPH_VALKEY_CACHE_SCHEMA_JSON: &str = include_str!(
    "../../../../../resources/xiuxian_wendao.link_graph.valkey_cache_snapshot.v1.schema.json"
);
static LINK_GRAPH_CACHE_SCHEMA_FINGERPRINT: OnceLock<String> = OnceLock::new();
pub(in crate::link_graph::index::build) const LINK_GRAPH_VALKEY_CACHE_SCHEMA_VERSION: &str =
    "xiuxian_wendao.link_graph.valkey_cache_snapshot.v1";

pub(in crate::link_graph::index::build) fn cache_schema_fingerprint() -> &'static str {
    LINK_GRAPH_CACHE_SCHEMA_FINGERPRINT.get_or_init(|| {
        let mut hasher = DefaultHasher::new();
        LINK_GRAPH_VALKEY_CACHE_SCHEMA_JSON.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    })
}

pub(in crate::link_graph::index::build) fn cache_slot_key(
    root: &Path,
    include_dirs: &[String],
    excluded_dirs: &[String],
) -> String {
    let mut hasher = DefaultHasher::new();
    root.to_string_lossy().hash(&mut hasher);
    include_dirs.hash(&mut hasher);
    excluded_dirs.hash(&mut hasher);
    cache_schema_fingerprint().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
