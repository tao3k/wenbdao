//! Language-specific Repo Intelligence plugins bundled into the Wendao runtime.
//!
//! The Julia plugin now enters the host through a normal crate dependency.
//! Modelica now follows the same package-dependency path, which turns the
//! second plugin onboarding proof into a normal Cargo integration rather than
//! a sibling-source inclusion seam.

#[cfg(feature = "julia")]
pub use xiuxian_wendao_julia::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, JuliaRepoIntelligencePlugin,
    build_julia_flight_transport_client, process_julia_flight_batches,
    process_julia_flight_batches_for_repository, register_into as register_julia_plugin,
    validate_julia_arrow_response_batches,
};

#[cfg(feature = "modelica")]
pub use xiuxian_wendao_modelica::{
    ModelicaRepoIntelligencePlugin, register_into as register_modelica_plugin,
};
