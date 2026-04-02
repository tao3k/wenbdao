use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use super::types::MarkdownConfigLinkTarget;

/// Extracts normalized local/semantic link targets plus optional type-hints.
///
/// Type hints are parsed from wikilink shape `[[target#type]]`.
#[must_use]
pub fn extract_markdown_config_link_targets_by_id(
    markdown: &str,
    source_path: &str,
) -> HashMap<String, Vec<MarkdownConfigLinkTarget>> {
    let mut options = comrak::Options::default();
    options.extension.wikilinks_title_before_pipe = true;

    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, markdown, &options);

    let mut links_by_id: HashMap<String, Vec<MarkdownConfigLinkTarget>> = HashMap::new();
    let mut active_cursor: Option<MarkdownPropertyCursor> = None;

    for node in root.descendants() {
        match &node.data.borrow().value {
            comrak::nodes::NodeValue::Heading(heading) => {
                let heading_level = heading.level;
                if let Some(cursor) = &active_cursor
                    && heading_level <= cursor.heading_level
                {
                    active_cursor = None;
                }
                if let Some(next_cursor) = parse_cursor_from_heading(node, heading_level) {
                    active_cursor = Some(next_cursor);
                }
            }
            comrak::nodes::NodeValue::Link(link) => {
                let Some(cursor) = &active_cursor else {
                    continue;
                };
                insert_link_target(
                    &mut links_by_id,
                    &cursor.id,
                    link.url.as_str(),
                    source_path,
                    None,
                );
            }
            comrak::nodes::NodeValue::Image(image) => {
                let Some(cursor) = &active_cursor else {
                    continue;
                };
                insert_link_target(
                    &mut links_by_id,
                    &cursor.id,
                    image.url.as_str(),
                    source_path,
                    Some("attachment".to_string()),
                );
            }
            comrak::nodes::NodeValue::WikiLink(link) => {
                let Some(cursor) = &active_cursor else {
                    continue;
                };
                let (target, reference_type) = split_wikilink_type_hint(link.url.as_str());
                insert_link_target(
                    &mut links_by_id,
                    &cursor.id,
                    target,
                    source_path,
                    reference_type,
                );
            }
            _ => {}
        }
    }

    links_by_id
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownPropertyCursor {
    id: String,
    heading_level: u8,
}

fn parse_cursor_from_heading<'a>(
    heading_node: &'a comrak::nodes::AstNode<'a>,
    heading_level: u8,
) -> Option<MarkdownPropertyCursor> {
    let sibling = heading_node.next_sibling()?;
    let comrak::nodes::NodeValue::HtmlBlock(html) = &sibling.data.borrow().value else {
        return None;
    };
    let tag = parse_property_tag(&html.literal)?;
    Some(MarkdownPropertyCursor {
        id: tag.id,
        heading_level,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownPropertyTag {
    id: String,
}

fn parse_property_tag(html_block: &str) -> Option<MarkdownPropertyTag> {
    let body = html_block
        .trim()
        .strip_prefix("<!--")?
        .strip_suffix("-->")?
        .trim();

    let mut id: Option<String> = None;
    for pair in body.split(',') {
        let Some((raw_key, raw_value)) = pair.split_once(':') else {
            continue;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let value = trim_quotes(raw_value.trim());
        if value.is_empty() {
            continue;
        }
        if key == "id" {
            id = Some(value.to_string());
        }
    }

    Some(MarkdownPropertyTag { id: id? })
}

fn trim_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn insert_link_target(
    links_by_id: &mut HashMap<String, Vec<MarkdownConfigLinkTarget>>,
    id: &str,
    raw_target: &str,
    source_path: &str,
    reference_type: Option<String>,
) {
    let Some(target) = normalize_local_link_target(raw_target, source_path) else {
        return;
    };
    let normalized_type = normalize_reference_type(reference_type, target.as_str());
    let links = links_by_id.entry(id.to_string()).or_default();
    if !links
        .iter()
        .any(|existing| existing.target == target && existing.reference_type == normalized_type)
    {
        links.push(MarkdownConfigLinkTarget {
            target,
            reference_type: normalized_type,
        });
    }
}

fn normalize_reference_type(reference_type: Option<String>, target: &str) -> Option<String> {
    let explicit = reference_type
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    explicit.or_else(|| infer_reference_type_from_target(target))
}

fn infer_reference_type_from_target(target: &str) -> Option<String> {
    let ext = extract_extension(target)?;
    if is_attachment_extension(ext) {
        return Some("attachment".to_string());
    }
    None
}

fn extract_extension(target: &str) -> Option<&str> {
    let without_fragment = strip_fragment_and_query(target);
    let leaf = without_fragment.rsplit('/').next()?;
    let (_, extension) = leaf.rsplit_once('.')?;
    let trimmed = extension.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn is_attachment_extension(extension: &str) -> bool {
    matches!(
        extension.trim().to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "pdf"
    )
}

fn split_wikilink_type_hint(raw_target: &str) -> (&str, Option<String>) {
    let trimmed = raw_target.trim();
    if trimmed.is_empty() {
        return (trimmed, None);
    }
    let before_alias = trimmed.split('|').next().unwrap_or(trimmed).trim();
    let Some((target, hint)) = before_alias.rsplit_once('#') else {
        return (before_alias, None);
    };
    let hint = hint.trim();
    if target.trim().is_empty() || hint.is_empty() {
        return (before_alias, None);
    }
    (target.trim(), Some(hint.to_string()))
}

fn normalize_local_link_target(raw_target: &str, source_path: &str) -> Option<String> {
    let target = strip_fragment_and_query(raw_target);
    if target.is_empty() || target.starts_with('#') {
        return None;
    }
    if is_wendao_resource_uri(target) {
        return Some(target.to_string());
    }
    if is_external_target(target) {
        return None;
    }

    let source_parent = Path::new(source_path).parent().unwrap_or(Path::new(""));
    let target_path = if target.starts_with('/') {
        PathBuf::from(target.trim_start_matches('/'))
    } else {
        source_parent.join(target)
    };
    normalize_relative_path(&target_path)
}

fn normalize_relative_path(path: &Path) -> Option<String> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(value) => normalized.push(value),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    let candidate = normalized.to_string_lossy().replace('\\', "/");
    if candidate.is_empty() {
        None
    } else {
        Some(candidate)
    }
}

fn strip_fragment_and_query(raw: &str) -> &str {
    let mut end = raw.len();
    if let Some(index) = raw.find('#') {
        end = end.min(index);
    }
    if let Some(index) = raw.find('?') {
        end = end.min(index);
    }
    raw[..end].trim()
}

fn is_external_target(target: &str) -> bool {
    target.contains("://") || target.starts_with("mailto:") || target.starts_with("tel:")
}

fn is_wendao_resource_uri(target: &str) -> bool {
    target.trim().to_ascii_lowercase().starts_with("wendao://")
}
