use super::KnowledgeGraph;
use super::core::read_lock;
use crate::entity::GraphStats;
use std::collections::HashMap;

fn saturating_usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

impl KnowledgeGraph {
    /// Get graph statistics.
    #[must_use]
    pub fn get_stats(&self) -> GraphStats {
        let entities = read_lock(&self.entities);
        let relations = read_lock(&self.relations);
        let entities_by_type = read_lock(&self.entities_by_type);

        let mut entities_by_type_count: HashMap<String, i64> = HashMap::new();
        for (etype, eids) in entities_by_type.iter() {
            entities_by_type_count.insert(etype.clone(), saturating_usize_to_i64(eids.len()));
        }

        let mut relations_by_type: HashMap<String, i64> = HashMap::new();
        for rel in relations.values() {
            let rtype = rel.relation_type.to_string();
            *relations_by_type.entry(rtype).or_insert(0) += 1;
        }

        GraphStats {
            total_entities: saturating_usize_to_i64(entities.len()),
            total_relations: saturating_usize_to_i64(relations.len()),
            entities_by_type: entities_by_type_count,
            relations_by_type,
            last_updated: None,
        }
    }
}
