use std::path::Path;

use super::types::SourceFile;

/// Resolve source files from directory paths.
///
/// This is a simple implementation that scans for common source file extensions.
/// For more sophisticated discovery, use the `dependency_indexer`.
#[must_use]
pub fn resolve_source_files(paths: &[&Path], lang: xiuxian_ast::Lang) -> Vec<SourceFile> {
    let mut files = Vec::new();
    let extensions = lang.extensions();

    for path in paths {
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && extensions.contains(&ext)
                && let Ok(content) = std::fs::read_to_string(path)
            {
                files.push(SourceFile {
                    path: path.display().to_string(),
                    content,
                });
            }
        } else if path.is_dir() {
            // Simple directory scan - could be enhanced with walkdir
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file()
                        && let Some(ext) = entry_path.extension().and_then(|e| e.to_str())
                        && extensions.contains(&ext)
                        && let Ok(content) = std::fs::read_to_string(&entry_path)
                    {
                        files.push(SourceFile {
                            path: entry_path.display().to_string(),
                            content,
                        });
                    }
                }
            }
        }
    }

    files
}
