mod extract;
mod orchestration;
mod plan;
mod types;
mod write;

#[cfg(test)]
mod tests;

pub(crate) use orchestration::ensure_reference_occurrence_index_started;
#[cfg(test)]
pub(crate) use orchestration::publish_reference_occurrences_from_projects;
pub(crate) use plan::{fingerprint_projects, plan_reference_occurrence_build};
#[cfg(test)]
pub(crate) use types::ReferenceOccurrenceBuildError;
pub(crate) use types::{ReferenceOccurrenceBuildPlan, ReferenceOccurrenceWriteResult};
pub(crate) use write::write_reference_occurrence_epoch;
