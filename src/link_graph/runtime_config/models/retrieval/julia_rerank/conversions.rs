use super::runtime::LinkGraphJuliaRerankRuntimeConfig;
use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;

/// Build a generic capability binding from the legacy Julia rerank runtime config.
#[must_use]
pub fn build_rerank_provider_binding(
    runtime: &LinkGraphJuliaRerankRuntimeConfig,
) -> PluginCapabilityBinding {
    xiuxian_wendao_julia::compatibility::link_graph::build_rerank_provider_binding(runtime)
}

#[cfg(test)]
/// Build a generic capability binding from the legacy Julia rerank runtime config.
#[must_use]
pub fn build_plugin_capability_binding_for_julia_rerank(
    runtime: &LinkGraphJuliaRerankRuntimeConfig,
) -> PluginCapabilityBinding {
    build_rerank_provider_binding(runtime)
}

#[cfg(test)]
mod tests {
    use super::{
        LinkGraphJuliaRerankRuntimeConfig, build_plugin_capability_binding_for_julia_rerank,
        build_rerank_provider_binding,
    };

    #[test]
    fn legacy_julia_binding_builder_shims_to_provider_binding() {
        let runtime = LinkGraphJuliaRerankRuntimeConfig {
            base_url: Some("http://127.0.0.1:8088".to_string()),
            route: Some("/rerank".to_string()),
            health_route: Some("/healthz".to_string()),
            schema_version: Some("v1".to_string()),
            timeout_secs: Some(15),
            service_mode: None,
            analyzer_config_path: None,
            analyzer_strategy: None,
            vector_weight: None,
            similarity_weight: None,
        };

        assert_eq!(
            build_plugin_capability_binding_for_julia_rerank(&runtime),
            build_rerank_provider_binding(&runtime)
        );
    }
}
