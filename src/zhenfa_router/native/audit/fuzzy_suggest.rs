//! Fuzzy Pattern Suggestion for Code Observations (Blueprint v2.9).
//!
//! When an `:OBSERVE:` pattern fails validation (e.g., a symbol was renamed),
//! this module searches for similar patterns and suggests updated patterns.
//!
//! ## Architecture
//!
//! 1. **Detect**: Pattern validation fails → Error diagnostic
//! 2. **Search**: Fuzzy structural search → Candidate matches
//! 3. **Suggest**: Rank & format suggestions → `replacement_drawer` content
//!
//! ## Example
//!
//! ```ignore
//! use crate::zhenfa_router::native::audit::fuzzy_suggest::{
//!     suggest_pattern_fix, SourceFile,
//! };
//!
//! let source = SourceFile {
//!     path: "src/lib.rs".to_string(),
//!     content: "fn process_records(data: Vec<u8>) -> Result<()>".to_string(),
//! };
//!
//! let suggestion = suggest_pattern_fix(
//!     "fn process_data($$$)",
//!     xiuxian_ast::Lang::Rust,
//!     &[source],
//! );
//!
//! assert!(suggestion.is_some());
//! assert!(suggestion.unwrap().suggested_pattern.contains("process_records"));
//! ```
//!
//! ## Performance Caching (Blueprint v2.9)
//!
//! To avoid re-scanning the same source files repeatedly, this module
//! uses a thread-local cache for candidate matches. The cache is invalidated
//! when source files change.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Minimum confidence threshold for suggesting a pattern fix.
/// Set to 0.65 to allow reasonable renamed symbol detection while filtering out poor matches.
const CONFIDENCE_THRESHOLD: f32 = 0.65;

/// Thread-local cache for candidate matches.
/// Key: `file_path`
/// Value: (`content_hash`, Vec<CandidateMatch>)
type CacheValue = (u64, Vec<CandidateMatch>);
thread_local! {
    static CANDIDATE_CACHE: RefCell<HashMap<String, CacheValue>> = RefCell::new(HashMap::new());
}

/// Simple hash function for content.
fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Clear the candidate cache.
/// Call this when source files have been modified.
pub fn clear_candidate_cache() {
    CANDIDATE_CACHE.with(|cache| cache.borrow_mut().clear());
}

/// Get cache statistics for debugging.
#[must_use]
pub fn cache_stats() -> (usize, usize) {
    CANDIDATE_CACHE.with(|cache| {
        let c = cache.borrow();
        (c.len(), c.values().map(|v| v.1.len()).sum())
    })
}

/// A source file to scan for pattern matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// File path (for diagnostics).
    pub path: String,
    /// Source code content.
    pub content: String,
}

/// Result of a fuzzy pattern suggestion search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzySuggestion {
    /// The suggested updated pattern.
    pub suggested_pattern: String,
    /// Similarity score (0.0 - 1.0).
    pub confidence: f32,
    /// Source location where match was found.
    pub source_location: Option<String>,
    /// Ready-to-use replacement drawer content.
    pub replacement_drawer: String,
}

/// Structural elements extracted from a sgrep pattern.
///
/// Used for fuzzy matching when exact patterns fail.
#[derive(Debug, Clone, Default)]
struct PatternSkeleton {
    /// Language keywords (fn, def, class, struct, etc.).
    keywords: Vec<String>,
    /// Structural punctuation ((), {}, <>, [], ->, :, ;).
    structure: Vec<String>,
    /// Metavariables ($NAME, $$$ARGS, etc.).
    metavariables: Vec<String>,
    /// Identifier-like terms (function names, type names).
    identifiers: Vec<String>,
}

impl PatternSkeleton {
    /// Extract structural skeleton from a sgrep pattern.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let skeleton = PatternSkeleton::extract("fn $NAME($$$ARGS) -> Result<$$$>");
    /// // skeleton.keywords = ["fn", "Result"]
    /// // skeleton.structure = ["(", ")", "->", "<", ">"]
    /// // skeleton.metavariables = ["$NAME", "$$$ARGS", "$$$"]
    /// ```
    fn extract(pattern: &str) -> Self {
        let mut skeleton = Self::default();

        // Tokenize the pattern
        let tokens = tokenize_pattern(pattern);

        for token in tokens {
            match token.as_str() {
                // Keywords (language constructs)
                "fn" | "def" | "class" | "struct" | "enum" | "impl" | "trait" | "interface"
                | "type" | "const" | "let" | "var" | "pub" | "private" | "async" | "await"
                | "return" | "if" | "else" | "for" | "while" | "match" | "case" | "func"
                | "function" | "public" | "protected" | "Result" | "Option" | "Vec" | "String"
                | "str" | "int" | "bool" | "void" | "null" | "None" | "Some" | "Ok" | "Err" => {
                    if !skeleton.keywords.contains(&token) {
                        skeleton.keywords.push(token);
                    }
                }
                // Structural elements
                "(" | ")" | "{" | "}" | "[" | "]" | "<" | ">" | "->" | "=>" | ":" | ";" | ","
                | "." => {
                    skeleton.structure.push(token);
                }
                // Metavariables
                t if t.starts_with('$') => {
                    skeleton.metavariables.push(token);
                }
                // Identifiers (everything else that looks like a name)
                t if is_identifier_like(t) => {
                    if !skeleton.identifiers.contains(&token) {
                        skeleton.identifiers.push(token);
                    }
                }
                _ => {}
            }
        }

        skeleton
    }
}

/// Tokenize a sgrep pattern into individual elements.
fn tokenize_pattern(pattern: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Two-character operators
            '-' if chars.peek() == Some(&'>') => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                chars.next();
                tokens.push("->".to_string());
            }
            '=' if chars.peek() == Some(&'>') => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                chars.next();
                tokens.push("=>".to_string());
            }
            // Single-character structural elements
            '(' | ')' | '{' | '}' | '[' | ']' | '<' | '>' | ':' | ';' | ',' | '.' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
                tokens.push(ch.to_string());
            }
            // Whitespace separates tokens
            ' ' | '\t' | '\n' | '\r' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            // Accumulate identifier/keyword characters
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

/// Check if a string looks like an identifier.
fn is_identifier_like(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
        && s.chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
}

/// Calculate similarity score between two pattern skeletons.
///
/// Returns a score in [0.0, 1.0] where:
/// - 1.0 = perfect match
/// - 0.0 = no similarity
///
/// Weights (adjusted for fuzzy matching):
/// - Keywords: 35% (language keywords like fn, def, class)
/// - Structure: 25% (punctuation and operators)
/// - Identifiers: 30% (function/type names)
/// - Metavariables: 10% (pattern variables)
fn calculate_skeleton_similarity(original: &PatternSkeleton, candidate: &PatternSkeleton) -> f32 {
    // For keywords, we only care if they share the same base keyword (fn, def, class, etc.)
    // This is more lenient than full Jaccard similarity
    let keyword_score = if original.keywords.is_empty() || candidate.keywords.is_empty() {
        0.0
    } else {
        // Check if they share at least one important keyword (fn, def, class, struct, etc.)
        let has_common_keyword = original
            .keywords
            .iter()
            .any(|k| candidate.keywords.contains(k));
        if has_common_keyword {
            // Bonus for matching the primary keyword
            0.8 + (jaccard_similarity(&original.keywords, &candidate.keywords) * 0.2)
        } else {
            jaccard_similarity(&original.keywords, &candidate.keywords)
        }
    };

    // For structure, use a more lenient comparison
    // We care about basic structure (parentheses, braces) more than exact matches
    let structure_score = if original.structure.is_empty() && candidate.structure.is_empty() {
        1.0
    } else if original.structure.is_empty() || candidate.structure.is_empty() {
        0.3 // Give partial credit if one has structure but both are functions
    } else {
        // Check for basic structural elements (parentheses for functions)
        let orig_has_parens = original.structure.contains(&"(".to_string())
            && original.structure.contains(&")".to_string());
        let cand_has_parens = candidate.structure.contains(&"(".to_string())
            && candidate.structure.contains(&")".to_string());

        if orig_has_parens && cand_has_parens {
            // Both have function-like structure, give good score
            0.6 + (jaccard_similarity(&original.structure, &candidate.structure) * 0.4)
        } else {
            jaccard_similarity(&original.structure, &candidate.structure)
        }
    };

    let identifier_score = jaccard_similarity(&original.identifiers, &candidate.identifiers);
    let metavar_score = jaccard_similarity(&original.metavariables, &candidate.metavariables);

    // Weighted average with adjusted weights
    (keyword_score * 0.35)
        + (structure_score * 0.25)
        + (identifier_score * 0.30)
        + (metavar_score * 0.10)
}

/// Calculate Jaccard similarity between two sets.
fn jaccard_similarity<T: std::hash::Hash + Eq + std::borrow::Borrow<T>>(a: &[T], b: &[T]) -> f32 {
    use std::collections::HashSet;

    let set_a: HashSet<&T> = a.iter().collect();
    let set_b: HashSet<&T> = b.iter().collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        0.0
    } else {
        bounded_ratio(intersection, union)
    }
}

/// Convert small heuristic counts into `f32` without unchecked casts.
fn bounded_ratio(numerator: usize, denominator: usize) -> f32 {
    let numerator = bounded_usize_to_f32(numerator);
    let denominator = bounded_usize_to_f32(denominator);
    numerator / denominator
}

/// Saturate large counts because fuzzy matching only needs stable heuristic ratios.
fn bounded_usize_to_f32(value: usize) -> f32 {
    u16::try_from(value).map_or(f32::from(u16::MAX), f32::from)
}

/// Calculate Levenshtein edit distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }

    if let Some(first_row) = matrix.first_mut() {
        for (j, cell) in first_row.iter_mut().enumerate() {
            *cell = j;
        }
    }

    for (i, a_char) in a_chars.iter().enumerate() {
        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = usize::from(a_char != b_char);
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Calculate string similarity based on edit distance.
fn string_similarity(a: &str, b: &str) -> f32 {
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }

    let distance = levenshtein_distance(a, b);
    1.0 - bounded_ratio(distance, max_len)
}

/// Extract the primary capture name from a pattern.
fn extract_capture_name(pattern: &str) -> Option<String> {
    let mut chars = pattern.chars().peekable();
    let mut in_capture = false;
    let mut capture = String::new();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            in_capture = true;
            capture.clear();

            // Check for $$$ (multi-match)
            if chars.peek() == Some(&'$') {
                chars.next();
                if chars.peek() == Some(&'$') {
                    chars.next();
                    // Skip $$$ captures
                    in_capture = false;
                }
            }
            continue;
        }

        if in_capture {
            if ch.is_alphanumeric() || ch == '_' {
                capture.push(ch);
            } else {
                if !capture.is_empty() {
                    return Some(capture.clone());
                }
                in_capture = false;
            }
        }
    }

    if capture.is_empty() {
        None
    } else {
        Some(capture)
    }
}

/// A candidate match found in source files.
#[derive(Debug, Clone)]
struct CandidateMatch {
    /// The matched text from source.
    matched_text: String,
    /// Source file path.
    file_path: String,
    /// Line number (1-indexed).
    line_number: usize,
    /// Identifier that was matched (if any).
    identifier: Option<String>,
    /// Skeleton of the matched code.
    skeleton: PatternSkeleton,
}

/// Search for similar patterns when validation fails.
///
/// # Arguments
///
/// * `original_pattern` - The pattern that failed validation
/// * `lang` - Target language for the pattern
/// * `source_files` - Source files to search for candidates
/// * `threshold` - Optional confidence threshold (uses `CONFIDENCE_THRESHOLD` if None)
///
/// # Returns
///
/// `Some(FuzzySuggestion)` if a good match is found, `None` otherwise.
#[must_use]
pub fn suggest_pattern_fix(
    original_pattern: &str,
    lang: xiuxian_ast::Lang,
    source_files: &[SourceFile],
) -> Option<FuzzySuggestion> {
    suggest_pattern_fix_with_threshold(original_pattern, lang, source_files, None)
}

/// Search for similar patterns with a custom confidence threshold.
///
/// # Arguments
///
/// * `original_pattern` - The pattern that failed validation
/// * `lang` - Target language for the pattern
/// * `source_files` - Source files to search for candidates
/// * `threshold` - Optional confidence threshold (uses `CONFIDENCE_THRESHOLD` if None)
///
/// # Returns
///
/// `Some(FuzzySuggestion)` if a good match is found, `None` otherwise.
#[must_use]
pub fn suggest_pattern_fix_with_threshold(
    original_pattern: &str,
    lang: xiuxian_ast::Lang,
    source_files: &[SourceFile],
    threshold: Option<f32>,
) -> Option<FuzzySuggestion> {
    let effective_threshold = threshold.unwrap_or(CONFIDENCE_THRESHOLD);

    if source_files.is_empty() {
        return None;
    }

    let original_skeleton = PatternSkeleton::extract(original_pattern);
    let mut candidates: Vec<CandidateMatch> = Vec::new();

    // Scan source files for potential matches
    for source_file in source_files {
        let matches = scan_for_candidates(&source_file.content, lang, &source_file.path);
        candidates.extend(matches);
    }

    if candidates.is_empty() {
        return None;
    }

    // Score and rank candidates
    let mut scored_candidates: Vec<(f32, &CandidateMatch)> = candidates
        .iter()
        .filter_map(|candidate| {
            let skeleton_score =
                calculate_skeleton_similarity(&original_skeleton, &candidate.skeleton);

            // Bonus for identifier similarity (renamed symbol detection)
            let identifier_bonus = if let (Some(orig_id), Some(cand_id)) =
                (original_skeleton.identifiers.first(), &candidate.identifier)
            {
                let id_sim = string_similarity(orig_id, cand_id);
                // Add bonus if there's meaningful similarity (lowered threshold to 0.2)
                if id_sim > 0.2 {
                    // Increased bonus weight from 0.2 to 0.4 for better renamed symbol detection
                    id_sim * 0.4
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let total_score = skeleton_score + identifier_bonus;

            // Filter by threshold
            if total_score >= effective_threshold {
                Some((total_score, candidate))
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending
    scored_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Return best match
    if let Some((confidence, best_match)) = scored_candidates.first() {
        let suggested_pattern = generate_suggested_pattern(original_pattern, best_match, lang);
        let replacement_drawer = format!(
            ":OBSERVE: lang:{} \"{}\"",
            lang.as_str(),
            suggested_pattern.replace('"', "\\\"")
        );
        let source_location = Some(format!(
            "{}:{}",
            best_match.file_path, best_match.line_number
        ));

        Some(FuzzySuggestion {
            suggested_pattern,
            confidence: *confidence,
            source_location,
            replacement_drawer,
        })
    } else {
        None
    }
}

/// Scan source content for candidate code patterns.
/// Uses caching to avoid re-scanning unchanged files.
fn scan_for_candidates(
    content: &str,
    lang: xiuxian_ast::Lang,
    file_path: &str,
) -> Vec<CandidateMatch> {
    let content_hash = hash_content(content);

    // Check cache first
    let cached = CANDIDATE_CACHE.with(|cache| {
        let cache = cache.borrow();
        if let Some((cached_hash, candidates)) = cache.get(file_path)
            && *cached_hash == content_hash
        {
            // Cache hit! Return cached candidates
            return Some(candidates.clone());
        }
        None
    });

    if let Some(candidates) = cached {
        return candidates;
    }

    // Cache miss - scan the content
    let patterns = xiuxian_ast::get_skeleton_patterns(lang);
    let mut candidates = Vec::new();

    for pattern in patterns {
        let capture_name = extract_capture_name(pattern);
        let capture_filters = capture_name.as_ref().map(|name| vec![name.as_str()]);
        let results = xiuxian_ast::extract_items(content, pattern, lang, capture_filters);

        for result in results {
            // Extract identifier from captures
            let identifier = capture_name
                .as_ref()
                .and_then(|name| result.captures.get(name))
                .cloned()
                .or_else(|| result.captures.values().next().cloned());

            // Extract just the signature line for skeleton comparison
            // This ensures we compare similar structural complexity
            let signature = extract_signature_line(&result.text, lang);

            // Build skeleton from signature (not full matched text)
            let skeleton = PatternSkeleton::extract(&signature);

            candidates.push(CandidateMatch {
                matched_text: result.text.clone(),
                file_path: file_path.to_string(),
                line_number: result.line_start,
                identifier,
                skeleton,
            });
        }
    }

    // Store in cache for future lookups
    CANDIDATE_CACHE.with(|cache| {
        cache
            .borrow_mut()
            .insert(file_path.to_string(), (content_hash, candidates.clone()));
    });

    candidates
}

/// Generate a suggested pattern from the original pattern and best match.
fn generate_suggested_pattern(
    _original_pattern: &str,
    best_match: &CandidateMatch,
    lang: xiuxian_ast::Lang,
) -> String {
    // Extract the signature line from the match
    let signature = extract_signature_line(&best_match.matched_text, lang);

    // Convert to a pattern by replacing identifiers with metavariables
    patternize_signature(&signature, best_match.identifier.as_ref(), lang)
}

/// Extract just the signature line from matched code.
fn extract_signature_line(code: &str, lang: xiuxian_ast::Lang) -> String {
    let first_line = code.lines().next().unwrap_or(code);

    match lang {
        xiuxian_ast::Lang::Python
        | xiuxian_ast::Lang::Ruby
        | xiuxian_ast::Lang::Lua
        | xiuxian_ast::Lang::Bash => {
            // For Python-like, include everything up to the colon
            if let Some(colon_pos) = first_line.find(':') {
                format!("{}: $$$BODY", first_line[..=colon_pos].trim())
            } else {
                first_line.trim().to_string()
            }
        }
        xiuxian_ast::Lang::Rust
        | xiuxian_ast::Lang::C
        | xiuxian_ast::Lang::Cpp
        | xiuxian_ast::Lang::CSharp
        | xiuxian_ast::Lang::Java
        | xiuxian_ast::Lang::Go
        | xiuxian_ast::Lang::Swift
        | xiuxian_ast::Lang::Kotlin
        | xiuxian_ast::Lang::Php
        | xiuxian_ast::Lang::JavaScript
        | xiuxian_ast::Lang::TypeScript => {
            // For C-like, truncate at the first '{' and add ellipsis
            if let Some(brace_pos) = first_line.find('{') {
                format!("{} {{ $$$BODY }}", first_line[..brace_pos].trim())
            } else {
                first_line.trim().to_string()
            }
        }
        _ => first_line.trim().to_string(),
    }
}

/// Convert a signature to a pattern by adding metavariables.
fn patternize_signature(
    signature: &str,
    identifier: Option<&String>,
    _lang: xiuxian_ast::Lang,
) -> String {
    let mut pattern = signature.to_string();

    // If we have an identifier, keep it concrete but add wildcards for arguments
    if let Some(id) = identifier {
        // For function patterns, replace argument lists with wildcards
        if signature.contains(id) {
            // Pattern already has identifier - just ensure wildcards
            if signature.contains('(') && signature.contains(')') {
                // Keep the structure, add $$$ for body if not present
                if !pattern.contains("$$$") {
                    pattern = format!("{} $$$", pattern.trim());
                }
            }
        }
    }

    // Clean up multiple spaces
    while pattern.contains("  ") {
        pattern = pattern.replace("  ", " ");
    }

    pattern.trim().to_string()
}

/// Format a fuzzy suggestion for display in diagnostics.
#[must_use]
pub fn format_suggestion(suggestion: &FuzzySuggestion) -> String {
    format!(
        "Consider updating pattern to: {}\nConfidence: {:.0}%\n{}",
        suggestion.suggested_pattern,
        suggestion.confidence * 100.0,
        suggestion
            .source_location
            .as_ref()
            .map(|l| format!("Found similar code at: {l}"))
            .unwrap_or_default()
    )
}

/// Resolve source files from directory paths.
///
/// This is a simple implementation that scans for common source file extensions.
/// For more sophisticated discovery, use the `dependency_indexer`.
#[must_use]
pub fn resolve_source_files(paths: &[&Path], lang: xiuxian_ast::Lang) -> Vec<SourceFile> {
    let mut files = Vec::new();
    let extensions = lang.extensions();

    for path in paths {
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && extensions.contains(&ext)
                && let Ok(content) = std::fs::read_to_string(path)
            {
                files.push(SourceFile {
                    path: path.display().to_string(),
                    content,
                });
            }
        } else if path.is_dir() {
            // Simple directory scan - could be enhanced with walkdir
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file()
                        && let Some(ext) = entry_path.extension().and_then(|e| e.to_str())
                        && extensions.contains(&ext)
                        && let Ok(content) = std::fs::read_to_string(&entry_path)
                    {
                        files.push(SourceFile {
                            path: entry_path.display().to_string(),
                            content,
                        });
                    }
                }
            }
        }
    }

    files
}

#[cfg(test)]
#[path = "../../../../tests/unit/zhenfa_router/native/audit/fuzzy_suggest.rs"]
mod tests;
