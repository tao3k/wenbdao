#[cfg(feature = "julia")]
mod client;
mod endpoint;
mod kind;
#[cfg(feature = "julia")]
mod server;

#[cfg(feature = "julia")]
pub use client::{
    CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER, NegotiatedFlightTransportClient,
    NegotiatedTransportSelection, negotiate_flight_transport_client_from_bindings,
};
pub use endpoint::PluginTransportEndpoint;
pub use kind::PluginTransportKind;
#[cfg(feature = "julia")]
pub use server::{
    SearchPlaneRepoSearchFlightRouteProvider, bootstrap_sample_repo_search_content,
    build_search_plane_flight_service, build_search_plane_flight_service_with_weights,
    build_search_plane_studio_flight_service, build_search_plane_studio_flight_service_for_roots,
    build_search_plane_studio_flight_service_for_roots_with_weights,
    build_search_plane_studio_flight_service_with_weights,
};
