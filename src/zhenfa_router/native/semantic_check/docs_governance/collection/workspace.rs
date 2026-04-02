use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use super::package_docs::collect_doc_governance_issues;
use crate::zhenfa_router::native::semantic_check::docs_governance::parsing::{
    collect_index_body_links, collect_lines, parse_footer_block, parse_relations_links_line,
};
use crate::zhenfa_router::native::semantic_check::docs_governance::rendering::{
    link_target, plan_index_footer_block_insertion, plan_index_relations_block_insertion,
    plan_index_section_link_insertion, render_package_docs_index, render_section_landing_page,
    standard_section_specs,
};
use crate::zhenfa_router::native::semantic_check::docs_governance::scope::{
    scope_matches, scope_matches_doc,
};
use crate::zhenfa_router::native::semantic_check::docs_governance::types::{
    MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE, MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE, MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE,
};
use crate::zhenfa_router::native::semantic_check::{IssueLocation, SemanticIssue};

/// Collects workspace-wide doc governance issues.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn collect_workspace_doc_governance_issues(
    root: &Path,
    scope: Option<&str>,
) -> Vec<SemanticIssue> {
    let crates_dir = root.join("packages").join("rust").join("crates");
    let Ok(entries) = fs::read_dir(crates_dir) else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    for entry in entries.flatten() {
        let package_dir = entry.path();
        if !is_workspace_crate_dir(&package_dir) {
            continue;
        }

        let docs_dir = package_dir.join("docs");
        let index_path = docs_dir.join("index.md");

        let crate_name = package_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if !docs_dir.is_dir() {
            if scope_matches(scope, &package_dir, &docs_dir, &index_path) {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE.to_string(),
                    doc: index_path.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Missing documentation tree for package `{crate_name}`. Expected at `docs/`."
                    ),
                    location: None,
                    suggestion: Some(render_package_docs_index(
                        crate_name,
                        &index_path.to_string_lossy(),
                        &docs_dir,
                    )),
                    fuzzy_suggestion: None,
                });
            }
            continue;
        }

        for doc_entry in WalkDir::new(&docs_dir).into_iter().flatten() {
            let path = doc_entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                if !scope_matches_doc(scope, &package_dir, &docs_dir, path) {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(path) {
                    issues.extend(collect_doc_governance_issues(
                        &path.to_string_lossy(),
                        &content,
                    ));
                }
            }
        }

        if !scope_matches(scope, &package_dir, &docs_dir, &index_path) {
            continue;
        }

        if !index_path.is_file() {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE.to_string(),
                doc: index_path.to_string_lossy().into_owned(),
                node_id: crate_name.to_string(),
                message: format!(
                    "Missing documentation index for package `{crate_name}`. Expected at `docs/index.md`."
                ),
                location: None,
                suggestion: Some(render_package_docs_index(
                    crate_name,
                    &index_path.to_string_lossy(),
                    &docs_dir,
                )),
                fuzzy_suggestion: None,
            });
            continue;
        }

        let Ok(index_content) = fs::read_to_string(&index_path) else {
            continue;
        };

        let index_lines = collect_lines(&index_content);

        if parse_footer_block(&index_lines).is_none() {
            let (location, suggestion) = plan_index_footer_block_insertion(&index_content);
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE.to_string(),
                doc: index_path.to_string_lossy().into_owned(),
                node_id: crate_name.to_string(),
                message: "Missing mandatory :FOOTER: block in documentation index".to_string(),
                location: Some(location),
                suggestion: Some(suggestion),
                fuzzy_suggestion: None,
            });
        }

        let relations_links = parse_relations_links_line(&index_lines);
        let body_links = collect_index_body_links(&index_lines);

        if !body_links.is_empty() {
            match relations_links {
                None => {
                    let (location, suggestion) =
                        plan_index_relations_block_insertion(&index_content, &body_links);
                    issues.push(SemanticIssue {
                        severity: "warning".to_string(),
                        issue_type: MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE
                            .to_string(),
                        doc: index_path.to_string_lossy().into_owned(),
                        node_id: crate_name.to_string(),
                        message: format!("Missing mandatory :RELATIONS: block in documentation index with body links: {}",
                            body_links.iter().map(|l| format!("[[{l}]]")).collect::<Vec<_>>().join(", ")
                        ),
                        location: Some(location),
                        suggestion: Some(suggestion),
                        fuzzy_suggestion: None,
                    });
                }
                Some(links) => {
                    let mut missing_in_relations = Vec::new();
                    for body_link in &body_links {
                        if !links.value.contains(&format!("[[{body_link}]]")) {
                            missing_in_relations.push(body_link.clone());
                        }
                    }

                    if !missing_in_relations.is_empty() {
                        issues.push(SemanticIssue {
                            severity: "warning".to_string(),
                            issue_type: MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE
                                .to_string(),
                            doc: index_path.to_string_lossy().into_owned(),
                            node_id: crate_name.to_string(),
                            message: format!(
                                "Documentation links missing from :RELATIONS: block: {}",
                                missing_in_relations
                                    .iter()
                                    .map(|l| format!("[[{l}]]"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            location: Some(IssueLocation {
                                line: links.line,
                                heading_path: "Index Relations".to_string(),
                                byte_range: Some((links.value_start, links.value_end)),
                            }),
                            suggestion: Some(
                                body_links
                                    .iter()
                                    .map(|l| format!("[[{l}]]"))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                            fuzzy_suggestion: None,
                        });
                    }
                }
            }
        }

        let specs = standard_section_specs(crate_name);
        for spec in &specs {
            let section_dir = docs_dir.join(spec.section_name);
            let section_path = docs_dir.join(&spec.relative_path);

            if !scope_matches_doc(scope, &package_dir, &docs_dir, &section_path) {
                continue;
            }

            if !section_path.is_file() {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE.to_string(),
                    doc: section_path.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Missing mandatory section landing page for `{}` quadrant.",
                        spec.section_name
                    ),
                    location: None,
                    suggestion: Some(render_section_landing_page(
                        crate_name,
                        &package_dir,
                        &section_path.to_string_lossy(),
                        spec,
                    )),
                    fuzzy_suggestion: None,
                });
            }

            let target = link_target(&spec.relative_path);
            if !body_links.iter().any(|l| l == &target) {
                let (location, suggestion) =
                    plan_index_section_link_insertion(&index_content, spec, &target);
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE.to_string(),
                    doc: index_path.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Mandatory section `{}` is not linked in documentation index.",
                        spec.section_name
                    ),
                    location: Some(location),
                    suggestion: Some(suggestion),
                    fuzzy_suggestion: None,
                });
            }

            if !section_dir.is_dir() {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE.to_string(),
                    doc: section_dir.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Missing directory tree for `{}` documentation quadrant.",
                        spec.section_name
                    ),
                    location: None,
                    suggestion: None,
                    fuzzy_suggestion: None,
                });
            }
        }
    }

    issues
}

fn is_workspace_crate_dir(path: &Path) -> bool {
    path.join("Cargo.toml").is_file()
}
