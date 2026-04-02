use comrak::{
    Arena, Options,
    nodes::{AstNode, NodeValue},
    parse_document,
};
use std::path::Path;

use super::parse_target::{parse_markdown_target, parse_wikilink_target};
use super::types::{ExtractedLinkTargets, ParsedTarget};

pub(super) fn extract_markdown_links_with_comrak(
    body: &str,
    source_path: &Path,
    root: &Path,
) -> ExtractedLinkTargets {
    let mut options = Options::default();
    // Support Obsidian-style `[[url|title]]` wikilinks in AST parsing.
    options.extension.wikilinks_title_after_pipe = true;

    let arena = Arena::new();
    let root_node = parse_document(&arena, body, &options);

    let mut notes: Vec<String> = Vec::new();
    let mut attachments: Vec<String> = Vec::new();
    for node in root_node.descendants() {
        let parsed_target = match &node.data().value {
            NodeValue::Link(link) => parse_markdown_target(&link.url, source_path, root),
            NodeValue::Image(image) => parse_markdown_target(&image.url, source_path, root)
                .and_then(|target| match target {
                    ParsedTarget::Attachment(path) => Some(ParsedTarget::Attachment(path)),
                    ParsedTarget::Note(_) => None,
                }),
            NodeValue::WikiLink(link) => {
                parse_wikilink_target(&link.url, source_path, root, is_embedded_wikilink(node))
            }
            _ => None,
        };
        let Some(parsed_target) = parsed_target else {
            continue;
        };
        match parsed_target {
            ParsedTarget::Note(path) => notes.push(path),
            ParsedTarget::Attachment(path) => attachments.push(path),
        }
    }
    notes.sort();
    notes.dedup();
    attachments.sort();
    attachments.dedup();
    ExtractedLinkTargets {
        note_links: notes,
        attachments,
    }
}

fn is_embedded_wikilink(node: &AstNode<'_>) -> bool {
    let Some(previous) = node.previous_sibling() else {
        return false;
    };
    let NodeValue::Text(text) = &previous.data().value else {
        return false;
    };
    // Obsidian embed marker is `![[...]]`. In comrak AST, the `!` is usually
    // retained in the previous text node. Accept inline forms such as
    // `prefix ![[note]]` by checking the immediate trailing character.
    text.as_ref().ends_with('!')
}
