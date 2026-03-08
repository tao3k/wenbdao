use std::fs;

use tempfile::TempDir;
use xiuxian_wendao::{SyncEngine, SyncManifest};

#[test]
fn test_batch_diff_computation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    for i in 0..50 {
        fs::write(
            temp_dir.path().join(format!("file_{i}.py")),
            format!("content {i}"),
        )?;
    }

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    let empty_manifest = SyncManifest::default();
    let files = engine.discover_files();
    let diff = engine.compute_diff(&empty_manifest, &files);

    assert_eq!(diff.added.len(), 50);
    assert!(diff.modified.is_empty());
    assert_eq!(diff.unchanged, 0);
    Ok(())
}
