use xiuxian_vector::{ColumnarScanOptions, VectorStoreError};

use crate::search_plane::attachment::build::{AttachmentBuildPlan, AttachmentWriteResult};
use crate::search_plane::attachment::schema::{
    attachment_batches, attachment_schema, source_path_column,
};
use crate::search_plane::{
    SearchBuildLease, SearchCorpusKind, SearchPlaneService, delete_paths_from_table,
};

pub(crate) async fn write_attachment_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &AttachmentBuildPlan,
) -> Result<AttachmentWriteResult, VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::Attachment).await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
    let schema = attachment_schema();
    let changed_batches = attachment_batches(plan.changed_hits.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::Attachment, base_epoch);
        store
            .clone_table(base_table_name.as_str(), table_name.as_str(), true)
            .await?;
        delete_paths_from_table(
            &store,
            table_name.as_str(),
            source_path_column(),
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
    export_attachment_epoch_parquet(service, lease.epoch).await?;
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(AttachmentWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
}

pub(crate) async fn export_attachment_epoch_parquet(
    service: &SearchPlaneService,
    epoch: u64,
) -> Result<(), VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::Attachment).await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, epoch);
    let parquet_path = service.local_epoch_parquet_path(SearchCorpusKind::Attachment, epoch);
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            parquet_path.as_path(),
            ColumnarScanOptions::default(),
        )
        .await
}
