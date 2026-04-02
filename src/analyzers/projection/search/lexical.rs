use crate::FuzzyMatcher;
use crate::analyzers::{ProjectedPageRecord, ProjectionPageKind};
use crate::search::{FuzzySearchOptions, LexicalMatcher};

use super::mapping::{fuzzy_match_score, page_matches_kind, stable_page_score};
use super::sort::sort_ranked_pages;

fn projected_page_title(page: &ProjectedPageRecord) -> &str {
    page.title.as_str()
}

pub(super) fn lexical_projected_page_matches(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    pages: &[ProjectedPageRecord],
    limit: usize,
    options: FuzzySearchOptions,
) -> Vec<(u8, ProjectedPageRecord)> {
    let filtered_pages = pages
        .iter()
        .filter(|page| page_matches_kind(page, kind_filter))
        .cloned()
        .collect::<Vec<_>>();

    let lexical_matcher =
        LexicalMatcher::new(filtered_pages.as_slice(), projected_page_title, options);
    let mut ranked_pages = lexical_matcher
        .search(query, limit)
        .expect("lexical matcher is infallible")
        .into_iter()
        .map(|matched_page| {
            (
                stable_page_score(
                    &matched_page.item,
                    &query.to_ascii_lowercase(),
                    fuzzy_match_score(matched_page.score),
                ),
                matched_page.item,
            )
        })
        .collect::<Vec<_>>();
    sort_ranked_pages(&mut ranked_pages);
    ranked_pages
}
