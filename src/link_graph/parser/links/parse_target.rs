use std::path::Path;

use super::normalize::{
    has_external_scheme, has_supported_note_extension, normalize_attachment_target,
    normalize_markdown_note_target, normalize_wikilink_note_target, strip_fragment_and_query,
    strip_target_decorations,
};
use super::types::ParsedTarget;

pub(super) fn parse_markdown_target(
    raw: &str,
    source_path: &Path,
    root: &Path,
) -> Option<ParsedTarget> {
    let decorated = strip_target_decorations(raw)?;
    if decorated.is_empty() {
        return None;
    }
    let lower = decorated.to_lowercase();
    if lower.starts_with('#') {
        return None;
    }
    if has_external_scheme(&lower) {
        return None;
    }
    if lower.starts_with("file:") {
        return normalize_attachment_target(&decorated, source_path, root)
            .map(ParsedTarget::Attachment);
    }
    let stripped = strip_fragment_and_query(&decorated);
    if stripped.is_empty() {
        return None;
    }
    if has_supported_note_extension(stripped) {
        return normalize_markdown_note_target(stripped, source_path, root).map(ParsedTarget::Note);
    }
    normalize_attachment_target(stripped, source_path, root).map(ParsedTarget::Attachment)
}

pub(super) fn parse_wikilink_target(
    raw: &str,
    source_path: &Path,
    root: &Path,
    embedded: bool,
) -> Option<ParsedTarget> {
    if let Some(note) = normalize_wikilink_note_target(raw) {
        if embedded {
            return None;
        }
        return Some(ParsedTarget::Note(note));
    }
    normalize_attachment_target(raw, source_path, root).map(ParsedTarget::Attachment)
}
