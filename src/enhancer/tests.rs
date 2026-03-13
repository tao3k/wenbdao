use crate::link_graph_refs::LinkGraphEntityRef;

use super::*;

#[test]
fn test_parse_frontmatter_basic() {
    let content =
        "---\ntitle: My Note\ndescription: A test\ntags:\n  - python\n  - rust\n---\n# Content";
    let fm = parse_frontmatter(content);
    assert_eq!(fm.title.as_deref(), Some("My Note"));
    assert_eq!(fm.description.as_deref(), Some("A test"));
    assert_eq!(fm.tags, vec!["python", "rust"]);
}

#[test]
fn test_parse_frontmatter_skill() {
    let content = "---\nname: git\ndescription: Git ops\nmetadata:\n  routing_keywords:\n    - commit\n    - branch\n  intents:\n    - version_control\n---\n# SKILL";
    let fm = parse_frontmatter(content);
    assert_eq!(fm.name.as_deref(), Some("git"));
    assert_eq!(fm.routing_keywords, vec!["commit", "branch"]);
    assert_eq!(fm.intents, vec!["version_control"]);
}

#[test]
fn test_parse_frontmatter_empty() {
    let fm = parse_frontmatter("# No frontmatter");
    assert!(fm.title.is_none());
    assert!(fm.tags.is_empty());
}

#[test]
fn test_parse_frontmatter_malformed() {
    let fm = parse_frontmatter("---\n: bad [[\n---\n");
    assert!(fm.title.is_none());
}

#[test]
fn test_infer_relations_documented_in() {
    let refs = vec![LinkGraphEntityRef::new(
        "Python".to_string(),
        None,
        "[[Python]]".to_string(),
    )];
    let fm = NoteFrontmatter::default();
    let relations = infer_relations("docs/test.md", "Test Doc", &fm, &refs);

    assert_eq!(relations.len(), 1);
    assert_eq!(relations[0].source, "Python");
    assert_eq!(relations[0].relation_type, "DOCUMENTED_IN");
}

#[test]
fn test_infer_relations_skill_contains() {
    let fm = NoteFrontmatter {
        name: Some("git".to_string()),
        ..Default::default()
    };
    let relations = infer_relations("assets/skills/git/SKILL.md", "Git Skill", &fm, &[]);

    let contains: Vec<_> = relations
        .iter()
        .filter(|r| r.relation_type == "CONTAINS")
        .collect();
    assert_eq!(contains.len(), 1);
    assert_eq!(contains[0].source, "git");
}

#[test]
fn test_infer_relations_tags() {
    let fm = NoteFrontmatter {
        tags: vec!["search".to_string(), "vector".to_string()],
        ..Default::default()
    };
    let relations = infer_relations("docs/test.md", "Test", &fm, &[]);

    let tag_rels: Vec<_> = relations
        .iter()
        .filter(|r| r.relation_type == "RELATED_TO")
        .collect();
    assert_eq!(tag_rels.len(), 2);
}

#[test]
fn test_enhance_note_full() {
    let input = NoteInput {
        path: "docs/test.md".to_string(),
        title: "Test Doc".to_string(),
        content: "---\ntitle: Test\ntags:\n  - demo\n---\nContent with [[Python#lang]] ref"
            .to_string(),
    };

    let result = enhance_note(&input);
    assert_eq!(result.frontmatter.title.as_deref(), Some("Test"));
    assert_eq!(result.entity_refs.len(), 1);
    assert_eq!(result.entity_refs[0].name, "Python");
    assert_eq!(result.entity_refs[0].entity_type.as_deref(), Some("lang"));
    assert!(result.ref_stats.total_refs >= 1);
    // DOCUMENTED_IN + RELATED_TO(tag:demo)
    assert!(result.inferred_relations.len() >= 2);
}

#[test]
fn test_enhance_notes_batch() {
    let inputs = vec![
        NoteInput {
            path: "a.md".to_string(),
            title: "A".to_string(),
            content: "About [[X]]".to_string(),
        },
        NoteInput {
            path: "b.md".to_string(),
            title: "B".to_string(),
            content: "About [[Y]] and [[Z]]".to_string(),
        },
    ];

    let results = enhance_notes_batch(&inputs);
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].entity_refs.len(), 1);
    assert_eq!(results[1].entity_refs.len(), 2);
}
