//! Ingress pipeline adapters for bringing external content into Wendao.

mod spider;
mod transmuter;

pub use spider::{
    ContentHashStore, InMemoryContentHashStore, KnowledgeGraphAssimilationSink,
    NoopPartialReindexHook, PartialReindexHook, SpiderIngressError, SpiderPagePayload,
    SpiderWendaoBridge, WebAssimilationSink, WebIngestionSignal, canonical_web_uri,
    web_namespace_from_url,
};
