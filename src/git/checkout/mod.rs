//! Git checkout materialization and synchronization.

mod lock;
mod managed;
mod metadata;
mod namespace;
mod refs;
mod source;
mod types;

pub use metadata::discover_checkout_metadata;
pub use source::resolve_repository_source;
pub use types::{
    CheckoutSyncMode, LocalCheckoutMetadata, RepositoryLifecycleState, RepositorySyncMode,
    ResolvedRepositorySource, ResolvedRepositorySourceKind,
};

#[cfg(test)]
mod tests;
