use std::path::PathBuf;
use std::time::Duration;

use crate::gateway::studio::repo_index::state::task::AdaptiveConcurrencyController;
use crate::gateway::studio::repo_index::state::tests::new_coordinator;
use crate::search_plane::SearchPlaneService;

#[test]
fn adaptive_controller_expands_with_backlog_and_fast_feedback() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(4);

    assert_eq!(controller.target_limit(8, 0), 1);

    controller.record_success(Duration::from_millis(20), 7);
    assert_eq!(controller.target_limit(7, 0), 2);

    controller.record_success(Duration::from_millis(18), 6);
    assert_eq!(controller.target_limit(6, 0), 2);

    controller.record_success(Duration::from_millis(18), 5);
    assert_eq!(controller.target_limit(5, 0), 3);

    controller.record_failure();
    assert_eq!(controller.target_limit(5, 0), 1);
}

#[test]
fn adaptive_controller_contracts_when_efficiency_collapses() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(6);
    controller.current_limit = 4;
    controller.ema_elapsed_ms = Some(100.0);
    controller.baseline_elapsed_ms = Some(100.0);
    controller.previous_efficiency = Some(4.0 / 100.0);

    controller.record_success(Duration::from_millis(600), 8);

    assert_eq!(controller.target_limit(8, 0), 2);
}

#[tokio::test]
async fn sync_permit_blocks_second_remote_sync_until_first_releases() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));

    let first = coordinator
        .acquire_sync_permit("alpha/repo")
        .await
        .unwrap_or_else(|error| panic!("first permit: {error}"));
    let blocked = tokio::time::timeout(
        Duration::from_millis(25),
        coordinator.acquire_sync_permit("beta/repo"),
    )
    .await;
    assert!(blocked.is_err());

    drop(first);

    let second = tokio::time::timeout(
        Duration::from_secs(1),
        coordinator.acquire_sync_permit("beta/repo"),
    )
    .await
    .unwrap_or_else(|error| panic!("second permit should become available: {error}"))
    .unwrap_or_else(|error| panic!("second permit acquisition failed: {error}"));
    drop(second);
}
