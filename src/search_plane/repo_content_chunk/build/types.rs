use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::{RepoStagedMutationAction, RepoStagedMutationPlan};

pub(crate) const REPO_CONTENT_CHUNK_EXTRACTOR_VERSION: u32 = 1;

pub(crate) type RepoContentChunkBuildAction = RepoStagedMutationAction<Vec<RepoCodeDocument>>;
pub(crate) type RepoContentChunkBuildPlan = RepoStagedMutationPlan<Vec<RepoCodeDocument>>;
