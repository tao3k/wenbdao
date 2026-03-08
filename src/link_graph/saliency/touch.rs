use crate::link_graph::LinkGraphDisplayHit;
use crate::link_graph::saliency::{LinkGraphSaliencyTouchRequest, valkey_saliency_touch};
use std::thread;

/// Asynchronously touches a set of search hits to trigger saliency evolution.
///
/// This follows the Hebbian learning principle where frequently retrieved nodes
/// gain higher structural authority over time.
pub fn touch_search_hits_async(hits: &[LinkGraphDisplayHit]) {
    if hits.is_empty() {
        return;
    }

    let node_ids: Vec<String> = hits.iter().map(|h| h.stem.clone()).collect();

    // Spawn a background thread to avoid blocking the main search response.
    // In a production environment, this might use a dedicated task queue.
    thread::spawn(move || {
        for node_id in node_ids {
            let request = LinkGraphSaliencyTouchRequest {
                node_id,
                activation_delta: 1, // Basic boost for retrieval hit
                ..Default::default()
            };

            if let Err(err) = valkey_saliency_touch(request) {
                log::error!("Failed to touch search hit during evolution: {err}");
            }
        }
    });
}
