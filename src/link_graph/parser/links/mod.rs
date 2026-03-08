use std::path::Path;

mod normalize;
mod parse_target;
mod scan;

#[derive(Debug, Default)]
pub(super) struct ExtractedLinkTargets {
    pub note_links: Vec<String>,
    pub attachments: Vec<String>,
}

#[derive(Debug)]
enum ParsedTarget {
    Note(String),
    Attachment(String),
}

pub(super) fn extract_link_targets(
    body: &str,
    source_path: &Path,
    root: &Path,
) -> ExtractedLinkTargets {
    scan::extract_markdown_links_with_comrak(body, source_path, root)
}
