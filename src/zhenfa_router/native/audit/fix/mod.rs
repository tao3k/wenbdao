//! CLI Fix Tool for Wendao (Blueprint v3.1).
//!
//! This module provides the foreground executor for the audit bridge,
//! enabling the `wendao fix` CLI command with atomic write-back semantics.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌──────────────────┐
//! │ Semantic Check  │ --> │ generate_surgical │ --> │  AtomicFixBatch  │
//! │ (Issues)        │     │ _fixes            │     │  (apply_all)     │
//! └─────────────────┘     └──────────────────┘     └──────────────────┘
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
//! let report = batch.apply_all();
//!
//! println!("Applied {} fixes to {} files", report.successes, report.files_modified);
//! ```

mod batch;
mod format;
mod hashing;
mod preview;
mod report;

pub(crate) use super::audit_bridge::BatchFix;
pub use batch::AtomicFixBatch;
pub use format::format_fix_preview;
pub use preview::FixPreview;
pub use report::{FileFixResult, FixReport};

#[cfg(test)]
#[path = "../../../../../tests/unit/zhenfa_router/native/audit/fix.rs"]
mod tests;
