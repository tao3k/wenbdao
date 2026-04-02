mod candidates;
mod decode;
mod search;
#[cfg(test)]
mod tests;

pub(crate) use search::{ReferenceOccurrenceSearchError, search_reference_occurrences};
