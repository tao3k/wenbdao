//! Fixture-backed contracts for internal skill VFS resolution.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/skill_vfs_contract_support.rs"]
mod skill_vfs_contract_support;
#[path = "support/skill_vfs_fixture_tree.rs"]
mod skill_vfs_fixture_tree;

use std::io;
use std::sync::Arc;

use xiuxian_wendao::{SkillVfsResolver, WendaoResourceUri};

use skill_vfs_contract_support::{
    assert_skill_vfs_contract, authorized_native_alias_scan_projection, authorized_scan_projection,
    content_excerpt, error_chain, error_label, manifest_projection, relative_path, scan_projection,
    uri_projection,
};
use skill_vfs_fixture_tree::materialize_skill_vfs_fixture;

#[test]
fn internal_skill_resolution_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("internal_skill_resolution")?;
    let internal_root = fixture.path().join("internal_skills");

    let skill_doc_uri = WendaoResourceUri::parse("$wendao://skills-internal/agenda/SKILL.md")?;
    let manifest_uri =
        WendaoResourceUri::parse("wendao://skills-internal/agenda/references/add/qianji.toml")?;
    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_internal_manifests();

    let actual = serde_json::json!({
        "skill_doc_uri": uri_projection(&skill_doc_uri),
        "manifest_uri": uri_projection(&manifest_uri),
        "resolved_paths": {
            "skill_doc": relative_path(
                internal_root.as_path(),
                resolver.resolve_path("$wendao://skills-internal/agenda/SKILL.md")?.as_path(),
            ),
            "manifest": relative_path(
                internal_root.as_path(),
                resolver.resolve_path("wendao://skills-internal/agenda/references/add/qianji.toml")?.as_path(),
            ),
        },
        "manifest_uris": resolver.list_internal_manifest_uris(),
        "scan": scan_projection(internal_root.as_path(), scan),
        "skill_doc_preview": resolver.read_utf8("$wendao://skills-internal/agenda/SKILL.md")?,
    });
    assert_skill_vfs_contract("internal_skill_resolution", "result.json", &actual);
    Ok(())
}

#[test]
fn authorized_internal_skill_scan_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_scans")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_authorized_internal_manifests()?;

    let actual = serde_json::json!({
        "authorized_scan": authorized_scan_projection(internal_root.as_path(), scan),
    });
    assert_skill_vfs_contract("authorized_internal_scans", "manifest_scan.json", &actual);
    Ok(())
}

#[test]
fn authorized_internal_native_alias_scan_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("authorized_internal_scans")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_authorized_internal_native_aliases(internal_root.as_path())?;

    let actual = serde_json::json!({
        "authorized_native_alias_scan": authorized_native_alias_scan_projection(
            internal_root.as_path(),
            scan,
        ),
    });
    assert_skill_vfs_contract(
        "authorized_internal_scans",
        "native_alias_scan.json",
        &actual,
    );
    Ok(())
}

#[test]
fn loaded_internal_skill_manifest_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_loaded")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let manifest = resolver.load_internal_skill_manifest(
        "wendao://skills-internal/agenda/references/add/qianji.toml",
    )?;

    let actual = manifest_projection(internal_root.as_path(), &manifest);
    assert_skill_vfs_contract("manifest_loaded", "contract.json", &actual);
    Ok(())
}

#[test]
fn internal_skill_manifest_error_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_errors")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let invalid_description = resolver
        .load_internal_skill_manifest("wendao://skills-internal/agenda/references/view/qianji.toml")
        .err()
        .ok_or_else(|| io::Error::other("invalid description must fail"))?;
    let missing_background = resolver
        .load_internal_skill_manifest("wendao://skills-internal/agenda/references/add/qianji.toml")
        .err()
        .ok_or_else(|| io::Error::other("missing background binding must fail"))?;

    let actual = serde_json::json!({
        "invalid_description": error_chain(&invalid_description),
        "missing_background": error_chain(&missing_background),
    });
    assert_skill_vfs_contract("manifest_errors", "result.json", &actual);
    Ok(())
}

#[test]
fn internal_skill_manifest_scan_issues_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("manifest_scan_issues")?;
    let internal_root = fixture.path().join("internal_skills");

    let resolver =
        SkillVfsResolver::from_roots_with_internal(&[], std::slice::from_ref(&internal_root))?;
    let scan = resolver.scan_internal_manifests();

    let actual = scan_projection(internal_root.as_path(), scan);
    assert_skill_vfs_contract("manifest_scan_issues", "contract.json", &actual);
    Ok(())
}

#[test]
fn resolver_support_contract() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = materialize_skill_vfs_fixture("resolver_support")?;
    let semantic_internal = fixture.path().join("internal");
    let semantic_user = fixture.path().join("user");
    let internal_first = fixture.path().join("internal-first");
    let internal_second = fixture.path().join("internal-second");

    let semantic_resolver =
        SkillVfsResolver::from_roots(&[semantic_user.clone(), semantic_internal.clone()])?;
    let semantic_uri = "wendao://skills/agenda-management/references/steward.md";
    let semantic_path = semantic_resolver.resolve_path(semantic_uri)?;
    let semantic_content = semantic_resolver.read_utf8(semantic_uri)?;
    let local_first = semantic_resolver.read_utf8_shared(semantic_uri)?;
    let local_second = semantic_resolver.read_utf8_shared(semantic_uri)?;
    let missing_semantic_resource = match semantic_resolver
        .resolve_path("wendao://skills/agenda-management/references/teacher.md")
    {
        Err(error) => error_label(&error),
        Ok(_) => {
            return Err(io::Error::other("missing semantic entity must fail").into());
        }
    };

    let base_resolver = SkillVfsResolver::from_roots(&[])?;
    let missing_embedded_error = match base_resolver.read_utf8(semantic_uri) {
        Err(error) => error_label(&error),
        Ok(_) => {
            return Err(
                io::Error::other("embedded reference should require explicit mount").into(),
            );
        }
    };
    let embedded_resolver = base_resolver.mount_embedded_dir();
    let embedded_content = embedded_resolver.read_utf8(semantic_uri)?;
    let embedded_first = embedded_resolver.read_utf8_shared(semantic_uri)?;
    let embedded_second = embedded_resolver.read_utf8_shared(semantic_uri)?;
    let embedded_semantic = embedded_resolver.read_semantic(semantic_uri)?;

    let internal_resolver =
        SkillVfsResolver::from_roots_with_internal(&[], &[internal_first, internal_second])?;
    let internal_uri = "$wendao://skills-internal/agenda/SKILL.md";
    let internal_content = internal_resolver.read_utf8(internal_uri)?;
    let internal_first_arc = internal_resolver.read_utf8_shared(internal_uri)?;
    let internal_second_arc = internal_resolver.read_utf8_shared(internal_uri)?;
    let missing_internal_namespace =
        match SkillVfsResolver::from_roots_with_internal(&[], &[])?.resolve_path(internal_uri) {
            Err(error) => error_label(&error),
            Ok(_) => {
                return Err(io::Error::other("missing internal namespace must fail").into());
            }
        };

    let actual = serde_json::json!({
        "runtime_internal_root": relative_path(
            fixture.path(),
            SkillVfsResolver::resolve_runtime_internal_root_with(fixture.path(), Some("skills-override"))
                .as_path(),
        ),
        "semantic_resolution": {
            "selected_root": if semantic_path.starts_with(&semantic_user) { "user" } else { "internal" },
            "resolved_path": relative_path(fixture.path(), semantic_path.as_path()),
            "content_excerpt": content_excerpt(&semantic_content),
            "shared_cache": Arc::ptr_eq(&local_first, &local_second),
            "missing_resource_error": missing_semantic_resource,
        },
        "embedded_resolution": {
            "requires_explicit_mount_error": missing_embedded_error,
            "content_excerpt": content_excerpt(&embedded_content),
            "shared_cache": Arc::ptr_eq(&embedded_first, &embedded_second),
            "semantic_alias_shared": Arc::ptr_eq(&embedded_semantic, &embedded_first),
        },
        "internal_overlay": {
            "selected_root": if internal_content.contains("first") { "first" } else { "second" },
            "content_excerpt": content_excerpt(&internal_content),
            "shared_cache": Arc::ptr_eq(&internal_first_arc, &internal_second_arc),
            "missing_namespace_error": missing_internal_namespace,
        },
    });
    assert_skill_vfs_contract("resolver_support", "result.json", &actual);
    Ok(())
}
