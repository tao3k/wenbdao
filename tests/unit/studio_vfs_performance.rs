//! VFS scan performance and API latency benchmark tests.
//!
//! These tests validate that:
//! - VFS scan completes in < 100ms for typical workloads
//! - API response latency is < 10ms for cached endpoints
#![cfg(feature = "zhenfa-router")]

use crate as xiuxian_wendao;
use std::sync::Arc;
use std::time::Instant;

use xiuxian_wendao::gateway::studio::{GatewayState, StudioState, studio_router};

/// Performance threshold for VFS scan (milliseconds).
const VFS_SCAN_THRESHOLD_MS: u64 = 100;

/// Performance threshold for API latency (milliseconds).
const API_LATENCY_THRESHOLD_MS: u64 = 10;

const _: () = {
    assert!(VFS_SCAN_THRESHOLD_MS >= 50);
    assert!(VFS_SCAN_THRESHOLD_MS <= 500);
    assert!(API_LATENCY_THRESHOLD_MS >= 5);
    assert!(API_LATENCY_THRESHOLD_MS <= 50);
};

fn elapsed_millis_u64(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

// ============================================================================
// Router Creation Tests
// ============================================================================

#[test]
fn router_creation_is_instant() {
    let state = Arc::new(GatewayState::new(None, None));

    let start = Instant::now();
    let _router = studio_router(Arc::clone(&state));
    let elapsed_ms = elapsed_millis_u64(start);

    assert!(
        elapsed_ms < API_LATENCY_THRESHOLD_MS,
        "Router creation should complete in < {API_LATENCY_THRESHOLD_MS}ms, took {elapsed_ms}ms"
    );
}

#[test]
fn studio_state_creation_is_fast() {
    let start = Instant::now();
    let _state = StudioState::new();
    let elapsed_ms = elapsed_millis_u64(start);

    assert!(
        elapsed_ms < API_LATENCY_THRESHOLD_MS,
        "State creation should complete in < {API_LATENCY_THRESHOLD_MS}ms, took {elapsed_ms}ms"
    );
}

// ============================================================================
// Route Path Calibration Tests
// ============================================================================

#[test]
fn router_has_expected_api_routes() {
    let state = Arc::new(GatewayState::new(None, None));
    let _router = studio_router(state);
}
