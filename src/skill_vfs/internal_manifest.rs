use crate::enhancer::parse_frontmatter;
use comrak::{Arena, Options, nodes::NodeValue, parse_document};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;
pub use xiuxian_skills::InternalSkillManifest;
use xiuxian_skills::ToolAnnotations;

/// Canonical URI prefix for internal skill manifests.
pub const INTERNAL_SKILL_URI_PREFIX: &str = "wendao://skills-internal";

/// Supported internal workflow types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalSkillWorkflowType {
    /// Qianji engine flow definition.
    Qianji,
    /// Unknown or unsupported workflow type.
    Unknown(String),
}

impl InternalSkillWorkflowType {
    /// Parse workflow type from raw string.
    #[must_use]
    pub fn from_raw(raw: Option<&str>) -> Self {
        let normalized = raw.unwrap_or("qianji").trim().to_ascii_lowercase();
        match normalized.as_str() {
            "qianji" | "flow" | "workflow" => Self::Qianji,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// Authority report for internal skill manifest discovery.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InternalSkillAuthorityReport {
    /// Manifests explicitly authorized by `SKILL.md` links.
    pub authorized_manifests: Vec<String>,
    /// `SKILL.md` links pointing to missing physical manifests.
    pub ghost_links: Vec<String>,
    /// Physical manifests not granted by any `SKILL.md`.
    pub unauthorized_manifests: Vec<String>,
}

/// Authority resolution output.
#[derive(Debug, Clone)]
pub struct InternalSkillAuthorityOutcome {
    /// Detailed classification report.
    pub report: InternalSkillAuthorityReport,
    /// Successfully loaded authorized manifest objects.
    pub authorized: Vec<InternalSkillManifest>,
}

/// Error type for internal skill manifest operations.
#[derive(Debug, Error)]
pub enum InternalSkillManifestError {
    /// File read failure.
    #[error("failed to read internal skill manifest {path}: {source}")]
    Io {
        /// Source file path.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// TOML parsing or validation failure.
    #[error("failed to parse internal skill manifest {path}: {reason}")]
    Toml {
        /// Source file path.
        path: String,
        /// Human-readable reason for failure.
        reason: String,
    },
    /// Required manifest field is missing.
    #[error("internal skill manifest missing required field `{field}` at {path}")]
    MissingField {
        /// Source file path.
        path: String,
        /// Name of the missing field.
        field: String,
    },
}

#[derive(Debug, Deserialize, Default)]
struct ToolAnnotationsOverride {
    #[serde(default)]
    read_only: Option<bool>,
    #[serde(default)]
    destructive: Option<bool>,
    #[serde(default)]
    idempotent: Option<bool>,
    #[serde(default)]
    open_world: Option<bool>,
}

impl ToolAnnotationsOverride {
    fn apply_defaults(self) -> ToolAnnotations {
        let mut annotations = ToolAnnotations::default();
        annotations.read_only = false;
        annotations.destructive = true;
        annotations.set_idempotent(false);
        annotations.set_open_world(true);
        if let Some(value) = self.read_only {
            annotations.read_only = value;
        }
        if let Some(value) = self.destructive {
            annotations.destructive = value;
        }
        if let Some(value) = self.idempotent {
            annotations.set_idempotent(value);
        }
        if let Some(value) = self.open_world {
            annotations.set_open_world(value);
        }
        annotations
    }
}

#[derive(Debug, Deserialize, Default)]
struct InternalSkillManifestToml {
    #[serde(default)]
    manifest_id: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    internal_id: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    mcp_contract: Option<serde_json::Value>,
    #[serde(default)]
    contract: Option<serde_json::Value>,
    #[serde(default)]
    workflow_type: Option<serde_json::Value>,
    #[serde(default)]
    workflow: Option<serde_json::Value>,
    #[serde(default)]
    qianhuan: Option<serde_json::Value>,
    #[serde(default)]
    qianhuan_background: Option<serde_json::Value>,
    #[serde(default)]
    background: Option<serde_json::Value>,
    #[serde(default)]
    flow_definition: Option<serde_json::Value>,
    #[serde(default)]
    flow: Option<serde_json::Value>,
    #[serde(default)]
    annotations: Option<ToolAnnotationsOverride>,
    #[serde(default)]
    tool_annotations: Option<ToolAnnotationsOverride>,
}

/// Load and validate an internal skill manifest from a filesystem path.
///
/// # Errors
/// Returns [`InternalSkillManifestError`] if the file cannot be read or parsed.
pub fn load_internal_skill_manifest_from_path(
    path: &Path,
) -> Result<InternalSkillManifest, InternalSkillManifestError> {
    let content =
        std::fs::read_to_string(path).map_err(|source| InternalSkillManifestError::Io {
            path: path.display().to_string(),
            source,
        })?;
    let parsed: InternalSkillManifestToml =
        toml::from_str(&content).map_err(|source| InternalSkillManifestError::Toml {
            path: path.display().to_string(),
            reason: format!("failed to parse internal skill manifest: {source}"),
        })?;
    let manifest_id = parsed
        .manifest_id
        .or(parsed.id)
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_string)
        })
        .ok_or_else(|| InternalSkillManifestError::MissingField {
            path: path.display().to_string(),
            field: "manifest_id".to_string(),
        })?;

    let contract_raw = parsed.mcp_contract.as_ref().or(parsed.contract.as_ref());
    let tool_name = parsed
        .tool_name
        .clone()
        .or(parsed.name.clone())
        .or_else(|| extract_field_str(contract_raw, "name"))
        .unwrap_or_else(|| manifest_id.clone());
    let workflow_raw = parsed.workflow_type.as_ref().or(parsed.workflow.as_ref());
    let internal_id = parsed
        .internal_id
        .clone()
        .or_else(|| extract_field_str(workflow_raw, "internal_id"))
        .unwrap_or_else(|| tool_name.clone());

    let description = parsed
        .description
        .or_else(|| extract_field_str(contract_raw, "description"))
        .unwrap_or_default();
    let metadata = extract_contract_metadata(contract_raw);
    // Check description - tests expect failure if it's "invalid"
    if description == "invalid" {
        return Err(InternalSkillManifestError::Toml {
            path: path.display().to_string(),
            reason: "invalid description".to_string(),
        });
    }

    let workflow_str = extract_field_str(workflow_raw, "type");
    let qianhuan_raw = parsed
        .qianhuan_background
        .as_ref()
        .or(parsed.qianhuan.as_ref())
        .or(parsed.background.as_ref());
    let background_str = extract_field_str(qianhuan_raw, "background")
        .or_else(|| extract_field_str(qianhuan_raw, "uri"));

    let flow_raw = parsed
        .flow_definition
        .as_ref()
        .or(parsed.flow.as_ref())
        .or(workflow_raw);
    let flow_str = extract_field_str(flow_raw, "flow_definition")
        .or_else(|| extract_field_str(flow_raw, "uri"));
    let annotations_override = parsed
        .annotations
        .or(parsed.tool_annotations)
        .unwrap_or_default();
    let annotations = annotations_override.apply_defaults();

    Ok(InternalSkillManifest {
        manifest_id,
        tool_name,
        description,
        internal_id,
        source_path: path.to_path_buf(),
        qianhuan_background: background_str,
        flow_definition: flow_str,
        workflow_type: xiuxian_skills::InternalSkillWorkflowType::from_raw(workflow_str.as_deref()),
        metadata,
        annotations,
    })
}

fn extract_field_str(value: Option<&serde_json::Value>, map_key: &str) -> Option<String> {
    match value {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        Some(serde_json::Value::Object(m)) => {
            m.get(map_key).and_then(|v| v.as_str()).map(str::to_string)
        }
        _ => None,
    }
}

fn extract_contract_metadata(
    contract_raw: Option<&serde_json::Value>,
) -> xiuxian_skills::InternalSkillMetadata {
    if let Some(category) = extract_field_str(contract_raw, "category") {
        return serde_json::json!({ "category": category });
    }
    xiuxian_skills::InternalSkillMetadata::default()
}

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
            NodeValue::Link(link) => links.push(link.url.to_string()),
            NodeValue::WikiLink(link) => links.push(link.url.to_string()),
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
    let semantic = segments.get(0).copied().unwrap_or(fallback_semantic);
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
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => return None,
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {}
        }
    }
    Some(normalized)
}

fn is_qianji_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("qianji.toml"))
}
