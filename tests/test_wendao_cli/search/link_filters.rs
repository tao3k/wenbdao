use crate::test_wendao_cli::support::wendao_cmd;
use serde_json::Value;

#[path = "link_filters_fixture_contract_support.rs"]
mod link_filters_fixture_contract_support;

use link_filters_fixture_contract_support::{
    SearchLinkFiltersFixture, assert_search_link_filters_fixture, payload_snapshot,
};

#[test]
fn test_wendao_search_link_filters_flags() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchLinkFiltersFixture::build("link_to_filter")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg(".md")
        .arg("--limit")
        .arg("10")
        .arg("--link-to")
        .arg("b")
        .arg("--sort-term")
        .arg("path_asc")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with link filters failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = payload_snapshot(&payload);
    assert_search_link_filters_fixture("link_to_filter", &actual);
    Ok(())
}

#[test]
fn test_wendao_search_related_ppr_flags() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = SearchLinkFiltersFixture::build("related_ppr_filter")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(fixture.root())
        .arg("search")
        .arg(".md")
        .arg("--limit")
        .arg("10")
        .arg("--related")
        .arg("b")
        .arg("--max-distance")
        .arg("2")
        .arg("--related-ppr-alpha")
        .arg("0.9")
        .arg("--related-ppr-max-iter")
        .arg("64")
        .arg("--related-ppr-tol")
        .arg("1e-6")
        .arg("--related-ppr-subgraph-mode")
        .arg("force")
        .arg("--sort-term")
        .arg("path_asc")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with related ppr flags failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let actual = payload_snapshot(&payload);
    assert_search_link_filters_fixture("related_ppr_filter", &actual);

    Ok(())
}
