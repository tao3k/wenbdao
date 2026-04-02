use std::thread;
use std::time::Duration;

use git2::{AutotagOption, FetchOptions, Repository, build::RepoBuilder};

const MANAGED_REMOTE_RETRY_ATTEMPTS: usize = 3;
const MANAGED_GIT_OPEN_RETRY_ATTEMPTS: usize = 5;
const MANAGED_GIT_OPEN_RETRY_DELAY: Duration = Duration::from_millis(100);

pub(crate) fn should_fetch(
    refresh: crate::analyzers::config::RepositoryRefreshPolicy,
    mode: crate::git::checkout::RepositorySyncMode,
) -> bool {
    matches!(mode, crate::git::checkout::RepositorySyncMode::Refresh)
        || (matches!(mode, crate::git::checkout::RepositorySyncMode::Ensure)
            && matches!(
                refresh,
                crate::analyzers::config::RepositoryRefreshPolicy::Fetch
            ))
}

fn fetch_origin_once(repository: &Repository) -> Result<(), git2::Error> {
    let mut remote = repository.find_remote("origin")?;
    let mut options = FetchOptions::new();
    options.download_tags(AutotagOption::All);
    let refspecs: &[&str] = if repository.is_bare() {
        &["+refs/heads/*:refs/heads/*", "+refs/tags/*:refs/tags/*"]
    } else {
        &[
            "+refs/heads/*:refs/remotes/origin/*",
            "+HEAD:refs/remotes/origin/HEAD",
            "+refs/tags/*:refs/tags/*",
        ]
    };
    remote.fetch(refspecs, Some(&mut options), None)?;
    Ok(())
}

pub(crate) fn fetch_origin_with_retry(repository: &Repository) -> Result<(), git2::Error> {
    retry_remote_operation(|| fetch_origin_once(repository))
}

pub(crate) fn clone_bare_with_retry(
    upstream_url: &str,
    mirror_root: &std::path::Path,
) -> Result<Repository, git2::Error> {
    retry_remote_operation(|| {
        let mut builder = RepoBuilder::new();
        builder.bare(true);
        builder.clone(upstream_url, mirror_root)
    })
}

fn retry_remote_operation<T>(
    mut operation: impl FnMut() -> Result<T, git2::Error>,
) -> Result<T, git2::Error> {
    let mut attempt = 1;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt >= MANAGED_REMOTE_RETRY_ATTEMPTS
                    || !is_retryable_remote_error_message(error.message())
                {
                    return Err(error);
                }
                thread::sleep(retry_delay_for_attempt(attempt));
                attempt += 1;
            }
        }
    }
}

pub(crate) fn open_bare_with_retry(path: &std::path::Path) -> Result<Repository, git2::Error> {
    retry_git_open_operation(|| Repository::open_bare(path))
}

pub(crate) fn open_checkout_with_retry(path: &std::path::Path) -> Result<Repository, git2::Error> {
    retry_git_open_operation(|| Repository::open(path))
}

fn retry_git_open_operation<T>(
    mut operation: impl FnMut() -> Result<T, git2::Error>,
) -> Result<T, git2::Error> {
    let mut attempts = 0usize;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if attempts + 1 < MANAGED_GIT_OPEN_RETRY_ATTEMPTS
                    && retryable_git_open_error_message(error.message()) =>
            {
                attempts += 1;
                thread::sleep(MANAGED_GIT_OPEN_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
}

pub(crate) fn retryable_git_open_error_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("too many open files")
}

fn retry_delay_for_attempt(attempt: usize) -> Duration {
    match attempt {
        0 | 1 => Duration::from_millis(250),
        2 => Duration::from_millis(500),
        _ => Duration::from_secs(1),
    }
}

fn is_retryable_remote_error_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "can't assign requested address",
        "failed to connect",
        "could not connect",
        "timed out",
        "timeout",
        "temporary failure",
        "connection reset",
        "connection refused",
        "connection aborted",
        "network is unreachable",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn ensure_remote_url(
    repository: &Repository,
    remote_name: &str,
    expected_url: &str,
) -> Result<bool, git2::Error> {
    match current_remote_url(repository, remote_name) {
        Some(current) if current == expected_url => Ok(false),
        Some(_) => {
            repository.remote_set_url(remote_name, expected_url)?;
            Ok(true)
        }
        None => {
            repository.remote(remote_name, expected_url)?;
            Ok(true)
        }
    }
}

pub(crate) fn current_remote_url(repository: &Repository, remote_name: &str) -> Option<String> {
    repository
        .find_remote(remote_name)
        .ok()
        .and_then(|remote| remote.url().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::{is_retryable_remote_error_message, retry_delay_for_attempt};

    #[test]
    fn retryable_remote_error_message_matches_transient_transport_failures() {
        assert!(is_retryable_remote_error_message(
            "failed to connect to github.com: Can't assign requested address; class=Os (2)"
        ));
        assert!(is_retryable_remote_error_message(
            "connection reset by peer while fetching packfile"
        ));
        assert!(is_retryable_remote_error_message(
            "operation timed out after 30 seconds"
        ));
    }

    #[test]
    fn retryable_remote_error_message_rejects_non_transient_failures() {
        assert!(!is_retryable_remote_error_message(
            "authentication required but no callback set"
        ));
        assert!(!is_retryable_remote_error_message("reference not found"));
    }

    #[test]
    fn retry_delay_for_attempt_caps_backoff_growth() {
        assert_eq!(retry_delay_for_attempt(1).as_millis(), 250);
        assert_eq!(retry_delay_for_attempt(2).as_millis(), 500);
        assert_eq!(retry_delay_for_attempt(3).as_millis(), 1000);
        assert_eq!(retry_delay_for_attempt(9).as_millis(), 1000);
    }
}
