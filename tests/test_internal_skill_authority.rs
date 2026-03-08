//! Authority checks for internal skill intent vs physical manifest presence.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/internal_skill_authority_fixture_support.rs"]
mod internal_skill_authority_fixture_support;
#[path = "support/skill_vfs_fixture_tree.rs"]
mod skill_vfs_fixture_tree;

use xiuxian_wendao::{InternalSkillIntentCatalog, LinkGraphIndex, SkillVfsResolver};

use internal_skill_authority_fixture_support::{
    assert_internal_skill_authority_fixture, authority_report_projection,
    authorized_manifest_scan_summary, intent_catalog_projection, relative_path,
};
use skill_vfs_fixture_tree::materialize_skill_vfs_fixture;

#[test]
fn audit_internal_manifest_authority_classifies_authorized_ghost_and_unauthorized()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_scans")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver = SkillVfsResolver::from_roots_with_internal(&[], &[internal_root])?;
    let report = resolver.audit_internal_manifest_authority()?;

    let actual = authority_report_projection(&report);
    assert_internal_skill_authority_fixture(
        "authorized_internal_scans",
        "authority_report.json",
        &actual,
    );
    Ok(())
}

#[test]
fn authority_catalog_fast_path_matches_slow_path() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_scans")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let index = LinkGraphIndex::build(internal_root.as_path()).map_err(|error| error.clone())?;
    let fast_catalog = InternalSkillIntentCatalog::from_link_graph_indexes([&index])?;
    let slow_catalog = resolver.collect_internal_manifest_intents()?;
    let fast_authority = resolver.audit_internal_manifest_authority_with_catalog(&fast_catalog);
    let slow_authority = resolver.audit_internal_manifest_authority()?;
    let fast_scan = resolver.scan_authorized_internal_manifests_with_catalog(&fast_catalog);
    let slow_scan = resolver.scan_authorized_internal_manifests()?;

    let actual = serde_json::json!({
        "fast_catalog": intent_catalog_projection(&fast_catalog),
        "slow_catalog": intent_catalog_projection(&slow_catalog),
        "catalogs_match": fast_catalog == slow_catalog,
        "authority_match": fast_authority == slow_authority,
        "scan_match": fast_scan == slow_scan,
    });
    assert_internal_skill_authority_fixture(
        "authorized_internal_scans",
        "catalog_fast_path.json",
        &actual,
    );
    Ok(())
}

#[test]
fn scan_authorized_internal_manifests_keeps_only_authorized_entries()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_scans")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_authorized_internal_manifests()?;

    let actual = authorized_manifest_scan_summary(internal_root.as_path(), &scan);
    assert_internal_skill_authority_fixture(
        "authorized_internal_scans",
        "manifest_scan_summary.json",
        &actual,
    );
    Ok(())
}

#[test]
fn scan_authorized_internal_native_aliases_prepares_report_and_specs()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_scans")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let prepared = resolver.scan_authorized_internal_native_aliases(internal_root.as_path())?;

    let actual = serde_json::json!({
        "root": relative_path(internal_root.parent().unwrap_or(internal_root.as_path()), prepared.report.root.as_path()),
        "discovered_count": prepared.report.discovered_count(),
        "authorized_count": prepared.report.authorized_count(),
        "ghost_count": prepared.report.ghost_count(),
        "unauthorized_count": prepared.report.unauthorized_count(),
        "mounted_spec_count": prepared.report.mounted_specs.len(),
        "tool_names": prepared
            .compiled_specs
            .iter()
            .map(|spec| spec.tool_name.clone())
            .collect::<Vec<_>>(),
        "issues": prepared.report.issues,
    });
    assert_internal_skill_authority_fixture(
        "authorized_internal_scans",
        "native_alias_summary.json",
        &actual,
    );
    Ok(())
}

#[test]
fn scan_authorized_internal_manifests_reports_validation_failures()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_invalid")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_authorized_internal_manifests()?;

    let actual = authorized_manifest_scan_summary(internal_root.as_path(), &scan);
    assert_internal_skill_authority_fixture("authorized_internal_invalid", "result.json", &actual);
    Ok(())
}

#[test]
fn audit_internal_manifest_authority_returns_empty_report_without_internal_roots()
-> Result<(), Box<dyn std::error::Error>> {
    let resolver = SkillVfsResolver::from_roots_with_internal(&[], &[])?;
    let report = resolver.audit_internal_manifest_authority()?;

    let actual = authority_report_projection(&report);
    assert_internal_skill_authority_fixture("no_internal_roots", "result.json", &actual);
    Ok(())
}
