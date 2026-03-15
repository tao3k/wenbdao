//! Block-level granularity for semantic addressing.
//!
//! This module provides fine-grained block-level addressing for Markdown content,
//! enabling operations on specific paragraphs, code fences, lists, etc. within a section.
//!
//! ## Block Types
//!
//! - [`MarkdownBlock`] - Represents a single block element with byte ranges and content
//! - [`MarkdownBlockKind`] - The type variant of a block (Paragraph, CodeFence, List, etc.)
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::models::MarkdownBlock;
//!
//! // Blocks are typically created by the block parser
//! let blocks = extract_blocks(section_text, section_offset);
//!
//! // Access specific block types
//! for block in &blocks {
//!     match &block.kind {
//!         MarkdownBlockKind::CodeFence { language } => {
//!             println!("Code block in {}: {}", language, block.content);
//!         }
//!         MarkdownBlockKind::Paragraph => {
//!             println!("Paragraph: {}", block.content);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

use std::sync::Arc;

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

/// The type variant of a Markdown block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownBlockKind {
    /// Standard paragraph text.
    Paragraph,

    /// Fenced code block with optional language tag.
    CodeFence {
        /// Language identifier (e.g., "rust", "python", "toml").
        language: String,
    },

    /// Ordered or unordered list.
    List {
        /// Whether this is an ordered (numbered) list.
        ordered: bool,
    },

    /// Blockquote content.
    BlockQuote,

    /// Horizontal rule / thematic break.
    ThematicBreak,

    /// GitHub-flavored Markdown table.
    Table,

    /// Raw HTML block.
    HtmlBlock,
}

impl MarkdownBlockKind {
    /// Returns a short identifier string for this block kind.
    ///
    /// Used in auto-generated block IDs like `block-para-0`, `block-code-2`.
    #[must_use]
    pub fn id_prefix(&self) -> &'static str {
        match self {
            Self::Paragraph => "para",
            Self::CodeFence { .. } => "code",
            Self::List { ordered: true } => "olist",
            Self::List { ordered: false } => "ulist",
            Self::BlockQuote => "quote",
            Self::ThematicBreak => "hr",
            Self::Table => "table",
            Self::HtmlBlock => "html",
        }
    }

    /// Returns a human-readable name for this block kind.
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Paragraph => "Paragraph",
            Self::CodeFence { .. } => "Code Fence",
            Self::List { ordered: true } => "Ordered List",
            Self::List { ordered: false } => "Unordered List",
            Self::BlockQuote => "Block Quote",
            Self::ThematicBreak => "Thematic Break",
            Self::Table => "Table",
            Self::HtmlBlock => "HTML Block",
        }
    }
}

impl std::fmt::Display for MarkdownBlockKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paragraph => write!(f, "Paragraph"),
            Self::CodeFence { language } => write!(f, "CodeFence({})", language),
            Self::List { ordered } => {
                if *ordered {
                    write!(f, "OrderedList")
                } else {
                    write!(f, "UnorderedList")
                }
            }
            Self::BlockQuote => write!(f, "BlockQuote"),
            Self::ThematicBreak => write!(f, "ThematicBreak"),
            Self::Table => write!(f, "Table"),
            Self::HtmlBlock => write!(f, "HtmlBlock"),
        }
    }
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
    pub fn matches_kind(&self, specifier: &BlockKindSpecifier) -> bool {
        match specifier {
            BlockKindSpecifier::Paragraph => self.kind == MarkdownBlockKind::Paragraph,
            BlockKindSpecifier::CodeFence => {
                matches!(self.kind, MarkdownBlockKind::CodeFence { .. })
            }
            BlockKindSpecifier::List => matches!(self.kind, MarkdownBlockKind::List { .. }),
            BlockKindSpecifier::BlockQuote => self.kind == MarkdownBlockKind::BlockQuote,
            BlockKindSpecifier::Item => false, // Items are nested within List blocks
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

/// Block kind specifier for path-based addressing.
///
/// Used in block address paths like `/Section/Paragraph[2]` or `/Section/CodeFence[0]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockKindSpecifier {
    /// Match paragraph blocks.
    Paragraph,
    /// Match code fence blocks.
    CodeFence,
    /// Match list blocks (ordered or unordered).
    List,
    /// Match blockquote blocks.
    BlockQuote,
    /// Match list items within a list.
    Item,
}

impl BlockKindSpecifier {
    /// Parse a block kind specifier from a string.
    ///
    /// # Supported formats
    ///
    /// - `Paragraph` or `para` - Paragraph blocks
    /// - `CodeFence`, `Code`, or `code` - Code fence blocks
    /// - `List` or `list` - List blocks
    /// - `BlockQuote`, `Quote`, or `quote` - Blockquote blocks
    /// - `Item` or `item` - List items
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "paragraph" | "para" => Some(Self::Paragraph),
            "codefence" | "code" => Some(Self::CodeFence),
            "list" => Some(Self::List),
            "blockquote" | "quote" => Some(Self::BlockQuote),
            "item" => Some(Self::Item),
            _ => None,
        }
    }

    /// Get the canonical name for this specifier.
    #[must_use]
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Self::Paragraph => "Paragraph",
            Self::CodeFence => "CodeFence",
            Self::List => "List",
            Self::BlockQuote => "BlockQuote",
            Self::Item => "Item",
        }
    }
}

impl std::fmt::Display for BlockKindSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical_name())
    }
}

/// Address for a specific block within a section.
///
/// Used in conjunction with a section path to address blocks like:
/// `/Section/Paragraph[2]` or `/Section/List[1]/Item[3]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockAddress {
    /// The block kind to match.
    pub kind: BlockKindSpecifier,
    /// The index of the matching block (0-based).
    pub index: usize,
    /// For list items: the item index within the parent list.
    pub sub_index: Option<usize>,
}

impl BlockAddress {
    /// Create a new block address.
    #[must_use]
    pub fn new(kind: BlockKindSpecifier, index: usize) -> Self {
        Self {
            kind,
            index,
            sub_index: None,
        }
    }

    /// Create a block address for a list item.
    #[must_use]
    pub fn list_item(list_index: usize, item_index: usize) -> Self {
        Self {
            kind: BlockKindSpecifier::List,
            index: list_index,
            sub_index: Some(item_index),
        }
    }

    /// Parse a block address from a path component.
    ///
    /// # Formats
    ///
    /// - `Paragraph[2]` - Second paragraph in section
    /// - `CodeFence[0]` - First code fence
    /// - `List[1]/Item[3]` - Third item in second list
    ///
    /// Returns `(kind, index, sub_index)` tuple.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        // Try to parse "Kind[index]" format
        let open_bracket = s.find('[')?;
        let close_bracket = s.find(']')?;

        if close_bracket <= open_bracket {
            return None;
        }

        let kind_str = &s[..open_bracket];
        let index_str = &s[open_bracket + 1..close_bracket];

        let kind = BlockKindSpecifier::parse(kind_str)?;
        let index = index_str.parse().ok()?;

        // Check for sub-index after the closing bracket
        let rest = &s[close_bracket + 1..];
        let sub_index = if rest.starts_with("/Item[") {
            let item_open = rest.find('[')?;
            let item_close = rest.find(']')?;
            if item_close > item_open {
                rest[item_open + 1..item_close].parse().ok()
            } else {
                None
            }
        } else {
            None
        };

        Some(Self {
            kind,
            index,
            sub_index,
        })
    }

    /// Format as a path component string.
    #[must_use]
    pub fn to_path_component(&self) -> String {
        match self.sub_index {
            Some(item_idx) => format!(
                "{}[{}]/Item[{}]",
                self.kind.canonical_name(),
                self.index,
                item_idx
            ),
            None => format!("{}[{}]", self.kind.canonical_name(), self.index),
        }
    }
}

/// Compute Blake3 hash for block content (truncated to 16 hex chars).
pub fn compute_block_hash(content: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..16].to_string()
}

#[cfg(test)]
#[path = "../../../../tests/unit/link_graph/models/records/markdown_block.rs"]
mod tests;
