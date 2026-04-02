use xiuxian_vector::{ColumnarScanOptions, VectorStoreError};

use crate::search_plane::local_symbol::build::{LocalSymbolBuildPlan, LocalSymbolWriteResult};
use crate::search_plane::local_symbol::schema::{
    local_symbol_batches, local_symbol_schema, path_column,
};
use crate::search_plane::{
    SearchBuildLease, SearchCorpusKind, SearchPlaneService, delete_paths_from_table,
};

pub(crate) async fn write_local_symbol_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &LocalSymbolBuildPlan,
) -> Result<LocalSymbolWriteResult, VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let schema = local_symbol_schema();
    let mut row_count = 0_u64;
    let mut fragment_count = 0_u64;

    for (partition_id, partition_plan) in &plan.partitions {
        let table_name = SearchPlaneService::local_partition_table_name(
            SearchCorpusKind::LocalSymbol,
            lease.epoch,
            partition_id.as_str(),
        );
        let changed_batches = local_symbol_batches(partition_plan.changed_hits.as_slice())?;

        if let Some(base_epoch) = plan.base_epoch {
            let base_table_name = SearchPlaneService::local_partition_table_name(
                SearchCorpusKind::LocalSymbol,
                base_epoch,
                partition_id.as_str(),
            );
            if service.local_table_exists(SearchCorpusKind::LocalSymbol, base_table_name.as_str()) {
                store
                    .clone_table(base_table_name.as_str(), table_name.as_str(), true)
                    .await?;
                delete_paths_from_table(
                    &store,
                    table_name.as_str(),
                    path_column(),
                    &partition_plan.replaced_paths,
                )
                .await?;
                if !changed_batches.is_empty() {
                    store
                        .merge_insert_record_batches(
                            table_name.as_str(),
                            schema.clone(),
                            changed_batches,
                            &["id".to_string()],
                        )
                        .await?;
                }
            } else if !changed_batches.is_empty() {
                store
                    .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
                    .await?;
            } else {
                continue;
            }
        } else if !changed_batches.is_empty() {
            store
                .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
                .await?;
        } else {
            continue;
        }

        let table_info = store.get_table_info(table_name.as_str()).await?;
        store
            .write_vector_store_table_to_parquet_file(
                table_name.as_str(),
                service
                    .local_table_parquet_path(SearchCorpusKind::LocalSymbol, table_name.as_str())
                    .as_path(),
                ColumnarScanOptions::default(),
            )
            .await?;
        row_count = row_count.saturating_add(table_info.num_rows);
        fragment_count = fragment_count
            .saturating_add(u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX));
    }

    Ok(LocalSymbolWriteResult {
        row_count,
        fragment_count,
    })
}
