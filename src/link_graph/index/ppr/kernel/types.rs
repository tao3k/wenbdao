use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(crate) struct PassageEntityAdjacency {
    pub(crate) passage_entities_by_idx: HashMap<usize, Vec<usize>>,
    pub(crate) entity_passages_by_idx: HashMap<usize, Vec<usize>>,
}

#[derive(Debug, Clone)]
pub(crate) struct RestartState {
    pub(crate) teleport: Vec<f64>,
    pub(crate) restart_nodes: Vec<(usize, f64)>,
}

#[derive(Debug, Clone)]
pub(crate) struct KernelIterationOutcome {
    pub(crate) scores: Vec<f64>,
    pub(crate) iteration_count: usize,
    pub(crate) final_residual: f64,
    pub(crate) timed_out: bool,
}
