use crate::link_graph::index::LinkGraphIndex;
use crate::link_graph::models::{LinkGraphDirection, LinkGraphPromotedOverlayTelemetry};

#[allow(dead_code)]
pub(super) fn merge_direction(
    existing: LinkGraphDirection,
    new_dir: LinkGraphDirection,
) -> LinkGraphDirection {
    if existing == new_dir {
        existing
    } else {
        LinkGraphDirection::Both
    }
}

impl LinkGraphIndex {
    #[allow(dead_code)]
    pub(super) fn promoted_overlay_telemetry(
        &self,
    ) -> (Option<Self>, LinkGraphPromotedOverlayTelemetry) {
        let (overlay, stats) = self.with_promoted_edges_overlay_with_stats();
        let telemetry = LinkGraphPromotedOverlayTelemetry {
            applied: stats.applied,
            source: stats.source.to_string(),
            scanned_rows: stats.scanned_rows,
            promoted_rows: stats.promoted_rows,
            added_edges: stats.added_edges,
        };
        (overlay, telemetry)
    }
}
