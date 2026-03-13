//! Link-graph incremental refresh mode unit tests via public API.

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use xiuxian_wendao::LinkGraphIndex;
use xiuxian_wendao::link_graph::LinkGraphRefreshMode;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_file(path: &Path, content: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[test]
fn link_graph_refresh_mode_is_noop_when_no_changes() -> TestResult {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# Alpha\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# Beta\n\n[[a]]\n")?;

    let mut index = LinkGraphIndex::build(tmp.path()).map_err(|error| error.clone())?;
    let mode = index
        .refresh_incremental_with_threshold(&[], 256)
        .map_err(|error| error.clone())?;
    assert_eq!(mode, LinkGraphRefreshMode::Noop);
    Ok(())
}

#[test]
fn link_graph_refresh_mode_is_full_when_threshold_is_exceeded() -> TestResult {
    let tmp = TempDir::new()?;
    let a_path = tmp.path().join("docs/a.md");
    let b_path = tmp.path().join("docs/b.md");
    write_file(&a_path, "# Alpha\n\n[[b]]\n")?;
    write_file(&b_path, "# Beta\n\n[[a]]\n")?;

    let mut index = LinkGraphIndex::build(tmp.path()).map_err(|error| error.clone())?;
    write_file(&a_path, "# Alpha\n\n[[b]]\n\nnew token\n")?;

    let mode = index
        .refresh_incremental_with_threshold(std::slice::from_ref(&a_path), 1)
        .map_err(|error| error.clone())?;
    assert_eq!(mode, LinkGraphRefreshMode::Full);
    Ok(())
}

#[test]
fn link_graph_refresh_mode_is_delta_when_threshold_not_exceeded() -> TestResult {
    let tmp = TempDir::new()?;
    let a_path = tmp.path().join("docs/a.md");
    let b_path = tmp.path().join("docs/b.md");
    write_file(&a_path, "# Alpha\n\n[[b]]\n")?;
    write_file(&b_path, "# Beta\n\n[[a]]\n")?;

    let mut index = LinkGraphIndex::build(tmp.path()).map_err(|error| error.clone())?;
    write_file(&a_path, "# Alpha\n\n[[b]]\n\nnew token\n")?;

    let mode = index
        .refresh_incremental_with_threshold(std::slice::from_ref(&a_path), 256)
        .map_err(|error| error.clone())?;
    assert_eq!(mode, LinkGraphRefreshMode::Delta);
    Ok(())
}
