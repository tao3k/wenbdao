//! Semantic check routines.

mod contracts;
mod identity;
mod links;
mod observations;
mod structure;

pub(crate) use contracts::check_contracts;
pub(crate) use identity::{check_legacy_syntax, check_missing_identity};
pub(crate) use links::{check_dead_links, check_deprecated_refs, check_hash_alignment};
pub(crate) use observations::check_code_observations;
pub(crate) use structure::check_id_collisions;
