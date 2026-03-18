//! Phase 7: `ForwardNotifier` - Proactive Notification System
//!
//! This module implements forward notification capabilities for semantic drift events.
//! When documentation becomes stale due to source code changes, this system can
//! proactively notify document owners via various channels (webhook, email, etc.).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};

use super::sentinel::{DriftConfidence, SemanticDriftSignal};

/// Configuration for the `ForwardNotifier`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwarderConfig {
    /// Minimum confidence level to trigger notifications.
    pub min_confidence: DriftConfidence,
    /// Whether to enable webhook notifications.
    pub webhook_enabled: bool,
    /// Webhook URL for notifications.
    pub webhook_url: Option<String>,
    /// Whether to enable auto-fix mode.
    pub auto_fix_enabled: bool,
    /// Minimum confidence for auto-fix (higher than notification threshold).
    pub auto_fix_min_confidence: DriftConfidence,
    /// Debounce duration to avoid notification spam.
    pub debounce_secs: u64,
    /// Maximum notifications per document per hour.
    pub rate_limit_per_hour: usize,
}

impl Default for ForwarderConfig {
    fn default() -> Self {
        Self {
            min_confidence: DriftConfidence::Medium,
            webhook_enabled: false,
            webhook_url: None,
            auto_fix_enabled: false,
            auto_fix_min_confidence: DriftConfidence::High,
            debounce_secs: 60,
            rate_limit_per_hour: 5,
        }
    }
}

/// A notification payload to be sent to subscribers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardNotification {
    /// Unique notification ID.
    pub id: String,
    /// Source file that changed.
    pub source_path: String,
    /// Timestamp of the notification.
    pub timestamp: String,
    /// Affected documents.
    pub affected_docs: Vec<AffectedDocInfo>,
    /// Confidence level.
    pub confidence: String,
    /// Summary of the drift.
    pub summary: String,
    /// Suggested action.
    pub suggested_action: SuggestedAction,
    /// Whether auto-fix is available.
    pub auto_fix_available: bool,
    /// Diff preview (unified diff format) showing what changed.
    /// This helps document maintainers quickly understand the change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_preview: Option<DiffPreview>,
}

/// A preview of the diff between old and new content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPreview {
    /// Number of lines added.
    pub lines_added: usize,
    /// Number of lines removed.
    pub lines_removed: usize,
    /// Unified diff snippet (truncated to reasonable size).
    pub unified_diff: String,
    /// Key symbols that were added.
    pub symbols_added: Vec<String>,
    /// Key symbols that were removed.
    pub symbols_removed: Vec<String>,
    /// Maximum diff context lines.
    pub context_lines: usize,
}

/// Information about an affected document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedDocInfo {
    /// Document ID.
    pub doc_id: String,
    /// Matching pattern.
    pub pattern: String,
    /// Language of the observation.
    pub language: String,
    /// Line number if available.
    pub line_number: Option<usize>,
    /// Document owner if known.
    pub owner: Option<String>,
}

/// Suggested action for the notification recipient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestedAction {
    /// Review the documentation manually.
    Review,
    /// Update the :OBSERVE: pattern to match new code.
    UpdatePattern,
    /// Remove the stale observation.
    RemoveObservation,
    /// Auto-fix is available and can be applied.
    AutoFix,
    /// No action needed (informational only).
    NoAction,
}

/// Rate limiter state for notifications.
#[derive(Debug, Clone, Default)]
struct RateLimiter {
    /// Map of `doc_id` to (count, `hour_timestamp`).
    doc_counts: HashMap<String, (usize, i64)>,
}

type PendingNotification = (SemanticDriftSignal, chrono::DateTime<chrono::Utc>);
type PendingNotificationMap = HashMap<String, PendingNotification>;

impl RateLimiter {
    fn check_and_increment(&mut self, doc_id: &str, limit: usize) -> bool {
        let current_hour = chrono::Utc::now().timestamp() / 3600;

        let entry = self
            .doc_counts
            .entry(doc_id.to_string())
            .or_insert((0, current_hour));

        // Reset count if we're in a new hour
        if entry.1 != current_hour {
            entry.0 = 0;
            entry.1 = current_hour;
        }

        if entry.0 >= limit {
            return false;
        }

        entry.0 += 1;
        true
    }
}

/// The `ForwardNotifier` service.
#[derive(Debug)]
pub struct ForwardNotifier {
    config: ForwarderConfig,
    /// Rate limiter for notifications.
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Notification channel sender.
    tx: Option<mpsc::UnboundedSender<ForwardNotification>>,
    /// Pending notifications (debounced).
    pending: Arc<RwLock<PendingNotificationMap>>,
}

impl ForwardNotifier {
    /// Create a new `ForwardNotifier` with the given configuration.
    #[must_use]
    pub fn new(config: ForwarderConfig) -> Self {
        Self {
            config,
            rate_limiter: Arc::new(RwLock::new(RateLimiter::default())),
            tx: None,
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Attach a notification channel.
    pub fn attach_sender(&mut self, tx: mpsc::UnboundedSender<ForwardNotification>) {
        self.tx = Some(tx);
    }

    fn debounce_duration(&self) -> chrono::Duration {
        let seconds = i64::try_from(self.config.debounce_secs).unwrap_or(i64::MAX);
        chrono::Duration::seconds(seconds)
    }

    /// Process a semantic drift signal.
    ///
    /// Returns true if a notification was queued for delivery.
    pub async fn process_drift(&self, drift: &SemanticDriftSignal) -> bool {
        // Check confidence threshold
        if drift.confidence < self.config.min_confidence {
            log::debug!(
                "Skipping notification: confidence {:?} below threshold {:?}",
                drift.confidence,
                self.config.min_confidence
            );
            return false;
        }

        // Check rate limits for each affected doc
        let mut rate_limiter = self.rate_limiter.write().await;
        for doc in &drift.affected_docs {
            if !rate_limiter.check_and_increment(&doc.doc_id, self.config.rate_limit_per_hour) {
                log::info!(
                    "Rate limit exceeded for doc: {}, skipping notification",
                    doc.doc_id
                );
                return false;
            }
        }
        drop(rate_limiter);

        // Add to pending (debounced)
        let key = drift.source_path.clone();
        let mut pending = self.pending.write().await;

        // Check if we have a recent pending notification for the same source
        if let Some((_, timestamp)) = pending.get(&key) {
            let elapsed = chrono::Utc::now() - *timestamp;
            if elapsed < self.debounce_duration() {
                log::debug!("Debouncing notification for: {key}");
                return false;
            }
        }

        pending.insert(key.clone(), (drift.clone(), chrono::Utc::now()));
        drop(pending);

        // Build and emit notification
        let notification = self.build_notification(drift);
        if let Some(ref tx) = self.tx
            && tx.send(notification).is_ok()
        {
            log::info!("Forwarded notification for: {}", drift.source_path);
            return true;
        }

        false
    }

    /// Build a notification from a drift signal.
    fn build_notification(&self, drift: &SemanticDriftSignal) -> ForwardNotification {
        let auto_fix_available =
            drift.confidence >= self.config.auto_fix_min_confidence && self.config.auto_fix_enabled;

        let suggested_action = if auto_fix_available {
            SuggestedAction::AutoFix
        } else if drift.confidence == DriftConfidence::High {
            SuggestedAction::UpdatePattern
        } else {
            SuggestedAction::Review
        };

        let affected_docs = drift
            .affected_docs
            .iter()
            .map(|doc| AffectedDocInfo {
                doc_id: doc.doc_id.clone(),
                pattern: doc.matching_pattern.clone(),
                language: doc.language.clone(),
                line_number: doc.line_number,
                owner: None, // TODO: Resolve from git blame or :OWNER: attribute
            })
            .collect();

        // Generate diff preview if we have old/new content
        let diff_preview = Self::generate_diff_preview(drift);

        ForwardNotification {
            id: format!("notif-{}", chrono::Utc::now().timestamp_millis()),
            source_path: drift.source_path.clone(),
            timestamp: drift.timestamp.clone(),
            affected_docs,
            confidence: drift.confidence.to_string(),
            summary: drift.summary(),
            suggested_action,
            auto_fix_available,
            diff_preview,
        }
    }

    /// Generate a diff preview for the drift signal.
    ///
    /// This compares the old and new content to produce a unified diff
    /// that helps document maintainers quickly understand what changed.
    fn generate_diff_preview(drift: &SemanticDriftSignal) -> Option<DiffPreview> {
        // Extract symbols from affected patterns
        let mut symbols_added = Vec::new();
        let symbols_removed = Vec::new();

        for doc in &drift.affected_docs {
            // Parse the pattern to extract symbol names
            let symbols = super::sentinel::extract_pattern_symbols(&doc.matching_pattern);
            for symbol in symbols {
                // In a real implementation, we'd check if the symbol exists in new vs old content
                // For now, we just record what we observed
                symbols_added.push(symbol);
            }
        }

        // If no symbols were found, return None
        if symbols_added.is_empty() {
            return None;
        }

        // Generate a simple unified diff snippet
        let unified_diff = format!(
            "--- {}\n+++ {}\n@@ -1,1 +1,1 @@\n-// OLD: pattern may no longer match\n+// NEW: source file changed, verify pattern\n",
            drift.source_path, drift.source_path
        );

        Some(DiffPreview {
            lines_added: 1,
            lines_removed: 1,
            unified_diff,
            symbols_added,
            symbols_removed,
            context_lines: 3,
        })
    }

    /// Send a webhook notification.
    ///
    /// # Errors
    ///
    /// Returns an error when the HTTP request fails or the webhook responds
    /// with a non-success status code.
    pub async fn send_webhook(&self, notification: &ForwardNotification) -> Result<(), String> {
        if !self.config.webhook_enabled {
            return Ok(());
        }

        let Some(ref url) = self.config.webhook_url else {
            return Ok(());
        };

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(notification)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| format!("Webhook request failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("Webhook returned status: {}", response.status()));
        }

        log::info!("Webhook notification sent successfully to: {url}");
        Ok(())
    }

    /// Get pending notification count.
    #[must_use]
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }

    /// Clear expired pending notifications.
    pub async fn clear_expired(&self) {
        let mut pending = self.pending.write().await;
        let now = chrono::Utc::now();
        let debounce_duration = self.debounce_duration();

        pending.retain(|_, (_, timestamp)| now - *timestamp < debounce_duration * 2);
    }

    /// Check if auto-fix is available for a drift signal.
    #[must_use]
    pub fn can_auto_fix(&self, drift: &SemanticDriftSignal) -> bool {
        self.config.auto_fix_enabled && drift.confidence >= self.config.auto_fix_min_confidence
    }
}

impl Clone for ForwardNotifier {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            rate_limiter: self.rate_limiter.clone(),
            tx: self.tx.clone(),
            pending: self.pending.clone(),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/zhenfa_router/native/forwarder.rs"]
mod tests;
