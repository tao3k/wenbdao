use crate::search_plane::{SearchCorpusKind, SearchPlaneCoordinator, SearchPlanePhase};

use super::state::timestamp_now;
use super::types::{SearchCompactionReason, SearchCompactionTask};

impl SearchPlaneCoordinator {
    /// Record that compaction completed for the currently active epoch.
    pub(crate) fn mark_compaction_complete(
        &self,
        corpus: SearchCorpusKind,
        active_epoch: u64,
        row_count: u64,
        fragment_count: u64,
        reason: SearchCompactionReason,
    ) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&corpus) else {
            return false;
        };
        if runtime.status.active_epoch != Some(active_epoch) {
            return false;
        }

        runtime.last_compacted_row_count = Some(row_count);
        runtime.status.fragment_count = Some(fragment_count);
        runtime.status.maintenance.compaction_pending = false;
        runtime.status.maintenance.publish_count_since_compaction = 0;
        runtime.status.maintenance.last_compacted_at = Some(timestamp_now());
        runtime.status.maintenance.last_compaction_reason = Some(reason.as_str().to_string());
        runtime.status.updated_at = runtime.status.maintenance.last_compacted_at.clone();
        true
    }

    /// Record that a staging or active epoch was successfully prewarmed.
    pub(crate) fn mark_prewarm_complete(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(epoch) && runtime.status.active_epoch != Some(epoch)
        {
            return false;
        }

        let now = timestamp_now();
        runtime.status.maintenance.prewarm_running = false;
        runtime.status.maintenance.last_prewarmed_at = Some(now.clone());
        runtime.status.maintenance.last_prewarmed_epoch = Some(epoch);
        runtime.status.updated_at = Some(now);
        true
    }

    /// Record that a staging or active epoch prewarm has started.
    pub(crate) fn mark_prewarm_running(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        self.set_prewarm_running(corpus, epoch, true)
    }

    /// Record that a staging or active epoch prewarm is no longer running.
    pub(crate) fn clear_prewarm_running(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        self.set_prewarm_running(corpus, epoch, false)
    }

    /// Return the current compaction task for a ready corpus, if maintenance is pending.
    #[must_use]
    pub(crate) fn pending_compaction_task(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchCompactionTask> {
        let state = self
            .state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let runtime = state.get(&corpus)?;
        if !matches!(runtime.status.phase, SearchPlanePhase::Ready)
            || !runtime.status.maintenance.compaction_pending
        {
            return None;
        }
        let active_epoch = runtime.status.active_epoch?;
        let row_count = runtime.status.row_count?;
        let reason = self.maintenance_policy.compaction_reason(
            runtime.status.maintenance.publish_count_since_compaction,
            runtime.last_compacted_row_count,
            row_count,
        )?;
        Some(SearchCompactionTask {
            corpus,
            active_epoch,
            row_count,
            reason,
        })
    }

    fn set_prewarm_running(&self, corpus: SearchCorpusKind, epoch: u64, running: bool) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(epoch) && runtime.status.active_epoch != Some(epoch)
        {
            return false;
        }

        runtime.status.maintenance.prewarm_running = running;
        runtime.status.updated_at = Some(timestamp_now());
        true
    }
}
