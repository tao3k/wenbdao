pub(crate) use axum::extract::{Query, State};
pub(crate) use std::sync::Arc;

pub(crate) use crate::gateway::studio::search::handlers::code_search::{
    content::{
        CODE_CONTENT_EXCLUDE_GLOBS, is_supported_code_extension, parse_content_search_line,
        path_matches_language_filters, truncate_content_search_snippet,
    },
    helpers::repo_navigation_target,
    query::parse_repo_code_search_query,
};
pub(crate) use crate::gateway::studio::search::handlers::queries::{
    AstSearchQuery, AttachmentSearchQuery, ReferenceSearchQuery, SearchQuery, SymbolSearchQuery,
};
