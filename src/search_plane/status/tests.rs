use crate::search_plane::coordinator::SearchCompactionReason;
use crate::search_plane::status::maintenance::{SearchMaintenancePolicy, ratio_threshold_parts};

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
