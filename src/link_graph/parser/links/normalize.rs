use std::path::{Component, Path};

use crate::link_graph::parser::paths::{normalize_slashes, trim_md_extension};

pub(super) fn has_supported_note_extension(raw: &str) -> bool {
    let ext = Path::new(raw)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_lowercase());
    matches!(ext.as_deref(), None | Some("" | "md" | "markdown" | "mdx"))
}

pub(super) fn strip_target_decorations(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let unwrapped = if trimmed.starts_with('<') {
        let end = trimmed.find('>')?;
        &trimmed[1..end]
    } else {
        trimmed.split_whitespace().next().unwrap_or_default()
    };
    if unwrapped.is_empty() {
        return None;
    }
    Some(normalize_slashes(unwrapped))
}

pub(super) fn strip_fragment_and_query(raw: &str) -> String {
    let mut candidate = raw.to_string();
    if let Some((left, _right)) = candidate.split_once('#') {
        candidate = left.to_string();
    }
    if let Some((left, _right)) = candidate.split_once('?') {
        candidate = left.to_string();
    }
    candidate
}

pub(super) fn has_external_scheme(candidate_lower: &str) -> bool {
    candidate_lower.starts_with("http://")
        || candidate_lower.starts_with("https://")
        || candidate_lower.starts_with("mailto:")
        || candidate_lower.starts_with("tel:")
        || candidate_lower.starts_with("data:")
        || candidate_lower.starts_with("javascript:")
}

fn normalize_link_target(raw: &str) -> String {
    trim_md_extension(&normalize_slashes(raw.trim()))
        .trim_matches('/')
        .to_string()
}

fn is_windows_absolute_path(raw: &str) -> bool {
    let mut chars = raw.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(drive), Some(':'), Some('/')) if drive.is_ascii_alphabetic()
    )
}

fn extract_relative_dir_parts(path: &Path, root: &Path) -> Vec<String> {
    let rel = path.strip_prefix(root).ok();
    let Some(parent) = rel.and_then(Path::parent) else {
        return Vec::new();
    };
    parent
        .components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

pub(super) fn normalize_local_target_path(
    raw: &str,
    source_path: &Path,
    root: &Path,
) -> Option<String> {
    let candidate = normalize_slashes(raw.trim());
    if candidate.is_empty() {
        return None;
    }
    let absolute = candidate.starts_with('/') || is_windows_absolute_path(&candidate);
    let absolute_prefix = if candidate.starts_with('/') { "/" } else { "" };
    let mut parts: Vec<String> = if absolute {
        Vec::new()
    } else {
        extract_relative_dir_parts(source_path, root)
    };

    for segment in candidate.split('/') {
        let cleaned = segment.trim();
        if cleaned.is_empty() || cleaned == "." {
            continue;
        }
        if cleaned == ".." {
            parts.pop();
            continue;
        }
        parts.push(cleaned.to_string());
    }

    if parts.is_empty() {
        return None;
    }
    let joined = parts.join("/");
    if joined.is_empty() {
        return None;
    }
    if absolute_prefix.is_empty() {
        Some(joined)
    } else {
        Some(format!("{absolute_prefix}{joined}"))
    }
}

pub(super) fn normalize_wikilink_note_target(raw: &str) -> Option<String> {
    let mut candidate = raw.trim().to_string();
    if candidate.is_empty() {
        return None;
    }
    candidate = strip_fragment_and_query(&candidate);
    if !has_supported_note_extension(&candidate) {
        return None;
    }
    let normalized = normalize_link_target(&candidate);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub(super) fn normalize_markdown_note_target(
    raw: &str,
    source_path: &Path,
    root: &Path,
) -> Option<String> {
    if !has_supported_note_extension(raw) {
        return None;
    }
    let normalized = normalize_local_target_path(raw, source_path, root)?;
    let normalized = trim_md_extension(&normalized).trim_matches('/').to_string();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized)
}

pub(super) fn normalize_attachment_target(
    raw: &str,
    source_path: &Path,
    root: &Path,
) -> Option<String> {
    let mut candidate = raw.trim().to_string();
    if candidate.is_empty() {
        return None;
    }
    let lower = candidate.to_lowercase();
    if lower.starts_with("file://") {
        candidate = candidate[7..].to_string();
    } else if lower.starts_with("file:") {
        candidate = candidate[5..].to_string();
    }
    candidate = strip_fragment_and_query(&candidate);
    let normalized = normalize_local_target_path(&candidate, source_path, root)?;
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}
