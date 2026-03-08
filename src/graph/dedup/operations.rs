use super::super::core::read_lock;
use super::super::{GraphError, KnowledgeGraph};
use super::DeduplicationResult;
use crate::entity::Entity;
use serde_json::json;
use std::collections::HashSet;

impl KnowledgeGraph {
    /// Find potential duplicate entities.
    #[must_use]
    pub fn find_duplicates(&self, threshold: f32) -> Vec<Vec<String>> {
        let entities = read_lock(&self.entities);
        let names: Vec<(String, String)> = entities
            .values()
            .map(|entity| (entity.name.clone(), entity.id.clone()))
            .collect();

        let mut groups: Vec<Vec<String>> = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();

        for (name, id) in &names {
            if visited.contains(id) {
                continue;
            }

            let mut group: Vec<String> = vec![id.clone()];
            visited.insert(id.clone());

            for (other_name, other_id) in &names {
                if id == other_id || visited.contains(other_id) {
                    continue;
                }

                if Self::name_similarity(name, other_name) >= threshold {
                    group.push(other_id.clone());
                    visited.insert(other_id.clone());
                }
            }

            if group.len() > 1 {
                groups.push(group);
            }
        }

        groups
    }

    /// Merge multiple entities into a single canonical entity.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EntityNotFound`] when no IDs can be resolved.
    pub fn merge_entities(
        &self,
        entity_ids: &[String],
        canonical_name: &str,
    ) -> Result<Entity, GraphError> {
        let entities = read_lock(&self.entities);

        let mut merged = None;
        let mut all_aliases: Vec<String> = Vec::new();
        let mut sources: Vec<String> = Vec::new();
        let mut max_confidence: f32 = 0.0;

        for id in entity_ids {
            if let Some(entity) = entities.get(id) {
                if merged.is_none() {
                    merged = Some(entity.clone());
                } else if let Some(current) = &mut merged {
                    for alias in &entity.aliases {
                        if !current.aliases.contains(alias) {
                            all_aliases.push(alias.clone());
                        }
                    }
                    if !current.aliases.contains(&entity.name) {
                        all_aliases.push(entity.name.clone());
                    }

                    if let Some(ref src) = entity.source
                        && !sources.contains(src)
                    {
                        sources.push(src.clone());
                    }

                    max_confidence = max_confidence.max(entity.confidence);
                }
            }
        }

        if let Some(mut canonical) = merged {
            if !canonical_name.is_empty() {
                canonical.name = canonical_name.to_string();
            }
            let mut existing_aliases = canonical.aliases.clone();
            existing_aliases.extend(all_aliases);
            existing_aliases.sort();
            existing_aliases.dedup();
            canonical.aliases = existing_aliases;

            if !sources.is_empty() {
                canonical
                    .metadata
                    .insert("merged_sources".to_string(), json!(sources));
            }

            canonical.confidence = max_confidence.max(canonical.confidence);
            canonical.updated_at = chrono::Utc::now();

            // Remove old entities and add canonical
            drop(entities);

            for id in entity_ids {
                self.remove_entity(id)?;
            }

            self.add_entity(canonical.clone())?;

            Ok(canonical)
        } else {
            Err(GraphError::EntityNotFound(entity_ids.join(", ")))
        }
    }

    /// Auto-deduplicate the graph based on similarity threshold.
    #[must_use]
    pub fn deduplicate(&self, threshold: f32) -> DeduplicationResult {
        let duplicates = self.find_duplicates(threshold);

        let mut merged_count = 0;
        let duplicate_groups = duplicates.len();

        for group in &duplicates {
            if group.len() > 1 {
                let canonical_name = self.find_canonical_name(group);
                if self.merge_entities(group, &canonical_name).is_ok() {
                    merged_count += group.len() - 1;
                }
            }
        }

        DeduplicationResult {
            duplicate_groups_found: duplicate_groups,
            entities_merged: merged_count,
        }
    }

    /// Find the most canonical name from a group of entity IDs.
    fn find_canonical_name(&self, entity_ids: &[String]) -> String {
        let entities = read_lock(&self.entities);

        let mut best: Option<(usize, String)> = None;

        for id in entity_ids {
            if let Some(entity) = entities.get(id) {
                let score = entity.description.len() + entity.aliases.len() * 10;
                if let Some((best_score, _)) = &best {
                    if score > *best_score {
                        best = Some((score, entity.name.clone()));
                    }
                } else {
                    best = Some((score, entity.name.clone()));
                }
            }
        }

        best.map_or_else(
            || entity_ids.first().cloned().unwrap_or_default(),
            |(_, name)| name,
        )
    }
}
