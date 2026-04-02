use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::task::JoinSet;

use crate::gateway::studio::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::gateway::studio::repo_index::state::task::{RepoTaskFeedback, RepoTaskOutcome};
use crate::gateway::studio::repo_index::types::RepoIndexPhase;

impl RepoIndexCoordinator {
    pub(crate) async fn run(self: Arc<Self>) {
        let mut running = JoinSet::new();
        loop {
            if self.shutdown_requested.load(Ordering::SeqCst) {
                break;
            }

            self.dispatch_pending_tasks(&mut running);

            if self.shutdown_requested.load(Ordering::SeqCst) {
                break;
            }

            if running.is_empty() {
                self.notify.notified().await;
                continue;
            }

            tokio::select! {
                biased;
                Some(result) = running.join_next() => {
                    self.handle_task_result(result);
                }
                () = self.notify.notified() => {}
            }
        }

        running.abort_all();
        while let Some(_result) = running.join_next().await {}
    }

    fn dispatch_pending_tasks(self: &Arc<Self>, running: &mut JoinSet<RepoTaskFeedback>) {
        loop {
            let target = self.target_parallelism(running.len());
            if running.len() >= target {
                break;
            }

            let Some(task) = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pop_front()
            else {
                break;
            };

            self.mark_active(task.repository.id.as_str());
            let coordinator = Arc::clone(self);
            running.spawn(async move { coordinator.process_task(task).await });
        }
    }

    fn target_parallelism(&self, active_count: usize) -> usize {
        let queued = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        let mut controller = self
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        controller.target_limit(queued, active_count)
    }

    fn handle_task_result(&self, result: Result<RepoTaskFeedback, tokio::task::JoinError>) {
        let feedback = match result {
            Ok(feedback) => feedback,
            Err(error) => {
                let mut controller = self
                    .concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                controller.record_failure();
                self.refresh_status_snapshot();
                if error.is_panic() {
                    self.notify.notify_one();
                }
                return;
            }
        };

        match feedback.outcome {
            RepoTaskOutcome::Success { revision } => {
                self.record_repo_status(
                    feedback.repo_id.as_str(),
                    RepoIndexPhase::Ready,
                    revision,
                    None,
                );
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_success(
                        feedback.elapsed,
                        self.pending
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .len(),
                    );
            }
            RepoTaskOutcome::Failure { revision, error } => {
                self.record_failure_status(feedback.repo_id.as_str(), &error, revision);
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_failure();
            }
            RepoTaskOutcome::Requeued { task, error } => {
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_failure();
                self.release_repo(feedback.repo_id.as_str());
                if !self.enqueue_task(task, true) {
                    self.record_failure_status(feedback.repo_id.as_str(), &error, None);
                }
                self.notify.notify_one();
                return;
            }
            RepoTaskOutcome::Skipped => {}
        }
        self.release_repo(feedback.repo_id.as_str());
        self.notify.notify_one();
    }
}
