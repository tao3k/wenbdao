use crate::link_graph::runtime_config::{
    julia_deployment_artifact_selector, resolve_link_graph_retrieval_policy_runtime,
};
use xiuxian_wendao_core::artifacts::{PluginArtifactPayload, PluginArtifactSelector};
use xiuxian_wendao_runtime::artifacts::{
    resolve_plugin_artifact_for_selector_with, resolve_plugin_artifact_with,
};
#[cfg(feature = "julia")]
use xiuxian_wendao_runtime::transport::negotiate_flight_transport_client_from_bindings;

/// Resolve one plugin artifact through the current runtime compatibility layer.
#[must_use]
pub fn resolve_plugin_artifact(
    plugin_id: &str,
    artifact_id: &str,
) -> Option<PluginArtifactPayload> {
    resolve_plugin_artifact_with(plugin_id, artifact_id, resolve_plugin_artifact_for_selector)
}

/// Resolve one plugin artifact through the current runtime compatibility layer.
#[must_use]
pub fn resolve_plugin_artifact_for_selector(
    selector: &PluginArtifactSelector,
) -> Option<PluginArtifactPayload> {
    resolve_plugin_artifact_for_selector_with(selector, |selector| {
        if selector == &julia_deployment_artifact_selector() {
            let runtime = resolve_link_graph_retrieval_policy_runtime();
            let binding = runtime.julia_rerank.rerank_provider_binding();
            Some(attach_plugin_artifact_transport_diagnostics(
                runtime.julia_rerank.plugin_artifact_payload(),
                binding.as_ref(),
            ))
        } else {
            None
        }
    })
}

#[cfg(feature = "julia")]
fn attach_plugin_artifact_transport_diagnostics(
    mut artifact: PluginArtifactPayload,
    binding: Option<&xiuxian_wendao_core::capabilities::PluginCapabilityBinding>,
) -> PluginArtifactPayload {
    let Some(binding) = binding else {
        return artifact;
    };

    match negotiate_flight_transport_client_from_bindings(std::slice::from_ref(binding)) {
        Ok(Some(transport)) => {
            let selection = transport.selection();
            artifact.selected_transport = Some(selection.selected_transport);
            artifact.fallback_from = selection.fallback_from;
            artifact.fallback_reason = selection.fallback_reason.clone();
        }
        Ok(None) => {
            artifact.fallback_from = Some(binding.transport);
            artifact.fallback_reason = Some(format!(
                "configured transport {:?} is unavailable because the binding has no base_url",
                binding.transport
            ));
        }
        Err(error) => {
            artifact.fallback_from = Some(binding.transport);
            artifact.fallback_reason = Some(error);
        }
    }

    artifact
}

#[cfg(not(feature = "julia"))]
fn attach_plugin_artifact_transport_diagnostics(
    mut artifact: PluginArtifactPayload,
    binding: Option<&xiuxian_wendao_core::capabilities::PluginCapabilityBinding>,
) -> PluginArtifactPayload {
    if let Some(binding) = binding {
        artifact.selected_transport = Some(binding.transport);
    }
    artifact
}
