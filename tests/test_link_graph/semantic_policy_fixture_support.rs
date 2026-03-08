use serde_json::{Value, json};
use xiuxian_wendao::ParsedLinkGraphQuery;
use xiuxian_wendao::link_graph::LinkGraphPlannedSearchPayload;
use xiuxian_wendao::{LinkGraphSemanticDocumentScope, LinkGraphSemanticSearchPolicy};

use crate::fixture_json_assertions::assert_json_fixture_eq;
use crate::link_graph_fixture_tree::materialize_link_graph_fixture;

pub(super) struct SemanticPolicyFixture {
    _temp_dir: tempfile::TempDir,
    root: std::path::PathBuf,
}

impl SemanticPolicyFixture {
    pub(super) fn build(scenario: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = materialize_link_graph_fixture(&format!(
            "link_graph/semantic_policy/{scenario}/input"
        ))?;
        let root = temp_dir.path().to_path_buf();
        Ok(Self {
            _temp_dir: temp_dir,
            root,
        })
    }

    pub(super) fn build_index(
        &self,
    ) -> Result<xiuxian_wendao::LinkGraphIndex, Box<dyn std::error::Error>> {
        xiuxian_wendao::LinkGraphIndex::build(self.root.as_path())
            .map_err(|error| error.clone().into())
    }
}

pub(super) fn assert_semantic_policy_fixture(scenario: &str, relative: &str, actual: &Value) {
    assert_json_fixture_eq(
        &format!("link_graph/semantic_policy/{scenario}/expected"),
        relative,
        actual,
    );
}

pub(super) fn parsed_semantic_policy_snapshot(parsed: &ParsedLinkGraphQuery) -> Value {
    json!({
        "query": parsed.query,
        "semantic_policy": semantic_policy_snapshot(&parsed.options.semantic_policy),
    })
}

pub(super) fn planned_payload_semantic_policy_snapshot(
    payload: &LinkGraphPlannedSearchPayload,
) -> Value {
    json!({
        "payload_options": semantic_policy_snapshot(&payload.options.semantic_policy),
        "retrieval_plan_policy": payload
            .retrieval_plan
            .as_ref()
            .map(|plan| semantic_policy_snapshot(&plan.semantic_policy)),
    })
}

fn semantic_policy_snapshot(policy: &LinkGraphSemanticSearchPolicy) -> Value {
    json!({
        "document_scope": semantic_document_scope_label(policy.document_scope),
        "min_vector_score": policy.min_vector_score,
    })
}

fn semantic_document_scope_label(scope: LinkGraphSemanticDocumentScope) -> &'static str {
    match scope {
        LinkGraphSemanticDocumentScope::All => "all",
        LinkGraphSemanticDocumentScope::SummaryOnly => "summary_only",
    }
}
