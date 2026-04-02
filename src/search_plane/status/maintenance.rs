use serde::{Deserialize, Serialize};

use crate::search_plane::coordinator::SearchCompactionReason;

/// Heuristics for deciding when background compaction should be scheduled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMaintenancePolicy {
    /// Force compaction after this many publishes since the last compact.
    pub publish_count_threshold: u32,
    /// Force compaction when row count drift exceeds this ratio.
    pub row_delta_ratio_threshold: f32,
}

impl SearchMaintenancePolicy {
    /// Return the first compaction reason whose threshold is currently violated.
    #[must_use]
    pub(crate) fn compaction_reason(
        &self,
        publish_count_since_compaction: u32,
        last_compacted_row_count: Option<u64>,
        next_row_count: u64,
    ) -> Option<SearchCompactionReason> {
        if publish_count_since_compaction >= self.publish_count_threshold {
            return Some(SearchCompactionReason::PublishThreshold);
        }
        let previous_row_count = last_compacted_row_count?;
        if previous_row_count == 0 {
            return (next_row_count > 0).then_some(SearchCompactionReason::RowDeltaRatio);
        }
        let delta = previous_row_count.abs_diff(next_row_count);
        let (threshold_numerator, threshold_denominator) =
            ratio_threshold_parts(self.row_delta_ratio_threshold);
        (u128::from(delta) * threshold_denominator
            >= u128::from(previous_row_count) * threshold_numerator)
            .then_some(SearchCompactionReason::RowDeltaRatio)
    }

    /// Decide whether background compaction should be scheduled.
    #[must_use]
    pub fn should_compact(
        &self,
        publish_count_since_compaction: u32,
        last_compacted_row_count: Option<u64>,
        next_row_count: u64,
    ) -> bool {
        self.compaction_reason(
            publish_count_since_compaction,
            last_compacted_row_count,
            next_row_count,
        )
        .is_some()
    }
}

impl Default for SearchMaintenancePolicy {
    fn default() -> Self {
        Self {
            publish_count_threshold: 8,
            row_delta_ratio_threshold: 0.25,
        }
    }
}

pub(crate) fn ratio_threshold_parts(threshold: f32) -> (u128, u128) {
    let normalized = if threshold.is_sign_negative() {
        String::from("0")
    } else {
        format!("{threshold:.6}")
    };
    let trimmed = normalized.trim_end_matches('0').trim_end_matches('.');
    let (whole_part, fractional_part) = match trimmed.split_once('.') {
        Some((whole_part, fractional_part)) => (whole_part, fractional_part),
        None => (trimmed, ""),
    };
    let whole_value = whole_part.parse::<u128>().ok().unwrap_or_default();
    if fractional_part.is_empty() {
        return (whole_value, 1);
    }
    let denominator = 10_u128.pow(
        u32::try_from(fractional_part.len())
            .ok()
            .unwrap_or_default(),
    );
    let fractional_value = fractional_part.parse::<u128>().ok().unwrap_or_default();
    (
        whole_value
            .saturating_mul(denominator)
            .saturating_add(fractional_value),
        denominator,
    )
}

/// Background maintenance state derived from publish/compaction history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchMaintenanceStatus {
    #[serde(default)]
    /// Whether a staging-table prewarm task is actively running.
    pub prewarm_running: bool,
    #[serde(default)]
    /// Number of queued prewarm tasks currently waiting behind the active worker.
    pub prewarm_queue_depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// One-based queue position for this corpus when its prewarm is queued in repo maintenance.
    pub prewarm_queue_position: Option<u32>,
    /// Whether a compaction task is actively running for the readable publication.
    pub compaction_running: bool,
    #[serde(default)]
    /// Number of queued compaction tasks currently waiting behind the active worker.
    pub compaction_queue_depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// One-based queue position for this corpus when its compaction is queued locally.
    pub compaction_queue_position: Option<u32>,
    #[serde(default)]
    /// Whether enqueue-time fairness aging has already promoted this queued compaction task.
    pub compaction_queue_aged: bool,
    /// Whether the coordinator should schedule a compact/optimize run.
    pub compaction_pending: bool,
    /// Number of publishes observed since the last successful compaction.
    pub publish_count_since_compaction: u32,
    /// RFC3339 timestamp of the most recent successful staging-table prewarm.
    pub last_prewarmed_at: Option<String>,
    /// Epoch identifier of the most recent successful staging-table prewarm.
    pub last_prewarmed_epoch: Option<u64>,
    /// RFC3339 timestamp of the most recent successful compaction.
    pub last_compacted_at: Option<String>,
    /// Human-readable reason for the most recent compaction.
    pub last_compaction_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Row count observed when compaction most recently completed.
    pub last_compacted_row_count: Option<u64>,
}

#[cfg(test)]
mod tests {
    use crate::search_plane::coordinator::SearchCompactionReason;
    use crate::search_plane::status::maintenance::{
        SearchMaintenancePolicy, ratio_threshold_parts,
    };

    #[test]
    fn ratio_threshold_parts_preserves_decimal_thresholds() {
        assert_eq!(ratio_threshold_parts(0.25), (25, 100));
        assert_eq!(ratio_threshold_parts(0.9), (9, 10));
        assert_eq!(ratio_threshold_parts(1.0), (1, 1));
        assert_eq!(ratio_threshold_parts(-1.0), (0, 1));
    }

    #[test]
    fn compaction_reason_uses_fixed_precision_ratio_comparison() {
        let policy = SearchMaintenancePolicy {
            publish_count_threshold: 99,
            row_delta_ratio_threshold: 0.25,
        };

        assert_eq!(policy.compaction_reason(0, Some(100), 124), None);
        assert_eq!(
            policy.compaction_reason(0, Some(100), 125),
            Some(SearchCompactionReason::RowDeltaRatio)
        );
    }
}
