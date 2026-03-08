use super::ast::{heading_kind, node_line, parse_code_fence_info};
use super::report::HmasValidationReport;
use crate::hmas::protocol::HmasRecordKind;
use comrak::{Arena, Options, nodes::AstNode, nodes::NodeValue, parse_document};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExtractedBlock {
    pub(super) kind: HmasRecordKind,
    pub(super) line: usize,
    pub(super) json_payload: String,
}

pub(super) fn collect_blocks(
    markdown: &str,
    report: &mut HmasValidationReport,
) -> Vec<ExtractedBlock> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &Options::default());
    let mut blocks = Vec::new();
    let mut active_heading_kind: Option<HmasRecordKind> = None;

    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let mut children: Vec<&AstNode<'_>> = node.children().collect();
        children.reverse();
        stack.extend(children);

        match &node.data.borrow().value {
            NodeValue::Heading(_) => {
                active_heading_kind = heading_kind(node);
            }
            NodeValue::CodeBlock(block) => {
                let line = node_line(node);
                let info = block.info.trim().to_string();
                let payload = block.literal.clone();
                let (is_json, explicit_kind) = parse_code_fence_info(&info);

                let resolved_kind = match (active_heading_kind, explicit_kind) {
                    (Some(heading_kind), Some(fence_kind)) => {
                        if heading_kind != fence_kind {
                            report.push_issue(
                                line,
                                "fence_heading_kind_mismatch",
                                format!(
                                    "heading kind {} does not match fenced block kind {}",
                                    heading_kind.as_code(),
                                    fence_kind.as_code()
                                ),
                                Some(fence_kind),
                            );
                        }
                        Some(fence_kind)
                    }
                    (Some(heading_kind), None) => Some(heading_kind),
                    (None, Some(fence_kind)) => Some(fence_kind),
                    (None, None) => None,
                };

                let Some(kind) = resolved_kind else {
                    continue;
                };

                if !is_json {
                    report.push_issue(
                        line,
                        "unexpected_fence_language",
                        format!(
                            "{} block must use JSON fenced code block (`json` language)",
                            kind.as_code()
                        ),
                        Some(kind),
                    );
                    continue;
                }
                blocks.push(ExtractedBlock {
                    kind,
                    line,
                    json_payload: payload,
                });
            }
            _ => {}
        }
    }

    blocks
}
