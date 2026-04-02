use crate::link_graph::saliency::{edge_in_key, edge_out_key};

pub(super) fn update_inbound_edge_scores(
    conn: &mut redis::Connection,
    node_id: &str,
    key_prefix: &str,
    saliency_score: f64,
) {
    let inbound_key = edge_in_key(node_id, key_prefix);
    let inbound_sources = redis::cmd("SMEMBERS")
        .arg(&inbound_key)
        .query::<Vec<String>>(conn)
        .unwrap_or_default();
    for source in inbound_sources {
        let out_key = edge_out_key(source.trim(), key_prefix);
        let _ = redis::cmd("ZADD")
            .arg(&out_key)
            .arg(saliency_score)
            .arg(node_id)
            .query::<i64>(conn);
    }
}
