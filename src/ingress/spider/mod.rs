//! Spider-to-Wendao ingestion bridge.

mod bridge;
mod content;
mod errors;
mod hash_store;
mod locking;
mod reindex;
mod sink;
mod types;
mod url;

pub use bridge::SpiderWendaoBridge;
pub use errors::SpiderIngressError;
pub use hash_store::{ContentHashStore, InMemoryContentHashStore};
pub use reindex::{NoopPartialReindexHook, PartialReindexHook};
pub use sink::{KnowledgeGraphAssimilationSink, WebAssimilationSink};
pub use types::{SpiderPagePayload, WebIngestionSignal};
pub use url::{canonical_web_uri, web_namespace_from_url};

#[cfg(test)]
mod tests;
