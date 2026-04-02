use std::time::Duration;

use crate::analyzers::{RegisteredRepository, RepoIntelligenceError};

pub(crate) const REPO_INDEX_ANALYSIS_TIMEOUT: Duration = Duration::from_secs(45);
const DEFAULT_REPO_INDEX_SYNC_CONCURRENCY: usize = 1;
const REPO_INDEX_SYNC_CONCURRENCY_ENV: &str = "XIUXIAN_WENDAO_REPO_INDEX_SYNC_CONCURRENCY";
const MAX_REPO_INDEX_SYNC_REQUEUE_ATTEMPTS: usize = 1;

fn bounded_usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

pub(crate) fn repo_index_sync_concurrency_limit() -> usize {
    repo_index_sync_concurrency_limit_with_lookup(&|key| std::env::var(key).ok())
}

fn repo_index_sync_concurrency_limit_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> usize {
    lookup(REPO_INDEX_SYNC_CONCURRENCY_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_REPO_INDEX_SYNC_CONCURRENCY)
}

pub(crate) fn should_retry_sync_failure(error: &RepoIntelligenceError, retry_count: usize) -> bool {
    retry_count < MAX_REPO_INDEX_SYNC_REQUEUE_ATTEMPTS && is_retryable_sync_failure(error)
}

fn is_retryable_sync_failure(error: &RepoIntelligenceError) -> bool {
    let message = match error {
        RepoIntelligenceError::AnalysisFailed { message } => message.as_str(),
        RepoIntelligenceError::InvalidRepositoryPath { reason, .. } => reason.as_str(),
        _ => return false,
    }
    .to_ascii_lowercase();
    [
        "can't assign requested address",
        "failed to connect to github.com",
        "failed to resolve address",
        "connection reset by peer",
        "temporary failure in name resolution",
        "resource temporarily unavailable",
        "operation timed out",
        "timed out",
        "too many open files",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepoIndexTaskPriority {
    Background,
    Interactive,
}

#[derive(Debug, Clone)]
pub(crate) struct RepoIndexTask {
    pub(crate) repository: RegisteredRepository,
    pub(crate) refresh: bool,
    pub(crate) fingerprint: String,
    pub(crate) priority: RepoIndexTaskPriority,
    pub(crate) retry_count: usize,
}

#[derive(Debug)]
pub(crate) struct AdaptiveConcurrencyController {
    pub(crate) current_limit: usize,
    pub(crate) max_limit: usize,
    pub(crate) success_streak: usize,
    pub(crate) ema_elapsed_ms: Option<f64>,
    pub(crate) baseline_elapsed_ms: Option<f64>,
    pub(crate) previous_efficiency: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdaptiveConcurrencySnapshot {
    pub(crate) current_limit: usize,
    pub(crate) max_limit: usize,
}

impl AdaptiveConcurrencyController {
    pub(crate) fn new() -> Self {
        let max_limit = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1)
            .max(1);
        Self {
            current_limit: 1,
            max_limit,
            success_streak: 0,
            ema_elapsed_ms: None,
            baseline_elapsed_ms: None,
            previous_efficiency: None,
        }
    }

    #[cfg(test)]
    pub(super) fn new_for_test(max_limit: usize) -> Self {
        Self {
            current_limit: 1,
            max_limit: max_limit.max(1),
            success_streak: 0,
            ema_elapsed_ms: None,
            baseline_elapsed_ms: None,
            previous_efficiency: None,
        }
    }

    pub(crate) fn snapshot(&self) -> AdaptiveConcurrencySnapshot {
        AdaptiveConcurrencySnapshot {
            current_limit: self.current_limit.max(1).min(self.max_limit.max(1)),
            max_limit: self.max_limit.max(1),
        }
    }

    pub(crate) fn target_limit(&mut self, queued: usize, active: usize) -> usize {
        let demand = queued.saturating_add(active);
        if demand <= 1 {
            self.current_limit = 1;
            return 1;
        }
        if queued == 0 && active < self.current_limit {
            self.current_limit = active.max(1);
        }
        self.current_limit
            .max(1)
            .min(self.max_limit.max(1))
            .min(demand)
    }

    pub(crate) fn record_success(&mut self, elapsed: Duration, queued_remaining: usize) {
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        let baseline = self.ema_elapsed_ms.unwrap_or(elapsed_ms);
        self.ema_elapsed_ms = Some(if self.ema_elapsed_ms.is_some() {
            baseline.mul_add(0.75, elapsed_ms * 0.25)
        } else {
            elapsed_ms
        });
        let ema_elapsed_ms = self.ema_elapsed_ms.unwrap_or(elapsed_ms);
        self.baseline_elapsed_ms = Some(
            self.baseline_elapsed_ms
                .map_or(ema_elapsed_ms, |existing| existing.min(ema_elapsed_ms)),
        );

        let efficiency = bounded_usize_to_f64(self.current_limit) / ema_elapsed_ms.max(1.0);
        let previous_efficiency = self.previous_efficiency.unwrap_or(efficiency);
        let efficiency_ratio = if previous_efficiency > 0.0 {
            efficiency / previous_efficiency
        } else {
            1.0
        };
        let io_pressure_detected = self
            .baseline_elapsed_ms
            .is_some_and(|baseline_ms| ema_elapsed_ms >= baseline_ms * 3.0);

        if queued_remaining == 0 {
            self.success_streak = 0;
            self.previous_efficiency = Some(efficiency);
            return;
        }

        if io_pressure_detected || efficiency_ratio < 0.80 {
            self.current_limit = (self.current_limit / 2).max(1);
            self.success_streak = 0;
            self.previous_efficiency = Some(efficiency);
            return;
        }

        if efficiency_ratio >= 0.95 {
            self.success_streak = self.success_streak.saturating_add(1);
            if self.success_streak >= self.current_limit && self.current_limit < self.max_limit {
                self.current_limit += 1;
                self.success_streak = 0;
            }
            self.previous_efficiency = Some(efficiency);
            return;
        }

        self.success_streak = 0;
        self.previous_efficiency = Some(efficiency);
    }

    pub(crate) fn record_failure(&mut self) {
        self.current_limit = (self.current_limit / 2).max(1);
        self.success_streak = 0;
        self.previous_efficiency = None;
    }
}

#[derive(Debug)]
pub(crate) enum RepoTaskOutcome {
    Success {
        revision: Option<String>,
    },
    Failure {
        revision: Option<String>,
        error: RepoIntelligenceError,
    },
    Requeued {
        task: RepoIndexTask,
        error: RepoIntelligenceError,
    },
    Skipped,
}

#[derive(Debug)]
pub(crate) struct RepoTaskFeedback {
    pub(crate) repo_id: String,
    pub(crate) elapsed: Duration,
    pub(crate) outcome: RepoTaskOutcome,
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_REPO_INDEX_SYNC_CONCURRENCY, repo_index_sync_concurrency_limit_with_lookup,
        should_retry_sync_failure,
    };
    use crate::analyzers::RepoIntelligenceError;

    #[test]
    fn repo_index_sync_concurrency_limit_defaults_when_env_is_missing() {
        let limit = repo_index_sync_concurrency_limit_with_lookup(&|_| None);
        assert_eq!(limit, DEFAULT_REPO_INDEX_SYNC_CONCURRENCY);
    }

    #[test]
    fn repo_index_sync_concurrency_limit_uses_positive_override() {
        let limit = repo_index_sync_concurrency_limit_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_CONCURRENCY").then(|| "3".to_string())
        });
        assert_eq!(limit, 3);
    }

    #[test]
    fn repo_index_sync_concurrency_limit_ignores_invalid_override() {
        let limit = repo_index_sync_concurrency_limit_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_CONCURRENCY").then(|| "invalid".to_string())
        });
        assert_eq!(limit, DEFAULT_REPO_INDEX_SYNC_CONCURRENCY);
    }

    #[test]
    fn retryable_sync_failure_matches_transient_network_transport_errors() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message: "failed to refresh managed mirror `DifferentialEquations.jl` from `https://github.com/SciML/DifferentialEquations.jl.git`: failed to connect to github.com: Can't assign requested address; class=Os (2)".to_string(),
        };
        assert!(should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn retryable_sync_failure_stops_after_retry_budget_is_exhausted() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message:
                "failed to refresh managed mirror `DifferentialEquations.jl`: operation timed out"
                    .to_string(),
        };
        assert!(!should_retry_sync_failure(&error, 1));
    }

    #[test]
    fn retryable_sync_failure_rejects_non_transport_errors() {
        let error = RepoIntelligenceError::MissingRepositorySource {
            repo_id: "DifferentialEquations.jl".to_string(),
        };
        assert!(!should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn retryable_sync_failure_matches_descriptor_pressure_errors() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message: "failed to acquire managed checkout lock `/tmp/example.lock`: Too many open files (os error 24)".to_string(),
        };
        assert!(should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn retryable_sync_failure_matches_retryable_invalid_repository_path_reasons() {
        let error = RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: "DifferentialEquations.jl".to_string(),
            path: "/tmp/example.git".to_string(),
            reason: "failed to open managed mirror as bare git repository: could not open '/tmp/example.git/config': Too many open files; class=Os (2)".to_string(),
        };
        assert!(should_retry_sync_failure(&error, 0));
    }
}
