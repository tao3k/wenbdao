use crate::gateway::studio::types::AutocompleteSuggestion;
use crate::search_plane::local_symbol::query::shared::{
    LocalSymbolSearchError, compare_suggestions, execute_local_symbol_autocomplete,
    suggestion_window,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

pub(crate) async fn autocomplete_local_symbols(
    service: &SearchPlaneService,
    prefix: &str,
    limit: usize,
) -> Result<Vec<AutocompleteSuggestion>, LocalSymbolSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol);
    let Some(active_epoch) = status.active_epoch else {
        return Err(LocalSymbolSearchError::NotReady);
    };

    let normalized_prefix = prefix.trim().to_ascii_lowercase();
    if normalized_prefix.is_empty() {
        return Ok(Vec::new());
    }

    let table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if table_names.is_empty() {
        return Ok(Vec::new());
    }
    for table_name in &table_names {
        let parquet_path =
            service.local_table_parquet_path(SearchCorpusKind::LocalSymbol, table_name.as_str());
        if !parquet_path.exists() {
            return Err(LocalSymbolSearchError::NotReady);
        }
        service
            .search_engine()
            .ensure_parquet_table_registered(table_name.as_str(), parquet_path.as_path(), &[])
            .await?;
    }
    let execution = execute_local_symbol_autocomplete(
        service.search_engine(),
        table_names.as_slice(),
        normalized_prefix.as_str(),
        suggestion_window(limit),
    )
    .await?;
    let mut suggestions = execution.suggestions;
    suggestions.sort_by(|left, right| compare_suggestions(left, right));
    suggestions.truncate(limit);
    service.record_query_telemetry(
        SearchCorpusKind::LocalSymbol,
        execution.telemetry.finish(
            execution.source,
            Some("autocomplete".to_string()),
            suggestions.len(),
        ),
    );
    Ok(suggestions)
}
