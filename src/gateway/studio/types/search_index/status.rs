use std::collections::BTreeMap;

use super::definitions::*;

impl From<&crate::search_plane::SearchPlaneStatusSnapshot> for SearchIndexStatusResponse {
    fn from(value: &crate::search_plane::SearchPlaneStatusSnapshot) -> Self {
        let corpora = value
            .corpora
            .iter()
            .map(SearchCorpusIndexStatus::from)
            .collect::<Vec<_>>();
        let total = corpora.len();
        let idle = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Idle))
            .count();
        let indexing = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Indexing))
            .count();
        let ready = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Ready))
            .count();
        let failed = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Failed))
            .count();
        let degraded = corpora
            .iter()
            .filter(|status| matches!(status.phase, SearchIndexPhase::Degraded))
            .count();
        let compaction_pending = corpora
            .iter()
            .filter(|status| status.maintenance.compaction_pending)
            .count();
        let status_reason = summarize_response_status_reason(&corpora);
        let maintenance_summary = summarize_response_maintenance(&corpora);
        let query_telemetry_summary = summarize_response_query_telemetry(&corpora);
        let repo_read_pressure = value
            .repo_read_pressure
            .as_ref()
            .map(SearchIndexRepoReadPressure::from);
        Self {
            total,
            idle,
            indexing,
            ready,
            degraded,
            failed,
            compaction_pending,
            status_reason,
            maintenance_summary,
            query_telemetry_summary,
            repo_read_pressure,
            corpora,
        }
    }
}

fn summarize_response_status_reason(
    corpora: &[SearchCorpusIndexStatus],
) -> Option<SearchIndexAggregateStatusReason> {
    let reasons = corpora
        .iter()
        .filter_map(|status| status.status_reason.as_ref())
        .collect::<Vec<_>>();
    let primary = reasons.into_iter().min_by_key(|reason| {
        (
            response_reason_severity_priority(reason.severity),
            response_reason_code_priority(reason.code),
        )
    })?;
    let affected_corpus_count = corpora
        .iter()
        .filter(|status| status.status_reason.is_some())
        .count();
    let readable_corpus_count = corpora
        .iter()
        .filter_map(|status| status.status_reason.as_ref())
        .filter(|reason| reason.readable)
        .count();
    let blocking_corpus_count = affected_corpus_count.saturating_sub(readable_corpus_count);
    Some(SearchIndexAggregateStatusReason {
        code: primary.code,
        severity: primary.severity,
        action: primary.action,
        affected_corpus_count,
        readable_corpus_count,
        blocking_corpus_count,
    })
}

fn summarize_response_query_telemetry(
    corpora: &[SearchCorpusIndexStatus],
) -> Option<SearchIndexAggregateQueryTelemetry> {
    let telemetry = corpora
        .iter()
        .filter_map(|status| status.last_query_telemetry.as_ref())
        .collect::<Vec<_>>();
    if telemetry.is_empty() {
        return None;
    }

    let mut summary = QueryTelemetryAccumulator::default();
    let mut scopes = BTreeMap::<String, QueryTelemetryAccumulator>::new();

    for entry in telemetry {
        summary.observe(entry);
        if let Some(scope) = entry.scope.as_deref().filter(|scope| !scope.is_empty()) {
            scopes.entry(scope.to_string()).or_default().observe(entry);
        }
    }

    Some(
        summary.into_aggregate(
            scopes
                .into_iter()
                .map(|(scope, bucket)| bucket.into_scope_summary(scope))
                .collect(),
        ),
    )
}

fn summarize_response_maintenance(
    corpora: &[SearchCorpusIndexStatus],
) -> Option<SearchIndexAggregateMaintenanceSummary> {
    let prewarm_running_count = corpora
        .iter()
        .filter(|status| status.maintenance.prewarm_running)
        .count();
    let prewarm_queued_corpus_count = corpora
        .iter()
        .filter(|status| status.maintenance.prewarm_queue_depth > 0)
        .count();
    let max_prewarm_queue_depth = corpora
        .iter()
        .map(|status| status.maintenance.prewarm_queue_depth)
        .max()
        .unwrap_or_default();
    let compaction_running_count = corpora
        .iter()
        .filter(|status| status.maintenance.compaction_running)
        .count();
    let compaction_queued_corpus_count = corpora
        .iter()
        .filter(|status| status.maintenance.compaction_queue_depth > 0)
        .count();
    let max_compaction_queue_depth = corpora
        .iter()
        .map(|status| status.maintenance.compaction_queue_depth)
        .max()
        .unwrap_or_default();
    let compaction_pending_count = corpora
        .iter()
        .filter(|status| status.maintenance.compaction_pending)
        .count();
    let aged_compaction_queue_count = corpora
        .iter()
        .filter(|status| status.maintenance.compaction_queue_aged)
        .count();

    if prewarm_running_count == 0
        && prewarm_queued_corpus_count == 0
        && compaction_running_count == 0
        && compaction_queued_corpus_count == 0
        && compaction_pending_count == 0
        && aged_compaction_queue_count == 0
    {
        return None;
    }

    Some(SearchIndexAggregateMaintenanceSummary {
        prewarm_running_count,
        prewarm_queued_corpus_count,
        max_prewarm_queue_depth,
        compaction_running_count,
        compaction_queued_corpus_count,
        max_compaction_queue_depth,
        compaction_pending_count,
        aged_compaction_queue_count,
    })
}

fn max_optional_u64(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn response_reason_severity_priority(severity: SearchIndexStatusSeverity) -> u8 {
    match severity {
        SearchIndexStatusSeverity::Error => 0,
        SearchIndexStatusSeverity::Warning => 1,
        SearchIndexStatusSeverity::Info => 2,
    }
}

fn response_reason_code_priority(code: SearchIndexStatusReasonCode) -> u8 {
    match code {
        SearchIndexStatusReasonCode::PublishedManifestMissing => 0,
        SearchIndexStatusReasonCode::BuildFailed => 1,
        SearchIndexStatusReasonCode::PublishedRevisionMissing => 2,
        SearchIndexStatusReasonCode::PublishedRevisionMismatch => 3,
        SearchIndexStatusReasonCode::RepoIndexFailed => 4,
        SearchIndexStatusReasonCode::WarmingUp => 5,
        SearchIndexStatusReasonCode::Prewarming => 6,
        SearchIndexStatusReasonCode::Refreshing => 7,
        SearchIndexStatusReasonCode::Compacting => 8,
        SearchIndexStatusReasonCode::CompactionPending => 9,
    }
}

#[derive(Debug, Default)]
struct QueryTelemetryAccumulator {
    corpus_count: usize,
    latest_captured_at: String,
    scan_count: usize,
    fts_count: usize,
    fts_fallback_scan_count: usize,
    total_rows_scanned: u64,
    total_matched_rows: u64,
    total_result_count: u64,
    max_batch_row_limit: Option<u64>,
    max_recall_limit_rows: Option<u64>,
    max_working_set_budget_rows: u64,
    max_trim_threshold_rows: u64,
    max_peak_working_set_rows: u64,
    total_trim_count: u64,
    total_dropped_candidate_count: u64,
}

impl QueryTelemetryAccumulator {
    fn observe(&mut self, entry: &SearchIndexQueryTelemetry) {
        self.corpus_count = self.corpus_count.saturating_add(1);
        if self.latest_captured_at.as_str() < entry.captured_at.as_str() {
            self.latest_captured_at = entry.captured_at.clone();
        }
        match entry.source {
            SearchIndexQueryTelemetrySource::Scan => {
                self.scan_count = self.scan_count.saturating_add(1);
            }
            SearchIndexQueryTelemetrySource::Fts => {
                self.fts_count = self.fts_count.saturating_add(1);
            }
            SearchIndexQueryTelemetrySource::FtsFallbackScan => {
                self.fts_fallback_scan_count = self.fts_fallback_scan_count.saturating_add(1);
            }
        }
        self.total_rows_scanned = self.total_rows_scanned.saturating_add(entry.rows_scanned);
        self.total_matched_rows = self.total_matched_rows.saturating_add(entry.matched_rows);
        self.total_result_count = self.total_result_count.saturating_add(entry.result_count);
        self.max_batch_row_limit =
            max_optional_u64(self.max_batch_row_limit, entry.batch_row_limit);
        self.max_recall_limit_rows =
            max_optional_u64(self.max_recall_limit_rows, entry.recall_limit_rows);
        self.max_working_set_budget_rows = self
            .max_working_set_budget_rows
            .max(entry.working_set_budget_rows);
        self.max_trim_threshold_rows = self.max_trim_threshold_rows.max(entry.trim_threshold_rows);
        self.max_peak_working_set_rows = self
            .max_peak_working_set_rows
            .max(entry.peak_working_set_rows);
        self.total_trim_count = self.total_trim_count.saturating_add(entry.trim_count);
        self.total_dropped_candidate_count = self
            .total_dropped_candidate_count
            .saturating_add(entry.dropped_candidate_count);
    }

    fn into_aggregate(
        self,
        scopes: Vec<SearchIndexQueryTelemetryScopeSummary>,
    ) -> SearchIndexAggregateQueryTelemetry {
        SearchIndexAggregateQueryTelemetry {
            corpus_count: self.corpus_count,
            latest_captured_at: self.latest_captured_at,
            scan_count: self.scan_count,
            fts_count: self.fts_count,
            fts_fallback_scan_count: self.fts_fallback_scan_count,
            total_rows_scanned: self.total_rows_scanned,
            total_matched_rows: self.total_matched_rows,
            total_result_count: self.total_result_count,
            max_batch_row_limit: self.max_batch_row_limit,
            max_recall_limit_rows: self.max_recall_limit_rows,
            max_working_set_budget_rows: self.max_working_set_budget_rows,
            max_trim_threshold_rows: self.max_trim_threshold_rows,
            max_peak_working_set_rows: self.max_peak_working_set_rows,
            total_trim_count: self.total_trim_count,
            total_dropped_candidate_count: self.total_dropped_candidate_count,
            scopes,
        }
    }

    fn into_scope_summary(self, scope: String) -> SearchIndexQueryTelemetryScopeSummary {
        SearchIndexQueryTelemetryScopeSummary {
            scope,
            corpus_count: self.corpus_count,
            latest_captured_at: self.latest_captured_at,
            scan_count: self.scan_count,
            fts_count: self.fts_count,
            fts_fallback_scan_count: self.fts_fallback_scan_count,
            total_rows_scanned: self.total_rows_scanned,
            total_matched_rows: self.total_matched_rows,
            total_result_count: self.total_result_count,
            max_batch_row_limit: self.max_batch_row_limit,
            max_recall_limit_rows: self.max_recall_limit_rows,
            max_working_set_budget_rows: self.max_working_set_budget_rows,
            max_trim_threshold_rows: self.max_trim_threshold_rows,
            max_peak_working_set_rows: self.max_peak_working_set_rows,
            total_trim_count: self.total_trim_count,
            total_dropped_candidate_count: self.total_dropped_candidate_count,
        }
    }
}
