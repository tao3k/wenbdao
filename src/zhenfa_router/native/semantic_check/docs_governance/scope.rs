//! Scope matching utilities for docs governance.

use std::path::{Path, PathBuf};

/// Checks if a scope matches the given paths.
pub(crate) fn scope_matches(
    scope: Option<&str>,
    crate_dir: &Path,
    docs_dir: &Path,
    index_path: &Path,
) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    if scope.is_empty() || scope == "." {
        return true;
    }

    if scope_looks_path_like(scope) {
        return path_scope_matches(scope, crate_dir)
            || path_scope_matches(scope, docs_dir)
            || path_scope_matches(scope, index_path);
    }

    let normalized_scope = scope.replace('\\', "/").to_lowercase();
    let crate_path = crate_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let docs_path = docs_dir.to_string_lossy().replace('\\', "/").to_lowercase();
    let index_path = index_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();

    crate_path.contains(&normalized_scope)
        || docs_path.contains(&normalized_scope)
        || index_path.contains(&normalized_scope)
        || normalized_scope.contains(&crate_path)
        || normalized_scope.contains(&docs_path)
        || normalized_scope.contains(&index_path)
}

/// Checks if a scope matches a specific doc path.
pub(crate) fn scope_matches_doc(
    scope: Option<&str>,
    crate_dir: &Path,
    docs_dir: &Path,
    doc_path: &Path,
) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    if scope.is_empty() || scope == "." {
        return true;
    }

    if scope_looks_path_like(scope) {
        if Path::new(scope).extension().and_then(|ext| ext.to_str()) == Some("md") {
            return path_scope_matches(scope, doc_path);
        }

        return path_scope_matches(scope, crate_dir)
            || path_scope_matches(scope, docs_dir)
            || path_scope_matches(scope, doc_path);
    }

    let normalized_scope = scope.replace('\\', "/").to_lowercase();
    let crate_path = crate_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let docs_path = docs_dir.to_string_lossy().replace('\\', "/").to_lowercase();
    let doc_path = doc_path.to_string_lossy().replace('\\', "/").to_lowercase();

    crate_path.contains(&normalized_scope)
        || docs_path.contains(&normalized_scope)
        || doc_path.contains(&normalized_scope)
        || normalized_scope.contains(&crate_path)
        || normalized_scope.contains(&docs_path)
        || normalized_scope.contains(&doc_path)
}

fn scope_looks_path_like(scope: &str) -> bool {
    scope.contains('/')
        || scope.contains('\\')
        || scope.starts_with('.')
        || Path::new(scope).extension().is_some()
}

fn path_scope_matches(scope: &str, target: &Path) -> bool {
    let target_candidates = path_match_candidates(target);
    path_scope_candidates(scope).iter().any(|scope_candidate| {
        target_candidates.iter().any(|target_candidate| {
            target_candidate.starts_with(scope_candidate)
                || scope_candidate.starts_with(target_candidate)
        })
    })
}

fn path_scope_candidates(scope: &str) -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from(scope)];
    if let Ok(canonical) = Path::new(scope).canonicalize()
        && !candidates.iter().any(|candidate| candidate == &canonical)
    {
        candidates.push(canonical);
    }
    candidates
}

fn path_match_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![path.to_path_buf()];
    if let Ok(canonical) = path.canonicalize()
        && !candidates.iter().any(|candidate| candidate == &canonical)
    {
        candidates.push(canonical);
    }
    candidates
}
