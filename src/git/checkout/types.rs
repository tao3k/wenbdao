use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Synchronization mode for repository checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepositorySyncMode {
    /// Ensure checkout exists and is up to date.
    #[default]
    Ensure,
    /// Force refresh from remote.
    Refresh,
    /// Report status without making changes.
    Status,
}

/// Backward-compatible alias used by the analysis service.
pub type CheckoutSyncMode = RepositorySyncMode;

/// Metadata discovered from a local checkout.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalCheckoutMetadata {
    /// Current revision of the checkout.
    pub revision: Option<String>,
    /// Upstream remote URL when the checkout is a git repository.
    pub remote_url: Option<String>,
}

/// Lifecycle state of a managed repository.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepositoryLifecycleState {
    /// No lifecycle phase was required.
    NotApplicable,
    /// The expected asset is missing.
    Missing,
    /// A local checkout was validated without mutation.
    Validated,
    /// An existing asset was observed without mutation.
    Observed,
    /// A new asset was created.
    Created,
    /// An existing asset was reused.
    Reused,
    /// An existing asset was refreshed.
    Refreshed,
}

/// Resolved source information for a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedRepositorySource {
    /// Root path of the checkout.
    pub checkout_root: PathBuf,
    /// Optional managed mirror path.
    pub mirror_root: Option<PathBuf>,
    /// Revision of the mirror (if managed).
    pub mirror_revision: Option<String>,
    /// Revision being tracked.
    pub tracking_revision: Option<String>,
    /// Last observed fetch timestamp in RFC3339 format.
    pub last_fetched_at: Option<String>,
    /// Drift summary between tracked and checkout revisions.
    pub drift_state: crate::analyzers::query::RepoSyncDriftState,
    /// Mirror lifecycle state.
    pub mirror_state: RepositoryLifecycleState,
    /// Checkout lifecycle state.
    pub checkout_state: RepositoryLifecycleState,
    /// Kind of the resolved source.
    pub source_kind: ResolvedRepositorySourceKind,
}

/// Kind of resolved repository source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolvedRepositorySourceKind {
    /// Local checkout path provided by user.
    LocalCheckout,
    /// Managed remote repository materialized under `PRJ_DATA_HOME`.
    ManagedRemote,
}
