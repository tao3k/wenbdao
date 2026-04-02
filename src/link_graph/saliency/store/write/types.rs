use crate::link_graph::saliency::LinkGraphSaliencyPolicy;

#[derive(Debug, Clone, Copy)]
pub(super) struct TouchUpdateSpec {
    pub(super) activation_delta: u64,
    pub(super) saliency_base: Option<f64>,
    pub(super) decay_rate_override: Option<f64>,
    pub(super) policy: LinkGraphSaliencyPolicy,
    pub(super) now_unix: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CoactivationNeighborDirection {
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CoactivationNeighbor {
    pub(super) node_id: String,
    pub(super) direction: CoactivationNeighborDirection,
    pub(super) rank: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CoactivationPropagationTarget {
    pub(super) node_id: String,
    pub(super) hop: usize,
    pub(super) weight: f64,
}
