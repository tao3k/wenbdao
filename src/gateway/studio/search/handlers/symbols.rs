use std::path::Path;
use std::sync::Arc;

use globset::{Glob, GlobSet, GlobSetBuilder};
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
    LanceUInt64Array,
};
use xiuxian_wendao_runtime::transport::SearchFlightRouteResponse;

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::symbol_index::SymbolIndexPhase;
use crate::gateway::studio::types::{
    StudioNavigationTarget, SymbolSearchHit, SymbolSearchResponse, UiProjectConfig,
};

use super::queries::SymbolSearchQuery;
use crate::gateway::studio::search::project_scope::project_metadata_for_path;

pub(crate) async fn load_symbol_search_response(
    state: &GatewayState,
    query: SymbolSearchQuery,
) -> Result<SymbolSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Symbol search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let status = state.studio.symbol_index_status()?;
    let Some(index) = state.studio.current_symbol_index() else {
        return Ok(SymbolSearchResponse {
            query: query_text.to_string(),
            hit_count: 0,
            selected_scope: "project".to_string(),
            partial: true,
            indexing_state: Some(status.phase.as_str().to_string()),
            index_error: status.last_error,
            hits: Vec::new(),
        });
    };
    let projects = state.studio.configured_projects();
    let glob_matcher = build_project_glob_matcher(projects.as_slice());
    let mut hits: Vec<SymbolSearchHit> = index
        .search_unified(query_text, limit)
        .into_iter()
        .enumerate()
        .map(|(rank, symbol)| {
            symbol_search_hit(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
                symbol,
                rank,
            )
        })
        .filter(|hit| {
            glob_matcher
                .as_ref()
                .is_none_or(|matcher| matcher.is_match(hit.path.as_str()))
        })
        .collect();
    hits.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
    });

    Ok(SymbolSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        selected_scope: "project".to_string(),
        partial: false,
        indexing_state: Some(SymbolIndexPhase::Ready.as_str().to_string()),
        index_error: None,
        hits: {
            hits.truncate(limit);
            hits
        },
    })
}

pub(crate) async fn load_symbol_search_flight_response(
    state: Arc<GatewayState>,
    query: SymbolSearchQuery,
) -> Result<SearchFlightRouteResponse, StudioApiError> {
    let response = load_symbol_search_response(state.as_ref(), query).await?;
    let app_metadata = serde_json::to_vec(&response).map_err(|error| {
        StudioApiError::internal(
            "SEARCH_SYMBOL_FLIGHT_METADATA_ENCODE_FAILED",
            "Failed to encode symbol-search Flight metadata",
            Some(error.to_string()),
        )
    })?;
    build_symbol_hits_flight_batch(&response.hits)
        .map(|batch| SearchFlightRouteResponse::new(batch).with_app_metadata(app_metadata))
        .map_err(|error| {
            StudioApiError::internal(
                "SEARCH_SYMBOL_FLIGHT_BATCH_BUILD_FAILED",
                "Failed to build symbol-search Flight batch",
                Some(error),
            )
        })
}

fn build_symbol_hits_flight_batch(hits: &[SymbolSearchHit]) -> Result<LanceRecordBatch, String> {
    let names = hits.iter().map(|hit| hit.name.as_str()).collect::<Vec<_>>();
    let kinds = hits.iter().map(|hit| hit.kind.as_str()).collect::<Vec<_>>();
    let paths = hits.iter().map(|hit| hit.path.as_str()).collect::<Vec<_>>();
    let lines = hits.iter().map(|hit| hit.line as u64).collect::<Vec<_>>();
    let locations = hits
        .iter()
        .map(|hit| hit.location.as_str())
        .collect::<Vec<_>>();
    let languages = hits
        .iter()
        .map(|hit| hit.language.as_str())
        .collect::<Vec<_>>();
    let sources = hits
        .iter()
        .map(|hit| hit.source.as_str())
        .collect::<Vec<_>>();
    let crate_names = hits
        .iter()
        .map(|hit| hit.crate_name.as_str())
        .collect::<Vec<_>>();
    let project_names = hits
        .iter()
        .map(|hit| hit.project_name.as_deref())
        .collect::<Vec<_>>();
    let root_labels = hits
        .iter()
        .map(|hit| hit.root_label.as_deref())
        .collect::<Vec<_>>();
    let navigation_targets_json = hits
        .iter()
        .map(|hit| serde_json::to_string(&hit.navigation_target).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let scores = hits.iter().map(|hit| hit.score).collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("name", LanceDataType::Utf8, false),
            LanceField::new("kind", LanceDataType::Utf8, false),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("line", LanceDataType::UInt64, false),
            LanceField::new("location", LanceDataType::Utf8, false),
            LanceField::new("language", LanceDataType::Utf8, false),
            LanceField::new("source", LanceDataType::Utf8, false),
            LanceField::new("crateName", LanceDataType::Utf8, false),
            LanceField::new("projectName", LanceDataType::Utf8, true),
            LanceField::new("rootLabel", LanceDataType::Utf8, true),
            LanceField::new("navigationTargetJson", LanceDataType::Utf8, false),
            LanceField::new("score", LanceDataType::Float64, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceStringArray::from(kinds)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceUInt64Array::from(lines)),
            Arc::new(LanceStringArray::from(locations)),
            Arc::new(LanceStringArray::from(languages)),
            Arc::new(LanceStringArray::from(sources)),
            Arc::new(LanceStringArray::from(crate_names)),
            Arc::new(LanceStringArray::from(project_names)),
            Arc::new(LanceStringArray::from(root_labels)),
            Arc::new(LanceStringArray::from(
                navigation_targets_json
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceFloat64Array::from(scores)),
        ],
    )
    .map_err(|error| format!("failed to build symbol-search Flight batch: {error}"))
}

fn symbol_search_hit(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    symbol: crate::unified_symbol::UnifiedSymbol,
    rank: usize,
) -> SymbolSearchHit {
    let (path, line) = parse_symbol_location(symbol.location.as_str());
    let metadata = project_metadata_for_path(project_root, config_root, projects, path.as_str());
    let source = if symbol.is_project() {
        "project".to_string()
    } else {
        "external".to_string()
    };
    let language =
        crate::gateway::studio::search::support::source_language_label(Path::new(path.as_str()))
            .unwrap_or("unknown")
            .to_string();

    SymbolSearchHit {
        name: symbol.name,
        kind: symbol.kind,
        path: path.clone(),
        line,
        location: symbol.location,
        language,
        source,
        crate_name: symbol.crate_name,
        project_name: metadata.project_name.clone(),
        root_label: metadata.root_label.clone(),
        navigation_target: StudioNavigationTarget {
            path,
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: Some(line),
            line_end: Some(line),
            column: None,
        },
        score: if rank == usize::MAX { 0.0 } else { 0.95 },
    }
}

fn parse_symbol_location(location: &str) -> (String, usize) {
    match location.rsplit_once(':') {
        Some((path, line)) => (path.to_string(), line.parse::<usize>().unwrap_or(1)),
        None => (location.to_string(), 1),
    }
}

fn build_project_glob_matcher(projects: &[UiProjectConfig]) -> Option<GlobSet> {
    let patterns = projects
        .iter()
        .flat_map(|project| project.dirs.iter())
        .filter(|dir| is_glob_pattern(dir.as_str()))
        .collect::<Vec<_>>();
    if patterns.is_empty() {
        return None;
    }

    let mut builder = GlobSetBuilder::new();
    let mut has_pattern = false;
    for pattern in patterns {
        let Ok(glob) = Glob::new(pattern.as_str()) else {
            continue;
        };
        builder.add(glob);
        has_pattern = true;
    }

    if !has_pattern {
        return None;
    }

    builder.build().ok()
}

fn is_glob_pattern(value: &str) -> bool {
    value.contains('*') || value.contains('?') || value.contains('[')
}
