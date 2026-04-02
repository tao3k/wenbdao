use super::definitions::*;

impl From<crate::search_plane::SearchPlanePhase> for SearchIndexPhase {
    fn from(value: crate::search_plane::SearchPlanePhase) -> Self {
        match value {
            crate::search_plane::SearchPlanePhase::Idle => Self::Idle,
            crate::search_plane::SearchPlanePhase::Indexing => Self::Indexing,
            crate::search_plane::SearchPlanePhase::Ready => Self::Ready,
            crate::search_plane::SearchPlanePhase::Degraded => Self::Degraded,
            crate::search_plane::SearchPlanePhase::Failed => Self::Failed,
        }
    }
}

impl From<crate::search_plane::SearchCorpusIssueCode> for SearchIndexIssueCode {
    fn from(value: crate::search_plane::SearchCorpusIssueCode) -> Self {
        match value {
            crate::search_plane::SearchCorpusIssueCode::PublishedManifestMissing => {
                Self::PublishedManifestMissing
            }
            crate::search_plane::SearchCorpusIssueCode::PublishedRevisionMissing => {
                Self::PublishedRevisionMissing
            }
            crate::search_plane::SearchCorpusIssueCode::PublishedRevisionMismatch => {
                Self::PublishedRevisionMismatch
            }
            crate::search_plane::SearchCorpusIssueCode::RepoIndexFailed => Self::RepoIndexFailed,
        }
    }
}

impl From<crate::search_plane::SearchCorpusIssueFamily> for SearchIndexIssueFamily {
    fn from(value: crate::search_plane::SearchCorpusIssueFamily) -> Self {
        match value {
            crate::search_plane::SearchCorpusIssueFamily::Manifest => Self::Manifest,
            crate::search_plane::SearchCorpusIssueFamily::Revision => Self::Revision,
            crate::search_plane::SearchCorpusIssueFamily::RepoSync => Self::RepoSync,
            crate::search_plane::SearchCorpusIssueFamily::Mixed => Self::Mixed,
        }
    }
}

impl From<&crate::search_plane::SearchCorpusIssue> for SearchIndexIssue {
    fn from(value: &crate::search_plane::SearchCorpusIssue) -> Self {
        Self {
            code: value.code.into(),
            readable: value.readable,
            repo_id: value.repo_id.clone(),
            current_revision: value.current_revision.clone(),
            published_revision: value.published_revision.clone(),
            message: value.message.clone(),
        }
    }
}

impl From<&crate::search_plane::SearchCorpusIssueSummary> for SearchIndexIssueSummary {
    fn from(value: &crate::search_plane::SearchCorpusIssueSummary) -> Self {
        Self {
            family: value.family.into(),
            primary_code: value.primary_code.into(),
            issue_count: value.issue_count,
            readable_issue_count: value.readable_issue_count,
        }
    }
}

impl From<crate::search_plane::SearchCorpusStatusSeverity> for SearchIndexStatusSeverity {
    fn from(value: crate::search_plane::SearchCorpusStatusSeverity) -> Self {
        match value {
            crate::search_plane::SearchCorpusStatusSeverity::Info => Self::Info,
            crate::search_plane::SearchCorpusStatusSeverity::Warning => Self::Warning,
            crate::search_plane::SearchCorpusStatusSeverity::Error => Self::Error,
        }
    }
}

impl From<crate::search_plane::SearchCorpusStatusAction> for SearchIndexStatusAction {
    fn from(value: crate::search_plane::SearchCorpusStatusAction) -> Self {
        match value {
            crate::search_plane::SearchCorpusStatusAction::Wait => Self::Wait,
            crate::search_plane::SearchCorpusStatusAction::RetryBuild => Self::RetryBuild,
            crate::search_plane::SearchCorpusStatusAction::ResyncRepo => Self::ResyncRepo,
            crate::search_plane::SearchCorpusStatusAction::InspectRepoSync => Self::InspectRepoSync,
        }
    }
}

impl From<crate::search_plane::SearchCorpusStatusReasonCode> for SearchIndexStatusReasonCode {
    fn from(value: crate::search_plane::SearchCorpusStatusReasonCode) -> Self {
        match value {
            crate::search_plane::SearchCorpusStatusReasonCode::WarmingUp => Self::WarmingUp,
            crate::search_plane::SearchCorpusStatusReasonCode::Prewarming => Self::Prewarming,
            crate::search_plane::SearchCorpusStatusReasonCode::Refreshing => Self::Refreshing,
            crate::search_plane::SearchCorpusStatusReasonCode::Compacting => Self::Compacting,
            crate::search_plane::SearchCorpusStatusReasonCode::CompactionPending => {
                Self::CompactionPending
            }
            crate::search_plane::SearchCorpusStatusReasonCode::BuildFailed => Self::BuildFailed,
            crate::search_plane::SearchCorpusStatusReasonCode::PublishedManifestMissing => {
                Self::PublishedManifestMissing
            }
            crate::search_plane::SearchCorpusStatusReasonCode::PublishedRevisionMissing => {
                Self::PublishedRevisionMissing
            }
            crate::search_plane::SearchCorpusStatusReasonCode::PublishedRevisionMismatch => {
                Self::PublishedRevisionMismatch
            }
            crate::search_plane::SearchCorpusStatusReasonCode::RepoIndexFailed => {
                Self::RepoIndexFailed
            }
        }
    }
}

impl From<&crate::search_plane::SearchCorpusStatusReason> for SearchIndexStatusReason {
    fn from(value: &crate::search_plane::SearchCorpusStatusReason) -> Self {
        Self {
            code: value.code.into(),
            severity: value.severity.into(),
            action: value.action.into(),
            readable: value.readable,
        }
    }
}

impl From<&crate::search_plane::SearchMaintenanceStatus> for SearchIndexMaintenanceStatus {
    fn from(value: &crate::search_plane::SearchMaintenanceStatus) -> Self {
        Self {
            prewarm_running: value.prewarm_running,
            prewarm_queue_depth: value.prewarm_queue_depth,
            prewarm_queue_position: value.prewarm_queue_position,
            compaction_running: value.compaction_running,
            compaction_queue_depth: value.compaction_queue_depth,
            compaction_queue_position: value.compaction_queue_position,
            compaction_queue_aged: value.compaction_queue_aged,
            compaction_pending: value.compaction_pending,
            publish_count_since_compaction: value.publish_count_since_compaction,
            last_prewarmed_at: value.last_prewarmed_at.clone(),
            last_prewarmed_epoch: value.last_prewarmed_epoch,
            last_compacted_at: value.last_compacted_at.clone(),
            last_compaction_reason: value.last_compaction_reason.clone(),
            last_compacted_row_count: value.last_compacted_row_count,
        }
    }
}

impl From<&crate::search_plane::SearchRepoReadPressure> for SearchIndexRepoReadPressure {
    fn from(value: &crate::search_plane::SearchRepoReadPressure) -> Self {
        Self {
            budget: value.budget,
            in_flight: value.in_flight,
            captured_at: value.captured_at.clone(),
            requested_repo_count: value.requested_repo_count,
            searchable_repo_count: value.searchable_repo_count,
            parallelism: value.parallelism,
            fanout_capped: value.fanout_capped,
        }
    }
}

impl From<crate::search_plane::SearchQueryTelemetrySource> for SearchIndexQueryTelemetrySource {
    fn from(value: crate::search_plane::SearchQueryTelemetrySource) -> Self {
        match value {
            crate::search_plane::SearchQueryTelemetrySource::Scan => Self::Scan,
            crate::search_plane::SearchQueryTelemetrySource::Fts => Self::Fts,
            crate::search_plane::SearchQueryTelemetrySource::FtsFallbackScan => {
                Self::FtsFallbackScan
            }
        }
    }
}

impl From<&crate::search_plane::SearchQueryTelemetry> for SearchIndexQueryTelemetry {
    fn from(value: &crate::search_plane::SearchQueryTelemetry) -> Self {
        Self {
            captured_at: value.captured_at.clone(),
            scope: value.scope.clone(),
            source: value.source.into(),
            batch_count: value.batch_count,
            rows_scanned: value.rows_scanned,
            matched_rows: value.matched_rows,
            result_count: value.result_count,
            batch_row_limit: value.batch_row_limit,
            recall_limit_rows: value.recall_limit_rows,
            working_set_budget_rows: value.working_set_budget_rows,
            trim_threshold_rows: value.trim_threshold_rows,
            peak_working_set_rows: value.peak_working_set_rows,
            trim_count: value.trim_count,
            dropped_candidate_count: value.dropped_candidate_count,
        }
    }
}

impl From<&crate::search_plane::SearchCorpusStatus> for SearchCorpusIndexStatus {
    fn from(value: &crate::search_plane::SearchCorpusStatus) -> Self {
        Self {
            corpus: value.corpus.to_string(),
            phase: value.phase.into(),
            active_epoch: value.active_epoch,
            staging_epoch: value.staging_epoch,
            schema_version: value.schema_version,
            fingerprint: value.fingerprint.clone(),
            progress: value.progress,
            row_count: value.row_count,
            fragment_count: value.fragment_count,
            build_started_at: value.build_started_at.clone(),
            build_finished_at: value.build_finished_at.clone(),
            updated_at: value.updated_at.clone(),
            last_error: value.last_error.clone(),
            issues: value.issues.iter().map(SearchIndexIssue::from).collect(),
            issue_summary: value
                .issue_summary
                .as_ref()
                .map(SearchIndexIssueSummary::from),
            status_reason: value
                .status_reason
                .as_ref()
                .map(SearchIndexStatusReason::from),
            last_query_telemetry: value
                .last_query_telemetry
                .as_ref()
                .map(SearchIndexQueryTelemetry::from),
            maintenance: SearchIndexMaintenanceStatus::from(&value.maintenance),
        }
    }
}
