use std::collections::BTreeMap;

use crate::analyzers::{ModuleRecord, RelationRecord, RepoSymbolKind, SymbolRecord};
use crate::gateway::studio::router::tests::{repo_project, studio_with_repo_projects};
use crate::gateway::studio::router::{
    build_code_ast_analysis_response, configured_repositories, configured_repository,
};

#[test]
fn resolve_code_ast_repository_and_path_infers_repo_from_prefixed_path() {
    use crate::gateway::studio::router::code_ast::resolve_code_ast_repository_and_path;

    let studio = studio_with_repo_projects(vec![repo_project("sciml"), repo_project("mcl")]);
    let repositories = configured_repositories(&studio);
    let (repository, path) =
        resolve_code_ast_repository_and_path(&repositories, None, "sciml/src/BaseModelica.jl")
            .unwrap_or_else(|error| {
                panic!("repo should be inferred from prefixed path: {error:?}")
            });
    assert_eq!(repository.id, "sciml");
    assert_eq!(path, "src/BaseModelica.jl");
}

#[test]
fn resolve_code_ast_repository_and_path_strips_explicit_repo_prefix() {
    use crate::gateway::studio::router::code_ast::resolve_code_ast_repository_and_path;

    let studio = studio_with_repo_projects(vec![repo_project("kernel")]);
    let repositories = configured_repositories(&studio);
    let (repository, path) =
        resolve_code_ast_repository_and_path(&repositories, Some("kernel"), "kernel/src/lib.rs")
            .unwrap_or_else(|error| {
                panic!("explicit repo-scoped code AST path should normalize: {error:?}")
            });
    assert_eq!(repository.id, "kernel");
    assert_eq!(path, "src/lib.rs");
}

#[test]
fn resolve_code_ast_repository_and_path_rejects_conflicting_repo_prefix() {
    use crate::gateway::studio::router::code_ast::resolve_code_ast_repository_and_path;
    use axum::http::StatusCode;

    let studio = studio_with_repo_projects(vec![repo_project("kernel"), repo_project("main")]);
    let repositories = configured_repositories(&studio);
    let Err(error) =
        resolve_code_ast_repository_and_path(&repositories, Some("kernel"), "main/docs/index.md")
    else {
        panic!("conflicting repo-scoped code AST path should fail");
    };
    assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "REPO_PATH_MISMATCH");
}

#[test]
fn resolve_code_ast_repository_and_path_requires_repo_when_ambiguous() {
    use crate::gateway::studio::router::code_ast::resolve_code_ast_repository_and_path;
    use axum::http::StatusCode;

    let studio = studio_with_repo_projects(vec![repo_project("sciml"), repo_project("mcl")]);
    let repositories = configured_repositories(&studio);
    let Err(error) =
        resolve_code_ast_repository_and_path(&repositories, None, "src/BaseModelica.jl")
    else {
        panic!("should fail when repo cannot be inferred");
    };
    assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    assert_eq!(error.code(), "MISSING_REPO");
}

#[test]
fn configured_repository_matches_repo_identifier_case_insensitively() {
    let studio = studio_with_repo_projects(vec![repo_project("DifferentialEquations.jl")]);

    let repository = configured_repository(&studio, "differentialequations.jl")
        .unwrap_or_else(|error| panic!("repo lookup should ignore ASCII case: {error:?}"));

    assert_eq!(repository.id, "DifferentialEquations.jl");
}

#[test]
fn build_code_ast_analysis_response_emits_uses_projection_and_external_node() {
    use crate::gateway::studio::types::{
        CodeAstEdgeKind, CodeAstNodeKind, CodeAstProjectionKind, CodeAstRetrievalAtomScope,
    };

    let analysis = crate::analyzers::RepositoryAnalysisOutput {
        modules: vec![ModuleRecord {
            repo_id: "sciml".to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![
            SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:reexport".to_string(),
                module_id: Some("module:BaseModelica".to_string()),
                name: "reexport".to_string(),
                qualified_name: "BaseModelica.reexport".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/BaseModelica.jl".to_string(),
                line_start: Some(7),
                line_end: Some(9),
                signature: None,
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
            SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:ModelicaSystem".to_string(),
                module_id: None,
                name: "ModelicaSystem".to_string(),
                qualified_name: "ModelicaSystem".to_string(),
                kind: RepoSymbolKind::Type,
                path: "src/modelica/system.jl".to_string(),
                line_start: Some(1),
                line_end: Some(3),
                signature: None,
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
        ],
        relations: vec![RelationRecord {
            repo_id: "sciml".to_string(),
            source_id: "symbol:reexport".to_string(),
            target_id: "symbol:ModelicaSystem".to_string(),
            kind: crate::analyzers::RelationKind::Uses,
        }],
        ..crate::analyzers::RepositoryAnalysisOutput::default()
    };
    let payload = build_code_ast_analysis_response(
        "sciml".to_string(),
        "src/BaseModelica.jl".to_string(),
        Some(7),
        Some(
            "module BaseModelica\n\
\n\
# prelude\n\
# prelude\n\
# prelude\n\
# prelude\n\
pub fn reexport(\n\
    input,\n\
) {\n\
    if isempty(input)\n\
        return Err(Empty)\n\
    end\n\
\n\
    let meta = parse(input)\n\
\n\
    return Ok(meta)\n\
}\n",
        ),
        &analysis,
    );
    assert_eq!(payload.language, "julia");
    assert!(
        payload
            .nodes
            .iter()
            .any(|node| matches!(node.kind, CodeAstNodeKind::ExternalSymbol))
    );
    assert!(
        payload
            .edges
            .iter()
            .any(|edge| matches!(edge.kind, CodeAstEdgeKind::Uses))
    );
    assert!(payload.projections.iter().any(|projection| {
        matches!(projection.kind, CodeAstProjectionKind::Calls) && projection.edge_count > 0
    }));
    assert!(payload.focus_node_id.is_some());
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id == "symbol:reexport"
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Declaration))
            && atom
                .chunk_id
                .starts_with("ast:src-basemodelica-jl:declaration:function:")
            && atom.token_estimate > 0
    }));
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id == "symbol:ModelicaSystem"
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Symbol))
            && atom
                .chunk_id
                .starts_with("ast:src-modelica-system-jl:symbol:externalsymbol:")
            && atom.fingerprint.starts_with("fp:")
    }));
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id.starts_with("block:validation:")
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Block))
            && atom.semantic_type == "validation"
            && atom.line_start.is_some()
            && atom.line_end >= atom.line_start
    }));
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id.starts_with("block:return:")
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Block))
            && atom.semantic_type == "return"
            && atom.line_start.is_some()
            && atom.line_end >= atom.line_start
    }));
}
