use std::collections::HashMap;

use serde_json::json;
use xiuxian_testing::{
    ContractKnowledgeBatch, ContractKnowledgeDecision, ContractKnowledgeEnvelope,
};

use crate::{KnowledgeCategory, KnowledgeEntry};

/// Adapter that turns contract findings into Wendao-native knowledge entries.
#[derive(Debug, Default, Clone, Copy)]
pub struct WendaoContractFeedbackAdapter;

impl WendaoContractFeedbackAdapter {
    /// Convert one contract-knowledge envelope into a `KnowledgeEntry`.
    #[must_use]
    pub fn knowledge_entry_from_envelope(envelope: &ContractKnowledgeEnvelope) -> KnowledgeEntry {
        let mut entry = KnowledgeEntry::new(
            envelope.entry_id.clone(),
            envelope.title.clone(),
            envelope.content.clone(),
            Self::knowledge_category(envelope),
        )
        .with_tags(Self::entry_tags(envelope))
        .with_source(
            envelope
                .source_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
        );

        entry.metadata = Self::entry_metadata(envelope);
        entry
    }

    /// Convert one contract-knowledge batch into Wendao-native `KnowledgeEntry` records.
    #[must_use]
    pub fn knowledge_entries_from_batch(batch: &ContractKnowledgeBatch) -> Vec<KnowledgeEntry> {
        batch
            .entries
            .iter()
            .map(Self::knowledge_entry_from_envelope)
            .collect()
    }

    /// Return the Wendao category best aligned to the contract envelope.
    #[must_use]
    pub fn knowledge_category(envelope: &ContractKnowledgeEnvelope) -> KnowledgeCategory {
        if envelope.decision == ContractKnowledgeDecision::Fail {
            return KnowledgeCategory::Error;
        }

        match envelope.pack_id.as_str() {
            "modularity" => KnowledgeCategory::Architecture,
            "multi_role_audit" => KnowledgeCategory::Workflow,
            "rest_docs" => KnowledgeCategory::Reference,
            "knowledge_feedback" => KnowledgeCategory::Technique,
            _ => KnowledgeCategory::Note,
        }
    }

    fn entry_tags(envelope: &ContractKnowledgeEnvelope) -> Vec<String> {
        let mut tags = envelope.tags.clone();
        tags.push("contract_feedback".to_string());
        tags.push(format!("decision:{}", envelope.decision.as_str()));
        tags.push(format!(
            "category:{}",
            Self::knowledge_category(envelope).as_str()
        ));
        tags.sort();
        tags.dedup();
        tags
    }

    fn entry_metadata(envelope: &ContractKnowledgeEnvelope) -> HashMap<String, serde_json::Value> {
        let mut metadata: HashMap<String, serde_json::Value> = envelope
            .metadata
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        metadata.insert("entry_id".to_string(), json!(envelope.entry_id));
        metadata.insert("rule_id".to_string(), json!(envelope.rule_id));
        metadata.insert("pack_id".to_string(), json!(envelope.pack_id));
        metadata.insert("domain".to_string(), json!(envelope.domain));
        metadata.insert("decision".to_string(), json!(envelope.decision.as_str()));
        metadata.insert("severity".to_string(), json!(envelope.severity));
        metadata.insert("confidence".to_string(), json!(envelope.confidence));
        metadata.insert("summary".to_string(), json!(envelope.summary));
        metadata.insert("why_it_matters".to_string(), json!(envelope.why_it_matters));
        metadata.insert("remediation".to_string(), json!(envelope.remediation));
        metadata.insert("generated_at".to_string(), json!(envelope.generated_at));
        metadata.insert("suite_id".to_string(), json!(envelope.suite_id));
        if let Some(evidence_excerpt) = &envelope.evidence_excerpt {
            metadata.insert("evidence_excerpt".to_string(), json!(evidence_excerpt));
        }
        if let Some(good_example) = &envelope.good_example {
            metadata.insert("good_example".to_string(), json!(good_example));
        }
        if let Some(bad_example) = &envelope.bad_example {
            metadata.insert("bad_example".to_string(), json!(bad_example));
        }
        if let Some(source_path) = &envelope.source_path {
            metadata.insert(
                "source_path".to_string(),
                json!(source_path.to_string_lossy().into_owned()),
            );
        }
        metadata
    }
}

#[cfg(test)]
#[path = "../../tests/unit/contract_feedback/knowledge_entry.rs"]
mod tests;
