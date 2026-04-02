use crate::search_plane::knowledge_section::build::types::{
    KnowledgeSectionBuildPlan, KnowledgeSectionWriteResult,
};
use crate::search_plane::knowledge_section::schema::{
    knowledge_section_batches, knowledge_section_schema, path_column,
};
use crate::search_plane::{
    SearchBuildLease, SearchCorpusKind, SearchPlaneService, delete_paths_from_table,
};
use xiuxian_vector::{ColumnarScanOptions, VectorStoreError};

#[cfg(test)]
use crate::gateway::studio::types::UiProjectConfig;
#[cfg(test)]
use crate::search_plane::BeginBuildDecision;
#[cfg(test)]
use crate::search_plane::knowledge_section::build::orchestration::plan_knowledge_section_build;
#[cfg(test)]
use crate::search_plane::knowledge_section::build::types::KnowledgeSectionBuildError;
#[cfg(test)]
use crate::search_plane::knowledge_section::schema::projected_columns;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::Path;

pub(super) async fn write_knowledge_section_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &KnowledgeSectionBuildPlan,
) -> Result<KnowledgeSectionWriteResult, VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::KnowledgeSection)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, lease.epoch);
    let schema = knowledge_section_schema();
    let changed_batches = knowledge_section_batches(plan.changed_rows.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, base_epoch);
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
    export_knowledge_section_epoch_parquet(service, lease.epoch).await?;
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(KnowledgeSectionWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
}

pub(super) async fn export_knowledge_section_epoch_parquet(
    service: &SearchPlaneService,
    epoch: u64,
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::KnowledgeSection)
        .await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, epoch);
    let parquet_path = service.local_epoch_parquet_path(SearchCorpusKind::KnowledgeSection, epoch);
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            parquet_path.as_path(),
            ColumnarScanOptions::default(),
        )
        .await
}

#[cfg(test)]
pub(crate) async fn publish_knowledge_sections_from_projects(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    fingerprint: &str,
) -> Result<(), KnowledgeSectionBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::KnowledgeSection,
        fingerprint,
        SearchCorpusKind::KnowledgeSection.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Ok(());
        }
    };
    let plan =
        plan_knowledge_section_build(project_root, config_root, projects, None, BTreeMap::new());
    match write_knowledge_section_epoch(service, &lease, &plan).await {
        Ok(write) => {
            let prewarm_columns = projected_columns();
            service
                .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                .await?;
            service.publish_ready_and_maintain(&lease, write.row_count, write.fragment_count);
            Ok(())
        }
        Err(error) => {
            service.coordinator().fail_build(
                &lease,
                format!("knowledge section epoch write failed: {error}"),
            );
            Err(KnowledgeSectionBuildError::Storage(error))
        }
    }
}
