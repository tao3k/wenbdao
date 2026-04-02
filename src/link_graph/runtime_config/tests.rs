use super::{
    export_link_graph_compat_deployment_artifact_toml, resolve_link_graph_agentic_runtime,
    resolve_link_graph_coactivation_runtime, resolve_link_graph_compat_deployment_artifact,
    resolve_link_graph_rerank_binding, resolve_link_graph_rerank_flight_runtime_settings,
    resolve_link_graph_rerank_schema_version, resolve_link_graph_rerank_score_weights,
    resolve_link_graph_retrieval_policy_runtime,
};
use crate::link_graph::runtime_config::constants::DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION;
use crate::link_graph::runtime_config::models::LinkGraphSemanticIgnitionBackend;
use crate::link_graph::runtime_config::models::retrieval::{
    julia_deployment_artifact_selector, julia_rerank_provider_selector,
};
use crate::link_graph::set_link_graph_wendao_config_override;
use chrono::DateTime;
use serial_test::serial;
use std::fs;
use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH, DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH,
};
use xiuxian_wendao_runtime::runtime_config::{
    DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH,
};

#[test]
fn test_coactivation_touch_queue_depth_default() {
    let runtime = resolve_link_graph_coactivation_runtime();
    assert_eq!(runtime.max_hops, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS);
    assert_eq!(
        runtime.max_total_propagations,
        DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION.saturating_mul(2)
    );
    assert!(
        (runtime.hop_decay_scale - DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE).abs()
            <= f64::EPSILON,
        "unexpected hop_decay_scale: {}",
        runtime.hop_decay_scale
    );
    assert_eq!(
        runtime.touch_queue_depth,
        DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH
    );
}

#[test]
#[serial]
fn test_agentic_runtime_resolves_override_values() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.agentic.suggested_link]
max_entries = 111
ttl_seconds = 600

[link_graph.agentic.search]
include_provisional_default = true
provisional_limit = 17

[link_graph.agentic.expansion]
max_workers = 3
max_candidates = 90
max_pairs_per_worker = 11
time_budget_ms = 44.0

[link_graph.agentic.execution]
worker_time_budget_ms = 33.0
persist_suggestions_default = true
persist_retry_attempts = 4
idempotency_scan_limit = 77
relation = "supports"
agent_id = "runtime-agent"
evidence_prefix = "runtime-prefix"
"#,
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_agentic_runtime();
    assert_eq!(runtime.suggested_link_max_entries, 111);
    assert_eq!(runtime.suggested_link_ttl_seconds, Some(600));
    assert!(runtime.search_include_provisional_default);
    assert_eq!(runtime.search_provisional_limit, 17);
    assert_eq!(runtime.expansion_max_workers, 3);
    assert_eq!(runtime.expansion_max_candidates, 90);
    assert_eq!(runtime.expansion_max_pairs_per_worker, 11);
    assert!((runtime.expansion_time_budget_ms - 44.0).abs() <= f64::EPSILON);
    assert!((runtime.execution_worker_time_budget_ms - 33.0).abs() <= f64::EPSILON);
    assert!(runtime.execution_persist_suggestions_default);
    assert_eq!(runtime.execution_persist_retry_attempts, 4);
    assert_eq!(runtime.execution_idempotency_scan_limit, 77);
    assert_eq!(runtime.execution_relation, "supports");
    assert_eq!(runtime.execution_agent_id, "runtime-agent");
    assert_eq!(runtime.execution_evidence_prefix, "runtime-prefix");

    Ok(())
}

#[test]
#[serial]
fn test_retrieval_runtime_resolves_semantic_ignition_config()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    let shared_path = temp.path().join("wendao.shared.toml");
    fs::write(
        &shared_path,
        r#"[semantic_ignition]
backend = "openai-compatible"
vector_store_path = ".cache/glm-anchor-store"
table_name = "glm_anchor_index"
embedding_base_url = "http://127.0.0.1:11434"
embedding_model = "glm-5"
"#,
    )?;
    fs::write(
        &config_path,
        r#"[link_graph.retrieval]
imports = ["wendao.shared.toml"]
mode = "hybrid"
candidate_multiplier = 3
max_sources = 5
graph_rows_per_source = 4
"#,
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    assert_eq!(
        runtime.semantic_ignition.backend,
        LinkGraphSemanticIgnitionBackend::OpenAiCompatible
    );
    assert_eq!(runtime.candidate_multiplier, 3);
    assert_eq!(runtime.max_sources, 5);
    assert_eq!(runtime.graph_rows_per_source, 4);
    assert_eq!(
        runtime.semantic_ignition.vector_store_path.as_deref(),
        Some(".cache/glm-anchor-store")
    );
    assert_eq!(
        runtime.semantic_ignition.table_name.as_deref(),
        Some("glm_anchor_index")
    );
    assert_eq!(
        runtime.semantic_ignition.embedding_base_url.as_deref(),
        Some("http://127.0.0.1:11434")
    );
    assert_eq!(
        runtime.semantic_ignition.embedding_model.as_deref(),
        Some("glm-5")
    );
    assert!(runtime.julia_rerank.base_url.is_none());
    assert!(runtime.rerank_binding().is_none());

    Ok(())
}

#[test]
#[serial]
fn test_retrieval_runtime_resolves_julia_rerank_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        format!(
            r#"[link_graph.retrieval]
mode = "hybrid"

[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
health_route = "/healthz"
schema_version = "v1"
timeout_secs = 15
service_mode = "stream"
analyzer_config_path = "{}"
analyzer_strategy = "similarity_only"
vector_weight = 0.2
similarity_weight = 0.8
"#,
            DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH,
        ),
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    assert_eq!(
        runtime.julia_rerank.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(runtime.julia_rerank.route.as_deref(), Some("/rerank"));
    assert_eq!(
        runtime.julia_rerank.health_route.as_deref(),
        Some("/healthz")
    );
    assert_eq!(runtime.julia_rerank.schema_version.as_deref(), Some("v1"));
    assert_eq!(runtime.julia_rerank.timeout_secs, Some(15));
    assert_eq!(runtime.julia_rerank.service_mode.as_deref(), Some("stream"));
    assert_eq!(
        runtime.julia_rerank.analyzer_config_path.as_deref(),
        Some(DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH)
    );
    assert_eq!(
        runtime.julia_rerank.analyzer_strategy.as_deref(),
        Some("similarity_only")
    );
    assert_eq!(runtime.julia_rerank.vector_weight, Some(0.2));
    assert_eq!(runtime.julia_rerank.similarity_weight, Some(0.8));
    let score_weights =
        resolve_link_graph_rerank_score_weights().expect("score weights should resolve");
    assert!((score_weights.vector_weight - 0.2).abs() < f64::EPSILON);
    assert!((score_weights.semantic_weight - 0.8).abs() < f64::EPSILON);
    assert_eq!(
        resolve_link_graph_rerank_schema_version().as_deref(),
        Some("v1")
    );
    let flight_settings = resolve_link_graph_rerank_flight_runtime_settings();
    assert_eq!(flight_settings.schema_version.as_deref(), Some("v1"));
    let flight_weights = flight_settings
        .score_weights
        .expect("flight score weights should resolve");
    assert!((flight_weights.vector_weight - 0.2).abs() < f64::EPSILON);
    assert!((flight_weights.semantic_weight - 0.8).abs() < f64::EPSILON);

    let descriptor = runtime.julia_rerank.analyzer_service_descriptor();
    let provider_descriptor = runtime.julia_rerank.provider_launch_descriptor();
    assert_eq!(descriptor.service_mode.as_deref(), Some("stream"));
    assert_eq!(provider_descriptor, descriptor);
    assert_eq!(
        descriptor.analyzer_config_path.as_deref(),
        Some(DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH)
    );
    assert_eq!(
        descriptor.analyzer_strategy.as_deref(),
        Some("similarity_only")
    );
    assert_eq!(descriptor.vector_weight, Some(0.2));
    assert_eq!(descriptor.similarity_weight, Some(0.8));

    let manifest = runtime.julia_rerank.analyzer_launch_manifest();
    let launch_spec = runtime.julia_rerank.plugin_launch_spec();
    assert_eq!(manifest.launcher_path, launch_spec.launcher_path);
    assert_eq!(manifest.args, launch_spec.args);
    assert_eq!(manifest.launcher_path, DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH);
    assert_eq!(
        manifest.args,
        vec![
            "--service-mode",
            "stream",
            "--analyzer-config",
            DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH,
            "--analyzer-strategy",
            "similarity_only",
            "--vector-weight",
            "0.2",
            "--similarity-weight",
            "0.8",
        ]
    );

    let artifact = runtime.julia_rerank.deployment_artifact();
    let artifact_payload = runtime.julia_rerank.plugin_artifact_payload();
    let artifact_selector = julia_deployment_artifact_selector();
    assert_eq!(
        artifact.artifact_schema_version,
        DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
    );
    assert_eq!(artifact_payload.plugin_id, artifact_selector.plugin_id);
    assert_eq!(artifact_payload.artifact_id, artifact_selector.artifact_id);
    DateTime::parse_from_rfc3339(&artifact.generated_at)?;
    assert_eq!(artifact.base_url.as_deref(), Some("http://127.0.0.1:8088"));
    assert_eq!(artifact.route.as_deref(), Some("/rerank"));
    assert_eq!(artifact.health_route.as_deref(), Some("/healthz"));
    assert_eq!(artifact.schema_version.as_deref(), Some("v1"));
    assert_eq!(artifact.timeout_secs, Some(15));
    assert_eq!(artifact.launch, manifest);

    let direct_binding = runtime
        .julia_rerank
        .rerank_provider_binding()
        .expect("direct generic rerank binding");
    let binding = runtime.rerank_binding().expect("generic rerank binding");
    assert_eq!(
        direct_binding.selector.provider.0,
        binding.selector.provider.0
    );
    assert_eq!(
        direct_binding.selector.capability_id.0,
        binding.selector.capability_id.0
    );
    assert_eq!(direct_binding.endpoint.base_url, binding.endpoint.base_url);
    assert_eq!(binding.selector, julia_rerank_provider_selector());
    assert_eq!(
        binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(binding.endpoint.route.as_deref(), Some("/rerank"));
    assert_eq!(
        binding.transport,
        xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight
    );
    assert_eq!(binding.endpoint.health_route.as_deref(), Some("/healthz"));
    assert_eq!(binding.endpoint.timeout_secs, Some(15));
    assert_eq!(binding.contract_version.0, "v1");
    assert_eq!(
        binding
            .launch
            .as_ref()
            .map(|launch| launch.launcher_path.as_str()),
        Some(DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH)
    );

    let encoded = toml::to_string_pretty(&artifact)?;
    assert!(encoded.contains("launcher_path"));
    assert!(encoded.contains("base_url = \"http://127.0.0.1:8088\""));
    assert_eq!(artifact.to_toml_string()?, encoded);
    let resolved_artifact = resolve_link_graph_compat_deployment_artifact();
    assert_eq!(
        resolved_artifact.artifact_schema_version,
        DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
    );
    DateTime::parse_from_rfc3339(&resolved_artifact.generated_at)?;
    assert_eq!(resolved_artifact.base_url, artifact.base_url);
    assert_eq!(resolved_artifact.route, artifact.route);
    assert_eq!(resolved_artifact.health_route, artifact.health_route);
    assert_eq!(resolved_artifact.schema_version, artifact.schema_version);
    assert_eq!(resolved_artifact.timeout_secs, artifact.timeout_secs);
    assert_eq!(resolved_artifact.launch, artifact.launch);

    let resolved_binding =
        resolve_link_graph_rerank_binding().expect("resolved generic rerank binding");
    assert_eq!(resolved_binding.selector, julia_rerank_provider_selector());
    assert_eq!(
        resolved_binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );

    let exported = export_link_graph_compat_deployment_artifact_toml()?;
    assert!(exported.contains("artifact_schema_version = \"v1\""));
    assert!(exported.contains("generated_at = "));

    Ok(())
}

#[test]
fn test_compat_deployment_artifact_writes_toml_file() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = super::models::retrieval::LinkGraphCompatDeploymentArtifact {
        artifact_schema_version: DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
            .to_string(),
        generated_at: "2026-03-27T16:00:00+00:00".to_string(),
        base_url: Some("http://127.0.0.1:18080".to_string()),
        route: Some("/rerank".to_string()),
        health_route: Some("/health".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        launch: super::models::retrieval::LinkGraphCompatAnalyzerLaunchManifest {
            launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
            args: vec![
                "--service-mode".to_string(),
                "stream".to_string(),
                "--analyzer-strategy".to_string(),
                "similarity_only".to_string(),
            ],
        },
    };

    let temp = tempfile::tempdir()?;
    let artifact_path = temp
        .path()
        .join("nested")
        .join("compat_deployment_artifact.toml");
    artifact.write_toml_file(&artifact_path)?;

    let written = fs::read_to_string(&artifact_path)?;
    assert!(written.contains("artifact_schema_version = \"v1\""));
    assert!(written.contains("generated_at = \"2026-03-27T16:00:00+00:00\""));
    assert!(written.contains("base_url = \"http://127.0.0.1:18080\""));
    assert!(written.contains(&format!(
        "launcher_path = \"{}\"",
        DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
    )));
    assert!(written.contains("\"similarity_only\""));
    assert_eq!(written, artifact.to_toml_string()?);

    Ok(())
}

#[test]
fn test_compat_deployment_artifact_writes_json_file() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = super::models::retrieval::LinkGraphCompatDeploymentArtifact {
        artifact_schema_version: DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
            .to_string(),
        generated_at: "2026-03-27T16:00:00+00:00".to_string(),
        base_url: Some("http://127.0.0.1:18080".to_string()),
        route: Some("/rerank".to_string()),
        health_route: Some("/health".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        launch: super::models::retrieval::LinkGraphCompatAnalyzerLaunchManifest {
            launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
            args: vec![
                "--service-mode".to_string(),
                "stream".to_string(),
                "--analyzer-strategy".to_string(),
                "similarity_only".to_string(),
            ],
        },
    };

    let temp = tempfile::tempdir()?;
    let artifact_path = temp
        .path()
        .join("nested")
        .join("compat_deployment_artifact.json");
    artifact.write_json_file(&artifact_path)?;

    let written = fs::read_to_string(&artifact_path)?;
    assert!(written.contains("\"artifact_schema_version\": \"v1\""));
    assert!(written.contains("\"generated_at\": \"2026-03-27T16:00:00+00:00\""));
    assert!(written.contains("\"base_url\": \"http://127.0.0.1:18080\""));
    assert!(written.contains(&format!(
        "\"launcher_path\": \"{}\"",
        DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
    )));
    assert_eq!(written, artifact.to_json_string()?);

    Ok(())
}

#[test]
fn legacy_julia_runtime_type_aliases_match_compat_aliases() {
    let compat_artifact =
        std::any::type_name::<super::models::retrieval::LinkGraphCompatDeploymentArtifact>();
    let legacy_artifact =
        std::any::type_name::<super::models::retrieval::LinkGraphJuliaDeploymentArtifact>();
    assert_eq!(compat_artifact, legacy_artifact);

    let compat_launch =
        std::any::type_name::<super::models::retrieval::LinkGraphCompatAnalyzerLaunchManifest>();
    let legacy_launch =
        std::any::type_name::<super::models::retrieval::LinkGraphJuliaAnalyzerLaunchManifest>();
    assert_eq!(compat_launch, legacy_launch);

    let compat_runtime =
        std::any::type_name::<super::models::retrieval::LinkGraphCompatRerankRuntimeConfig>();
    let legacy_runtime =
        std::any::type_name::<super::models::retrieval::LinkGraphJuliaRerankRuntimeConfig>();
    assert_eq!(compat_runtime, legacy_runtime);
}
