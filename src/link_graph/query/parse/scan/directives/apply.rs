use super::{filters, links, search, structure};
use crate::link_graph::query::parse::state::ParsedDirectiveState;

pub(in crate::link_graph::query::parse::scan) fn apply_directive(
    key: &str,
    value: &str,
    negated_key: bool,
    state: &mut ParsedDirectiveState,
    residual_terms: &mut Vec<String>,
) -> bool {
    search::apply(key, value, state, residual_terms)
        || links::apply(key, value, negated_key, state)
        || filters::apply(key, value, negated_key, state)
        || structure::apply(key, value, state)
}
