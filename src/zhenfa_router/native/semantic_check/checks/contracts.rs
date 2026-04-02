use crate::link_graph::PageIndexNode;
use crate::zhenfa_router::native::semantic_check::parsing::validate_contract;
use crate::zhenfa_router::native::semantic_check::types::{IssueLocation, SemanticIssue, attrs};

/// Check contract constraints.
pub(crate) fn check_contracts(node: &PageIndexNode, doc_id: &str, issues: &mut Vec<SemanticIssue>) {
    if let Some(contract) = node.metadata.attributes.get(attrs::CONTRACT) {
        let content = &node.text;

        if let Some(violation) = validate_contract(contract, content) {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: "contract_violation".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Contract violation: {violation} (contract: '{contract}')"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(
                    "Update the content to satisfy the contract constraint".to_string(),
                ),
                fuzzy_suggestion: None,
            });
        }
    }
}
