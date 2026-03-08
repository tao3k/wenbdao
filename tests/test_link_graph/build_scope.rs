use super::build_scope_fixture_support::{
    BuildScopeFixture, assert_build_scope_fixture, docs_snapshot, stats_and_toc_snapshot,
};
use xiuxian_wendao::link_graph::LinkGraphIndex;

#[test]
fn test_link_graph_build_with_excluded_dirs_skips_cache_tree()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = BuildScopeFixture::build("excluded_dirs")?;
    let excluded = vec![".cache".to_string()];
    let index = LinkGraphIndex::build_with_excluded_dirs(fixture.root(), &excluded)
        .map_err(|e| e.clone())?;

    let actual = stats_and_toc_snapshot(index.stats(), index.toc(10).as_slice());
    assert_build_scope_fixture("excluded_dirs", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_build_skips_hidden_dirs_by_default() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = BuildScopeFixture::build("hidden_dirs")?;
    let index = LinkGraphIndex::build(fixture.root()).map_err(|e| e.clone())?;

    let actual = stats_and_toc_snapshot(index.stats(), index.toc(10).as_slice());
    assert_build_scope_fixture("hidden_dirs", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_build_with_include_dirs_limits_scope() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = BuildScopeFixture::build("include_dirs")?;
    let include = vec!["docs".to_string()];
    let index =
        LinkGraphIndex::build_with_filters(fixture.root(), &include, &[]).map_err(|e| e.clone())?;

    let actual = stats_and_toc_snapshot(index.stats(), index.toc(10).as_slice());
    assert_build_scope_fixture("include_dirs", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_build_promotes_skill_metadata_into_skill_doc_tags()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = BuildScopeFixture::build("skill_metadata")?;
    let index = LinkGraphIndex::build(fixture.root()).map_err(|e| e.clone())?;
    let skill_docs = index
        .toc(10)
        .into_iter()
        .filter(|doc| doc.path == "skills/demo/SKILL.md")
        .collect::<Vec<_>>();

    let actual = docs_snapshot(skill_docs.as_slice());
    assert_build_scope_fixture("skill_metadata", "result.json", &actual);
    Ok(())
}
