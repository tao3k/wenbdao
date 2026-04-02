use std::fmt;

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

impl fmt::Display for MarkdownBlockKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Paragraph => write!(f, "Paragraph"),
            Self::CodeFence { language } => write!(f, "CodeFence({language})"),
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
