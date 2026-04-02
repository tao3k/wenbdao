use crate::link_graph::models::LinkGraphRelatedPprDiagnostics;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(in crate::link_graph::index) struct RelatedPprComputation {
    pub(in crate::link_graph::index) ranked_doc_ids: Vec<(String, usize, f64)>,
    pub(in crate::link_graph::index) diagnostics: LinkGraphRelatedPprDiagnostics,
}

#[derive(Debug, Clone)]
pub(in crate::link_graph::index::ppr) struct RelatedPprKernelResult {
    pub(in crate::link_graph::index::ppr) scores_by_doc_id: HashMap<String, f64>,
    pub(in crate::link_graph::index::ppr) iteration_count: usize,
    pub(in crate::link_graph::index::ppr) final_residual: f64,
    pub(in crate::link_graph::index::ppr) timed_out: bool,
}
