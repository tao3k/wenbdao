//! Search backend integration for Studio API.

mod ast;
mod attachments;
mod autocomplete;
#[path = "code_search/mod.rs"]
mod code_search;
mod definition;
mod flight;
mod index;
mod knowledge;
mod queries;
mod references;
mod status;
mod symbols;
#[cfg(test)]
mod test_prelude;

#[cfg(test)]
pub use ast::search_ast;
#[cfg(test)]
pub(crate) use attachments::load_attachment_search_response_from_studio;
#[cfg(test)]
pub(crate) use autocomplete::build_autocomplete_response;
#[cfg(test)]
pub(crate) use definition::build_definition_response;
#[cfg(feature = "julia")]
pub(crate) use flight::build_studio_search_flight_service_with_repo_provider;
#[cfg(test)]
pub use index::build_ast_index;
pub use index::build_symbol_index;
#[cfg(test)]
pub(crate) use knowledge::build_knowledge_search_response;
#[cfg(test)]
pub(crate) use knowledge::load_intent_search_response_with_metadata;
#[cfg(test)]
pub(crate) use references::load_reference_search_response;
pub use status::search_index_status;
#[cfg(test)]
pub(crate) use symbols::load_symbol_search_response;

#[cfg(test)]
#[path = "../../../../../tests/unit/gateway/studio/search.rs"]
mod studio_search_tests;

#[cfg(test)]
pub(crate) mod tests;
