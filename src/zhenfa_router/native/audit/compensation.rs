//! Compensation request types for secondary search.

use serde::{Deserialize, Serialize};

/// Compensation parameters for secondary search when CCS < threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationRequest {
    /// Increase max_distance for broader retrieval.
    pub max_distance_delta: usize,
    /// Increase related_limit for more context.
    pub related_limit_delta: usize,
}

impl Default for CompensationRequest {
    fn default() -> Self {
        Self {
            max_distance_delta: 1,
            related_limit_delta: 5,
        }
    }
}
