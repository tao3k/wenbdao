use std::fs;

use tempfile::TempDir;
use xiuxian_wendao::{DiscoveryOptions, SyncEngine};

#[test]
fn test_custom_discovery_options() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join("test.rs"), "fn main() {}")?;
    fs::write(temp_dir.path().join("test.go"), "package main")?;

    let manifest_path = temp_dir.path().join("manifest.json");
    let options = DiscoveryOptions {
        extensions: vec!["rs".to_string()],
        ..Default::default()
    };

    let engine = SyncEngine::new(temp_dir.path(), &manifest_path).with_options(options);
    let files = engine.discover_files();

    assert_eq!(files.len(), 1);
    assert!(
        files[0]
            .extension()
            .is_some_and(|extension| extension == "rs")
    );
    Ok(())
}
