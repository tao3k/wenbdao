use crate::link_graph::context_snapshot::runtime::{
    normalize_snapshot_key_prefix, snapshot_redis_client,
};
use crate::link_graph::runtime_config::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;

#[test]
fn normalize_snapshot_key_prefix_falls_back_for_blank_input() {
    assert_eq!(
        normalize_snapshot_key_prefix("   "),
        DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string()
    );
}

#[test]
fn normalize_snapshot_key_prefix_trims_non_blank_input() {
    assert_eq!(
        normalize_snapshot_key_prefix("  xiuxian:snapshot  "),
        "xiuxian:snapshot".to_string()
    );
}

#[test]
fn snapshot_redis_client_opens_trimmed_valid_url() {
    let client = snapshot_redis_client(" redis://127.0.0.1/ ");
    assert!(client.is_ok());
}

#[test]
fn snapshot_redis_client_preserves_snapshot_error_context() {
    let Err(error) = snapshot_redis_client("  ") else {
        panic!("blank URL should fail");
    };
    assert!(error.contains("link_graph snapshot store"));
}
