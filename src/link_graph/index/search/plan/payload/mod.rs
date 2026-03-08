use super::super::super::LinkGraphIndex;
use crate::link_graph::{LinkGraphPromotedOverlayTelemetry, LinkGraphSearchOptions};

mod core;
mod policy;

impl LinkGraphIndex {
    /// Parse/execute search and return canonical external payload shape.
    #[must_use]
    pub fn search_planned_payload(
        &self,
        query: &str,
        limit: usize,
        base_options: LinkGraphSearchOptions,
    ) -> crate::link_graph::LinkGraphPlannedSearchPayload {
        self.search_planned_payload_with_agentic(query, limit, base_options, None, None)
    }

    /// Parse/execute search and return canonical external payload shape with
    /// optional provisional suggested-link injection policy.
    #[must_use]
    pub fn search_planned_payload_with_agentic(
        &self,
        query: &str,
        limit: usize,
        base_options: LinkGraphSearchOptions,
        include_provisional: Option<bool>,
        provisional_limit: Option<usize>,
    ) -> crate::link_graph::LinkGraphPlannedSearchPayload {
        let (overlay, overlay_stats) = self.with_promoted_edges_overlay_with_stats();
        let promoted_overlay = Some(LinkGraphPromotedOverlayTelemetry {
            applied: overlay_stats.applied,
            source: overlay_stats.source.to_string(),
            scanned_rows: overlay_stats.scanned_rows,
            promoted_rows: overlay_stats.promoted_rows,
            added_edges: overlay_stats.added_edges,
        });

        if let Some(overlay) = overlay {
            return overlay.search_planned_payload_with_agentic_core(
                query,
                limit,
                base_options,
                include_provisional,
                provisional_limit,
                promoted_overlay,
            );
        }
        self.search_planned_payload_with_agentic_core(
            query,
            limit,
            base_options,
            include_provisional,
            provisional_limit,
            promoted_overlay,
        )
    }
}
