use crate::link_graph::runtime_config::models::LinkGraphIndexRuntimeConfig;
use crate::link_graph::runtime_config::settings::{
    dedup_dirs, get_setting_bool, get_setting_string_list, merged_wendao_settings,
    normalize_relative_dir,
};
use std::path::Path;

/// Resolve `LinkGraph` index scope from merged `wendao` settings.
///
/// Order:
/// 1) Explicit `link_graph.include_dirs`
/// 2) `link_graph.include_dirs_auto_candidates` when `include_dirs_auto=true`
///    and candidate directory exists under `root_dir`
/// 3) `link_graph.exclude_dirs` (non-hidden additions only)
#[must_use]
pub fn resolve_link_graph_index_runtime(root_dir: &Path) -> LinkGraphIndexRuntimeConfig {
    let settings = merged_wendao_settings();

    let explicit_include = dedup_dirs(
        get_setting_string_list(&settings, "link_graph.include_dirs")
            .into_iter()
            .filter_map(|item| normalize_relative_dir(&item))
            .collect(),
    );

    let include_dirs = if explicit_include.is_empty()
        && get_setting_bool(&settings, "link_graph.include_dirs_auto").unwrap_or(true)
    {
        dedup_dirs(
            get_setting_string_list(&settings, "link_graph.include_dirs_auto_candidates")
                .into_iter()
                .filter_map(|item| normalize_relative_dir(&item))
                .filter(|candidate| root_dir.join(candidate).is_dir())
                .collect(),
        )
    } else {
        explicit_include
    };

    let exclude_dirs = dedup_dirs(
        get_setting_string_list(&settings, "link_graph.exclude_dirs")
            .into_iter()
            .filter_map(|item| normalize_relative_dir(&item))
            .filter(|value| !value.starts_with('.'))
            .collect(),
    );

    LinkGraphIndexRuntimeConfig {
        include_dirs,
        exclude_dirs,
    }
}
