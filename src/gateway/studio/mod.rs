//! Studio API gateway for Qianji frontend.
//!
//! Provides HTTP endpoints for VFS operations, graph queries, and UI configuration.

pub mod types;

#[cfg(feature = "zhenfa-router")]
mod analysis;
#[cfg(feature = "zhenfa-router")]
mod graph;
#[cfg(feature = "zhenfa-router")]
mod pathing;
#[cfg(feature = "zhenfa-router")]
mod router;
#[cfg(feature = "zhenfa-router")]
mod search;
#[cfg(feature = "zhenfa-router")]
mod vfs;

#[cfg(feature = "zhenfa-router")]
pub use router::{GatewayState, StudioState, studio_router, studio_routes};

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(all(test, feature = "zhenfa-router"))]
#[path = "../../../tests/unit/studio_vfs_performance.rs"]
mod studio_vfs_performance_tests;
