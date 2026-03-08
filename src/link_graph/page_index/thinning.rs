use crate::link_graph::models::PageIndexNode;
use std::sync::Arc;

/// Default pruning threshold for folding tiny parent sections into a single chunk.
pub(in crate::link_graph) const DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD: usize = 12;

/// Thin a page tree in place by folding descendants into undersized parent nodes.
pub(in crate::link_graph) fn thin_page_index_tree(nodes: &mut [PageIndexNode], threshold: usize) {
    let effective_threshold = threshold.max(1);
    for node in nodes {
        thin_node(node, effective_threshold);
    }
}

fn thin_node(node: &mut PageIndexNode, threshold: usize) {
    for child in &mut node.children {
        thin_node(child, threshold);
    }

    if node.children.is_empty() || node.metadata.token_count >= threshold {
        return;
    }

    let merged_text = merged_text(node);
    let merged_range = merged_line_range(node);
    node.text = Arc::<str>::from(merged_text.as_str());
    node.metadata.line_range = merged_range;
    node.metadata.token_count = count_tokens(&merged_text);
    node.metadata.is_thinned = true;
    node.children.clear();
}

fn merged_text(node: &PageIndexNode) -> String {
    let mut segments = Vec::with_capacity(node.children.len().saturating_add(1));
    let own = node.text.trim();
    if !own.is_empty() {
        segments.push(own.to_string());
    }
    for child in &node.children {
        let child_text = child.text.trim();
        if child_text.is_empty() {
            segments.push(child.title.clone());
        } else {
            segments.push(format!("{}\n{}", child.title, child_text));
        }
    }
    segments.join("\n\n")
}

fn merged_line_range(node: &PageIndexNode) -> (usize, usize) {
    node.children
        .iter()
        .fold(node.metadata.line_range, |range, child| {
            (
                range.0.min(child.metadata.line_range.0),
                range.1.max(child.metadata.line_range.1),
            )
        })
}

fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}
