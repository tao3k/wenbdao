//! Dense cluster identification for knowledge distillation.
//!
//! Finds subgraphs where nodes have:
//! 1. High saliency (>= `SALIENCY_THRESHOLD_HIGH`)
//! 2. Strong mutual linking (edge density >= `MIN_EDGE_DENSITY`)
//!
//! ## Algorithm
//!
//! Uses a greedy expansion approach:
//! 1. Start from highest-saliency seed nodes
//! 2. Expand to neighbors if they maintain density threshold
//! 3. Stop when no more qualifying neighbors exist
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::index::build::cluster_finder::{find_dense_clusters, DenseCluster};
//!
//! let clusters = find_dense_clusters(
//!     &high_saliency_nodes,
//!     &outgoing,
//!     &incoming,
//!     &saliency_map,
//! );
//! ```

use super::saliency_snapshot::SALIENCY_THRESHOLD_HIGH;
use std::collections::{HashMap, HashSet};

/// Minimum cluster size (nodes).
pub const MIN_CLUSTER_SIZE: usize = 3;

/// Maximum cluster size (prevents over-expansion).
pub const MAX_CLUSTER_SIZE: usize = 15;

/// Minimum internal edge density for cluster validity.
pub const MIN_EDGE_DENSITY: f64 = 0.4;

/// A dense cluster of high-saliency nodes.
#[derive(Debug, Clone)]
pub struct DenseCluster {
    /// Node IDs in the cluster.
    pub members: Vec<String>,
    /// Average saliency of members.
    pub avg_saliency: f64,
    /// Internal edge count (edges between members).
    pub internal_edges: usize,
    /// Edge density within cluster.
    pub edge_density: f64,
}

impl DenseCluster {
    /// Create a new cluster with the given members.
    #[must_use]
    pub fn new(
        members: Vec<String>,
        saliency_map: &HashMap<String, f64>,
        outgoing: &HashMap<String, HashSet<String>>,
    ) -> Self {
        let avg_saliency = if members.is_empty() {
            0.0
        } else {
            members
                .iter()
                .filter_map(|id| saliency_map.get(id))
                .sum::<f64>()
                / members.len() as f64
        };

        // Count internal edges
        let member_set: HashSet<&String> = members.iter().collect();
        let mut internal_edges = 0usize;
        for member in &members {
            if let Some(neighbors) = outgoing.get(member) {
                internal_edges += neighbors.iter().filter(|n| member_set.contains(*n)).count();
            }
        }

        // Edge density = actual_edges / possible_edges
        // possible_edges = n * (n-1) for directed graph
        let n = members.len();
        let possible_edges = if n > 1 { n * (n - 1) } else { 1 };
        let edge_density = internal_edges as f64 / possible_edges as f64;

        Self {
            members,
            avg_saliency,
            internal_edges,
            edge_density,
        }
    }

    /// Check if cluster meets validity criteria.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.members.len() >= MIN_CLUSTER_SIZE
            && self.edge_density >= MIN_EDGE_DENSITY
            && self.avg_saliency >= SALIENCY_THRESHOLD_HIGH
    }
}

/// Find dense clusters in the graph using greedy expansion.
///
/// # Arguments
/// * `high_saliency_nodes` - Nodes that exceed the saliency threshold
/// * `outgoing` - Map from node_id to its outgoing edge targets
/// * `incoming` - Map from node_id to its incoming edge sources
/// * `saliency_map` - Map from node_id to its saliency value
///
/// # Returns
/// List of valid dense clusters, sorted by average saliency (descending).
#[must_use]
pub fn find_dense_clusters(
    high_saliency_nodes: &[String],
    outgoing: &HashMap<String, HashSet<String>>,
    incoming: &HashMap<String, HashSet<String>>,
    saliency_map: &HashMap<String, f64>,
) -> Vec<DenseCluster> {
    if high_saliency_nodes.len() < MIN_CLUSTER_SIZE {
        return Vec::new();
    }

    let high_set: HashSet<&String> = high_saliency_nodes.iter().collect();
    let mut visited: HashSet<String> = HashSet::new();
    let mut clusters: Vec<DenseCluster> = Vec::new();

    // Sort high-saliency nodes by saliency (descending)
    let mut sorted_seeds: Vec<&String> = high_saliency_nodes.iter().collect();
    sorted_seeds.sort_by(|a, b| {
        let sa = saliency_map.get(*a).unwrap_or(&0.0);
        let sb = saliency_map.get(*b).unwrap_or(&0.0);
        sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Greedy expansion from each seed
    for seed in sorted_seeds {
        if visited.contains(seed) {
            continue;
        }

        let cluster = expand_cluster(seed, &high_set, &visited, outgoing, incoming, saliency_map);

        if cluster.members.len() >= MIN_CLUSTER_SIZE {
            // Mark all members as visited
            for member in &cluster.members {
                visited.insert(member.clone());
            }

            if cluster.is_valid() {
                clusters.push(cluster);
            }
        }
    }

    // Sort by average saliency descending
    clusters.sort_by(|a, b| {
        b.avg_saliency
            .partial_cmp(&a.avg_saliency)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    clusters
}

/// Expand a cluster from a seed node using greedy density optimization.
fn expand_cluster(
    seed: &str,
    high_set: &HashSet<&String>,
    visited: &HashSet<String>,
    outgoing: &HashMap<String, HashSet<String>>,
    incoming: &HashMap<String, HashSet<String>>,
    saliency_map: &HashMap<String, f64>,
) -> DenseCluster {
    let mut members: HashSet<String> = HashSet::new();
    members.insert(seed.to_string());

    // Get all neighbors that are high-saliency and not visited
    let get_candidates = |members: &HashSet<String>| -> Vec<String> {
        let mut candidates: HashSet<String> = HashSet::new();
        for member in members {
            // Check outgoing neighbors
            if let Some(neighbors) = outgoing.get(member) {
                for n in neighbors {
                    if high_set.contains(&n) && !visited.contains(n) && !members.contains(n) {
                        candidates.insert(n.clone());
                    }
                }
            }
            // Check incoming neighbors
            if let Some(neighbors) = incoming.get(member) {
                for n in neighbors {
                    if high_set.contains(&n) && !visited.contains(n) && !members.contains(n) {
                        candidates.insert(n.clone());
                    }
                }
            }
        }
        candidates.into_iter().collect()
    };

    // Greedy expansion: add candidate that maximizes density
    while members.len() < MAX_CLUSTER_SIZE {
        let candidates = get_candidates(&members);
        if candidates.is_empty() {
            break;
        }

        // Find best candidate (maintains highest density)
        let mut best_candidate: Option<String> = None;
        let mut best_density = 0.0;

        for candidate in &candidates {
            let mut test_members = members.clone();
            test_members.insert(candidate.clone());

            let density = compute_edge_density(&test_members, outgoing);
            if density > best_density {
                best_density = density;
                best_candidate = Some(candidate.clone());
            }
        }

        // Add best candidate if it maintains minimum density
        if let Some(candidate) = best_candidate {
            if best_density >= MIN_EDGE_DENSITY {
                members.insert(candidate);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    DenseCluster::new(members.into_iter().collect(), saliency_map, outgoing)
}

/// Compute edge density within a member set.
fn compute_edge_density(
    members: &HashSet<String>,
    outgoing: &HashMap<String, HashSet<String>>,
) -> f64 {
    if members.len() < 2 {
        return 1.0;
    }

    let mut internal_edges = 0usize;
    for member in members {
        if let Some(neighbors) = outgoing.get(member) {
            internal_edges += neighbors.intersection(members).count();
        }
    }

    let n = members.len();
    let possible_edges = n * (n - 1);
    internal_edges as f64 / possible_edges as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_saliency_map(nodes: &[(&str, f64)]) -> HashMap<String, f64> {
        nodes.iter().map(|(id, s)| (id.to_string(), *s)).collect()
    }

    fn make_edge_map(edges: &[(&str, &str)]) -> HashMap<String, HashSet<String>> {
        let mut map: HashMap<String, HashSet<String>> = HashMap::new();
        for (from, to) in edges {
            map.entry(from.to_string())
                .or_default()
                .insert(to.to_string());
        }
        map
    }

    #[test]
    fn test_empty_cluster() {
        let saliency = make_saliency_map(&[]);
        let outgoing = HashMap::new();
        let cluster = DenseCluster::new(vec![], &saliency, &outgoing);
        assert_eq!(cluster.members.len(), 0);
        assert_eq!(cluster.avg_saliency, 0.0);
        assert!(!cluster.is_valid());
    }

    #[test]
    fn test_cluster_validity() {
        let saliency = make_saliency_map(&[("a", 0.8), ("b", 0.85), ("c", 0.9)]);
        // a -> b, b -> c, c -> a (density = 3/6 = 0.5)
        let outgoing = make_edge_map(&[("a", "b"), ("b", "c"), ("c", "a")]);
        let cluster = DenseCluster::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            &saliency,
            &outgoing,
        );
        assert!(cluster.is_valid());
        assert!((cluster.edge_density - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_low_density_cluster_invalid() {
        let saliency = make_saliency_map(&[("a", 0.8), ("b", 0.85), ("c", 0.9)]);
        // Only a -> b (density = 1/6 = 0.167)
        let outgoing = make_edge_map(&[("a", "b")]);
        let cluster = DenseCluster::new(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            &saliency,
            &outgoing,
        );
        assert!(!cluster.is_valid()); // density < MIN_EDGE_DENSITY
    }

    #[test]
    fn test_find_clusters_insufficient_nodes() {
        let high = vec!["a".to_string(), "b".to_string()];
        let outgoing = HashMap::new();
        let incoming = HashMap::new();
        let saliency = make_saliency_map(&[("a", 0.8), ("b", 0.85)]);

        let clusters = find_dense_clusters(&high, &outgoing, &incoming, &saliency);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_min_cluster_size_constant() {
        assert_eq!(MIN_CLUSTER_SIZE, 3);
    }

    #[test]
    fn test_min_edge_density_constant() {
        assert!((MIN_EDGE_DENSITY - 0.4).abs() < f64::EPSILON);
    }
}
