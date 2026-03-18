//! CLI Fix Tool for Wendao (Blueprint v3.1)
//!
//! This module provides the "foreground executor" for the audit bridge,
//! enabling `wendao fix` CLI command with atomic write-back semantics.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌────────────────���──┐     ┌──────────────────┐
//! │ Semantic Check  │ --> │ generate_surgical │ --> │  AtomicFixBatch  │
//! │ (Issues)        │     │ _fixes            │     │  (apply_all)     │
//! └─────────────────┘     └───────────────────┘     └──────────────────┘
//!                                                          │
//!                                                          ▼
//!                                                 ┌──────────────────┐
//!                                                 │ File System      │
//!                                                 │ (atomic writes)  │
//!                                                 └──────────────────┘
//! ```
//!
//! ## Atomic Write-Back Protocol
//!
//! 1. **Collect**: Gather all fixes grouped by file
//! 2. **Preview**: Show diff preview of each fix
//! 3. **Apply (In-Memory)**: Apply all fixes to in-memory content
//! 4. **Verify**: All fixes must succeed for any file to be written
//! 5. **Commit**: Write all modified files atomically
//!
//! ## Usage
//!
//! ```ignore
//! use crate::zhenfa_router::native::audit::fix::{AtomicFixBatch, FixReport};
//!
//! let batch = AtomicFixBatch::new(fixes);
//! let report = batch.apply_all()?;
//!
//! println!("Applied {} fixes to {} files", report.successes, report.files_modified);
//! ```

use std::collections::HashMap;
use std::fmt::Write as _;
use std::hash::BuildHasher;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::audit_bridge::{BatchFix, FixResult};

/// Compute Blake3 hash of content (one-time verification).
fn compute_blake3_hash(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}

/// Result of applying a single fix to a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFixResult {
    /// Path to the file.
    pub path: String,
    /// Result of the fix operation (as string for serialization).
    pub result: String,
    /// The line number of the fix.
    pub line_number: usize,
    /// Confidence score.
    pub confidence: f32,
}

/// Summary report of a batch fix operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FixReport {
    /// Number of fixes successfully applied.
    pub successes: usize,
    /// Number of fixes that failed.
    pub failures: usize,
    /// Number of files modified.
    pub files_modified: usize,
    /// Number of files skipped (due to failures).
    pub files_skipped: usize,
    /// Detailed results for each fix.
    pub results: Vec<FileFixResult>,
    /// Error messages for failed fixes.
    pub errors: Vec<String>,
}

impl FixReport {
    /// Check if all fixes were successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.failures == 0
    }

    /// Get a summary string for display.
    #[must_use]
    pub fn summary(&self) -> String {
        if self.is_success() {
            format!(
                "✓ Applied {} fixes to {} files",
                self.successes, self.files_modified
            )
        } else {
            format!(
                "✗ {} fixes failed, {} succeeded ({} files modified, {} skipped)",
                self.failures, self.successes, self.files_modified, self.files_skipped
            )
        }
    }
}

/// Atomic batch fix executor.
///
/// This struct manages the atomic application of multiple fixes across
/// multiple files. It ensures that either all fixes for a file succeed,
/// or none are applied (all-or-nothing per file).
#[derive(Debug)]
pub struct AtomicFixBatch {
    /// Fixes grouped by file path.
    fixes_by_file: HashMap<PathBuf, Vec<BatchFix>>,
    /// Whether to perform dry-run (preview only).
    dry_run: bool,
    /// Minimum confidence threshold for automatic application.
    confidence_threshold: f32,
}

impl AtomicFixBatch {
    /// Create a new atomic fix batch from a list of fixes.
    #[must_use]
    pub fn new(fixes: Vec<BatchFix>) -> Self {
        let mut fixes_by_file: HashMap<PathBuf, Vec<BatchFix>> = HashMap::new();
        for fix in fixes {
            let path = PathBuf::from(&fix.doc_path);
            fixes_by_file.entry(path).or_default().push(fix);
        }

        Self {
            fixes_by_file,
            dry_run: false,
            confidence_threshold: 0.0,
        }
    }

    /// Set dry-run mode (preview only, no file modifications).
    #[must_use]
    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    /// Set minimum confidence threshold for automatic application.
    #[must_use]
    pub fn confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Filter fixes by confidence threshold.
    fn filter_by_confidence(&self) -> HashMap<PathBuf, Vec<BatchFix>> {
        self.fixes_by_file
            .iter()
            .map(|(path, fixes)| {
                let filtered: Vec<BatchFix> = fixes
                    .iter()
                    .filter(|f| f.confidence >= self.confidence_threshold)
                    .cloned()
                    .collect();
                (path.clone(), filtered)
            })
            .filter(|(_, fixes)| !fixes.is_empty())
            .collect()
    }

    /// Preview all fixes without applying them.
    ///
    /// Returns a map of file paths to preview strings (diff-like output).
    #[must_use]
    pub fn preview_all(&self) -> HashMap<PathBuf, Vec<FixPreview>> {
        let filtered = self.filter_by_confidence();
        let mut previews = HashMap::new();

        for (path, fixes) in filtered {
            // Read file content
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };

            let file_previews: Vec<FixPreview> = fixes
                .iter()
                .filter_map(|fix| {
                    let preview_content = fix.preview(&content).ok()?;
                    Some(FixPreview {
                        line_number: fix.line_number,
                        original: fix.original_content.clone(),
                        replacement: fix.replacement.clone(),
                        confidence: fix.confidence,
                        is_surgical: fix.is_surgical(),
                        preview_content,
                    })
                })
                .collect();

            if !file_previews.is_empty() {
                previews.insert(path, file_previews);
            }
        }

        previews
    }

    /// Apply all fixes atomically.
    ///
    /// For each file:
    /// 1. Read current content
    /// 2. ONE-TIME hash verification (CAS check) before any modifications
    /// 3. Sort fixes by byte range (descending) to avoid offset issues
    /// 4. Apply all fixes to in-memory content
    /// 5. If ALL fixes succeed, write back to file
    /// 6. If ANY fix fails, skip the file entirely
    ///
    /// # One-Time Hash Verification (v3.1)
    ///
    /// Instead of checking the hash in each `apply_surgical` call, we verify
    /// the file hash ONCE before applying any fixes. This is:
    /// - More efficient (single hash computation per file)
    /// - More correct (hash is checked before ANY modifications)
    /// - Simpler (hash verification logic is centralized here)
    ///
    /// # Reverse Application Strategy
    ///
    /// Fixes are applied from END to BEGINNING of the file. This ensures that
    /// applying one fix doesn't invalidate the byte ranges of subsequent fixes.
    /// For example, if Fix A modifies bytes 0-10 and Fix B modifies bytes 20-30,
    /// applying them in order (A then B) works fine. But if Fix A changes the
    /// content length, Fix B's byte range becomes invalid. By applying from
    /// highest byte offset to lowest, we avoid this problem.
    ///
    /// # Errors
    ///
    /// Returns a `FixReport` even on errors - check `report.is_success()`
    /// and `report.errors` for details.
    #[must_use]
    pub fn apply_all(&self) -> FixReport {
        let mut report = FixReport::default();
        let filtered = self.filter_by_confidence();

        for (path, mut fixes) in filtered {
            // Read file content
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    report
                        .errors
                        .push(format!("Failed to read {}: {}", path.display(), e));
                    report.files_skipped += 1;
                    continue;
                }
            };

            // ONE-TIME hash verification (CAS check) before any modifications
            // Get expected hash from first surgical fix (all fixes for same file share same base_hash)
            if let Some(first_fix) = fixes.iter().find(|f| f.is_surgical())
                && let Some(ref expected_hash) = first_fix.base_hash
            {
                let actual_hash = compute_blake3_hash(&content);
                if &actual_hash != expected_hash {
                    report.failures += fixes.len();
                    report.files_skipped += 1;
                    report.errors.push(format!(
                        "Hash mismatch for {}: expected {}..8, got {}..8",
                        path.display(),
                        &expected_hash[..8.min(expected_hash.len())],
                        &actual_hash[..8]
                    ));
                    continue;
                }
            }

            // Sort fixes by byte range (descending) to avoid offset issues
            // Fixes without byte_range go last (they use string search)
            fixes.sort_by(|a, b| {
                let a_start = a.byte_range.as_ref().map_or(usize::MAX, |r| r.start);
                let b_start = b.byte_range.as_ref().map_or(usize::MAX, |r| r.start);
                b_start.cmp(&a_start)
            });

            // Apply all fixes to in-memory content
            let mut modified_content = content.clone();
            let mut file_success = true;

            for fix in &fixes {
                let result = fix.apply_surgical(&mut modified_content);
                let is_success = matches!(result, FixResult::Success);

                report.results.push(FileFixResult {
                    path: path.to_string_lossy().to_string(),
                    result: result.to_string(),
                    line_number: fix.line_number,
                    confidence: fix.confidence,
                });

                if is_success {
                    report.successes += 1;
                } else {
                    report.failures += 1;
                    report.errors.push(format!(
                        "Fix failed at {}:{}: {}",
                        path.display(),
                        fix.line_number,
                        result
                    ));
                    file_success = false;
                }
            }

            // Only write if all fixes succeeded AND not in dry-run mode
            if file_success && !self.dry_run {
                match std::fs::write(&path, &modified_content) {
                    Ok(()) => {
                        report.files_modified += 1;
                    }
                    Err(e) => {
                        report
                            .errors
                            .push(format!("Failed to write {}: {}", path.display(), e));
                        report.files_skipped += 1;
                    }
                }
            } else if file_success && self.dry_run {
                // Count as modified in dry-run mode for reporting
                report.files_modified += 1;
            } else {
                report.files_skipped += 1;
            }
        }

        report
    }

    /// Get the total number of fixes.
    #[must_use]
    pub fn total_fixes(&self) -> usize {
        self.fixes_by_file.values().map(std::vec::Vec::len).sum()
    }

    /// Get the number of files affected.
    #[must_use]
    pub fn files_affected(&self) -> usize {
        self.fixes_by_file.len()
    }
}

/// Preview of a single fix operation.
#[derive(Debug, Clone)]
pub struct FixPreview {
    /// Line number where the fix applies.
    pub line_number: usize,
    /// Original content to be replaced.
    pub original: String,
    /// Replacement content.
    pub replacement: String,
    /// Confidence score for this fix.
    pub confidence: f32,
    /// Whether this is a surgical (byte-precise) fix.
    pub is_surgical: bool,
    /// Full preview of the file content after applying this fix.
    pub preview_content: String,
}

impl std::fmt::Display for FixPreview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Line {}: (confidence: {:.0}%)",
            self.line_number,
            self.confidence * 100.0
        )?;
        writeln!(f, "  - {}", self.original)?;
        writeln!(f, "  + {}", self.replacement)?;
        if self.is_surgical {
            write!(f, "  [surgical: byte-precise]")
        } else {
            write!(f, "  [legacy: string search]")
        }
    }
}

/// Generate a diff-style preview of fixes.
#[must_use]
pub fn format_fix_preview<S: BuildHasher>(
    previews: &HashMap<PathBuf, Vec<FixPreview>, S>,
) -> String {
    let mut output = String::new();

    macro_rules! append {
        ($($arg:tt)*) => {
            if write!(output, $($arg)*).is_err() {
                unreachable!("writing fix preview into String cannot fail");
            }
        };
    }

    for (path, file_previews) in previews {
        append!(
            "=== {} ({} fixes) ===\n",
            path.display(),
            file_previews.len()
        );

        for preview in file_previews {
            append!("{preview}\n\n");
        }
    }

    output
}

#[cfg(test)]
#[path = "../../../../tests/unit/zhenfa_router/native/audit/fix.rs"]
mod tests;
