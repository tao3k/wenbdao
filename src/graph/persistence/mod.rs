//! JSON persistence: save, load, export, and dict-based parsing.

mod export;
mod parse;
mod save_load;

pub use parse::{entity_from_dict, relation_from_dict};
