/// Find the closing quote in a string, handling escaped quotes.
pub(super) fn find_closing_quote(s: &str) -> Option<usize> {
    let mut chars = s.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '\\' {
            // Skip the next character (escaped).
            chars.next();
            continue;
        }
        if ch == '"' {
            return Some(i);
        }
    }

    None
}

/// Check if a file path matches a scope glob pattern.
///
/// # Supported Patterns
///
/// - `**` - Match any number of directories
/// - `*` - Match any single path segment (no slashes)
/// - `?` - Match any single character
/// - Literal characters match themselves
///
/// # Examples
///
/// ```
/// use xiuxian_wendao::link_graph::parser::code_observation::path_matches_scope;
///
/// assert!(path_matches_scope("src/api/handler.rs", "src/api/**"));
/// assert!(path_matches_scope("src/api/handler.rs", "**/*.rs"));
/// assert!(path_matches_scope("packages/core/src/lib.rs", "packages/core/**/*.rs"));
/// assert!(!path_matches_scope("src/api/handler.rs", "src/db/**"));
/// ```
#[must_use]
pub fn path_matches_scope(file_path: &str, scope: &str) -> bool {
    // Normalize paths to use forward slashes.
    let normalized_path = file_path.replace('\\', "/");
    let normalized_scope = scope.replace('\\', "/");

    // Handle ** glob pattern.
    if normalized_scope.contains("**") {
        match_glob_with_double_star(&normalized_path, &normalized_scope)
    } else {
        // Simple glob matching for patterns without **.
        match_simple_glob(&normalized_path, &normalized_scope)
    }
}

/// Match a path against a glob pattern containing **.
fn match_glob_with_double_star(path: &str, pattern: &str) -> bool {
    // Split pattern by **.
    let parts: Vec<&str> = pattern.split("**").collect();

    if parts.is_empty() {
        return true;
    }

    // First part must match the beginning of the path.
    if !parts[0].is_empty() && !path.starts_with(parts[0]) {
        return false;
    }

    // Last part must match the end of the path.
    if let Some(last) = parts
        .last()
        .filter(|last| parts.len() > 1 && !last.is_empty())
    {
        // Handle trailing patterns like "/*.rs" (after **).
        if let Some(trailing_pattern) = last.strip_prefix('/') {
            // Use glob matching for the trailing pattern.
            if !path.ends_with(trailing_pattern) {
                // Try glob matching on the filename portion.
                if trailing_pattern.contains('*') || trailing_pattern.contains('?') {
                    // For "*.rs", find the filename portion.
                    if let Some(slash_pos) = path.rfind('/') {
                        let filename = &path[slash_pos + 1..];
                        if !match_simple_glob(filename, trailing_pattern) {
                            return false;
                        }
                    } else if !match_simple_glob(path, trailing_pattern) {
                        // No slash in path, match entire path against pattern.
                        return false;
                    }
                } else {
                    return false;
                }
            }
        } else if !path.ends_with(last) {
            // Check if path contains last part anywhere.
            let remaining = if parts[0].is_empty() {
                path
            } else {
                &path[parts[0].len()..]
            };
            if !remaining.contains(last) && !remaining.ends_with(last) {
                return false;
            }
        }
    }

    // Check middle parts if any.
    let mut search_pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 || i == parts.len() - 1 {
            continue;
        }

        if part.is_empty() {
            continue;
        }

        // Skip leading slash for middle parts.
        let part = if let Some(stripped) = part.strip_prefix('/') {
            stripped
        } else {
            *part
        };

        if search_pos >= path.len() {
            return false;
        }

        if let Some(pos) = path[search_pos..].find(part) {
            search_pos += pos + part.len();
        } else {
            return false;
        }
    }

    true
}

/// Match a path against a simple glob pattern (no **).
///
/// In this context, `*` matches any characters EXCEPT `/` (path separator).
/// This ensures `src/*.rs` matches `src/lib.rs` but not `src/sub/lib.rs`.
fn match_simple_glob(path: &str, pattern: &str) -> bool {
    let path_chars: Vec<char> = path.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let mut path_idx = 0;
    let mut pattern_idx = 0;

    while pattern_idx < pattern_chars.len() {
        let p = pattern_chars[pattern_idx];

        if p == '*' {
            // Skip consecutive *.
            while pattern_idx + 1 < pattern_chars.len() && pattern_chars[pattern_idx + 1] == '*' {
                pattern_idx += 1;
            }

            // Try matching zero or more characters (but not across /).
            let remaining_pattern = &pattern_chars[pattern_idx + 1..];

            // Find the maximum match length before hitting a path separator.
            let max_match_len = path_chars[path_idx..]
                .iter()
                .take_while(|&&c| c != '/')
                .count();

            for try_len in 0..=max_match_len {
                let remaining_path: String = path_chars[path_idx + try_len..].iter().collect();
                let remaining_pattern_str: String = remaining_pattern.iter().collect();

                if match_simple_glob(&remaining_path, &remaining_pattern_str) {
                    return true;
                }
            }
            return false;
        } else if p == '?' {
            // ? matches any single character except /.
            if path_idx >= path_chars.len() || path_chars[path_idx] == '/' {
                return false;
            }
            path_idx += 1;
            pattern_idx += 1;
        } else {
            if path_idx >= path_chars.len() || path_chars[path_idx] != p {
                return false;
            }
            path_idx += 1;
            pattern_idx += 1;
        }
    }

    path_idx == path_chars.len()
}
