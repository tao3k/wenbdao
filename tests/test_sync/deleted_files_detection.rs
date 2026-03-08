use std::fs;

use tempfile::TempDir;
use xiuxian_wendao::{SyncEngine, SyncManifest};

#[test]
fn test_deleted_files_detection() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    let mut old_manifest = SyncManifest::default();
    old_manifest
        .0
        .insert("deleted1.py".to_string(), "hash1".to_string());
    old_manifest
        .0
        .insert("deleted2.rs".to_string(), "hash2".to_string());
    old_manifest.0.insert(
        "still_exists.py".to_string(),
        SyncEngine::compute_hash("exists"),
    );

    fs::write(temp_dir.path().join("still_exists.py"), "exists")?;

    let files = engine.discover_files();
    let diff = engine.compute_diff(&old_manifest, &files);

    assert!(
        diff.deleted
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "deleted1.py"))
    );
    assert!(
        diff.deleted
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "deleted2.rs"))
    );
    Ok(())
}
