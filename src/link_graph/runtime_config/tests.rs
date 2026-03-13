use super::resolve_link_graph_coactivation_runtime;
use crate::link_graph::runtime_config::constants::DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH;

#[test]
fn test_coactivation_touch_queue_depth_default() {
    let runtime = resolve_link_graph_coactivation_runtime();
    assert_eq!(
        runtime.touch_queue_depth,
        DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH
    );
}
