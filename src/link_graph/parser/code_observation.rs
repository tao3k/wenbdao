//! Code observation parsing for Blueprint v2.7 (Internal AST Integration).
//!
//! This module provides parsing support for the `:OBSERVE:` property drawer attribute,
//! enabling documentation to observe code patterns via `xiuxian-ast` structural queries.
//!
//! ## Format
//!
//! The `:OBSERVE:` attribute uses the following syntax:
//! ```markdown
//! :OBSERVE: lang:<language> "<sgrep-pattern>"
//! :OBSERVE: lang:<language> scope:"<path-filter>" "<sgrep-pattern>"
//! ```
//!
//! ## Scope Filter
//!
//! The optional `scope:` attribute restricts pattern matching to specific file paths.
//! This prevents false positives when the same symbol exists in multiple packages.
//!
//! ```markdown
//! ## API Handler
//! :OBSERVE: lang:rust scope:"src/api/**" "fn $NAME($$$) -> Result<$$$>"
//! ```
//!
//! ## Example
//!
//! ```markdown
//! ## Storage Module
//! :OBSERVE: lang:rust "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>"
//! ```

use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasher;

use serde::{Deserialize, Serialize};

/// Parsed code observation entry from `:OBSERVE:` property drawer.
///
/// Represents a structural code pattern that this documentation section observes.
/// The pattern is validated by `xiuxian-ast` during the audit phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeObservation {
    /// Target language for the pattern (e.g., "rust", "python", "typescript").
    pub language: String,
    /// The sgrep/ast-grep pattern to match in source code.
    pub pattern: String,
    /// Optional scope filter to restrict pattern matching to specific paths.
    ///
    /// Supports glob patterns like:
    /// - `"src/api/**"` - Match files under src/api/
    /// - `"packages/core/**/*.rs"` - Match Rust files in packages/core/
    /// - `"**/handler.rs"` - Match any handler.rs file
    pub scope: Option<String>,
    /// The original raw value from the property drawer (for diagnostics).
    pub raw_value: String,
    /// Line number within the document where this observation was declared.
    pub line_number: Option<usize>,
    /// Whether the pattern has been validated by xiuxian-ast.
    pub is_validated: bool,
    /// Validation error message if pattern validation failed.
    pub validation_error: Option<String>,
}

impl CodeObservation {
    /// Create a new code observation.
    #[must_use]
    pub fn new(language: String, pattern: String, raw_value: String) -> Self {
        Self {
            language,
            pattern,
            scope: None,
            raw_value,
            line_number: None,
            is_validated: false,
            validation_error: None,
        }
    }

    /// Create a code observation with scope filter.
    #[must_use]
    pub fn with_scope(mut self, scope: String) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Create a code observation with line number.
    #[must_use]
    pub fn with_line(mut self, line_number: usize) -> Self {
        self.line_number = Some(line_number);
        self
    }

    /// Mark this observation as validated.
    #[must_use]
    pub fn validated(mut self) -> Self {
        self.is_validated = true;
        self
    }

    /// Mark this observation as having a validation error.
    #[must_use]
    pub fn with_error(mut self, error: String) -> Self {
        self.validation_error = Some(error);
        self
    }

    /// Check if a file path matches this observation's scope.
    ///
    /// Returns `true` if:
    /// - No scope is defined (matches all files)
    /// - The path matches the scope glob pattern
    #[must_use]
    pub fn matches_scope(&self, file_path: &str) -> bool {
        match &self.scope {
            None => true,
            Some(scope) => path_matches_scope(file_path, scope),
        }
    }

    /// Parse a `:OBSERVE:` value string into a `CodeObservation`.
    ///
    /// # Format
    ///
    /// - `lang:<language> "<pattern>"`
    /// - `lang:<language> scope:"<filter>" "<pattern>"`
    ///
    /// # Examples
    ///
    /// ```
    /// use xiuxian_wendao::link_graph::parser::code_observation::CodeObservation;
    ///
    /// // Without scope
    /// let obs = CodeObservation::parse(r#"lang:rust "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>""#);
    /// assert!(obs.is_some());
    /// let obs = obs.unwrap();
    /// assert_eq!(obs.language, "rust");
    /// assert_eq!(obs.pattern, "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>");
    /// assert!(obs.scope.is_none());
    ///
    /// // With scope
    /// let obs = CodeObservation::parse(r#"lang:rust scope:"src/api/**" "fn $NAME($$$) -> Result<$$$>""#);
    /// assert!(obs.is_some());
    /// let obs = obs.unwrap();
    /// assert_eq!(obs.scope, Some("src/api/**".to_string()));
    /// ```
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn parse(value: &str) -> Option<Self> {
        let trimmed = value.trim();

        // Must start with "lang:"
        if !trimmed.starts_with("lang:") {
            return None;
        }

        // Find the space after "lang:<language>"
        let after_lang = &trimmed[5..]; // Skip "lang:"
        let space_pos = after_lang.find(' ')?;

        let language = after_lang[..space_pos].trim().to_string();
        if language.is_empty() {
            return None;
        }

        let mut rest = after_lang[space_pos..].trim();
        let mut scope = None;

        // Check for optional scope:"..." before the pattern
        if rest.starts_with("scope:\"") {
            let scope_str = &rest[7..]; // Skip 'scope:"'
            if let Some(end_quote) = find_closing_quote(scope_str) {
                scope = Some(scope_str[..end_quote].replace("\\\"", "\""));
                rest = scope_str[end_quote + 1..].trim();
            }
        }

        // Extract the quoted pattern
        // Pattern must be in quotes
        if !rest.starts_with('"') {
            return None;
        }

        // Find the closing quote (handle escaped quotes)
        let pattern_str = &rest[1..]; // Skip opening quote
        let end_pos = find_closing_quote(pattern_str)?;
        let pattern = pattern_str[..end_pos].replace("\\\"", "\"");

        let mut obs = Self::new(language, pattern, value.to_string());
        if let Some(s) = scope {
            obs = obs.with_scope(s);
        }

        Some(obs)
    }

    /// Get the language for xiuxian-ast queries.
    ///
    /// Returns `None` if the language string is not a supported AST language.
    #[must_use]
    pub fn ast_language(&self) -> Option<xiuxian_ast::Lang> {
        xiuxian_ast::Lang::try_from(self.language.as_str()).ok()
    }

    /// Validate the pattern using xiuxian-ast.
    ///
    /// # Errors
    ///
    /// Returns an error when the observation language is not supported by `xiuxian-ast` or when
    /// the configured pattern is not accepted by the target parser.
    pub fn validate_pattern(&self) -> Result<(), String> {
        let lang = self
            .ast_language()
            .ok_or_else(|| format!("Unsupported language: {}", self.language))?;

        xiuxian_ast::pattern(&self.pattern, lang).map_err(|e| format!("Invalid pattern: {e}"))?;

        Ok(())
    }
}

/// Find the closing quote in a string, handling escaped quotes.
fn find_closing_quote(s: &str) -> Option<usize> {
    let mut chars = s.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '\\' {
            // Skip the next character (escaped)
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
    // Normalize paths to use forward slashes
    let normalized_path = file_path.replace('\\', "/");
    let normalized_scope = scope.replace('\\', "/");

    // Handle ** glob pattern
    if normalized_scope.contains("**") {
        match_glob_with_double_star(&normalized_path, &normalized_scope)
    } else {
        // Simple glob matching for patterns without **
        match_simple_glob(&normalized_path, &normalized_scope)
    }
}

/// Match a path against a glob pattern containing **.
fn match_glob_with_double_star(path: &str, pattern: &str) -> bool {
    // Split pattern by **
    let parts: Vec<&str> = pattern.split("**").collect();

    if parts.is_empty() {
        return true;
    }

    // First part must match the beginning of the path
    if !parts[0].is_empty() && !path.starts_with(parts[0]) {
        return false;
    }

    // Last part must match the end of the path
    if let Some(last) = parts
        .last()
        .filter(|last| parts.len() > 1 && !last.is_empty())
    {
        // Handle trailing patterns like "/*.rs" (after **)
        if let Some(trailing_pattern) = last.strip_prefix('/') {
            // Use glob matching for the trailing pattern
            if !path.ends_with(trailing_pattern) {
                // Try glob matching on the filename portion
                if trailing_pattern.contains('*') || trailing_pattern.contains('?') {
                    // For "*.rs", find the filename portion
                    if let Some(slash_pos) = path.rfind('/') {
                        let filename = &path[slash_pos + 1..];
                        if !match_simple_glob(filename, trailing_pattern) {
                            return false;
                        }
                    } else {
                        // No slash in path, match entire path against pattern
                        if !match_simple_glob(path, trailing_pattern) {
                            return false;
                        }
                    }
                } else {
                    return false;
                }
            }
        } else if !path.ends_with(last) {
            // Check if path contains last part anywhere
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

    // Check middle parts if any
    let mut search_pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 || i == parts.len() - 1 {
            continue;
        }

        if part.is_empty() {
            continue;
        }

        // Skip leading slash for middle parts
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
            // Skip consecutive *
            while pattern_idx + 1 < pattern_chars.len() && pattern_chars[pattern_idx + 1] == '*' {
                pattern_idx += 1;
            }

            // Try matching zero or more characters (but not across /)
            let remaining_pattern = &pattern_chars[pattern_idx + 1..];

            // Find the maximum match length before hitting a path separator
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
            // ? matches any single character except /
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

impl fmt::Display for CodeObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":OBSERVE: lang:{}", self.language)?;
        if let Some(ref scope) = self.scope {
            write!(f, " scope:\"{}\"", scope.replace('"', "\\\""))?;
        }
        write!(f, " \"{}\"", self.pattern.replace('"', "\\\""))
    }
}

/// Extract all `:OBSERVE:` entries from property drawer attributes.
///
/// Supports multiple observation patterns per section by using:
/// - Single `:OBSERVE:` with the full format
/// - Multiple `:OBSERVE:` entries (numbered or repeated)
///
/// # Example
///
/// ```markdown
/// :OBSERVE: lang:rust "fn $NAME($$$) -> Result<$$$>"
/// ```
#[must_use]
pub fn extract_observations<S: BuildHasher>(
    attributes: &HashMap<String, String, S>,
) -> Vec<CodeObservation> {
    let mut observations = Vec::new();

    // Check for single :OBSERVE: entry
    if let Some(value) = attributes.get("OBSERVE")
        && let Some(obs) = CodeObservation::parse(value)
    {
        observations.push(obs);
    }

    // Check for numbered entries: :OBSERVE_1:, :OBSERVE_2:, etc.
    for (key, value) in attributes {
        if key.starts_with("OBSERVE_")
            && let Some(obs) = CodeObservation::parse(value)
        {
            observations.push(obs);
        }
    }

    observations
}

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/parser/code_observation.rs"]
mod tests;
