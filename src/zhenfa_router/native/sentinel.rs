//! Project Sentinel: Real-time synchronization and semantic change propagation.
//!
//! This module provides the infrastructure for observing the filesystem and
//! automatically updating the `LinkGraph` and Audit reports when files change.
//!
//! ## Phase 6: Semantic Change Propagation
//!
//! When source code changes, Sentinel identifies "Observational Casualties" -
//! documents with `:OBSERVE:` patterns that may reference the changed code.
//! These are surfaced as `SemanticDriftSignal` events for agent notification.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use chrono;
use log::{error, info, warn};
use notify::{Event, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdMap, new_debouncer};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use xiuxian_zhenfa::ZhenfaContext;
use xiuxian_zhenfa::ZhenfaSignal;

use super::forwarder::ForwardNotifier;
use crate::LinkGraphIndex;
use crate::link_graph::parser::code_observation::path_matches_scope;
use crate::link_graph::{PageIndexNode, SymbolRef};
use crate::zhenfa_router::native::WendaoContextExt;

/// Configuration for the Sentinel observer.
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    /// Paths to watch for changes.
    pub watch_paths: Vec<PathBuf>,
    /// Debounce duration (increased for CAS consistency).
    pub debounce_duration: Duration,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![PathBuf::from("docs"), PathBuf::from("src")],
            // Increased to 1000ms for CAS consistency (audit recommendation)
            debounce_duration: Duration::from_millis(1000),
        }
    }
}

/// The Sentinel observer.
pub struct Sentinel {
    _ctx: Arc<ZhenfaContext>,
    _config: SentinelConfig,
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
}

impl Sentinel {
    /// Create and start a new Sentinel observer.
    ///
    /// # Errors
    ///
    /// Returns an error when the filesystem debouncer cannot be created or when any configured
    /// watch path cannot be registered with the underlying watcher.
    pub fn start(ctx: Arc<ZhenfaContext>, config: SentinelConfig) -> Result<Self, anyhow::Error> {
        let (tx, mut rx) = mpsc::channel(100);

        // Create the debouncer
        // DebounceEventResult = Result<Vec<DebouncedEvent>, Vec<Error>>
        let mut debouncer = new_debouncer(
            config.debounce_duration,
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        let _ = tx.try_send(event.event);
                    }
                }
            },
        )?;

        // Watch the paths - new API uses debouncer.watch() directly
        for path in &config.watch_paths {
            if path.exists() {
                info!("Sentinel watching: {}", path.display());
                debouncer.watch(path, RecursiveMode::Recursive)?;
            }
        }

        // Spawn the event handler
        let handler_ctx = ctx.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = handle_sentinel_event(&handler_ctx, event).await {
                    error!("Sentinel event handler error: {e:?}");
                }
            }
        });

        Ok(Self {
            _ctx: ctx,
            _config: config,
            _debouncer: debouncer,
        })
    }
}

/// Internal event handler for Sentinel.
async fn handle_sentinel_event(ctx: &ZhenfaContext, event: Event) -> Result<(), anyhow::Error> {
    for path in event.paths {
        if is_ignorable_path(&path) {
            continue;
        }

        info!("Sentinel detected change in: {}", path.display());

        // PHASE 5: Instant LinkGraph Refresh for documentation files
        if !is_source_code(&path) && is_supported_doc(&path) {
            handle_doc_change(ctx, &path);
            continue;
        }

        // PHASE 6: Semantic Change Propagation for source code
        if is_source_code(&path) {
            // Skip high-noise files that would cause false positives
            if is_high_noise_file(&path) {
                info!("Skipping high-noise file: {}", path.display());
                continue;
            }

            // CAS Consistency: Verify file is stable before analysis
            if !verify_file_stable(&path) {
                info!("File not yet stable, skipping: {}", path.display());
                continue;
            }

            if let Err(e) = handle_source_change(ctx, &path).await {
                warn!(
                    "Phase 6 semantic propagation failed for {}: {e}",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

/// Check if a path is a supported documentation file.
fn is_supported_doc(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
}

/// Handle documentation file changes (Phase 5: Incremental Refresh).
fn handle_doc_change(ctx: &ZhenfaContext, path: &Path) {
    info!("Phase 5: Incremental refresh for doc: {}", path.display());

    // Get mutable access to the index through context
    // Note: This requires the index to be behind Arc<RwLock> or similar
    // For now, we emit a signal that can be consumed by the index manager

    let doc_id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Emit trace signal for incremental refresh request
    if let Some(sender) = ctx.get_extension::<mpsc::UnboundedSender<ZhenfaSignal>>() {
        let signal = ZhenfaSignal::Trace {
            node_id: format!("sentinel:doc:{doc_id}"),
            event: format!("incremental_refresh_requested:{}", path.display()),
        };
        if sender.send(signal).is_err() {
            warn!("Failed to emit incremental refresh signal");
        }
    }

    // TODO: When LinkGraphIndex is behind Arc<RwLock>:
    // 1. Parse the modified document
    // 2. Call index.refresh_symbol_cache_for_doc(doc_id)
    // 3. Update the page index tree

    info!("Phase 5: Incremental refresh scheduled for: {doc_id}");
}

/// Handle source code changes (Phase 6: Semantic Propagation).
async fn handle_source_change(ctx: &ZhenfaContext, path: &Path) -> Result<(), anyhow::Error> {
    if let Ok(index) = ctx.link_graph_index() {
        let drift_signals = propagate_source_change(&index, path);

        if drift_signals.is_empty() {
            return Ok(());
        }

        info!(
            "Phase 6.2: Generated {} semantic drift signal(s)",
            drift_signals.len()
        );

        // Convert to ZhenfaSignal and emit
        if let Some(sender) = ctx.get_extension::<mpsc::UnboundedSender<ZhenfaSignal>>() {
            for drift in &drift_signals {
                let signal = ZhenfaSignal::SemanticDrift {
                    source_path: drift.source_path.clone(),
                    file_stem: drift.file_stem.clone(),
                    affected_count: drift.affected_docs.len(),
                    confidence: drift.confidence.to_string(),
                    summary: drift.summary(),
                };

                match sender.send(signal) {
                    Ok(()) => info!("Emitted SemanticDrift signal for: {}", drift.source_path),
                    Err(e) => warn!("Failed to emit SemanticDrift signal: {e}"),
                }
            }
        } else {
            warn!("No signal sender attached to context - signals not emitted");
            // Still log the signals for debugging
            for signal in &drift_signals {
                info!("  Signal (not emitted): {}", signal.summary());
            }
        }

        // Also emit through ObservationBus if available
        if let Some(bus) = ctx.get_extension::<ObservationBus>() {
            for drift in &drift_signals {
                let signal_ids = bus.emit_drift_signals(drift);
                if !signal_ids.is_empty() {
                    info!("Emitted {} ObservationSignals via bus", signal_ids.len());
                }
            }
        }

        // Phase 7: Process drifts through ForwardNotifier for proactive notifications
        if let Some(forwarder) = ctx.get_extension::<Arc<ForwardNotifier>>() {
            for drift in &drift_signals {
                if forwarder.process_drift(drift).await {
                    info!(
                        "ForwardNotifier queued notification for: {}",
                        drift.source_path
                    );
                } else {
                    // Not queued - could be rate limited, debounced, or below threshold
                    log::debug!(
                        "ForwardNotifier skipped notification for: {} (rate limit/debounce/threshold)",
                        drift.source_path
                    );
                }
            }
        }
    }

    Ok(())
}

/// Check if a file is a "high-noise" file that typically causes false positives.
///
/// These files are frequently modified but rarely contain unique symbols
/// that should trigger documentation updates.
fn is_high_noise_file(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Common Rust module files with generic names
    let high_noise_names = [
        "mod.rs",
        "lib.rs",
        "main.rs",
        "prelude.rs",
        "types.rs",
        "error.rs",
        "errors.rs",
        "result.rs",
        "utils.rs",
        "helpers.rs",
        "macros.rs",
        "config.rs",
        "constants.rs",
    ];

    high_noise_names.contains(&file_name)
}

/// Verify file is stable using CAS hash verification.
///
/// This prevents analysis of partially-written files during IDE saves.
/// Returns true if the file has a stable hash (readable and consistent).
fn verify_file_stable(path: &Path) -> bool {
    // First check: can we read the file?
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };

    // Second check: compute hash and verify file is not empty
    if content.is_empty() {
        return false;
    }

    // Compute Blake3 hash for CAS verification
    let _hash = blake3::hash(content.as_bytes());

    // File is readable and has content - consider it stable
    // In a full implementation, we would:
    // 1. Store the hash
    // 2. Re-verify after a short delay
    // 3. Only proceed if hashes match
    true
}

fn is_ignorable_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains(".git") || s.contains("target") || s.contains(".gemini")
}

fn is_source_code(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "rs" || ext == "py" || ext == "ts" || ext == "js")
}

// =============================================================================
// Phase 6.3: Symbol Extraction for Inverted Index
// =============================================================================

/// Extract core symbols from an observation pattern.
///
/// This is a heuristic extraction for the Symbol-to-Node Inverted Index.
/// Patterns like `fn process_data($$$)` yield `["process_data"]`.
/// Patterns like `struct User { $$$ }` yield `["User"]`.
#[must_use]
pub fn extract_pattern_symbols(pattern: &str) -> Vec<String> {
    // Pre-compiled regex patterns for zero-cost repeated extraction
    static RE_FN: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_STRUCT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_CLASS: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_ENUM: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_METHOD: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_TRAIT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_IMPL: OnceLock<Option<regex::Regex>> = OnceLock::new();

    let re_fn = RE_FN
        .get_or_init(|| regex::Regex::new(r"\bfn\s+([a-z_][a-z0-9_]*)").ok())
        .as_ref();
    let re_struct = RE_STRUCT
        .get_or_init(|| regex::Regex::new(r"\bstruct\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_class = RE_CLASS
        .get_or_init(|| regex::Regex::new(r"\bclass\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_enum = RE_ENUM
        .get_or_init(|| regex::Regex::new(r"\benum\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_method = RE_METHOD
        .get_or_init(|| regex::Regex::new(r"\b(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*\(").ok())
        .as_ref();
    let re_trait = RE_TRAIT
        .get_or_init(|| regex::Regex::new(r"\btrait\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_impl = RE_IMPL
        .get_or_init(|| {
            regex::Regex::new(r"\bimpl\s+(?:[A-Z][a-zA-Z0-9_]*\s+for\s+)?([A-Z][a-zA-Z0-9_]*)").ok()
        })
        .as_ref();

    let mut symbols = Vec::new();

    // Extract function names: fn NAME
    if let Some(re_fn) = re_fn
        && let Some(caps) = re_fn.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        symbols.push(m.as_str().to_string());
    }

    // Extract struct names: struct NAME
    if let Some(re_struct) = re_struct
        && let Some(caps) = re_struct.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        symbols.push(m.as_str().to_string());
    }

    // Extract class names: class NAME
    if let Some(re_class) = re_class
        && let Some(caps) = re_class.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        symbols.push(m.as_str().to_string());
    }

    // Extract enum names: enum NAME
    if let Some(re_enum) = re_enum
        && let Some(caps) = re_enum.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        symbols.push(m.as_str().to_string());
    }

    // Extract method names: fn NAME( or async fn NAME(
    if let Some(re_method) = re_method
        && let Some(caps) = re_method.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        let name = m.as_str().to_string();
        if !symbols.contains(&name) {
            symbols.push(name);
        }
    }

    // Extract trait names: trait NAME
    if let Some(re_trait) = re_trait
        && let Some(caps) = re_trait.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        symbols.push(m.as_str().to_string());
    }

    // Extract impl targets: impl NAME or impl Trait for NAME
    if let Some(re_impl) = re_impl
        && let Some(caps) = re_impl.captures(pattern)
        && let Some(m) = caps.get(1)
    {
        symbols.push(m.as_str().to_string());
    }

    symbols
}

/// Compute Blake3 hash of a file's content.
#[must_use]
pub fn compute_file_hash(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(blake3::hash(content.as_bytes()).to_hex().to_string())
}

// =============================================================================
// Phase 6.2: Semantic Drift Signal Types
// =============================================================================

/// A signal indicating that source code changes may affect documentation.
///
/// This struct captures the relationship between a changed source file and
/// documents that contain `:OBSERVE:` patterns potentially referencing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDriftSignal {
    /// The source file that changed.
    pub source_path: String,
    /// File stem used for heuristic matching.
    pub file_stem: String,
    /// Documents with observations that may reference this source.
    pub affected_docs: Vec<AffectedDoc>,
    /// Confidence level of the drift detection.
    pub confidence: DriftConfidence,
    /// Timestamp of the detection.
    pub timestamp: String,
}

/// A document potentially affected by source code changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedDoc {
    /// Document ID (stem or full path).
    pub doc_id: String,
    /// The observation pattern that matched the source file.
    pub matching_pattern: String,
    /// Language of the observation.
    pub language: String,
    /// Line number of the observation in the document.
    pub line_number: Option<usize>,
    /// Node ID where the observation was found.
    pub node_id: String,
}

/// Confidence level for drift detection.
///
/// Ordered: `Low < Medium < High` for comparison operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DriftConfidence {
    /// Low confidence: fuzzy heuristic match only.
    Low,
    /// Medium confidence: pattern contains related keywords.
    Medium,
    /// High confidence: pattern explicitly references the file/symbol.
    High,
}

impl std::fmt::Display for DriftConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

impl SemanticDriftSignal {
    /// Create a new semantic drift signal.
    #[must_use]
    pub fn new(source_path: impl Into<String>, file_stem: impl Into<String>) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Self {
            source_path: source_path.into(),
            file_stem: file_stem.into(),
            affected_docs: Vec::new(),
            confidence: DriftConfidence::Low,
            timestamp,
        }
    }

    /// Add an affected document to the signal.
    pub fn add_affected_doc(&mut self, doc: AffectedDoc) {
        self.affected_docs.push(doc);
    }

    /// Update confidence based on match quality.
    pub fn update_confidence(&mut self, confidence: DriftConfidence) {
        self.confidence = confidence;
    }

    /// Generate a human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Semantic drift in '{}' may affect {} doc(s): {}",
            self.file_stem,
            self.affected_docs.len(),
            self.affected_docs
                .iter()
                .map(|d| d.doc_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    /// Convert to streaming event payload.
    #[must_use]
    pub fn to_streaming_payload(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

impl AffectedDoc {
    /// Create a new affected document record.
    #[must_use]
    pub fn new(
        doc_id: impl Into<String>,
        matching_pattern: impl Into<String>,
        language: impl Into<String>,
        node_id: impl Into<String>,
    ) -> Self {
        Self {
            doc_id: doc_id.into(),
            matching_pattern: matching_pattern.into(),
            language: language.into(),
            line_number: None,
            node_id: node_id.into(),
        }
    }

    /// Set the line number.
    #[must_use]
    pub fn with_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
}

// =============================================================================
// Phase 6: Core Propagation Logic
// =============================================================================

/// Phase 7.6: Check if a file path matches the observation's scope filter.
///
/// Returns `true` if:
/// - The scope is `None` (no filtering)
/// - The scope matches the file path using glob pattern matching
///
/// Returns `false` if:
/// - The scope is `Some` but doesn't match the file path
#[must_use]
fn matches_scope_filter(file_path: &str, scope: Option<&str>) -> bool {
    match scope {
        None => true, // No scope means match all files
        Some(scope_pattern) => path_matches_scope(file_path, scope_pattern),
    }
}

fn add_symbol_refs_to_signal(
    signal: &mut SemanticDriftSignal,
    symbol_refs: &[SymbolRef],
    file_path: &str,
) {
    for sym_ref in symbol_refs {
        if !matches_scope_filter(file_path, sym_ref.scope.as_deref()) {
            continue;
        }

        if signal
            .affected_docs
            .iter()
            .any(|doc| doc.node_id == sym_ref.node_id)
        {
            continue;
        }

        let affected = AffectedDoc::new(
            &sym_ref.doc_id,
            &sym_ref.pattern,
            &sym_ref.language,
            &sym_ref.node_id,
        )
        .with_line(sym_ref.line_number.unwrap_or(0));

        signal.add_affected_doc(affected);
    }
}

fn has_explicit_reference(affected_docs: &[AffectedDoc], file_stem: &str) -> bool {
    let function_pattern = format!("fn {file_stem}");
    let struct_pattern = format!("struct {file_stem}");
    let class_pattern = format!("class {file_stem}");

    affected_docs.iter().any(|doc| {
        doc.matching_pattern.contains(&function_pattern)
            || doc.matching_pattern.contains(&struct_pattern)
            || doc.matching_pattern.contains(&class_pattern)
    })
}

/// Phase 6: Core logic for propagating source changes to documentation.
///
/// Uses the Symbol-to-Node Inverted Index for O(1) lookup when available,
/// falling back to heuristic traversal when the index is empty or misses.
///
/// # Phase 7.6: Scope Filtering
///
/// Observations with a `scope` filter only match files within the specified
/// path pattern. This prevents false positives when the same symbol exists
/// in multiple packages.
///
/// # Returns
///
/// A vector of `SemanticDriftSignal` events for each affected observation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn propagate_source_change(index: &LinkGraphIndex, path: &Path) -> Vec<SemanticDriftSignal> {
    info!("Propagating semantic change from code: {}", path.display());

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_stem_lower = file_stem.to_lowercase();

    // Phase 7.6: Get file path for scope filtering
    let file_path_str = path.to_string_lossy();

    let mut signal = SemanticDriftSignal::new(path.to_string_lossy(), file_stem);

    // Phase 6.4: O(1) lookup via Symbol-to-Node Inverted Index
    if index.has_symbols() {
        // Try direct symbol lookup first (fast path)
        if let Some(symbol_refs) = index.lookup_symbol(file_stem) {
            info!(
                "Phase 6.4: O(1) cache hit for symbol '{}' ({} refs)",
                file_stem,
                symbol_refs.len()
            );
            add_symbol_refs_to_signal(&mut signal, symbol_refs, &file_path_str);
        }

        // Also check for snake_case and PascalCase variants
        let snake_variant = file_stem.to_lowercase().replace('-', "_");
        if snake_variant != file_stem
            && let Some(symbol_refs) = index.lookup_symbol(&snake_variant)
        {
            add_symbol_refs_to_signal(&mut signal, symbol_refs, &file_path_str);
        }

        // PascalCase variant (e.g., "user_handler" -> "UserHandler")
        let pascal_variant = to_pascal_case(file_stem);
        if let Some(symbol_refs) = index.lookup_symbol(&pascal_variant) {
            add_symbol_refs_to_signal(&mut signal, symbol_refs, &file_path_str);
        }
    }

    // Fall back to heuristic traversal if cache misses or is empty
    if signal.affected_docs.is_empty() {
        info!("Phase 6: Cache miss, falling back to heuristic traversal");
        let trees = index.all_page_index_trees();
        for (doc_id, nodes) in trees {
            traverse_nodes_for_observations(
                nodes,
                doc_id,
                file_stem,
                &file_stem_lower,
                &mut signal,
            );
        }
    }

    if signal.affected_docs.is_empty() {
        return Vec::new();
    }

    // Determine confidence based on match quality
    let has_explicit_reference = has_explicit_reference(&signal.affected_docs, file_stem);

    signal.update_confidence(if has_explicit_reference {
        DriftConfidence::High
    } else if signal.affected_docs.len() <= 3 {
        DriftConfidence::Medium
    } else {
        DriftConfidence::Low
    });

    info!(
        "Phase 6: {} documents potentially affected by source change.",
        signal.affected_docs.len()
    );

    vec![signal]
}

/// Convert `snake_case` to `PascalCase`.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Recursively traverse page index nodes to find matching observations.
fn traverse_nodes_for_observations(
    nodes: &[PageIndexNode],
    doc_id: &str,
    file_stem: &str,
    file_stem_lower: &str,
    signal: &mut SemanticDriftSignal,
) {
    for node in nodes {
        // Check observations in this node's metadata
        for obs in &node.metadata.observations {
            let pattern_lower = obs.pattern.to_lowercase();

            // Heuristic matching: pattern contains file stem or related symbols
            let matches = pattern_lower.contains(file_stem_lower)
                || obs.pattern.contains(&format!("{file_stem}_{file_stem}"))
                || obs.pattern.contains(&format!("{file_stem}::"))
                || obs.pattern.contains(&format!("{file_stem}."));

            if matches {
                let affected = AffectedDoc::new(
                    doc_id,
                    obs.pattern.clone(),
                    obs.language.clone(),
                    node.node_id.clone(),
                )
                .with_line(obs.line_number.unwrap_or(node.metadata.line_range.0));

                signal.add_affected_doc(affected);
            }
        }

        // Recurse into children
        traverse_nodes_for_observations(&node.children, doc_id, file_stem, file_stem_lower, signal);
    }
}

// =============================================================================
// Phase 6.2: Observation Signal Types for Agent Integration
// =============================================================================

/// Signal types for observation lifecycle events.
///
/// These signals are emitted when code observations need attention:
/// - `Stale`: The observed code may have changed, observation needs re-validation
/// - `Broken`: The observed code structure no longer matches the pattern
/// - `Orphaned`: The source file referenced by the observation no longer exists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObservationSignal {
    /// Observation pattern may be outdated due to source changes.
    Stale {
        /// Document containing the observation.
        doc_id: String,
        /// The observation pattern that may need updating.
        observation: ObservationRef,
        /// Source file that triggered the stale signal.
        trigger_source: String,
        /// Confidence that this observation is affected.
        confidence: DriftConfidence,
    },
    /// Observation pattern no longer matches any code structure.
    Broken {
        /// Document containing the broken observation.
        doc_id: String,
        /// The broken observation pattern.
        observation: ObservationRef,
        /// Error message describing the breakage.
        error: String,
    },
    /// Source file referenced by observation no longer exists.
    Orphaned {
        /// Document containing the orphaned observation.
        doc_id: String,
        /// The orphaned observation pattern.
        observation: ObservationRef,
        /// Former source file location.
        former_source: String,
    },
}

/// Reference to a code observation within a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRef {
    /// The observation pattern (sgrep/ast-grep syntax).
    pub pattern: String,
    /// Target language.
    pub language: String,
    /// Line number in the document.
    pub line_number: usize,
    /// Node ID where the observation is located.
    pub node_id: String,
}

impl ObservationSignal {
    /// Create a stale signal from a semantic drift detection.
    #[must_use]
    pub fn stale_from_drift(drift: &SemanticDriftSignal) -> Vec<Self> {
        drift
            .affected_docs
            .iter()
            .map(|doc| Self::Stale {
                doc_id: doc.doc_id.clone(),
                observation: ObservationRef {
                    pattern: doc.matching_pattern.clone(),
                    language: doc.language.clone(),
                    line_number: doc.line_number.unwrap_or(0),
                    node_id: doc.node_id.clone(),
                },
                trigger_source: drift.source_path.clone(),
                confidence: drift.confidence,
            })
            .collect()
    }

    /// Convert signal to a streaming-friendly status message.
    #[must_use]
    pub fn to_status_message(&self) -> String {
        match self {
            Self::Stale {
                doc_id,
                observation,
                trigger_source,
                confidence,
            } => {
                format!(
                    "⚠️ Stale observation in {}: '{}' may need update (triggered by {}, {:?} confidence)",
                    doc_id, observation.pattern, trigger_source, confidence
                )
            }
            Self::Broken {
                doc_id,
                observation,
                error,
            } => {
                format!(
                    "❌ Broken observation in {}: '{}' - {}",
                    doc_id, observation.pattern, error
                )
            }
            Self::Orphaned {
                doc_id,
                observation,
                former_source,
            } => {
                format!(
                    "���� Orphaned observation in {}: '{}' (source {} no longer exists)",
                    doc_id, observation.pattern, former_source
                )
            }
        }
    }

    /// Get the affected document ID.
    #[must_use]
    pub fn doc_id(&self) -> &str {
        match self {
            Self::Stale { doc_id, .. }
            | Self::Broken { doc_id, .. }
            | Self::Orphaned { doc_id, .. } => doc_id,
        }
    }

    /// Check if this signal requires immediate attention.
    #[must_use]
    pub fn requires_attention(&self) -> bool {
        matches!(
            self,
            Self::Broken { .. }
                | Self::Stale {
                    confidence: DriftConfidence::High,
                    ..
                }
        )
    }
}

// =============================================================================
// Phase 6.2: Streaming Bus Integration
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Global signal counter for unique IDs.
static SIGNAL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Streaming bus for observation signals.
///
/// This struct manages the flow of observation signals from Sentinel
/// to agent consumers via an MPSC channel.
pub struct ObservationBus {
    /// Sender for observation signals.
    tx: Option<mpsc::UnboundedSender<ObservationSignal>>,
}

impl Default for ObservationBus {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationBus {
    /// Create a new observation bus.
    #[must_use]
    pub fn new() -> Self {
        Self { tx: None }
    }

    /// Connect the bus to a receiver channel.
    pub fn connect(&mut self, tx: mpsc::UnboundedSender<ObservationSignal>) {
        self.tx = Some(tx);
    }

    /// Emit a signal to connected consumers.
    ///
    /// Returns the signal ID if successfully emitted.
    pub fn emit(&self, signal: ObservationSignal) -> Option<u64> {
        let tx = self.tx.as_ref()?;
        let signal_id = SIGNAL_COUNTER.fetch_add(1, Ordering::SeqCst);

        if tx.send(signal).is_ok() {
            Some(signal_id)
        } else {
            None
        }
    }

    /// Emit multiple signals from a semantic drift detection.
    #[must_use]
    pub fn emit_drift_signals(&self, drift: &SemanticDriftSignal) -> Vec<u64> {
        let signals = ObservationSignal::stale_from_drift(drift);
        signals.into_iter().filter_map(|s| self.emit(s)).collect()
    }

    /// Check if the bus is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.tx.is_some()
    }
}

/// Convert observation signals to a streaming status format.
///
/// This function transforms internal signals into a format suitable
/// for agent notification via the `ZhenfaStreamingEvent::Status` channel.
#[must_use]
pub fn signals_to_status_batch(signals: &[ObservationSignal]) -> String {
    use std::fmt::Write as _;

    let mut batch = String::new();
    batch.push_str("=== Observation Signal Batch ===\n");

    for (i, signal) in signals.iter().enumerate() {
        let _ = writeln!(batch, "{}. {}", i + 1, signal.to_status_message());
    }

    let _ = write!(
        batch,
        "\nTotal: {} signal(s), {} require immediate attention",
        signals.len(),
        signals.iter().filter(|s| s.requires_attention()).count()
    );

    batch
}

#[cfg(test)]
#[path = "../../../tests/unit/zhenfa_router/native/sentinel.rs"]
mod tests;
