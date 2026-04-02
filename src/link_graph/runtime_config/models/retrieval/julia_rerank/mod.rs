mod artifact;
mod conversions;
mod launch;
mod runtime;

#[cfg(test)]
pub(crate) use artifact::LinkGraphJuliaDeploymentArtifact;
pub use conversions::build_rerank_provider_binding;
#[cfg(test)]
pub(crate) use launch::LinkGraphJuliaAnalyzerLaunchManifest;
pub use runtime::LinkGraphJuliaRerankRuntimeConfig;
pub use xiuxian_wendao_julia::compatibility::link_graph::julia_deployment_artifact_selector;
#[cfg(test)]
pub(crate) use xiuxian_wendao_julia::compatibility::link_graph::julia_rerank_provider_selector;

/// Compatibility-first alias for the Julia deployment-artifact record.
#[cfg(test)]
pub(crate) type LinkGraphCompatDeploymentArtifact = LinkGraphJuliaDeploymentArtifact;
/// Compatibility-first alias for the Julia analyzer launch manifest.
#[cfg(test)]
pub(crate) type LinkGraphCompatAnalyzerLaunchManifest = LinkGraphJuliaAnalyzerLaunchManifest;
/// Compatibility-first alias for the Julia rerank runtime record.
#[cfg(test)]
pub(crate) type LinkGraphCompatRerankRuntimeConfig = LinkGraphJuliaRerankRuntimeConfig;
