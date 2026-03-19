use super::super::super::LinkGraphIndex;
use crate::link_graph::{LinkGraphPlannedSearchPayload, LinkGraphSearchOptions};

impl LinkGraphIndex {
    pub(super) fn search_planned_payload_with_agentic_runtime_bridge(
        &self,
        query: &str,
        limit: usize,
        base_options: LinkGraphSearchOptions,
        include_provisional: Option<bool>,
        provisional_limit: Option<usize>,
    ) -> LinkGraphPlannedSearchPayload {
        let fallback_options = base_options.clone();
        let fallback_query = query.to_string();
        let fallback_index = self.clone();

        let worker_index = self.clone();
        let worker_query = query.to_string();
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
                    );
                };
                runtime.block_on(worker_index.search_planned_payload_with_agentic_async(
                    &worker_query,
                    limit,
                    base_options,
                    include_provisional,
                    provisional_limit,
                ))
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
                ),
            },
            Err(_) => fallback_index.search_planned_payload_with_agentic_core_sync(
                &fallback_query,
                limit,
                fallback_options,
                include_provisional,
                provisional_limit,
                None,
            ),
        }
    }
}
