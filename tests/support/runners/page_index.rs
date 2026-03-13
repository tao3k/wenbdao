//! Runner for page_index scenario tests.

use std::error::Error;
use std::path::Path;

use serde_json::{Value, json};
use xiuxian_testing::{Scenario, ScenarioRunner, find_first_doc_name};
use xiuxian_wendao::LinkGraphIndex;
use xiuxian_wendao::link_graph::PageIndexNode;

/// Runner for `page_index` category scenarios.
pub struct PageIndexRunner;

impl ScenarioRunner for PageIndexRunner {
    fn category(&self) -> &str {
        "page_index"
    }

    fn run(&self, _scenario: &Scenario, temp_dir: &Path) -> Result<Value, Box<dyn Error>> {
        // Build the link graph index
        let index = LinkGraphIndex::build(temp_dir)?;

        // Get the document name from input
        let doc_name = find_first_doc_name(temp_dir)?;

        // Get the page index roots
        let roots = index
            .page_index(&doc_name)
            .ok_or_else(|| format!("missing page index for {}", doc_name))?;

        // Generate snapshot
        Ok(page_index_tree_snapshot(roots))
    }
}

/// Generate a JSON snapshot of page index nodes.
fn page_index_tree_snapshot(nodes: &[PageIndexNode]) -> Value {
    json!({
        "nodes": nodes.iter().map(snapshot_page_index_node).collect::<Vec<_>>(),
    })
}

fn snapshot_page_index_node(node: &PageIndexNode) -> Value {
    json!({
        "node_id": node.node_id,
        "title": node.title,
        "level": node.level,
        "text": node.text.as_ref(),
        "summary": node.summary,
        "line_range": [node.metadata.line_range.0, node.metadata.line_range.1],
        "token_count": node.metadata.token_count,
        "is_thinned": node.metadata.is_thinned,
        "children": node.children.iter().map(snapshot_page_index_node).collect::<Vec<_>>(),
    })
}
