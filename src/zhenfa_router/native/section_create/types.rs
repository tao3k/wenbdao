/// Information about a sibling section for context.
#[derive(Debug, Clone)]
pub(crate) struct SiblingInfo {
    /// Title of the sibling section.
    pub(crate) title: String,
    /// First line of content (truncated for context).
    pub(crate) preview: String,
}

/// Information about where to insert new sections.
#[derive(Debug, Clone)]
pub(crate) struct InsertionInfo {
    /// Byte offset where new content should be inserted.
    pub(crate) insertion_byte: usize,
    /// Starting heading level for new sections (1-6).
    pub(crate) start_level: usize,
    /// Path components that still need to be created.
    pub(crate) remaining_path: Vec<String>,
    /// Previous sibling section (if any) for narrative context.
    pub(crate) prev_sibling: Option<SiblingInfo>,
    /// Next sibling section (if any) for narrative context.
    pub(crate) next_sibling: Option<SiblingInfo>,
}

/// Options for building new section content.
#[derive(Debug, Clone, Default)]
pub(crate) struct BuildSectionOptions {
    /// If true, generate a `:ID: <uuid>` property drawer for each new section.
    pub(crate) generate_id: bool,
    /// Custom ID prefix (e.g., "arch" -> ":ID: arch-uuid").
    pub(crate) id_prefix: Option<String>,
}

impl Default for InsertionInfo {
    fn default() -> Self {
        Self {
            insertion_byte: 0,
            start_level: 1,
            remaining_path: Vec::new(),
            prev_sibling: None,
            next_sibling: None,
        }
    }
}
