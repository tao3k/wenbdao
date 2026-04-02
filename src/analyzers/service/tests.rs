use std::path::Path;

use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
use crate::analyzers::query::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::analyzers::records::RepositoryRecord;
use crate::git::checkout::LocalCheckoutMetadata;

use super::merge::{hydrate_repository_record, merge_repository_record};

#[test]
fn test_refine_contract_serialization() {
    let req = RefineEntityDocRequest {
        repo_id: "test".to_string(),
        entity_id: "sym1".to_string(),
        user_hints: Some("more details".to_string()),
    };
    let res = RefineEntityDocResponse {
        repo_id: "test".to_string(),
        entity_id: "sym1".to_string(),
        refined_content: "Refined".to_string(),
        verification_state: "verified".to_string(),
    };
    assert_eq!(req.repo_id, "test");
    assert_eq!(res.verification_state, "verified");
}

#[test]
fn merge_repository_record_prefers_overlay_metadata() {
    let base = RepositoryRecord {
        repo_id: "demo".to_string(),
        name: "demo".to_string(),
        path: "/tmp/demo".to_string(),
        url: Some("https://base.invalid/demo.git".to_string()),
        revision: Some("base-rev".to_string()),
        version: None,
        uuid: None,
        dependencies: Vec::new(),
    };
    let overlay = RepositoryRecord {
        repo_id: "demo".to_string(),
        name: "DemoPkg".to_string(),
        path: "/tmp/demo".to_string(),
        url: None,
        revision: None,
        version: Some("0.1.0".to_string()),
        uuid: Some("uuid-demo".to_string()),
        dependencies: vec!["LinearAlgebra".to_string()],
    };

    let merged = merge_repository_record(base, overlay);

    assert_eq!(merged.name, "DemoPkg");
    assert_eq!(merged.url.as_deref(), Some("https://base.invalid/demo.git"));
    assert_eq!(merged.revision.as_deref(), Some("base-rev"));
    assert_eq!(merged.version.as_deref(), Some("0.1.0"));
    assert_eq!(merged.uuid.as_deref(), Some("uuid-demo"));
    assert_eq!(merged.dependencies, vec!["LinearAlgebra".to_string()]);
}

#[test]
fn hydrate_repository_record_backfills_checkout_metadata() {
    let repository = RegisteredRepository {
        id: "sample".to_string(),
        path: Some("/tmp/sample".into()),
        url: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        git_ref: None,
        plugins: Vec::new(),
    };
    let mut record = RepositoryRecord {
        repo_id: String::new(),
        name: String::new(),
        path: String::new(),
        url: None,
        revision: None,
        version: None,
        uuid: None,
        dependencies: Vec::new(),
    };

    hydrate_repository_record(
        &mut record,
        &repository,
        Path::new("/tmp/sample"),
        Some(&LocalCheckoutMetadata {
            revision: Some("abc123".to_string()),
            remote_url: Some("https://example.invalid/sample.git".to_string()),
        }),
    );

    assert_eq!(record.repo_id, "sample");
    assert_eq!(record.name, "sample");
    assert_eq!(record.path, "/tmp/sample");
    assert_eq!(
        record.url.as_deref(),
        Some("https://example.invalid/sample.git")
    );
    assert_eq!(record.revision.as_deref(), Some("abc123"));
}
