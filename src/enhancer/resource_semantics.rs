use std::sync::Arc;

use crate::entity::{EntityType, RelationType};

/// Structural semantics classification for a skill reference link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillReferenceSemantics {
    /// Recommended entity type for the target reference.
    pub entity: EntityType,
    /// Recommended relation type from skill to this reference.
    pub relation: RelationType,
    /// Semantic reference category (e.g. `template`, `knowledge`).
    pub reference_type: Option<Arc<str>>,
}

/// Classify a skill reference link based on its target path and type-hints.
#[must_use]
pub fn classify_skill_reference(
    explicit_type: Option<&str>,
    config_type: Option<&str>,
    entity_path: &str,
) -> SkillReferenceSemantics {
    let lower_path = entity_path.trim().to_ascii_lowercase();
    let explicit_lower = explicit_type.map(str::trim).map(str::to_ascii_lowercase);
    let config_lower = config_type.map(str::trim).map(str::to_ascii_lowercase);

    // 1. Explicit type-hint takes precedence (e.g. [[note#template]])
    if let Some(ref_type) = explicit_lower {
        match ref_type.as_str() {
            "template" | "tpl" => {
                return SkillReferenceSemantics {
                    entity: EntityType::Other("Template".to_string()),
                    relation: RelationType::RelatedTo,
                    reference_type: Some(Arc::from("template")),
                };
            }
            "persona" | "agent" => {
                return SkillReferenceSemantics {
                    entity: EntityType::Other("Persona".to_string()),
                    relation: RelationType::Manifests,
                    reference_type: Some(Arc::from("persona")),
                };
            }
            "knowledge" | "doc" => {
                return SkillReferenceSemantics {
                    entity: EntityType::Document,
                    relation: RelationType::DocumentedIn,
                    reference_type: Some(Arc::from("knowledge")),
                };
            }
            "qianji-flow" | "flow" => {
                return SkillReferenceSemantics {
                    entity: EntityType::Other("QianjiFlow".to_string()),
                    relation: RelationType::Governs,
                    reference_type: Some(Arc::from("qianji-flow")),
                };
            }
            _ => {}
        }
    }

    // 2. Config block type fallback
    if let Some(cfg_type) = config_lower {
        if cfg_type.contains("prompt") || cfg_type.contains("template") {
            return SkillReferenceSemantics {
                entity: EntityType::Other("Template".to_string()),
                relation: RelationType::RelatedTo,
                reference_type: Some(Arc::from("template")),
            };
        }
    }

    // 3. Path-based heuristics
    if lower_path.contains("/templates/") || lower_path.contains("/tpl/") {
        return SkillReferenceSemantics {
            entity: EntityType::Other("Template".to_string()),
            relation: RelationType::RelatedTo,
            reference_type: Some(Arc::from("template")),
        };
    }
    if lower_path.contains("/personas/") {
        return SkillReferenceSemantics {
            entity: EntityType::Person,
            relation: RelationType::Governs,
            reference_type: Some(Arc::from("persona")),
        };
    }

    // 4. Attachment detection by file extension
    let attachment_extensions = [
        ".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp", ".ico", ".bmp", ".pdf", ".doc", ".docx",
        ".xls", ".xlsx", ".ppt", ".pptx", ".zip", ".tar", ".gz", ".rar", ".7z", ".mp3", ".mp4",
        ".wav", ".avi", ".mov", ".webm", ".ttf", ".otf", ".woff", ".woff2",
    ];
    for ext in &attachment_extensions {
        if lower_path.ends_with(ext) {
            return SkillReferenceSemantics {
                entity: EntityType::Other("Attachment".to_string()),
                relation: RelationType::AttachedTo,
                reference_type: Some(Arc::from("attachment")),
            };
        }
    }

    // Default fallback: generic relationship
    SkillReferenceSemantics {
        entity: EntityType::Other("Resource".to_string()),
        relation: RelationType::RelatedTo,
        reference_type: None,
    }
}
