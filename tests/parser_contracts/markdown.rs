use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use xiuxian_wendao::{
    NoteInput, build_embedded_wendao_registry, embedded_skill_markdown, enhance_note,
    extract_markdown_config_blocks, extract_markdown_config_link_targets_by_id, parse_frontmatter,
};

use crate::fixture_json_assertions::assert_json_fixture_eq;

#[derive(Debug, Serialize)]
struct SkillRegistrySnapshot {
    skill_name: Option<String>,
    links_by_id: BTreeMap<String, Vec<String>>,
    templates: Vec<String>,
    personas: Vec<String>,
    knowledge: Vec<String>,
    qianji_flows: Vec<String>,
}

#[test]
fn skill_registry_projection_contract() -> Result<(), Box<dyn std::error::Error>> {
    let registry = build_embedded_wendao_registry()?;
    let skill_markdown = embedded_skill_markdown()
        .ok_or_else(|| std::io::Error::other("missing embedded SKILL.md payload for zhixing"))?;
    let frontmatter = parse_frontmatter(skill_markdown);
    let skill_file = registry
        .file("zhixing/skills/agenda-management/SKILL.md")
        .ok_or_else(|| {
            std::io::Error::other(
                "missing zhixing/skills/agenda-management/SKILL.md registry entry",
            )
        })?;

    let mut links_by_id = skill_file
        .links_by_id()
        .clone()
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    for links in links_by_id.values_mut() {
        links.sort();
    }

    let mut templates = skill_file.links_for_reference_type("template");
    templates.sort();
    let mut personas = skill_file.links_for_reference_type("persona");
    personas.sort();
    let mut knowledge = skill_file.links_for_reference_type("knowledge");
    knowledge.sort();
    let mut qianji_flows = skill_file.links_for_reference_type("qianji-flow");
    qianji_flows.sort();

    let actual = serde_json::to_value(SkillRegistrySnapshot {
        skill_name: frontmatter.name,
        links_by_id,
        templates,
        personas,
        knowledge,
        qianji_flows,
    })?;
    assert_json_fixture_eq(
        "parser/markdown/skill_registry/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

#[test]
fn reference_relation_extraction_contract() {
    let content = r"---
metadata:
  title: Snapshot Persona
  tags:
    - agenda
---

# Snapshot Persona

See [[rules#knowledge]] and [[agenda_flow.toml#qianji-flow]].
";

    let enhanced = enhance_note(&NoteInput {
        path: "zhixing/skills/agenda-management/references/steward.md".to_string(),
        title: "Snapshot Persona".to_string(),
        content: content.to_string(),
    });

    let actual = json!({
        "entity_refs": enhanced.entity_refs,
        "inferred_relations": enhanced.inferred_relations,
    });
    assert_json_fixture_eq(
        "parser/markdown/reference_relations/expected",
        "result.json",
        &actual,
    );
}

#[test]
fn frontmatter_parse_contract() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = r"---
name: parser-snapshot-skill
description: Markdown parser snapshot fixture
metadata:
  routing_keywords:
    - planner
    - execution
  intents:
    - Build resilient schedule
    - Critique agenda quality
  tags:
    - parser
    - markdown
---

# Fixture
";
    let frontmatter = parse_frontmatter(markdown);
    let actual = serde_json::to_value(frontmatter)?;
    assert_json_fixture_eq(
        "parser/markdown/frontmatter/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

#[test]
fn markdown_config_blocks_contract() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = r#"
## Steward Persona
<!-- id: "steward", type: "persona", target: "steward.md" -->

```toml
name = "steward"
```

## Draft Template
<!-- id: "draft", type: "template", target: "draft_agenda.j2" -->

```jinja2
<agenda_draft>{{ user_input }}</agenda_draft>
```
"#;
    let blocks = extract_markdown_config_blocks(markdown);
    let actual = serde_json::to_value(blocks)?;
    assert_json_fixture_eq(
        "parser/markdown/config_blocks/expected",
        "result.json",
        &actual,
    );
    Ok(())
}

#[test]
fn markdown_link_targets_contract() -> Result<(), Box<dyn std::error::Error>> {
    let markdown = r#"
## Persona
<!-- id: "steward", type: "persona", target: "steward.md" -->
[[references/steward.md#persona]]
![Logo](references/logo.png)

## Template
<!-- id: "draft", type: "template", target: "draft_agenda.j2" -->
[[references/draft_agenda.j2#template]]
[Rules](references/rules.md)
"#;
    let links = extract_markdown_config_link_targets_by_id(
        markdown,
        "zhixing/skills/agenda-management/SKILL.md",
    );
    let ordered = links.into_iter().collect::<BTreeMap<_, _>>();
    let actual = serde_json::to_value(ordered)?;
    assert_json_fixture_eq(
        "parser/markdown/link_targets/expected",
        "result.json",
        &actual,
    );
    Ok(())
}
