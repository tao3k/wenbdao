//! Spider-to-Wendao ingestion bridge.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, PoisonError, RwLock};

use super::transmuter::resolve_and_wash;
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::sync::SyncEngine;
use crate::{Entity, EntityType, KnowledgeGraph, Relation, RelationType};

const DEFAULT_INGEST_LOCK_SEGMENTS: usize = 64;

/// Signal emitted after one web page has been assimilated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebIngestionSignal {
    /// Original web URL.
    pub url: String,
    /// Crawl depth used by upstream crawler.
    pub depth: u32,
    /// Stable content hash used for deduplication.
    pub content_hash: String,
}

/// Web page payload accepted by the Spider-to-Wendao bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiderPagePayload {
    /// Absolute source URL.
    pub url: String,
    /// Crawl depth of the source page.
    pub depth: u32,
    /// Optional title extracted by upstream crawler.
    pub title: Option<String>,
    /// Crawled content body (cleaned HTML or markdown-like payload).
    pub markdown_content: Arc<str>,
    /// Additional metadata from upstream crawler/runtime.
    pub metadata: HashMap<String, String>,
}

impl SpiderPagePayload {
    /// Construct payload with required fields.
    #[must_use]
    pub fn new(url: impl Into<String>, depth: u32, markdown_content: impl Into<Arc<str>>) -> Self {
        Self {
            url: url.into(),
            depth,
            title: None,
            markdown_content: markdown_content.into(),
            metadata: HashMap::new(),
        }
    }

    /// Attach optional title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Attach additional metadata map.
    #[must_use]
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Spider ingress pipeline errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SpiderIngressError {
    /// URL is not a valid absolute web URL.
    #[error("invalid web URL `{url}`")]
    InvalidWebUrl {
        /// Invalid URL value.
        url: String,
    },
    /// URL scheme is unsupported for ingress.
    #[error("unsupported web URL scheme `{scheme}` for `{url}`")]
    UnsupportedWebScheme {
        /// Input URL.
        url: String,
        /// Scheme extracted from URL.
        scheme: String,
    },
    /// Zhenfa transmutation failed.
    #[error("zhenfa transmutation failed for `{uri}`: {reason}")]
    TransmutationFailed {
        /// Canonical `wendao://web/...` URI.
        uri: String,
        /// Sanitized error message.
        reason: String,
    },
    /// Assimilation sink failed.
    #[error("web assimilation failed for `{uri}`: {reason}")]
    AssimilationFailed {
        /// Canonical `wendao://web/...` URI.
        uri: String,
        /// Sink error details.
        reason: String,
    },
    /// Partial re-index hook failed.
    #[error("partial re-index failed for namespace `{namespace}`: {reason}")]
    PartialReindexFailed {
        /// Namespace selected for refresh.
        namespace: String,
        /// Hook error details.
        reason: String,
    },
    /// Internal ingestion lock is poisoned.
    #[error("ingestion state lock poisoned")]
    StateLockPoisoned,
}

/// Deduplication store for crawled content hashes.
pub trait ContentHashStore: Send + Sync {
    /// Return true when hash is already present.
    fn contains_hash(&self, content_hash: &str) -> bool;
    /// Record a hash after successful assimilation.
    fn insert_hash(&self, content_hash: &str);
}

/// Thread-safe in-memory deduplication store.
#[derive(Debug, Default)]
pub struct InMemoryContentHashStore {
    hashes: RwLock<HashSet<String>>,
}

impl InMemoryContentHashStore {
    /// Construct an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContentHashStore for InMemoryContentHashStore {
    fn contains_hash(&self, content_hash: &str) -> bool {
        self.hashes
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(content_hash)
    }

    fn insert_hash(&self, content_hash: &str) {
        self.hashes
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(content_hash.to_string());
    }
}

/// Destination sink for washed web content.
pub trait WebAssimilationSink: Send + Sync {
    /// Persist one washed web payload into Wendao domain storage.
    ///
    /// # Errors
    ///
    /// Returns [`SpiderIngressError`] when persistence fails.
    fn assimilate(
        &self,
        canonical_uri: &str,
        washed_markdown: &str,
        signal: &WebIngestionSignal,
        title: Option<&str>,
        metadata: &HashMap<String, String>,
    ) -> Result<(), SpiderIngressError>;
}

/// Hook for scheduling namespace-level partial re-index.
pub trait PartialReindexHook: Send + Sync {
    /// Trigger one partial re-index for changed semantic URIs.
    ///
    /// # Errors
    ///
    /// Returns [`SpiderIngressError`] when scheduling fails.
    fn trigger_partial_reindex(
        &self,
        namespace: &str,
        changed_uris: &[String],
    ) -> Result<(), SpiderIngressError>;
}

/// No-op partial re-index hook.
#[derive(Debug, Default)]
pub struct NoopPartialReindexHook;

impl PartialReindexHook for NoopPartialReindexHook {
    fn trigger_partial_reindex(
        &self,
        _namespace: &str,
        _changed_uris: &[String],
    ) -> Result<(), SpiderIngressError> {
        Ok(())
    }
}

/// KnowledgeGraph-backed sink for web ingestion.
#[derive(Debug, Clone)]
pub struct KnowledgeGraphAssimilationSink {
    graph: KnowledgeGraph,
}

impl KnowledgeGraphAssimilationSink {
    /// Construct sink from existing graph handle.
    #[must_use]
    pub fn new(graph: KnowledgeGraph) -> Self {
        Self { graph }
    }
}

impl WebAssimilationSink for KnowledgeGraphAssimilationSink {
    fn assimilate(
        &self,
        canonical_uri: &str,
        washed_markdown: &str,
        signal: &WebIngestionSignal,
        title: Option<&str>,
        metadata: &HashMap<String, String>,
    ) -> Result<(), SpiderIngressError> {
        let namespace = web_namespace_from_url(signal.url.as_str())?;
        let cluster_id = format!("web-cluster:{namespace}");
        let cluster_name = format!("web://{namespace}");

        let cluster = Entity::new(
            cluster_id,
            cluster_name.clone(),
            EntityType::Concept,
            format!("Web namespace cluster for {namespace}"),
        )
        .with_source(Some("wendao://web".to_string()));
        self.graph
            .add_entity(cluster)
            .map_err(|error| SpiderIngressError::AssimilationFailed {
                uri: canonical_uri.to_string(),
                reason: error.to_string(),
            })?;

        let mut document = Entity::new(
            canonical_uri.to_string(),
            canonical_uri.to_string(),
            EntityType::Document,
            build_document_description(title, washed_markdown),
        )
        .with_source(Some(signal.url.clone()));
        document.metadata.insert(
            "wendao.uri".to_string(),
            Value::String(canonical_uri.to_string()),
        );
        document
            .metadata
            .insert("web.url".to_string(), Value::String(signal.url.clone()));
        document.metadata.insert(
            "web.content_hash".to_string(),
            Value::String(signal.content_hash.clone()),
        );
        document.metadata.insert(
            "web.depth".to_string(),
            Value::from(u64::from(signal.depth)),
        );
        if let Some(title) = title {
            document
                .metadata
                .insert("web.title".to_string(), Value::String(title.to_string()));
        }
        for (key, value) in metadata {
            document
                .metadata
                .insert(format!("web.meta.{key}"), Value::String(value.clone()));
        }
        self.graph.add_entity(document).map_err(|error| {
            SpiderIngressError::AssimilationFailed {
                uri: canonical_uri.to_string(),
                reason: error.to_string(),
            }
        })?;

        let relation = Relation::new(
            cluster_name,
            canonical_uri.to_string(),
            RelationType::Contains,
            "Web namespace contains crawled document".to_string(),
        )
        .with_source_doc(Some(canonical_uri.to_string()));
        self.graph.add_relation(&relation).map_err(|error| {
            SpiderIngressError::AssimilationFailed {
                uri: canonical_uri.to_string(),
                reason: error.to_string(),
            }
        })?;

        Ok(())
    }
}

/// Spider-to-Wendao bridge that enforces URI normalization, deduplication,
/// Zhenfa transmutation, assimilation, and partial re-index hooks.
pub struct SpiderWendaoBridge {
    content_hash_store: Arc<dyn ContentHashStore>,
    sink: Arc<dyn WebAssimilationSink>,
    reindex_hook: Arc<dyn PartialReindexHook>,
    ingest_locks: Vec<Mutex<()>>,
}

impl SpiderWendaoBridge {
    /// Construct a bridge from explicit extension points.
    #[must_use]
    pub fn new(
        content_hash_store: Arc<dyn ContentHashStore>,
        sink: Arc<dyn WebAssimilationSink>,
        reindex_hook: Arc<dyn PartialReindexHook>,
    ) -> Self {
        Self::with_lock_segments(
            content_hash_store,
            sink,
            reindex_hook,
            DEFAULT_INGEST_LOCK_SEGMENTS,
        )
    }

    /// Construct a bridge with explicit lock-striping width for ingestion.
    #[must_use]
    pub fn with_lock_segments(
        content_hash_store: Arc<dyn ContentHashStore>,
        sink: Arc<dyn WebAssimilationSink>,
        reindex_hook: Arc<dyn PartialReindexHook>,
        lock_segments: usize,
    ) -> Self {
        let lock_count = lock_segments.max(1);
        let mut ingest_locks = Vec::with_capacity(lock_count);
        ingest_locks.resize_with(lock_count, || Mutex::new(()));
        Self {
            content_hash_store,
            sink,
            reindex_hook,
            ingest_locks,
        }
    }

    /// Construct a bridge with in-memory dedup + `KnowledgeGraph` sink.
    #[must_use]
    pub fn for_knowledge_graph(graph: KnowledgeGraph) -> Self {
        Self::new(
            Arc::new(InMemoryContentHashStore::new()),
            Arc::new(KnowledgeGraphAssimilationSink::new(graph)),
            Arc::new(NoopPartialReindexHook),
        )
    }

    /// Assimilate one crawled web page into Wendao.
    ///
    /// Returns `Ok(None)` when payload is deduplicated by `content_hash`.
    ///
    /// # Errors
    ///
    /// Returns [`SpiderIngressError`] when URL normalization, transmutation,
    /// sink persistence, or partial re-index dispatch fails.
    pub fn ingest_page(
        &self,
        payload: &SpiderPagePayload,
    ) -> Result<Option<WebIngestionSignal>, SpiderIngressError> {
        let canonical_uri = canonical_web_uri(payload.url.as_str())?;
        let content_hash = SyncEngine::compute_hash(payload.markdown_content.as_ref());
        let lock_slot = lock_slot_for_hash(content_hash.as_str(), self.ingest_locks.len());

        let guard = self.ingest_locks[lock_slot]
            .lock()
            .map_err(|_| SpiderIngressError::StateLockPoisoned)?;
        if self.content_hash_store.contains_hash(content_hash.as_str()) {
            return Ok(None);
        }

        let washed_markdown =
            resolve_and_wash(canonical_uri.as_str(), payload.markdown_content.as_ref()).map_err(
                |error| SpiderIngressError::TransmutationFailed {
                    uri: canonical_uri.clone(),
                    reason: error.to_string(),
                },
            )?;
        let signal = WebIngestionSignal {
            url: payload.url.clone(),
            depth: payload.depth,
            content_hash: content_hash.clone(),
        };
        self.sink
            .assimilate(
                canonical_uri.as_str(),
                washed_markdown.as_str(),
                &signal,
                payload.title.as_deref(),
                &payload.metadata,
            )
            .map_err(|error| SpiderIngressError::AssimilationFailed {
                uri: canonical_uri.clone(),
                reason: error.to_string(),
            })?;
        self.content_hash_store.insert_hash(content_hash.as_str());
        drop(guard);

        let namespace = web_namespace_from_url(payload.url.as_str())?;
        self.reindex_hook
            .trigger_partial_reindex(namespace.as_str(), std::slice::from_ref(&canonical_uri))
            .map_err(|error| SpiderIngressError::PartialReindexFailed {
                namespace,
                reason: error.to_string(),
            })?;
        Ok(Some(signal))
    }
}

/// Normalize one absolute web URL into canonical Wendao URI:
/// `wendao://web/<absolute_url>`.
///
/// # Errors
///
/// Returns [`SpiderIngressError::InvalidWebUrl`] or
/// [`SpiderIngressError::UnsupportedWebScheme`] when URL cannot be normalized.
pub fn canonical_web_uri(url: &str) -> Result<String, SpiderIngressError> {
    let normalized = parse_absolute_web_url(url)?;
    Ok(format!("wendao://web/{normalized}"))
}

/// Resolve semantic namespace from absolute web URL host.
///
/// # Errors
///
/// Returns [`SpiderIngressError`] when `url` is not a valid absolute `http(s)` URL.
pub fn web_namespace_from_url(url: &str) -> Result<String, SpiderIngressError> {
    let parsed = parse_absolute_web_url(url)?;
    let parsed = Url::parse(parsed.as_str()).map_err(|_| SpiderIngressError::InvalidWebUrl {
        url: url.to_string(),
    })?;
    parsed
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| SpiderIngressError::InvalidWebUrl {
            url: url.to_string(),
        })
}

fn parse_absolute_web_url(url: &str) -> Result<String, SpiderIngressError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(SpiderIngressError::InvalidWebUrl {
            url: url.to_string(),
        });
    }

    let mut parsed = Url::parse(trimmed).map_err(|_| SpiderIngressError::InvalidWebUrl {
        url: trimmed.to_string(),
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(SpiderIngressError::UnsupportedWebScheme {
            url: trimmed.to_string(),
            scheme: parsed.scheme().to_string(),
        });
    }
    if parsed.host_str().is_none() {
        return Err(SpiderIngressError::InvalidWebUrl {
            url: trimmed.to_string(),
        });
    }

    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

fn build_document_description(title: Option<&str>, washed_markdown: &str) -> String {
    let first_non_empty = washed_markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Ingested web document");
    let snippet = first_non_empty.chars().take(220).collect::<String>();
    match title {
        Some(title) if !title.trim().is_empty() => format!("{title}: {snippet}"),
        _ => snippet,
    }
}

fn lock_slot_for_hash(content_hash: &str, lock_segments: usize) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content_hash.hash(&mut hasher);
    let segment_count = lock_segments.max(1);
    let segment_count_u64 = u64::try_from(segment_count).unwrap_or(u64::MAX);
    let slot_u64 = hasher.finish() % segment_count_u64;
    usize::try_from(slot_u64).unwrap_or_default()
}
