use std::path::Path;

use xiuxian_skills::{SkillScanner, parse_frontmatter_from_markdown};

use crate::skill_vfs::SkillVfsError;

pub(in crate::skill_vfs::index) fn is_skill_descriptor(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md") || name == "skill.md")
}

pub(in crate::skill_vfs::index) fn parse_semantic_name_from_skill_doc(
    path: &Path,
    scanner: &SkillScanner,
) -> Result<Option<String>, SkillVfsError> {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name == "SKILL.md")
    {
        return parse_semantic_name_with_scanner(path, scanner);
    }

    let content =
        std::fs::read_to_string(path).map_err(|source| SkillVfsError::ReadSkillDescriptor {
            path: path.to_path_buf(),
            source,
        })?;
    parse_semantic_name_from_markdown(path, content.as_str())
}

fn parse_semantic_name_from_markdown(
    path: &Path,
    markdown: &str,
) -> Result<Option<String>, SkillVfsError> {
    let Some(value) = parse_frontmatter_from_markdown(markdown).map_err(|source| {
        SkillVfsError::ParseSkillFrontmatter {
            path: path.to_path_buf(),
            source,
        }
    })?
    else {
        return Ok(None);
    };
    let name = value
        .get("name")
        .and_then(serde_yaml::Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_ascii_lowercase);
    Ok(name)
}

fn parse_semantic_name_with_scanner(
    path: &Path,
    scanner: &SkillScanner,
) -> Result<Option<String>, SkillVfsError> {
    let Some(skill_dir) = path.parent() else {
        return Ok(None);
    };
    let metadata =
        scanner
            .scan_skill(skill_dir, None)
            .map_err(|error| SkillVfsError::ScanSkillMetadata {
                path: path.to_path_buf(),
                reason: error.to_string(),
            })?;
    let semantic_name = metadata
        .as_ref()
        .map(|item| item.skill_name.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    Ok(semantic_name)
}
