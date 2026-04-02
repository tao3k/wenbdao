use std::fs;
use std::sync::Arc;

use crate::analyzers::analyze_registered_repository_with_registry;
use crate::gateway::studio::router::code_ast::{
    build_code_ast_analysis_response, resolve_code_ast_repository_and_path,
};
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repositories, map_repo_intelligence_error,
};
use crate::gateway::studio::types::CodeAstAnalysisResponse;

pub(crate) async fn load_code_ast_analysis_response(
    state: &GatewayState,
    path: &str,
    repo_id: &str,
    line_hint: Option<usize>,
) -> Result<CodeAstAnalysisResponse, StudioApiError> {
    let cwd = state.studio.project_root.clone();
    let repositories = configured_repositories(&state.studio);
    let (repository, repo_relative_path) =
        resolve_code_ast_repository_and_path(&repositories, Some(repo_id), path)?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);

    let repo_id = repository.id.clone();
    let request_path = path.to_string();
    let repo_path = repo_relative_path;
    let repository = repository.clone();

    tokio::task::spawn_blocking(
        move || -> Result<CodeAstAnalysisResponse, crate::analyzers::RepoIntelligenceError> {
            let analysis = analyze_registered_repository_with_registry(
                &repository,
                cwd.as_path(),
                &plugin_registry,
            )?;
            let source_content = repository.path.as_ref().and_then(|root| {
                let source_path = root.join(&repo_path);
                source_path
                    .is_file()
                    .then(|| fs::read_to_string(source_path).ok())
                    .flatten()
            });
            let mut response = build_code_ast_analysis_response(
                repo_id,
                repo_path,
                line_hint,
                source_content.as_deref(),
                &analysis,
            );
            response.path = request_path;
            Ok(response)
        },
    )
    .await
    .map_err(|error: tokio::task::JoinError| {
        StudioApiError::internal(
            "CODE_AST_PANIC",
            "Code AST analysis task failed unexpectedly",
            Some(error.to_string()),
        )
    })?
    .map_err(map_repo_intelligence_error)
}
