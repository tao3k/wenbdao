use super::super::router::StudioState;
use super::super::types::{AnalysisNode, MarkdownAnalysisResponse};
use super::super::vfs;
use super::markdown;
use super::projection;

#[derive(Debug)]
pub(crate) enum AnalysisError {
    UnsupportedContentType(String),
    Vfs(vfs::VfsError),
}

impl From<vfs::VfsError> for AnalysisError {
    fn from(value: vfs::VfsError) -> Self {
        Self::Vfs(value)
    }
}

pub(crate) fn compile_markdown_nodes(path: &str, content: &str) -> Vec<AnalysisNode> {
    markdown::compile_markdown_ir(path, content).nodes
}

pub(crate) async fn analyze_markdown(
    state: &StudioState,
    path: &str,
) -> Result<MarkdownAnalysisResponse, AnalysisError> {
    let content = vfs::read_content(state, path).await?;
    if content.content_type != "text/markdown" {
        return Err(AnalysisError::UnsupportedContentType(content.content_type));
    }

    let mut compiled = markdown::compile_markdown_ir(path, content.content.as_str());
    markdown::enrich_property_drawers(path, content.content.as_str(), &mut compiled);
    let projections = projection::build_mermaid_projections(path, &compiled.nodes, &compiled.edges);

    Ok(MarkdownAnalysisResponse {
        path: path.to_string(),
        document_hash: compiled.document_hash,
        node_count: compiled.nodes.len(),
        edge_count: compiled.edges.len(),
        nodes: compiled.nodes,
        edges: compiled.edges,
        projections,
        diagnostics: compiled.diagnostics,
    })
}
