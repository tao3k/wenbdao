//! TOML configuration loading using xiuxian-config-core.
//!
//! This module provides unified TOML configuration loading with cascading support:
//! 1. Embedded defaults from `resources/config/wendao.toml`
//! 2. User overrides from `--conf` CLI flag or `wendao.toml` in project

use super::overrides::wendao_config_file_override;
use serde_yaml::Value;
use std::path::Path;
use xiuxian_config_core::{
    ConfigCascadeSpec, load_toml_value_with_imports, resolve_and_merge_toml,
};

/// Embedded default TOML configuration.
const EMBEDDED_WENDAO_TOML: &str = include_str!("../../../../resources/config/wendao.toml");

/// Merge TOML configuration with cascading support.
///
/// Priority order:
/// 1. Embedded defaults (resources/config/wendao.toml)
/// 2. User config from `--conf` flag or wendao.toml in project
pub(in crate::link_graph::runtime_config) fn merged_wendao_settings() -> Value {
    let spec = ConfigCascadeSpec::new("link_graph", EMBEDDED_WENDAO_TOML, "wendao.toml")
        .with_embedded_source_path(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/config/wendao.toml"
        ));

    // If user specified a config file via --conf, use it directly
    if let Some(user_path) = wendao_config_file_override() {
        // Try to load the user's config file, including recursive imports.
        if let Ok(user_config) = load_toml_value_with_imports(user_path.as_path()) {
            // Merge with embedded defaults
            let mut merged = load_embedded_defaults();
            deep_merge(&mut merged, toml_value_to_yaml(&user_config));
            return merged;
        }
    }

    // Otherwise, use xiuxian-config-core cascade resolver
    match resolve_and_merge_toml(spec) {
        Ok(toml_value) => {
            // Convert to YAML Value via JSON intermediate
            let json_str = serde_json::to_string(&toml_value)
                .ok()
                .unwrap_or_else(|| "{}".to_string());
            serde_json::from_str::<Value>(&json_str).unwrap_or_else(|_| load_embedded_defaults())
        }
        Err(_) => load_embedded_defaults(),
    }
}

/// Load embedded defaults from bundled TOML.
fn load_embedded_defaults() -> Value {
    let source_path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/config/wendao.toml"
    ));
    if let Ok(toml_value) = load_toml_value_with_imports(source_path) {
        return toml_value_to_yaml(&toml_value);
    }

    let toml_value: toml::Value =
        toml::from_str(EMBEDDED_WENDAO_TOML).unwrap_or(toml::Value::Table(toml::map::Map::new()));
    toml_value_to_yaml(&toml_value)
}

/// Deep merge overlay into base value.
fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Mapping(base_map), Value::Mapping(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(existing) = base_map.get_mut(&key) {
                    deep_merge(existing, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

fn toml_value_to_yaml(value: &toml::Value) -> Value {
    let json_str = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str::<Value>(&json_str).unwrap_or(Value::Null)
}
