use std::path::Path;

use xiuxian_skills::SkillScanner;

use crate::skill_vfs::SkillVfsError;

pub(super) fn is_skill_descriptor(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
}

pub(super) fn parse_semantic_name_from_skill_doc(
    path: &Path,
    scanner: &SkillScanner,
) -> Result<Option<String>, SkillVfsError> {
    let Some(skill_dir) = path.parent() else {
        return Ok(None);
    };

    let metadata =
        scanner
            .scan_skill(skill_dir, None)
            .map_err(
                |error: Box<dyn std::error::Error>| SkillVfsError::ScanSkillMetadata {
                    path: path.to_path_buf(),
                    reason: error.to_string(),
                },
            )?;

    let Some(metadata) = metadata else {
        return Ok(None);
    };

    let name = metadata.skill_name.trim().to_string();

    if name.is_empty() {
        return Ok(None);
    }

    Ok(Some(name.to_ascii_lowercase()))
}
