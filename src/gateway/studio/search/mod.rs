mod definition;
mod handlers;
mod observation_hints;
mod project_scope;
mod source_index;
mod support;

pub(crate) use definition::{
    DefinitionMatchMode, DefinitionResolveOptions, resolve_best_definition,
};
pub(crate) use handlers::{build_ast_index, build_symbol_index};
pub(super) use handlers::{
    search_ast, search_attachments, search_autocomplete, search_definition, search_knowledge,
    search_references, search_symbols,
};
