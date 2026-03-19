use std::collections::HashSet;
use std::path::{Component, Path};

use regex::RegexBuilder;
use walkdir::{DirEntry, WalkDir};
use xiuxian_ast::{Lang, extract_items, get_skeleton_patterns};

use crate::dependency_indexer::extract_symbols;
use crate::gateway::studio::analysis;
use crate::gateway::studio::types::{
    AnalysisNodeKind, AstSearchHit, ReferenceSearchHit, StudioNavigationTarget, UiProjectConfig,
};
use crate::link_graph::parser::{ParsedSection, parse_note};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::project_scope::{
    configured_project_scan_roots, index_path_for_entry, project_metadata_for_path,
};
use super::support::{
    first_signature_line, infer_crate_name, score_reference_hit, source_language_label,
    symbol_kind_label,
};

pub(super) fn build_ast_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<Vec<AstSearchHit>, String> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();

    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            let normalized_path_ref = Path::new(normalized_path.as_str());
            if is_markdown_path(normalized_path_ref) {
                let content = std::fs::read_to_string(entry.path())
                    .map_err(|error| format!("{}: {error}", entry.path().display()))?;
                let crate_name = markdown_scope_name(normalized_path_ref);

                for hit in build_markdown_ast_hits(
                    root.as_path(),
                    entry.path(),
                    normalized_path.as_str(),
                    content.as_str(),
                    crate_name.as_str(),
                ) {
                    let dedupe_key = format!(
                        "{}:{}:{}:{}",
                        hit.path, hit.line_start, hit.line_end, hit.name
                    );
                    if seen.insert(dedupe_key) {
                        hits.push(hit);
                    }
                }
                continue;
            }

            let Some(lang) = ast_search_lang(normalized_path_ref) else {
                continue;
            };

            let content = std::fs::read_to_string(entry.path())
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;
            let crate_name = infer_crate_name(normalized_path_ref);

            for pattern in get_skeleton_patterns(lang) {
                for result in extract_items(content.as_str(), pattern, lang, Some(vec!["NAME"])) {
                    let name =
                        result.captures.get("NAME").cloned().unwrap_or_else(|| {
                            first_signature_line(result.text.as_str()).to_string()
                        });
                    let signature = first_signature_line(result.text.as_str()).to_string();
                    if signature.is_empty() {
                        continue;
                    }
                    let dedupe_key = format!(
                        "{normalized_path}:{}:{}:{name}",
                        result.line_start, result.line_end
                    );
                    if !seen.insert(dedupe_key) {
                        continue;
                    }

                    hits.push(AstSearchHit {
                        name,
                        signature,
                        path: normalized_path.clone(),
                        language: lang.as_str().to_string(),
                        crate_name: crate_name.clone(),
                        project_name: None,
                        root_label: None,
                        node_kind: None,
                        owner_title: None,
                        navigation_target: ast_navigation_target(
                            normalized_path.as_str(),
                            crate_name.as_str(),
                            None,
                            None,
                            result.line_start,
                            result.line_end,
                        ),
                        line_start: result.line_start,
                        line_end: result.line_end,
                        score: 0.0,
                    });
                }
            }
        }
    }

    Ok(hits)
}

pub(super) fn build_symbol_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<UnifiedSymbolIndex, String> {
    let mut index = UnifiedSymbolIndex::new();

    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }

            let Some(language) = source_language_label(entry.path()) else {
                continue;
            };
            let normalized_path = index_path_for_entry(project_root, entry.path());
            let crate_name = infer_crate_name(Path::new(normalized_path.as_str()));
            let symbols = extract_symbols(entry.path(), language)
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            for symbol in symbols {
                let location = format!("{normalized_path}:{}", symbol.line);
                index.add_project_symbol(
                    symbol.name.as_str(),
                    symbol_kind_label(&symbol.kind),
                    location.as_str(),
                    crate_name.as_str(),
                );
            }
        }
    }

    Ok(index)
}

pub(super) fn build_reference_hits(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    ast_hits: &[AstSearchHit],
    query: &str,
    limit: usize,
) -> Result<Vec<ReferenceSearchHit>, String> {
    let regex = RegexBuilder::new(format!(r"\b{}\b", regex::escape(query)).as_str())
        .case_insensitive(true)
        .build()
        .map_err(|error| error.to_string())?;
    let definition_locations = ast_hits
        .iter()
        .filter(|hit| hit.name.eq_ignore_ascii_case(query))
        .map(|hit| (hit.path.clone(), hit.line_start))
        .collect::<HashSet<_>>();

    let mut hits = Vec::new();

    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            let normalized_path_ref = Path::new(normalized_path.as_str());
            let Some(lang) = ast_search_lang(normalized_path_ref) else {
                continue;
            };
            let crate_name = infer_crate_name(normalized_path_ref);
            let content = std::fs::read_to_string(entry.path())
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            for (line_idx, line_text) in content.lines().enumerate() {
                let line_number = line_idx + 1;
                if definition_locations.contains(&(normalized_path.clone(), line_number)) {
                    continue;
                }

                let Some(mat) = regex.find(line_text) else {
                    continue;
                };
                let metadata = project_metadata_for_path(
                    project_root,
                    config_root,
                    projects,
                    normalized_path.as_str(),
                );
                let navigation_target = reference_navigation_target(
                    normalized_path.as_str(),
                    crate_name.as_str(),
                    metadata.project_name.as_deref(),
                    metadata.root_label.as_deref(),
                    line_number,
                    line_text[..mat.start()].chars().count() + 1,
                );

                hits.push(ReferenceSearchHit {
                    name: query.to_string(),
                    path: normalized_path.clone(),
                    language: lang.as_str().to_string(),
                    crate_name: crate_name.clone(),
                    project_name: metadata.project_name,
                    root_label: metadata.root_label,
                    navigation_target,
                    line: line_number,
                    column: line_text[..mat.start()].chars().count() + 1,
                    line_text: line_text.trim().to_string(),
                    score: score_reference_hit(line_text, query),
                });
            }
        }
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
    });
    hits.truncate(limit);

    Ok(hits)
}

fn should_skip_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git"
            | ".cache"
            | ".devenv"
            | ".direnv"
            | ".run"
            | "target"
            | "node_modules"
            | "dist"
            | "coverage"
            | "__pycache__"
    )
}

fn is_markdown_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

fn markdown_scope_name(path: &Path) -> String {
    path.components()
        .find_map(|component| match component {
            Component::Normal(segment) => segment.to_str().map(ToString::to_string),
            _ => None,
        })
        .filter(|segment| !segment.is_empty())
        .unwrap_or_else(|| "docs".to_string())
}

fn build_markdown_ast_hits(
    root: &Path,
    source_path: &Path,
    path: &str,
    content: &str,
    crate_name: &str,
) -> Vec<AstSearchHit> {
    let mut hits = analysis::compile_markdown_nodes(path, content)
        .into_iter()
        .filter_map(|node| {
            let signature = markdown_signature(node.kind, node.depth, node.label.as_str())?;
            Some(AstSearchHit {
                name: node.label,
                signature,
                path: path.to_string(),
                language: "markdown".to_string(),
                crate_name: crate_name.to_string(),
                project_name: None,
                root_label: None,
                node_kind: markdown_node_kind(node.kind).map(ToOwned::to_owned),
                owner_title: None,
                navigation_target: ast_navigation_target(
                    path,
                    crate_name,
                    None,
                    None,
                    node.line_start,
                    node.line_end,
                ),
                line_start: node.line_start,
                line_end: node.line_end,
                score: 0.0,
            })
        })
        .collect::<Vec<_>>();

    if let Some(parsed) = parse_note(source_path, root, content) {
        for section in &parsed.sections {
            hits.extend(build_markdown_property_hits(path, crate_name, section));
            hits.extend(build_markdown_observation_hits(path, crate_name, section));
        }
    }

    hits
}

fn markdown_signature(kind: AnalysisNodeKind, depth: usize, label: &str) -> Option<String> {
    match kind {
        AnalysisNodeKind::Section => Some(format!("{} {label}", "#".repeat(depth.clamp(1, 6)))),
        AnalysisNodeKind::Task => Some(format!("- [ ] {label}")),
        _ => None,
    }
}

fn markdown_node_kind(kind: AnalysisNodeKind) -> Option<&'static str> {
    match kind {
        AnalysisNodeKind::Section => Some("section"),
        AnalysisNodeKind::Task => Some("task"),
        _ => None,
    }
}

fn build_markdown_property_hits(
    path: &str,
    crate_name: &str,
    section: &ParsedSection,
) -> Vec<AstSearchHit> {
    let owner_title = markdown_owner_title(section);
    section
        .attributes
        .iter()
        .filter(|(key, _)| !is_observation_attribute(key.as_str()))
        .map(|(key, value)| AstSearchHit {
            name: key.clone(),
            signature: format!(":{key}: {value}"),
            path: path.to_string(),
            language: "markdown".to_string(),
            crate_name: crate_name.to_string(),
            project_name: None,
            root_label: None,
            node_kind: Some("property".to_string()),
            owner_title: owner_title.clone(),
            navigation_target: ast_navigation_target(
                path,
                crate_name,
                None,
                None,
                section.line_start,
                section.line_end,
            ),
            line_start: section.line_start,
            line_end: section.line_end,
            score: 0.0,
        })
        .collect()
}

fn build_markdown_observation_hits(
    path: &str,
    crate_name: &str,
    section: &ParsedSection,
) -> Vec<AstSearchHit> {
    let owner_title = markdown_owner_title(section);
    section
        .observations
        .iter()
        .map(|observation| AstSearchHit {
            name: "OBSERVE".to_string(),
            signature: format!(":OBSERVE: {}", observation.raw_value),
            path: path.to_string(),
            language: "markdown".to_string(),
            crate_name: crate_name.to_string(),
            project_name: None,
            root_label: None,
            node_kind: Some("observation".to_string()),
            owner_title: owner_title.clone(),
            navigation_target: ast_navigation_target(
                path,
                crate_name,
                None,
                None,
                section.line_start,
                section.line_end,
            ),
            line_start: section.line_start,
            line_end: section.line_end,
            score: 0.0,
        })
        .collect()
}

fn markdown_owner_title(section: &ParsedSection) -> Option<String> {
    if !section.heading_path.trim().is_empty() {
        Some(section.heading_path.clone())
    } else if !section.heading_title.trim().is_empty() {
        Some(section.heading_title.clone())
    } else {
        None
    }
}

fn is_observation_attribute(key: &str) -> bool {
    key == "OBSERVE" || key.starts_with("OBSERVE_")
}

fn ast_search_lang(path: &Path) -> Option<Lang> {
    match Lang::from_path(path)? {
        Lang::Python
        | Lang::Rust
        | Lang::JavaScript
        | Lang::TypeScript
        | Lang::Bash
        | Lang::Go
        | Lang::Java
        | Lang::C
        | Lang::Cpp
        | Lang::CSharp
        | Lang::Ruby
        | Lang::Swift
        | Lang::Kotlin
        | Lang::Lua
        | Lang::Php => Lang::from_path(path),
        _ => None,
    }
}

fn ast_navigation_target(
    path: &str,
    crate_name: &str,
    project_name: Option<&str>,
    root_label: Option<&str>,
    line_start: usize,
    line_end: usize,
) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: path.to_string(),
        category: "doc".to_string(),
        project_name: project_name
            .map(ToString::to_string)
            .or_else(|| Some(crate_name.to_string())),
        root_label: root_label.map(ToString::to_string),
        line: Some(line_start),
        line_end: Some(line_end),
        column: None,
    }
}

fn reference_navigation_target(
    path: &str,
    crate_name: &str,
    project_name: Option<&str>,
    root_label: Option<&str>,
    line: usize,
    column: usize,
) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: path.to_string(),
        category: "doc".to_string(),
        project_name: project_name
            .map(ToString::to_string)
            .or_else(|| Some(crate_name.to_string())),
        root_label: root_label.map(ToString::to_string),
        line: Some(line),
        line_end: Some(line),
        column: Some(column),
    }
}
