use crate::link_graph::{PageIndexNode, RegistryIndex};
use crate::zhenfa_router::native::semantic_check::parsing::{
    extract_hash_references, extract_id_references,
};
use crate::zhenfa_router::native::semantic_check::types::{
    IssueLocation, NodeStatus, SemanticIssue, attrs,
};

/// Check for dead links (references to non-existent IDs).
pub(crate) fn check_dead_links(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let id_refs = extract_id_references(&node.text);

    for entity in id_refs {
        let target_id = &entity[1..];
        if !registry.contains(target_id) {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: "dead_link".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Dead link: reference to non-existent ID '{target_id}'"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(format!(
                    "Remove the reference or create a node with :ID: {target_id}"
                )),
                fuzzy_suggestion: None,
            });
        }
    }
}

/// Check for references to deprecated nodes.
pub(crate) fn check_deprecated_refs(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let id_refs = extract_id_references(&node.text);

    for entity in id_refs {
        let target_id = &entity[1..];
        if let Some(indexed) = registry.get(target_id)
            && let Some(status_str) = indexed.node.metadata.attributes.get(attrs::STATUS)
            && NodeStatus::parse_lossy(status_str) == NodeStatus::Deprecated
        {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: "deprecated_ref".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Reference to deprecated node '{target_id}' (status: DEPRECATED)"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(format!(
                    "Update reference from deprecated node '{target_id}' to its replacement"
                )),
                fuzzy_suggestion: None,
            });
        }
    }
}

/// Check hash alignment (`expect_hash` vs actual `content_hash`).
pub(crate) fn check_hash_alignment(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let hash_refs = extract_hash_references(&node.text);

    for hash_ref in hash_refs {
        if let Some(expect_hash) = &hash_ref.expect_hash
            && let Some(indexed) = registry.get(&hash_ref.target_id)
        {
            if let Some(actual_hash) = &indexed.node.metadata.content_hash {
                if expect_hash != actual_hash {
                    issues.push(SemanticIssue {
                        severity: "warning".to_string(),
                        issue_type: "content_drift".to_string(),
                        doc: doc_id.to_string(),
                        node_id: node.node_id.clone(),
                        message: format!(
                            "Content drift: reference to '{}' expects hash '{}' but current hash is '{}'",
                            hash_ref.target_id, expect_hash, actual_hash
                        ),
                        location: Some(IssueLocation::from_node(node)),
                        suggestion: Some(format!(
                            "Update the reference hash to '{actual_hash}' or verify the content change is intentional"
                        )),
                        fuzzy_suggestion: None,
                    });
                }
            } else {
                issues.push(SemanticIssue {
                    severity: "info".to_string(),
                    issue_type: "missing_content_hash".to_string(),
                    doc: doc_id.to_string(),
                    node_id: node.node_id.clone(),
                    message: format!(
                        "Target '{}' has no content_hash for verification",
                        hash_ref.target_id
                    ),
                    location: Some(IssueLocation::from_node(node)),
                    suggestion: None,
                    fuzzy_suggestion: None,
                });
            }
        }
    }
}
