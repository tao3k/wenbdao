use xiuxian_vector::VectorStoreError;

use crate::search_plane::repo_entity::schema::{RepoEntityRow, path_column, repo_entity_batches};
use crate::search_plane::repo_publication_parquet::{
    ParquetPublicationStats, inspect_repo_publication_parquet, rewrite_repo_publication_parquet,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

pub(crate) async fn write_replaced_table(
    service: &SearchPlaneService,
    table_name: &str,
    rows: &[RepoEntityRow],
) -> Result<ParquetPublicationStats, VectorStoreError> {
    let changed_batches = repo_entity_batches(rows)?;
    rewrite_repo_publication_parquet(
        service,
        SearchCorpusKind::RepoEntity,
        None,
        table_name,
        path_column(),
        &std::collections::BTreeSet::new(),
        changed_batches.as_slice(),
    )
    .await
}

pub(crate) async fn write_mutated_table(
    service: &SearchPlaneService,
    base_table_name: &str,
    target_table_name: &str,
    replaced_paths: &std::collections::BTreeSet<String>,
    changed_rows: &[RepoEntityRow],
) -> Result<ParquetPublicationStats, VectorStoreError> {
    let changed_batches = repo_entity_batches(changed_rows)?;
    rewrite_repo_publication_parquet(
        service,
        SearchCorpusKind::RepoEntity,
        Some(base_table_name),
        target_table_name,
        path_column(),
        replaced_paths,
        changed_batches.as_slice(),
    )
    .await
}

pub(crate) async fn inspect_repo_entity_parquet(
    service: &SearchPlaneService,
    table_name: &str,
) -> Result<ParquetPublicationStats, VectorStoreError> {
    inspect_repo_publication_parquet(service, SearchCorpusKind::RepoEntity, table_name).await
}
