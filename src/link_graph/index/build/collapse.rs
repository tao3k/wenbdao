//! Graph collapse operators for knowledge distillation.
//!
//! Collapses dense clusters into `VirtualNodes` that:
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
    /// Internal edge count (edges between members).
    pub internal_edges: usize,
    /// Edge density within cluster.
    pub edge_density: f64,
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
        format!("virtual:cluster:{cluster_index}:{hash_val:08x}")
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
/// Vector of `VirtualNodes` created
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
            internal_edges: cluster.internal_edges,
            edge_density: cluster.edge_density,
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
#[path = "../../../../tests/unit/link_graph/index/build/collapse.rs"]
mod tests;
