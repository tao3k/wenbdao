use super::super::super::LinkGraphIndex;
use crate::link_graph::{LinkGraphPlannedSearchPayload, LinkGraphSearchOptions};

impl LinkGraphIndex {
    pub(super) fn search_planned_payload_with_agentic_runtime_bridge_with_query_vector(
        &self,
        query: &str,
        limit: usize,
        base_options: LinkGraphSearchOptions,
        include_provisional: Option<bool>,
        provisional_limit: Option<usize>,
        query_vector_override: Option<Vec<f32>>,
    ) -> LinkGraphPlannedSearchPayload {
        let fallback_options = base_options.clone();
        let fallback_query = query.to_string();
        let fallback_index = self.clone();
        let fallback_query_vector = query_vector_override.clone();

        let worker_index = self.clone();
        let worker_query = query.to_string();
        let worker_query_vector = query_vector_override.clone();
        let worker_name = "wendao-semantic-ignition-bridge".to_string();
        match std::thread::Builder::new()
            .name(worker_name)
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return worker_index.search_planned_payload_with_agentic_core_sync(
                        &worker_query,
                        limit,
                        base_options,
                        include_provisional,
                        provisional_limit,
                        None,
                        worker_query_vector,
                    );
                };
                runtime.block_on(
                    worker_index.search_planned_payload_with_agentic_async_with_query_vector(
                        &worker_query,
                        worker_query_vector.as_deref().unwrap_or(&[]),
                        limit,
                        base_options,
                        include_provisional,
                        provisional_limit,
                    ),
                )
            }) {
            Ok(handle) => match handle.join() {
                Ok(payload) => payload,
                Err(_) => fallback_index.search_planned_payload_with_agentic_core_sync(
                    &fallback_query,
                    limit,
                    fallback_options,
                    include_provisional,
                    provisional_limit,
                    None,
                    fallback_query_vector,
                ),
            },
            Err(_) => fallback_index.search_planned_payload_with_agentic_core_sync(
                &fallback_query,
                limit,
                fallback_options,
                include_provisional,
                provisional_limit,
                None,
                fallback_query_vector,
            ),
        }
    }
}
