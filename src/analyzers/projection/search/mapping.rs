use std::collections::HashMap;

use crate::analyzers::{ProjectedPageRecord, ProjectionPageKind};
use crate::search::SearchDocumentHit;

use super::sort::sort_ranked_pages;

pub(super) fn page_matches_kind(
    page: &ProjectedPageRecord,
    kind_filter: Option<ProjectionPageKind>,
) -> bool {
    match kind_filter {
        None => true,
        Some(kind) => page.kind == kind,
    }
}

pub(super) fn calculate_search_score(page: &ProjectedPageRecord, query: &str) -> u8 {
    let title_lc = page.title.to_ascii_lowercase();
    if title_lc == query {
        return 100;
    }
    if title_lc.starts_with(query) {
        return 85;
    }
    if title_lc.contains(query) {
        return 70;
    }

    if page
        .keywords
        .iter()
        .any(|keyword: &String| keyword.to_ascii_lowercase().contains(query))
    {
        return 60;
    }

    if page.path.to_ascii_lowercase().contains(query) {
        return 40;
    }

    0
}

pub(super) fn stable_page_score(page: &ProjectedPageRecord, query: &str, fallback_score: u8) -> u8 {
    calculate_search_score(page, query).max(fallback_score)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn fuzzy_match_score(score: f32) -> u8 {
    let bounded = score.clamp(0.0, 1.0);
    let scaled = 45.0 + (bounded * 35.0);
    scaled as u8
}

pub(super) fn map_search_documents_to_pages(
    records: Vec<SearchDocumentHit>,
    page_by_id: &HashMap<String, ProjectedPageRecord>,
    kind_filter: Option<ProjectionPageKind>,
    limit: usize,
    query: &str,
    fallback_score: Option<u8>,
) -> Vec<(u8, ProjectedPageRecord)> {
    let mut pages = Vec::new();
    for record in records {
        let Some(page) = page_by_id.get(record.id.as_str()) else {
            continue;
        };
        if !page_matches_kind(page, kind_filter) {
            continue;
        }
        let score = stable_page_score(page, query, fallback_score.unwrap_or_default());
        pages.push((score, page.clone()));
        if pages.len() >= limit {
            break;
        }
    }
    sort_ranked_pages(&mut pages);
    pages
}

pub(super) fn map_fuzzy_search_documents_to_pages(
    records: Vec<SearchDocumentHit>,
    page_by_id: &HashMap<String, ProjectedPageRecord>,
    kind_filter: Option<ProjectionPageKind>,
    limit: usize,
    query: &str,
) -> Vec<(u8, ProjectedPageRecord)> {
    let mut pages = Vec::new();
    for record in records {
        let Some(page) = page_by_id.get(record.id.as_str()) else {
            continue;
        };
        if !page_matches_kind(page, kind_filter) {
            continue;
        }
        let score = stable_page_score(page, query, fuzzy_match_score(record.score));
        pages.push((score, page.clone()));
        if pages.len() >= limit {
            break;
        }
    }
    sort_ranked_pages(&mut pages);
    pages
}
