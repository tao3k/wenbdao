//! Skeptic audit logic for verifying documentation and symbol consistency.

use super::records::{DocRecord, RelationKind, RelationRecord, SymbolRecord};
use std::collections::HashMap;

/// Result of a skepticism audit for a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditResult {
    /// The entity is verified against its documentation or implementation.
    Verified,
    /// The entity is suspicious or has mismatched documentation.
    Unverified,
    /// No sufficient evidence to audit.
    Unknown,
}

impl AuditResult {
    /// Returns the string representation of the audit result.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
            Self::Unknown => "unknown",
        }
    }
}

/// Perform a basic skepticism audit on all symbols in the analysis output.
/// Returns a map from symbol ID to its verification state.
pub fn audit_symbols(
    symbols: &[SymbolRecord],
    docs: &[DocRecord],
    relations: &[RelationRecord],
) -> HashMap<String, String> {
    let mut audit_map = HashMap::new();

    // 1. Build a lookup for which symbols are documented by which docs
    let mut symbol_to_docs = HashMap::new();
    for rel in relations {
        if rel.kind == RelationKind::Documents {
            symbol_to_docs
                .entry(rel.target_id.clone())
                .or_insert_with(Vec::new)
                .push(rel.source_id.clone());
        }
    }

    let doc_map: HashMap<String, &DocRecord> =
        docs.iter().map(|doc| (doc.doc_id.clone(), doc)).collect();

    // 2. Audit each symbol
    for symbol in symbols {
        let result = if let Some(doc_ids) = symbol_to_docs.get(&symbol.symbol_id) {
            // Basic check: does any associated doc title contain the symbol name?
            // (In a future version, we would read the actual file content via VFS)
            let has_valid_doc = doc_ids.iter().any(|doc_id| {
                if let Some(doc) = doc_map.get(doc_id) {
                    let title = doc.title.to_lowercase();
                    let name = symbol.name.to_lowercase();

                    title.contains(&name) || name.contains(&title)
                } else {
                    false
                }
            });

            if has_valid_doc {
                AuditResult::Verified
            } else {
                AuditResult::Unverified
            }
        } else {
            // No documentation linked
            AuditResult::Unknown
        };

        audit_map.insert(symbol.symbol_id.clone(), result.as_str().to_string());
    }

    audit_map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::RepoSymbolKind;
    use crate::analyzers::records::{DocRecord, RelationKind, RelationRecord, SymbolRecord};
    use std::collections::BTreeMap;

    #[test]
    fn test_audit_symbols_verified() {
        let symbols = vec![SymbolRecord {
            repo_id: "test".to_string(),
            symbol_id: "sym1".to_string(),
            module_id: None,
            name: "solve_ode".to_string(),
            qualified_name: "solve_ode".to_string(),
            kind: RepoSymbolKind::Function,
            path: "src/main.jl".to_string(),
            line_start: None,
            line_end: None,
            signature: None,
            audit_status: None,
            verification_state: None,
            attributes: BTreeMap::new(),
        }];

        let docs = vec![DocRecord {
            repo_id: "test".to_string(),
            doc_id: "doc1".to_string(),
            title: "How to use solve_ode".to_string(),
            path: "docs/solve.md".to_string(),
            format: None,
        }];

        let relations = vec![RelationRecord {
            repo_id: "test".to_string(),
            source_id: "doc1".to_string(),
            target_id: "sym1".to_string(),
            kind: RelationKind::Documents,
        }];

        let results = audit_symbols(&symbols, &docs, &relations);
        assert_eq!(
            results
                .get("sym1")
                .unwrap_or_else(|| panic!("sym1 audit result should be present")),
            "verified"
        );
    }

    #[test]
    fn test_audit_symbols_unverified() {
        let symbols = vec![SymbolRecord {
            repo_id: "test".to_string(),
            symbol_id: "sym1".to_string(),
            module_id: None,
            name: "solve_ode".to_string(),
            qualified_name: "solve_ode".to_string(),
            kind: RepoSymbolKind::Function,
            path: "src/main.jl".to_string(),
            line_start: None,
            line_end: None,
            signature: None,
            audit_status: None,
            verification_state: None,
            attributes: BTreeMap::new(),
        }];

        let docs = vec![DocRecord {
            repo_id: "test".to_string(),
            doc_id: "doc1".to_string(),
            title: "General Tutorial".to_string(), // Mismatch
            path: "docs/tutorial.md".to_string(),
            format: None,
        }];

        let relations = vec![RelationRecord {
            repo_id: "test".to_string(),
            source_id: "doc1".to_string(),
            target_id: "sym1".to_string(),
            kind: RelationKind::Documents,
        }];

        let results = audit_symbols(&symbols, &docs, &relations);
        assert_eq!(
            results
                .get("sym1")
                .unwrap_or_else(|| panic!("sym1 audit result should be present")),
            "unverified"
        );
    }
}
