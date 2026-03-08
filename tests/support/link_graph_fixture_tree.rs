//! Shared fixture-tree materialization for `xiuxian-wendao` `LinkGraph` tests.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

pub(crate) fn materialize_link_graph_fixture(relative_root: &str) -> io::Result<TempDir> {
    let temp_dir = TempDir::new()?;
    copy_tree(fixture_root(relative_root).as_path(), temp_dir.path())?;
    Ok(temp_dir)
}

fn fixture_root(relative_root: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative_root)
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
