use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::router::state::helpers::{graph_include_dirs, supported_code_kinds};
use crate::gateway::studio::router::state::lifecycle::gateway_bootstrap_background_indexing_with_lookup;
use crate::gateway::studio::types::UiProjectConfig;

#[test]
fn supported_code_kinds_contains_reference_and_doc() {
    let kinds = supported_code_kinds();
    assert!(kinds.iter().any(|kind| kind == "reference"));
    assert!(kinds.iter().any(|kind| kind == "doc"));
}

#[test]
fn graph_include_dirs_deduplicates_normalized_paths() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let project_root = temp_dir.path().to_path_buf();
    let config_root = temp_dir.path().to_path_buf();
    std::fs::create_dir_all(temp_dir.path().join("docs"))
        .unwrap_or_else(|error| panic!("create docs dir: {error}"));
    std::fs::create_dir_all(temp_dir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src dir: {error}"));

    let projects = vec![UiProjectConfig {
        name: "kernel".to_string(),
        root: ".".to_string(),
        dirs: vec![
            "docs".to_string(),
            "./docs".to_string(),
            "src".to_string(),
            "src/".to_string(),
        ],
    }];

    let include_dirs = graph_include_dirs(
        project_root.as_path(),
        config_root.as_path(),
        projects.as_slice(),
    );

    assert_eq!(include_dirs, vec!["docs".to_string(), "src".to_string()]);
}

#[test]
fn gateway_bootstrap_background_indexing_defaults_to_disabled() {
    assert!(!gateway_bootstrap_background_indexing_with_lookup(&|_| {
        None
    }));
    assert!(!gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("invalid".to_string())
    }));
    assert!(!gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("false".to_string())
    }));
}

#[test]
fn gateway_bootstrap_background_indexing_accepts_truthy_env_values() {
    assert!(gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("true".to_string())
    }));
    assert!(gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some(" YES ".to_string())
    }));
    assert!(gateway_bootstrap_background_indexing_with_lookup(&|_| {
        Some("1".to_string())
    }));
}

#[test]
fn bootstrap_background_indexing_telemetry_reports_default_deferred_state() {
    let studio = StudioState::new();
    let telemetry = studio.bootstrap_background_indexing_telemetry();

    assert!(!telemetry.enabled());
    assert_eq!(telemetry.mode(), "deferred");
    assert!(!telemetry.deferred_activation_observed());
    assert_eq!(telemetry.deferred_activation_at(), None);
    assert_eq!(telemetry.deferred_activation_source(), None);
}
