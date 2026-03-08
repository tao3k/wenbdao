use super::extract::collect_blocks;
use super::report::HmasValidationReport;
use crate::hmas::protocol::{
    HmasConclusionPayload, HmasDigitalThreadPayload, HmasEvidencePayload, HmasRecordKind,
    HmasSourceNode, HmasTaskPayload,
};
use std::collections::HashSet;
use std::path::Path;

fn has_empty_source_nodes(source_nodes: &[HmasSourceNode]) -> bool {
    source_nodes
        .iter()
        .any(|node| node.node_id.trim().is_empty())
}

fn validate_task_block(block: &super::extract::ExtractedBlock, report: &mut HmasValidationReport) {
    match serde_json::from_str::<HmasTaskPayload>(&block.json_payload) {
        Ok(payload) => {
            if payload.requirement_id.trim().is_empty() {
                report.push_issue(
                    block.line,
                    "missing_requirement_id",
                    "task.requirement_id must be non-empty",
                    Some(block.kind),
                );
            }
            if payload.hard_constraints.is_empty() {
                report.push_issue(
                    block.line,
                    "missing_hard_constraints",
                    "task.hard_constraints must be non-empty",
                    Some(block.kind),
                );
            }
        }
        Err(err) => report.push_issue(
            block.line,
            "invalid_json_payload",
            format!("failed to decode task payload: {err}"),
            Some(block.kind),
        ),
    }
}

fn validate_evidence_block(
    block: &super::extract::ExtractedBlock,
    report: &mut HmasValidationReport,
) {
    match serde_json::from_str::<HmasEvidencePayload>(&block.json_payload) {
        Ok(payload) => {
            if payload.requirement_id.trim().is_empty() {
                report.push_issue(
                    block.line,
                    "missing_requirement_id",
                    "evidence.requirement_id must be non-empty",
                    Some(block.kind),
                );
            }
        }
        Err(err) => report.push_issue(
            block.line,
            "invalid_json_payload",
            format!("failed to decode evidence payload: {err}"),
            Some(block.kind),
        ),
    }
}

fn validate_conclusion_block(
    block: &super::extract::ExtractedBlock,
    report: &mut HmasValidationReport,
    conclusion_requirements: &mut Vec<(String, usize)>,
) {
    match serde_json::from_str::<HmasConclusionPayload>(&block.json_payload) {
        Ok(payload) => {
            let requirement_id = payload.requirement_id.trim().to_string();
            if requirement_id.is_empty() {
                report.push_issue(
                    block.line,
                    "missing_requirement_id",
                    "conclusion.requirement_id must be non-empty",
                    Some(block.kind),
                );
            } else {
                conclusion_requirements.push((requirement_id, block.line));
            }
            if !(0.0..=1.0).contains(&payload.confidence_score) {
                report.push_issue(
                    block.line,
                    "invalid_confidence_score",
                    "conclusion.confidence_score must be between 0 and 1",
                    Some(block.kind),
                );
            }
        }
        Err(err) => report.push_issue(
            block.line,
            "invalid_json_payload",
            format!("failed to decode conclusion payload: {err}"),
            Some(block.kind),
        ),
    }
}

fn validate_digital_thread_block(
    block: &super::extract::ExtractedBlock,
    report: &mut HmasValidationReport,
    digital_thread_requirements: &mut HashSet<String>,
) {
    match serde_json::from_str::<HmasDigitalThreadPayload>(&block.json_payload) {
        Ok(payload) => {
            let requirement_id = payload.requirement_id.trim();
            if requirement_id.is_empty() {
                report.push_issue(
                    block.line,
                    "missing_requirement_id",
                    "digital_thread.requirement_id must be non-empty",
                    Some(block.kind),
                );
            } else {
                digital_thread_requirements.insert(requirement_id.to_string());
            }

            if payload.source_nodes_accessed.is_empty() {
                report.push_issue(
                    block.line,
                    "missing_source_nodes",
                    "digital_thread.source_nodes_accessed must be non-empty",
                    Some(block.kind),
                );
            } else if has_empty_source_nodes(&payload.source_nodes_accessed) {
                report.push_issue(
                    block.line,
                    "empty_source_node_id",
                    "digital_thread.source_nodes_accessed[*].node_id must be non-empty",
                    Some(block.kind),
                );
            }

            if payload.hard_constraints_checked.is_empty() {
                report.push_issue(
                    block.line,
                    "missing_constraints_checked",
                    "digital_thread.hard_constraints_checked must be non-empty",
                    Some(block.kind),
                );
            }
            if !(0.0..=1.0).contains(&payload.confidence_score) {
                report.push_issue(
                    block.line,
                    "invalid_confidence_score",
                    "digital_thread.confidence_score must be between 0 and 1",
                    Some(block.kind),
                );
            }
        }
        Err(err) => report.push_issue(
            block.line,
            "invalid_json_payload",
            format!("failed to decode digital_thread payload: {err}"),
            Some(block.kind),
        ),
    }
}

fn report_missing_digital_thread_requirements(
    report: &mut HmasValidationReport,
    conclusion_requirements: Vec<(String, usize)>,
    digital_thread_requirements: &HashSet<String>,
) {
    for (requirement_id, line) in conclusion_requirements {
        if !digital_thread_requirements.contains(&requirement_id) {
            report.push_issue(
                line,
                "missing_digital_thread",
                format!(
                    "conclusion requirement_id={requirement_id} has no matching digital_thread payload"
                ),
                Some(HmasRecordKind::Conclusion),
            );
        }
    }
}

/// Validate HMAS protocol blocks from markdown text.
///
/// This parser accepts heading-driven or fenced-tag driven HMAS JSON blocks and
/// enforces payload structure and cross-block consistency checks.
#[must_use]
pub fn validate_blackboard_markdown(markdown: &str) -> HmasValidationReport {
    let mut report = HmasValidationReport::ok();
    let blocks = collect_blocks(markdown, &mut report);

    let mut digital_thread_requirements = HashSet::new();
    let mut conclusion_requirements = Vec::new();

    for block in blocks {
        match block.kind {
            HmasRecordKind::Task => {
                report.task_count += 1;
                validate_task_block(&block, &mut report);
            }
            HmasRecordKind::Evidence => {
                report.evidence_count += 1;
                validate_evidence_block(&block, &mut report);
            }
            HmasRecordKind::Conclusion => {
                report.conclusion_count += 1;
                validate_conclusion_block(&block, &mut report, &mut conclusion_requirements);
            }
            HmasRecordKind::DigitalThread => {
                report.digital_thread_count += 1;
                validate_digital_thread_block(
                    &block,
                    &mut report,
                    &mut digital_thread_requirements,
                );
            }
        }
    }
    report_missing_digital_thread_requirements(
        &mut report,
        conclusion_requirements,
        &digital_thread_requirements,
    );
    report
}

/// Validate HMAS protocol blocks from a file path.
///
/// # Errors
///
/// Returns an error when the markdown file cannot be read.
pub fn validate_blackboard_file(path: &Path) -> Result<HmasValidationReport, String> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read blackboard markdown file {}: {err}",
            path.display()
        )
    })?;
    Ok(validate_blackboard_markdown(&content))
}
