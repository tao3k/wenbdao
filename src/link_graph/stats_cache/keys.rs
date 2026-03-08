use super::schema::LINK_GRAPH_STATS_CACHE_SCHEMA_VERSION;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn stats_cache_slot_key(source_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    source_key.hash(&mut hasher);
    LINK_GRAPH_STATS_CACHE_SCHEMA_VERSION.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(super) fn stats_cache_key(source_key: &str, key_prefix: &str) -> String {
    format!("{key_prefix}:stats:{}", stats_cache_slot_key(source_key))
}
