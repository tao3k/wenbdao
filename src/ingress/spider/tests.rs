use super::content::build_document_description;
use super::locking::lock_slot_for_hash;
use super::url::{canonical_web_uri, web_namespace_from_url};

fn ok_or_panic<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

#[test]
fn canonical_web_uri_normalizes_absolute_url_and_namespace() {
    let uri = ok_or_panic(
        canonical_web_uri("https://docs.rs/spider/latest/spider/?q=1#frag"),
        "canonical uri should parse",
    );
    assert_eq!(
        uri,
        "wendao://web/https://docs.rs/spider/latest/spider/?q=1"
    );
    let namespace = ok_or_panic(
        web_namespace_from_url("https://docs.rs/spider/latest/spider/?q=1#frag"),
        "namespace should parse",
    );
    assert_eq!(namespace, "docs.rs");
}

#[test]
fn canonical_web_uri_rejects_non_http_scheme() {
    let Err(error) = canonical_web_uri("file:///tmp/index.html") else {
        panic!("must fail");
    };
    assert!(matches!(
        error,
        super::errors::SpiderIngressError::UnsupportedWebScheme { .. }
    ));
}

#[test]
fn build_document_description_uses_title_and_first_content_line() {
    let description = build_document_description(Some("Guide"), "\n\nAlpha\nBeta");
    assert_eq!(description, "Guide: Alpha");
}

#[test]
fn lock_slot_for_hash_is_bounded_by_segment_count() {
    let slot = lock_slot_for_hash("same-hash", 16);
    assert!(slot < 16);
    assert_eq!(slot, lock_slot_for_hash("same-hash", 16));
}
