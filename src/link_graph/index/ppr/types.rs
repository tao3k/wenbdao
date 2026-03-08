use super::super::LinkGraphRelatedPprDiagnostics;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(in super::super) struct RelatedPprComputation {
    pub(in super::super) ranked_doc_ids: Vec<(String, usize, f64)>,
    pub(in super::super) diagnostics: LinkGraphRelatedPprDiagnostics,
}

#[derive(Debug, Clone)]
pub(super) struct RelatedPprKernelResult {
    pub(super) scores_by_doc_id: HashMap<String, f64>,
    pub(super) iteration_count: usize,
    pub(super) final_residual: f64,
    pub(super) timed_out: bool,
}
