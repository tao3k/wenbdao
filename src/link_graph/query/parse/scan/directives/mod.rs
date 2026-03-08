use super::super::state::ParsedDirectiveState;

mod filters;
mod links;
mod search;
mod semantic;
mod structure;

pub(super) fn apply_directive(
    key: &str,
    value: &str,
    negated_key: bool,
    state: &mut ParsedDirectiveState,
    residual_terms: &mut Vec<String>,
) -> bool {
    search::apply(key, value, state, residual_terms)
        || semantic::apply(key, value, state)
        || links::apply(key, value, negated_key, state)
        || filters::apply(key, value, negated_key, state)
        || structure::apply(key, value, state)
}
