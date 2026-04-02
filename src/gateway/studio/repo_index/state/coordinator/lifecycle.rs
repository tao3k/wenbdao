use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::runtime::Handle;
use tokio::sync::OwnedSemaphorePermit;

use crate::analyzers::errors::RepoIntelligenceError;

use crate::gateway::studio::repo_index::state::coordinator::RepoIndexCoordinator;

impl RepoIndexCoordinator {
    pub(crate) fn start(self: &Arc<Self>) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        self.shutdown_requested.store(false, Ordering::SeqCst);
        if let Ok(handle) = Handle::try_current() {
            let coordinator = Arc::clone(self);
            let run_task = handle.spawn(async move {
                coordinator.run().await;
            });
            *self
                .run_task
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(run_task);
        }
    }

    pub(crate) fn stop(&self) {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        if let Some(run_task) = self
            .run_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            run_task.abort();
        }
    }

    pub(crate) async fn acquire_sync_permit(
        &self,
        repo_id: &str,
    ) -> Result<OwnedSemaphorePermit, RepoIntelligenceError> {
        Arc::clone(&self.sync_permits)
            .acquire_owned()
            .await
            .map_err(|_| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id}` sync semaphore was closed while waiting to start remote sync"
                ),
            })
    }
}
