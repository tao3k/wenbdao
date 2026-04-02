//! Saliency calculation for repository entities using structural topology.

use super::plugin::RepositoryAnalysisOutput;
use petgraph::graph::DiGraph;
#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Compute structural saliency scores for all symbols and modules in the analysis output.
/// Returns a map from entity ID to normalized saliency score (0.0 - 1.0).
pub fn compute_repository_saliency(analysis: &RepositoryAnalysisOutput) -> HashMap<String, f64> {
    let mut graph = DiGraph::<String, ()>::new();
    let mut nodes = HashMap::new();

    // 1. Collect all potential entities from records
    let mut entity_ids = Vec::new();
    for module in &analysis.modules {
        entity_ids.push(module.module_id.clone());
    }
    for symbol in &analysis.symbols {
        entity_ids.push(symbol.symbol_id.clone());
    }
    for example in &analysis.examples {
        entity_ids.push(example.example_id.clone());
    }

    for id in entity_ids {
        nodes
            .entry(id.clone())
            .or_insert_with(|| graph.add_node(id));
    }

    // 2. Add edges from relations
    for relation in &analysis.relations {
        if let (Some(&source), Some(&target)) = (
            nodes.get(&relation.source_id),
            nodes.get(&relation.target_id),
        ) {
            // Weight can be adjusted based on RelationKind
            graph.add_edge(source, target, ());
        }
    }

    // 3. Compute simple degree-based saliency (Placeholder for PPR)
    // Core hub nodes (high in-degree) get higher scores.
    let mut scores = HashMap::new();
    let node_count = graph.node_count();
    if node_count == 0 {
        return scores;
    }

    for idx in graph.node_indices() {
        let id = graph[idx].clone();
        let in_degree = graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .count();
        let out_degree = graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .count();

        // Saliency = normalized (in_degree * 2 + out_degree)
        // Hubs (like base types or common solvers) will have many incoming edges (Uses/Implements).
        let raw_score =
            (bounded_usize_to_f64(in_degree) * 2.0) + (bounded_usize_to_f64(out_degree) * 0.5);
        scores.insert(id, raw_score);
    }

    // 4. Normalize scores to 0.0 - 1.0
    let max_score = scores.values().copied().fold(0.0, f64::max);
    if max_score > 0.0 {
        for score in scores.values_mut() {
            *score /= max_score;
        }
    }

    scores
}

fn bounded_usize_to_f64(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::plugin::RepositoryAnalysisOutput;
    use crate::analyzers::records::{ModuleRecord, RelationKind, RelationRecord, SymbolRecord};

    #[test]
    fn test_compute_repository_saliency_basic() {
        let mut analysis = RepositoryAnalysisOutput::default();

        // Setup: A hub module with two symbols
        analysis.modules.push(ModuleRecord {
            repo_id: "test".to_string(),
            module_id: "mod1".to_string(),
            qualified_name: "Mod1".to_string(),
            path: "src/Mod1.jl".to_string(),
        });

        analysis.symbols.push(SymbolRecord {
            repo_id: "test".to_string(),
            symbol_id: "sym1".to_string(),
            module_id: Some("mod1".to_string()),
            name: "sym1".to_string(),
            qualified_name: "Mod1.sym1".to_string(),
            kind: crate::analyzers::RepoSymbolKind::Function,
            path: "src/Mod1.jl".to_string(),
            line_start: None,
            line_end: None,
            signature: None,
            audit_status: None,
            verification_state: None,
            attributes: BTreeMap::new(),
        });

        // Relation: mod1 contains sym1
        analysis.relations.push(RelationRecord {
            repo_id: "test".to_string(),
            source_id: "mod1".to_string(),
            target_id: "sym1".to_string(),
            kind: RelationKind::Contains,
        });

        let scores = compute_repository_saliency(&analysis);

        assert!(scores.contains_key("mod1"));
        assert!(scores.contains_key("sym1"));

        // mod1 has out-degree 1, sym1 has in-degree 1
        // raw_score(mod1) = 0*2 + 1*0.5 = 0.5
        // raw_score(sym1) = 1*2 + 0*0.5 = 2.0
        // Normalized: sym1 should be 1.0, mod1 should be 0.25
        let sym1_score = *scores
            .get("sym1")
            .unwrap_or_else(|| panic!("sym1 score should be present"));
        let mod1_score = *scores
            .get("mod1")
            .unwrap_or_else(|| panic!("mod1 score should be present"));
        assert!((sym1_score - 1.0).abs() < f64::EPSILON);
        assert!((mod1_score - 0.25).abs() < f64::EPSILON);
    }
}
