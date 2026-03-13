use super::{DiscoveryOptions, SyncManifest, SyncResult};
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Common extension policy for incremental sync routing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalSyncPolicy {
    /// File extensions allowed for sync (e.g. `md`, `txt`).
    pub extensions: Vec<String>,
    /// Glob patterns used to include files.
    pub include_globs: Vec<String>,
    /// Glob patterns used to exclude files.
    pub exclude_globs: Vec<String>,
}

impl Default for IncrementalSyncPolicy {
    fn default() -> Self {
        Self {
            extensions: vec!["md".to_string(), "markdown".to_string()],
            include_globs: Vec::new(),
            exclude_globs: Vec::new(),
        }
    }
}

impl IncrementalSyncPolicy {
    /// Create a new policy with explicit extensions.
    pub fn new(extensions: &[String]) -> Self {
        Self {
            extensions: extensions.to_vec(),
            ..Self::default()
        }
    }

    /// Derives sync policy from glob patterns.
    #[must_use]
    pub fn from_glob_patterns(patterns: &[String], fallback_extensions: &[&str]) -> Self {
        let mut extensions = extract_extensions_from_glob_patterns(patterns);
        if extensions.is_empty() {
            extensions = fallback_extensions.iter().map(|s| s.to_string()).collect();
        }
        Self {
            extensions,
            include_globs: patterns.to_vec(),
            ..Self::default()
        }
    }

    /// Returns true if the path extension matches policy.
    pub fn supports_path(&self, path: &Path) -> bool {
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            return false;
        };
        let lower = ext.to_lowercase();
        self.extensions.iter().any(|e| e == &lower)
    }
}

/// Helper to extract base extensions from a list of globs.
pub fn extract_extensions_from_glob_patterns(patterns: &[String]) -> Vec<String> {
    let mut values = BTreeSet::new();
    for pattern in patterns {
        if let Some(index) = pattern.rfind("*.") {
            let ext = &pattern[index + 2..];
            values.insert(ext.to_lowercase());
        }
    }
    values.into_iter().collect()
}

/// Core synchronization engine.
#[derive(Debug, Clone)]
pub struct SyncEngine {
    /// Root directory of the project to sync.
    pub project_root: PathBuf,
    /// Path where sync manifest is persisted.
    pub manifest_path: PathBuf,
    /// Discovery behavior options.
    pub options: DiscoveryOptions,
}

impl SyncEngine {
    /// Construct a new sync engine for a project.
    pub fn new(project_root: PathBuf, manifest_path: PathBuf) -> Self {
        Self {
            project_root,
            manifest_path,
            options: DiscoveryOptions::default(),
        }
    }

    /// Attach discovery options to the engine.
    pub fn with_options(mut self, options: DiscoveryOptions) -> Self {
        self.options = options;
        self
    }

    /// Discover files under the project root according to discovery options.
    #[must_use]
    pub fn discover_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let root = self.project_root.as_path();
        if !root.is_dir() {
            return files;
        }

        let options = &self.options;
        let extensions: HashSet<String> = options
            .extensions
            .iter()
            .map(|ext| ext.to_ascii_lowercase())
            .collect();
        let skip_dirs: HashSet<String> = options
            .skip_dirs
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if entry.depth() == 0 {
                    return true;
                }
                if entry.file_type().is_dir() {
                    let name = entry.file_name().to_string_lossy();
                    if options.skip_hidden && name.starts_with('.') {
                        return false;
                    }
                    if skip_dirs.contains(name.to_ascii_lowercase().as_str()) {
                        return false;
                    }
                }
                true
            })
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if options.skip_hidden && is_hidden_path(path) {
                continue;
            }
            let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
                continue;
            };
            if !extensions.contains(&ext.to_ascii_lowercase()) {
                continue;
            }
            if let Ok(metadata) = path.metadata() {
                if metadata.len() > options.max_file_size {
                    continue;
                }
            }
            files.push(path.to_path_buf());
            if let Some(limit) = options.max_files {
                if files.len() >= limit {
                    break;
                }
            }
        }

        files.sort();
        files
    }

    /// Compute diff between a manifest snapshot and current file list.
    #[must_use]
    pub fn compute_diff(&self, manifest: &SyncManifest, files: &[PathBuf]) -> SyncResult {
        let mut result = SyncResult::default();
        let mut seen: HashSet<String> = HashSet::new();

        for file in files {
            let key = manifest_key_for_path(file, &self.project_root);
            seen.insert(key.clone());
            match manifest.0.get(&key) {
                None => result.added.push(file.clone()),
                Some(previous) => match Self::compute_file_hash(file) {
                    Some(current) if current == *previous => result.unchanged += 1,
                    _ => result.modified.push(file.clone()),
                },
            }
        }

        for key in manifest.0.keys() {
            if !seen.contains(key) {
                result.deleted.push(PathBuf::from(key));
            }
        }

        result
    }
}

fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn manifest_key_for_path(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}
