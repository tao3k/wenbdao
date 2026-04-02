mod scan;
mod scoring;
mod search;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use search::search_attachment_hits;
pub(crate) use types::AttachmentSearchError;
