//! Graph collapse operators for knowledge distillation.
//!
//! Collapses dense clusters into VirtualNodes that:
//! 1. Inherit all outgoing/incoming edges of member nodes
//! 2. Store references to original member IDs
//! 3. Get synthesized stem/title from cluster essence
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::index::build::collapse::{collapse_clusters, VirtualNode};
//!
//! let virtual_nodes = collapse_clusters(&clusters, &docs_by_id, &mut outgoing, &mut incoming);
//! ```

use super::cluster_finder::DenseCluster;
use crate::link_graph::models::LinkGraphDocument;
use std::collections::{HashMap, HashSet};
use std::hash::Hasher;

/// A virtual node created from collapsing a dense cluster.
#[derive(Debug, Clone)]
pub struct VirtualNode {
    /// Synthesized identifier for the virtual node.
    pub id: String,
    /// Original member node IDs that were collapsed.
    pub members: Vec<String>,
    /// Average saliency of collapsed nodes.
    pub avg_saliency: f64,
    /// Synthesized title (e.g., "Cluster: essence-topic").
    pub title: String,
    /// All outgoing edges from members to non-members.
    pub outgoing_edges: HashSet<String>,
    /// All incoming edges from non-members to members.
    pub incoming_edges: HashSet<String>,
}

impl VirtualNode {
    /// Generate a virtual node ID from cluster members.
    #[must_use]
    pub fn generate_id(members: &[String], cluster_index: usize) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::default();
        for m in members {
            hasher.write(m.as_bytes());
        }
        let hash_val = hasher.finish();
        format!("virtual:cluster:{}:{:08x}", cluster_index, hash_val)
    }

    /// Generate a title from member titles (first 3 words of top member).
    #[must_use]
    pub fn synthesize_title(member_titles: &[&str]) -> String {
        if member_titles.is_empty() {
            return "Collapsed Cluster".to_string();
        }

        // Take first 3 words from all member titles
        let words: Vec<&str> = member_titles
            .iter()
            .flat_map(|t| t.split_whitespace().take(3))
            .collect();
        format!("Cluster: {}", words.join(" "))
    }
}

/// Collapse dense clusters into virtual nodes.
///
/// # Arguments
/// * `clusters` - Dense clusters to collapse
/// * `docs_by_id` - Document map (read-only, used for title extraction)
/// * `outgoing` - Outgoing edge map (will be modified)
/// * `incoming` - Incoming edge map (will be modified)
///
/// # Returns
/// Vector of VirtualNodes created
pub fn collapse_clusters(
    clusters: Vec<DenseCluster>,
    docs_by_id: &HashMap<String, LinkGraphDocument>,
    outgoing: &mut HashMap<String, HashSet<String>>,
    incoming: &mut HashMap<String, HashSet<String>>,
) -> Vec<VirtualNode> {
    if clusters.is_empty() {
        return Vec::new();
    }

    let mut virtual_nodes: Vec<VirtualNode> = Vec::new();
    let mut cluster_info: Vec<(DenseCluster, String)> = Vec::new();

    for (cluster_index, cluster) in clusters.into_iter().enumerate() {
        let member_set: HashSet<&String> = cluster.members.iter().collect();

        // Collect edges
        let mut outgoing_edges: HashSet<String> = HashSet::new();
        let mut incoming_edges: HashSet<String> = HashSet::new();

        for member_id in &cluster.members {
            // Outgoing edges to non-members
            if let Some(neighbors) = outgoing.get(member_id) {
                for neighbor in neighbors {
                    if !member_set.contains(neighbor) {
                        outgoing_edges.insert(neighbor.clone());
                    }
                }
            }

            // Incoming edges from non-members
            if let Some(neighbors) = incoming.get(member_id) {
                for neighbor in neighbors {
                    if !member_set.contains(neighbor) {
                        incoming_edges.insert(neighbor.clone());
                    }
                }
            }
        }

        // Get member titles for title synthesis (use stem as title proxy)
        let member_titles: Vec<&str> = cluster
            .members
            .iter()
            .filter_map(|id| docs_by_id.get(id).map(|doc| doc.stem.as_str()))
            .collect();

        let virtual_id = VirtualNode::generate_id(&cluster.members, cluster_index);

        let virtual_node = VirtualNode {
            id: virtual_id.clone(),
            members: cluster.members.clone(),
            avg_saliency: cluster.avg_saliency,
            title: VirtualNode::synthesize_title(&member_titles),
            outgoing_edges: outgoing_edges.clone(),
            incoming_edges: incoming_edges.clone(),
        };

        virtual_nodes.push(virtual_node);
        cluster_info.push((cluster, virtual_id));
    }

    // Rewire edges: remove member-to-member edges, add virtual node edges
    for (cluster, virtual_id) in &cluster_info {
        let member_set: HashSet<&String> = cluster.members.iter().collect();

        // Remove internal edges between members
        for member_id in &cluster.members {
            if let Some(neighbors) = outgoing.get_mut(member_id) {
                neighbors.retain(|n| !member_set.contains(n));
            }
            if let Some(neighbors) = incoming.get_mut(member_id) {
                neighbors.retain(|n| !member_set.contains(n));
            }
        }

        // Get virtual node edges
        let Some(vnode) = virtual_nodes.iter().find(|vn| vn.id == *virtual_id) else {
            debug_assert!(false, "virtual node should exist");
            continue;
        };

        // Add edges from virtual node to external nodes
        outgoing
            .entry(virtual_id.clone())
            .or_default()
            .extend(vnode.outgoing_edges.iter().cloned());

        incoming
            .entry(virtual_id.clone())
            .or_default()
            .extend(vnode.incoming_edges.iter().cloned());

        // Add reverse edges from external nodes to virtual node
        for ext_node in &vnode.outgoing_edges {
            incoming
                .entry(ext_node.clone())
                .or_default()
                .insert(virtual_id.clone());
        }

        for ext_node in &vnode.incoming_edges {
            outgoing
                .entry(ext_node.clone())
                .or_default()
                .insert(virtual_id.clone());
        }
    }

    virtual_nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cluster(members: Vec<&str>, avg_saliency: f64) -> DenseCluster {
        DenseCluster {
            members: members.iter().map(|s| s.to_string()).collect(),
            avg_saliency,
            internal_edges: members.len() * 2,
            edge_density: 0.5,
        }
    }

    fn make_doc(id: &str, stem: &str) -> LinkGraphDocument {
        LinkGraphDocument {
            id: id.to_string(),
            id_lower: id.to_lowercase(),
            stem: stem.to_string(),
            stem_lower: stem.to_lowercase(),
            path: format!("{}.md", id),
            path_lower: format!("{}.md", id.to_lowercase()),
            title: stem.to_string(),
            title_lower: stem.to_lowercase(),
            tags: Vec::new(),
            tags_lower: Vec::new(),
            lead: String::new(),
            doc_type: None,
            word_count: 0,
            search_text: String::new(),
            search_text_lower: String::new(),
            saliency_base: 0.5,
            decay_rate: 0.1,
            created_ts: None,
            modified_ts: None,
        }
    }

    #[test]
    fn test_virtual_node_id_generation() {
        let members = vec!["a.md".to_string(), "b.md".to_string(), "c.md".to_string()];
        let id = VirtualNode::generate_id(&members, 0);
        assert!(id.starts_with("virtual:cluster:0:"));
    }

    #[test]
    fn test_virtual_node_title_synthesis() {
        let titles = vec!["Understanding Performance Optimization"];
        let title = VirtualNode::synthesize_title(&titles);
        assert!(title.contains("Cluster:"));
    }

    #[test]
    fn test_collapse_empty_clusters() {
        let docs_by_id = HashMap::new();
        let mut outgoing = HashMap::new();
        let mut incoming = HashMap::new();

        let result = collapse_clusters(vec![], &docs_by_id, &mut outgoing, &mut incoming);
        assert!(result.is_empty());
    }

    #[test]
    fn test_collapse_single_cluster() {
        let docs_by_id: HashMap<String, LinkGraphDocument> = [
            ("a.md".to_string(), make_doc("a.md", "Doc A")),
            ("b.md".to_string(), make_doc("b.md", "Doc B")),
            ("c.md".to_string(), make_doc("c.md", "Doc C")),
        ]
        .into_iter()
        .collect();
        let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
        let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();

        // Setup: a -> d (external), b -> c (internal, c -> e (external)
        outgoing.insert(
            "a.md".to_string(),
            ["d.md".to_string()].into_iter().collect(),
        );
        outgoing.insert(
            "b.md".to_string(),
            ["c.md".to_string()].into_iter().collect(),
        );
        outgoing.insert(
            "c.md".to_string(),
            ["e.md".to_string()].into_iter().collect(),
        );

        // b has incoming from x (external)
        incoming.insert(
            "b.md".to_string(),
            ["x.md".to_string()].into_iter().collect(),
        );

        let cluster = make_cluster(vec!["a.md", "b.md", "c.md"], 0.85);
        let result = collapse_clusters(vec![cluster], &docs_by_id, &mut outgoing, &mut incoming);

        assert_eq!(result.len(), 1);
        let vn = &result[0];
        assert_eq!(vn.members.len(), 3);
        assert!((vn.avg_saliency - 0.85).abs() < 0.01);

        // Check edge rewiring
        assert!(vn.outgoing_edges.contains("d.md"));
        assert!(vn.outgoing_edges.contains("e.md"));
        assert!(vn.incoming_edges.contains("x.md"));
    }
}
