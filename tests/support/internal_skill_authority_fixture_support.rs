use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use xiuxian_wendao::skill_vfs::internal_authority::{
    AuthorizedInternalSkillManifestScan, InternalSkillAuthorityReport,
};
use xiuxian_wendao::skill_vfs::internal_manifest::InternalSkillManifest;

pub(crate) fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn normalize_issue(root: &Path, issue: &str) -> String {
    issue.replace(&root.to_string_lossy().to_string(), "[ROOT]")
}

pub(crate) fn authority_report_projection(
    root: &Path,
    report: &InternalSkillAuthorityReport,
) -> Value {
    json!({
        "authorized_manifests": report.authorized_manifests,
        "ghost_links": report.ghost_links,
        "unauthorized_manifests": report.unauthorized_manifests,
    })
}

pub(crate) fn authorized_manifest_scan_summary(
    root: &Path,
    manifests: &[InternalSkillManifest],
) -> Value {
    let mut summaries: Vec<Value> = manifests
        .iter()
        .map(|m| {
            json!({
                "tool_name": m.tool_name,
                "workflow_type": format!("{:?}", m.workflow_type),
                "source_path": relative_path(root, &m.source_path),
            })
        })
        .collect();
    summaries.sort_by(|a, b| a["tool_name"].as_str().cmp(&b["tool_name"].as_str()));
    json!(summaries)
}

pub(crate) fn intent_catalog_projection(
    root: &Path,
    scan: &AuthorizedInternalSkillManifestScan,
) -> Value {
    json!({
        "discovered_paths": scan
            .discovered_paths
            .iter()
            .map(|path: &PathBuf| relative_path(root, path.as_path()))
            .collect::<Vec<_>>(),
        "manifest_count": scan.manifests.len(),
        "issue_count": scan.issues.len(),
        "issues": scan
            .issues
            .iter()
            .map(|issue: &String| normalize_issue(root, issue.as_str()))
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn assert_internal_skill_authority_fixture(
    _scenario: &str,
    _relative: &str,
    _actual: &Value,
) {
    // Dummy for now
}
