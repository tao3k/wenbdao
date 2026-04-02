use std::collections::BTreeMap;

use xiuxian_vector::VectorStoreError;

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_content_chunk::build::plan::plan_repo_content_chunk_build;
use crate::search_plane::repo_content_chunk::build::types::RepoContentChunkBuildAction;
use crate::search_plane::repo_content_chunk::build::write::{
    inspect_repo_content_chunk_parquet, write_mutated_table, write_replaced_table,
};
use crate::search_plane::repo_content_chunk::schema::projected_columns;
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, SearchPlaneService, SearchPublicationStorageFormat,
    SearchRepoPublicationInput,
};

pub(crate) async fn publish_repo_content_chunks(
    service: &SearchPlaneService,
    repo_id: &str,
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
) -> Result<(), VectorStoreError> {
    let previous_fingerprints = service
        .repo_corpus_file_fingerprints(SearchCorpusKind::RepoContentChunk, repo_id)
        .await;
    let current_record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
        .await;
    let plan = plan_repo_content_chunk_build(
        repo_id,
        documents,
        source_revision,
        current_record
            .as_ref()
            .and_then(|record| record.publication.as_ref()),
        previous_fingerprints,
    );

    match &plan.action {
        RepoContentChunkBuildAction::Noop => {
            service
                .set_repo_corpus_file_fingerprints(
                    SearchCorpusKind::RepoContentChunk,
                    repo_id,
                    &plan.file_fingerprints,
                )
                .await;
            Ok(())
        }
        RepoContentChunkBuildAction::RefreshPublication { table_name } => {
            let parquet_stats =
                inspect_repo_content_chunk_parquet(service, table_name.as_str()).await?;
            service
                .record_repo_publication_input_with_storage_format(
                    SearchCorpusKind::RepoContentChunk,
                    repo_id,
                    SearchRepoPublicationInput {
                        table_name: table_name.clone(),
                        schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                        source_revision: source_revision.map(str::to_string),
                        table_version_id: parquet_stats.table_version_id,
                        row_count: parquet_stats.row_count,
                        fragment_count: parquet_stats.fragment_count,
                        published_at: parquet_stats.published_at,
                    },
                    SearchPublicationStorageFormat::Parquet,
                )
                .await;
            service
                .set_repo_corpus_file_fingerprints(
                    SearchCorpusKind::RepoContentChunk,
                    repo_id,
                    &plan.file_fingerprints,
                )
                .await;
            Ok(())
        }
        RepoContentChunkBuildAction::ReplaceAll {
            table_name,
            payload: documents,
        } => {
            let parquet_stats =
                write_replaced_table(service, table_name.as_str(), documents).await?;
            finalize_repo_content_publication(
                service,
                repo_id,
                table_name.as_str(),
                source_revision,
                parquet_stats,
                &plan.file_fingerprints,
            )
            .await
        }
        RepoContentChunkBuildAction::CloneAndMutate {
            base_table_name,
            target_table_name,
            replaced_paths,
            changed_payload: changed_documents,
        } => {
            let parquet_stats = write_mutated_table(
                service,
                base_table_name.as_str(),
                target_table_name.as_str(),
                replaced_paths,
                changed_documents,
            )
            .await?;
            finalize_repo_content_publication(
                service,
                repo_id,
                target_table_name.as_str(),
                source_revision,
                parquet_stats,
                &plan.file_fingerprints,
            )
            .await
        }
    }
}

async fn finalize_repo_content_publication(
    service: &SearchPlaneService,
    repo_id: &str,
    table_name: &str,
    source_revision: Option<&str>,
    parquet_stats: crate::search_plane::repo_publication_parquet::ParquetPublicationStats,
    file_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
) -> Result<(), VectorStoreError> {
    let prewarm_columns = projected_columns();
    service
        .prewarm_repo_table(
            SearchCorpusKind::RepoContentChunk,
            repo_id,
            table_name,
            &prewarm_columns,
        )
        .await?;
    service
        .record_repo_publication_input_with_storage_format(
            SearchCorpusKind::RepoContentChunk,
            repo_id,
            SearchRepoPublicationInput {
                table_name: table_name.to_string(),
                schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                source_revision: source_revision.map(str::to_string),
                table_version_id: parquet_stats.table_version_id,
                row_count: parquet_stats.row_count,
                fragment_count: parquet_stats.fragment_count,
                published_at: parquet_stats.published_at,
            },
            SearchPublicationStorageFormat::Parquet,
        )
        .await;
    service
        .set_repo_corpus_file_fingerprints(
            SearchCorpusKind::RepoContentChunk,
            repo_id,
            file_fingerprints,
        )
        .await;
    Ok(())
}
