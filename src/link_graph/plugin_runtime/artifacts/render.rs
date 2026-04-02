use super::{resolve_plugin_artifact, resolve_plugin_artifact_for_selector};
use xiuxian_wendao_core::artifacts::PluginArtifactSelector;
use xiuxian_wendao_runtime::artifacts::{
    render_plugin_artifact_toml_for_selector_with, render_plugin_artifact_toml_with,
};

/// Render a resolved plugin artifact as pretty TOML.
///
/// # Errors
///
/// Returns an error when the resolved artifact cannot be serialized into TOML.
pub fn render_plugin_artifact_toml(
    plugin_id: &str,
    artifact_id: &str,
) -> Result<Option<String>, toml::ser::Error> {
    render_plugin_artifact_toml_with(plugin_id, artifact_id, resolve_plugin_artifact)
}

/// Render a resolved plugin artifact as pretty TOML using a typed selector.
///
/// # Errors
///
/// Returns an error when the resolved artifact cannot be serialized into TOML.
pub fn render_plugin_artifact_toml_for_selector(
    selector: &PluginArtifactSelector,
) -> Result<Option<String>, toml::ser::Error> {
    render_plugin_artifact_toml_for_selector_with(selector, resolve_plugin_artifact_for_selector)
}
