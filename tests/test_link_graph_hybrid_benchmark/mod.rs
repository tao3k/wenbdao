//! Benchmark test for `LinkGraph` hybrid batch latency on large fixtures.
//!
//! This benchmark is intentionally `ignored` by default because it materializes
//! a 2k+ markdown fixture to validate batch-native quantum retrieval runtime behavior.

mod link_graph_hybrid_batch_latency_on_2k_fixture;
mod support;
