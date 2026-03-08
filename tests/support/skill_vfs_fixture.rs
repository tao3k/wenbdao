//! Shared write helpers for internal-skill VFS tests that seed temporary trees.

use std::fs;
use std::io;
use std::path::Path;

use super::fixture_read::read_fixture;

const FIXTURE_ROOT: &str = "skill_vfs";

pub(crate) fn write_internal_skill_doc(
    root: &Path,
    skill_name: &str,
    content: &str,
) -> io::Result<()> {
    let skill_dir = root.join(skill_name);
    fs::create_dir_all(&skill_dir)?;
    fs::write(skill_dir.join("SKILL.md"), content)
}

pub(crate) fn write_internal_skill_manifest(
    root: &Path,
    skill_name: &str,
    reference_name: &str,
    fixture_name: &str,
) -> io::Result<()> {
    let manifest_dir = root
        .join(skill_name)
        .join("references")
        .join(reference_name);
    fs::create_dir_all(&manifest_dir)?;
    fs::write(
        manifest_dir.join("qianji.toml"),
        read_fixture(FIXTURE_ROOT, fixture_name),
    )
}
