use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::Router;

use crate::gateway::studio::perf_support::git::{create_local_git_repo, write_default_repo_config};
use crate::gateway::studio::perf_support::root::{
    DEFAULT_REAL_WORKSPACE_ROOT, GatewayPerfRoot, REAL_WORKSPACE_ROOT_ENV, create_perf_root,
    resolve_real_workspace_root,
};
use crate::gateway::studio::perf_support::state::gateway_state_for_project;
use crate::gateway::studio::perf_support::workspace::{
    publish_code_search_snapshot, warm_real_workspace_search_plane,
};
use crate::gateway::studio::router::studio_router;

/// Prepared Studio gateway fixture for performance tests.
pub struct GatewayPerfFixture {
    root: GatewayPerfRoot,
    state: Arc<crate::gateway::studio::router::GatewayState>,
}

impl GatewayPerfFixture {
    /// Return a fresh Studio router bound to the prepared gateway state.
    pub fn router(&self) -> Router {
        studio_router(Arc::clone(&self.state))
    }

    /// Return the shared gateway state backing this fixture.
    #[must_use]
    pub fn state(&self) -> Arc<crate::gateway::studio::router::GatewayState> {
        Arc::clone(&self.state)
    }

    /// Return the temporary project root backing this fixture.
    #[must_use]
    pub fn root(&self) -> &Path {
        match &self.root {
            GatewayPerfRoot::Owned(path) | GatewayPerfRoot::External(path) => path.as_path(),
        }
    }

    /// Execute one direct repo-scoped search-plane query so status telemetry
    /// exposes a repo-specific scope bucket without paying for the full HTTP
    /// search handler chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the repo-backed search-plane query fails or does
    /// not return any hits for the requested repo/query pair.
    pub async fn warm_repo_scope_query(&self, repo_id: &str, query: &str) -> Result<()> {
        let hits = self
            .state
            .studio
            .search_plane
            .search_repo_entities(repo_id, query, &HashSet::new(), &HashSet::new(), 5)
            .await
            .map_err(|error| anyhow!("failed to warm repo-scoped search telemetry: {error}"))?;
        if hits.is_empty() {
            return Err(anyhow!(
                "repo-scoped search warmup returned no hits for repo `{repo_id}` and query `{query}`"
            ));
        }
        Ok(())
    }
}

impl Drop for GatewayPerfFixture {
    fn drop(&mut self) {
        self.state.studio.stop_background_services();
        if let GatewayPerfRoot::Owned(path) = &self.root {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

/// Build a warm-cache gateway fixture with one Julia repository.
///
/// # Errors
///
/// Returns an error if the temporary project cannot be created, initialized as
/// a Git repository, analyzed, or published into the search plane.
pub async fn prepare_gateway_perf_fixture() -> Result<GatewayPerfFixture> {
    let root = create_perf_root()?;
    let repo_dir = create_local_git_repo(root.as_path(), "GatewaySyncPkg")?;
    write_default_repo_config(root.as_path(), repo_dir.as_path(), "gateway-sync")?;
    let state = gateway_state_for_project(root.as_path())?;
    publish_code_search_snapshot(&state, "gateway-sync").await?;
    Ok(GatewayPerfFixture {
        root: GatewayPerfRoot::Owned(root),
        state,
    })
}

/// Build a warm-cache gateway fixture backed by a real multi-repository
/// workspace.
///
/// The workspace root is resolved from
/// `XIUXIAN_WENDAO_GATEWAY_PERF_WORKSPACE_ROOT` first, then falls back to
/// `.data/wendao-frontend` under the current project root when present.
///
/// # Errors
///
/// Returns an error when no real workspace root can be resolved, when the
/// target workspace cannot bootstrap gateway state, or when repo indexing does
/// not reach a query-ready state before the configured timeout.
pub async fn prepare_gateway_real_workspace_perf_fixture() -> Result<GatewayPerfFixture> {
    let root = resolve_real_workspace_root().ok_or_else(|| {
        anyhow!(
            "real gateway perf workspace root is not available; set {} or create {}",
            REAL_WORKSPACE_ROOT_ENV,
            DEFAULT_REAL_WORKSPACE_ROOT
        )
    })?;
    let state = gateway_state_for_project(root.as_path())?;
    warm_real_workspace_search_plane(&state).await?;
    Ok(GatewayPerfFixture {
        root: GatewayPerfRoot::External(root),
        state,
    })
}
