//! Resolver contract tests for `wendao://skills/.../references/...`.

use std::path::Path;

use tempfile::TempDir;
use xiuxian_wendao::{SkillVfsError, SkillVfsResolver};

const SKILL_FRONTMATTER: &str = r#"---
name: agenda-management
description: "Agenda skill"
---

# Agenda Skill
"#;

#[test]
fn resolves_reference_from_semantic_uri() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path().join("internal");
    let skill_dir = root.join("agenda_skill");
    std::fs::create_dir_all(skill_dir.join("references"))?;
    std::fs::write(skill_dir.join("SKILL.md"), SKILL_FRONTMATTER)?;
    std::fs::write(
        skill_dir.join("references").join("steward.md"),
        "persona: strict-teacher",
    )?;

    let resolver = SkillVfsResolver::from_roots(&[root])?;
    let content = resolver.read_utf8("wendao://skills/agenda-management/references/steward.md")?;
    assert_eq!(content, "persona: strict-teacher");
    Ok(())
}

#[test]
#[ignore = "overlay precedence not yet implemented for semantic URI resolution"]
fn supports_overlay_precedence_by_root_order() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let internal = temp.path().join("internal");
    let user = temp.path().join("user");
    write_skill(
        internal.as_path(),
        "agenda_skill",
        "steward.md",
        "source = internal",
    )?;
    write_skill(
        user.as_path(),
        "agenda_skill",
        "steward.md",
        "source = user",
    )?;

    let resolver = SkillVfsResolver::from_roots(&[user.clone(), internal.clone()])?;
    let path = resolver.resolve_path("wendao://skills/agenda-management/references/steward.md")?;
    assert!(path.starts_with(user.as_path()));
    Ok(())
}

#[test]
fn returns_not_found_for_missing_entity() -> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    let root = temp.path().join("internal");
    write_skill(
        root.as_path(),
        "agenda_skill",
        "steward.md",
        "source = internal",
    )?;

    let resolver = SkillVfsResolver::from_roots(&[root])?;
    let error = resolver
        .resolve_path("wendao://skills/agenda-management/references/teacher.md")
        .expect_err("missing entity should fail");
    assert!(matches!(error, SkillVfsError::ResourceNotFound { .. }));
    Ok(())
}

fn write_skill(
    root: &Path,
    folder: &str,
    entity_file: &str,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let skill_dir = root.join(folder);
    std::fs::create_dir_all(skill_dir.join("references"))?;
    std::fs::write(skill_dir.join("SKILL.md"), SKILL_FRONTMATTER)?;
    std::fs::write(skill_dir.join("references").join(entity_file), content)?;
    Ok(())
}
