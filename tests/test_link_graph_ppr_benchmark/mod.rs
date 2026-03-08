//! Benchmark test for `LinkGraph` related-PPR latency on large fixtures.
//!
//! This benchmark is intentionally `ignored` by default because it materializes
//! a 10k+ markdown fixture to validate long-horizon PPR runtime behavior.

mod link_graph_related_ppr_latency_on_10k_fixture;
mod support;
