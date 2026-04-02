use crate::search::FuzzySearchOptions;

pub(super) fn projected_page_document_search_options() -> FuzzySearchOptions {
    FuzzySearchOptions::document_search()
}
