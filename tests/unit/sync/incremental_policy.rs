use std::path::Path;

use xiuxian_wendao::sync::IncrementalSyncPolicy;

#[test]
fn test_incremental_sync_policy_supports_configured_extensions() {
    let configured_extensions = vec![
        "md".to_string(),
        ".org".to_string(),
        "J2".to_string(),
        "toml".to_string(),
    ];
    let policy = IncrementalSyncPolicy::new(&configured_extensions);

    assert!(policy.supports_path(Path::new("note.md")));
    assert!(policy.supports_path(Path::new("agenda.org")));
    assert!(policy.supports_path(Path::new("template.j2")));
    assert!(policy.supports_path(Path::new("config.toml")));
    assert!(!policy.supports_path(Path::new("README.txt")));
}

#[test]
fn test_incremental_policy_prefers_explicit_extensions_over_patterns() {
    let patterns = vec!["**/*.md".to_string()];
    let explicit = vec!["toml".to_string(), "j2".to_string()];
    let policy = IncrementalSyncPolicy::from_patterns_and_extensions(
        &patterns,
        &explicit,
        &["md", "markdown", "org"],
    );

    assert!(policy.supports_path(Path::new("task.toml")));
    assert!(policy.supports_path(Path::new("template.j2")));
    assert!(!policy.supports_path(Path::new("note.md")));
}
