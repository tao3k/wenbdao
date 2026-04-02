use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::gateway::studio::router::handlers::repo::analysis::search::service::run_repo_import_search;
use crate::gateway::studio::router::handlers::repo::{
    RepoImportSearchApiQuery, required_import_search_filters, required_repo_id,
};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

/// Import search endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, both `package` and `module` are
/// missing, repository lookup or analysis fails, or the background task panics.
pub async fn import_search(
    Query(query): Query<RepoImportSearchApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::ImportSearchResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let (package, module) =
        required_import_search_filters(query.package.as_deref(), query.module.as_deref())?;
    let limit = query.limit.unwrap_or(10).max(1);
    let result =
        run_repo_import_search(Arc::clone(&state), repo_id, package, module, limit).await?;
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::extract::{Query, State};

    use crate::gateway::studio::router::handlers::repo::RepoImportSearchApiQuery;
    use crate::gateway::studio::router::{GatewayState, StudioState};

    #[tokio::test]
    async fn import_search_requires_repo() {
        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(StudioState::new()),
        });

        let error = super::import_search(
            Query(RepoImportSearchApiQuery {
                repo: None,
                package: Some("SciMLBase".to_string()),
                module: None,
                limit: Some(10),
            }),
            State(state),
        )
        .await
        .expect_err("missing repo should fail before repository execution");

        assert_eq!(error.code(), "MISSING_REPO");
    }

    #[tokio::test]
    async fn import_search_requires_package_or_module() {
        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(StudioState::new()),
        });

        let error = super::import_search(
            Query(RepoImportSearchApiQuery {
                repo: Some("alpha/repo".to_string()),
                package: None,
                module: None,
                limit: Some(10),
            }),
            State(state),
        )
        .await
        .expect_err("missing import filters should fail before repository execution");

        assert_eq!(error.code(), "MISSING_IMPORT_FILTER");
    }
}
