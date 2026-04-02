use super::types::MarkdownConfigLinkTarget;
use super::{
    MarkdownConfigBlock, MarkdownConfigMemoryIndex, extract_markdown_config_blocks,
    extract_markdown_config_link_targets_by_id,
};

#[test]
fn markdown_config_blocks_are_indexed_by_id() {
    let markdown = r#"
# Template Config
<!-- id: template-config, type: template, target: ./templates/base.md -->

```jinja2
Hello {{ name }}
```

# Persona Config
<!-- id: persona-config, type: persona -->

```toml
name = "agent"
```
"#;

    let blocks = extract_markdown_config_blocks(markdown);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].id, "template-config");
    assert_eq!(blocks[0].config_type, "template");
    assert_eq!(blocks[0].target.as_deref(), Some("./templates/base.md"));
    assert_eq!(blocks[0].heading, "Template Config");
    assert_eq!(blocks[0].language, "jinja2");
    assert!(blocks[0].content.contains("Hello"));
    assert_eq!(blocks[1].id, "persona-config");
    assert_eq!(blocks[1].config_type, "persona");
    assert_eq!(blocks[1].language, "toml");

    let mut index = MarkdownConfigMemoryIndex::from_markdown(markdown);
    assert_eq!(index.len(), 2);
    assert_eq!(
        index
            .get("template-config")
            .unwrap_or_else(|| panic!("template-config should exist"))
            .heading,
        "Template Config"
    );

    let replacement = MarkdownConfigBlock {
        id: "template-config".to_string(),
        config_type: "template".to_string(),
        target: Some("templates/override.md".to_string()),
        heading: "Override".to_string(),
        language: "jinja2".to_string(),
        content: "replacement".to_string(),
    };
    assert!(index.insert(replacement.clone()).is_some());
    assert_eq!(index.get("template-config"), Some(&replacement));
}

#[test]
fn markdown_config_link_targets_are_normalized_and_deduplicated() {
    let markdown = r"
# Link Config
<!-- id: link-config, type: template -->

[Guide](../docs/guide.md)
[Guide Again](../docs/guide.md)
![Diagram](../assets/diagram.png)
[[wendao://repo/sciml/repo-a/resources/notes#persona]]
[External](https://example.com)
";

    let links = extract_markdown_config_link_targets_by_id(markdown, "notes/section/page.md");
    assert_eq!(
        links.get("link-config"),
        Some(&vec![
            MarkdownConfigLinkTarget {
                target: "notes/docs/guide.md".to_string(),
                reference_type: None,
            },
            MarkdownConfigLinkTarget {
                target: "notes/assets/diagram.png".to_string(),
                reference_type: Some("attachment".to_string()),
            },
            MarkdownConfigLinkTarget {
                target: "wendao://repo/sciml/repo-a/resources/notes".to_string(),
                reference_type: Some("persona".to_string()),
            },
        ])
    );
}
