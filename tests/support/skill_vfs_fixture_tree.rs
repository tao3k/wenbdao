//! Shared fixture-tree materialization for `xiuxian-wendao` skill VFS tests.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

const FIXTURE_ROOT: &str = "skill_vfs";

pub(crate) fn materialize_skill_vfs_fixture(scenario: &str) -> io::Result<TempDir> {
    let temp_dir = TempDir::new()?;
    copy_tree(
        fixture_path(&format!("{scenario}/input")).as_path(),
        temp_dir.path(),
    )?;
    Ok(temp_dir)
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(FIXTURE_ROOT)
        .join(relative)
}

fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target_path)?;
            copy_tree(source_path.as_path(), target_path.as_path())?;
        } else {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path.as_path(), target_path.as_path())?;
        }
    }

    Ok(())
}
