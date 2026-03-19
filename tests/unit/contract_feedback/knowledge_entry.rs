use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use tempfile::TempDir;
use xiuxian_testing::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, CollectionContext, ContractExecutionMode,
    ContractKnowledgeBatch, ContractKnowledgeDecision, ContractKnowledgeEnvelope, ContractReport,
    FindingConfidence, FindingSeverity, ModularityRulePack, RestDocsRulePack, RulePack,
};

use crate::{KnowledgeCategory, WendaoContractFeedbackAdapter};

fn must_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

fn rest_failure_envelope() -> ContractKnowledgeEnvelope {
    ContractKnowledgeEnvelope {
        entry_id: "wendao-contracts::rest_docs::REST-R003::/api/search".to_string(),
        suite_id: "wendao-contracts".to_string(),
        generated_at: "2026-03-17T00:00:00Z".to_string(),
        rule_id: "REST-R003".to_string(),
        pack_id: "rest_docs".to_string(),
        domain: "rest".to_string(),
        severity: FindingSeverity::Error,
        decision: ContractKnowledgeDecision::Fail,
        confidence: FindingConfidence::High,
        title: "[REST-R003] Incomplete response documentation".to_string(),
        content: "Summary: Missing error response description".to_string(),
        summary: "The endpoint is missing response descriptions.".to_string(),
        evidence_excerpt: Some(
            "Error response `500` is missing a non-empty description.".to_string(),
        ),
        why_it_matters: "Clients need explicit success and error coverage.".to_string(),
        remediation: "Document both success and error responses.".to_string(),
        good_example: Some(
            "Document `200` and `400` with short response descriptions.".to_string(),
        ),
        bad_example: Some("Expose statuses without any descriptions.".to_string()),
        source_path: Some(PathBuf::from(
            "packages/rust/crates/xiuxian-wendao/openapi.json",
        )),
        tags: vec![
            "contract_finding".to_string(),
            "domain:rest".to_string(),
            "pack:rest_docs".to_string(),
        ],
        metadata: BTreeMap::from([("trace_ids".to_string(), serde_json::json!(["trace-42"]))]),
    }
}

fn openapi_artifact(content: serde_json::Value) -> CollectedArtifact {
    CollectedArtifact {
        id: "wendao-openapi".to_string(),
        kind: ArtifactKind::OpenApiDocument,
        path: Some(PathBuf::from(
            "packages/rust/crates/xiuxian-wendao/openapi.json",
        )),
        content,
        labels: BTreeMap::new(),
    }
}

fn generated_rest_docs_batch() -> ContractKnowledgeBatch {
    let artifacts = CollectedArtifacts {
        artifacts: vec![openapi_artifact(json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Gateway",
                "version": "v1"
            },
            "paths": {
                "/documents": {
                    "post": {
                        "summary": "Create a document",
                        "responses": {
                            "201": {
                                "description": "Document created."
                            },
                            "400": {
                                "description": "Invalid request."
                            }
                        },
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "title": {
                                                "type": "string"
                                            },
                                            "body": {
                                                "type": "string"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }))],
        metadata: BTreeMap::new(),
    };

    let findings = must_ok(
        RestDocsRulePack.evaluate(&artifacts),
        "rest_docs pack should evaluate for downstream consumer coverage",
    );
    let report = ContractReport::from_findings(
        "wendao-contracts",
        ContractExecutionMode::Strict,
        "2026-03-18T00:00:00Z",
        findings,
    );
    ContractKnowledgeBatch::from_report(&report)
}

fn crate_src_root(temp_dir: &TempDir, crate_name: &str) -> PathBuf {
    temp_dir
        .path()
        .join("packages")
        .join("rust")
        .join("crates")
        .join(crate_name)
        .join("src")
}

fn write_rust_file(src_root: &Path, relative_path: &str, content: &str) {
    let path = src_root.join(relative_path);
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("target file should have parent: {}", path.display()));
    must_ok(
        fs::create_dir_all(parent),
        "should create parent directories for fixture source file",
    );
    must_ok(
        fs::write(&path, content),
        "should write fixture Rust source file",
    );
}

fn generated_modularity_batch() -> ContractKnowledgeBatch {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "xiuxian-wendao");
    write_rust_file(
        &src_root,
        "internal/state.rs",
        r"
pub struct InternalState {
    value: usize,
}
",
    );

    let ctx = CollectionContext {
        suite_id: "wendao-contracts".to_string(),
        crate_name: Some("xiuxian-wendao".to_string()),
        workspace_root: Some(temp_dir.path().to_path_buf()),
        labels: BTreeMap::new(),
    };
    let pack = ModularityRulePack;
    let artifacts = must_ok(
        pack.collect(&ctx),
        "modularity pack should collect fixture source files",
    );
    let findings = must_ok(
        pack.evaluate(&artifacts),
        "modularity pack should evaluate fixture source files",
    );
    let report = ContractReport::from_findings(
        "wendao-contracts",
        ContractExecutionMode::Strict,
        "2026-03-18T00:00:00Z",
        findings,
    );
    ContractKnowledgeBatch::from_report(&report)
}

#[test]
fn contract_feedback_adapter_maps_failure_to_error_entry() {
    let envelope = rest_failure_envelope();

    let entry = WendaoContractFeedbackAdapter::knowledge_entry_from_envelope(&envelope);

    assert_eq!(entry.id, envelope.entry_id);
    assert_eq!(entry.title, envelope.title);
    assert_eq!(entry.content, envelope.content);
    assert_eq!(entry.category, KnowledgeCategory::Error);
    assert_eq!(
        entry.source.as_deref(),
        Some("packages/rust/crates/xiuxian-wendao/openapi.json")
    );
    assert!(entry.tags.iter().any(|tag| tag == "contract_feedback"));
    assert!(entry.tags.iter().any(|tag| tag == "decision:fail"));
    assert!(entry.tags.iter().any(|tag| tag == "category:error"));
    assert!(
        entry.metadata.get("evidence_excerpt").is_some_and(
            |value| value == "Error response `500` is missing a non-empty description."
        )
    );
    assert!(
        entry
            .metadata
            .get("trace_ids")
            .is_some_and(|value| value == &serde_json::json!(["trace-42"]))
    );
}

#[test]
fn contract_feedback_adapter_uses_pack_specific_non_failure_categories() {
    let mut envelope = rest_failure_envelope();
    envelope.decision = ContractKnowledgeDecision::Warn;
    envelope.severity = FindingSeverity::Warning;
    envelope.pack_id = "modularity".to_string();
    envelope.domain = "architecture".to_string();

    let category = WendaoContractFeedbackAdapter::knowledge_category(&envelope);

    assert_eq!(category, KnowledgeCategory::Architecture);
}

#[test]
fn contract_feedback_adapter_maps_batches_to_entries() {
    let first = rest_failure_envelope();
    let mut second = rest_failure_envelope();
    second.entry_id = "wendao-contracts::multi_role_audit::AUDIT-R003::global".to_string();
    second.rule_id = "AUDIT-R003".to_string();
    second.pack_id = "multi_role_audit".to_string();
    second.domain = "audit".to_string();
    second.decision = ContractKnowledgeDecision::Warn;
    second.severity = FindingSeverity::Warning;
    second.title = "[AUDIT-R003] Runtime drift warning".to_string();

    let batch = ContractKnowledgeBatch {
        suite_id: "wendao-contracts".to_string(),
        generated_at: "2026-03-17T00:00:00Z".to_string(),
        entries: vec![first, second],
    };

    let entries = WendaoContractFeedbackAdapter::knowledge_entries_from_batch(&batch);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].category, KnowledgeCategory::Error);
    assert_eq!(entries[1].category, KnowledgeCategory::Workflow);
}

#[test]
fn contract_feedback_adapter_maps_generated_rest_docs_batch_to_reference_entries() {
    let batch = generated_rest_docs_batch();

    assert_eq!(batch.entries.len(), 1);
    assert_eq!(batch.entries[0].pack_id, "rest_docs");
    assert_eq!(batch.entries[0].decision, ContractKnowledgeDecision::Warn);

    let entries = WendaoContractFeedbackAdapter::knowledge_entries_from_batch(&batch);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, KnowledgeCategory::Reference);
    assert!(entries[0].tags.iter().any(|tag| tag == "pack:rest_docs"));
    assert!(
        entries[0]
            .tags
            .iter()
            .any(|tag| tag == "category:reference")
    );
    assert!(
        entries[0]
            .metadata
            .get("rule_id")
            .is_some_and(|value| value == "REST-R007")
    );
}

#[test]
fn contract_feedback_adapter_maps_generated_modularity_batch_to_architecture_entries() {
    let batch = generated_modularity_batch();

    assert_eq!(batch.entries.len(), 1);
    assert_eq!(batch.entries[0].pack_id, "modularity");
    assert_eq!(batch.entries[0].decision, ContractKnowledgeDecision::Warn);

    let entries = WendaoContractFeedbackAdapter::knowledge_entries_from_batch(&batch);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].category, KnowledgeCategory::Architecture);
    assert!(entries[0].tags.iter().any(|tag| tag == "pack:modularity"));
    assert!(
        entries[0]
            .tags
            .iter()
            .any(|tag| tag == "category:architecture")
    );
    assert!(
        entries[0]
            .metadata
            .get("rule_id")
            .is_some_and(|value| value == "MOD-R002")
    );
}
