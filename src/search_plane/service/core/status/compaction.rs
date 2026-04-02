use crate::search_plane::coordinator::{SearchCompactionReason, SearchCompactionTask};
use crate::search_plane::service::core::types::{
    LocalMaintenanceRuntime, QueuedLocalCompactionTask, SearchPlaneService,
};
use crate::search_plane::{SearchBuildLease, SearchCorpusKind};

pub(super) const COMPACTION_STARVATION_GUARD_ENQUEUE_LAG: u64 = 3;

impl SearchPlaneService {
    pub(crate) fn publish_ready_and_maintain(
        &self,
        lease: &SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        if !self
            .coordinator
            .publish_ready(lease, row_count, fragment_count)
        {
            return false;
        }
        self.schedule_pending_compaction(lease.corpus);
        true
    }

    fn schedule_pending_compaction(&self, corpus: SearchCorpusKind) {
        let Some(task) = self.coordinator.pending_compaction_task(corpus) else {
            return;
        };
        let start_worker = {
            let mut runtime = self
                .local_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if runtime.shutdown_requested || runtime.active_compaction == Some(corpus) {
                return;
            }
            Self::enqueue_local_compaction_task(&mut runtime, task);
            if runtime.worker_running {
                false
            } else {
                runtime.worker_running = true;
                true
            }
        };
        self.ensure_local_compaction_worker(start_worker);
    }

    pub(super) fn enqueue_local_compaction_task(
        runtime: &mut LocalMaintenanceRuntime,
        task: SearchCompactionTask,
    ) {
        let enqueue_sequence = runtime.next_enqueue_sequence;
        runtime.next_enqueue_sequence = runtime.next_enqueue_sequence.saturating_add(1);
        runtime
            .compaction_queue
            .retain(|queued| queued.task.corpus != task.corpus);
        runtime.running_compactions.insert(task.corpus);
        let queued_task = QueuedLocalCompactionTask {
            task,
            enqueue_sequence,
        };
        let insert_at = runtime
            .compaction_queue
            .iter()
            .position(|queued| {
                Self::local_compaction_priority(
                    queued_task.task.reason,
                    queued_task.task.row_count,
                    queued_task.enqueue_sequence,
                    enqueue_sequence,
                ) < Self::local_compaction_priority(
                    queued.task.reason,
                    queued.task.row_count,
                    queued.enqueue_sequence,
                    enqueue_sequence,
                )
            })
            .unwrap_or(runtime.compaction_queue.len());
        runtime.compaction_queue.insert(insert_at, queued_task);
    }

    const fn local_compaction_priority(
        reason: SearchCompactionReason,
        row_count: u64,
        enqueue_sequence: u64,
        current_sequence: u64,
    ) -> (u8, u64, u64) {
        let reason_rank = match reason {
            SearchCompactionReason::RowDeltaRatio
                if Self::local_compaction_is_aged(reason, enqueue_sequence, current_sequence) =>
            {
                0
            }
            SearchCompactionReason::PublishThreshold => 1,
            SearchCompactionReason::RowDeltaRatio => 2,
        };
        (reason_rank, row_count, enqueue_sequence)
    }

    pub(super) const fn local_compaction_is_aged(
        reason: SearchCompactionReason,
        enqueue_sequence: u64,
        current_sequence: u64,
    ) -> bool {
        matches!(reason, SearchCompactionReason::RowDeltaRatio)
            && current_sequence.saturating_sub(enqueue_sequence)
                >= COMPACTION_STARVATION_GUARD_ENQUEUE_LAG
    }

    fn ensure_local_compaction_worker(&self, start_worker: bool) {
        if !start_worker {
            return;
        }
        if self.local_maintenance_shutdown_requested() {
            let mut runtime = self
                .local_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            runtime.worker_running = false;
            return;
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let service = self.clone();
            let worker_handle = handle.spawn(async move {
                service.run_local_compaction_worker().await;
            });
            let mut runtime = self
                .local_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if runtime.shutdown_requested {
                runtime.worker_running = false;
                runtime.worker_handle = None;
                runtime.compaction_queue.clear();
                runtime.running_compactions.clear();
                runtime.active_compaction = None;
                drop(runtime);
                worker_handle.abort();
                return;
            }
            runtime.worker_handle = Some(worker_handle);
        } else {
            let mut runtime = self
                .local_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            runtime.worker_running = false;
            runtime.compaction_queue.clear();
            runtime.running_compactions.clear();
            runtime.active_compaction = None;
            runtime.worker_handle = None;
        }
    }

    async fn run_local_compaction_worker(&self) {
        loop {
            let task = {
                let mut runtime = self
                    .local_maintenance
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if runtime.shutdown_requested {
                    runtime.active_compaction = None;
                    runtime.worker_running = false;
                    runtime.worker_handle = None;
                    break;
                }
                match runtime.compaction_queue.pop_front() {
                    Some(queued) => {
                        runtime.active_compaction = Some(queued.task.corpus);
                        queued.task
                    }
                    None => {
                        runtime.active_compaction = None;
                        runtime.worker_running = false;
                        runtime.worker_handle = None;
                        break;
                    }
                }
            };
            let corpus = task.corpus;
            self.run_compaction_task(task).await;
            self.finish_local_compaction(corpus);
        }
    }

    fn finish_local_compaction(&self, corpus: SearchCorpusKind) {
        let mut runtime = self
            .local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        runtime.running_compactions.remove(&corpus);
        if runtime.active_compaction == Some(corpus) {
            runtime.active_compaction = None;
        }
    }

    async fn run_compaction_task(&self, task: SearchCompactionTask) {
        if self.local_maintenance_shutdown_requested() {
            return;
        }
        let table_names = self.local_epoch_table_names_for_reads(task.corpus, task.active_epoch);
        let store = match self.open_store(task.corpus).await {
            Ok(store) => store,
            Err(error) => {
                log::warn!(
                    "search-plane compaction failed to open store for {} epoch {}: {}",
                    task.corpus,
                    task.active_epoch,
                    error
                );
                return;
            }
        };
        let mut fragment_count = 0_u64;
        for table_name in table_names {
            match store.compact(table_name.as_str()).await {
                Ok(_) => match store.get_table_info(table_name.as_str()).await {
                    Ok(table_info) => {
                        fragment_count = fragment_count.saturating_add(
                            u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
                        );
                    }
                    Err(error) => {
                        log::warn!(
                            "search-plane compaction failed to inspect {} epoch {} table {} after compact: {}",
                            task.corpus,
                            task.active_epoch,
                            table_name,
                            error
                        );
                        return;
                    }
                },
                Err(error) => {
                    log::warn!(
                        "search-plane compaction failed for {} epoch {} table {}: {}",
                        task.corpus,
                        task.active_epoch,
                        table_name,
                        error
                    );
                    return;
                }
            }
        }
        let _ = self.coordinator.mark_compaction_complete(
            task.corpus,
            task.active_epoch,
            task.row_count,
            fragment_count,
            task.reason,
        );
    }
}
