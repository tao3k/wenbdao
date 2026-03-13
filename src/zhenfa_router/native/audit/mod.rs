//! Context Completeness Score (CCS) audit module for wendao.search.
//!
//! Implements Synapse-Audit (2025) principles for context quality gating.
//! This module is intentionally self-contained to avoid circular dependencies
//! with xiuxian-qianhuan.
//!
//! ## Module Structure
//!
//! - `ccs` - Core CCS calculation and drift scoring
//! - `verdict` - Audit verdict and result types
//! - `compensation` - Compensation request for secondary search
//!
//! ## Usage
//!
//! ```ignore
//! use crate::zhenfa_router::native::audit::{audit_search_payload, AuditResult};
//!
//! let result = audit_search_payload(&evidence, &anchors);
//! if !result.passed {
//!     // Apply compensation
//! }
//! ```

mod ccs;
mod compensation;
mod verdict;

pub use ccs::{audit_search_payload, evaluate_alignment};
pub use compensation::CompensationRequest;
pub use verdict::AuditResult;
