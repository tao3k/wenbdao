use std::collections::BTreeSet;

use arrow::array::{Array, BooleanArray, LargeStringArray, StringArray, StringViewArray};
use arrow::compute::filter_record_batch;
use chrono::{DateTime, Utc};
use xiuxian_vector::{
    EngineRecordBatch, LanceRecordBatch, SearchEngineContext, VectorStoreError,
    lance_batches_to_engine_batches, write_engine_batches_to_parquet_file,
};

use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParquetPublicationStats {
    pub(crate) table_version_id: u64,
    pub(crate) row_count: u64,
    pub(crate) fragment_count: u64,
    pub(crate) published_at: String,
}

pub(crate) async fn rewrite_repo_publication_parquet(
    service: &SearchPlaneService,
    corpus: SearchCorpusKind,
    base_table_name: Option<&str>,
    target_table_name: &str,
    path_column: &str,
    replaced_paths: &BTreeSet<String>,
    changed_batches: &[LanceRecordBatch],
) -> Result<ParquetPublicationStats, VectorStoreError> {
    let mut output_batches = if let Some(base_table_name) = base_table_name {
        load_repo_publication_parquet_batches(service, corpus, base_table_name).await?
    } else {
        Vec::new()
    };

    if !replaced_paths.is_empty() {
        let mut filtered_batches = Vec::with_capacity(output_batches.len());
        for batch in output_batches {
            if let Some(filtered) =
                filter_batch_excluding_paths(batch, path_column, replaced_paths)?
            {
                filtered_batches.push(filtered);
            }
        }
        output_batches = filtered_batches;
    }

    output_batches.extend(lance_batches_to_engine_batches(changed_batches)?);

    let parquet_path = service.repo_publication_parquet_path(corpus, target_table_name);
    write_engine_batches_to_parquet_file(parquet_path.as_path(), &output_batches)?;
    let published_at = Utc::now().to_rfc3339();
    Ok(stats_from_batches(
        target_table_name,
        &output_batches,
        published_at,
    ))
}

pub(crate) async fn inspect_repo_publication_parquet(
    service: &SearchPlaneService,
    corpus: SearchCorpusKind,
    table_name: &str,
) -> Result<ParquetPublicationStats, VectorStoreError> {
    let parquet_path = service.repo_publication_parquet_path(corpus, table_name);
    let published_at =
        DateTime::<Utc>::from(std::fs::metadata(parquet_path.as_path())?.modified()?).to_rfc3339();
    let batches = load_repo_publication_parquet_batches(service, corpus, table_name).await?;
    Ok(stats_from_batches(table_name, &batches, published_at))
}

async fn load_repo_publication_parquet_batches(
    service: &SearchPlaneService,
    corpus: SearchCorpusKind,
    table_name: &str,
) -> Result<Vec<EngineRecordBatch>, VectorStoreError> {
    let parquet_path = service.repo_publication_parquet_path(corpus, table_name);
    let engine = SearchEngineContext::new();
    engine
        .register_parquet_table("repo_publication_source", parquet_path.as_path(), &[])
        .await?;
    let dataframe = engine.table("repo_publication_source").await?;
    engine.collect_dataframe(dataframe).await
}

fn filter_batch_excluding_paths(
    batch: EngineRecordBatch,
    path_column: &str,
    replaced_paths: &BTreeSet<String>,
) -> Result<Option<EngineRecordBatch>, VectorStoreError> {
    let path_index = batch.schema().index_of(path_column).map_err(|error| {
        VectorStoreError::General(format!(
            "missing repo publication path column `{path_column}` in parquet batch: {error}"
        ))
    })?;
    let path_values = batch.column(path_index);
    let keep_mask = match path_values.data_type() {
        arrow::datatypes::DataType::Utf8 => {
            let strings = path_values
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| {
                    VectorStoreError::General(format!(
                        "failed to decode Utf8 repo publication path column `{path_column}`"
                    ))
                })?;
            BooleanArray::from(
                (0..strings.len())
                    .map(|row| strings.is_null(row) || !replaced_paths.contains(strings.value(row)))
                    .collect::<Vec<_>>(),
            )
        }
        arrow::datatypes::DataType::LargeUtf8 => {
            let strings = path_values
                .as_any()
                .downcast_ref::<LargeStringArray>()
                .ok_or_else(|| {
                    VectorStoreError::General(format!(
                        "failed to decode LargeUtf8 repo publication path column `{path_column}`"
                    ))
                })?;
            BooleanArray::from(
                (0..strings.len())
                    .map(|row| strings.is_null(row) || !replaced_paths.contains(strings.value(row)))
                    .collect::<Vec<_>>(),
            )
        }
        arrow::datatypes::DataType::Utf8View => {
            let strings = path_values
                .as_any()
                .downcast_ref::<StringViewArray>()
                .ok_or_else(|| {
                    VectorStoreError::General(format!(
                        "failed to decode Utf8View repo publication path column `{path_column}`"
                    ))
                })?;
            BooleanArray::from(
                (0..strings.len())
                    .map(|row| strings.is_null(row) || !replaced_paths.contains(strings.value(row)))
                    .collect::<Vec<_>>(),
            )
        }
        other => {
            return Err(VectorStoreError::General(format!(
                "unsupported repo publication path column type for `{path_column}`: {other:?}"
            )));
        }
    };
    let filtered = filter_record_batch(&batch, &keep_mask)?;
    if filtered.num_rows() == 0 {
        Ok(None)
    } else {
        Ok(Some(filtered))
    }
}

fn stats_from_batches(
    table_name: &str,
    batches: &[EngineRecordBatch],
    published_at: String,
) -> ParquetPublicationStats {
    let row_count = batches
        .iter()
        .map(|batch| u64::try_from(batch.num_rows()).unwrap_or(u64::MAX))
        .fold(0_u64, u64::saturating_add);
    let fragment_count = u64::try_from(batches.len()).unwrap_or(u64::MAX);
    let payload = format!("{table_name}|{published_at}|{row_count}|{fragment_count}");
    let hash = blake3::hash(payload.as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    ParquetPublicationStats {
        table_version_id: u64::from_be_bytes(bytes),
        row_count,
        fragment_count,
        published_at,
    }
}
