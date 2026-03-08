use std::path::{Component, Path, PathBuf};

use super::SkillVfsError;

/// Canonical URI scheme for Wendao skill resource addressing.
pub const WENDAO_URI_SCHEME: &str = "wendao";
const SKILLS_SEGMENT: &str = "skills";
const SKILLS_INTERNAL_SEGMENT: &str = "skills-internal";
const REFERENCES_SEGMENT: &str = "references";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WendaoUriNamespace {
    Semantic,
    Internal,
}

impl WendaoUriNamespace {
    fn segment(self) -> &'static str {
        match self {
            Self::Semantic => SKILLS_SEGMENT,
            Self::Internal => SKILLS_INTERNAL_SEGMENT,
        }
    }
}

/// Parsed Wendao skill resource URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WendaoResourceUri {
    namespace: WendaoUriNamespace,
    skill_name: String,
    entity_name: String,
}

impl WendaoResourceUri {
    /// Parse one URI string in one of these shapes:
    ///
    /// - `wendao://skills/<semantic_name>/references/<entity_name>.<ext>`
    /// - `wendao://skills-internal/<skill_name>/<relative_path>.<ext>`
    ///
    /// A leading `$` is accepted for placeholder-style inputs.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when URI syntax or path safety checks fail.
    pub fn parse(uri: &str) -> Result<Self, SkillVfsError> {
        let trimmed = normalize_raw_uri(uri);
        let (scheme, payload) = split_scheme(trimmed)?;
        if !scheme.eq_ignore_ascii_case(WENDAO_URI_SCHEME) {
            return Err(SkillVfsError::UnsupportedScheme {
                uri: trimmed.to_string(),
                scheme: scheme.to_string(),
            });
        }

        let payload = strip_uri_suffix(payload);
        let segments: Vec<&str> = payload.split('/').collect();
        if segments.len() < 3 {
            return Err(SkillVfsError::InvalidUri(trimmed.to_string()));
        }

        let namespace = match segments.first().copied() {
            Some(SKILLS_SEGMENT) => WendaoUriNamespace::Semantic,
            Some(SKILLS_INTERNAL_SEGMENT) => WendaoUriNamespace::Internal,
            _ => return Err(SkillVfsError::InvalidUri(trimmed.to_string())),
        };

        let skill_name = normalize_segment(segments.get(1).copied()).ok_or_else(|| {
            SkillVfsError::MissingUriSegment {
                uri: trimmed.to_string(),
                segment: "skill_name",
            }
        })?;

        let raw_entity = match namespace {
            WendaoUriNamespace::Semantic => {
                if segments.len() < 4 || segments.get(2).copied() != Some(REFERENCES_SEGMENT) {
                    return Err(SkillVfsError::InvalidUri(trimmed.to_string()));
                }
                segments[3..].join("/")
            }
            WendaoUriNamespace::Internal => segments[2..].join("/"),
        };

        let entity_name = normalize_entity_path(&raw_entity, trimmed).map_err(|entity| {
            SkillVfsError::InvalidEntityPath {
                uri: trimmed.to_string(),
                entity,
            }
        })?;
        if Path::new(entity_name.as_str()).extension().is_none() {
            return Err(SkillVfsError::MissingEntityExtension {
                uri: trimmed.to_string(),
                entity: entity_name,
            });
        }

        Ok(Self {
            namespace,
            skill_name,
            entity_name,
        })
    }

    /// Mounted skill identifier.
    ///
    /// For semantic URIs this is the semantic namespace from `skills/<name>`.
    /// For internal URIs this is the mounted internal skill directory name.
    #[must_use]
    pub fn skill_name(&self) -> &str {
        &self.skill_name
    }

    /// Backward-compatible alias for callers that treat the mounted skill id as
    /// a semantic namespace.
    #[must_use]
    pub fn semantic_name(&self) -> &str {
        self.skill_name()
    }

    /// Resource path within the mounted skill namespace.
    #[must_use]
    pub fn entity_name(&self) -> &str {
        &self.entity_name
    }

    /// Returns `true` when the URI targets `skills-internal`.
    #[must_use]
    pub fn is_internal_skill(&self) -> bool {
        matches!(self.namespace, WendaoUriNamespace::Internal)
    }

    /// Canonical URI string with normalized segments.
    #[must_use]
    pub fn canonical_uri(&self) -> String {
        match self.namespace {
            WendaoUriNamespace::Semantic => format!(
                "{WENDAO_URI_SCHEME}://{}/{}/{REFERENCES_SEGMENT}/{}",
                self.namespace.segment(),
                self.skill_name,
                self.entity_name
            ),
            WendaoUriNamespace::Internal => format!(
                "{WENDAO_URI_SCHEME}://{}/{}/{}",
                self.namespace.segment(),
                self.skill_name,
                self.entity_name
            ),
        }
    }

    /// Zero-allocation relative entity path inside the mounted skill namespace.
    #[must_use]
    pub fn entity_relative_path(&self) -> &Path {
        Path::new(self.entity_name())
    }

    /// Candidate relative paths inside the mounted skill namespace.
    #[must_use]
    pub fn candidate_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(self.entity_relative_path())]
    }
}

fn normalize_raw_uri(uri: &str) -> &str {
    uri.trim().trim_start_matches('$').trim_start()
}

fn split_scheme(uri: &str) -> Result<(&str, &str), SkillVfsError> {
    uri.split_once("://")
        .ok_or_else(|| SkillVfsError::InvalidUri(uri.to_string()))
}

fn strip_uri_suffix(payload: &str) -> &str {
    let mut end = payload.len();
    if let Some(index) = payload.find('#') {
        end = end.min(index);
    }
    if let Some(index) = payload.find('?') {
        end = end.min(index);
    }
    payload[..end].trim_matches('/')
}

fn normalize_segment(segment: Option<&str>) -> Option<String> {
    segment
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

fn normalize_entity_path(raw_entity: &str, uri: &str) -> Result<String, String> {
    let trimmed = raw_entity.trim();
    if trimmed.is_empty() {
        return Err(trimmed.to_string());
    }
    if trimmed.split('/').any(|segment| segment.trim().is_empty()) {
        return Err(trimmed.to_string());
    }

    let path = Path::new(trimmed);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("{trimmed} (uri={uri})"));
            }
        }
    }

    let rendered = normalized.to_string_lossy().replace('\\', "/");
    if rendered.is_empty() {
        Err(trimmed.to_string())
    } else {
        Ok(rendered)
    }
}
