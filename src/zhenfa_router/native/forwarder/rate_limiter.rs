use std::collections::HashMap;

/// Rate limiter state for notifications.
#[derive(Debug, Clone, Default)]
pub(super) struct RateLimiter {
    /// Map of `doc_id` to (count, `hour_timestamp`).
    doc_counts: HashMap<String, (usize, i64)>,
}

impl RateLimiter {
    pub(super) fn check_and_increment(&mut self, doc_id: &str, limit: usize) -> bool {
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
