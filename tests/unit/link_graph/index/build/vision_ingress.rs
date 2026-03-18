//! Unit tests for vision_ingress module.

use super::*;

#[tokio::test]
async fn test_noop_provider_returns_none() {
    let provider = VisionProvider::noop();
    let path = PathBuf::from("/nonexistent.png");
    let result = provider.analyze(&path).await;
    assert!(result.is_none());
}

#[test]
fn test_dots_provider_status() {
    let provider = VisionProvider::dots();
    // Status should return a string (either reason or path)
    let _status = provider.status();
}

#[test]
fn test_extract_entities() {
    let text =
        r#"This is a diagram showing `MyClass` and `my_function` interacting with SomeModule"#;
    let entities = extract_entities(text);

    assert!(entities.contains(&"MyClass".to_string()));
    assert!(entities.contains(&"my_function".to_string()));
    assert!(entities.contains(&"SomeModule".to_string()));
}

#[test]
fn test_cross_modal_edges_empty() {
    let annotations = HashMap::new();
    let doc_ids: Vec<String> = Vec::new();

    let edges = build_cross_modal_edges(&annotations, &doc_ids);
    assert!(edges.is_empty());
}

#[test]
fn test_cross_modal_edges_basic() {
    let mut annotations = HashMap::new();
    annotations.insert(
        "image1.png".to_string(),
        VisionAnnotation {
            description: "Rust performance optimization diagram".to_string(),
            confidence: 0.9,
            entities: vec!["rust".to_string(), "performance".to_string()],
            annotated_at: 0,
        },
    );

    let doc_ids = vec!["rust.md".to_string(), "performance.md".to_string()];

    let edges = build_cross_modal_edges(&annotations, &doc_ids);
    assert_eq!(edges.len(), 1);
    assert!(edges.contains_key("image1.png"));
}
