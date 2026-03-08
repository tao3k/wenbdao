use super::super::{LinkGraphPprSubgraphMode, LinkGraphRelatedPprOptions};
use super::constants::{
    RELATED_PPR_DEFAULT_ALPHA, RELATED_PPR_DEFAULT_MAX_ITER, RELATED_PPR_DEFAULT_TOL,
};

pub(super) fn resolve_related_ppr_runtime(
    options: Option<&LinkGraphRelatedPprOptions>,
) -> (f64, usize, f64, LinkGraphPprSubgraphMode) {
    let alpha = options
        .and_then(|row| row.alpha)
        .unwrap_or(RELATED_PPR_DEFAULT_ALPHA);
    let max_iter = options
        .and_then(|row| row.max_iter)
        .unwrap_or(RELATED_PPR_DEFAULT_MAX_ITER)
        .max(1);
    let tol = options
        .and_then(|row| row.tol)
        .unwrap_or(RELATED_PPR_DEFAULT_TOL);
    let subgraph_mode = options
        .and_then(|row| row.subgraph_mode)
        .unwrap_or(LinkGraphPprSubgraphMode::Auto);
    (alpha, max_iter, tol, subgraph_mode)
}
