use std::path::{Component, Path};

use crate::dependency_indexer::SymbolKind;

pub(in crate::gateway::studio::search) fn infer_crate_name(relative_path: &Path) -> String {
    let components = relative_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();

    match components.as_slice() {
        [packages, rust, crates, crate_name, ..]
            if packages == "packages" && rust == "rust" && crates == "crates" =>
        {
            crate_name.clone()
        }
        [packages, python, package_name, ..] if packages == "packages" && python == "python" => {
            package_name.clone()
        }
        [data, workspace_name, ..] if data == ".data" => workspace_name.clone(),
        [skills, skill_name, ..] if skills == "internal_skills" => skill_name.clone(),
        [first, ..] => first.clone(),
        [] => "workspace".to_string(),
    }
}

pub(in crate::gateway::studio::search) fn source_language_label(
    path: &Path,
) -> Option<&'static str> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Some("rust"),
        Some("py") => Some("python"),
        _ => None,
    }
}

pub(in crate::gateway::studio::search) fn first_signature_line(text: &str) -> &str {
    text.lines().next().map(str::trim).unwrap_or_default()
}

pub(in crate::gateway::studio::search) fn score_reference_hit(line_text: &str, query: &str) -> f64 {
    let normalized_line = line_text.trim();
    if normalized_line.contains(query) {
        0.9
    } else if normalized_line
        .to_ascii_lowercase()
        .contains(query.to_ascii_lowercase().as_str())
    {
        0.82
    } else {
        0.7
    }
}

pub(in crate::gateway::studio::search) fn symbol_kind_label(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Trait => "trait",
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Field => "field",
        SymbolKind::Impl => "impl",
        SymbolKind::Mod => "module",
        SymbolKind::Const => "const",
        SymbolKind::Static => "static",
        SymbolKind::TypeAlias => "type_alias",
        SymbolKind::Unknown => "unknown",
    }
}
