use crate::link_graph_refs::LinkGraphEntityRef;

use super::types::{InferredRelation, NoteFrontmatter};

/// Infer relations from note structure.
///
/// Relations inferred:
/// - `DOCUMENTED_IN`: Entity refs → this document
/// - `CONTAINS`: Skill SKILL.md → its skill name
/// - `RELATED_TO`: Document → tags
#[must_use]
pub fn infer_relations(
    note_path: &str,
    note_title: &str,
    frontmatter: &NoteFrontmatter,
    entity_refs: &[LinkGraphEntityRef],
) -> Vec<InferredRelation> {
    let mut relations = Vec::new();
    let doc_name = note_title;

    // Entity refs → DOCUMENTED_IN
    for entity_ref in entity_refs {
        relations.push(InferredRelation {
            source: entity_ref.name.clone(),
            target: doc_name.to_string(),
            relation_type: "DOCUMENTED_IN".to_string(),
            description: format!("{} documented in {}", entity_ref.name, doc_name),
        });
    }

    // Skill SKILL.md → CONTAINS
    let is_skill = note_path.to_uppercase().contains("SKILL.MD")
        || note_path.to_uppercase().ends_with("SKILL.MD");
    if is_skill && let Some(ref name) = frontmatter.name {
        relations.push(InferredRelation {
            source: name.clone(),
            target: doc_name.to_string(),
            relation_type: "CONTAINS".to_string(),
            description: format!("Skill {name} defined in {doc_name}"),
        });
    }

    // Tags → RELATED_TO
    for tag in &frontmatter.tags {
        relations.push(InferredRelation {
            source: doc_name.to_string(),
            target: format!("tag:{tag}"),
            relation_type: "RELATED_TO".to_string(),
            description: format!("{doc_name} tagged with {tag}"),
        });
    }

    relations
}
