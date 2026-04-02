use super::content::{
    count_words, extract_doc_type, extract_lead, extract_saliency_params, extract_tags,
    extract_title, parse_frontmatter,
};
use super::links::extract_link_targets;
use super::paths::{normalize_slashes, relative_doc_id};
use super::sections::extract_sections;
use super::time::resolve_note_timestamps;
use super::types::ParsedNote;
use crate::link_graph::models::LinkGraphDocument;
use std::path::Path;

/// Parse one note file into structured document row plus outgoing link targets.
#[must_use]
pub fn parse_note(path: &Path, root: &Path, content: &str) -> Option<ParsedNote> {
    let doc_id = relative_doc_id(path, root)?;
    let stem = path.file_stem()?.to_string_lossy().to_string();
    if stem.is_empty() {
        return None;
    }
    let rel_path = normalize_slashes(
        path.strip_prefix(root)
            .ok()
            .map_or_else(
                || path.to_string_lossy().to_string(),
                |p| p.to_string_lossy().to_string(),
            )
            .as_str(),
    );
    let (frontmatter, body) = parse_frontmatter(content);
    let title = extract_title(frontmatter.as_ref(), body, &stem);
    let tags = extract_tags(frontmatter.as_ref());
    let doc_type = extract_doc_type(frontmatter.as_ref());
    let lead = extract_lead(body);
    let word_count = count_words(body);
    let (saliency_base, decay_rate) = extract_saliency_params(frontmatter.as_ref());
    let search_text = body.to_string();
    let search_text_lower = search_text.to_lowercase();
    let id_lower = doc_id.to_lowercase();
    let stem_lower = stem.to_lowercase();
    let path_lower = rel_path.to_lowercase();
    let title_lower = title.to_lowercase();
    let tags_lower: Vec<String> = tags.iter().map(|tag| tag.to_lowercase()).collect();
    let (created_ts, modified_ts) = resolve_note_timestamps(frontmatter.as_ref(), path);
    let extracted = extract_link_targets(body, path, root);
    let sections = extract_sections(body, path, root);
    Some(ParsedNote {
        doc: LinkGraphDocument {
            id: doc_id,
            id_lower,
            stem,
            stem_lower,
            path: rel_path,
            path_lower,
            title,
            title_lower,
            tags,
            tags_lower,
            lead,
            doc_type,
            word_count,
            search_text,
            search_text_lower,
            saliency_base,
            decay_rate,
            created_ts,
            modified_ts,
        },
        link_targets: extracted.note_links,
        attachment_targets: extracted.attachments,
        sections,
    })
}
