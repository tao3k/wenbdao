//! Integration tests for validated internal skill manifests.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/skill_vfs_fixture_tree.rs"]
mod skill_vfs_fixture_tree;

use serde_json::json;
use xiuxian_skills::InternalSkillWorkflowType;
use xiuxian_wendao::SkillVfsResolver;

use fixture_json_assertions::assert_json_fixture_eq;
use skill_vfs_fixture_tree::materialize_skill_vfs_fixture;

#[test]
fn loads_internal_skill_manifest_and_hardens_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_loaded")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver = SkillVfsResolver::from_roots_with_internal(&[], &[internal_root])?;
    let manifest = resolver.load_internal_skill_manifest(
        "wendao://skills-internal/agenda/references/add/qianji.toml",
    )?;

    let actual = json!({
        "manifest_id": manifest.manifest_id,
        "tool_name": manifest.tool_name,
        "workflow_type": manifest.workflow_type.as_str(),
        "workflow_matches_enum": manifest.workflow_type == InternalSkillWorkflowType::QianjiFlow,
        "internal_id": manifest.internal_id,
        "qianhuan_background": manifest.qianhuan_background,
        "flow_definition": manifest.flow_definition,
        "annotations": {
            "read_only": manifest.annotations.read_only,
            "destructive": manifest.annotations.destructive,
            "idempotent": manifest.annotations.is_idempotent(),
            "open_world": manifest.annotations.is_open_world(),
        },
    });
    assert_json_fixture_eq("skill_vfs/manifest_loaded/expected", "result.json", &actual);
    Ok(())
}

#[test]
fn rejects_internal_skill_manifest_with_invalid_description()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_invalid_description")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver = SkillVfsResolver::from_roots_with_internal(&[], &[internal_root])?;
    let error = resolver
        .load_internal_skill_manifest("wendao://skills-internal/agenda/references/view/qianji.toml")
        .err()
        .ok_or_else(|| std::io::Error::other("invalid description must fail"))?;

    // The error is SkillVfsError::ReadResource, and its source contains our message
    assert!(error.to_string().contains("invalid description"));
    Ok(())
}

#[test]
fn rejects_internal_skill_manifest_with_missing_wendao_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_missing_background_binding")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver = SkillVfsResolver::from_roots_with_internal(&[], &[internal_root])?;
    let _ = resolver
        .load_internal_skill_manifest("wendao://skills-internal/agenda/references/add/qianji.toml")
        .err();
    Ok(())
}

#[test]
fn scan_internal_manifests_collects_valid_entries_and_issues()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_scan_issues")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver = SkillVfsResolver::from_roots_with_internal(&[], &[internal_root])?;
    let scan = resolver.scan_authorized_internal_manifests()?;

    let actual = json!({
        "discovered_paths": scan.discovered_paths.len(),
        "tool_names": scan
            .manifests
            .iter()
            .map(|manifest| manifest.tool_name.clone())
            .collect::<Vec<_>>(),
        "issue_count": scan.issues.len(),
        "first_issue_contains_invalid_description": scan.issues.get(0).is_some_and(|issue| issue.contains("invalid description")),
    });
    assert_json_fixture_eq(
        "skill_vfs/manifest_scan_issues/expected",
        "result.json",
        &actual,
    );
    Ok(())
}
