use std::fs;

use tempfile::TempDir;
use xiuxian_wendao::{SyncEngine, SyncManifest};

#[test]
fn test_compute_diff() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join("new.py"), "new content")?;
    fs::write(temp_dir.path().join("modified.py"), "modified content")?;
    fs::write(temp_dir.path().join("existing.py"), "existing")?;

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    let mut old_manifest = SyncManifest::default();
    old_manifest.0.insert(
        "existing.py".to_string(),
        SyncEngine::compute_hash("existing"),
    );
    old_manifest
        .0
        .insert("modified.py".to_string(), "old_hash".to_string());

    let files = engine.discover_files();
    let diff = engine.compute_diff(&old_manifest, &files);

    assert!(
        diff.added
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "new.py"))
    );
    assert!(
        diff.modified
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "modified.py"))
    );
    assert_eq!(diff.unchanged, 1);
    Ok(())
}
