use std::collections::HashSet;
use std::sync::Arc;

use crate::gateway::studio::repo_index::RepoIndexStatusResponse;
use crate::gateway::studio::router::config::persist_ui_config_to_wendao_toml;
use crate::gateway::studio::router::repository::configured_repositories;
use crate::gateway::studio::router::sanitization::{sanitize_projects, sanitize_repo_projects};
use crate::gateway::studio::router::state::helpers::supported_code_kinds;
use crate::gateway::studio::router::state::types::StudioState;
use crate::gateway::studio::types::{
    UiCapabilities, UiConfig, UiProjectConfig, UiRepoProjectConfig,
};

impl StudioState {
    pub(crate) fn ui_config(&self) -> UiConfig {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn ui_capabilities(&self) -> UiCapabilities {
        let ui_config = self.ui_config();
        let bootstrap_background_indexing = self.bootstrap_background_indexing_telemetry();
        let mut seen_repositories = HashSet::new();
        let supported_repositories = ui_config
            .repo_projects
            .into_iter()
            .filter_map(|project| {
                let repository_id = project.id.trim().to_string();
                if repository_id.is_empty() || !seen_repositories.insert(repository_id.clone()) {
                    return None;
                }
                Some(repository_id)
            })
            .collect();
        let supported_languages = self
            .plugin_registry
            .plugin_ids()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect();

        UiCapabilities {
            languages: supported_languages,
            repositories: supported_repositories,
            kinds: supported_code_kinds(),
            studio_bootstrap_background_indexing_enabled: bootstrap_background_indexing.enabled(),
            studio_bootstrap_background_indexing_mode: bootstrap_background_indexing
                .mode()
                .to_string(),
            studio_bootstrap_background_indexing_deferred_activation_observed:
                bootstrap_background_indexing.deferred_activation_observed(),
        }
    }

    pub(crate) fn set_ui_config(&self, config: UiConfig) {
        self.apply_ui_config(config, true);
    }

    fn ensure_repo_background_indexing_started(&self, source: &'static str) {
        let repositories = configured_repositories(self);
        if repositories.is_empty() {
            return;
        }

        self.record_deferred_bootstrap_background_indexing_activation(source);
        self.repo_index.sync_repositories(repositories);
    }

    fn ensure_background_indexes_started(&self, source: &'static str) {
        let configured_projects = self.configured_projects();
        if !configured_projects.is_empty() {
            self.record_deferred_bootstrap_background_indexing_activation(source);
            self.symbol_index_coordinator
                .sync_projects(configured_projects, Arc::clone(&self.symbol_index));
        }
        self.ensure_repo_background_indexing_started(source);
    }

    pub(crate) fn apply_ui_config(&self, config: UiConfig, eager_background_indexing: bool) {
        let sanitized_projects = sanitize_projects(config.projects);
        let sanitized_repo_projects = sanitize_repo_projects(config.repo_projects);
        let mut guard = self
            .ui_config
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.projects == sanitized_projects && guard.repo_projects == sanitized_repo_projects {
            drop(guard);
            if eager_background_indexing {
                self.ensure_background_indexes_started("set_ui_config");
            }
            return;
        }
        guard.projects = sanitized_projects;
        guard.repo_projects = sanitized_repo_projects;
        drop(guard);

        let mut graph_guard = self
            .graph_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *graph_guard = None;
        drop(graph_guard);

        let mut symbol_guard = self
            .symbol_index
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *symbol_guard = None;
        drop(symbol_guard);

        let mut vfs_guard = self
            .vfs_scan
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *vfs_guard = None;
        drop(vfs_guard);

        if eager_background_indexing {
            self.ensure_background_indexes_started("set_ui_config");
        }
    }

    pub(crate) fn repo_index_status(&self, repo: Option<&str>) -> RepoIndexStatusResponse {
        let status = self.repo_index.status_response(repo);
        if status.total > 0 {
            return status;
        }

        self.ensure_repo_background_indexing_started("repo_index_status");
        self.repo_index.status_response(repo)
    }

    pub(crate) fn set_ui_config_and_persist(&self, config: UiConfig) -> Result<(), String> {
        self.set_ui_config(config);
        persist_ui_config_to_wendao_toml(self.config_root.as_path(), &self.ui_config())
    }

    pub(crate) fn configured_projects(&self) -> Vec<UiProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .projects
            .clone()
    }

    pub(crate) fn configured_repo_projects(&self) -> Vec<UiRepoProjectConfig> {
        self.ui_config
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_projects
            .clone()
    }
}
