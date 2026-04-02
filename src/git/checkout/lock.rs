use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use xiuxian_io::PrjDirs;

use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;

const CHECKOUT_LOCK_RETRY_DELAY: Duration = Duration::from_millis(100);
const CHECKOUT_LOCK_MAX_WAIT_ENV: &str = "XIUXIAN_WENDAO_CHECKOUT_LOCK_MAX_WAIT_SECS";
const DEFAULT_CHECKOUT_LOCK_MAX_WAIT_SECS: u64 = 20;
const CHECKOUT_LOCK_STALE_AFTER: Duration = Duration::from_secs(120);
const TOO_MANY_OPEN_FILES_OS_ERROR: i32 = 24;

#[derive(Debug)]
pub(super) struct ManagedCheckoutLock {
    path: PathBuf,
    _file: fs::File,
}

impl Drop for ManagedCheckoutLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(super) fn acquire_managed_checkout_lock(
    repository: &RegisteredRepository,
) -> Result<ManagedCheckoutLock, RepoIntelligenceError> {
    acquire_managed_checkout_lock_with_policy(
        managed_lock_path_for(repository),
        CHECKOUT_LOCK_RETRY_DELAY,
        checkout_lock_max_wait(),
        CHECKOUT_LOCK_STALE_AFTER,
    )
}

fn checkout_lock_max_wait() -> Duration {
    checkout_lock_max_wait_with_lookup(&|key| std::env::var(key).ok())
}

pub(super) fn checkout_lock_max_wait_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Duration {
    let parsed = lookup(CHECKOUT_LOCK_MAX_WAIT_ENV)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0);
    Duration::from_secs(parsed.unwrap_or(DEFAULT_CHECKOUT_LOCK_MAX_WAIT_SECS))
}

pub(super) fn managed_lock_path_for(repository: &RegisteredRepository) -> PathBuf {
    let intelligence_root = PrjDirs::data_home()
        .join("xiuxian-wendao")
        .join("repo-intelligence");
    let mirrors_root = intelligence_root.join("mirrors");
    let managed_mirror_root = super::namespace::managed_mirror_root_for(repository);
    let relative_path = managed_mirror_root.strip_prefix(&mirrors_root).map_or_else(
        |_| PathBuf::from(format!("{}.git", repository.id)),
        Path::to_path_buf,
    );

    intelligence_root
        .join("locks")
        .join(relative_path)
        .with_extension("lock")
}

pub(super) fn acquire_managed_checkout_lock_with_policy(
    lock_path: PathBuf,
    retry_delay: Duration,
    max_wait: Duration,
    stale_after: Duration,
) -> Result<ManagedCheckoutLock, RepoIntelligenceError> {
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to create managed checkout lock dir `{}`: {error}",
                parent.display()
            ),
        })?;
    }

    let started_at = Instant::now();
    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut file) => {
                let _ = writeln!(
                    file,
                    "pid={} acquired_at={}",
                    std::process::id(),
                    Utc::now().to_rfc3339()
                );
                return Ok(ManagedCheckoutLock {
                    path: lock_path,
                    _file: file,
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if lockfile_is_stale(&lock_path, stale_after) {
                    match fs::remove_file(&lock_path) {
                        Ok(()) => continue,
                        Err(remove_error)
                            if remove_error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            continue;
                        }
                        Err(remove_error) => {
                            return Err(RepoIntelligenceError::AnalysisFailed {
                                message: format!(
                                    "failed to reclaim stale managed checkout lock `{}`: {remove_error}",
                                    lock_path.display()
                                ),
                            });
                        }
                    }
                }

                if started_at.elapsed() >= max_wait {
                    return Err(RepoIntelligenceError::AnalysisFailed {
                        message: format!(
                            "timed out waiting for managed checkout lock `{}`",
                            lock_path.display()
                        ),
                    });
                }

                thread::sleep(retry_delay);
            }
            Err(error) if is_descriptor_pressure_error(&error) => {
                if started_at.elapsed() >= max_wait {
                    return Err(RepoIntelligenceError::AnalysisFailed {
                        message: format!(
                            "timed out waiting for managed checkout lock `{}` while file-descriptor pressure persisted: {error}",
                            lock_path.display()
                        ),
                    });
                }

                thread::sleep(retry_delay);
            }
            Err(error) => {
                return Err(RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to acquire managed checkout lock `{}`: {error}",
                        lock_path.display()
                    ),
                });
            }
        }
    }
}

fn lockfile_is_stale(lock_path: &Path, stale_after: Duration) -> bool {
    fs::metadata(lock_path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified_at| modified_at.elapsed().ok())
        .is_some_and(|elapsed| elapsed >= stale_after)
}

pub(super) fn is_descriptor_pressure_error(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(TOO_MANY_OPEN_FILES_OS_ERROR)
}
