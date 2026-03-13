//! Studio API gateway for Qianji frontend.
//!
//! Provides HTTP endpoints for VFS operations, graph queries, and UI configuration.

pub mod types;

#[cfg(feature = "zhenfa-router")]
mod graph;
#[cfg(feature = "zhenfa-router")]
mod router;
#[cfg(feature = "zhenfa-router")]
mod vfs;

#[cfg(feature = "zhenfa-router")]
pub use router::{StudioState, studio_router};
