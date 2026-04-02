//! Markdown section parsing for link-graph indexing.

mod extract;
mod logbook;
mod properties;
mod types;

pub(crate) use extract::extract_sections;
pub use types::{LogbookEntry, ParsedSection};

#[cfg(test)]
pub(crate) use logbook::{extract_logbook_entries, parse_logbook_entry};
#[cfg(test)]
pub(crate) use properties::{extract_property_drawers, parse_property_drawer};

#[cfg(test)]
#[path = "../../../../tests/unit/link_graph/parser/sections.rs"]
mod tests;
