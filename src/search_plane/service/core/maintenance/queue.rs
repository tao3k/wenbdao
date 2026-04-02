use tokio::sync::oneshot;

use super::helpers::{COMPACTION_STARVATION_GUARD_ENQUEUE_LAG, REPO_MAINTENANCE_SHUTDOWN_MESSAGE};
use crate::search_plane::coordinator::SearchCompactionReason;
use crate::search_plane::service::core::types::{
    QueuedRepoMaintenanceTask, RepoMaintenanceRuntime, RepoMaintenanceTask,
    RepoMaintenanceTaskResult, SearchPlaneService,
};

impl SearchPlaneService {
    pub(crate) fn register_repo_maintenance_task(
        &self,
        task: RepoMaintenanceTask,
        wait_for_result: bool,
    ) -> (
        Option<oneshot::Receiver<RepoMaintenanceTaskResult>>,
        bool,
        bool,
    ) {
        let task_key = task.task_key();
        let mut runtime = self
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if runtime.shutdown_requested {
            if wait_for_result {
                let (sender, receiver) = oneshot::channel();
                let _ = sender.send(Err(REPO_MAINTENANCE_SHUTDOWN_MESSAGE.to_string()));
                return (Some(receiver), false, false);
            }
            return (None, false, false);
        }
        let receiver = if wait_for_result {
            let (sender, receiver) = oneshot::channel();
            runtime
                .waiters
                .entry(task_key.clone())
                .or_default()
                .push(sender);
            Some(receiver)
        } else {
            None
        };
        if !runtime.in_flight.insert(task_key) {
            return (receiver, false, false);
        }
        Self::enqueue_repo_maintenance_task(&mut runtime, task);
        let start_worker = if runtime.worker_running {
            false
        } else {
            runtime.worker_running = true;
            true
        };
        (receiver, true, start_worker)
    }

    fn enqueue_repo_maintenance_task(
        runtime: &mut RepoMaintenanceRuntime,
        task: RepoMaintenanceTask,
    ) {
        let enqueue_sequence = runtime.next_enqueue_sequence;
        runtime.next_enqueue_sequence = runtime.next_enqueue_sequence.saturating_add(1);
        match task {
            RepoMaintenanceTask::Prewarm(_) => {
                let insert_at = runtime
                    .queue
                    .iter()
                    .position(|queued| matches!(queued.task, RepoMaintenanceTask::Compaction(_)))
                    .unwrap_or(runtime.queue.len());
                runtime.queue.insert(
                    insert_at,
                    QueuedRepoMaintenanceTask {
                        task,
                        enqueue_sequence,
                    },
                );
            }
            RepoMaintenanceTask::Compaction(compaction) => {
                let stale_compactions = runtime
                    .queue
                    .iter()
                    .filter_map(|queued| match &queued.task {
                        RepoMaintenanceTask::Compaction(queued_task)
                            if queued_task.corpus == compaction.corpus
                                && queued_task.repo_id == compaction.repo_id =>
                        {
                            Some(queued_task.task_key())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                runtime.queue.retain(|queued| {
                    !matches!(
                        &queued.task,
                        RepoMaintenanceTask::Compaction(queued_task)
                            if queued_task.corpus == compaction.corpus
                                && queued_task.repo_id == compaction.repo_id
                    )
                });
                for stale_key in stale_compactions {
                    runtime.in_flight.remove(&stale_key);
                    runtime.waiters.remove(&stale_key);
                }
                let insert_at = runtime
                    .queue
                    .iter()
                    .position(|queued| match &queued.task {
                        RepoMaintenanceTask::Compaction(queued_task) => {
                            Self::repo_compaction_priority(
                                compaction.reason,
                                compaction.row_count,
                                enqueue_sequence,
                                enqueue_sequence,
                            ) < Self::repo_compaction_priority(
                                queued_task.reason,
                                queued_task.row_count,
                                queued.enqueue_sequence,
                                enqueue_sequence,
                            )
                        }
                        RepoMaintenanceTask::Prewarm(_) => false,
                    })
                    .unwrap_or(runtime.queue.len());
                runtime.queue.insert(
                    insert_at,
                    QueuedRepoMaintenanceTask {
                        task: RepoMaintenanceTask::Compaction(compaction),
                        enqueue_sequence,
                    },
                );
            }
        }
    }

    const fn repo_compaction_priority(
        reason: SearchCompactionReason,
        row_count: u64,
        enqueue_sequence: u64,
        current_sequence: u64,
    ) -> (u8, u64, u64) {
        let age = current_sequence.saturating_sub(enqueue_sequence);
        let reason_rank = match reason {
            SearchCompactionReason::RowDeltaRatio
                if age >= COMPACTION_STARVATION_GUARD_ENQUEUE_LAG =>
            {
                0
            }
            SearchCompactionReason::PublishThreshold => 1,
            SearchCompactionReason::RowDeltaRatio => 2,
        };
        (reason_rank, row_count, enqueue_sequence)
    }
}
