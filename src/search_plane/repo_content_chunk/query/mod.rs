mod candidates;
mod error;
mod execution;
mod helpers;
mod scan;
mod search;
#[cfg(test)]
mod tests;

pub(crate) use candidates::RepoContentChunkCandidate;
pub(crate) use error::RepoContentChunkSearchError;
pub(crate) use helpers::{candidate_path_key, compare_candidates};
pub(crate) use scan::retained_window;
pub(crate) use search::search_repo_content_chunks;
