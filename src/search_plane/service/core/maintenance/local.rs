use xiuxian_vector::{ColumnarScanOptions, VectorStoreError};

use super::helpers::{LOCAL_MAINTENANCE_SHUTDOWN_MESSAGE, PREWARM_ROW_LIMIT};
use crate::search_plane::SearchCorpusKind;
use crate::search_plane::service::core::types::SearchPlaneService;

impl SearchPlaneService {
    pub(crate) fn stop_background_maintenance(&self) {
        self.stop_local_maintenance();
        self.stop_repo_maintenance();
    }

    pub(crate) fn stop_local_maintenance(&self) {
        let worker_handle = {
            let mut runtime = self
                .local_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            runtime.shutdown_requested = true;
            runtime.running_compactions.clear();
            runtime.compaction_queue.clear();
            runtime.worker_running = false;
            runtime.active_compaction = None;
            runtime.worker_handle.take()
        };
        if let Some(worker_handle) = worker_handle {
            worker_handle.abort();
        }
    }

    pub(crate) async fn prewarm_epoch_table(
        &self,
        corpus: SearchCorpusKind,
        epoch: u64,
        projected_columns: &[&str],
    ) -> Result<(), VectorStoreError> {
        if self.local_maintenance_shutdown_requested() {
            return Err(VectorStoreError::General(
                LOCAL_MAINTENANCE_SHUTDOWN_MESSAGE.to_string(),
            ));
        }
        let table_names = self.local_epoch_table_names_for_reads(corpus, epoch);
        let _ = self.coordinator.mark_prewarm_running(corpus, epoch);
        let result = async {
            for table_name in table_names {
                self.prewarm_named_table(corpus, table_name.as_str(), projected_columns)
                    .await?;
            }
            Ok::<(), VectorStoreError>(())
        }
        .await;
        match result {
            Ok(()) => {
                let _ = self.coordinator.mark_prewarm_complete(corpus, epoch);
                Ok(())
            }
            Err(error) => {
                let _ = self.coordinator.clear_prewarm_running(corpus, epoch);
                Err(error)
            }
        }
    }

    pub(crate) async fn prewarm_named_table(
        &self,
        corpus: SearchCorpusKind,
        table_name: &str,
        projected_columns: &[&str],
    ) -> Result<(), VectorStoreError> {
        let parquet_path = self.named_table_parquet_path(corpus, table_name);
        if parquet_path.exists() {
            if (!corpus.is_repo_backed() && self.local_maintenance_shutdown_requested())
                || (corpus.is_repo_backed() && self.repo_maintenance_shutdown_requested())
            {
                return Err(VectorStoreError::General(if corpus.is_repo_backed() {
                    crate::search_plane::service::core::maintenance::helpers::REPO_MAINTENANCE_SHUTDOWN_MESSAGE.to_string()
                } else {
                    LOCAL_MAINTENANCE_SHUTDOWN_MESSAGE.to_string()
                }));
            }
            let engine_table_name = Self::maintenance_engine_table_name(corpus, table_name);
            self.search_engine
                .ensure_parquet_table_registered(
                    engine_table_name.as_str(),
                    parquet_path.as_path(),
                    &[],
                )
                .await?;
            let projection = if projected_columns.is_empty() {
                "*".to_string()
            } else {
                projected_columns.join(", ")
            };
            let query = format!(
                "SELECT {projection} FROM {} LIMIT {}",
                engine_table_name, PREWARM_ROW_LIMIT
            );
            let _ = self.search_engine.sql_batches(query.as_str()).await?;
            return Ok(());
        }

        let store = self.open_store(corpus).await?;
        store
            .scan_record_batches_streaming_async(
                table_name,
                ColumnarScanOptions {
                    projected_columns: projected_columns
                        .iter()
                        .map(|column| (*column).to_string())
                        .collect(),
                    batch_size: Some(PREWARM_ROW_LIMIT),
                    fragment_readahead: Some(1),
                    batch_readahead: Some(1),
                    limit: Some(PREWARM_ROW_LIMIT),
                    ..ColumnarScanOptions::default()
                },
                |_batch| async {
                    if self.local_maintenance_shutdown_requested() {
                        Err(VectorStoreError::General(
                            LOCAL_MAINTENANCE_SHUTDOWN_MESSAGE.to_string(),
                        ))
                    } else {
                        Ok::<(), VectorStoreError>(())
                    }
                },
            )
            .await
    }

    pub(crate) fn local_maintenance_shutdown_requested(&self) -> bool {
        self.local_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .shutdown_requested
    }
}
