use super::LinkGraphIndex;
use crate::link_graph::agentic::{LinkGraphSuggestedLink, LinkGraphSuggestedLinkState};
use crate::link_graph::runtime_config::{
    resolve_link_graph_agentic_runtime, resolve_link_graph_cache_runtime,
};
use crate::link_graph::valkey_suggested_link_recent_latest_with_valkey;

const PROMOTED_OVERLAY_SOURCE: &str = "valkey.suggested_link_recent_latest";

#[derive(Debug, Clone)]
pub(super) struct PromotedOverlayStats {
    pub(super) applied: bool,
    pub(super) source: &'static str,
    pub(super) scanned_rows: usize,
    pub(super) promoted_rows: usize,
    pub(super) added_edges: usize,
}

impl Default for PromotedOverlayStats {
    fn default() -> Self {
        Self {
            applied: false,
            source: PROMOTED_OVERLAY_SOURCE,
            scanned_rows: 0,
            promoted_rows: 0,
            added_edges: 0,
        }
    }
}

impl LinkGraphIndex {
    /// Build a query-time graph overlay with promoted agentic edges.
    ///
    /// Returns `None` when runtime config is unavailable, no promoted rows exist,
    /// or no new edges are introduced.
    pub(super) fn with_promoted_edges_overlay(&self) -> Option<Self> {
        let (overlay, _) = self.with_promoted_edges_overlay_with_stats();
        overlay
    }

    /// Build query-time promoted-edge overlay and return telemetry stats.
    pub(super) fn with_promoted_edges_overlay_with_stats(
        &self,
    ) -> (Option<Self>, PromotedOverlayStats) {
        let mut stats = PromotedOverlayStats::default();
        let Ok(cache_runtime) = resolve_link_graph_cache_runtime() else {
            return (None, stats);
        };
        let agentic_runtime = resolve_link_graph_agentic_runtime();
        let scan_limit = agentic_runtime
            .execution_idempotency_scan_limit
            .max(agentic_runtime.suggested_link_max_entries)
            .max(1);
        let Ok(rows) = valkey_suggested_link_recent_latest_with_valkey(
            scan_limit,
            &cache_runtime.valkey_url,
            Some(&cache_runtime.key_prefix),
            Some(LinkGraphSuggestedLinkState::Promoted),
            Some(scan_limit),
        ) else {
            return (None, stats);
        };
        stats.scanned_rows = rows.len();
        stats.promoted_rows = rows
            .iter()
            .filter(|row| row.promotion_state == LinkGraphSuggestedLinkState::Promoted)
            .count();

        let (overlay, added_edges) = self.with_promoted_edges_from_rows(&rows);
        stats.added_edges = added_edges;
        stats.applied = overlay.is_some();
        (overlay, stats)
    }

    fn with_promoted_edges_from_rows(
        &self,
        rows: &[LinkGraphSuggestedLink],
    ) -> (Option<Self>, usize) {
        if rows.is_empty() {
            return (None, 0);
        }

        let mut merged = self.clone();
        let mut added_edges = 0usize;

        for row in rows {
            if row.promotion_state != LinkGraphSuggestedLinkState::Promoted {
                continue;
            }
            let Some(source_id) = merged.resolve_doc_id(&row.source_id).map(str::to_string) else {
                continue;
            };
            let Some(target_id) = merged.resolve_doc_id(&row.target_id).map(str::to_string) else {
                continue;
            };
            if source_id == target_id {
                continue;
            }

            let inserted = merged
                .outgoing
                .entry(source_id.clone())
                .or_default()
                .insert(target_id.clone());
            if !inserted {
                continue;
            }
            merged
                .incoming
                .entry(target_id)
                .or_default()
                .insert(source_id);
            added_edges = added_edges.saturating_add(1);
        }

        if added_edges == 0 {
            return (None, 0);
        }

        merged.edge_count = merged.edge_count.saturating_add(added_edges);
        merged.rank_by_id =
            Self::compute_rank_by_id(&merged.docs_by_id, &merged.incoming, &merged.outgoing);
        (Some(merged), added_edges)
    }
}
