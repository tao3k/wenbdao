use crate::analyzers::{RepoSymbolKind, RepositoryAnalysisOutput};
use crate::gateway::studio::router::code_ast::atoms::build_code_ast_retrieval_atom;
use crate::gateway::studio::router::code_ast::blocks::build_code_block_retrieval_atoms;
use crate::gateway::studio::router::code_ast::resolve::{
    focus_symbol_for_blocks, path_has_extension, repo_relative_path_matches,
    retrieval_semantic_type,
};
use crate::gateway::studio::types::{
    CodeAstAnalysisResponse, CodeAstEdge, CodeAstEdgeKind, CodeAstNode, CodeAstNodeKind,
    CodeAstProjection, CodeAstProjectionKind, CodeAstRetrievalAtomScope,
};

/// Build the code-AST response payload for one repository-relative source path.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_code_ast_analysis_response(
    repo_id: String,
    path: String,
    line_hint: Option<usize>,
    source_content: Option<&str>,
    analysis: &RepositoryAnalysisOutput,
) -> CodeAstAnalysisResponse {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut retrieval_atoms = Vec::new();
    let mut contains_edge_count = 0usize;
    let mut uses_edge_count = 0usize;
    let mut interaction_edge_count = 0usize;

    // Convert modules to nodes
    for module in &analysis.modules {
        let line = Some(1usize);
        nodes.push(CodeAstNode {
            id: module.module_id.clone(),
            label: module.qualified_name.clone(),
            kind: CodeAstNodeKind::Module,
            path: Some(module.path.clone()),
            line,
        });
        let content = format!("{}|{}", module.qualified_name, module.path);
        let declaration_locator = format!("l{}", line.unwrap_or(0));
        let symbol_locator = format!("{}-l{}", module.qualified_name, line.unwrap_or(0));
        retrieval_atoms.push(build_code_ast_retrieval_atom(
            module.module_id.as_str(),
            module.path.as_str(),
            CodeAstRetrievalAtomScope::Declaration,
            "module",
            declaration_locator.as_str(),
            content.as_str(),
        ));
        retrieval_atoms.push(build_code_ast_retrieval_atom(
            module.module_id.as_str(),
            module.path.as_str(),
            CodeAstRetrievalAtomScope::Symbol,
            "module",
            symbol_locator.as_str(),
            content.as_str(),
        ));
    }

    // Convert symbols to nodes
    for symbol in &analysis.symbols {
        let same_file = repo_relative_path_matches(symbol.path.as_str(), path.as_str());
        let kind = if same_file {
            match symbol.kind {
                RepoSymbolKind::Function => CodeAstNodeKind::Function,
                RepoSymbolKind::Type => CodeAstNodeKind::Type,
                RepoSymbolKind::Constant => CodeAstNodeKind::Constant,
                _ => CodeAstNodeKind::Other,
            }
        } else {
            CodeAstNodeKind::ExternalSymbol
        };
        nodes.push(CodeAstNode {
            id: symbol.symbol_id.clone(),
            label: symbol.name.clone(),
            kind,
            path: Some(symbol.path.clone()),
            line: symbol.line_start,
        });
        let semantic_type = retrieval_semantic_type(symbol.kind, same_file);
        let declaration_locator = format!("l{}", symbol.line_start.unwrap_or(0));
        let symbol_locator = format!("{}-l{}", symbol.name, symbol.line_start.unwrap_or(0));
        let content = format!(
            "{}|{}|{}|{}",
            symbol.qualified_name,
            symbol.path,
            semantic_type,
            symbol.signature.as_deref().unwrap_or(symbol.name.as_str())
        );

        if same_file {
            retrieval_atoms.push(build_code_ast_retrieval_atom(
                symbol.symbol_id.as_str(),
                symbol.path.as_str(),
                CodeAstRetrievalAtomScope::Declaration,
                semantic_type,
                declaration_locator.as_str(),
                content.as_str(),
            ));
        }

        retrieval_atoms.push(build_code_ast_retrieval_atom(
            symbol.symbol_id.as_str(),
            symbol.path.as_str(),
            CodeAstRetrievalAtomScope::Symbol,
            semantic_type,
            symbol_locator.as_str(),
            content.as_str(),
        ));
    }

    if let Some(primary_symbol) = focus_symbol_for_blocks(line_hint, analysis, path.as_str()) {
        if let Some(content) = source_content {
            retrieval_atoms.extend(build_code_block_retrieval_atoms(
                path.as_str(),
                primary_symbol.line_start,
                content,
            ));
        }
    }

    // Convert relations to edges
    for relation in &analysis.relations {
        let kind = match relation.kind {
            crate::analyzers::RelationKind::Contains => {
                contains_edge_count += 1;
                CodeAstEdgeKind::Contains
            }
            crate::analyzers::RelationKind::Calls => {
                interaction_edge_count += 1;
                CodeAstEdgeKind::Calls
            }
            crate::analyzers::RelationKind::Uses => {
                interaction_edge_count += 1;
                uses_edge_count += 1;
                CodeAstEdgeKind::Uses
            }
            crate::analyzers::RelationKind::Imports => {
                interaction_edge_count += 1;
                CodeAstEdgeKind::Imports
            }
            _ => CodeAstEdgeKind::Other,
        };
        edges.push(CodeAstEdge {
            id: format!(
                "{}-{}-{}",
                relation.source_id, relation.target_id, relation.kind as u8
            ),
            source_id: relation.source_id.clone(),
            target_id: relation.target_id.clone(),
            kind,
            label: None,
        });
    }

    let language = if path_has_extension(path.as_str(), "jl") {
        "julia"
    } else {
        "modelica"
    };
    let focus_node_id = line_hint
        .and_then(|line| {
            analysis.symbols.iter().find(|symbol| {
                if !repo_relative_path_matches(symbol.path.as_str(), path.as_str()) {
                    return false;
                }
                match (symbol.line_start, symbol.line_end) {
                    (Some(start), Some(end)) => start <= line && line <= end,
                    (Some(start), None) => start == line,
                    _ => false,
                }
            })
        })
        .or_else(|| {
            line_hint.and_then(|_| {
                analysis
                    .symbols
                    .iter()
                    .find(|symbol| repo_relative_path_matches(symbol.path.as_str(), path.as_str()))
            })
        })
        .map(|symbol| symbol.symbol_id.clone());
    let projections = vec![
        CodeAstProjection {
            kind: CodeAstProjectionKind::Contains,
            node_count: nodes.len(),
            edge_count: contains_edge_count,
        },
        CodeAstProjection {
            kind: CodeAstProjectionKind::Calls,
            node_count: nodes.len(),
            edge_count: interaction_edge_count,
        },
        CodeAstProjection {
            kind: CodeAstProjectionKind::Uses,
            node_count: nodes.len(),
            edge_count: uses_edge_count,
        },
    ];

    CodeAstAnalysisResponse {
        repo_id,
        path,
        language: language.to_string(),
        nodes,
        edges,
        projections,
        retrieval_atoms,
        focus_node_id,
        diagnostics: Vec::new(),
    }
}
