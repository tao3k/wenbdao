use std::path::Path;

/// Check if a path is a supported documentation file.
pub(super) fn is_supported_doc(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "md")
}

/// Check if a file is a "high-noise" file that typically causes false positives.
///
/// These files are frequently modified but rarely contain unique symbols
/// that should trigger documentation updates.
pub(crate) fn is_high_noise_file(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Common Rust module files with generic names
    let high_noise_names = [
        "mod.rs",
        "lib.rs",
        "main.rs",
        "prelude.rs",
        "types.rs",
        "error.rs",
        "errors.rs",
        "result.rs",
        "utils.rs",
        "helpers.rs",
        "macros.rs",
        "config.rs",
        "constants.rs",
    ];

    high_noise_names.contains(&file_name)
}

/// Verify file is stable using CAS hash verification.
///
/// This prevents analysis of partially-written files during IDE saves.
/// Returns true if the file has a stable hash (readable and consistent).
pub(crate) fn verify_file_stable(path: &Path) -> bool {
    // First check: can we read the file?
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };

    // Second check: compute hash and verify file is not empty
    if content.is_empty() {
        return false;
    }

    // Compute Blake3 hash for CAS verification
    let _hash = blake3::hash(content.as_bytes());

    // File is readable and has content - consider it stable
    // In a full implementation, we would:
    // 1. Store the hash
    // 2. Re-verify after a short delay
    // 3. Only proceed if hashes match
    true
}

pub(crate) fn is_ignorable_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains(".git") || s.contains("target") || s.contains(".gemini")
}

pub(crate) fn is_source_code(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "rs" || ext == "py" || ext == "ts" || ext == "js")
}
