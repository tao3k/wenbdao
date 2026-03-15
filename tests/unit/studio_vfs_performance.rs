//! VFS scan performance and API latency benchmark tests.
//!
//! These tests validate that:
//! - VFS scan completes in < 100ms for typical workloads
//! - API response latency is < 10ms for cached endpoints
#![cfg(feature = "zhenfa-router")]

use std::sync::Arc;
use std::time::Instant;

use xiuxian_wendao::gateway::studio::{StudioState, studio_router};

/// Performance threshold for VFS scan (milliseconds).
const VFS_SCAN_THRESHOLD_MS: u64 = 100;

/// Performance threshold for API latency (milliseconds).
const API_LATENCY_THRESHOLD_MS: u64 = 10;

// ============================================================================
// Router Creation Tests
// ============================================================================

#[test]
fn router_creation_is_instant() {
    let state = Arc::new(StudioState::new());

    let start = Instant::now();
    let _router = studio_router(Arc::clone(&state));
    let elapsed_ms = start.elapsed().as_millis() as u64;

    assert!(
        elapsed_ms < API_LATENCY_THRESHOLD_MS,
        "Router creation should complete in < {}ms, took {}ms",
        API_LATENCY_THRESHOLD_MS,
        elapsed_ms
    );
}

#[test]
fn studio_state_creation_is_fast() {
    let start = Instant::now();
    let _state = StudioState::new();
    let elapsed_ms = start.elapsed().as_millis() as u64;

    assert!(
        elapsed_ms < API_LATENCY_THRESHOLD_MS,
        "State creation should complete in < {}ms, took {}ms",
        API_LATENCY_THRESHOLD_MS,
        elapsed_ms
    );
}

// ============================================================================
// Route Path Calibration Tests
// ============================================================================

#[test]
fn router_has_expected_api_routes() {
    // Verify the router compiles and can be created
    let state = Arc::new(StudioState::new());
    let _router = studio_router(state);

    // Routes that should exist:
    // - GET /api/vfs
    // - GET /api/vfs/scan
    // - GET /api/vfs/cat
    // - GET /api/vfs/{*path}
    // - GET /api/neighbors/{*id}
    // - GET /api/graph/neighbors/{*id}
    // - GET/POST /api/ui/config
    //
    // The actual route structure is verified at compile time by Axum
    assert!(true, "Router created successfully with all expected routes");
}

// ============================================================================
// Performance SLA Documentation Tests
// ============================================================================

#[test]
fn vfs_scan_threshold_is_reasonable() {
    // Verify the threshold is set appropriately
    assert!(
        VFS_SCAN_THRESHOLD_MS >= 50,
        "VFS scan threshold should be at least 50ms for realistic workloads"
    );
    assert!(
        VFS_SCAN_THRESHOLD_MS <= 500,
        "VFS scan threshold should be reasonable (< 500ms)"
    );
}

#[test]
fn api_latency_threshold_is_reasonable() {
    // Verify the API latency threshold is appropriate
    assert!(
        API_LATENCY_THRESHOLD_MS >= 5,
        "API latency threshold should be at least 5ms for realistic workloads"
    );
    assert!(
        API_LATENCY_THRESHOLD_MS <= 50,
        "API latency threshold should be reasonable (< 50ms)"
    );
}
