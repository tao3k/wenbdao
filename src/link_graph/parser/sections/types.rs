use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::super::code_observation::CodeObservation;

/// Execution log entry from `:LOGBOOK:` drawer (Blueprint v2.4).
///
/// Represents a single entry in the execution log for workflow tracking.
/// Format: `- [TIMESTAMP] MESSAGE`
///
/// # Example
///
/// ```markdown
/// :LOGBOOK:
/// - [2025-03-14] Agent Started: Initiating structural audit.
/// - [2025-03-14] Step [audit] completed with status OK.
/// :END:
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogbookEntry {
    /// Timestamp of the log entry (e.g., "2025-03-14").
    pub timestamp: String,
    /// The log message content.
    pub message: String,
    /// 1-based line number within the document.
    pub line_number: usize,
}

/// Parsed section row for section-aware retrieval and `HippoRAG 2` `Passage Nodes`.
#[derive(Debug, Clone)]
pub struct ParsedSection {
    /// Leaf heading title for this section.
    pub heading_title: String,
    /// Slash-delimited heading ancestry for this section.
    pub heading_path: String,
    /// Lower-cased `heading_path` for case-insensitive matching.
    pub heading_path_lower: String,
    /// Markdown heading depth for this section.
    pub heading_level: usize,
    /// Inclusive 1-based start line within the markdown body.
    pub line_start: usize,
    /// Inclusive 1-based end line within the markdown body.
    pub line_end: usize,
    /// Byte offset from start of document where this section begins.
    pub byte_start: usize,
    /// Byte offset (exclusive) where this section ends.
    pub byte_end: usize,
    /// Content contained by this section.
    pub section_text: String,
    /// Lower-cased section text for case-insensitive matching.
    pub section_text_lower: String,
    /// List of entity IDs mentioned in this specific section.
    pub entities: Vec<String>,
    /// Property drawer attributes extracted from heading (e.g., :ID: arch-v1).
    pub attributes: HashMap<String, String>,
    /// Execution log entries from `:LOGBOOK:` drawer (Blueprint v2.4).
    pub logbook: Vec<LogbookEntry>,
    /// Code observations from `:OBSERVE:` property drawer (Blueprint v2.7).
    pub observations: Vec<CodeObservation>,
}
