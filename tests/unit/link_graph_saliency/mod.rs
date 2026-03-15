//! Integration tests for `LinkGraph` saliency persistence and update behavior.

mod compute_link_graph_saliency_activation_boosts_score;
mod compute_link_graph_saliency_clamps_bounds;
// mod coactivation_touch_updates_neighbor;
mod saliency_store_auto_repairs_invalid_payload;
mod saliency_touch_and_get_with_valkey;
mod saliency_touch_updates_inbound_edge_zset;
mod support;
