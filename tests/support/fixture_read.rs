//! Shared read-only fixture helpers for `xiuxian-wendao` integration tests.

use std::fs;
use std::path::PathBuf;

fn fixture_path(fixture_root: &str, relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(fixture_root)
        .join(relative)
}

pub(crate) fn read_fixture(fixture_root: &str, relative: &str) -> String {
    let path = fixture_path(fixture_root, relative);
    fs::read_to_string(path.as_path())
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()))
}
