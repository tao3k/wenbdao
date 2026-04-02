use crate::gateway::studio::pathing::studio_display_path;
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::StudioNavigationTarget;

pub(crate) fn resolve_navigation_target(state: &StudioState, path: &str) -> StudioNavigationTarget {
    let normalized = studio_display_path(state, path);
    let project_name = state
        .configured_projects()
        .into_iter()
        .find(|project| {
            normalized == project.name
                || normalized.starts_with(format!("{}/", project.name).as_str())
        })
        .map(|project| project.name);

    StudioNavigationTarget {
        path: normalized,
        category: "file".to_string(),
        project_name,
        root_label: None,
        line: None,
        line_end: None,
        column: None,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_navigation_target;
    use crate::gateway::studio::router::StudioState;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig};

    #[test]
    fn resolve_navigation_target_prefixes_configured_project_for_relative_docs_path() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let mut state = StudioState::new();
        state.project_root = temp_dir.path().to_path_buf();
        state.config_root = temp_dir.path().to_path_buf();
        state.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "main".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: Vec::new(),
        });

        let target = resolve_navigation_target(&state, "docs/index.md");

        assert_eq!(target.path, "main/docs/index.md");
        assert_eq!(target.project_name.as_deref(), Some("main"));
    }
}
