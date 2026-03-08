use crate::link_graph::index::LinkGraphIndex;

mod schema;
mod snapshot;
mod storage;

pub(super) use schema::{
    LINK_GRAPH_VALKEY_CACHE_SCHEMA_VERSION, cache_schema_fingerprint, cache_slot_key,
};
pub(super) use storage::{load_cached_index_from_valkey, save_cached_index_to_valkey};

#[derive(Debug)]
pub(super) enum CacheLookupOutcome {
    Hit(Box<LinkGraphIndex>),
    Miss(&'static str),
}
