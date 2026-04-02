//! Background repo indexing coordinator for Studio.

mod state;
mod types;

pub(crate) use state::RepoIndexCoordinator;
pub(crate) use types::RepoCodeDocument;
#[cfg(test)]
pub(crate) use types::RepoIndexSnapshot;
pub use types::{RepoIndexEntryStatus, RepoIndexPhase, RepoIndexRequest, RepoIndexStatusResponse};
