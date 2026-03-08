use crate::link_graph::LinkGraphAttachmentKind;
use crate::link_graph::models::LinkGraphAttachment;
use crate::link_graph::parser::ParsedNote;
use std::path::Path;

fn attachment_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map_or_else(|| path.to_string(), ToString::to_string)
}

fn attachment_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().trim_start_matches('.').to_lowercase())
        .unwrap_or_default()
}

pub(super) fn attachments_for_parsed_note(parsed: &ParsedNote) -> Vec<LinkGraphAttachment> {
    let mut rows: Vec<LinkGraphAttachment> = parsed
        .attachment_targets
        .iter()
        .map(|attachment_path| {
            let ext = attachment_ext(attachment_path);
            LinkGraphAttachment {
                source_id: parsed.doc.id.clone(),
                source_stem: parsed.doc.stem.clone(),
                source_path: parsed.doc.path.clone(),
                source_title: parsed.doc.title.clone(),
                attachment_path: attachment_path.clone(),
                attachment_name: attachment_name(attachment_path),
                attachment_ext: ext.clone(),
                kind: LinkGraphAttachmentKind::from_extension(&ext),
            }
        })
        .collect();
    rows.sort_by(|left, right| {
        left.attachment_path
            .cmp(&right.attachment_path)
            .then(left.source_path.cmp(&right.source_path))
    });
    rows.dedup_by(|left, right| {
        left.source_id == right.source_id && left.attachment_path == right.attachment_path
    });
    rows
}
