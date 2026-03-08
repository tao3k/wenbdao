fn section_heading_parts(heading_path: &str) -> Vec<&str> {
    heading_path
        .split(" / ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

pub(in crate::link_graph::index) fn section_tree_distance(
    left_heading: &str,
    right_heading: &str,
) -> usize {
    let left = section_heading_parts(left_heading);
    let right = section_heading_parts(right_heading);
    if left.is_empty() || right.is_empty() {
        return if left.is_empty() && right.is_empty() {
            0
        } else {
            usize::MAX
        };
    }

    let mut lca_len = 0usize;
    let min_len = left.len().min(right.len());
    while lca_len < min_len && left[lca_len] == right[lca_len] {
        lca_len += 1;
    }
    (left.len() - lca_len) + (right.len() - lca_len)
}

pub(in crate::link_graph::index) fn normalize_with_case(
    value: &str,
    case_sensitive: bool,
) -> String {
    if case_sensitive {
        value.to_string()
    } else {
        value.to_lowercase()
    }
}

pub(in crate::link_graph::index) fn tokenize(value: &str, case_sensitive: bool) -> Vec<String> {
    value
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|token| normalize_with_case(token, case_sensitive))
        .collect()
}
