use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

use crate::link_graph::{
    LinkGraphAttachmentKind, LinkGraphDisplayHit, LinkGraphIndex, LinkGraphRetrievalMode,
    LinkGraphSearchOptions,
};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::super::pathing;
use super::super::router::{GatewayState, StudioApiError};
use super::super::types::{
    AstSearchHit, AstSearchResponse, AttachmentSearchHit, AttachmentSearchKind,
    AttachmentSearchResponse, AutocompleteResponse, AutocompleteSuggestion,
    AutocompleteSuggestionType, DefinitionResolveResponse, ReferenceSearchResponse, SearchHit,
    SearchResponse, StudioNavigationTarget, SymbolSearchHit, SymbolSearchResponse,
    SymbolSearchSource, UiProjectConfig,
};
use super::super::vfs::{graph_lookup_candidates, studio_display_path};

use super::definition::{
    DefinitionResolveOptions, ast_hit_matches, enrich_ast_hit_project_metadata,
    resolve_definition_candidates, score_ast_hit,
};
use super::observation_hints::definition_observation_hints;
use super::project_scope::project_metadata_for_path;
use super::source_index;
use super::source_index::build_reference_hits;
use super::support::source_language_label;

const DEFAULT_SEARCH_LIMIT: usize = 10;
const MAX_SEARCH_LIMIT: usize = 200;
const DEFAULT_ATTACHMENT_SEARCH_LIMIT: usize = 10;
const MAX_ATTACHMENT_SEARCH_LIMIT: usize = 200;
const DEFAULT_AST_SEARCH_LIMIT: usize = 10;
const MAX_AST_SEARCH_LIMIT: usize = 200;
const DEFAULT_REFERENCE_SEARCH_LIMIT: usize = 10;
const MAX_REFERENCE_SEARCH_LIMIT: usize = 200;
const DEFAULT_SYMBOL_SEARCH_LIMIT: usize = 10;
const MAX_SYMBOL_SEARCH_LIMIT: usize = 200;
const DEFAULT_AUTOCOMPLETE_LIMIT: usize = 5;
const MAX_AUTOCOMPLETE_LIMIT: usize = 20;

pub(crate) fn build_ast_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<Vec<AstSearchHit>, String> {
    source_index::build_ast_index(project_root, config_root, projects)
}

pub(crate) fn build_symbol_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<UnifiedSymbolIndex, String> {
    source_index::build_symbol_index(project_root, config_root, projects)
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct SearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct AttachmentSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    ext: Vec<String>,
    #[serde(default)]
    kind: Vec<String>,
    #[serde(default)]
    case_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct AutocompleteQuery {
    prefix: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct SymbolSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct AstSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct DefinitionResolveQuery {
    q: Option<String>,
    path: Option<String>,
    #[serde(default)]
    line: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(in crate::gateway::studio) struct ReferenceSearchQuery {
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(in crate::gateway::studio) async fn search_knowledge(
    Query(query): Query<SearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.link_graph_index().await?;
    let payload = index.search_planned_payload(raw_query, limit, LinkGraphSearchOptions::default());

    let hits = payload
        .hits
        .into_iter()
        .filter_map(|hit| {
            let canonical_path =
                canonical_graph_path(state.as_ref(), index.as_ref(), hit.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                canonical_path.as_str(),
            )
            .then_some((hit, canonical_path))
        })
        .map(|(hit, canonical_path)| {
            let path = studio_display_path(state.studio.as_ref(), canonical_path.as_str());
            let navigation_target = crate::gateway::studio::vfs::resolve_navigation_target(
                state.studio.as_ref(),
                path.as_str(),
            );
            SearchHit {
                stem: hit.stem,
                title: strip_option(&hit.title),
                path,
                doc_type: hit.doc_type,
                tags: hit.tags,
                score: hit.score.max(0.0),
                best_section: strip_option(&hit.best_section),
                match_reason: strip_option(&hit.match_reason),
                navigation_target,
            }
        })
        .collect::<Vec<_>>();
    let hit_count = hits.len();

    Ok(Json(SearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        graph_confidence_score: Some(payload.graph_confidence_score),
        selected_mode: Some(retrieval_mode_to_string(payload.selected_mode)),
    }))
}

pub(in crate::gateway::studio) async fn search_attachments(
    Query(query): Query<AttachmentSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AttachmentSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_ATTACHMENT_SEARCH_LIMIT)
        .clamp(1, MAX_ATTACHMENT_SEARCH_LIMIT);
    let kinds = query
        .kind
        .iter()
        .map(|kind| LinkGraphAttachmentKind::from_alias(kind.as_str()))
        .collect::<Vec<_>>();

    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.link_graph_index().await?;
    let hits = index
        .search_attachments(
            raw_query,
            limit,
            query.ext.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .into_iter()
        .filter_map(|hit| {
            let canonical_source_path =
                canonical_graph_path(state.as_ref(), index.as_ref(), hit.source_path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                canonical_source_path.as_str(),
            )
            .then_some((hit, canonical_source_path))
        })
        .map(|(hit, canonical_source_path)| {
            let source_path =
                studio_display_path(state.studio.as_ref(), canonical_source_path.as_str());
            let source_id = hit.source_id;
            let attachment_path = hit.attachment_path;
            AttachmentSearchHit {
                path: source_path.clone(),
                source_id: source_id.clone(),
                source_stem: hit.source_stem,
                source_title: strip_option(hit.source_title.as_str()),
                source_path,
                attachment_id: attachment_id_for(source_id.as_str(), attachment_path.as_str()),
                attachment_path,
                attachment_name: hit.attachment_name,
                attachment_ext: hit.attachment_ext,
                kind: attachment_kind_to_api(hit.kind),
                score: hit.score.max(0.0),
                vision_snippet: hit
                    .vision_snippet
                    .and_then(|value| strip_option(value.as_str())),
            }
        })
        .collect::<Vec<_>>();
    let hit_count = hits.len();

    Ok(Json(AttachmentSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "attachments".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_autocomplete(
    Query(query): Query<AutocompleteQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AutocompleteResponse>, StudioApiError> {
    let prefix = query
        .prefix
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PREFIX", "`prefix` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AUTOCOMPLETE_LIMIT)
        .clamp(1, MAX_AUTOCOMPLETE_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.link_graph_index().await?;
    let payload =
        index.search_planned_payload(prefix, limit.max(2), LinkGraphSearchOptions::default());
    let filtered_hits = payload
        .hits
        .into_iter()
        .filter_map(|hit| {
            let canonical_path =
                canonical_graph_path(state.as_ref(), index.as_ref(), hit.path.as_str());
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                canonical_path.as_str(),
            )
            .then(|| {
                let mut hit = hit;
                hit.path = canonical_path;
                hit
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(AutocompleteResponse {
        prefix: prefix.to_string(),
        suggestions: collect_autocomplete_suggestions(prefix, filtered_hits.as_slice(), limit),
    }))
}

pub(in crate::gateway::studio) async fn search_ast(
    Query(query): Query<AstSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_AST_SEARCH_LIMIT)
        .clamp(1, MAX_AST_SEARCH_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.studio.ast_index().await?;
    let mut hits = index
        .iter()
        .filter(|hit| {
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                hit.path.as_str(),
            )
        })
        .filter(|hit| ast_hit_matches(hit, raw_query))
        .map(|hit| {
            let mut hit = hit.clone();
            enrich_ast_hit_project_metadata(
                &mut hit,
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
            );
            hit.score = score_ast_hit(&hit, raw_query);
            hit.navigation_target = ast_navigation_target(&hit);
            hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
            hit.navigation_target.path = hit.path.clone();
            hit
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    hits.truncate(limit);
    let hit_count = hits.len();

    Ok(Json(AstSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "definitions".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_definition(
    Query(query): Query<DefinitionResolveQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<DefinitionResolveResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let source_path = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let source_path_candidates = source_path
        .as_deref()
        .map(|path| graph_lookup_candidates(state.studio.as_ref(), path))
        .filter(|candidates| !candidates.is_empty());
    let source_line = query.line.filter(|line| *line > 0);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.studio.ast_index().await?;
    let markdown_observation_hints = definition_observation_hints(
        state.as_ref(),
        source_path_candidates.as_deref(),
        source_line,
        raw_query,
    )
    .await;
    let mut candidates = resolve_definition_candidates(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
        index.as_slice(),
        raw_query,
        DefinitionResolveOptions {
            source_paths: source_path_candidates.as_deref(),
            scope_patterns: markdown_observation_hints
                .as_ref()
                .map(|hints| hints.scope_patterns.as_slice()),
            languages: markdown_observation_hints
                .as_ref()
                .map(|hints| hints.languages.as_slice()),
            ..DefinitionResolveOptions::default()
        },
    );
    for hit in &mut candidates {
        hit.navigation_target = ast_navigation_target(hit);
        hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
        hit.navigation_target.path = hit.path.clone();
    }

    let candidate_count = candidates.len();
    let definition = candidates.into_iter().next().ok_or_else(|| {
        StudioApiError::not_found(format!("No definition found for `{raw_query}`"))
    })?;

    Ok(Json(DefinitionResolveResponse {
        query: raw_query.to_string(),
        source_path,
        source_line,
        navigation_target: definition.navigation_target.clone(),
        definition,
        candidate_count,
        selected_scope: "definition".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_symbols(
    Query(query): Query<SymbolSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<SymbolSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_SYMBOL_SEARCH_LIMIT)
        .clamp(1, MAX_SYMBOL_SEARCH_LIMIT);
    let search_window = limit.saturating_mul(4).min(MAX_SYMBOL_SEARCH_LIMIT);
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let index = state.studio.symbol_index().await?;
    let mut hits = index
        .search_project(raw_query, search_window)
        .into_iter()
        .map(|symbol| {
            let mut hit = symbol_to_hit(
                symbol,
                raw_query,
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
            );
            hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
            hit.navigation_target.path = hit.path.clone();
            hit
        })
        .filter(|hit| {
            pathing::path_matches_project_file_filters(
                project_root.as_path(),
                config_root.as_path(),
                projects.as_slice(),
                hit.path.as_str(),
            )
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });
    hits.truncate(limit);
    let hit_count = hits.len();

    Ok(Json(SymbolSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "project".to_string(),
    }))
}

pub(in crate::gateway::studio) async fn search_references(
    Query(query): Query<ReferenceSearchQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<ReferenceSearchResponse>, StudioApiError> {
    let raw_query = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`q` is required"))?;

    let limit = query
        .limit
        .unwrap_or(DEFAULT_REFERENCE_SEARCH_LIMIT)
        .clamp(1, MAX_REFERENCE_SEARCH_LIMIT);
    let ast_index = state.studio.ast_index().await?;
    let project_root = state.studio.project_root.clone();
    let config_root = state.studio.config_root.clone();
    let projects = state.studio.configured_projects();
    let worker_project_root = project_root.clone();
    let worker_config_root = config_root.clone();
    let worker_projects = projects.clone();
    let query_owned = raw_query.to_string();
    let ast_hits = ast_index.as_ref().clone();
    let hits = tokio::task::spawn_blocking(move || {
        build_reference_hits(
            worker_project_root.as_path(),
            worker_config_root.as_path(),
            worker_projects.as_slice(),
            ast_hits.as_slice(),
            query_owned.as_str(),
            limit,
        )
    })
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_PANIC",
            "Failed to execute studio reference search",
            Some(error.to_string()),
        )
    })?
    .map_err(|error| {
        StudioApiError::internal(
            "REFERENCE_SEARCH_FAILED",
            "Failed to execute studio reference search",
            Some(error),
        )
    })?;
    let mut hits = hits;
    hits.retain(|hit| {
        pathing::path_matches_project_file_filters(
            project_root.as_path(),
            config_root.as_path(),
            projects.as_slice(),
            hit.path.as_str(),
        )
    });
    for hit in &mut hits {
        hit.path = studio_display_path(state.studio.as_ref(), hit.path.as_str());
        hit.navigation_target.path = hit.path.clone();
    }
    let hit_count = hits.len();

    Ok(Json(ReferenceSearchResponse {
        query: raw_query.to_string(),
        hits,
        hit_count,
        selected_scope: "references".to_string(),
    }))
}

fn retrieval_mode_to_string(mode: LinkGraphRetrievalMode) -> String {
    match mode {
        LinkGraphRetrievalMode::GraphOnly => "graph_only".to_string(),
        LinkGraphRetrievalMode::Hybrid => "hybrid".to_string(),
        LinkGraphRetrievalMode::VectorOnly => "vector_only".to_string(),
    }
}

fn attachment_kind_to_api(kind: LinkGraphAttachmentKind) -> AttachmentSearchKind {
    match kind {
        LinkGraphAttachmentKind::Image => AttachmentSearchKind::Image,
        LinkGraphAttachmentKind::Pdf => AttachmentSearchKind::Pdf,
        LinkGraphAttachmentKind::Gpg => AttachmentSearchKind::Gpg,
        LinkGraphAttachmentKind::Document => AttachmentSearchKind::Document,
        LinkGraphAttachmentKind::Archive => AttachmentSearchKind::Archive,
        LinkGraphAttachmentKind::Audio => AttachmentSearchKind::Audio,
        LinkGraphAttachmentKind::Video => AttachmentSearchKind::Video,
        LinkGraphAttachmentKind::Other => AttachmentSearchKind::Other,
    }
}

fn attachment_id_for(source_id: &str, attachment_path: &str) -> String {
    let owner = source_id.trim();
    let owner = if owner.is_empty() { "unknown" } else { owner };
    let normalized_attachment = attachment_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();
    if normalized_attachment.is_empty() {
        format!("att://{owner}")
    } else {
        format!("att://{owner}/{normalized_attachment}")
    }
}

fn strip_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn canonical_graph_path(state: &GatewayState, index: &LinkGraphIndex, raw_path: &str) -> String {
    graph_lookup_candidates(state.studio.as_ref(), raw_path)
        .into_iter()
        .find_map(|candidate| {
            index
                .metadata(candidate.as_str())
                .map(|metadata| metadata.path)
        })
        .unwrap_or_else(|| raw_path.replace('\\', "/"))
}

fn symbol_to_hit(
    symbol: &crate::unified_symbol::UnifiedSymbol,
    query: &str,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> SymbolSearchHit {
    let (path, line) = split_location(symbol.location.as_str());
    let metadata = project_metadata_for_path(project_root, config_root, projects, path.as_str());
    let navigation_target = symbol_navigation_target(
        path.as_str(),
        symbol.crate_or_local(),
        metadata.project_name.as_deref(),
        metadata.root_label.as_deref(),
        line,
    );

    SymbolSearchHit {
        name: symbol.name.clone(),
        kind: symbol.kind.clone(),
        path: path.clone(),
        line,
        location: symbol.location.clone(),
        language: source_language_label(Path::new(path.as_str()))
            .unwrap_or("unknown")
            .to_string(),
        crate_name: symbol.crate_or_local().to_string(),
        project_name: metadata.project_name,
        root_label: metadata.root_label,
        navigation_target,
        source: if symbol.is_project() {
            SymbolSearchSource::Project
        } else {
            SymbolSearchSource::External
        },
        score: score_symbol(symbol.name.as_str(), path.as_str(), query),
    }
}

fn ast_navigation_target(hit: &AstSearchHit) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: hit.path.clone(),
        category: "doc".to_string(),
        project_name: hit
            .project_name
            .clone()
            .or_else(|| Some(hit.crate_name.clone())),
        root_label: hit.root_label.clone(),
        line: Some(hit.line_start),
        line_end: Some(hit.line_end),
        column: None,
    }
}

fn symbol_navigation_target(
    path: &str,
    crate_name: &str,
    project_name: Option<&str>,
    root_label: Option<&str>,
    line: usize,
) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: path.to_string(),
        category: "doc".to_string(),
        project_name: project_name
            .map(ToString::to_string)
            .or_else(|| Some(crate_name.to_string())),
        root_label: root_label.map(ToString::to_string),
        line: Some(line),
        line_end: Some(line),
        column: None,
    }
}

struct AutocompleteCollector<'a> {
    suggestions: Vec<AutocompleteSuggestion>,
    seen: HashSet<String>,
    prefix_lc: &'a str,
    limit: usize,
}

fn split_location(location: &str) -> (String, usize) {
    match location.rsplit_once(':') {
        Some((path, line)) => (
            path.to_string(),
            line.parse::<usize>().unwrap_or_default().max(1),
        ),
        None => (location.to_string(), 1),
    }
}

fn score_symbol(name: &str, path: &str, query: &str) -> f64 {
    let name_lc = name.to_ascii_lowercase();
    let path_lc = path.to_ascii_lowercase();
    let query_lc = query.to_ascii_lowercase();

    if name_lc == query_lc {
        1.0
    } else if name_lc.starts_with(query_lc.as_str()) {
        0.95
    } else if name_lc.contains(query_lc.as_str()) {
        0.88
    } else if path_lc.contains(query_lc.as_str()) {
        0.72
    } else {
        0.5
    }
}

impl<'a> AutocompleteCollector<'a> {
    fn new(prefix_lc: &'a str, limit: usize) -> Self {
        Self {
            suggestions: Vec::with_capacity(limit),
            seen: HashSet::new(),
            prefix_lc,
            limit,
        }
    }

    fn add(
        &mut self,
        text: &str,
        path: &str,
        doc_type: Option<&str>,
        suggestion_type: AutocompleteSuggestionType,
    ) {
        if self.suggestions.len() >= self.limit {
            return;
        }

        let normalized_text = text.trim();
        if normalized_text.is_empty()
            || !normalized_text
                .to_ascii_lowercase()
                .starts_with(self.prefix_lc)
        {
            return;
        }

        let key = format!("{suggestion_type:?}|{normalized_text}|{path}");
        if !self.seen.insert(key) {
            return;
        }

        self.suggestions.push(AutocompleteSuggestion {
            text: normalized_text.to_string(),
            suggestion_type,
            path: Some(path.to_string()),
            doc_type: doc_type.map(ToString::to_string),
        });
    }
}

fn collect_autocomplete_suggestions(
    prefix: &str,
    hits: &[LinkGraphDisplayHit],
    limit: usize,
) -> Vec<AutocompleteSuggestion> {
    let prefix_lc = prefix.to_ascii_lowercase();
    let mut collector = AutocompleteCollector::new(&prefix_lc, limit);

    for hit in hits {
        collector.add(
            &hit.stem,
            hit.path.as_str(),
            hit.doc_type.as_deref(),
            AutocompleteSuggestionType::Stem,
        );

        if !hit.title.is_empty() {
            collector.add(
                &hit.title,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Title,
            );
        }

        for tag in &hit.tags {
            collector.add(
                tag,
                hit.path.as_str(),
                hit.doc_type.as_deref(),
                AutocompleteSuggestionType::Tag,
            );
        }

        if collector.suggestions.len() >= limit {
            break;
        }
    }

    collector.suggestions
}

#[cfg(test)]
#[path = "../../../../tests/unit/gateway/studio/search.rs"]
mod tests;
