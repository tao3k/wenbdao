//! Markdown note parsing for link-graph indexing.

#[path = "api.rs"]
mod api;
pub mod blocks;
pub mod code_observation;
mod content;
mod links;
mod paths;
mod sections;
mod time;
mod types;

pub use self::api::parse_note;
pub use self::blocks::extract_blocks;
pub use self::code_observation::{CodeObservation, extract_observations};
pub use self::paths::{is_supported_note, normalize_alias};
pub use self::sections::{LogbookEntry, ParsedSection};
pub use self::types::ParsedNote;
