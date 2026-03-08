//! Fixture-backed contracts for embedded Wendao resource registry fixtures.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use std::collections::BTreeMap;

use include_dir::{Dir, include_dir};
use xiuxian_wendao::{WendaoResourceRegistry, WendaoResourceRegistryError};

use fixture_json_assertions::assert_json_fixture_eq;

static VALID_EMBEDDED_RESOURCES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/tests/fixtures/embedded-registry/valid");
static MISSING_EMBEDDED_RESOURCES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/tests/fixtures/embedded-registry/missing");
static WENDAO_URI_EMBEDDED_RESOURCES: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/tests/fixtures/embedded-registry/wendao-uri");

#[test]
fn embedded_resource_registry_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let valid = WendaoResourceRegistry::build_from_embedded(&VALID_EMBEDDED_RESOURCES)?;
    let wendao_uri = WendaoResourceRegistry::build_from_embedded(&WENDAO_URI_EMBEDDED_RESOURCES)?;
    let missing = WendaoResourceRegistry::build_from_embedded(&MISSING_EMBEDDED_RESOURCES)
        .err()
        .ok_or_else(|| std::io::Error::other("missing-link fixture should fail validation"))?;

    let actual = serde_json::json!({
        "valid": registry_projection(&valid, &["zhixing/skill.md"]),
        "wendao_uri": registry_projection(&wendao_uri, &["zhixing/skills/agenda-management/SKILL.md"]),
        "missing": error_projection(&missing),
    });

    assert_json_fixture_eq(
        "wendao_registry/embedded_resource_registry/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

fn registry_projection(registry: &WendaoResourceRegistry, paths: &[&str]) -> serde_json::Value {
    let mut config_blocks = registry
        .config_index()
        .values()
        .map(|block| {
            serde_json::json!({
                "id": block.id,
                "config_type": block.config_type,
                "target": block.target,
                "heading": block.heading,
                "language": block.language,
            })
        })
        .collect::<Vec<_>>();
    config_blocks.sort_by(|left, right| {
        left["id"]
            .as_str()
            .cmp(&right["id"].as_str())
            .then_with(|| left["heading"].as_str().cmp(&right["heading"].as_str()))
    });

    let files = paths
        .iter()
        .map(|path| {
            let file = registry
                .file(path)
                .unwrap_or_else(|| panic!("expected registry file `{path}`"));
            let mut links_by_id = file
                .links_by_id()
                .iter()
                .map(|(id, links)| {
                    let mut links = links.clone();
                    links.sort();
                    (id.clone(), links)
                })
                .collect::<BTreeMap<_, _>>();
            for links in links_by_id.values_mut() {
                links.sort();
            }
            let link_targets_by_id = file
                .link_targets_by_id()
                .iter()
                .map(|(id, targets)| {
                    let mut targets = targets
                        .iter()
                        .map(|target| {
                            serde_json::json!({
                                "target_path": target.target_path,
                                "reference_type": target.reference_type,
                            })
                        })
                        .collect::<Vec<_>>();
                    targets.sort_by(|left, right| {
                        left["target_path"]
                            .as_str()
                            .cmp(&right["target_path"].as_str())
                            .then_with(|| {
                                left["reference_type"]
                                    .as_str()
                                    .cmp(&right["reference_type"].as_str())
                            })
                    });
                    (id.clone(), targets)
                })
                .collect::<BTreeMap<_, _>>();

            serde_json::json!({
                "path": file.path(),
                "links_by_id": links_by_id,
                "link_targets_by_id": link_targets_by_id,
                "reference_types": {
                    "persona": file.links_for_reference_type("persona"),
                    "template": file.links_for_reference_type("template"),
                    "attachment": file.links_for_reference_type("attachment"),
                }
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "files_len": registry.files_len(),
        "config_count": registry.config_index().len(),
        "config_blocks": config_blocks,
        "files": files,
    })
}

fn error_projection(error: &WendaoResourceRegistryError) -> serde_json::Value {
    match error {
        WendaoResourceRegistryError::InvalidUtf8 { path } => serde_json::json!({
            "kind": "InvalidUtf8",
            "path": path,
        }),
        WendaoResourceRegistryError::MissingLinkedResources { count, missing } => {
            let mut missing = missing
                .iter()
                .map(|entry| {
                    serde_json::json!({
                        "source_path": entry.source_path,
                        "id": entry.id,
                        "target_path": entry.target_path,
                    })
                })
                .collect::<Vec<_>>();
            missing.sort_by(|left, right| {
                left["source_path"]
                    .as_str()
                    .cmp(&right["source_path"].as_str())
                    .then_with(|| left["id"].as_str().cmp(&right["id"].as_str()))
                    .then_with(|| {
                        left["target_path"]
                            .as_str()
                            .cmp(&right["target_path"].as_str())
                    })
            });
            serde_json::json!({
                "kind": "MissingLinkedResources",
                "count": count,
                "missing": missing,
            })
        }
    }
}
