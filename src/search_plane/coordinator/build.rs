use crate::search_plane::{SearchCorpusKind, SearchPlaneCoordinator, SearchPlanePhase};

use super::state::timestamp_now;
use super::types::{BeginBuildDecision, SearchBuildLease};

impl SearchPlaneCoordinator {
    /// Attempt to start a new staging build for a corpus fingerprint.
    pub fn begin_build(
        &self,
        corpus: SearchCorpusKind,
        fingerprint: impl Into<String>,
        schema_version: u32,
    ) -> BeginBuildDecision {
        let _spawn_guard = self
            .spawn_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fingerprint = fingerprint.into();
        let now = timestamp_now();
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let runtime = state
            .entry(corpus)
            .or_insert_with(|| super::types::SearchCorpusRuntime::new(corpus));
        let schema_matches = runtime.status.schema_version == schema_version;
        if runtime.status.fingerprint.as_deref() == Some(fingerprint.as_str()) && schema_matches {
            if matches!(runtime.status.phase, SearchPlanePhase::Ready)
                && runtime.status.active_epoch.is_some()
            {
                return BeginBuildDecision::AlreadyReady(runtime.status.clone());
            }
            if matches!(runtime.status.phase, SearchPlanePhase::Indexing) {
                return BeginBuildDecision::AlreadyIndexing(runtime.status.clone());
            }
        }

        let epoch = runtime.next_epoch;
        runtime.next_epoch = runtime.next_epoch.saturating_add(1);
        runtime.status.phase = SearchPlanePhase::Indexing;
        runtime.status.staging_epoch = Some(epoch);
        runtime.status.schema_version = schema_version;
        runtime.status.fingerprint = Some(fingerprint.clone());
        runtime.status.progress = Some(0.0);
        runtime.status.build_started_at = Some(now.clone());
        runtime.status.build_finished_at = None;
        runtime.status.updated_at = Some(now);
        runtime.status.last_error = None;

        BeginBuildDecision::Started(SearchBuildLease {
            corpus,
            fingerprint,
            epoch,
            schema_version,
        })
    }

    /// Update build progress for a live lease.
    pub fn update_progress(&self, lease: &SearchBuildLease, progress: f32) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&lease.corpus) else {
            return false;
        };
        if !matches!(runtime.status.phase, SearchPlanePhase::Indexing)
            || runtime.status.staging_epoch != Some(lease.epoch)
            || runtime.status.fingerprint.as_deref() != Some(lease.fingerprint.as_str())
        {
            return false;
        }
        runtime.status.progress = Some(progress.clamp(0.0, 1.0));
        runtime.status.updated_at = Some(timestamp_now());
        true
    }

    /// Publish a completed staging epoch if the lease is still current.
    pub fn publish_ready(
        &self,
        lease: &SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&lease.corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(lease.epoch)
            || runtime.status.fingerprint.as_deref() != Some(lease.fingerprint.as_str())
        {
            return false;
        }

        let now = timestamp_now();
        let publish_count = runtime
            .status
            .maintenance
            .publish_count_since_compaction
            .saturating_add(1);
        runtime.status.phase = SearchPlanePhase::Ready;
        runtime.status.active_epoch = Some(lease.epoch);
        runtime.status.staging_epoch = None;
        runtime.status.schema_version = lease.schema_version;
        runtime.status.progress = None;
        runtime.status.row_count = Some(row_count);
        runtime.status.fragment_count = Some(fragment_count);
        runtime.status.build_finished_at = Some(now.clone());
        runtime.status.updated_at = Some(now);
        runtime.status.last_error = None;
        runtime.status.maintenance.publish_count_since_compaction = publish_count;
        runtime.status.maintenance.compaction_pending = self.maintenance_policy.should_compact(
            publish_count,
            runtime.last_compacted_row_count,
            row_count,
        );
        true
    }

    /// Mark an in-flight build as failed if the lease is still current.
    pub fn fail_build(&self, lease: &SearchBuildLease, error: impl Into<String>) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&lease.corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(lease.epoch)
            || runtime.status.fingerprint.as_deref() != Some(lease.fingerprint.as_str())
        {
            return false;
        }

        let now = timestamp_now();
        runtime.status.phase = SearchPlanePhase::Failed;
        runtime.status.staging_epoch = None;
        runtime.status.progress = None;
        runtime.status.build_finished_at = Some(now.clone());
        runtime.status.updated_at = Some(now);
        runtime.status.last_error = Some(error.into());
        true
    }
}
