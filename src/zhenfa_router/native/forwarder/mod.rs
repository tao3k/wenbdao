//! Phase 7: `ForwardNotifier` - Proactive Notification System
//!
//! This module implements forward notification capabilities for semantic drift events.
//! When documentation becomes stale due to source code changes, this system can
//! proactively notify document owners via various channels (webhook, email, etc.).

mod config;
mod payload;
mod rate_limiter;
mod service;

pub use config::ForwarderConfig;
pub use payload::{AffectedDocInfo, ForwardNotification, SuggestedAction};
pub use service::ForwardNotifier;

#[cfg(test)]
#[path = "../../../../tests/unit/zhenfa_router/native/forwarder.rs"]
mod tests;
