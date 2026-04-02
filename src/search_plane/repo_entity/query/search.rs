use std::collections::HashSet;

use crate::analyzers::query::{
    ExampleSearchResult, ImportSearchQuery, ImportSearchResult, ModuleSearchResult,
    SymbolSearchResult,
};
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;
use crate::search_plane::repo_entity::query::hydrate::{
    build_import_search_result, hydrate_repo_entity_hits, load_hydrated_rows_by_id,
    typed_repo_entity_columns,
};
use crate::search_plane::repo_entity::query::prepare::prepare_repo_entity_search;
use crate::search_plane::repo_entity::query::types::RepoEntitySearchError;

pub(crate) async fn search_repo_entities(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    language_filters: &HashSet<String>,
    kind_filters: &HashSet<String>,
    limit: usize,
) -> Result<Vec<SearchHit>, RepoEntitySearchError> {
    let Some(prepared) = prepare_repo_entity_search(
        service,
        repo_id,
        query,
        language_filters,
        kind_filters,
        limit,
    )
    .await?
    else {
        return Ok(Vec::new());
    };
    let hits: Vec<SearchHit> = hydrate_repo_entity_hits(
        service.search_engine(),
        prepared.engine_table_name.as_str(),
        prepared.candidates,
    )
    .await?;
    service.record_query_telemetry(
        crate::search_plane::SearchCorpusKind::RepoEntity,
        prepared
            .telemetry
            .finish(prepared.source, Some(repo_id.to_string()), hits.len()),
    );
    Ok(hits)
}

pub(crate) async fn search_repo_entity_module_results(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    limit: usize,
) -> Result<ModuleSearchResult, RepoEntitySearchError> {
    let language_filters = HashSet::new();
    let kind_filters =
        crate::search_plane::repo_entity::query::execution::fixed_kind_filters("module");
    let Some(prepared) = prepare_repo_entity_search(
        service,
        repo_id,
        query,
        &language_filters,
        &kind_filters,
        limit,
    )
    .await?
    else {
        return Ok(empty_module_search_result(repo_id));
    };
    let ids = prepared
        .candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    let result = build_module_search_result(
        repo_id,
        prepared.candidates,
        load_hydrated_rows_by_id(
            service.search_engine(),
            prepared.engine_table_name.as_str(),
            ids.as_slice(),
            typed_repo_entity_columns().as_slice(),
        )
        .await?,
    )?;
    service.record_query_telemetry(
        crate::search_plane::SearchCorpusKind::RepoEntity,
        prepared.telemetry.finish(
            prepared.source,
            Some(repo_id.to_string()),
            result.module_hits.len(),
        ),
    );
    Ok(result)
}

pub(crate) async fn search_repo_entity_symbol_results(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    limit: usize,
) -> Result<SymbolSearchResult, RepoEntitySearchError> {
    let language_filters = HashSet::new();
    let kind_filters =
        crate::search_plane::repo_entity::query::execution::fixed_kind_filters("symbol");
    let Some(prepared) = prepare_repo_entity_search(
        service,
        repo_id,
        query,
        &language_filters,
        &kind_filters,
        limit,
    )
    .await?
    else {
        return Ok(empty_symbol_search_result(repo_id));
    };
    let ids = prepared
        .candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    let result = build_symbol_search_result(
        repo_id,
        prepared.candidates,
        load_hydrated_rows_by_id(
            service.search_engine(),
            prepared.engine_table_name.as_str(),
            ids.as_slice(),
            typed_repo_entity_columns().as_slice(),
        )
        .await?,
    )?;
    service.record_query_telemetry(
        crate::search_plane::SearchCorpusKind::RepoEntity,
        prepared.telemetry.finish(
            prepared.source,
            Some(repo_id.to_string()),
            result.symbol_hits.len(),
        ),
    );
    Ok(result)
}

pub(crate) async fn search_repo_entity_example_results(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    limit: usize,
) -> Result<ExampleSearchResult, RepoEntitySearchError> {
    let language_filters = HashSet::new();
    let kind_filters =
        crate::search_plane::repo_entity::query::execution::fixed_kind_filters("example");
    let Some(prepared) = prepare_repo_entity_search(
        service,
        repo_id,
        query,
        &language_filters,
        &kind_filters,
        limit,
    )
    .await?
    else {
        return Ok(empty_example_search_result(repo_id));
    };
    let ids = prepared
        .candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    let result = build_example_search_result(
        repo_id,
        prepared.candidates,
        load_hydrated_rows_by_id(
            service.search_engine(),
            prepared.engine_table_name.as_str(),
            ids.as_slice(),
            typed_repo_entity_columns().as_slice(),
        )
        .await?,
    )?;
    service.record_query_telemetry(
        crate::search_plane::SearchCorpusKind::RepoEntity,
        prepared.telemetry.finish(
            prepared.source,
            Some(repo_id.to_string()),
            result.example_hits.len(),
        ),
    );
    Ok(result)
}

pub(crate) async fn search_repo_entity_import_results(
    service: &SearchPlaneService,
    query: &ImportSearchQuery,
) -> Result<ImportSearchResult, RepoEntitySearchError> {
    let language_filters = HashSet::new();
    let kind_filters =
        crate::search_plane::repo_entity::query::execution::fixed_kind_filters("import");
    let canonical_query = crate::analyzers::service::canonical_import_query_text(query);
    let Some(prepared) = prepare_repo_entity_search(
        service,
        query.repo_id.as_str(),
        canonical_query.as_str(),
        &language_filters,
        &kind_filters,
        query.limit,
    )
    .await?
    else {
        return Ok(empty_import_search_result(query.repo_id.as_str()));
    };
    let ids = prepared
        .candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    let result = build_import_search_result(
        query.repo_id.as_str(),
        prepared.candidates,
        load_hydrated_rows_by_id(
            service.search_engine(),
            prepared.engine_table_name.as_str(),
            ids.as_slice(),
            typed_repo_entity_columns().as_slice(),
        )
        .await?,
    )?;
    service.record_query_telemetry(
        crate::search_plane::SearchCorpusKind::RepoEntity,
        prepared.telemetry.finish(
            prepared.source,
            Some(query.repo_id.clone()),
            result.import_hits.len(),
        ),
    );
    Ok(result)
}

fn empty_module_search_result(repo_id: &str) -> ModuleSearchResult {
    ModuleSearchResult {
        repo_id: repo_id.to_string(),
        modules: Vec::new(),
        module_hits: Vec::new(),
    }
}

fn empty_symbol_search_result(repo_id: &str) -> SymbolSearchResult {
    SymbolSearchResult {
        repo_id: repo_id.to_string(),
        symbols: Vec::new(),
        symbol_hits: Vec::new(),
    }
}

fn empty_example_search_result(repo_id: &str) -> ExampleSearchResult {
    ExampleSearchResult {
        repo_id: repo_id.to_string(),
        examples: Vec::new(),
        example_hits: Vec::new(),
    }
}

fn empty_import_search_result(repo_id: &str) -> ImportSearchResult {
    ImportSearchResult {
        repo_id: repo_id.to_string(),
        imports: Vec::new(),
        import_hits: Vec::new(),
    }
}

fn build_module_search_result(
    repo_id: &str,
    candidates: Vec<crate::search_plane::repo_entity::query::types::RepoEntityCandidate>,
    rows: std::collections::BTreeMap<
        String,
        crate::search_plane::repo_entity::query::types::HydratedRepoEntityRow,
    >,
) -> Result<ModuleSearchResult, RepoEntitySearchError> {
    crate::search_plane::repo_entity::query::hydrate::build_module_search_result(
        repo_id, candidates, rows,
    )
}

fn build_symbol_search_result(
    repo_id: &str,
    candidates: Vec<crate::search_plane::repo_entity::query::types::RepoEntityCandidate>,
    rows: std::collections::BTreeMap<
        String,
        crate::search_plane::repo_entity::query::types::HydratedRepoEntityRow,
    >,
) -> Result<SymbolSearchResult, RepoEntitySearchError> {
    crate::search_plane::repo_entity::query::hydrate::build_symbol_search_result(
        repo_id, candidates, rows,
    )
}

fn build_example_search_result(
    repo_id: &str,
    candidates: Vec<crate::search_plane::repo_entity::query::types::RepoEntityCandidate>,
    rows: std::collections::BTreeMap<
        String,
        crate::search_plane::repo_entity::query::types::HydratedRepoEntityRow,
    >,
) -> Result<ExampleSearchResult, RepoEntitySearchError> {
    crate::search_plane::repo_entity::query::hydrate::build_example_search_result(
        repo_id, candidates, rows,
    )
}
