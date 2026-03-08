//! Shared projections and fixture assertions for Skill-VFS contract tests.

use std::path::Path;

use serde_json::Value;
use xiuxian_skills::{
    InternalSkillManifest, InternalSkillManifestScan, InternalSkillNativeAliasSpec,
    InternalSkillWorkflowType,
};
use xiuxian_wendao::{
    AuthorizedInternalSkillManifestScan, AuthorizedInternalSkillNativeAliasScan, SkillVfsError,
    WendaoResourceUri,
};

use crate::fixture_json_assertions::assert_json_fixture_eq;

pub(crate) fn assert_skill_vfs_contract(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(&format!("skill_vfs/{scenario}/expected"), relative, actual);
}

pub(crate) fn uri_projection(uri: &WendaoResourceUri) -> Value {
    serde_json::json!({
        "canonical_uri": uri.canonical_uri(),
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

pub(crate) fn manifest_projection(root: &Path, manifest: &InternalSkillManifest) -> Value {
    serde_json::json!({
        "manifest_id": manifest.manifest_id,
        "tool_name": manifest.tool_name,
        "description": manifest.description,
        "workflow_type": manifest.workflow_type.as_str(),
        "internal_id": manifest.internal_id,
        "metadata": manifest.metadata,
        "annotations": manifest.annotations,
        "source_path": relative_path(root, manifest.source_path.as_path()),
        "qianhuan_background": manifest.qianhuan_background,
        "flow_definition": manifest.flow_definition,
    })
}

pub(crate) fn scan_projection(root: &Path, scan: InternalSkillManifestScan) -> Value {
    serde_json::json!({
        "discovered_paths": scan
            .discovered_paths
            .into_iter()
            .map(|path| relative_path(root, path.as_path()))
            .collect::<Vec<_>>(),
        "manifests": scan
            .manifests
            .iter()
            .map(|manifest| manifest_projection(root, manifest))
            .collect::<Vec<_>>(),
        "issues": scan
            .issues
            .into_iter()
            .map(|issue| normalize_issue(root, issue.as_str()))
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn authorized_scan_projection(
    root: &Path,
    scan: AuthorizedInternalSkillManifestScan,
) -> Value {
    serde_json::json!({
        "authority": {
            "authorized_manifests": scan.authority.authorized_manifests,
            "ghost_links": scan.authority.ghost_links,
            "unauthorized_manifests": scan.authority.unauthorized_manifests,
        },
        "discovered_paths": scan
            .discovered_paths
            .into_iter()
            .map(|path| relative_path(root, path.as_path()))
            .collect::<Vec<_>>(),
        "manifests": scan
            .manifests
            .into_iter()
            .map(|manifest| {
                serde_json::json!({
                    "manifest_id": manifest.manifest_id,
                    "tool_name": manifest.tool_name,
                    "workflow_type": manifest.workflow_type.as_str(),
                    "internal_id": manifest.internal_id,
                    "metadata": manifest.metadata,
                    "source_path": relative_path(root, manifest.source_path.as_path()),
                })
            })
            .collect::<Vec<_>>(),
        "issues": scan
            .issues
            .into_iter()
            .map(|issue| normalize_issue(root, issue.as_str()))
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn authorized_native_alias_scan_projection(
    root: &Path,
    scan: AuthorizedInternalSkillNativeAliasScan,
) -> Value {
    let compiled_specs = scan
        .compiled_specs
        .iter()
        .map(|spec| alias_spec_projection(root, spec))
        .collect::<Vec<_>>();
    let report = scan.report;
    let authorized_count = report.authorized_count();
    let ghost_count = report.ghost_count();
    let unauthorized_count = report.unauthorized_count();
    let has_authority_drift = report.has_authority_drift();
    let is_critically_failed = report.is_critically_failed();

    serde_json::json!({
        "report": {
            "discovered_paths": report
                .discovered_paths
                .into_iter()
                .map(|path| relative_path(root, path.as_path()))
                .collect::<Vec<_>>(),
            "mounted_specs": report
                .mounted_specs
                .iter()
                .map(|spec| alias_spec_projection(root, spec))
                .collect::<Vec<_>>(),
            "issues": report
                .issues
                .into_iter()
                .map(|issue| normalize_issue(root, issue.as_str()))
                .collect::<Vec<_>>(),
            "authorized_count": authorized_count,
            "ghost_count": ghost_count,
            "unauthorized_count": unauthorized_count,
            "has_authority_drift": has_authority_drift,
            "is_critically_failed": is_critically_failed,
        },
        "compiled_specs": compiled_specs,
    })
}

pub(crate) fn error_label(error: &SkillVfsError) -> &'static str {
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

pub(crate) fn content_excerpt(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(8)
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn error_chain(error: &anyhow::Error) -> Vec<String> {
    error.chain().map(ToString::to_string).collect()
}

pub(crate) fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn alias_spec_projection(
    root: &Path,
    spec: &InternalSkillNativeAliasSpec<InternalSkillWorkflowType>,
) -> Value {
    serde_json::json!({
        "manifest_id": spec.manifest_id,
        "tool_name": spec.tool_name,
        "description": spec.description,
        "workflow_type": spec.workflow_type.as_str(),
        "internal_id": spec.internal_id,
        "metadata": spec.metadata,
        "target_tool_name": spec.target_tool_name,
        "annotations": spec.annotations,
        "source_path": relative_path(root, spec.source_path.as_path()),
    })
}

fn normalize_issue(root: &Path, issue: &str) -> String {
    if let Some((source, reason)) = issue.split_once(" -> ") {
        let source_path = Path::new(source);
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
