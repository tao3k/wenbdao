//! Scenario runners for different test categories.
//!
//! Each runner implements `ScenarioRunner` and handles a specific category
//! of tests (page_index, search, graph, etc.).

mod graph;
mod page_index;
mod search;

pub use graph::GraphRunner;
pub use page_index::PageIndexRunner;
pub use search::SearchRunner;
