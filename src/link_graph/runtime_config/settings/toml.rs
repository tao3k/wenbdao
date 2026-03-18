//! TOML configuration loading using xiuxian-config-core.
//!
//! This module provides unified TOML configuration loading with cascading support:
//! 1. Embedded defaults from `resources/config/wendao.toml`
//! 2. User overrides from `--conf` CLI flag or `wendao.toml` in project

use super::overrides::wendao_config_file_override;
use serde_yaml::Value;
use std::path::Path;
use xiuxian_config_core::{ConfigCascadeSpec, resolve_and_merge_toml};

/// Embedded default TOML configuration.
const EMBEDDED_WENDAO_TOML: &str = include_str!("../../../../resources/config/wendao.toml");

/// Read a single TOML file and convert to YAML Value.
fn read_toml_file(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let toml_value: toml::Value = toml::from_str(&content).ok()?;
    // Convert TOML Value to YAML Value via JSON intermediate
    let json_str = serde_json::to_string(&toml_value).ok()?;
    serde_json::from_str::<Value>(&json_str).ok()
}

/// Merge TOML configuration with cascading support.
///
/// Priority order:
/// 1. Embedded defaults (resources/config/wendao.toml)
/// 2. User config from `--conf` flag or wendao.toml in project
pub(in crate::link_graph::runtime_config) fn merged_wendao_settings() -> Value {
    // If user specified a config file via --conf, use it directly
    if let Some(user_path) = wendao_config_file_override() {
        // Try to load the user's config file
        if let Some(user_config) = read_toml_file(&user_path) {
            // Merge with embedded defaults
            let mut merged = load_embedded_defaults();
            deep_merge(&mut merged, user_config);
            return merged;
        }
    }

    // Otherwise, use xiuxian-config-core cascade resolver
    let spec = ConfigCascadeSpec::new("link_graph", EMBEDDED_WENDAO_TOML, "wendao.toml");

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
    let toml_value: toml::Value =
        toml::from_str(EMBEDDED_WENDAO_TOML).unwrap_or(toml::Value::Table(toml::map::Map::new()));
    let json_str = serde_json::to_string(&toml_value).unwrap_or_else(|_| "{}".to_string());
    serde_json::from_str::<Value>(&json_str).unwrap_or(Value::Null)
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
