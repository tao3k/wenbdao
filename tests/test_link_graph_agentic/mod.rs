//! Integration tests for passive `LinkGraph` suggested-link logging.

mod suggested_link_decide_promoted_with_audit;
mod suggested_link_decide_rejects_invalid_transition;
mod suggested_link_log_rejects_invalid_payload;
mod suggested_link_log_roundtrip;
mod suggested_link_log_trims_stream_by_max_entries;
mod support;
