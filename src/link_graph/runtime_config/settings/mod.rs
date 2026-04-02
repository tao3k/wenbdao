use serde_yaml::Value;
use xiuxian_wendao_runtime::settings::merged_toml_settings;

pub(super) use xiuxian_wendao_runtime::settings::{
    first_non_empty, get_setting_string, parse_positive_f64, parse_positive_usize,
};
pub use xiuxian_wendao_runtime::settings::{
    set_link_graph_config_home_override, set_link_graph_wendao_config_override,
};

/// Embedded default TOML configuration.
const EMBEDDED_WENDAO_TOML: &str = include_str!("../../../../resources/config/wendao.toml");
const EMBEDDED_WENDAO_SOURCE_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/resources/config/wendao.toml");

pub(super) fn merged_wendao_settings() -> Value {
    merged_toml_settings(
        "link_graph",
        EMBEDDED_WENDAO_TOML,
        EMBEDDED_WENDAO_SOURCE_PATH,
        "wendao.toml",
    )
}
