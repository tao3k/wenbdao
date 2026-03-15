//! Vision ingress operator for semantic injection.
//!
//! Scans all image attachments and calls external multimodal interfaces
//! (OCR/LLM Vision) to inject vision annotations into the graph index.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::index::build::vision_ingress::{VisionIngress, VisionProvider};
//!
//! let provider = VisionProvider::dots();
//! let ingress = VisionIngress::new(Arc::new(provider), Some(root.into()));
//! let annotations = ingress.process_attachments(&attachments, &docs).await;
//! ```

use crate::link_graph::models::{
    LinkGraphAttachment, LinkGraphAttachmentKind, LinkGraphDocument, VisionAnnotation,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use log::warn;
use xiuxian_llm::llm::vision::{
    DeepseekRuntime, PreparedVisionImage, get_deepseek_runtime, infer_deepseek_ocr_truth,
    preprocess_image,
};

/// Default max dimension for image preprocessing.
const DEFAULT_OCR_MAX_DIMENSION: u32 = 1024;

/// Vision provider enum supporting multiple backends.
#[derive(Debug, Clone)]
pub enum VisionProvider {
    /// No-op provider that returns None for all images.
    NoOp,
    /// Dots OCR provider using xiuxian-llm's DeepSeek runtime.
    Dots {
        runtime: Arc<DeepseekRuntime>,
        stop_signal: Option<Arc<AtomicBool>>,
        max_dimension: u32,
    },
}

impl VisionProvider {
    /// Create a no-op provider.
    #[must_use]
    pub fn noop() -> Self {
        Self::NoOp
    }

    /// Create a Dots OCR provider with the process-wide runtime.
    #[must_use]
    pub fn dots() -> Self {
        Self::Dots {
            runtime: get_deepseek_runtime(),
            stop_signal: None,
            max_dimension: DEFAULT_OCR_MAX_DIMENSION,
        }
    }

    /// Create a Dots OCR provider with a custom runtime.
    #[must_use]
    pub fn dots_with_runtime(runtime: Arc<DeepseekRuntime>) -> Self {
        Self::Dots {
            runtime,
            stop_signal: None,
            max_dimension: DEFAULT_OCR_MAX_DIMENSION,
        }
    }

    /// Create a Dots OCR provider with cancellation support.
    #[must_use]
    pub fn dots_with_stop_signal(stop_signal: Arc<AtomicBool>) -> Self {
        Self::Dots {
            runtime: get_deepseek_runtime(),
            stop_signal: Some(stop_signal),
            max_dimension: DEFAULT_OCR_MAX_DIMENSION,
        }
    }

    /// Set custom max dimension for preprocessing.
    #[must_use]
    pub fn with_max_dimension(self, max_dimension: u32) -> Self {
        match self {
            Self::Dots {
                runtime,
                stop_signal,
                ..
            } => Self::Dots {
                runtime,
                stop_signal,
                max_dimension,
            },
            other => other,
        }
    }

    /// Check if the provider is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::NoOp => false,
            Self::Dots { runtime, .. } => runtime.is_enabled(),
        }
    }

    /// Get the provider status description.
    #[must_use]
    pub fn status(&self) -> &str {
        match self {
            Self::NoOp => "no-op provider",
            Self::Dots { runtime, .. } => match runtime.as_ref() {
                DeepseekRuntime::Disabled { reason } => reason.as_ref(),
                DeepseekRuntime::Configured { model_root } => model_root.as_ref(),
                DeepseekRuntime::RemoteHttp { base_url } => base_url.as_ref(),
            },
        }
    }

    /// Analyze an image and return vision annotation.
    pub async fn analyze(&self, path: &PathBuf) -> Option<VisionAnnotation> {
        match self {
            Self::NoOp => None,
            Self::Dots {
                runtime,
                stop_signal,
                max_dimension,
            } => analyze_with_dots(runtime, stop_signal.as_ref(), *max_dimension, path).await,
        }
    }
}

impl Default for VisionProvider {
    fn default() -> Self {
        Self::noop()
    }
}

/// Analyze image using Dots OCR.
async fn analyze_with_dots(
    runtime: &Arc<DeepseekRuntime>,
    stop_signal: Option<&Arc<AtomicBool>>,
    _max_dimension: u32,
    path: &PathBuf,
) -> Option<VisionAnnotation> {
    // Skip if runtime is disabled
    if !runtime.is_enabled() {
        return None;
    }

    // Check for cancellation
    if let Some(signal) = stop_signal {
        if signal.load(Ordering::SeqCst) {
            return None;
        }
    }

    // Read image bytes
    let image_bytes = match std::fs::read(path) {
        Ok(bytes) => Arc::from(bytes),
        Err(e) => {
            warn!(
                "wendao.vision.read_failed: path={} error={}",
                path.display(),
                e
            );
            return None;
        }
    };

    // Preprocess image for OCR
    let prepared: PreparedVisionImage = match preprocess_image(image_bytes) {
        Ok(p) => p,
        Err(e) => {
            warn!(
                "wendao.vision.preprocess_failed: path={} error={}",
                path.display(),
                e
            );
            return None;
        }
    };

    // Run OCR inference
    let signal_clone = stop_signal.cloned();
    let result = infer_deepseek_ocr_truth(runtime, &prepared, signal_clone).await;

    match result {
        Ok(Some(text)) => {
            let entities = extract_entities(&text);
            let annotated_at = chrono::Utc::now().timestamp();
            Some(VisionAnnotation {
                description: text,
                confidence: 0.85, // Default confidence for OCR
                entities,
                annotated_at,
            })
        }
        Ok(None) => None,
        Err(e) => {
            warn!(
                "wendao.vision.ocr_failed: path={} error={}",
                path.display(),
                e
            );
            None
        }
    }
}

/// Extract code entities from OCR text (class names, function names, module names).
fn extract_entities(text: &str) -> Vec<String> {
    static PASCAL_CASE_REGEX: std::sync::LazyLock<Option<Regex>> =
        std::sync::LazyLock::new(|| Regex::new(r"[A-Z][a-zA-Z0-9]{2,}").ok());
    static BACKTICK_REGEX: std::sync::LazyLock<Option<Regex>> = std::sync::LazyLock::new(|| {
        Regex::new(r"`([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)`").ok()
    });
    let mut entities = HashSet::new();
    if let Some(pascal_regex) = PASCAL_CASE_REGEX.as_ref() {
        for cap in pascal_regex.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                entities.insert(m.as_str().to_string());
            }
        }
    } else {
        warn!("wendao.vision.regex_unavailable: pattern=pascal_case");
    }
    if let Some(backtick_regex) = BACKTICK_REGEX.as_ref() {
        for cap in backtick_regex.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                entities.insert(m.as_str().to_string());
            }
        }
    } else {
        warn!("wendao.vision.regex_unavailable: pattern=backtick_identifier");
    }
    entities.into_iter().collect()
}

/// Vision ingress operator.
pub struct VisionIngress {
    provider: VisionProvider,
    root: Option<PathBuf>,
}

impl VisionIngress {
    /// Create new vision ingress with provider.
    pub fn new(provider: VisionProvider, root: Option<PathBuf>) -> Self {
        Self { provider, root }
    }

    /// Process all image attachments in the index.
    pub async fn process_attachments(
        &self,
        attachments: &[LinkGraphAttachment],
        _docs: &HashMap<String, LinkGraphDocument>,
    ) -> HashMap<String, VisionAnnotation> {
        if attachments.is_empty() || _docs.is_empty() {
            return HashMap::new();
        }

        let mut results = HashMap::new();

        for attachment in attachments {
            if attachment.kind != LinkGraphAttachmentKind::Image {
                continue;
            }

            if let Some(full_path) = self.resolve_full_path(attachment) {
                if let Some(annotation) = self.provider.analyze(&full_path).await {
                    results.insert(attachment.attachment_name.clone(), annotation);
                }
            }
        }

        results
    }

    /// Resolve attachment relative path to absolute filesystem path.
    fn resolve_full_path(&self, attachment: &LinkGraphAttachment) -> Option<PathBuf> {
        let root = self.root.as_ref()?;

        // Try attachment path relative to root
        let full_path = root.join(&attachment.attachment_path);
        if full_path.is_file() {
            return Some(full_path);
        }

        // Try source document directory
        let source_dir = PathBuf::from(&attachment.source_path)
            .parent()
            .map(|p| root.join(p))?;
        let alt_path = source_dir.join(&attachment.attachment_name);
        if alt_path.is_file() {
            return Some(alt_path);
        }

        // Try direct name in source directory
        let direct_path = source_dir.join(&attachment.attachment_path);
        if direct_path.is_file() {
            return Some(direct_path);
        }

        None
    }
}

/// Build semantic edges from vision annotations.
///
/// Creates edges between images and documents based on
/// vision-extracted text/entities matching document IDs.
#[must_use]
pub fn build_cross_modal_edges(
    annotations: &HashMap<String, VisionAnnotation>,
    doc_ids: &[String],
) -> HashMap<String, Vec<String>> {
    if annotations.is_empty() || doc_ids.is_empty() {
        return HashMap::new();
    }

    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    let doc_id_set: HashSet<&str> = doc_ids.iter().map(|s| s.as_str()).collect();

    for (image_name, annotation) in annotations {
        // Match description words against doc IDs (substring match)
        for word in annotation.description.split_whitespace() {
            let lower_word = word.to_lowercase();
            for doc_id in &doc_id_set {
                if doc_id.contains(&lower_word) {
                    edges
                        .entry(image_name.clone())
                        .or_insert_with(Vec::new)
                        .push((*doc_id).to_string());
                }
            }
        }

        // Match entities against doc IDs (substring match)
        for entity in &annotation.entities {
            let lower_entity = entity.to_lowercase();
            for doc_id in &doc_id_set {
                if doc_id.contains(&lower_entity) {
                    edges
                        .entry(image_name.clone())
                        .or_insert_with(Vec::new)
                        .push((*doc_id).to_string());
                }
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_provider_returns_none() {
        let provider = VisionProvider::noop();
        let path = PathBuf::from("/nonexistent.png");
        let result = provider.analyze(&path).await;
        assert!(result.is_none());
    }

    #[test]
    fn test_dots_provider_status() {
        let provider = VisionProvider::dots();
        // Status should return a string (either reason or path)
        let _status = provider.status();
    }

    #[test]
    fn test_extract_entities() {
        let text =
            r#"This is a diagram showing `MyClass` and `my_function` interacting with SomeModule"#;
        let entities = extract_entities(text);

        assert!(entities.contains(&"MyClass".to_string()));
        assert!(entities.contains(&"my_function".to_string()));
        assert!(entities.contains(&"SomeModule".to_string()));
    }

    #[test]
    fn test_cross_modal_edges_empty() {
        let annotations = HashMap::new();
        let doc_ids: Vec<String> = Vec::new();

        let edges = build_cross_modal_edges(&annotations, &doc_ids);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_cross_modal_edges_basic() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "image1.png".to_string(),
            VisionAnnotation {
                description: "Rust performance optimization diagram".to_string(),
                confidence: 0.9,
                entities: vec!["rust".to_string(), "performance".to_string()],
                annotated_at: 0,
            },
        );

        let doc_ids = vec!["rust.md".to_string(), "performance.md".to_string()];

        let edges = build_cross_modal_edges(&annotations, &doc_ids);
        assert_eq!(edges.len(), 1);
        assert!(edges.contains_key("image1.png"));
    }
}
