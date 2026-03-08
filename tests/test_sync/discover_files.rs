use std::fs;

use tempfile::TempDir;
use xiuxian_wendao::SyncEngine;

#[test]
fn test_discover_files() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    fs::write(temp_dir.path().join("test.py"), "print('hello')")?;
    fs::write(temp_dir.path().join("test.md"), "# Hello")?;
    fs::write(temp_dir.path().join("test.txt"), "hello")?;

    let subdir = temp_dir.path().join("subdir");
    fs::create_dir_all(&subdir)?;
    fs::write(subdir.join("module.py"), "def foo(): pass")?;

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);
    let files = engine.discover_files();

    assert!(
        files
            .iter()
            .any(|path| path.extension().is_some_and(|extension| extension == "py"))
    );
    assert!(
        files
            .iter()
            .any(|path| path.extension().is_some_and(|extension| extension == "md"))
    );
    assert!(
        !files
            .iter()
            .any(|path| path.extension().is_some_and(|extension| extension == "txt"))
    );
    Ok(())
}
