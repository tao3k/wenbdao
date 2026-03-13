#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

#[tokio::test]
async fn test_save_and_load_valkey_roundtrip() {
    if !has_valkey() {
        return;
    }
    let temp_dir = TempDir::new().unwrap();
    let scope_key = temp_dir
        .path()
        .join("knowledge")
        .to_string_lossy()
        .into_owned();

    // Build graph
    let graph = KnowledgeGraph::new();

    let mut entity1 = Entity::new(
        "tool:python".to_string(),
        "Python".to_string(),
        EntityType::Skill,
        "Programming language".to_string(),
    );
    entity1.aliases = vec!["py".to_string(), "python3".to_string()];
    entity1.confidence = 0.95;

    let mut entity2 = Entity::new(
        "tool:claude-code".to_string(),
        "Claude Code".to_string(),
        EntityType::Tool,
        "AI coding assistant".to_string(),
    );
    entity2.vector = Some(vec![0.1; 128]);

    graph.add_entity(entity1).unwrap();
    graph.add_entity(entity2).unwrap();

    let relation = Relation::new(
        "Claude Code".to_string(),
        "Python".to_string(),
        RelationType::Uses,
        "Claude Code uses Python".to_string(),
    )
    .with_confidence(0.8);
    graph.add_relation(relation).unwrap();

    graph.save_to_valkey(&scope_key, 128).unwrap();

    // Load into new graph
    let mut graph2 = KnowledgeGraph::new();
    graph2.load_from_valkey(&scope_key).unwrap();

    // Verify entity counts
    let stats = graph2.get_stats();
    assert_eq!(stats.total_entities, 2, "Should have 2 entities");
    assert_eq!(stats.total_relations, 1, "Should have 1 relation");

    // Verify entity data
    let python = graph2.get_entity_by_name("Python").unwrap();
    assert_eq!(python.aliases.len(), 2);
    assert!(python.aliases.contains(&"py".to_string()));
    assert_eq!(python.confidence, 0.95);
    assert!(
        python.vector.is_none(),
        "Python entity should have no vector"
    );

    let claude = graph2.get_entity_by_name("Claude Code").unwrap();
    assert!(
        claude.vector.is_some(),
        "Claude entity should have a vector"
    );
    assert_eq!(claude.vector.as_ref().unwrap().len(), 128);

    // Verify relation data
    let rels = graph2.get_relations(None, None);
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].source, "Claude Code");
    assert_eq!(rels[0].target, "Python");
    assert_eq!(rels[0].confidence, 0.8);
}

#[tokio::test]
async fn test_valkey_persistence_with_skill_registration() {
    if !has_valkey() {
        return;
    }
    let temp_dir = TempDir::new().unwrap();
    let scope_key = temp_dir
        .path()
        .join("knowledge")
        .to_string_lossy()
        .into_owned();

    let graph = KnowledgeGraph::new();

    let docs = vec![
        SkillDoc {
            id: "git".to_string(),
            doc_type: "skill".to_string(),
            skill_name: "git".to_string(),
            tool_name: String::new(),
            content: "Git operations".to_string(),
            routing_keywords: vec![],
        },
        SkillDoc {
            id: "git.commit".to_string(),
            doc_type: "command".to_string(),
            skill_name: "git".to_string(),
            tool_name: "git.commit".to_string(),
            content: "Create a commit".to_string(),
            routing_keywords: vec!["commit".to_string(), "git".to_string()],
        },
    ];
    graph.register_skill_entities(&docs).unwrap();

    let stats_before = graph.get_stats();

    graph.save_to_valkey(&scope_key, 1024).unwrap();

    let mut graph2 = KnowledgeGraph::new();
    graph2.load_from_valkey(&scope_key).unwrap();

    let stats_after = graph2.get_stats();
    assert_eq!(stats_before.total_entities, stats_after.total_entities);
    assert_eq!(stats_before.total_relations, stats_after.total_relations);

    // Verify search still works after roundtrip
    let results = graph2.search_entities("git", 10);
    assert!(
        !results.is_empty(),
        "Search should find git entities after Valkey roundtrip"
    );
}
