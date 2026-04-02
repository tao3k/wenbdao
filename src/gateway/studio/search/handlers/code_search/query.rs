use std::collections::BTreeMap;
#[cfg(test)]
use std::time::Duration;

use crate::search_plane::{RepoSearchAvailability, RepoSearchPublicationState, SearchPlaneService};

#[cfg(test)]
use super::types::ParsedCodeSearchQuery;
use super::types::{ParsedRepoCodeSearchQuery, RepoSearchDispatch, RepoSearchTarget};

#[cfg(test)]
const DEFAULT_REPO_WIDE_CODE_SEARCH_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const DEFAULT_REPO_WIDE_PER_REPO_ENTITY_RESULT_LIMIT: usize = 12;
#[cfg(test)]
const DEFAULT_REPO_WIDE_PER_REPO_CONTENT_RESULT_LIMIT: usize = 4;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RepoSearchResultLimits {
    pub(crate) entity_limit: usize,
    pub(crate) content_limit: usize,
}

#[cfg(test)]
pub(crate) fn repo_wide_code_search_timeout(repo_hint: Option<&str>) -> Option<Duration> {
    repo_hint
        .is_none()
        .then_some(DEFAULT_REPO_WIDE_CODE_SEARCH_TIMEOUT)
}

#[cfg(test)]
pub(crate) fn repo_search_result_limits(
    repo_hint: Option<&str>,
    limit: usize,
) -> RepoSearchResultLimits {
    if repo_hint.is_some() {
        return RepoSearchResultLimits {
            entity_limit: limit,
            content_limit: limit,
        };
    }

    RepoSearchResultLimits {
        entity_limit: limit.min(DEFAULT_REPO_WIDE_PER_REPO_ENTITY_RESULT_LIMIT),
        content_limit: limit.min(DEFAULT_REPO_WIDE_PER_REPO_CONTENT_RESULT_LIMIT),
    }
}

pub(crate) fn collect_repo_search_targets(
    repo_ids: Vec<String>,
    publication_states: &BTreeMap<String, RepoSearchPublicationState>,
) -> RepoSearchDispatch {
    let mut dispatch = RepoSearchDispatch::default();
    for repo_id in repo_ids {
        let publication_state = publication_states.get(repo_id.as_str()).copied().unwrap_or(
            RepoSearchPublicationState {
                entity_published: false,
                content_published: false,
                availability: RepoSearchAvailability::Pending,
            },
        );
        if publication_state.is_searchable() {
            dispatch.searchable_repos.push(RepoSearchTarget {
                repo_id,
                publication_state,
            });
            continue;
        }
        match publication_state.availability {
            RepoSearchAvailability::Skipped => dispatch.skipped_repos.push(repo_id),
            RepoSearchAvailability::Pending => dispatch.pending_repos.push(repo_id),
            RepoSearchAvailability::Searchable => {}
        }
    }
    dispatch
}

pub(crate) fn repo_search_parallelism(
    search_plane: &SearchPlaneService,
    repo_count: usize,
) -> usize {
    search_plane.repo_search_parallelism(repo_count)
}

pub(crate) fn parse_repo_code_search_query(query: &str) -> ParsedRepoCodeSearchQuery {
    let mut spec = ParsedRepoCodeSearchQuery::default();
    let mut search_tokens = Vec::new();
    for token in query.split_whitespace() {
        if let Some(value) = token.strip_prefix("lang:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                spec.language_filters.insert(normalized);
            }
            continue;
        }

        if let Some(value) = token.strip_prefix("kind:") {
            let normalized = value.trim().to_ascii_lowercase();
            if matches!(
                normalized.as_str(),
                "file" | "symbol" | "function" | "module" | "example"
            ) {
                spec.kind_filters.insert(normalized);
                continue;
            }
        }

        search_tokens.push(token.to_string());
    }

    spec.search_term = (!search_tokens.is_empty()).then(|| search_tokens.join(" "));
    spec
}

#[cfg(test)]
pub(crate) fn parse_code_search_query(
    query: &str,
    repo_hint: Option<&str>,
) -> ParsedCodeSearchQuery {
    let mut parsed = ParsedCodeSearchQuery {
        repo: repo_hint
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        ..ParsedCodeSearchQuery::default()
    };
    let mut terms = Vec::new();

    for token in query.split_whitespace() {
        if let Some(value) = token.strip_prefix("lang:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() && !parsed.languages.contains(&normalized) {
                parsed.languages.push(normalized);
            }
            continue;
        }
        if let Some(value) = token.strip_prefix("kind:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() && !parsed.kinds.contains(&normalized) {
                parsed.kinds.push(normalized);
            }
            continue;
        }
        if let Some(value) = token.strip_prefix("repo:") {
            let repo_id = value.trim();
            if !repo_id.is_empty() {
                parsed.repo = Some(repo_id.to_string());
            }
            continue;
        }
        terms.push(token);
    }

    parsed.query = terms.join(" ").trim().to_string();
    parsed
}

#[cfg(test)]
pub(crate) fn infer_repo_hint_from_query<'a, I>(
    parsed: &ParsedCodeSearchQuery,
    repo_ids: I,
) -> Option<String>
where
    I: IntoIterator<Item = &'a str>,
{
    if parsed.repo.is_some() {
        return None;
    }

    let normalized_query = normalize_repo_search_seed(parsed.query.as_str());
    if normalized_query.is_empty() {
        return None;
    }

    let mut matches = repo_ids
        .into_iter()
        .filter(|repo_id| normalize_repo_search_seed(repo_id) == normalized_query);
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }

    Some(first.to_string())
}

#[cfg(test)]
fn normalize_repo_search_seed(value: &str) -> String {
    let mut normalized = value.trim().to_ascii_lowercase();
    if normalized.ends_with(".jl") {
        normalized.truncate(normalized.len().saturating_sub(3));
    }

    let mut collapsed = String::with_capacity(normalized.len());
    let mut in_whitespace = true;
    for character in normalized.chars() {
        let mapped = if matches!(character, '_' | '.' | '/' | '-') {
            ' '
        } else {
            character
        };
        if mapped.is_ascii_whitespace() {
            if !in_whitespace {
                collapsed.push(' ');
            }
            in_whitespace = true;
        } else {
            collapsed.push(mapped);
            in_whitespace = false;
        }
    }

    collapsed.trim().to_string()
}
