//! Fixture-backed contracts for `wendao://` URI parsing.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use xiuxian_wendao::{SkillVfsError, WendaoResourceUri};

use fixture_json_assertions::assert_json_fixture_eq;

#[test]
fn skill_vfs_uri_contract() -> Result<(), Box<dyn std::error::Error>> {
    let semantic = WendaoResourceUri::parse(
        "wendao://skills/agenda-management/references/personas/steward.md?rev=1#section",
    )?;
    let internal = WendaoResourceUri::parse("$wendao://skills-internal/agenda/SKILL.md")?;
    let explicit_extension =
        WendaoResourceUri::parse("wendao://skills/agenda/references/steward.md")?;

    let unsupported_scheme = WendaoResourceUri::parse("file://skills/agenda/references/steward")
        .err()
        .ok_or_else(|| std::io::Error::other("non-wendao schemes must fail"))?;
    let entity_traversal = WendaoResourceUri::parse("wendao://skills/agenda/references/../secrets")
        .err()
        .ok_or_else(|| std::io::Error::other("entity traversal must fail"))?;
    let internal_traversal =
        WendaoResourceUri::parse("wendao://skills-internal/agenda/../secrets.md")
            .err()
            .ok_or_else(|| std::io::Error::other("internal traversal must fail"))?;
    let missing_extension = WendaoResourceUri::parse("wendao://skills/agenda/references/steward")
        .err()
        .ok_or_else(|| std::io::Error::other("extensionless entities must fail"))?;

    let actual = serde_json::json!({
        "semantic": uri_projection(&semantic),
        "internal": uri_projection(&internal),
        "explicit_extension": uri_projection(&explicit_extension),
        "errors": {
            "unsupported_scheme": error_label(&unsupported_scheme),
            "entity_traversal": error_label(&entity_traversal),
            "internal_traversal": error_label(&internal_traversal),
            "missing_extension": error_label(&missing_extension),
        },
    });
    assert_json_fixture_eq(
        "skill_vfs/uri_parser_contract/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

fn uri_projection(uri: &WendaoResourceUri) -> serde_json::Value {
    serde_json::json!({
        "canonical_uri": uri.canonical_uri(),
        "semantic_name": uri.semantic_name(),
        "skill_name": uri.skill_name(),
        "entity_name": uri.entity_name(),
        "is_internal_skill": uri.is_internal_skill(),
        "candidate_paths": uri
            .candidate_paths()
            .into_iter()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>(),
    })
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
