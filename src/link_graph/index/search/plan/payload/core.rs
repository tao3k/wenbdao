use super::super::super::super::{LinkGraphIndex, parse_search_query};
use super::policy::{LinkGraphPolicyDecision, evaluate_link_graph_policy};
use crate::link_graph::agentic::{LinkGraphSuggestedLink, LinkGraphSuggestedLinkState};
use crate::link_graph::runtime_config::resolve_link_graph_agentic_runtime;
use crate::link_graph::valkey_suggested_link_recent_latest;
use crate::link_graph::{
    LinkGraphCcsAudit, LinkGraphDirection, LinkGraphDisplayHit, LinkGraphHit,
    LinkGraphPlannedSearchPayload, LinkGraphPromotedOverlayTelemetry, ParsedLinkGraphQuery,
};
use std::collections::HashMap;

impl LinkGraphIndex {
    pub(in crate::link_graph::index::search::plan) fn search_planned_payload_with_agentic_core(
        &self,
        query: &str,
        limit: usize,
        base_options: crate::link_graph::LinkGraphSearchOptions,
        include_provisional: Option<bool>,
        provisional_limit: Option<usize>,
        promoted_overlay: Option<LinkGraphPromotedOverlayTelemetry>,
    ) -> LinkGraphPlannedSearchPayload {
        let parsed = parse_search_query(query, base_options);
        let effective_limit = parsed.limit_override.unwrap_or(limit);

        if let Some(direct_id) = parsed.direct_id.as_deref() {
            let rows = self.execute_direct_id_lookup(direct_id, effective_limit, &parsed.options);
            let policy = evaluate_link_graph_policy(&rows, effective_limit);
            return self.build_planned_payload(
                parsed,
                rows,
                policy,
                Vec::new(),
                None,
                promoted_overlay,
            );
        }

        let (provisional_suggestions, provisional_error, provisional_doc_boosts) =
            self.resolve_provisional_search_inputs(&parsed, include_provisional, provisional_limit);

        let rows = self.execute_search_with_doc_boosts(
            &parsed.query,
            effective_limit,
            parsed.options.clone(),
            (!provisional_doc_boosts.is_empty()).then_some(&provisional_doc_boosts),
        );

        let policy = evaluate_link_graph_policy(&rows, effective_limit);

        self.build_planned_payload(
            parsed,
            rows,
            policy,
            provisional_suggestions,
            provisional_error,
            promoted_overlay,
        )
    }

    fn resolve_provisional_search_inputs(
        &self,
        parsed: &ParsedLinkGraphQuery,
        include_provisional: Option<bool>,
        provisional_limit: Option<usize>,
    ) -> (
        Vec<LinkGraphSuggestedLink>,
        Option<String>,
        HashMap<String, f64>,
    ) {
        let agentic_runtime = resolve_link_graph_agentic_runtime();
        let include_provisional =
            include_provisional.unwrap_or(agentic_runtime.search_include_provisional_default);
        let provisional_limit = provisional_limit
            .unwrap_or(agentic_runtime.search_provisional_limit)
            .max(1);
        let (provisional_suggestions, provisional_error) = if include_provisional {
            match valkey_suggested_link_recent_latest(
                provisional_limit,
                Some(LinkGraphSuggestedLinkState::Provisional),
            ) {
                Ok(rows) => (rows, None),
                Err(err) => (Vec::new(), Some(err)),
            }
        } else {
            (Vec::new(), None)
        };
        let provisional_doc_boosts = if include_provisional {
            self.build_provisional_doc_boosts(
                &parsed.query,
                parsed.options.case_sensitive,
                &provisional_suggestions,
            )
        } else {
            HashMap::new()
        };

        (
            provisional_suggestions,
            provisional_error,
            provisional_doc_boosts,
        )
    }

    fn build_planned_payload(
        &self,
        parsed: ParsedLinkGraphQuery,
        rows: Vec<LinkGraphHit>,
        policy: LinkGraphPolicyDecision,
        provisional_suggestions: Vec<LinkGraphSuggestedLink>,
        provisional_error: Option<String>,
        promoted_overlay: Option<LinkGraphPromotedOverlayTelemetry>,
    ) -> LinkGraphPlannedSearchPayload {
        let hit_count = rows.len();
        let section_hit_count = rows
            .iter()
            .filter(|row| {
                row.best_section
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|value| !value.is_empty())
            })
            .count();
        let hits = rows
            .iter()
            .map(LinkGraphDisplayHit::from)
            .collect::<Vec<_>>();

        crate::link_graph::saliency::touch_search_hits_with_coactivation_async(
            &hits,
            &coactivated_neighbor_node_ids(self, &hits),
        );

        // Compute CCS audit before moving ownership
        let ccs_audit = self.compute_ccs_audit(&parsed.options.style_anchors, &hits);

        LinkGraphPlannedSearchPayload {
            query: parsed.query,
            options: parsed.options,
            hits,
            hit_count,
            section_hit_count,
            requested_mode: policy.requested_mode,
            selected_mode: policy.selected_mode,
            reason: policy.reason,
            graph_hit_count: policy.graph_hit_count,
            source_hint_count: policy.source_hint_count,
            graph_confidence_score: policy.graph_confidence_score,
            graph_confidence_level: policy.graph_confidence_level,
            retrieval_plan: Some(policy.retrieval_plan),
            results: rows,
            provisional_suggestions: provisional_suggestions.to_vec(),
            provisional_error,
            promoted_overlay,
            ccs_audit,
        }
    }

    fn compute_ccs_audit(
        &self,
        style_anchors: &[String],
        hits: &[LinkGraphDisplayHit],
    ) -> Option<LinkGraphCcsAudit> {
        use crate::link_graph::LinkGraphCcsAudit;
        use crate::zhenfa_router::{audit_search_payload, evaluate_alignment};

        if style_anchors.is_empty() {
            return None;
        }

        // Extract evidence from search hits (titles, stems, sections)
        let evidence: Vec<String> = hits
            .iter()
            .flat_map(|hit| {
                let mut parts = vec![hit.title.clone(), hit.stem.clone()];
                if !hit.best_section.trim().is_empty() {
                    parts.push(hit.best_section.clone());
                }
                parts
            })
            .collect();

        // Run CCS audit using the zhenfa router audit module
        let audit = audit_search_payload(&evidence, style_anchors);
        let verdict = evaluate_alignment(style_anchors, &evidence);

        // Build the CCS audit result for payload
        Some(LinkGraphCcsAudit {
            ccs_score: audit.ccs_score,
            passed: audit.passed && verdict.is_aligned,
            compensated: false,
            missing_anchors: audit.missing_anchors,
        })
    }
}

fn coactivated_neighbor_node_ids(
    index: &LinkGraphIndex,
    hits: &[LinkGraphDisplayHit],
) -> Vec<crate::link_graph::saliency::SearchHitCoactivationLink> {
    use crate::link_graph::runtime_config::resolve_link_graph_coactivation_runtime;
    let runtime = resolve_link_graph_coactivation_runtime();
    if !runtime.enabled || runtime.max_neighbors_per_direction == 0 {
        return Vec::new();
    }

    let neighbor_limit = runtime.max_neighbors_per_direction.saturating_mul(2);
    hits.iter()
        .flat_map(|hit| {
            index
                .neighbors(&hit.stem, LinkGraphDirection::Outgoing, 1, neighbor_limit)
                .into_iter()
                .enumerate()
                .map(
                    |(rank, neighbor)| crate::link_graph::saliency::SearchHitCoactivationLink {
                        source_node_id: hit.stem.clone(),
                        neighbor_node_id: neighbor.stem,
                        pre_resolved_rank: rank,
                    },
                )
        })
        .collect()
}
