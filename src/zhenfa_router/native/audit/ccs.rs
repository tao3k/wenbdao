//! Core CCS (Context Completeness Score) calculation.
//!
//! The CCS measures how well the retrieved evidence aligns with
//! expected style anchors from the persona profile.

use super::CompensationRequest;
use super::verdict::AuditVerdict;
use crate::zhenfa_router::native::audit::AuditResult;

/// CCS threshold for accepting search results without compensation.
pub const CCS_THRESHOLD: f64 = 0.70;

/// Synapse-Audit alignment threshold (drift < 0.05 means CCS > 0.95).
const SYNAPSE_AUDIT_THRESHOLD: f32 = 0.05;

/// Evaluate alignment between search evidence and style anchors.
///
/// This implements the core CCS calculation:
/// ```text
/// CCS = 1.0 - drift_score
/// drift_score = 1.0 - (matches / total_anchors)
/// ```
///
/// # Arguments
/// * `anchors` - Required style anchors from persona profile
/// * `evidence` - Extracted text from search hits (stems, summaries)
///
/// # Returns
/// `AuditVerdict` with drift score and missing anchors
#[must_use]
pub fn evaluate_alignment(anchors: &[String], evidence: &[String]) -> AuditVerdict {
    if anchors.is_empty() {
        return AuditVerdict {
            drift_score: 0.0,
            is_aligned: true,
            missing_anchors: Vec::new(),
        };
    }

    let mut matches = 0;
    let mut missing = Vec::new();

    for anchor in anchors {
        let anchor_lower = anchor.to_lowercase();
        let found = evidence
            .iter()
            .any(|e| e.to_lowercase().contains(&anchor_lower));

        if found {
            matches += 1;
        } else {
            missing.push(anchor.clone());
        }
    }

    let drift = 1.0 - (matches as f32 / anchors.len() as f32).max(0.0);

    AuditVerdict {
        drift_score: drift,
        is_aligned: drift < SYNAPSE_AUDIT_THRESHOLD,
        missing_anchors: missing,
    }
}

/// Audit search payload against provided anchors.
///
/// # Arguments
/// * `evidence` - Extracted text from search hits (stems, summaries)
/// * `anchors` - Required style anchors from persona profile
///
/// # Returns
/// `AuditResult` with CCS score and optional compensation request
#[must_use]
pub fn audit_search_payload(evidence: &[String], anchors: &[String]) -> AuditResult {
    let verdict = evaluate_alignment(anchors, evidence);
    let ccs_score = 1.0 - verdict.drift_score as f64;

    AuditResult {
        ccs_score,
        passed: ccs_score >= CCS_THRESHOLD,
        missing_anchors: verdict.missing_anchors.clone(),
        compensation: if ccs_score < CCS_THRESHOLD {
            Some(CompensationRequest {
                max_distance_delta: 1,
                related_limit_delta: 5,
            })
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ccs_perfect_alignment() {
        let anchors = vec!["latency".to_string(), "memory".to_string()];
        let evidence = vec![
            "System latency is low".to_string(),
            "Memory usage is optimized".to_string(),
        ];

        let result = audit_search_payload(&evidence, &anchors);

        assert!(result.passed);
        assert!(result.ccs_score >= 0.99);
        assert!(result.missing_anchors.is_empty());
        assert!(result.compensation.is_none());
    }

    #[test]
    fn test_ccs_partial_alignment() {
        let anchors = vec![
            "latency".to_string(),
            "memory".to_string(),
            "throughput".to_string(),
        ];
        let evidence = vec!["Memory is full.".to_string()];

        let result = audit_search_payload(&evidence, &anchors);

        // CCS = 1/3 = 0.33 < 0.70 -> FAIL
        assert!(!result.passed);
        assert!(result.ccs_score < CCS_THRESHOLD);
        assert!(result.missing_anchors.contains(&"latency".to_string()));
        assert!(result.missing_anchors.contains(&"throughput".to_string()));
        assert!(result.compensation.is_some());
    }

    #[test]
    fn test_ccs_empty_anchors() {
        let evidence = vec!["Some content".to_string()];

        let result = audit_search_payload(&evidence, &[]);

        // Empty anchors = perfect score (nothing to miss)
        assert!(result.passed);
        assert_eq!(result.ccs_score, 1.0);
    }

    #[test]
    fn test_drift_score_calculation() {
        let anchors = vec![
            "latency".to_string(),
            "memory".to_string(),
            "throughput".to_string(),
        ];
        let evidence = vec!["contains latency".to_string()]; // 1/3 match

        let verdict = evaluate_alignment(&anchors, &evidence);

        // drift = 1 - (1/3) ≈ 0.667
        // Use f32 for comparison since drift_score is f32
        let expected_drift: f32 = 1.0 - 1.0 / 3.0;
        assert!(
            (verdict.drift_score - expected_drift).abs() < 0.01,
            "drift_score = {}, expected = {}",
            verdict.drift_score,
            expected_drift
        );
        assert!(!verdict.is_aligned);
    }
}
