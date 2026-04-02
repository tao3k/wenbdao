use std::path::Path;

use walkdir::WalkDir;

use crate::dependency_indexer::extract_symbols;
use crate::gateway::studio::types::UiProjectConfig;
use crate::unified_symbol::UnifiedSymbolIndex;

use super::super::project_scope::{configured_project_scan_roots, index_path_for_entry};
use super::super::support::{infer_crate_name, source_language_label, symbol_kind_label};
use super::filters::should_skip_entry;

pub(crate) fn build_symbol_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> UnifiedSymbolIndex {
    let mut index = UnifiedSymbolIndex::new();

    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }

            let Some(language) = source_language_label(entry.path()) else {
                continue;
            };
            let normalized_path = index_path_for_entry(project_root, entry.path());
            let crate_name = infer_crate_name(Path::new(normalized_path.as_str()));

            if let Ok(symbols) = extract_symbols(entry.path(), language) {
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
    }

    index
}
