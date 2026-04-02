use xiuxian_vector::{ColumnarScanOptions, VectorStoreError};

use crate::search_plane::reference_occurrence::build::{
    ReferenceOccurrenceBuildPlan, ReferenceOccurrenceWriteResult,
};
use crate::search_plane::reference_occurrence::schema::{
    path_column, reference_occurrence_batches, reference_occurrence_schema,
};
use crate::search_plane::{
    SearchBuildLease, SearchCorpusKind, SearchPlaneService, delete_paths_from_table,
};

pub(crate) async fn write_reference_occurrence_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &ReferenceOccurrenceBuildPlan,
) -> Result<ReferenceOccurrenceWriteResult, VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
    let schema = reference_occurrence_schema();
    let changed_batches = reference_occurrence_batches(plan.changed_hits.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, base_epoch);
        store
            .clone_table(base_table_name.as_str(), table_name.as_str(), true)
            .await?;
        delete_paths_from_table(
            &store,
            table_name.as_str(),
            path_column(),
            &plan.replaced_paths,
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
    } else {
        store
            .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
            .await?;
    }
    export_reference_occurrence_epoch_parquet(service, lease.epoch).await?;
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(ReferenceOccurrenceWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
}

pub(super) async fn export_reference_occurrence_epoch_parquet(
    service: &SearchPlaneService,
    epoch: u64,
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, epoch);
    let parquet_path =
        service.local_epoch_parquet_path(SearchCorpusKind::ReferenceOccurrence, epoch);
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            parquet_path.as_path(),
            ColumnarScanOptions::default(),
        )
        .await
}
