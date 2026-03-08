use super::*;

#[test]
fn test_link_graph_extracts_markdown_links_relative_and_anchor()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AttachmentFixture::build("relative_and_anchor")?;
    let index = fixture.build_index()?;

    let actual = stats_and_neighbors_snapshot(
        index.stats(),
        index
            .neighbors("a", LinkGraphDirection::Both, 1, 10)
            .as_slice(),
    );
    assert_markdown_attachment_fixture("relative_and_anchor", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_extracts_markdown_reference_links() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = AttachmentFixture::build("reference_links")?;
    let index = fixture.build_index()?;

    let actual = stats_and_neighbors_snapshot(
        index.stats(),
        index
            .neighbors("a", LinkGraphDirection::Both, 1, 10)
            .as_slice(),
    );
    assert_markdown_attachment_fixture("reference_links", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_uses_comrak_for_complex_markdown_links() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = AttachmentFixture::build("complex_markdown_links")?;
    let index = fixture.build_index()?;

    let actual = stats_and_neighbors_snapshot(
        index.stats(),
        index
            .neighbors("a", LinkGraphDirection::Both, 1, 10)
            .as_slice(),
    );
    assert_markdown_attachment_fixture("complex_markdown_links", "result.json", &actual);
    Ok(())
}

#[test]
fn test_link_graph_ignores_attachment_links_and_inline_embedded_wikilinks()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AttachmentFixture::build("ignore_attachments_and_inline_embeds")?;
    let index = fixture.build_index()?;

    let actual = stats_and_neighbors_snapshot(
        index.stats(),
        index
            .neighbors("a", LinkGraphDirection::Both, 1, 10)
            .as_slice(),
    );
    assert_markdown_attachment_fixture(
        "ignore_attachments_and_inline_embeds",
        "result.json",
        &actual,
    );
    Ok(())
}

#[test]
fn test_link_graph_attachment_search_filters_by_kind_and_extension()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = AttachmentFixture::build("attachment_search_filters")?;
    let index = fixture.build_index()?;

    let image_hits =
        index.search_attachments("", 20, &[], &[LinkGraphAttachmentKind::Image], false);
    let pdf_hits = index.search_attachments("", 20, &["pdf".to_string()], &[], false);

    let actual = json!({
        "image_hits": attachment_hits_snapshot(image_hits.as_slice()),
        "pdf_hits": attachment_hits_snapshot(pdf_hits.as_slice()),
    });
    assert_markdown_attachment_fixture("attachment_search_filters", "result.json", &actual);
    Ok(())
}
