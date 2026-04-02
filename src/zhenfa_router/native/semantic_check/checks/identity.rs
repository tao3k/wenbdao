use crate::link_graph::PageIndexNode;
use crate::zhenfa_router::native::semantic_check::parsing::generate_suggested_id;
use crate::zhenfa_router::native::semantic_check::types::{IssueLocation, SemanticIssue, attrs};

/// Check for missing mandatory :ID: property drawer (Blueprint v2.2).
pub(crate) fn check_missing_identity(
    node: &PageIndexNode,
    doc_id: &str,
    issues: &mut Vec<SemanticIssue>,
) {
    let should_have_id = node.level <= 2;

    if should_have_id && !node.metadata.attributes.contains_key(attrs::ID) {
        issues.push(SemanticIssue {
            severity: "warning".to_string(),
            issue_type: "missing_identity".to_string(),
            doc: doc_id.to_string(),
            node_id: node.node_id.clone(),
            message: format!(
                "Heading '{}' at level {} lacks explicit :ID: property drawer",
                node.title, node.level
            ),
            location: Some(IssueLocation::from_node(node)),
            suggestion: Some(format!(
                "Add a property drawer with :ID: {} to enable stable anchoring",
                generate_suggested_id(&node.title)
            )),
            fuzzy_suggestion: None,
        });
    }
}

/// Check for legacy syntax markers (Blueprint v2.2).
pub(crate) fn check_legacy_syntax(
    node: &PageIndexNode,
    doc_id: &str,
    issues: &mut Vec<SemanticIssue>,
) {
    let text = &node.text;

    let legacy_patterns = [
        ("SEE ALSO", "Use `[[#id]]` wiki-links instead"),
        ("RELATED TO", "Use `[[#id]]` wiki-links instead"),
        (
            "<<",
            "Use `[[#id]]` for internal links instead of <<legacy>> syntax",
        ),
    ];

    for (pattern, suggestion) in legacy_patterns {
        if text.contains(pattern) {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: "legacy_syntax".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Legacy syntax '{pattern}' detected"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(suggestion.to_string()),
                fuzzy_suggestion: None,
            });
        }
    }
}
