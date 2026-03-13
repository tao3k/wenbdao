//! JSON persistence: save, load, export, and dict-based parsing.

use super::parse::{entity_from_dict, relation_from_dict};
use crate::entity::{Entity, Relation};
use crate::graph::{GraphError, KnowledgeGraph, read_lock};
use log::info;
use serde_json::{Value, json, to_string};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

impl KnowledgeGraph {
    /// Save graph to a JSON file.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidRelation`] when serialization or file I/O fails.
    pub fn save_to_file(&self, path: &str) -> Result<(), GraphError> {
        let entities = read_lock::<std::collections::HashMap<String, Entity>>(&self.entities);
        let relations = read_lock::<std::collections::HashMap<String, Relation>>(&self.relations);

        let entities_json: Vec<Value> = entities
            .values()
            .map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "entity_type": e.entity_type.to_string(),
                    "description": e.description,
                    "source": e.source,
                    "aliases": e.aliases,
                    "confidence": e.confidence,
                    "metadata": e.metadata,
                    "created_at": e.created_at,
                    "updated_at": e.updated_at,
                })
            })
            .collect();

        let relations_json: Vec<Value> = relations
            .values()
            .map(|r| {
                json!({
                    "id": r.id,
                    "source": r.source,
                    "target": r.target,
                    "relation_type": r.relation_type.to_string(),
                    "description": r.description,
                    "source_doc": r.source_doc,
                    "confidence": r.confidence,
                    "metadata": r.metadata,
                })
            })
            .collect();

        let export = json!({
            "version": 1,
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "total_entities": entities_json.len(),
            "total_relations": relations_json.len(),
            "entities": entities_json,
            "relations": relations_json,
        });

        let path_buf = PathBuf::from(path);
        if let Some(parent) = path_buf.parent()
            && !parent.exists()
            && let Err(e) = fs::create_dir_all(parent)
        {
            return Err(GraphError::InvalidRelation(
                parent.to_string_lossy().to_string(),
                e.to_string(),
            ));
        }

        let json_str = to_string(&export)
            .map_err(|e| GraphError::InvalidRelation("serialization".to_string(), e.to_string()))?;

        let mut file = File::create(path_buf)
            .map_err(|e| GraphError::InvalidRelation(path.to_string(), e.to_string()))?;

        file.write_all(json_str.as_bytes())
            .map_err(|e| GraphError::InvalidRelation(path.to_string(), e.to_string()))?;

        info!(
            "Knowledge graph saved to: {} ({} entities, {} relations)",
            path,
            entities_json.len(),
            relations_json.len()
        );

        Ok(())
    }

    /// Load graph from JSON file.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::InvalidRelation`] when file read or JSON parse fails.
    pub fn load_from_file(&mut self, path: &str) -> Result<(), GraphError> {
        let mut file = File::open(path)
            .map_err(|e| GraphError::InvalidRelation(path.to_string(), e.to_string()))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| GraphError::InvalidRelation(path.to_string(), e.to_string()))?;

        let value: Value = serde_json::from_str(&content)
            .map_err(|e| GraphError::InvalidRelation("parse".to_string(), e.to_string()))?;

        self.clear();

        if let Some(entities_arr) = value.get("entities").and_then(|v| v.as_array()) {
            for entity_val in entities_arr {
                if let Some(entity) = entity_from_dict(entity_val) {
                    self.add_entity(entity).ok();
                }
            }
        }

        if let Some(relations_arr) = value.get("relations").and_then(|v| v.as_array()) {
            for relation_val in relations_arr {
                if let Some(relation) = relation_from_dict(relation_val) {
                    self.add_relation(relation).ok();
                }
            }
        }

        let stats = self.get_stats();
        info!(
            "Knowledge graph loaded from: {} ({} entities, {} relations)",
            path, stats.total_entities, stats.total_relations
        );

        Ok(())
    }
}
