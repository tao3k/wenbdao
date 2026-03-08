use super::runtime::now_unix_f64;
use super::schema::LINK_GRAPH_STATS_CACHE_SCHEMA_VERSION;
use super::types::LinkGraphStatsCachePayload;

pub(super) fn decode_stats_payload_if_fresh(
    raw: &str,
    source_key: &str,
    ttl_sec: f64,
) -> Option<LinkGraphStatsCachePayload> {
    let payload = serde_json::from_str::<LinkGraphStatsCachePayload>(raw).ok()?;
    if payload.schema != LINK_GRAPH_STATS_CACHE_SCHEMA_VERSION {
        return None;
    }
    if payload.source_key != source_key {
        return None;
    }
    if payload.updated_at_unix <= 0.0 {
        return None;
    }
    if ttl_sec > 0.0 && (now_unix_f64() - payload.updated_at_unix) > ttl_sec {
        return None;
    }
    Some(LinkGraphStatsCachePayload {
        schema: payload.schema,
        source_key: payload.source_key,
        updated_at_unix: payload.updated_at_unix,
        stats: payload.stats.normalize(),
    })
}
