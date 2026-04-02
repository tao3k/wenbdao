#[cfg(feature = "zhenfa-router")]
mod http;
mod models;
/// Native Zhenfa router implementations for Wendao.
///
/// This module contains the core logic for semantic operations,
/// search tools, and context extensions specific to the Wendao knowledge graph.
pub mod native;
mod rpc;

#[cfg(feature = "zhenfa-router")]
pub use http::WendaoZhenfaRouter;
pub use native::{
    WendaoAgenticNavTool, WendaoContextExt, WendaoPluginArtifactArgs,
    WendaoPluginArtifactOutputFormat, WendaoPluginArtifactTool, WendaoSearchTool,
    WendaoSemanticCheckTool, WendaoSemanticEditTool, WendaoSemanticReadTool, audit_search_payload,
    evaluate_alignment, export_plugin_artifact, render_plugin_artifact,
    render_plugin_artifact_json, render_plugin_artifact_toml, render_xml_lite_hits,
};
pub use rpc::{execute_search, export_plugin_artifact_from_rpc_params, search_from_rpc_params};
