use std::collections::{HashMap, HashSet};

use crate::link_graph::index::build::saliency_snapshot::SaliencySnapshot;
use crate::link_graph::index::build::{
    cluster_finder::find_dense_clusters, collapse::collapse_clusters,
};
use crate::link_graph::models::LinkGraphDocument;

pub(crate) fn build_virtual_nodes(
    docs_by_id: &HashMap<String, LinkGraphDocument>,
    outgoing: &mut HashMap<String, HashSet<String>>,
    incoming: &mut HashMap<String, HashSet<String>>,
    saliency_snapshot: Option<SaliencySnapshot>,
) -> HashMap<String, crate::link_graph::index::build::VirtualNode> {
    let Some(snapshot) = saliency_snapshot else {
        return HashMap::new();
    };

    let saliency_map: HashMap<String, f64> = snapshot
        .states
        .iter()
        .map(|(k, v)| (k.clone(), v.current_saliency))
        .collect();
    let clusters = find_dense_clusters(
        &snapshot.high_saliency_nodes,
        outgoing,
        incoming,
        &saliency_map,
    );

    collapse_clusters(clusters, docs_by_id, outgoing, incoming)
        .into_iter()
        .map(|vn| (vn.id.clone(), vn))
        .collect()
}
