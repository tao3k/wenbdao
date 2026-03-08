use super::super::{
    LinkGraphDirection, LinkGraphIndex, LinkGraphNeighbor, LinkGraphPromotedOverlayTelemetry,
    LinkGraphRelatedPprDiagnostics, LinkGraphRelatedPprOptions,
};
use std::collections::HashMap;

impl LinkGraphIndex {
    fn build_related_neighbors_from_ranked(
        &self,
        ranked: Vec<(String, usize, f64)>,
        limit: usize,
    ) -> Vec<LinkGraphNeighbor> {
        let bounded_limit = limit.max(1);
        ranked
            .into_iter()
            .filter_map(|(doc_id, distance, _score)| {
                self.docs_by_id.get(&doc_id).map(|doc| LinkGraphNeighbor {
                    stem: doc.stem.clone(),
                    direction: LinkGraphDirection::Both,
                    distance,
                    title: doc.title.clone(),
                    path: doc.path.clone(),
                })
            })
            .take(bounded_limit)
            .collect()
    }

    /// Find related notes through bidirectional traversal.
    #[must_use]
    pub fn related(
        &self,
        stem_or_id: &str,
        max_distance: usize,
        limit: usize,
    ) -> Vec<LinkGraphNeighbor> {
        self.related_with_options(stem_or_id, max_distance, limit, None)
    }

    /// Find related notes through bidirectional traversal with explicit PPR options.
    #[must_use]
    pub fn related_with_options(
        &self,
        stem_or_id: &str,
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> Vec<LinkGraphNeighbor> {
        self.related_with_diagnostics_and_overlay(stem_or_id, max_distance, limit, ppr)
            .0
    }

    /// Find related notes and return extra PPR diagnostics for debug/observability.
    #[must_use]
    pub fn related_with_diagnostics(
        &self,
        stem_or_id: &str,
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
    ) {
        let (rows, diagnostics, _) =
            self.related_with_diagnostics_and_overlay(stem_or_id, max_distance, limit, ppr);
        (rows, diagnostics)
    }

    /// Find related notes and return PPR diagnostics plus promoted overlay telemetry.
    #[must_use]
    pub fn related_with_diagnostics_and_overlay(
        &self,
        stem_or_id: &str,
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
        LinkGraphPromotedOverlayTelemetry,
    ) {
        let (overlay, telemetry) = self.promoted_overlay_telemetry();
        let (rows, diagnostics) = if let Some(overlay) = overlay {
            overlay.related_with_diagnostics_core(stem_or_id, max_distance, limit, ppr)
        } else {
            self.related_with_diagnostics_core(stem_or_id, max_distance, limit, ppr)
        };
        (rows, diagnostics, telemetry)
    }

    fn related_with_diagnostics_core(
        &self,
        stem_or_id: &str,
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
    ) {
        let seeds = vec![stem_or_id.to_string()];
        self.related_from_seeds_with_diagnostics_core(&seeds, max_distance, limit, ppr)
    }

    /// Find related notes from explicit seed notes and return PPR diagnostics.
    #[must_use]
    pub fn related_from_seeds_with_diagnostics(
        &self,
        seeds: &[String],
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
    ) {
        if let Some(overlay) = self.with_promoted_edges_overlay() {
            return overlay.related_from_seeds_with_diagnostics_core(
                seeds,
                max_distance,
                limit,
                ppr,
            );
        }
        self.related_from_seeds_with_diagnostics_core(seeds, max_distance, limit, ppr)
    }

    fn related_from_seeds_with_diagnostics_core(
        &self,
        seeds: &[String],
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
    ) {
        let seed_map: HashMap<String, f64> = seeds.iter().map(|s| (s.clone(), 1.0)).collect();
        self.related_from_weighted_seeds_with_diagnostics_core(&seed_map, max_distance, limit, ppr)
    }

    /// Find related notes from weighted seed notes and return PPR diagnostics.
    #[must_use]
    pub fn related_from_weighted_seeds_with_diagnostics(
        &self,
        seeds: &HashMap<String, f64>,
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
    ) {
        // Overlay logic (simplified here for brevity, assuming similar to unweighted)
        self.related_from_weighted_seeds_with_diagnostics_core(seeds, max_distance, limit, ppr)
    }

    fn related_from_weighted_seeds_with_diagnostics_core(
        &self,
        seeds: &HashMap<String, f64>,
        max_distance: usize,
        limit: usize,
        ppr: Option<&LinkGraphRelatedPprOptions>,
    ) -> (
        Vec<LinkGraphNeighbor>,
        Option<LinkGraphRelatedPprDiagnostics>,
    ) {
        let seed_ids = self.resolve_weighted_doc_ids(seeds);
        if seed_ids.is_empty() {
            return (Vec::new(), None);
        }
        let Some(computation) = self.related_ppr_compute(&seed_ids, max_distance.max(1), ppr)
        else {
            return (Vec::new(), None);
        };
        (
            self.build_related_neighbors_from_ranked(computation.ranked_doc_ids, limit),
            Some(computation.diagnostics),
        )
    }
}
