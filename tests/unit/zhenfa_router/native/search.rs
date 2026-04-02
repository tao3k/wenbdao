use super::*;

#[test]
fn wendao_search_args_deserialize_query_vector() {
    let args: WendaoSearchArgs = serde_json::from_value(serde_json::json!({
        "query": "native zhenfa",
        "query_vector": [1.0, 0.0, 0.0]
    }))
    .unwrap_or_else(|error| panic!("deserialize native args: {error}"));

    assert_eq!(args.query, "native zhenfa");
    assert_eq!(args.query_vector, Some(vec![1.0, 0.0, 0.0]));
}

#[test]
fn normalize_limit_defaults_and_clamps() {
    assert_eq!(normalize_limit(None), 20);
    assert_eq!(normalize_limit(Some(0)), 1);
    assert_eq!(normalize_limit(Some(42)), 42);
    assert_eq!(normalize_limit(Some(999)), 200);
}

#[test]
fn validate_root_dir_argument_accepts_real_paths() {
    assert!(validate_root_dir_argument(None).is_ok());
    assert!(validate_root_dir_argument(Some("docs")).is_ok());
    assert!(validate_root_dir_argument(Some("  docs ")).is_ok());
}

#[test]
fn validate_root_dir_argument_rejects_blank_values() {
    assert!(validate_root_dir_argument(Some("")).is_err());
    assert!(validate_root_dir_argument(Some("   ")).is_err());
}
