use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::search::handlers::knowledge::helpers::is_ui_config_required;
use crate::gateway::studio::search::handlers::knowledge::intent::types::IntentIndexState;

pub(crate) fn ensure_intent_indices(
    studio: &StudioState,
) -> Result<IntentIndexState, StudioApiError> {
    let knowledge_start = studio.ensure_knowledge_section_index_started();
    let symbol_start = studio.ensure_local_symbol_index_started();
    let knowledge_config_missing =
        matches!(knowledge_start, Err(ref error) if is_ui_config_required(error));
    let symbol_config_missing =
        matches!(symbol_start, Err(ref error) if is_ui_config_required(error));
    if let Err(error) = knowledge_start.as_ref()
        && !is_ui_config_required(error)
    {
        return Err(error.clone());
    }
    if let Err(error) = symbol_start.as_ref()
        && !is_ui_config_required(error)
    {
        return Err(error.clone());
    }
    Ok(IntentIndexState {
        knowledge_config_missing,
        symbol_config_missing,
    })
}
