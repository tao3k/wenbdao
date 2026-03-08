use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::super::{SkillVfsError, WendaoResourceUri};
use super::core::SkillVfsResolver;

const QIANJI_TOML_FILE: &str = "qianji.toml";
const REFERENCES_DIR: &str = "references";

impl SkillVfsResolver {
    /// Resolve one Wendao URI to one concrete file path.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when URI parsing fails, the mounted namespace is
    /// unknown, or no matching resource exists.
    pub fn resolve_path(&self, uri: &str) -> Result<PathBuf, SkillVfsError> {
        let parsed = WendaoResourceUri::parse(uri)?;
        self.resolve_parsed_uri(&parsed)
    }

    /// Resolve one parsed Wendao URI to one concrete file path.
    ///
    /// # Errors
    ///
    /// Returns [`SkillVfsError`] when the mounted namespace is unknown or the
    /// resource is missing.
    pub fn resolve_parsed_uri(&self, uri: &WendaoResourceUri) -> Result<PathBuf, SkillVfsError> {
        if uri.is_internal_skill() {
            return self.resolve_internal_uri(uri);
        }

        let Some(path) = self.index.path_for_uri(uri).cloned() else {
            let Some(_mounts) = self.index.mounts_for(uri.semantic_name()) else {
                return Err(SkillVfsError::UnknownSemanticSkill {
                    semantic_name: uri.semantic_name().to_string(),
                });
            };
            return Err(SkillVfsError::ResourceNotFound {
                semantic_name: uri.semantic_name().to_string(),
                entity_name: uri.entity_name().to_string(),
            });
        };

        Ok(path)
    }

    /// List canonical internal manifest URIs discovered under mounted internal roots.
    #[must_use]
    pub fn list_internal_manifest_uris(&self) -> Vec<String> {
        let mut uris = Vec::new();
        for root in &self.internal_roots {
            uris.extend(discover_internal_manifest_uris(root.as_path()));
        }
        uris.sort();
        uris.dedup();
        uris
    }

    pub(in crate::skill_vfs::resolver) fn resolve_internal_uri(
        &self,
        uri: &WendaoResourceUri,
    ) -> Result<PathBuf, SkillVfsError> {
        for root in &self.internal_roots {
            let candidate = root.join(uri.skill_name()).join(uri.entity_relative_path());
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        let skill_name = uri.skill_name().to_string();
        if self
            .internal_roots
            .iter()
            .any(|root| root.join(uri.skill_name()).is_dir())
        {
            return Err(SkillVfsError::InternalResourceNotFound {
                skill_name,
                resource_path: uri.entity_name().to_string(),
            });
        }

        Err(SkillVfsError::UnknownInternalSkill { skill_name })
    }
}

fn discover_internal_manifest_uris(root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    let mut uris = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let skill_root = entry.path();
        if !skill_root.is_dir() {
            continue;
        }

        let Some(skill_name) = skill_root
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };

        let references_root = skill_root.join(REFERENCES_DIR);
        if !references_root.is_dir() {
            continue;
        }

        for manifest_path in WalkDir::new(&references_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| entry.file_name().to_str() == Some(QIANJI_TOML_FILE))
            .map(walkdir::DirEntry::into_path)
        {
            let Ok(relative) = manifest_path.strip_prefix(&skill_root) else {
                continue;
            };
            let normalized = normalize_relative_path(relative);
            let segments = normalized.split('/').collect::<Vec<_>>();
            if segments.len() < 3 || segments[0] != REFERENCES_DIR {
                continue;
            }
            uris.push(format!(
                "wendao://skills-internal/{skill_name}/{normalized}"
            ));
        }
    }

    uris
}

fn normalize_relative_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .trim()
        .trim_start_matches("./")
        .replace('\\', "/")
}
