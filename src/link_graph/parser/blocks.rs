//! Block-level parser for Markdown content using comrak AST.
//!
//! This module provides functions to extract fine-grained block elements
//! (paragraphs, code fences, lists, etc.) from Markdown section text.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::parser::blocks::extract_blocks;
//!
//! let section_text = r#"
//! This is a paragraph.
//!
//! ```rust
//! fn main() {}
//! ```
//!
//! - Item 1
//! - Item 2
//! "#;
//!
//! let blocks = extract_blocks(section_text, 0, 1);
//! // blocks[0] = Paragraph
//! // blocks[1] = CodeFence { language: "rust" }
//! // blocks[2] = List { ordered: false }
//! ```

use crate::link_graph::models::{MarkdownBlock, MarkdownBlockKind};
use comrak::{Arena, Options, nodes::AstNode, nodes::NodeValue, parse_document};

/// Extract block-level elements from Markdown section text.
///
/// # Arguments
///
/// * `section_text` - The raw Markdown content of a section
/// * `section_byte_offset` - Byte offset of the section within the document
/// * `section_line_offset` - Starting line number of the section (1-based)
/// * `structural_path` - Path from document root to this section
///
/// # Returns
///
/// A vector of `MarkdownBlock` instances representing the top-level block elements.
/// Nested content (like list items) is included within parent blocks.
#[must_use]
pub fn extract_blocks(
    section_text: &str,
    section_byte_offset: usize,
    section_line_offset: usize,
    structural_path: &[String],
) -> Vec<MarkdownBlock> {
    let arena = Arena::new();
    let root = parse_document(&arena, section_text, &Options::default());

    let mut blocks = Vec::new();
    let mut block_indices: BlockIndexCounter = Default::default();

    for node in root.children() {
        if let Some(block) = node_to_block(
            node,
            section_text,
            section_byte_offset,
            section_line_offset,
            &mut block_indices,
            structural_path,
        ) {
            blocks.push(block);
        }
    }

    blocks
}

/// Counter for generating unique block indices by kind.
#[derive(Default)]
struct BlockIndexCounter {
    para: usize,
    code: usize,
    ulist: usize,
    olist: usize,
    quote: usize,
    hr: usize,
    table: usize,
    html: usize,
}

impl BlockIndexCounter {
    fn next(&mut self, kind: &MarkdownBlockKind) -> usize {
        match kind {
            MarkdownBlockKind::Paragraph => {
                let idx = self.para;
                self.para += 1;
                idx
            }
            MarkdownBlockKind::CodeFence { .. } => {
                let idx = self.code;
                self.code += 1;
                idx
            }
            MarkdownBlockKind::List { ordered: true } => {
                let idx = self.olist;
                self.olist += 1;
                idx
            }
            MarkdownBlockKind::List { ordered: false } => {
                let idx = self.ulist;
                self.ulist += 1;
                idx
            }
            MarkdownBlockKind::BlockQuote => {
                let idx = self.quote;
                self.quote += 1;
                idx
            }
            MarkdownBlockKind::ThematicBreak => {
                let idx = self.hr;
                self.hr += 1;
                idx
            }
            MarkdownBlockKind::Table => {
                let idx = self.table;
                self.table += 1;
                idx
            }
            MarkdownBlockKind::HtmlBlock => {
                let idx = self.html;
                self.html += 1;
                idx
            }
        }
    }
}

/// Convert a comrak AST node to a MarkdownBlock.
pub(super) fn node_to_block(
    node: &AstNode<'_>,
    section_text: &str,
    section_byte_offset: usize,
    section_line_offset: usize,
    block_indices: &mut BlockIndexCounter,
    structural_path: &[String],
) -> Option<MarkdownBlock> {
    let ast = node.data.borrow();
    let sourcepos = ast.sourcepos;

    // Calculate byte range from source position
    let start_line = sourcepos.start.line.max(0) as usize;
    let start_col = sourcepos.start.column.max(0) as usize;
    let end_line = sourcepos.end.line.max(0) as usize;
    let end_col = sourcepos.end.column.max(0) as usize;

    // Convert line/column to byte offsets within section_text
    let byte_range =
        line_col_to_byte_range(section_text, start_line, start_col, end_line, end_col)?;

    // Calculate document-relative line range (1-based)
    let doc_line_range = (
        section_line_offset
            .saturating_add(start_line)
            .saturating_sub(1)
            .max(1),
        section_line_offset
            .saturating_add(end_line)
            .saturating_sub(1)
            .max(1),
    );

    // Extract content
    let content = if byte_range.0 <= byte_range.1 && byte_range.1 <= section_text.len() {
        &section_text[byte_range.0..byte_range.1]
    } else {
        return None;
    };

    // Skip empty content
    if content.trim().is_empty() {
        return None;
    }

    // Map node value to block kind
    let kind = match &ast.value {
        NodeValue::Paragraph => MarkdownBlockKind::Paragraph,
        NodeValue::CodeBlock(block) => {
            let language = block.info.trim().to_string();
            MarkdownBlockKind::CodeFence { language }
        }
        NodeValue::List(list) => {
            let ordered = list.list_type == comrak::nodes::ListType::Ordered;
            MarkdownBlockKind::List { ordered }
        }
        NodeValue::BlockQuote => MarkdownBlockKind::BlockQuote,
        NodeValue::ThematicBreak => MarkdownBlockKind::ThematicBreak,
        NodeValue::Table(_) => MarkdownBlockKind::Table,
        NodeValue::HtmlBlock(_) => MarkdownBlockKind::HtmlBlock,
        // Skip headings (handled at section level) and other inline elements
        NodeValue::Heading(_)
        | NodeValue::Document
        | NodeValue::FrontMatter(_) => {
            return None;
        }
        // Skip inline elements that shouldn't be standalone blocks
        NodeValue::Text(_)
        | NodeValue::SoftBreak
        | NodeValue::LineBreak
        | NodeValue::Code(_)
        | NodeValue::Emph
        | NodeValue::Strong
        | NodeValue::Strikethrough
        | NodeValue::Superscript
        | NodeValue::Link(_)
        | NodeValue::Image(_)
        | NodeValue::FootnoteDefinition(_)
        | NodeValue::FootnoteReference(_)
        | NodeValue::DescriptionList
        | NodeValue::DescriptionItem(_)
        | NodeValue::DescriptionTerm
        | NodeValue::DescriptionDetails
        | NodeValue::Math(_)
        | NodeValue::Escaped
        | NodeValue::MultilineBlockQuote(_)
        | NodeValue::EscapedTag(_)
        | NodeValue::Raw(_)
        | NodeValue::Underline
        | NodeValue::Subscript
        | NodeValue::SpoileredText
        // Additional inline/table elements that aren't standalone blocks
        | NodeValue::Item(_)
        | NodeValue::TableRow(_)
        | NodeValue::TableCell
        | NodeValue::TaskItem(_)
        | NodeValue::HtmlInline(_)
        | NodeValue::Highlight
        | NodeValue::WikiLink(_)
        | NodeValue::Alert(_)
        | NodeValue::Subtext => {
            return None;
        }
    };

    let index = block_indices.next(&kind);
    let block = MarkdownBlock::new(
        kind,
        index,
        (
            byte_range.0 + section_byte_offset,
            byte_range.1 + section_byte_offset,
        ),
        doc_line_range,
        content,
        structural_path.to_vec(),
    );

    Some(block)
}

/// Convert line/column positions to byte range within text.
///
/// Comrak uses 1-based line and column numbers.
pub(super) fn line_col_to_byte_range(
    text: &str,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
) -> Option<(usize, usize)> {
    let mut current_line = 1;

    // Find start byte
    for (byte_idx, ch) in text.char_indices() {
        if current_line == start_line {
            // Found start line - calculate column offset
            let col_offset = if start_col > 0 { start_col - 1 } else { 0 };
            let start_byte = byte_idx
                + text[byte_idx..]
                    .char_indices()
                    .nth(col_offset)
                    .map(|(i, _)| i)
                    .unwrap_or(0);

            // Now find end byte
            if start_line == end_line {
                // Same line - find end column
                let end_col_offset = if end_col > 0 { end_col } else { 1 };
                let end_byte = start_byte
                    + text[start_byte..]
                        .char_indices()
                        .nth(end_col_offset)
                        .map(|(i, _)| i)
                        .unwrap_or(text[start_byte..].len());
                return Some((start_byte, end_byte));
            }

            // Different end line - find it
            let mut remaining = &text[start_byte..];
            let mut current = start_line;
            let mut end_byte = start_byte;

            while current < end_line {
                if let Some(newline_pos) = remaining.find('\n') {
                    end_byte += newline_pos + 1;
                    remaining = &remaining[newline_pos + 1..];
                    current += 1;
                } else {
                    // No more newlines, end is at the end of text
                    return Some((start_byte, text.len()));
                }
            }

            // Now at end_line, find end column
            let end_col_offset = if end_col > 0 { end_col } else { 1 };
            let col_byte = remaining
                .char_indices()
                .nth(end_col_offset)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
            end_byte += col_byte;

            return Some((start_byte, end_byte));
        }

        if ch == '\n' {
            current_line += 1;
        }
    }

    // Handle case where start_line is at end of text
    if current_line == start_line && start_line == end_line {
        let start_byte = text.len();
        return Some((start_byte, text.len()));
    }

    None
}

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/parser/blocks.rs"]
mod tests;
