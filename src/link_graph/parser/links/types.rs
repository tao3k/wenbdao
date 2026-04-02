#[derive(Debug, Default)]
pub(in crate::link_graph::parser) struct ExtractedLinkTargets {
    pub note_links: Vec<String>,
    pub attachments: Vec<String>,
}

#[derive(Debug)]
pub(in crate::link_graph::parser) enum ParsedTarget {
    Note(String),
    Attachment(String),
}
