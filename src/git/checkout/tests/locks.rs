use std::fs;
use std::time::Duration;

use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
use crate::analyzers::errors::RepoIntelligenceError;

#[test]
fn managed_checkout_lock_reclaims_stale_lockfiles() {
    let repository = RegisteredRepository {
        id: format!("managed-lock-{}", uuid::Uuid::new_v4()),
        path: None,
        url: Some(format!(
            "https://example.com/org/{}.git",
            uuid::Uuid::new_v4()
        )),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let lock_path = crate::git::checkout::lock::managed_lock_path_for(&repository);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).expect("create lock dir");
    }
    fs::write(&lock_path, "stale").expect("write stale lock");

    let guard = crate::git::checkout::lock::acquire_managed_checkout_lock_with_policy(
        lock_path.clone(),
        Duration::from_millis(1),
        Duration::from_millis(5),
        Duration::ZERO,
    )
    .expect("reclaim stale lock");

    assert!(lock_path.exists());
    drop(guard);
    assert!(!lock_path.exists());
}

#[test]
fn managed_checkout_lock_times_out_for_active_lockfiles() {
    let repository = RegisteredRepository {
        id: format!("managed-lock-busy-{}", uuid::Uuid::new_v4()),
        path: None,
        url: Some(format!(
            "https://example.com/org/{}.git",
            uuid::Uuid::new_v4()
        )),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let lock_path = crate::git::checkout::lock::managed_lock_path_for(&repository);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).expect("create lock dir");
    }
    fs::write(&lock_path, "busy").expect("write active lock");

    let error = crate::git::checkout::lock::acquire_managed_checkout_lock_with_policy(
        lock_path.clone(),
        Duration::from_millis(1),
        Duration::from_millis(5),
        Duration::from_secs(60),
    )
    .expect_err("active lock should time out");

    match error {
        RepoIntelligenceError::AnalysisFailed { message } => {
            assert!(message.contains("timed out waiting for managed checkout lock"));
        }
        other => panic!("unexpected error: {other:?}"),
    }

    fs::remove_file(&lock_path).expect("cleanup active lock");
}

#[test]
fn managed_checkout_lock_wait_defaults_to_pressure_tolerant_window() {
    let wait = crate::git::checkout::lock::checkout_lock_max_wait_with_lookup(&|_| None);

    assert_eq!(wait, Duration::from_secs(20));
}

#[test]
fn managed_checkout_lock_wait_accepts_positive_env_override() {
    let wait = crate::git::checkout::lock::checkout_lock_max_wait_with_lookup(&|key| {
        (key == "XIUXIAN_WENDAO_CHECKOUT_LOCK_MAX_WAIT_SECS").then(|| "30".to_string())
    });

    assert_eq!(wait, Duration::from_secs(30));
}

#[test]
fn managed_checkout_lock_recognizes_descriptor_pressure_errors() {
    let error = std::io::Error::from_raw_os_error(24);
    assert!(crate::git::checkout::lock::is_descriptor_pressure_error(
        &error
    ));
}
