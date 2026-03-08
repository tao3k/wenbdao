//! Fixture-backed contracts for embedded skill resource APIs.

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;

use std::collections::BTreeMap;

use xiuxian_wendao::{
    ZHIXING_SKILL_DOC_PATH, embedded_resource_text, embedded_resource_text_from_wendao_uri,
    embedded_skill_links_for_id, embedded_skill_links_for_reference_type,
    embedded_skill_links_index, embedded_skill_markdown,
};

use fixture_json_assertions::assert_json_fixture_eq;

#[test]
fn embedded_skill_resource_api_contract() -> Result<(), Box<dyn std::error::Error>> {
    let mut links_index = embedded_skill_links_index()?
        .into_iter()
        .map(|(id, mut links)| {
            links.sort();
            (id, links)
        })
        .collect::<BTreeMap<_, _>>();
    for links in links_index.values_mut() {
        links.sort();
    }

    let actual = serde_json::json!({
        "skill_doc_path": ZHIXING_SKILL_DOC_PATH,
        "skill_markdown_preview": content_excerpt(
            embedded_skill_markdown().ok_or_else(|| std::io::Error::other("missing embedded skill markdown"))?
        ),
        "resource_text_preview": {
            "skill_doc": content_excerpt(
                embedded_resource_text(ZHIXING_SKILL_DOC_PATH)
                    .ok_or_else(|| std::io::Error::other("missing skill doc resource text"))?
            ),
            "rules_md": content_excerpt(
                embedded_resource_text("zhixing/skills/agenda-management/references/rules.md")
                    .ok_or_else(|| std::io::Error::other("missing rules.md resource text"))?
            ),
            "rules_uri": content_excerpt(
                embedded_resource_text_from_wendao_uri(
                    "wendao://skills/agenda-management/references/rules.md"
                )
                .ok_or_else(|| std::io::Error::other("missing rules.md semantic resource text"))?
            ),
        },
        "links_index": links_index,
        "links_for_id": {
            "agenda_flow": embedded_skill_links_for_id("agenda_flow")?,
            "draft_agenda.j2": embedded_skill_links_for_id("draft_agenda.j2")?,
            "missing": embedded_skill_links_for_id("missing")?,
        },
        "links_for_reference_type": {
            "template": embedded_skill_links_for_reference_type("template")?,
            "persona": embedded_skill_links_for_reference_type("persona")?,
            "qianji-flow": embedded_skill_links_for_reference_type("qianji-flow")?,
        },
    });

    assert_json_fixture_eq(
        "wendao_registry/embedded_skill_api/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

fn content_excerpt(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(10)
        .map(ToOwned::to_owned)
        .collect()
}
