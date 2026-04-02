use std::sync::Arc;

pub(crate) fn test_studio_state() -> crate::gateway::studio::router::StudioState {
    let nonce = format!(
        "search-plane-handlers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|error| panic!("system time before unix epoch: {error}"))
            .as_nanos()
    );
    let search_plane_root = std::env::temp_dir().join(nonce);
    crate::gateway::studio::router::StudioState::new_with_bootstrap_ui_config_and_search_plane_root(
        Arc::new(
            crate::analyzers::bootstrap_builtin_registry()
                .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
        ),
        search_plane_root,
    )
}

pub(crate) async fn publish_repo_content_chunk_index(
    studio: &crate::gateway::studio::router::StudioState,
    repo_id: &str,
    documents: Vec<crate::gateway::studio::repo_index::RepoCodeDocument>,
) {
    studio
        .search_plane
        .publish_repo_content_chunks_with_revision(repo_id, &documents, None)
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
}

pub(crate) async fn publish_repo_entity_index(
    studio: &crate::gateway::studio::router::StudioState,
    repo_id: &str,
    analysis: &crate::analyzers::RepositoryAnalysisOutput,
) {
    studio
        .search_plane
        .publish_repo_entities_with_revision(repo_id, analysis, &sample_repo_documents(), None)
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
}

pub(crate) fn sample_repo_analysis(repo_id: &str) -> crate::analyzers::RepositoryAnalysisOutput {
    crate::analyzers::RepositoryAnalysisOutput {
        modules: vec![crate::analyzers::ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![crate::analyzers::SymbolRecord {
            repo_id: repo_id.to_string(),
            symbol_id: "symbol:reexport".to_string(),
            module_id: Some("module:BaseModelica".to_string()),
            name: "reexport".to_string(),
            qualified_name: "BaseModelica.reexport".to_string(),
            kind: crate::analyzers::RepoSymbolKind::Function,
            path: "src/BaseModelica.jl".to_string(),
            line_start: Some(7),
            line_end: Some(9),
            signature: Some("reexport()".to_string()),
            audit_status: Some("verified".to_string()),
            verification_state: Some("verified".to_string()),
            attributes: std::collections::BTreeMap::new(),
        }],
        examples: vec![crate::analyzers::ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: "example:reexport".to_string(),
            title: "Reexport example".to_string(),
            path: "examples/reexport.jl".to_string(),
            summary: Some("Shows how to reexport ModelingToolkit".to_string()),
        }],
        ..crate::analyzers::RepositoryAnalysisOutput::default()
    }
}

pub(crate) fn sample_repo_documents() -> Vec<crate::gateway::studio::repo_index::RepoCodeDocument> {
    vec![
        crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "src/BaseModelica.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from(
                "module BaseModelica\nexport reexport\nreexport() = nothing\nend\n",
            ),
            size_bytes: 61,
            modified_unix_ms: 10,
        },
        crate::gateway::studio::repo_index::RepoCodeDocument {
            path: "examples/reexport.jl".to_string(),
            language: Some("julia".to_string()),
            contents: Arc::<str>::from("using BaseModelica\nreexport()\n"),
            size_bytes: 29,
            modified_unix_ms: 10,
        },
    ]
}
