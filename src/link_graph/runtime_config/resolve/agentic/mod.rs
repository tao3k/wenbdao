use crate::link_graph::runtime_config::models::LinkGraphAgenticRuntimeConfig;
use crate::link_graph::runtime_config::settings::merged_wendao_settings;

mod apply;
mod finalize;

pub fn resolve_link_graph_agentic_runtime() -> LinkGraphAgenticRuntimeConfig {
    let settings = merged_wendao_settings();
    let mut resolved = LinkGraphAgenticRuntimeConfig::default();

    apply::apply_suggested_link_settings(&settings, &mut resolved);
    apply::apply_search_settings(&settings, &mut resolved);
    apply::apply_expansion_settings(&settings, &mut resolved);
    apply::apply_execution_settings(&settings, &mut resolved);

    finalize::finalize_execution_defaults(&mut resolved);
    resolved
}
