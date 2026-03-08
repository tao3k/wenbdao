use crate::hmas::protocol::HmasRecordKind;
use comrak::nodes::{AstNode, NodeValue};

pub(super) fn node_line(node: &AstNode<'_>) -> usize {
    let line = node.data.borrow().sourcepos.start.line;
    if line == 0 { 1 } else { line }
}

fn push_text_from_node<'a>(node: &'a AstNode<'a>, out: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(value) => out.push_str(value),
        NodeValue::Code(value) => out.push_str(&value.literal),
        NodeValue::SoftBreak | NodeValue::LineBreak => out.push(' '),
        _ => {
            for child in node.children() {
                push_text_from_node(child, out);
            }
        }
    }
}

pub(super) fn heading_kind<'a>(node: &'a AstNode<'a>) -> Option<HmasRecordKind> {
    let mut out = String::new();
    for child in node.children() {
        push_text_from_node(child, &mut out);
    }
    HmasRecordKind::from_heading_text(&out)
}

pub(super) fn parse_code_fence_info(info: &str) -> (bool, Option<HmasRecordKind>) {
    let mut tokens = info.split_whitespace();
    let Some(language) = tokens.next() else {
        return (false, None);
    };
    let is_json = language.eq_ignore_ascii_case("json");
    let fence_kind = tokens.next().and_then(HmasRecordKind::from_fence_tag);
    (is_json, fence_kind)
}
