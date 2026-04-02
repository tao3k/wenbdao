//! Gateway configuration resolution.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::info;
use xiuxian_zhenfa::WebhookConfig;

use crate::execute::gateway::shared::DEFAULT_PORT;

/// Resolve the config file from CLI override, local project file, or `PRJ_ROOT`.
pub(crate) fn resolve_config_path(cli_config: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = cli_config {
        return Some(path.to_path_buf());
    }

    let local_config = Path::new("wendao.toml");
    if local_config.exists() {
        return Some(local_config.to_path_buf());
    }

    let config_path = std::env::var("PRJ_ROOT")
        .ok()
        .map(|root| Path::new(&root).join("wendao.toml"))?;
    config_path.exists().then_some(config_path)
}

/// Resolve the port from CLI arg, config file, or default.
pub(crate) fn resolve_port(cli_port: Option<u16>, config_path: Option<&Path>) -> u16 {
    // CLI arg takes highest priority
    if let Some(port) = cli_port {
        return port;
    }

    // Try config file
    if let Some(config_port) = get_port_from_config(config_path) {
        return config_port;
    }

    // Default
    DEFAULT_PORT
}

/// Get port from wendao.toml config file.
pub(crate) fn get_port_from_config(config_path: Option<&Path>) -> Option<u16> {
    parse_port_from_toml(config_path?)
}

/// Parse port from a TOML config file.
pub(crate) fn parse_port_from_toml(path: &Path) -> Option<u16> {
    let mut file = fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;

    // Parse [gateway] section for port
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("port") {
            // Parse: port = 9517 or port = "9517"
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if let Ok(port) = value.parse::<u16>() {
                    return Some(port);
                }
            }
        }
    }

    None
}

/// Resolve webhook config with priority: TOML > env var > defaults.
pub(crate) fn resolve_webhook_config(config_path: Option<&Path>) -> WebhookConfig {
    // Try TOML config first (highest priority)
    if let Some(config) = get_webhook_from_config(config_path) {
        info!("Gateway: Using webhook config from wendao.toml");
        return config;
    }

    // Fall back to environment variables
    let url = std::env::var("WENDAO_WEBHOOK_URL").unwrap_or_default();
    if !url.is_empty() {
        info!("Gateway: Using webhook config from WENDAO_WEBHOOK_URL env var");
    }

    WebhookConfig {
        url,
        secret: std::env::var("WENDAO_WEBHOOK_SECRET").ok(),
        timeout_secs: 10,
        retry_on_failure: true,
    }
}

/// Get webhook config from wendao.toml config file.
pub(crate) fn get_webhook_from_config(config_path: Option<&Path>) -> Option<WebhookConfig> {
    parse_webhook_from_toml(config_path?)
}

/// Parse webhook config from a TOML config file.
pub(crate) fn parse_webhook_from_toml(path: &Path) -> Option<WebhookConfig> {
    let mut file = fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;

    let mut url = None;
    let mut secret = None;
    let mut enabled = true;

    // Parse [gateway] section for webhook settings
    let mut in_gateway_section = false;
    for line in content.lines() {
        let line = line.trim();

        // Track section
        if line == "[gateway]" {
            in_gateway_section = true;
            continue;
        } else if line.starts_with('[') && line.ends_with(']') {
            in_gateway_section = false;
            continue;
        }

        if !in_gateway_section {
            continue;
        }

        // Parse settings
        if line.starts_with("webhook_url") {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if !value.is_empty() && !value.starts_with('#') {
                    url = Some(value.to_string());
                }
            }
        } else if line.starts_with("webhook_secret") {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if !value.is_empty() {
                    secret = Some(value.to_string());
                }
            }
        } else if line.starts_with("webhook_enabled")
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
            enabled = value.eq_ignore_ascii_case("true");
        }
    }

    if !enabled {
        return None;
    }

    // Only return config if URL was found
    url.map(|u| WebhookConfig {
        url: u,
        secret,
        timeout_secs: 10,
        retry_on_failure: true,
    })
}
