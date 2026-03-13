//! Sync Engine - Incremental Knowledge Synchronization.

mod diff;
mod discovery;
mod incremental;
mod manifest;
mod types;

pub use incremental::{IncrementalSyncPolicy, SyncEngine, extract_extensions_from_glob_patterns};
pub use types::{DiscoveryOptions, FileChange, SyncManifest, SyncResult};
