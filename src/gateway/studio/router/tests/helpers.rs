use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{UiConfig, UiRepoProjectConfig};

pub(crate) fn studio_with_repo_projects(repo_projects: Vec<UiRepoProjectConfig>) -> StudioState {
    let studio = StudioState::new();
    studio.set_ui_config(UiConfig {
        projects: Vec::new(),
        repo_projects,
    });
    studio
}

pub(crate) fn repo_project(id: &str) -> UiRepoProjectConfig {
    UiRepoProjectConfig {
        id: id.to_string(),
        root: Some(".".to_string()),
        url: None,
        git_ref: None,
        refresh: None,
        plugins: vec!["julia".to_string()],
    }
}
