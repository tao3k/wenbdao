//! Fixture projections for internal-skill authority tests.

use serde_json::Value;
use xiuxian_wendao::{
    AuthorizedInternalSkillManifestScan, InternalSkillAuthorityReport, InternalSkillIntentCatalog,
};

use crate::fixture_json_assertions::assert_json_fixture_eq;

pub(crate) fn assert_internal_skill_authority_fixture(
    scenario: &str,
    relative: &str,
    actual: &Value,
) {
    assert_json_fixture_eq(&format!("skill_vfs/{scenario}/expected"), relative, actual);
}

pub(crate) fn authority_report_projection(report: &InternalSkillAuthorityReport) -> Value {
    serde_json::json!({
        "authorized_manifests": report.authorized_manifests,
        "ghost_links": report.ghost_links,
        "unauthorized_manifests": report.unauthorized_manifests,
    })
}

pub(crate) fn intent_catalog_projection(catalog: &InternalSkillIntentCatalog) -> Value {
    serde_json::json!({
        "intended_manifests": catalog.intended_manifests,
    })
}

pub(crate) fn authorized_manifest_scan_summary(
    root: &std::path::Path,
    scan: &AuthorizedInternalSkillManifestScan,
) -> Value {
    serde_json::json!({
        "discovered_paths": scan
            .discovered_paths
            .iter()
            .map(|path| relative_path(root, path.as_path()))
            .collect::<Vec<_>>(),
        "issues": scan
            .issues
            .iter()
            .map(|issue| normalize_issue(root, issue.as_str()))
            .collect::<Vec<_>>(),
        "tool_names": scan
            .manifests
            .iter()
            .map(|manifest| manifest.tool_name.clone())
            .collect::<Vec<_>>(),
        "authority": authority_report_projection(&scan.authority),
    })
}

pub(crate) fn relative_path(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_issue(root: &std::path::Path, issue: &str) -> String {
    if let Some((source, reason)) = issue.split_once(" -> ") {
        let source_path = std::path::Path::new(source);
        let normalized_source = if source_path.is_absolute() {
            relative_path(root, source_path)
        } else {
            source.replace('\\', "/")
        };
        format!("{normalized_source} -> {reason}")
    } else {
        issue.replace('\\', "/")
    }
}
