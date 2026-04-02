/// Plugin artifact payloads and render helpers.
pub mod artifacts;
/// Generic capability bindings and contract versions.
pub mod capabilities;
/// Compatibility adapters for legacy language-named surfaces.
pub mod compat;
/// Stable identifiers for plugins, capabilities, and artifacts.
pub mod ids;
/// Transport kinds and endpoint metadata.
pub mod transport;

pub use artifacts::{
    render_plugin_artifact_toml, render_plugin_artifact_toml_for_selector, resolve_plugin_artifact,
    resolve_plugin_artifact_for_selector,
};
pub use compat::build_rerank_provider_binding;
#[cfg(feature = "julia")]
pub use transport::{
    CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, NegotiatedFlightTransportClient,
    NegotiatedTransportSelection, SearchPlaneRepoSearchFlightRouteProvider,
    bootstrap_sample_repo_search_content, build_search_plane_flight_service,
    build_search_plane_flight_service_with_weights, build_search_plane_studio_flight_service,
    build_search_plane_studio_flight_service_for_roots,
    build_search_plane_studio_flight_service_for_roots_with_weights,
    build_search_plane_studio_flight_service_with_weights,
    negotiate_flight_transport_client_from_bindings,
};
pub use xiuxian_wendao_core::artifacts::{
    PluginArtifactPayload, PluginArtifactSelector, PluginLaunchSpec,
};
pub use xiuxian_wendao_core::capabilities::{
    ContractVersion, PluginCapabilityBinding, PluginProviderSelector,
};
pub use xiuxian_wendao_core::ids::{ArtifactId, CapabilityId, PluginId};
pub use xiuxian_wendao_core::transport::{PluginTransportEndpoint, PluginTransportKind};

#[cfg(test)]
mod tests;
