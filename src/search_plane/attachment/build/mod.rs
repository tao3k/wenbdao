mod extract;
mod orchestration;
mod plan;
mod types;
mod write;

#[cfg(test)]
mod tests;

pub(crate) use extract::attachment_kind_label;
pub(crate) use orchestration::ensure_attachment_index_started;
#[cfg(test)]
pub(crate) use orchestration::publish_attachments_from_projects;
pub(crate) use plan::{fingerprint_projects, plan_attachment_build};
#[cfg(test)]
pub(crate) use types::AttachmentBuildError;
pub(crate) use types::{AttachmentBuildPlan, AttachmentWriteResult};
#[cfg(test)]
pub(crate) use write::export_attachment_epoch_parquet;
pub(crate) use write::write_attachment_epoch;
