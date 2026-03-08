use std::fs;

use tempfile::TempDir;
use xiuxian_wendao::SyncEngine;

#[test]
fn test_skip_hidden_and_directories() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join(".hidden.py"), "hidden")?;
    fs::create_dir_all(temp_dir.path().join(".git"))?;
    fs::write(temp_dir.path().join(".git").join("config"), "config")?;

    fs::write(temp_dir.path().join("visible.py"), "visible")?;

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);
    let files = engine.discover_files();

    assert!(!files.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('.'))
    }));
    assert!(
        files
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "visible.py"))
    );
    Ok(())
}
