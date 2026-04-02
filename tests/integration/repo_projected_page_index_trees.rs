//! Integration snapshot for projected page-index trees.

use insta::assert_json_snapshot;
use std::collections::BTreeMap;
use xiuxian_wendao::analyzers::{
    DocRecord, ExampleRecord, ModuleRecord, RelationKind, RelationRecord, RepoSymbolKind,
    RepositoryAnalysisOutput, RepositoryRecord, SymbolRecord, build_projected_page_index_trees,
};

#[test]
fn builds_projected_page_index_trees_from_stage_one_records() {
    let analysis = RepositoryAnalysisOutput {
        repository: Some(RepositoryRecord {
            repo_id: "demo".to_string(),
            name: "Demo".to_string(),
            path: "/tmp/demo".to_string(),
            url: None,
            revision: Some("abc123".to_string()),
            version: None,
            uuid: None,
            dependencies: Vec::new(),
        }),
        modules: vec![ModuleRecord {
            repo_id: "demo".to_string(),
            module_id: "repo:demo:module:Demo.Controllers".to_string(),
            qualified_name: "Demo.Controllers".to_string(),
            path: "Controllers/package.mo".to_string(),
        }],
        symbols: vec![SymbolRecord {
            repo_id: "demo".to_string(),
            symbol_id: "repo:demo:symbol:Demo.Controllers.PI".to_string(),
            module_id: Some("repo:demo:module:Demo.Controllers".to_string()),
            name: "PI".to_string(),
            qualified_name: "Demo.Controllers.PI".to_string(),
            kind: RepoSymbolKind::Type,
            path: "Controllers/PI.mo".to_string(),
            line_start: None,
            line_end: None,
            signature: None,
            audit_status: None,
            verification_state: None,
            attributes: BTreeMap::new(),
        }],
        imports: Vec::new(),
        examples: vec![ExampleRecord {
            repo_id: "demo".to_string(),
            example_id: "repo:demo:example:Controllers/Examples/Step.mo".to_string(),
            title: "Step".to_string(),
            path: "Controllers/Examples/Step.mo".to_string(),
            summary: None,
        }],
        docs: vec![
            DocRecord {
                repo_id: "demo".to_string(),
                doc_id: "repo:demo:doc:Controllers/UsersGuide/Tutorial/FirstSteps.mo".to_string(),
                title: "First Steps".to_string(),
                path: "Controllers/UsersGuide/Tutorial/FirstSteps.mo".to_string(),
                format: Some("modelica_users_guide_tutorial".to_string()),
            },
            DocRecord {
                repo_id: "demo".to_string(),
                doc_id: "repo:demo:doc:Controllers/PI.mo#annotation.documentation".to_string(),
                title: "PI documentation".to_string(),
                path: "Controllers/PI.mo#annotation.documentation".to_string(),
                format: Some("modelica_annotation".to_string()),
            },
        ],
        relations: vec![
            RelationRecord {
                repo_id: "demo".to_string(),
                source_id: "repo:demo:doc:Controllers/UsersGuide/Tutorial/FirstSteps.mo"
                    .to_string(),
                target_id: "repo:demo:module:Demo.Controllers".to_string(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: "demo".to_string(),
                source_id: "repo:demo:doc:Controllers/PI.mo#annotation.documentation".to_string(),
                target_id: "repo:demo:symbol:Demo.Controllers.PI".to_string(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: "demo".to_string(),
                source_id: "repo:demo:example:Controllers/Examples/Step.mo".to_string(),
                target_id: "repo:demo:module:Demo.Controllers".to_string(),
                kind: RelationKind::ExampleOf,
            },
        ],
        diagnostics: Vec::new(),
    };

    let trees = build_projected_page_index_trees(&analysis).expect("projected trees build");

    insta::with_settings!({
        snapshot_path => "../snapshots",
        prepend_module_to_snapshot => false,
    }, {
        assert_json_snapshot!(
            "repo_projected_page_index_trees__repo_projected_page_index_trees",
            trees
        );
    });
}
