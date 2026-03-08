//! Fixture-backed contracts for Wendao asset request APIs.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use std::sync::Arc;

use xiuxian_wendao::{SkillVfsError, WendaoAssetHandle};

use fixture_json_assertions::assert_json_fixture_eq;

#[test]
fn asset_request_api_contract() -> Result<(), Box<dyn std::error::Error>> {
    let normalized = WendaoAssetHandle::skill_reference_asset(
        "Agenda_Management",
        " prompts/../prompts/classifier.md ",
    )
    .err()
    .ok_or_else(|| std::io::Error::other("parent traversal must remain invalid"))?;
    let canonical =
        WendaoAssetHandle::skill_reference_asset("Agenda_Management", "prompts/classifier.md")?;
    let teacher = WendaoAssetHandle::skill_reference_asset("agenda-management", "teacher.md")?;
    let rules = WendaoAssetHandle::skill_reference_asset("agenda-management", "rules.md")?;
    let missing = WendaoAssetHandle::skill_reference_asset("agenda-management", "missing.md")?;

    let callback_text = rules.read_stripped_body_with(|uri| {
        if uri == "wendao://skills/agenda-management/references/rules.md" {
            Some("  callback body  \n".to_string())
        } else {
            None
        }
    })?;
    let callback_shared_first = rules.read_stripped_body_with_shared(|uri| {
        if uri == "wendao://skills/agenda-management/references/rules.md" {
            Some("  callback body  \n".to_string())
        } else {
            None
        }
    })?;
    let callback_shared_second = rules.read_stripped_body_with_shared(|uri| {
        if uri == "wendao://skills/agenda-management/references/rules.md" {
            Some("  callback body  \n".to_string())
        } else {
            None
        }
    })?;

    let teacher_utf8 = teacher.read_utf8()?;
    let teacher_stripped = teacher.read_stripped_body()?;
    let teacher_shared_first = teacher.read_utf8_shared()?;
    let teacher_shared_second = teacher.read_utf8_shared()?;
    let teacher_stripped_first = teacher.read_stripped_body_shared()?;
    let teacher_stripped_second = teacher.read_stripped_body_shared()?;

    let invalid_semantic_name =
        WendaoAssetHandle::skill_reference_asset("agenda management", "teacher.md")
            .err()
            .ok_or_else(|| std::io::Error::other("invalid semantic names must fail"))?;
    let missing_embedded_asset = missing
        .read_utf8()
        .err()
        .ok_or_else(|| std::io::Error::other("missing embedded asset must fail"))?;

    let actual = serde_json::json!({
        "canonical_uri": canonical.uri(),
        "teacher_uri": teacher.uri(),
        "rules_uri": rules.uri(),
        "errors": {
            "invalid_relative_path": error_label(&normalized),
            "invalid_semantic_name": error_label(&invalid_semantic_name),
            "missing_embedded_asset": error_label(&missing_embedded_asset),
        },
        "callback": {
            "stripped_text": callback_text,
            "shared_text": callback_shared_first.as_ref(),
            "shared_pointer_equal": Arc::ptr_eq(&callback_shared_first, &callback_shared_second),
        },
        "embedded_reads": {
            "teacher_utf8_preview": content_excerpt(&teacher_utf8),
            "teacher_plain_stripped_preview": content_excerpt(&teacher_stripped),
            "teacher_shared_preview": content_excerpt(teacher_shared_first.as_ref()),
            "teacher_shared_pointer_equal": Arc::ptr_eq(&teacher_shared_first, &teacher_shared_second),
            "teacher_stripped_preview": content_excerpt(teacher_stripped_first.as_ref()),
            "teacher_stripped_pointer_equal": Arc::ptr_eq(&teacher_stripped_first, &teacher_stripped_second),
        }
    });

    assert_json_fixture_eq(
        "skill_vfs/asset_request_api/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

fn error_label(error: &SkillVfsError) -> &'static str {
    match error {
        SkillVfsError::UnsupportedScheme { .. } => "UnsupportedScheme",
        SkillVfsError::InvalidUri(_) => "InvalidUri",
        SkillVfsError::MissingUriSegment { .. } => "MissingUriSegment",
        SkillVfsError::InvalidEntityPath { .. } => "InvalidEntityPath",
        SkillVfsError::MissingEntityExtension { .. } => "MissingEntityExtension",
        SkillVfsError::ReadSkillDescriptor { .. } => "ReadSkillDescriptor",
        SkillVfsError::ParseSkillFrontmatter { .. } => "ParseSkillFrontmatter",
        SkillVfsError::ScanSkillMetadata { .. } => "ScanSkillMetadata",
        SkillVfsError::UnknownSemanticSkill { .. } => "UnknownSemanticSkill",
        SkillVfsError::UnknownInternalSkill { .. } => "UnknownInternalSkill",
        SkillVfsError::ResourceNotFound { .. } => "ResourceNotFound",
        SkillVfsError::InternalResourceNotFound { .. } => "InternalResourceNotFound",
        SkillVfsError::ReadResource { .. } => "ReadResource",
        SkillVfsError::InvalidRelativeAssetPath { .. } => "InvalidRelativeAssetPath",
        SkillVfsError::EmbeddedAssetNotFound { .. } => "EmbeddedAssetNotFound",
    }
}

fn content_excerpt(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(10)
        .map(ToOwned::to_owned)
        .collect()
}
