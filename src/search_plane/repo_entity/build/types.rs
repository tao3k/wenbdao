use crate::search_plane::repo_entity::schema::RepoEntityRow;
use crate::search_plane::{RepoStagedMutationAction, RepoStagedMutationPlan};

pub(crate) type RepoEntityBuildAction = RepoStagedMutationAction<Vec<RepoEntityRow>>;
pub(crate) type RepoEntityBuildPlan = RepoStagedMutationPlan<Vec<RepoEntityRow>>;
