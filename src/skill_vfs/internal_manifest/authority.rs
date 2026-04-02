use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use comrak::{Arena, Options, nodes::NodeValue, parse_document};
use walkdir::WalkDir;

use super::load::load_internal_skill_manifest_from_path;
use super::types::{
    INTERNAL_SKILL_URI_PREFIX, InternalSkillAuthorityOutcome, InternalSkillAuthorityReport,
    InternalSkillManifestError,
};
use crate::enhancer::parse_frontmatter;

/// Resolve internal skill authority by intersecting intent links and physical manifests.
///
/// # Errors
/// Returns [`InternalSkillManifestError`] when manifest parsing fails.
pub fn resolve_internal_skill_authority(
    internal_root: &Path,
) -> Result<InternalSkillAuthorityOutcome, InternalSkillManifestError> {
    let mut intent_uris = HashSet::new();
    let mut ghost_links = HashSet::new();
    let mut manifest_paths = HashMap::new();
    let skills = discover_internal_skill_roots(internal_root);
    for skill_root in &skills {
        let Some(skill_doc) = locate_skill_doc(skill_root) else {
            continue;
        };
        let semantic_name = resolve_skill_semantic_name(skill_root, &skill_doc);
        let (intent, ghosts) = collect_intent_manifest_uris(skill_root, &skill_doc, &semantic_name);
        intent_uris.extend(intent);
        ghost_links.extend(ghosts);
        let physical = collect_physical_manifest_uris(skill_root, &semantic_name);
        for (uri, path) in physical {
            manifest_paths.insert(uri, path);
        }
    }

    let physical_uris: HashSet<String> = manifest_paths.keys().cloned().collect();
    let authorized_uris: HashSet<String> =
        intent_uris.intersection(&physical_uris).cloned().collect();
    let unauthorized_uris: HashSet<String> =
        physical_uris.difference(&intent_uris).cloned().collect();
    let mut authorized_manifests = Vec::new();
    for uri in &authorized_uris {
        if let Some(path) = manifest_paths.get(uri) {
            authorized_manifests.push(load_internal_skill_manifest_from_path(path)?);
        }
    }
    authorized_manifests.sort_by(|left, right| left.tool_name.cmp(&right.tool_name));
    let mut report = InternalSkillAuthorityReport {
        authorized_manifests: authorized_uris.into_iter().collect(),
        ghost_links: ghost_links.into_iter().collect(),
        unauthorized_manifests: unauthorized_uris.into_iter().collect(),
    };
    report.authorized_manifests.sort();
    report.ghost_links.sort();
    report.unauthorized_manifests.sort();
    Ok(InternalSkillAuthorityOutcome {
        report,
        authorized: authorized_manifests,
    })
}

fn discover_internal_skill_roots(root: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return roots;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            roots.push(path);
        }
    }
    roots.sort();
    roots
}

fn locate_skill_doc(skill_root: &Path) -> Option<PathBuf> {
    let candidates = ["SKILL.md", "skill.md"];
    for name in candidates {
        let path = skill_root.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn resolve_skill_semantic_name(skill_root: &Path, skill_doc: &Path) -> String {
    let content = std::fs::read_to_string(skill_doc).unwrap_or_default();
    let frontmatter = parse_frontmatter(&content);
    frontmatter
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            skill_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("internal")
                .to_string()
        })
        .trim()
        .to_ascii_lowercase()
}

fn collect_physical_manifest_uris(
    skill_root: &Path,
    semantic_name: &str,
) -> Vec<(String, PathBuf)> {
    let mut manifests = Vec::new();
    let references_root = skill_root.join("references");
    if !references_root.is_dir() {
        return manifests;
    }
    for entry in WalkDir::new(&references_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_qianji_manifest(path) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(&references_root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let uri = canonical_internal_manifest_uri(semantic_name, &relative);
        manifests.push((uri, path.to_path_buf()));
    }
    manifests
}

fn collect_intent_manifest_uris(
    skill_root: &Path,
    skill_doc: &Path,
    semantic_name: &str,
) -> (HashSet<String>, HashSet<String>) {
    let markdown = std::fs::read_to_string(skill_doc).unwrap_or_default();
    let targets = extract_markdown_links(&markdown);
    let mut intents = HashSet::new();
    let mut ghosts = HashSet::new();
    let references_root = skill_root.join("references");
    for target in targets {
        if let Some(uri) = normalize_internal_manifest_uri(&target, semantic_name) {
            intents.insert(uri);
            continue;
        }

        let Some(normalized) = normalize_local_target(&target, skill_doc, skill_root) else {
            continue;
        };
        if !normalized.starts_with(&references_root) {
            continue;
        }
        if !is_qianji_manifest(&normalized) {
            continue;
        }
        let Ok(relative) = normalized.strip_prefix(&references_root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let uri = canonical_internal_manifest_uri(semantic_name, &relative);
        if normalized.exists() {
            intents.insert(uri);
        } else {
            ghosts.insert(uri);
        }
    }
    (intents, ghosts)
}

fn extract_markdown_links(markdown: &str) -> Vec<String> {
    let mut options = Options::default();
    options.extension.wikilinks_title_before_pipe = true;
    options.extension.wikilinks_title_after_pipe = true;
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);
    let mut links = Vec::new();
    for node in root.descendants() {
        match &node.data.borrow().value {
            NodeValue::Link(link) => links.push(link.url.clone()),
            NodeValue::WikiLink(link) => links.push(link.url.clone()),
            _ => {}
        }
    }
    links
}

fn normalize_internal_manifest_uri(raw: &str, fallback_semantic: &str) -> Option<String> {
    let trimmed = raw.trim();
    if !trimmed.starts_with(INTERNAL_SKILL_URI_PREFIX) {
        return None;
    }
    let mut payload = trimmed.trim_start_matches(INTERNAL_SKILL_URI_PREFIX);
    payload = payload.trim_start_matches('/');
    let mut segments = payload.split('/').collect::<Vec<_>>();
    if segments.len() < 3 {
        return None;
    }
    let semantic = segments.first().copied().unwrap_or(fallback_semantic);
    if segments.get(1).copied()? != "references" {
        return None;
    }
    let entity = segments.split_off(2).join("/");
    if entity.is_empty() {
        return None;
    }
    Some(canonical_internal_manifest_uri(
        semantic,
        entity.trim_matches('/'),
    ))
}

fn canonical_internal_manifest_uri(semantic_name: &str, relative: &str) -> String {
    format!(
        "{}/{}/references/{}",
        INTERNAL_SKILL_URI_PREFIX,
        semantic_name.trim().to_ascii_lowercase(),
        relative.trim_matches('/')
    )
}

fn normalize_local_target(raw: &str, skill_doc: &Path, skill_root: &Path) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if is_external_link(trimmed) || trimmed.starts_with('#') {
        return None;
    }
    let trimmed = strip_fragment_and_query(trimmed);
    if trimmed.is_empty() {
        return None;
    }
    let base_dir = skill_doc.parent().unwrap_or(skill_root);
    let candidate = Path::new(trimmed);
    let joined = if candidate.is_absolute() {
        skill_root.join(candidate.strip_prefix("/").ok()?)
    } else {
        base_dir.join(candidate)
    };
    let normalized = normalize_path_no_parent(&joined)?;
    if normalized.starts_with(skill_root) {
        Some(normalized)
    } else {
        None
    }
}

fn strip_fragment_and_query(raw: &str) -> &str {
    let mut end = raw.len();
    if let Some(idx) = raw.find('#') {
        end = end.min(idx);
    }
    if let Some(idx) = raw.find('?') {
        end = end.min(idx);
    }
    raw[..end].trim_matches('/')
}

fn is_external_link(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("data:")
        || lower.starts_with("javascript:")
}

fn normalize_path_no_parent(path: &Path) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::CurDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {}
            std::path::Component::ParentDir => return None,
        }
    }
    Some(normalized)
}

fn is_qianji_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("qianji.toml"))
}
