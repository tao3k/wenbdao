use std::sync::Arc;

use super::BlockKindSpecifier;
use super::MarkdownBlockKind;
use super::compute_block_hash;

/// Block-level granularity for semantic addressing.
///
/// Each block represents a discrete content unit within a Markdown section,
/// enabling precise read/write operations at the paragraph, list, or code fence level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownBlock {
    /// Stable block identifier scoped to the parent section.
    ///
    /// Format: `block-{kind}-{index}` for anonymous blocks, or explicit `:ID:` value.
    /// Examples: `block-para-0`, `block-code-2`, `my-code-snippet`
    pub block_id: String,
    /// Block type variant.
    pub kind: MarkdownBlockKind,
    /// Byte range within the parent section content.
    ///
    /// `(start, end)` where `start` is inclusive and `end` is exclusive.
    /// These offsets are relative to the section text, not the document.
    pub byte_range: (usize, usize),
    /// Line range within the parent document (1-based, inclusive).
    ///
    /// `(start_line, end_line)` for precise line-level operations.
    pub line_range: (usize, usize),
    /// Content hash for self-healing (Blake3, truncated to 16 hex chars).
    ///
    /// Used to verify block identity when content may have shifted.
    pub content_hash: String,
    /// Raw block content including any formatting markers.
    ///
    /// For code fences, includes the fence markers and language tag.
    /// For lists, includes the list markers (-, *, 1.) on each line.
    pub content: Arc<str>,
    /// Optional explicit `:ID:` attribute from property drawer.
    ///
    /// When present, this takes precedence over the generated `block_id`.
    pub id: Option<String>,
    /// Structural path from document root to this block's parent section.
    ///
    /// Used for path-aware embedding and skeleton re-ranking.
    /// Enables semantic clustering by section context.
    /// Example: `["Architecture", "Storage", "Configuration"]`
    pub structural_path: Vec<String>,
}

impl MarkdownBlock {
    /// Create a new block with auto-generated ID.
    ///
    /// # Arguments
    ///
    /// * `kind` - The block type
    /// * `index` - The block index within its parent section
    /// * `byte_range` - Byte offsets within the section
    /// * `line_range` - Line numbers within the document
    /// * `content` - The raw block content
    /// * `structural_path` - Path from document root to parent section
    #[must_use]
    pub fn new(
        kind: MarkdownBlockKind,
        index: usize,
        byte_range: (usize, usize),
        line_range: (usize, usize),
        content: &str,
        structural_path: Vec<String>,
    ) -> Self {
        let block_id = format!("block-{}-{}", kind.id_prefix(), index);
        let content_hash = compute_block_hash(content);

        Self {
            block_id,
            kind,
            byte_range,
            line_range,
            content_hash,
            content: Arc::from(content),
            id: None,
            structural_path,
        }
    }

    /// Create a block with an explicit ID from a property drawer.
    #[must_use]
    pub fn with_explicit_id(mut self, id: String) -> Self {
        self.id = Some(id.clone());
        self.block_id = id;
        self
    }

    /// Check if this block matches a given kind specifier.
    ///
    /// Used for block path resolution like `/Section/Paragraph[2]`.
    #[must_use]
    pub fn matches_kind(&self, specifier: &BlockKindSpecifier) -> bool {
        match specifier {
            BlockKindSpecifier::Paragraph => self.kind == MarkdownBlockKind::Paragraph,
            BlockKindSpecifier::CodeFence => {
                matches!(self.kind, MarkdownBlockKind::CodeFence { .. })
            }
            BlockKindSpecifier::List => matches!(self.kind, MarkdownBlockKind::List { .. }),
            BlockKindSpecifier::BlockQuote => self.kind == MarkdownBlockKind::BlockQuote,
            BlockKindSpecifier::Item => false,
        }
    }

    /// Get the language for code fences, if applicable.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        match &self.kind {
            MarkdownBlockKind::CodeFence { language } => Some(language),
            _ => None,
        }
    }

    /// Check if this is a list block.
    #[must_use]
    pub fn is_list(&self) -> bool {
        matches!(self.kind, MarkdownBlockKind::List { .. })
    }

    /// Check if this is a code fence block.
    #[must_use]
    pub fn is_code(&self) -> bool {
        matches!(self.kind, MarkdownBlockKind::CodeFence { .. })
    }
}
