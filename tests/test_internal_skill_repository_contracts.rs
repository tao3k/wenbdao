//! Repository-backed regression coverage for live `internal_skills` manifests.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use std::path::{Path, PathBuf};

use xiuxian_skills::skills::skill_command::parser::{DescriptionLinter, StructuredDescription};
use xiuxian_wendao::SkillVfsResolver;

use fixture_json_assertions::assert_json_fixture_eq;

#[test]
fn repository_internal_manifests_are_hygienic() -> Result<(), Box<dyn std::error::Error>> {
    let project_root = workspace_root()?;
    let internal_root = project_root.join("internal_skills");
    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_internal_manifests();

    assert!(
        scan.issues.is_empty(),
        "repository internal skill manifests should be valid: {}",
        scan.issues.join(" | ")
    );
    assert!(
        !scan.manifests.is_empty(),
        "repository internal skill scan should discover at least one manifest"
    );

    let manifest_count = scan.manifests.len();
    let manifests = scan
        .manifests
        .into_iter()
        .map(|manifest| {
            let structured = DescriptionLinter::extract(manifest.description.as_str());
            assert_minimum_description_hygiene(
                manifest.manifest_id.as_str(),
                manifest.tool_name.as_str(),
                &structured,
            );
            let arg_count = structured.args.len();
            let example_count = structured.examples.len();
            serde_json::json!({
                "manifest_id": manifest.manifest_id,
                "tool_name": manifest.tool_name,
                "workflow_type": manifest.workflow_type.as_str(),
                "internal_id": manifest.internal_id,
                "annotations": manifest.annotations,
                "source_path": relative_path(project_root.as_path(), manifest.source_path.as_path()),
                "qianhuan_background": manifest.qianhuan_background,
                "flow_definition": manifest.flow_definition,
                "description": {
                    "summary": structured.summary,
                    "arg_count": arg_count,
                    "args": structured
                        .args
                        .into_iter()
                        .map(|arg| {
                            serde_json::json!({
                                "name": arg.name,
                                "arg_type": arg.arg_type,
                                "description": arg.description,
                            })
                        })
                        .collect::<Vec<_>>(),
                    "example_count": example_count,
                    "examples": structured
                        .examples
                        .into_iter()
                        .map(|example| {
                            serde_json::json!({
                                "input": example.input,
                                "output": example.output,
                            })
                        })
                        .collect::<Vec<_>>(),
                },
            })
        })
        .collect::<Vec<_>>();
    let total_arg_count = manifests
        .iter()
        .map(|manifest| manifest["description"]["arg_count"].as_u64().unwrap_or(0))
        .sum::<u64>();
    let total_example_count = manifests
        .iter()
        .map(|manifest| {
            manifest["description"]["example_count"]
                .as_u64()
                .unwrap_or(0)
        })
        .sum::<u64>();

    let actual = serde_json::json!({
        "internal_root": relative_path(project_root.as_path(), internal_root.as_path()),
        "discovered_paths": scan
            .discovered_paths
            .iter()
            .map(|path| relative_path(project_root.as_path(), path.as_path()))
            .collect::<Vec<_>>(),
        "counts": {
            "manifest_count": manifest_count,
            "total_arg_count": total_arg_count,
            "total_example_count": total_example_count,
        },
        "manifests": manifests,
        "issues": scan.issues,
    });
    assert_json_fixture_eq(
        "skill_vfs/repository_internal_manifests/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

fn assert_minimum_description_hygiene(
    manifest_id: &str,
    tool_name: &str,
    structured: &StructuredDescription,
) {
    assert!(
        !structured.summary.trim().is_empty(),
        "manifest `{manifest_id}` / tool `{tool_name}` must have a non-empty summary"
    );
    assert!(
        !structured.args.is_empty(),
        "manifest `{manifest_id}` / tool `{tool_name}` must expose at least one Args entry"
    );
    assert!(
        !structured.examples.is_empty(),
        "manifest `{manifest_id}` / tool `{tool_name}` must expose at least one Examples pair"
    );
}

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    for ancestor in Path::new(env!("CARGO_MANIFEST_DIR")).ancestors() {
        if ancestor.join("Cargo.toml").is_file() && ancestor.join("internal_skills").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }
    Err(std::io::Error::other("failed to resolve workspace root from CARGO_MANIFEST_DIR").into())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
