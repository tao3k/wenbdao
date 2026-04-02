use super::ValkeyAnalysisCache;
use super::runtime::resolve_valkey_analysis_cache_runtime_with_lookup;
use super::storage::{
    decode_analysis_payload, encode_analysis_payload, stable_revision, valkey_analysis_key,
};
use crate::analyzers::cache::RepositoryAnalysisCacheKey;
use crate::analyzers::plugin::RepositoryAnalysisOutput;

fn sample_cache_key(repo_id: &str) -> RepositoryAnalysisCacheKey {
    RepositoryAnalysisCacheKey {
        repo_id: repo_id.to_string(),
        checkout_root: format!("/virtual/{repo_id}"),
        checkout_revision: Some("rev-1".to_string()),
        mirror_revision: Some("mirror-1".to_string()),
        tracking_revision: Some("tracking-1".to_string()),
        plugin_ids: vec!["plugin-a".to_string()],
    }
}

#[test]
fn runtime_resolution_uses_first_non_empty_url_and_normalized_prefix() {
    let runtime = resolve_valkey_analysis_cache_runtime_with_lookup(&|name| match name {
        "XIUXIAN_WENDAO_ANALYZER_VALKEY_URL" => Some(" redis://127.0.0.1/ ".to_string()),
        "XIUXIAN_WENDAO_ANALYZER_VALKEY_KEY_PREFIX" => {
            Some("  xiuxian:test:repo-analysis  ".to_string())
        }
        "XIUXIAN_WENDAO_ANALYZER_VALKEY_TTL_SECS" => Some("3600".to_string()),
        _ => None,
    })
    .unwrap_or_else(|error| panic!("runtime resolution should succeed: {error}"))
    .unwrap_or_else(|| panic!("runtime should exist"));

    assert_eq!(runtime.key_prefix, "xiuxian:test:repo-analysis");
    assert_eq!(runtime.ttl_seconds, Some(3600));
    assert!(runtime.client.is_some());
}

#[test]
fn runtime_resolution_rejects_invalid_ttl() {
    let error = resolve_valkey_analysis_cache_runtime_with_lookup(&|name| match name {
        "XIUXIAN_WENDAO_ANALYZER_VALKEY_URL" => Some("redis://127.0.0.1/".to_string()),
        "XIUXIAN_WENDAO_ANALYZER_VALKEY_TTL_SECS" => Some("invalid".to_string()),
        _ => None,
    })
    .err()
    .unwrap_or_else(|| panic!("invalid ttl should fail"));

    assert!(
        error
            .to_string()
            .contains("XIUXIAN_WENDAO_ANALYZER_VALKEY_TTL_SECS")
    );
}

#[test]
fn stable_revision_prefers_checkout_revision() {
    let key = sample_cache_key("stable-revision");
    assert_eq!(stable_revision(&key), Some("rev-1"));
}

#[test]
fn valkey_analysis_key_is_none_without_stable_revision() {
    let key = RepositoryAnalysisCacheKey {
        repo_id: "no-revision".to_string(),
        checkout_root: "/tmp/no-revision".to_string(),
        checkout_revision: None,
        mirror_revision: None,
        tracking_revision: None,
        plugin_ids: vec!["plugin-a".to_string()],
    };

    assert!(valkey_analysis_key(&key, "xiuxian:test").is_none());
    assert!(encode_analysis_payload(&key, &RepositoryAnalysisOutput::default()).is_none());
}

#[test]
fn payload_roundtrip_preserves_analysis_output() {
    let key = sample_cache_key("payload-roundtrip");
    let analysis = RepositoryAnalysisOutput {
        modules: vec![crate::analyzers::ModuleRecord {
            repo_id: key.repo_id.clone(),
            module_id: "module:alpha".to_string(),
            qualified_name: "Alpha".to_string(),
            path: "src/lib.rs".to_string(),
        }],
        ..RepositoryAnalysisOutput::default()
    };
    let payload =
        encode_analysis_payload(&key, &analysis).unwrap_or_else(|| panic!("payload should encode"));
    let decoded = decode_analysis_payload(&key, payload.as_str())
        .unwrap_or_else(|| panic!("payload should decode"));

    assert_eq!(decoded, analysis);
}

#[test]
fn cache_roundtrip_uses_test_shadow_when_no_live_client_is_bound() {
    let cache = ValkeyAnalysisCache::for_tests("xiuxian:test:repo-analysis", Some(60));
    let key = sample_cache_key("shadow-roundtrip");
    let analysis = RepositoryAnalysisOutput {
        modules: vec![crate::analyzers::ModuleRecord {
            repo_id: key.repo_id.clone(),
            module_id: "module:alpha".to_string(),
            qualified_name: "Alpha".to_string(),
            path: "src/lib.rs".to_string(),
        }],
        ..RepositoryAnalysisOutput::default()
    };

    cache.set(&key, &analysis);
    let loaded = cache
        .get(&key)
        .unwrap_or_else(|| panic!("cached analysis should load"));

    assert_eq!(loaded, analysis);
}
