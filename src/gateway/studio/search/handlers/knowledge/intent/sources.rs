use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::search::handlers::knowledge::helpers::is_index_not_ready;
use crate::gateway::studio::search::handlers::knowledge::intent::types::{
    IntentIndexState, IntentSourceHits,
};

pub(crate) async fn search_intent_sources(
    studio: &StudioState,
    query_text: &str,
    candidate_limit: usize,
    index_state: &IntentIndexState,
) -> Result<IntentSourceHits, StudioApiError> {
    let (knowledge_result, symbol_result) = tokio::join!(
        async {
            if index_state.knowledge_config_missing {
                Ok(Vec::new())
            } else {
                studio
                    .search_knowledge_sections(query_text, candidate_limit)
                    .await
            }
        },
        async {
            if index_state.symbol_config_missing {
                Ok(Vec::new())
            } else {
                studio
                    .search_local_symbol_hits(query_text, candidate_limit)
                    .await
            }
        }
    );

    let (knowledge_hits, knowledge_indexing) = decode_intent_source_result(knowledge_result)?;
    let (local_symbol_hits, local_symbol_indexing) = decode_intent_source_result(symbol_result)?;
    Ok(IntentSourceHits {
        knowledge_hits,
        local_symbol_hits,
        knowledge_indexing,
        local_symbol_indexing,
    })
}

fn decode_intent_source_result<T>(
    result: Result<Vec<T>, StudioApiError>,
) -> Result<(Vec<T>, bool), StudioApiError> {
    match result {
        Ok(hits) => Ok((hits, false)),
        Err(error) if is_index_not_ready(&error) => Ok((Vec::new(), true)),
        Err(error) => Err(error),
    }
}
