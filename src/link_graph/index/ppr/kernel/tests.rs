use crate::link_graph::index::ppr::kernel::adjacency::build_node_index;
use crate::link_graph::index::ppr::kernel::iteration::{
    build_restart_state, run_kernel_iterations,
};
use crate::link_graph::index::ppr::kernel::types::RestartState;
use std::collections::HashMap;

#[test]
fn build_restart_state_normalizes_positive_weights() {
    let node_to_idx = build_node_index(&["alpha".to_string(), "beta".to_string()]);
    let mut seeds = HashMap::new();
    seeds.insert("alpha".to_string(), 2.0);
    seeds.insert("beta".to_string(), 1.0);

    let Some(state) = build_restart_state(2, &seeds, &node_to_idx) else {
        panic!("restart state should build");
    };

    assert_eq!(state.teleport.len(), 2);
    assert!((state.teleport[0] - (2.0 / 3.0)).abs() < 1e-12);
    assert!((state.teleport[1] - (1.0 / 3.0)).abs() < 1e-12);
    assert_eq!(state.restart_nodes.len(), 2);
}

#[test]
fn build_restart_state_rejects_non_positive_seeds() {
    let node_to_idx = build_node_index(&["alpha".to_string()]);
    let mut seeds = HashMap::new();
    seeds.insert("alpha".to_string(), 0.0);

    assert!(build_restart_state(1, &seeds, &node_to_idx).is_none());
}

#[test]
fn run_kernel_iterations_conserves_probability_mass() {
    let adjacency = vec![vec![1], vec![0]];
    let restart_state = RestartState {
        teleport: vec![1.0, 0.0],
        restart_nodes: vec![(0, 1.0)],
    };

    let outcome = run_kernel_iterations(&adjacency, &restart_state, 0.85, 40, 1e-12, None);
    let sum: f64 = outcome.scores.iter().sum();
    assert!((sum - 1.0).abs() < 1e-9);
    assert!(!outcome.timed_out);
    assert!(outcome.iteration_count > 0);
}
