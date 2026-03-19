use std::fmt::Write;

use crate::link_graph::LinkGraphPlannedSearchPayload;

pub(crate) fn render_xml_lite(payload: &LinkGraphPlannedSearchPayload) -> String {
    let mut rendered = String::new();

    // Add CCS audit telemetry header if present
    if let Some(ref audit) = payload.ccs_audit {
        let status = if audit.compensated {
            "COMPENSATED"
        } else if audit.passed {
            "PASS"
        } else {
            "FAIL"
        };
        let _ = writeln!(
            rendered,
            "<ccs score=\"{:.2}\" status=\"{}\" missing=\"{}\"/>",
            audit.ccs_score,
            status,
            audit.missing_anchors.len()
        );
    }

    if let Some(ref semantic_ignition) = payload.semantic_ignition {
        let _ = writeln!(
            rendered,
            "<semantic_ignition backend=\"{}\" backend_name=\"{}\" contexts=\"{}\" error=\"{}\"/>",
            escape_xml_attr(&semantic_ignition.backend),
            escape_xml_attr(semantic_ignition.backend_name.as_deref().unwrap_or("")),
            semantic_ignition.context_count,
            escape_xml_attr(semantic_ignition.error.as_deref().unwrap_or("")),
        );
    }

    for hit in &payload.results {
        let _ = writeln!(
            rendered,
            "  <hit id=\"{}\" path=\"{}\" score=\"{:.4}\" type=\"graph\">{}</hit>",
            escape_xml_attr(&hit.stem),
            escape_xml_attr(&hit.path),
            hit.score,
            escape_xml_text(&hit.title),
        );
    }
    for context in &payload.quantum_contexts {
        let _ = writeln!(
            rendered,
            "  <hit id=\"{}\" path=\"{}\" score=\"{:.4}\" type=\"quantum\">{}</hit>",
            escape_xml_attr(&context.anchor_id),
            escape_xml_attr(&context.path),
            context.saliency_score,
            escape_xml_text(&context.doc_id),
        );
    }
    rendered
}

fn escape_xml_attr(input: &str) -> String {
    escape_xml(input, true)
}

fn escape_xml_text(input: &str) -> String {
    escape_xml(input, false)
}

fn escape_xml(input: &str, escape_quotes: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if escape_quotes => out.push_str("&quot;"),
            '\'' if escape_quotes => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}
