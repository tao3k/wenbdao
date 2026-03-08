pub(in crate::link_graph::runtime_config) fn normalize_relative_dir(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    if normalized.is_empty() || normalized == "." {
        None
    } else {
        Some(normalized)
    }
}

pub(in crate::link_graph::runtime_config) fn dedup_dirs(entries: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in entries {
        let lowered = entry.to_lowercase();
        if seen.insert(lowered) {
            out.push(entry);
        }
    }
    out
}
