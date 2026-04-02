//! Parsing helpers for semantic checking.

use super::types::HashReference;

/// Extract ID references from text content.
///
/// Looks for wiki-style links like `[[#id]]` or `[[id]]`.
#[allow(clippy::expect_used)]
pub(super) fn extract_id_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next(); // consume second '['
            let mut link_content = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next(); // consume first ']'
                    if chars.peek() == Some(&']') {
                        chars.next(); // consume second ']'
                        break;
                    }
                    link_content.push(']');
                } else {
                    // SAFETY: We just peeked and know there's a character
                    link_content.push(chars.next().expect("char exists after peek"));
                }
            }
            // Extract ID from link content (may start with # or be a path)
            let link = link_content.trim();
            if link.starts_with('#') {
                refs.push(link.to_string());
            }
        }
    }
    refs
}

/// Extract hash-annotated references from text content.
///
/// Format: `[[#id@hash]]` where @hash is the expected content hash.
///
/// # Example
///
/// - `[[#arch-v1@abc123]]` -> `HashReference` { `target_id`: "arch-v1", `expect_hash`: Some("abc123") }
/// - `[[#intro]]` -> `HashReference` { `target_id`: "intro", `expect_hash`: None }
#[allow(clippy::expect_used)]
pub(super) fn extract_hash_references(text: &str) -> Vec<HashReference> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next(); // consume second '['
            let mut link_content = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next(); // consume first ']'
                    if chars.peek() == Some(&']') {
                        chars.next(); // consume second ']'
                        break;
                    }
                    link_content.push(']');
                } else {
                    // SAFETY: We just peeked and know there's a character
                    link_content.push(chars.next().expect("char exists after peek"));
                }
            }
            // Parse link content for #id[@hash] format
            let link = link_content.trim();
            if let Some(id_part) = link.strip_prefix('#') {
                // Check for @hash suffix
                if let Some(at_pos) = id_part.find('@') {
                    let target_id = id_part[..at_pos].to_string();
                    let expect_hash = id_part[at_pos + 1..].to_string();
                    refs.push(HashReference {
                        target_id,
                        expect_hash: Some(expect_hash),
                    });
                } else {
                    refs.push(HashReference {
                        target_id: id_part.to_string(),
                        expect_hash: None,
                    });
                }
            }
        }
    }
    refs
}

/// Validate a contract expression against content.
///
/// Supported contract formats:
/// - `must_contain("term1", "term2", ...)` - Content must contain all specified terms
/// - `must_not_contain("term")` - Content must not contain the specified term
/// - `min_length(N)` - Content must have at least N characters
pub(super) fn validate_contract(contract: &str, content: &str) -> Option<String> {
    let contract = contract.trim();

    // must_contain("term1", "term2", ...)
    if let Some(args) = extract_function_args(contract, "must_contain") {
        let terms: Vec<&str> = args
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim())
            .filter(|s| !s.is_empty())
            .collect();

        for term in terms {
            if !content.contains(term) {
                return Some(format!("missing required term '{term}'"));
            }
        }
        return None;
    }

    // must_not_contain("term")
    if let Some(args) = extract_function_args(contract, "must_not_contain") {
        let term = args.trim().trim_matches('"').trim();
        if content.contains(term) {
            return Some(format!("contains forbidden term '{term}'"));
        }
        return None;
    }

    // min_length(N)
    if let Some(args) = extract_function_args(contract, "min_length") {
        if let Ok(min_len) = args.trim().parse::<usize>()
            && content.len() < min_len
        {
            return Some(format!(
                "content length {} is less than required {}",
                content.len(),
                min_len
            ));
        }
        return None;
    }

    // Unknown contract type - skip validation
    None
}

/// Extract arguments from a function-like contract expression.
pub(super) fn extract_function_args<'a>(contract: &'a str, function_name: &str) -> Option<&'a str> {
    let prefix = format!("{function_name}(");
    if contract.starts_with(&prefix) && contract.ends_with(')') {
        Some(&contract[prefix.len()..contract.len() - 1])
    } else {
        None
    }
}

/// Generate a suggested ID from a title.
pub(super) fn generate_suggested_id(title: &str) -> String {
    title
        .to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        .trim_matches('-')
        .to_string()
}
