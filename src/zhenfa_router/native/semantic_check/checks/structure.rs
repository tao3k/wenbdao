use crate::link_graph::RegistryBuildResult;
use crate::zhenfa_router::native::semantic_check::types::{IssueLocation, SemanticIssue};

/// Check for ID collisions (same ID in multiple documents).
pub(crate) fn check_id_collisions(
    build_result: &RegistryBuildResult,
    issues: &mut Vec<SemanticIssue>,
) {
    for collision in &build_result.collisions {
        let locations_str = collision
            .locations
            .iter()
            .map(|(doc_id, path)| format!("{}:{}", doc_id, path.join("/")))
            .collect::<Vec<_>>()
            .join(", ");

        let (primary_doc, primary_path) = &collision.locations[0];

        issues.push(SemanticIssue {
            severity: "error".to_string(),
            issue_type: "id_collision".to_string(),
            doc: primary_doc.clone(),
            node_id: collision.id.clone(),
            message: format!(
                "ID collision: '{}' appears in {} locations: {}",
                collision.id,
                collision.locations.len(),
                locations_str
            ),
            location: Some(IssueLocation {
                line: 0,
                heading_path: primary_path.join(" / "),
                byte_range: None,
            }),
            suggestion: Some(
                "Rename one of the nodes to have a unique ID, or remove duplicate :ID: attributes"
                    .to_string(),
            ),
            fuzzy_suggestion: None,
        });
    }
}
