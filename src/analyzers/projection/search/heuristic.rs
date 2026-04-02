use crate::analyzers::{ProjectedPageRecord, ProjectionPageKind};

use super::mapping::{calculate_search_score, page_matches_kind};
use super::sort::sort_ranked_pages;

pub(super) fn heuristic_projected_page_matches(
    query: &str,
    kind_filter: Option<ProjectionPageKind>,
    pages: &[ProjectedPageRecord],
) -> Vec<(u8, ProjectedPageRecord)> {
    let mut matches = Vec::new();

    for page in pages {
        if !page_matches_kind(page, kind_filter) {
            continue;
        }

        let score = calculate_search_score(page, query);
        if score > 0 {
            matches.push((score, page.clone()));
        }
    }

    sort_ranked_pages(&mut matches);
    matches
}
