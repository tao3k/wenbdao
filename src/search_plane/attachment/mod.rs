mod build;
mod query;
mod schema;

pub(crate) use build::ensure_attachment_index_started;
#[cfg(test)]
pub(crate) use build::{AttachmentBuildError, publish_attachments_from_projects};
pub(crate) use query::{AttachmentSearchError, search_attachment_hits};
