//! Studio gateway capability handlers.

mod deployment;
mod service;
mod types;

pub use deployment::get_plugin_artifact;
pub use service::get;
pub use types::{PluginArtifactPath, PluginArtifactQuery};
