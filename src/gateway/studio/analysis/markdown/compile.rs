use std::collections::HashMap;

use comrak::{
    Arena, Options,
    nodes::{AstNode, NodeValue},
    parse_document,
};

use super::text::collect_plain_text;
use crate::gateway::studio::types::{
    AnalysisEdge, AnalysisEdgeKind, AnalysisEvidence, AnalysisNode, AnalysisNodeKind,
};

pub(crate) struct CompiledDocument {
    pub(crate) document_hash: String,
    pub(crate) nodes: Vec<AnalysisNode>,
    pub(crate) edges: Vec<AnalysisEdge>,
    pub(crate) diagnostics: Vec<String>,
}

pub(crate) fn compile_markdown_ir(path: &str, content: &str) -> CompiledDocument {
    MarkdownCompiler::new(path, content).compile()
}

#[derive(Debug)]
struct EdgeDraft<'a> {
    kind: AnalysisEdgeKind,
    source_id: String,
    target_id: String,
    label: Option<String>,
    path: &'a str,
    line_start: usize,
    line_end: usize,
    confidence: f64,
}

struct MarkdownCompiler<'a> {
    path: &'a str,
    content: &'a str,
    nodes: Vec<AnalysisNode>,
    edges: Vec<AnalysisEdge>,
    diagnostics: Vec<String>,
    section_stack: Vec<(usize, String)>,
    task_chain: HashMap<String, String>,
    reference_nodes: HashMap<String, String>,
    node_contexts: HashMap<usize, String>,
    edge_seq: usize,
}

impl<'a> MarkdownCompiler<'a> {
    fn new(path: &'a str, content: &'a str) -> Self {
        Self {
            path,
            content,
            nodes: vec![AnalysisNode {
                id: "doc:0".to_string(),
                kind: AnalysisNodeKind::Document,
                label: path.to_string(),
                depth: 0,
                line_start: 1,
                line_end: content.lines().count().max(1),
                parent_id: None,
            }],
            edges: Vec::new(),
            diagnostics: Vec::new(),
            section_stack: Vec::new(),
            task_chain: HashMap::new(),
            reference_nodes: HashMap::new(),
            node_contexts: HashMap::new(),
            edge_seq: 1,
        }
    }

    fn compile(mut self) -> CompiledDocument {
        let arena = Arena::new();
        let root = parse_document(&arena, self.content, &markdown_options());

        for node in root.descendants() {
            match &node.data().value {
                NodeValue::Heading(heading) => {
                    self.handle_heading(node, usize::from(heading.level));
                }
                NodeValue::TaskItem(_) => self.handle_task_item(node),
                NodeValue::CodeBlock(block) => self.handle_code_block(node, block.info.as_str()),
                NodeValue::Link(link) => self.handle_reference_node(node, link.url.as_str()),
                NodeValue::WikiLink(link) => self.handle_reference_node(node, link.url.as_str()),
                _ => {}
            }
        }

        CompiledDocument {
            document_hash: blake3::hash(self.content.as_bytes()).to_hex().to_string(),
            nodes: self.nodes,
            edges: self.edges,
            diagnostics: self.diagnostics,
        }
    }

    fn handle_heading<'b>(&mut self, node: &'b AstNode<'b>, level: usize) {
        while self
            .section_stack
            .last()
            .is_some_and(|(active_level, _)| *active_level >= level)
        {
            self.section_stack.pop();
        }

        let line_no = line_start(node);
        let node_id = format!("sec:{line_no}");
        let label = collect_plain_text(node);
        let parent_id = current_section_context(&self.section_stack).to_string();

        self.nodes.push(AnalysisNode {
            id: node_id.clone(),
            kind: AnalysisNodeKind::Section,
            label,
            depth: level,
            line_start: line_no,
            line_end: line_no,
            parent_id: Some(parent_id.clone()),
        });
        self.node_contexts.insert(node_key(node), node_id.clone());
        self.push_edge(EdgeDraft {
            kind: AnalysisEdgeKind::Contains,
            source_id: parent_id,
            target_id: node_id.clone(),
            label: Some("contains".to_string()),
            path: self.path,
            line_start: line_no,
            line_end: line_no,
            confidence: 1.0,
        });
        self.section_stack.push((level, node_id));
    }

    fn handle_task_item<'b>(&mut self, node: &'b AstNode<'b>) {
        let line_no = line_start(node);
        let parent_id = self.parent_context(node);
        let node_id = format!("task:{line_no}");
        let label = collect_plain_text(node);
        let depth = self
            .section_stack
            .last()
            .map_or(1, |(level, _)| level.saturating_add(1));

        self.nodes.push(AnalysisNode {
            id: node_id.clone(),
            kind: AnalysisNodeKind::Task,
            label,
            depth,
            line_start: line_no,
            line_end: line_no,
            parent_id: Some(parent_id.clone()),
        });
        self.node_contexts.insert(node_key(node), node_id.clone());
        self.push_edge(EdgeDraft {
            kind: AnalysisEdgeKind::Contains,
            source_id: parent_id.clone(),
            target_id: node_id.clone(),
            label: Some("contains".to_string()),
            path: self.path,
            line_start: line_no,
            line_end: line_no,
            confidence: 1.0,
        });

        if let Some(prev_task) = self.task_chain.get(parent_id.as_str()) {
            self.push_edge(EdgeDraft {
                kind: AnalysisEdgeKind::NextStep,
                source_id: prev_task.clone(),
                target_id: node_id.clone(),
                label: Some("next".to_string()),
                path: self.path,
                line_start: line_no,
                line_end: line_no,
                confidence: 0.9,
            });
        }
        self.task_chain.insert(parent_id, node_id);
    }

    fn handle_code_block<'b>(&mut self, node: &'b AstNode<'b>, info: &str) {
        let line_start = line_start(node);
        let line_end = line_end(node);
        let node_id = format!("code:{line_start}");
        let parent_id = self.parent_context(node);
        let language = info.trim();
        let label = if language.is_empty() {
            "code block".to_string()
        } else {
            format!("code block ({language})")
        };
        let depth = self
            .section_stack
            .last()
            .map_or(1, |(level, _)| level.saturating_add(1));

        self.nodes.push(AnalysisNode {
            id: node_id.clone(),
            kind: AnalysisNodeKind::CodeBlock,
            label,
            depth,
            line_start,
            line_end,
            parent_id: Some(parent_id.clone()),
        });
        self.node_contexts.insert(node_key(node), node_id.clone());
        self.push_edge(EdgeDraft {
            kind: AnalysisEdgeKind::Contains,
            source_id: parent_id,
            target_id: node_id,
            label: Some("contains".to_string()),
            path: self.path,
            line_start,
            line_end,
            confidence: 1.0,
        });
    }

    fn handle_reference_node<'b>(&mut self, node: &'b AstNode<'b>, target: &str) {
        let normalized = normalize_reference(target);
        if normalized.is_empty() {
            return;
        }

        let line_no = line_start(node);
        let reference_id = self.reference_node_id(normalized.as_str(), line_no);
        let context_id = self.parent_context(node);
        self.push_edge(EdgeDraft {
            kind: AnalysisEdgeKind::References,
            source_id: context_id,
            target_id: reference_id,
            label: Some(normalized),
            path: self.path,
            line_start: line_no,
            line_end: line_no,
            confidence: 0.85,
        });
    }

    fn reference_node_id(&mut self, normalized: &str, line_no: usize) -> String {
        if let Some(existing) = self.reference_nodes.get(normalized) {
            return existing.clone();
        }

        let node_id = format!("ref:{}", slugify(normalized));
        self.nodes.push(AnalysisNode {
            id: node_id.clone(),
            kind: AnalysisNodeKind::Reference,
            label: normalized.to_string(),
            depth: 1,
            line_start: line_no,
            line_end: line_no,
            parent_id: None,
        });
        self.reference_nodes
            .insert(normalized.to_string(), node_id.clone());
        node_id
    }

    fn parent_context<'b>(&self, node: &'b AstNode<'b>) -> String {
        for ancestor in node.ancestors() {
            if let Some(id) = self.node_contexts.get(&node_key(ancestor)) {
                return id.clone();
            }
        }
        current_section_context(&self.section_stack).to_string()
    }

    fn push_edge(&mut self, draft: EdgeDraft<'_>) {
        self.edges.push(make_edge(self.edge_seq, draft));
        self.edge_seq += 1;
    }
}

fn markdown_options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.wikilinks_title_after_pipe = true;
    options.extension.tasklist = true;
    options
}

fn line_start<'a>(node: &'a AstNode<'a>) -> usize {
    source_line(node.data().sourcepos.start.line)
}

fn line_end<'a>(node: &'a AstNode<'a>) -> usize {
    source_line(node.data().sourcepos.end.line).max(line_start(node))
}

fn source_line(raw: usize) -> usize {
    raw.max(1)
}

fn current_section_context(section_stack: &[(usize, String)]) -> &str {
    section_stack
        .last()
        .map_or("doc:0", |(_, node_id)| node_id.as_str())
}

fn make_edge(edge_seq: usize, draft: EdgeDraft<'_>) -> AnalysisEdge {
    AnalysisEdge {
        id: format!("edge:{edge_seq}"),
        kind: draft.kind,
        source_id: draft.source_id,
        target_id: draft.target_id,
        label: draft.label,
        evidence: AnalysisEvidence {
            path: draft.path.to_string(),
            line_start: draft.line_start,
            line_end: draft.line_end,
            confidence: draft.confidence,
        },
    }
}

fn normalize_reference(target: &str) -> String {
    target.trim().trim_matches('#').to_string()
}

fn slugify(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            slug.push('-');
            prev_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn node_key<'a>(node: &'a AstNode<'a>) -> usize {
    std::ptr::from_ref(node) as usize
}
