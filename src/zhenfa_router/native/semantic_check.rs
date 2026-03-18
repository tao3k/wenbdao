//! Semantic Check Tool (Blueprint v2.0 Section 3: Project Sentinel).
//!
//! This module implements the "Semantic Sentinel" concept:
//! - Dead link detection: Scan all `[[id]]` references and verify against global ID index
//! - Status sentinel: Report references to DEPRECATED nodes
//! - Contract validation: Check `:CONTRACT:` constraints
//! - Code observation validation: Check `:OBSERVE:` patterns (Blueprint v2.7)
//! - Fuzzy pattern suggestion: Suggest fixes for invalid patterns (Blueprint v2.9)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use xiuxian_zhenfa::{ZhenfaContext, ZhenfaError, zhenfa_tool};

use crate::link_graph::parser::CodeObservation;
use crate::link_graph::{PageIndexNode, RegistryBuildResult, RegistryIndex};

use super::WendaoContextExt;
use super::audit::{FuzzySuggestion, SourceFile, suggest_pattern_fix_with_threshold};

/// Standard property drawer attribute keys (Blueprint v2.0).
mod attrs {
    /// Explicit node identifier - takes precedence over `structural_path`.
    pub const ID: &str = "ID";
    /// Node status: STABLE | DRAFT | DEPRECATED.
    pub const STATUS: &str = "STATUS";
    /// Semantic contract constraint (e.g., `must_contain("Rust", "Lock")`).
    pub const CONTRACT: &str = "CONTRACT";
}

/// Node status values (Blueprint v2.0 Section 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum NodeStatus {
    /// Node is stable and can be safely referenced.
    #[default]
    Stable,
    /// Node is a draft, may change without notice.
    Draft,
    /// Node is deprecated, references should be updated.
    Deprecated,
}

impl NodeStatus {
    /// Parse status from string.
    fn from_str(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "DRAFT" => Self::Draft,
            "DEPRECATED" => Self::Deprecated,
            _ => Self::Stable,
        }
    }
}

/// Arguments for semantic check tool.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WendaoSemanticCheckArgs {
    /// Document stem or ID to check (optional, checks all docs if not specified).
    #[serde(default)]
    pub doc: Option<String>,
    /// Check types to run (default: all).
    #[serde(default)]
    pub checks: Option<Vec<CheckType>>,
    /// Include warnings in addition to errors.
    #[serde(default)]
    pub include_warnings: Option<bool>,
    /// Source file paths to scan for fuzzy pattern suggestions (Blueprint v2.9).
    /// When provided, invalid `:OBSERVE:` patterns will get suggested fixes.
    #[serde(default)]
    pub source_paths: Option<Vec<String>>,
    /// Minimum confidence threshold for fuzzy pattern suggestions (0.0 - 1.0).
    /// Default is 0.65. Lower values will suggest more matches, higher values will be more strict.
    #[serde(default)]
    pub fuzzy_confidence_threshold: Option<f32>,
}

/// Types of semantic checks available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
pub enum CheckType {
    /// Check for dead links (references to non-existent IDs).
    DeadLinks,
    /// Check for references to deprecated nodes.
    DeprecatedRefs,
    /// Validate :CONTRACT: constraints.
    Contracts,
    /// Check for ID collisions (same ID in multiple locations).
    IdCollisions,
    /// Check hash alignment (`expect_hash` vs actual `content_hash`).
    HashAlignment,
    /// Check for missing mandatory :ID: property drawer (Blueprint v2.2).
    MissingIdentity,
    /// Check for legacy syntax markers (Blueprint v2.2).
    LegacySyntax,
    /// Validate :OBSERVE: code patterns using xiuxian-ast (Blueprint v2.7).
    CodeObservations,
}

/// A reference with an optional expected hash.
#[derive(Debug, Clone)]
struct HashReference {
    /// Target ID (without # prefix).
    target_id: String,
    /// Expected content hash (if specified via @hash suffix).
    expect_hash: Option<String>,
}

/// Result of a semantic check operation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SemanticCheckResult {
    /// Overall status: "pass", "warning", or "fail".
    pub status: String,
    /// Total issues found.
    pub issue_count: usize,
    /// List of issues found.
    pub issues: Vec<SemanticIssue>,
    /// Summary message.
    pub summary: String,
    /// Per-document audit reports with health scores.
    pub file_reports: Vec<FileAuditReport>,
}

/// Per-document audit report with health score (Blueprint v2.2).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileAuditReport {
    /// Document path.
    pub path: String,
    /// Health score (0-100, where 100 is perfect).
    pub health_score: u8,
    /// Number of errors in this document.
    pub error_count: usize,
    /// Number of warnings in this document.
    pub warning_count: usize,
}

/// A single semantic issue found during check.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SemanticIssue {
    /// Issue severity: "error", "warning", or "info".
    pub severity: String,
    /// Issue type: "`dead_link`", "`deprecated_ref`", "`contract_violation`".
    pub issue_type: String,
    /// Document where the issue was found.
    pub doc: String,
    /// Node ID where the issue was found.
    pub node_id: String,
    /// Human-readable description.
    pub message: String,
    /// Optional location information.
    pub location: Option<IssueLocation>,
    /// Suggested fix (if available).
    pub suggestion: Option<String>,
    /// Structured fuzzy suggestion for code observation patterns (Blueprint v2.9).
    pub fuzzy_suggestion: Option<FuzzySuggestionData>,
}

/// Structured fuzzy suggestion data for XML output (Blueprint v2.9).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FuzzySuggestionData {
    /// The original invalid pattern.
    pub original_pattern: String,
    /// The suggested updated pattern.
    pub suggested_pattern: String,
    /// Similarity score (0.0 - 1.0).
    pub confidence: f32,
    /// Source location where match was found.
    pub source_location: Option<String>,
    /// Ready-to-use replacement drawer content.
    pub replacement_drawer: String,
}

impl FuzzySuggestionData {
    /// Create from a raw fuzzy suggestion and the original pattern.
    fn from_suggestion(s: FuzzySuggestion, original: String) -> Self {
        Self {
            original_pattern: original,
            suggested_pattern: s.suggested_pattern,
            confidence: s.confidence,
            source_location: s.source_location,
            replacement_drawer: s.replacement_drawer,
        }
    }
}

/// Location information for an issue.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IssueLocation {
    /// Line number (1-based).
    pub line: usize,
    /// Heading path.
    pub heading_path: String,
    /// Byte range (start, end) for precise AST-level mutations.
    pub byte_range: Option<(usize, usize)>,
}

impl IssueLocation {
    /// Create an `IssueLocation` from a `PageIndexNode`'s metadata.
    fn from_node(node: &PageIndexNode) -> Self {
        Self {
            line: node.metadata.line_range.0,
            heading_path: node.metadata.structural_path.join(" / "),
            byte_range: node.metadata.byte_range,
        }
    }
}

/// Perform semantic consistency check on the knowledge base.
///
/// This tool implements the "Semantic Sentinel" concept from Blueprint v2.0:
/// - **Dead link detection**: Automatically scan `[[id]]` references and verify
///   against the global ID index.
/// - **Status sentinel**: Report nodes that reference `:STATUS: DEPRECATED` blocks.
/// - **Contract validation**: Validate `:CONTRACT:` attribute constraints.
///
/// Returns an XML-Lite report of all issues found.
///
/// # Errors
///
/// Returns `ZhenfaError` when the link graph index cannot be loaded or when the
/// underlying audit core cannot complete.
#[allow(clippy::needless_pass_by_value)] // The tool macro keeps owned args for tool invocation wiring.
#[allow(missing_docs)]
#[zhenfa_tool(
    name = "wendao.semantic_check",
    description = "Perform semantic consistency check on the knowledge base (dead links, deprecated refs, contract violations).",
    tool_struct = "WendaoSemanticCheckTool",
    mutation_scope = "wendao.semantic_check"
)]
pub fn wendao_semantic_check(
    ctx: &ZhenfaContext,
    args: WendaoSemanticCheckArgs,
) -> Result<String, ZhenfaError> {
    let (issues, file_contents) = run_audit_core(ctx, &args)?;
    let docs_checked_count = file_contents.len();

    // Build result
    let error_count = issues.iter().filter(|i| i.severity == "error").count();
    let warning_count = issues.iter().filter(|i| i.severity == "warning").count();

    let status = if error_count > 0 {
        "fail"
    } else if warning_count > 0 {
        "warning"
    } else {
        "pass"
    };

    let summary = format!(
        "Found {error_count} errors and {warning_count} warnings across {docs_checked_count} documents"
    );

    // Build per-file reports with health scores
    let docs_list: Vec<String> = file_contents.keys().cloned().collect();
    let file_reports = build_file_reports(&issues, &docs_list);

    let result = SemanticCheckResult {
        status: status.to_string(),
        issue_count: issues.len(),
        issues,
        summary,
        file_reports,
    };

    // Format as XML-Lite
    Ok(format_result_as_xml(&result))
}

/// Run the core audit logic and return raw issues and file contents.
///
/// This allows the CLI and other tools to reuse the auditing logic without XML formatting.
///
/// # Errors
///
/// Returns `ZhenfaError` when the link graph index cannot be queried.
pub fn run_audit_core(
    ctx: &ZhenfaContext,
    args: &WendaoSemanticCheckArgs,
) -> Result<(Vec<SemanticIssue>, HashMap<String, String>), ZhenfaError> {
    let index = ctx.link_graph_index()?;
    let include_warnings = args.include_warnings.unwrap_or(true);

    // Collect file contents for auditing and hashing
    let mut file_contents = HashMap::new();

    // Determine which checks to run
    let checks = args.checks.clone().unwrap_or_else(|| {
        vec![
            CheckType::DeadLinks,
            CheckType::DeprecatedRefs,
            CheckType::Contracts,
            CheckType::IdCollisions,
            CheckType::HashAlignment,
            CheckType::MissingIdentity,
            CheckType::LegacySyntax,
            CheckType::CodeObservations,
        ]
    });

    // Resolve source files for fuzzy suggestions (Blueprint v2.9)
    let source_files: Vec<SourceFile> = if let Some(ref paths) = args.source_paths {
        let path_refs: Vec<&std::path::Path> = paths.iter().map(std::path::Path::new).collect();
        // Try to resolve for all supported languages
        let mut files = Vec::new();
        for lang in [
            xiuxian_ast::Lang::Rust,
            xiuxian_ast::Lang::Python,
            xiuxian_ast::Lang::TypeScript,
            xiuxian_ast::Lang::JavaScript,
            xiuxian_ast::Lang::Go,
        ] {
            files.extend(super::audit::resolve_source_files(&path_refs, lang));
        }
        files
    } else {
        Vec::new()
    };

    // Build ID registry with collision detection
    let build_result = index.build_registry_index_with_collisions();

    // Collect all issues
    let mut issues = Vec::new();

    // Check for ID collisions first (before moving registry out)
    if checks.contains(&CheckType::IdCollisions) {
        check_id_collisions(&build_result, &mut issues);
    }

    // Extract registry for other checks
    let registry = build_result.registry;

    // Get all trees from the index
    let trees = index.all_page_index_trees();

    // Get documents to check
    let docs_to_check: Vec<String> = if let Some(doc) = &args.doc {
        if doc == "." || doc.is_empty() {
            // Global audit
            trees.keys().cloned().collect()
        } else {
            // Check if it's an exact ID or path
            if trees.contains_key(doc) {
                vec![doc.clone()]
            } else {
                // Try fuzzy matching the path if exact ID not found
                trees.keys().filter(|k| k.contains(doc)).cloned().collect()
            }
        }
    } else {
        trees.keys().cloned().collect()
    };

    // Get fuzzy confidence threshold
    let fuzzy_threshold = args.fuzzy_confidence_threshold;

    for doc_id in &docs_to_check {
        // Load content for hash/context check
        if let Ok(content) = std::fs::read_to_string(doc_id) {
            file_contents.insert(doc_id.clone(), content);
        }

        if let Some(doc_trees) = trees.get(doc_id) {
            let audit_pass = AuditPass {
                doc_id,
                registry: &registry,
                checks: &checks,
                include_warnings,
                source_files: &source_files,
                fuzzy_threshold,
            };
            for root in doc_trees {
                check_node(root, &audit_pass, &mut issues);
            }
        }
    }

    Ok((issues, file_contents))
}

struct AuditPass<'a> {
    doc_id: &'a str,
    registry: &'a RegistryIndex,
    checks: &'a [CheckType],
    include_warnings: bool,
    source_files: &'a [SourceFile],
    fuzzy_threshold: Option<f32>,
}

/// Check a single node and its children for semantic issues.
fn check_node(node: &PageIndexNode, audit_pass: &AuditPass<'_>, issues: &mut Vec<SemanticIssue>) {
    // Check this node
    if audit_pass.checks.contains(&CheckType::DeadLinks) {
        check_dead_links(node, audit_pass.doc_id, audit_pass.registry, issues);
    }

    if audit_pass.checks.contains(&CheckType::DeprecatedRefs) && audit_pass.include_warnings {
        check_deprecated_refs(node, audit_pass.doc_id, audit_pass.registry, issues);
    }

    if audit_pass.checks.contains(&CheckType::Contracts) {
        check_contracts(node, audit_pass.doc_id, issues);
    }

    if audit_pass.checks.contains(&CheckType::HashAlignment) {
        check_hash_alignment(node, audit_pass.doc_id, audit_pass.registry, issues);
    }

    if audit_pass.checks.contains(&CheckType::MissingIdentity) && audit_pass.include_warnings {
        check_missing_identity(node, audit_pass.doc_id, issues);
    }

    if audit_pass.checks.contains(&CheckType::LegacySyntax) && audit_pass.include_warnings {
        check_legacy_syntax(node, audit_pass.doc_id, issues);
    }

    if audit_pass.checks.contains(&CheckType::CodeObservations) {
        check_code_observations(
            node,
            audit_pass.doc_id,
            audit_pass.source_files,
            audit_pass.fuzzy_threshold,
            issues,
        );
    }

    // Recurse into children
    for child in &node.children {
        check_node(child, audit_pass, issues);
    }
}

/// Extract ID references from text content.
///
/// Looks for wiki-style links like `[[#id]]` or `[[id]]`.
#[allow(clippy::expect_used)]
fn extract_id_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next(); // consume second '['
            let mut link_content = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next(); // consume first ']'
                    if chars.peek() == Some(&']') {
                        chars.next(); // consume second ']'
                        break;
                    }
                    link_content.push(']');
                } else {
                    // SAFETY: We just peeked and know there's a character
                    link_content.push(chars.next().expect("char exists after peek"));
                }
            }
            // Extract ID from link content (may start with # or be a path)
            let link = link_content.trim();
            if link.starts_with('#') {
                refs.push(link.to_string());
            }
        }
    }
    refs
}

/// Extract hash-annotated references from text content.
///
/// Format: `[[#id@hash]]` where @hash is the expected content hash.
///
/// # Example
///
/// - `[[#arch-v1@abc123]]` -> `HashReference` { `target_id`: "arch-v1", `expect_hash`: Some("abc123") }
/// - `[[#intro]]` -> `HashReference` { `target_id`: "intro", `expect_hash`: None }
#[allow(clippy::expect_used)]
fn extract_hash_references(text: &str) -> Vec<HashReference> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next(); // consume second '['
            let mut link_content = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next(); // consume first ']'
                    if chars.peek() == Some(&']') {
                        chars.next(); // consume second ']'
                        break;
                    }
                    link_content.push(']');
                } else {
                    // SAFETY: We just peeked and know there's a character
                    link_content.push(chars.next().expect("char exists after peek"));
                }
            }
            // Parse link content for #id[@hash] format
            let link = link_content.trim();
            if let Some(id_part) = link.strip_prefix('#') {
                // Check for @hash suffix
                if let Some(at_pos) = id_part.find('@') {
                    let target_id = id_part[..at_pos].to_string();
                    let expect_hash = id_part[at_pos + 1..].to_string();
                    refs.push(HashReference {
                        target_id,
                        expect_hash: Some(expect_hash),
                    });
                } else {
                    refs.push(HashReference {
                        target_id: id_part.to_string(),
                        expect_hash: None,
                    });
                }
            }
        }
    }
    refs
}

/// Check for dead links (references to non-existent IDs).
fn check_dead_links(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    // Extract ID references from node text
    let id_refs = extract_id_references(&node.text);

    for entity in id_refs {
        // entity is like "#id-name"
        let target_id = &entity[1..]; // Remove the '#' prefix
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
fn check_deprecated_refs(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let id_refs = extract_id_references(&node.text);

    for entity in id_refs {
        let target_id = &entity[1..]; // Remove the '#' prefix
        if let Some(indexed) = registry.get(target_id) {
            // Check if target is deprecated
            if let Some(status_str) = indexed.node.metadata.attributes.get(attrs::STATUS)
                && NodeStatus::from_str(status_str) == NodeStatus::Deprecated
            {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: "deprecated_ref".to_string(),
                    doc: doc_id.to_string(),
                    node_id: node.node_id.clone(),
                    message: format!(
                        "Reference to deprecated node '{target_id}' (status: DEPRECATED)"
                    ),
                    location: Some(IssueLocation::from_node(node)),
                    suggestion: Some(format!(
                        "Update reference from deprecated node '{target_id}' to its replacement"
                    )),
                    fuzzy_suggestion: None,
                });
            }
        }
    }
}

/// Check contract constraints.
fn check_contracts(node: &PageIndexNode, doc_id: &str, issues: &mut Vec<SemanticIssue>) {
    // Check if this node has a CONTRACT attribute
    if let Some(contract) = node.metadata.attributes.get(attrs::CONTRACT) {
        // Get content from node text
        let content = &node.text;

        // Parse and validate contract
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

/// Check hash alignment (`expect_hash` vs actual `content_hash`).
///
/// Scans for references with hash annotations like `[[#id@abc123]]` and verifies
/// that the target's current `content_hash` matches the expected hash.
fn check_hash_alignment(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    // Extract hash-annotated references from node text
    let hash_refs = extract_hash_references(&node.text);

    for hash_ref in hash_refs {
        // Only check references that have an expect_hash
        if let Some(expect_hash) = &hash_ref.expect_hash {
            // Look up the target in the registry
            if let Some(indexed) = registry.get(&hash_ref.target_id) {
                // Compare expected hash with actual content_hash
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
                    // Target exists but has no content_hash
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
            // Note: If target doesn't exist, that's already caught by dead_link check
        }
    }
}

/// Check for ID collisions (same ID in multiple documents).
fn check_id_collisions(build_result: &RegistryBuildResult, issues: &mut Vec<SemanticIssue>) {
    for collision in &build_result.collisions {
        // Format the location list for the message
        let locations_str = collision
            .locations
            .iter()
            .map(|(doc_id, path)| format!("{}:{}", doc_id, path.join("/")))
            .collect::<Vec<_>>()
            .join(", ");

        // Use the first location as the primary doc for the issue
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
                line: 0, // Line not applicable for global collision
                heading_path: primary_path.join(" / "),
                byte_range: None, // No byte range for global collision
            }),
            suggestion: Some(
                "Rename one of the nodes to have a unique ID, or remove duplicate :ID: attributes"
                    .to_string(),
            ),
            fuzzy_suggestion: None,
        });
    }
}

/// Check for missing mandatory :ID: property drawer (Blueprint v2.2).
///
/// Reports headings that should have an :ID: attribute but don't.
/// Top-level headings (level 1) are considered mandatory to have IDs.
fn check_missing_identity(node: &PageIndexNode, doc_id: &str, issues: &mut Vec<SemanticIssue>) {
    // Check if this node should have an ID
    // Heuristic: Level 1 and 2 headings should have explicit IDs for stable anchoring
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
///
/// Detects deprecated patterns like "SEE ALSO", "RELATED TO" as plain text
/// instead of using proper wiki-links `[[...]]`.
fn check_legacy_syntax(node: &PageIndexNode, doc_id: &str, issues: &mut Vec<SemanticIssue>) {
    let text = &node.text;

    // Legacy markers to detect
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

fn push_invalid_observation_language_issue(
    node: &PageIndexNode,
    doc_id: &str,
    obs: &CodeObservation,
    issues: &mut Vec<SemanticIssue>,
) {
    issues.push(SemanticIssue {
        severity: "error".to_string(),
        issue_type: "invalid_observation_language".to_string(),
        doc: doc_id.to_string(),
        node_id: node.node_id.clone(),
        message: format!(
            "Unsupported language '{}' in :OBSERVE: pattern",
            obs.language
        ),
        location: Some(IssueLocation::from_node(node)),
        suggestion: Some(
            "Use a supported language: rust, python, javascript, typescript, go, java, c, cpp, etc.".to_string()
        ),
        fuzzy_suggestion: None,
    });
}

fn build_observation_fuzzy_suggestion(
    obs: &CodeObservation,
    lang: xiuxian_ast::Lang,
    source_files: &[SourceFile],
    fuzzy_threshold: Option<f32>,
) -> Option<FuzzySuggestionData> {
    if source_files.is_empty() {
        return None;
    }

    suggest_pattern_fix_with_threshold(&obs.pattern, lang, source_files, fuzzy_threshold)
        .map(|suggestion| FuzzySuggestionData::from_suggestion(suggestion, obs.pattern.clone()))
}

fn format_observation_source_location(source_location: Option<&str>) -> String {
    source_location.map_or_else(String::new, |location| {
        format!("Found similar code at: {location}")
    })
}

fn format_observation_suggestion(
    pattern: &str,
    description: &str,
    fuzzy_suggestion_data: Option<&FuzzySuggestionData>,
    fallback: &str,
) -> String {
    if let Some(data) = fuzzy_suggestion_data {
        format!(
            "Pattern '{pattern}' {description} {}\nConfidence: {:.0}%\n{}",
            data.suggested_pattern,
            data.confidence * 100.0,
            format_observation_source_location(data.source_location.as_deref())
        )
    } else {
        fallback.to_string()
    }
}

fn count_observation_matches(
    obs: &CodeObservation,
    lang: xiuxian_ast::Lang,
    source_files: &[SourceFile],
) -> usize {
    source_files
        .iter()
        .filter_map(|file| {
            let file_path = Path::new(&file.path);
            xiuxian_ast::Lang::from_path(file_path)
                .filter(|file_lang| *file_lang == lang)
                .and_then(|_| xiuxian_ast::scan(&file.content, &obs.pattern, lang).ok())
                .map(|matches| matches.len())
        })
        .sum()
}

/// Check :OBSERVE: code patterns for validity using xiuxian-ast (Blueprint v2.7).
///
/// Validates that all `:OBSERVE:` property drawer entries have:
/// 1. Valid language identifiers supported by xiuxian-ast
/// 2. Syntactically valid sgrep/ast-grep patterns
///
/// When validation fails and source files are provided, attempts fuzzy
/// pattern suggestion to find renamed symbols (Blueprint v2.9).
fn check_code_observations(
    node: &PageIndexNode,
    doc_id: &str,
    source_files: &[SourceFile],
    fuzzy_threshold: Option<f32>,
    issues: &mut Vec<SemanticIssue>,
) {
    // Check all observations in this node's metadata
    for obs in &node.metadata.observations {
        let Some(lang) = obs.ast_language() else {
            push_invalid_observation_language_issue(node, doc_id, obs, issues);
            continue;
        };

        // Validate the pattern syntax
        if let Err(error) = obs.validate_pattern() {
            let fuzzy_suggestion_data =
                build_observation_fuzzy_suggestion(obs, lang, source_files, fuzzy_threshold);
            let suggestion_text = format_observation_suggestion(
                &obs.pattern,
                "is invalid. Consider updating to:",
                fuzzy_suggestion_data.as_ref(),
                "Fix the pattern syntax or check xiuxian-ast documentation for valid sgrep patterns",
            );

            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: "invalid_observation_pattern".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Invalid sgrep pattern in :OBSERVE:: {error}"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(suggestion_text),
                fuzzy_suggestion: fuzzy_suggestion_data,
            });
            continue;
        }

        if source_files.is_empty() || count_observation_matches(obs, lang, source_files) > 0 {
            continue;
        }

        let fuzzy_suggestion_data =
            build_observation_fuzzy_suggestion(obs, lang, source_files, fuzzy_threshold);
        let suggestion_text = format_observation_suggestion(
            &obs.pattern,
            "found no matches. Best fuzzy match:",
            fuzzy_suggestion_data.as_ref(),
            "The code may have been renamed, moved, or deleted. No similar code patterns found.",
        );

        issues.push(SemanticIssue {
            severity: "warning".to_string(),
            issue_type: "observation_target_missing".to_string(),
            doc: doc_id.to_string(),
            node_id: node.node_id.clone(),
            message: format!(
                "Observation pattern '{}' found no matches in source files",
                obs.pattern
            ),
            location: Some(IssueLocation::from_node(node)),
            suggestion: Some(suggestion_text),
            fuzzy_suggestion: fuzzy_suggestion_data,
        });
    }
}

/// Generate a suggested ID from a title.
fn generate_suggested_id(title: &str) -> String {
    // Convert to lowercase, replace spaces with hyphens, remove special chars
    title
        .to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        .trim_matches('-')
        .to_string()
}

/// Build per-file audit reports with health scores.
fn build_file_reports(issues: &[SemanticIssue], docs: &[String]) -> Vec<FileAuditReport> {
    let mut reports = Vec::new();

    for doc_id in docs {
        let doc_issues: Vec<_> = issues.iter().filter(|i| &i.doc == doc_id).collect();
        let error_count = doc_issues.iter().filter(|i| i.severity == "error").count();
        let warning_count = doc_issues
            .iter()
            .filter(|i| i.severity == "warning")
            .count();

        // Health score calculation:
        // Start at 100, subtract 20 for each error, 5 for each warning
        // Minimum score is 0
        let health_score = (100u8)
            .saturating_sub(issue_penalty(error_count, 20))
            .saturating_sub(issue_penalty(warning_count, 5));

        reports.push(FileAuditReport {
            path: doc_id.clone(),
            health_score,
            error_count,
            warning_count,
        });
    }

    reports
}

/// Validate a contract expression against content.
///
/// Supported contract formats:
/// - `must_contain("term1", "term2", ...)` - Content must contain all specified terms
/// - `must_not_contain("term")` - Content must not contain the specified term
/// - `min_length(N)` - Content must have at least N characters
fn validate_contract(contract: &str, content: &str) -> Option<String> {
    let contract = contract.trim();

    // must_contain("term1", "term2", ...)
    if let Some(args) = extract_function_args(contract, "must_contain") {
        let terms: Vec<&str> = args
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim())
            .filter(|s| !s.is_empty())
            .collect();

        for term in terms {
            if !content.contains(term) {
                return Some(format!("missing required term '{term}'"));
            }
        }
        return None;
    }

    // must_not_contain("term")
    if let Some(args) = extract_function_args(contract, "must_not_contain") {
        let term = args.trim().trim_matches('"').trim();
        if content.contains(term) {
            return Some(format!("contains forbidden term '{term}'"));
        }
        return None;
    }

    // min_length(N)
    if let Some(args) = extract_function_args(contract, "min_length") {
        if let Ok(min_len) = args.trim().parse::<usize>()
            && content.len() < min_len
        {
            return Some(format!(
                "content length {} is less than required {}",
                content.len(),
                min_len
            ));
        }
        return None;
    }

    // Unknown contract type - skip validation
    None
}

fn issue_penalty(count: usize, weight: u8) -> u8 {
    u8::try_from(count).map_or(u8::MAX, |count| count.saturating_mul(weight))
}

/// Extract arguments from a function-like contract expression.
fn extract_function_args<'a>(contract: &'a str, function_name: &str) -> Option<&'a str> {
    let prefix = format!("{function_name}(");
    if contract.starts_with(&prefix) && contract.ends_with(')') {
        Some(&contract[prefix.len()..contract.len() - 1])
    } else {
        None
    }
}

fn file_health_status(health_score: u8) -> &'static str {
    if health_score >= 80 {
        "HEALTHY"
    } else if health_score >= 50 {
        "DEGRADED"
    } else {
        "UNHEALTHY"
    }
}

fn append_file_reports_xml(output: &mut String, file_reports: &[FileAuditReport]) {
    if file_reports.is_empty() {
        return;
    }

    output.push_str("  <files>\n");
    for file_report in file_reports {
        let _ = writeln!(
            output,
            "    <file path=\"{}\" health=\"{}\" score=\"{}\">",
            xml_escape(&file_report.path),
            file_health_status(file_report.health_score),
            file_report.health_score
        );
        let _ = writeln!(output, "      <errors>{}</errors>", file_report.error_count);
        let _ = writeln!(
            output,
            "      <warnings>{}</warnings>",
            file_report.warning_count
        );
        output.push_str("    </file>\n");
    }
    output.push_str("  </files>\n");
}

fn append_issue_location_xml(output: &mut String, location: &IssueLocation) {
    let byte_range_attr = if let Some((start, end)) = location.byte_range {
        format!(" byte_start=\"{start}\" byte_end=\"{end}\"")
    } else {
        String::new()
    };
    let _ = writeln!(
        output,
        "      <location line=\"{}\" path=\"{}\"{}/>",
        location.line,
        xml_escape(&location.heading_path),
        byte_range_attr
    );
}

fn append_fuzzy_suggestion_xml(output: &mut String, fuzzy: &FuzzySuggestionData) {
    output.push_str("      <fuzzy_suggestion>\n");
    let _ = writeln!(
        output,
        "        <text>Pattern '{}' found with {:.0}% similarity.</text>",
        xml_escape(&fuzzy.suggested_pattern),
        fuzzy.confidence * 100.0
    );
    let _ = writeln!(
        output,
        "        <replacement_drawer>{}</replacement_drawer>",
        xml_escape(&fuzzy.replacement_drawer)
    );
    let _ = writeln!(
        output,
        "        <confidence>{:.2}</confidence>",
        fuzzy.confidence
    );
    if let Some(ref location) = fuzzy.source_location {
        let _ = writeln!(
            output,
            "        <source_location>{}</source_location>",
            xml_escape(location)
        );
    }
    output.push_str("      </fuzzy_suggestion>\n");
}

fn append_issue_xml(output: &mut String, issue: &SemanticIssue) {
    let _ = writeln!(
        output,
        "    <issue severity=\"{}\" code=\"{}\">",
        issue.severity.to_uppercase(),
        issue_type_to_code(&issue.issue_type)
    );
    let _ = writeln!(
        output,
        "      <message>{}</message>",
        xml_escape(&issue.message)
    );
    let _ = writeln!(output, "      <doc>{}</doc>", xml_escape(&issue.doc));
    let _ = writeln!(
        output,
        "      <node_id>{}</node_id>",
        xml_escape(&issue.node_id)
    );
    if let Some(ref location) = issue.location {
        append_issue_location_xml(output, location);
    }
    if let Some(ref suggestion) = issue.suggestion {
        let _ = writeln!(
            output,
            "      <suggestion>{}</suggestion>",
            xml_escape(suggestion)
        );
    }
    if let Some(ref fuzzy) = issue.fuzzy_suggestion {
        append_fuzzy_suggestion_xml(output, fuzzy);
    }
    output.push_str("    </issue>\n");
}

fn append_issues_xml(output: &mut String, issues: &[SemanticIssue]) {
    if issues.is_empty() {
        return;
    }

    output.push_str("  <issues>\n");
    for issue in issues {
        append_issue_xml(output, issue);
    }
    output.push_str("  </issues>\n");
}

/// Format the check result as XML-Lite (Blueprint v2.2).
fn format_result_as_xml(result: &SemanticCheckResult) -> String {
    let mut output = String::new();

    let _ = writeln!(
        output,
        "<wendao_audit_report version=\"2.9\" engine=\"anchoR-sentinel\" status=\"{}\" issue_count=\"{}\">",
        result.status, result.issue_count
    );

    let _ = writeln!(output, "  <summary>{}</summary>", result.summary);
    append_file_reports_xml(&mut output, &result.file_reports);
    append_issues_xml(&mut output, &result.issues);

    output.push_str("</wendao_audit_report>\n");
    output
}

/// Convert issue type to Blueprint diagnostic code.
fn issue_type_to_code(issue_type: &str) -> &'static str {
    match issue_type {
        "dead_link" => "ERR_DEAD_LINK",
        "deprecated_ref" => "WARN_DEPRECATED_REF",
        "contract_violation" => "ERR_CONTRACT_VIOLATION",
        "id_collision" => "ERR_DUPLICATE_ID",
        "content_drift" => "WARN_CONTENT_DRIFT",
        "missing_content_hash" => "INFO_MISSING_HASH",
        "missing_identity" => "ERR_MISSING_IDENTITY",
        "legacy_syntax" => "WARN_LEGACY_SYNTAX",
        "invalid_observation_language" => "ERR_INVALID_OBSERVER_LANG",
        "invalid_observation_pattern" => "ERR_INVALID_OBSERVER_PATTERN",
        _ => "UNKNOWN",
    }
}

/// Escape special XML characters.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
#[path = "../../../tests/unit/semantic_check_tests.rs"]
mod tests;
