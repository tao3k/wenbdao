use super::DependencyIndexer;
use crate::dependency_indexer::indexer::ExternalSymbol;
use crate::dependency_indexer::indexer::files::extract_symbols_from_crate;
use std::path::PathBuf;

impl DependencyIndexer {
    /// Process a single manifest (thread-safe version for parallel processing).
    pub(super) fn process_manifest_inner(
        manifest_path: &PathBuf,
    ) -> Result<(String, String, PathBuf, Vec<ExternalSymbol>), String> {
        use std::fs;

        let content = fs::read_to_string(manifest_path)
            .map_err(|error| format!("Failed to read manifest: {error}"))?;

        let value: toml::Value = content
            .parse()
            .map_err(|error| format!("Failed to parse TOML: {error}"))?;

        let package_name = value
            .get("package")
            .and_then(|pkg| pkg.get("name"))
            .and_then(|name| name.as_str())
            .ok_or("No package name found in manifest")?;

        let version = value
            .get("package")
            .and_then(|pkg| pkg.get("version"))
            .and_then(|version| version.as_str())
            .unwrap_or("unknown")
            .to_string();

        let source_path = manifest_path
            .parent()
            .ok_or("No parent directory for manifest")?
            .to_path_buf();

        // Extract symbols from Rust source files
        let symbols = extract_symbols_from_crate(&source_path, package_name);

        Ok((package_name.to_string(), version, source_path, symbols))
    }
}
