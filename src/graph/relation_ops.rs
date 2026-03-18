use super::core::{read_lock, write_lock};
use super::{GraphError, KnowledgeGraph};
use crate::entity::{Relation, RelationType};
use log::info;
use std::collections::{HashMap, HashSet};

impl KnowledgeGraph {
    /// Add a relation.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidRelation`] if source/target entities do not exist.
    #[allow(clippy::needless_pass_by_value)]
    pub fn add_relation(&self, relation: Relation) -> Result<(), GraphError> {
        let mut relations = write_lock::<HashMap<String, Relation>>(&self.relations);
        let mut outgoing = write_lock::<HashMap<String, HashSet<String>>>(&self.outgoing_relations);
        let mut incoming = write_lock::<HashMap<String, HashSet<String>>>(&self.incoming_relations);

        if relations.contains_key(&relation.id) {
            info!("Relation already exists: {}", relation.id);
            return Ok(());
        }

        {
            let entities_by_name = read_lock::<HashMap<String, String>>(&self.entities_by_name);
            if !entities_by_name.contains_key(&relation.source) {
                return Err(GraphError::InvalidRelation(
                    relation.source.clone(),
                    relation.target.clone(),
                ));
            }
            if !entities_by_name.contains_key(&relation.target) {
                return Err(GraphError::InvalidRelation(
                    relation.source.clone(),
                    relation.target.clone(),
                ));
            }
        }

        let relation_id = relation.id.clone();
        relations.insert(relation_id.clone(), relation.clone());

        outgoing
            .entry(relation.source.clone())
            .or_default()
            .insert(relation_id.clone());

        incoming
            .entry(relation.target.clone())
            .or_default()
            .insert(relation_id.clone());

        info!(
            "Added relation: {} -> {} ({})",
            relation.source, relation.target, relation.relation_type
        );
        Ok(())
    }

    /// Get relations for an entity.
    #[must_use]
    pub fn get_relations(
        &self,
        entity_name: Option<&str>,
        relation_type: Option<RelationType>,
    ) -> Vec<Relation> {
        let relations = read_lock::<HashMap<String, Relation>>(&self.relations);
        let mut results: Vec<Relation> = relations.values().cloned().collect();

        if let Some(name) = entity_name {
            let name_lower = name.to_lowercase();
            results.retain(|r| {
                r.source.to_lowercase() == name_lower || r.target.to_lowercase() == name_lower
            });
        }

        if let Some(rtype) = relation_type {
            results.retain(|r| r.relation_type == rtype);
        }

        results
    }

    /// Remove all relations where the given entity is the source.
    pub fn remove_relations_for_source(&self, source_name: &str) {
        let mut relations = write_lock::<HashMap<String, Relation>>(&self.relations);
        let mut outgoing = write_lock::<HashMap<String, HashSet<String>>>(&self.outgoing_relations);
        let mut incoming = write_lock::<HashMap<String, HashSet<String>>>(&self.incoming_relations);

        if let Some(rel_ids) = outgoing.remove(source_name) {
            for id in rel_ids {
                if let Some(rel) = relations.remove(&id)
                    && let Some(in_set) = incoming.get_mut(&rel.target)
                {
                    in_set.remove(&id);
                }
            }
        }
    }

    /// Get all relations as a vector.
    #[must_use]
    pub fn get_all_relations(&self) -> Vec<Relation> {
        read_lock::<HashMap<String, Relation>>(&self.relations)
            .values()
            .cloned()
            .collect()
    }
}
