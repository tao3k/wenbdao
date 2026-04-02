use std::path::Path;
use std::sync::OnceLock;

use log::info;

use crate::LinkGraphIndex;
use crate::link_graph::parser::code_observation::path_matches_scope;
use crate::link_graph::{PageIndexNode, SymbolRef};

use super::types::{AffectedDoc, DriftConfidence, SemanticDriftSignal};

fn capture_symbol(pattern: &str, regex: Option<&regex::Regex>) -> Option<String> {
    regex.and_then(|compiled| {
        compiled
            .captures(pattern)
            .and_then(|caps| caps.get(1))
            .map(|capture| capture.as_str().to_string())
    })
}

fn push_captured_symbol(symbols: &mut Vec<String>, pattern: &str, regex: Option<&regex::Regex>) {
    if let Some(symbol) = capture_symbol(pattern, regex) {
        symbols.push(symbol);
    }
}

fn push_unique_captured_symbol(
    symbols: &mut Vec<String>,
    pattern: &str,
    regex: Option<&regex::Regex>,
) {
    if let Some(symbol) = capture_symbol(pattern, regex)
        && !symbols.contains(&symbol)
    {
        symbols.push(symbol);
    }
}

/// Extract core symbols from an observation pattern.
///
/// This is a heuristic extraction for the Symbol-to-Node Inverted Index.
/// Patterns like `fn process_data($$$)` yield `["process_data"]`.
/// Patterns like `struct User { $$$ }` yield `["User"]`.
#[must_use]
pub fn extract_pattern_symbols(pattern: &str) -> Vec<String> {
    // Pre-compiled regex patterns for zero-cost repeated extraction
    static RE_FN: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_STRUCT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_CLASS: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_ENUM: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_METHOD: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_TRAIT: OnceLock<Option<regex::Regex>> = OnceLock::new();
    static RE_IMPL: OnceLock<Option<regex::Regex>> = OnceLock::new();

    let re_fn = RE_FN
        .get_or_init(|| regex::Regex::new(r"\bfn\s+([a-z_][a-z0-9_]*)").ok())
        .as_ref();
    let re_struct = RE_STRUCT
        .get_or_init(|| regex::Regex::new(r"\bstruct\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_class = RE_CLASS
        .get_or_init(|| regex::Regex::new(r"\bclass\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_enum = RE_ENUM
        .get_or_init(|| regex::Regex::new(r"\benum\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_method = RE_METHOD
        .get_or_init(|| regex::Regex::new(r"\b(?:async\s+)?fn\s+([a-z_][a-z0-9_]*)\s*\(").ok())
        .as_ref();
    let re_trait = RE_TRAIT
        .get_or_init(|| regex::Regex::new(r"\btrait\s+([A-Z][a-zA-Z0-9_]*)").ok())
        .as_ref();
    let re_impl = RE_IMPL
        .get_or_init(|| {
            regex::Regex::new(r"\bimpl\s+(?:[A-Z][a-zA-Z0-9_]*\s+for\s+)?([A-Z][a-zA-Z0-9_]*)").ok()
        })
        .as_ref();

    let mut symbols = Vec::new();

    // Extract function names: fn NAME
    push_captured_symbol(&mut symbols, pattern, re_fn);

    // Extract struct names: struct NAME
    push_captured_symbol(&mut symbols, pattern, re_struct);

    // Extract class names: class NAME
    push_captured_symbol(&mut symbols, pattern, re_class);

    // Extract enum names: enum NAME
    push_captured_symbol(&mut symbols, pattern, re_enum);

    // Extract method names: fn NAME( or async fn NAME(
    push_unique_captured_symbol(&mut symbols, pattern, re_method);

    // Extract trait names: trait NAME
    push_captured_symbol(&mut symbols, pattern, re_trait);

    // Extract impl targets: impl NAME or impl Trait for NAME
    push_captured_symbol(&mut symbols, pattern, re_impl);

    symbols
}

/// Compute Blake3 hash of a file's content.
#[must_use]
pub fn compute_file_hash(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(blake3::hash(content.as_bytes()).to_hex().to_string())
}

/// Check if a file path matches the observation's scope filter.
///
/// Returns `true` if:
/// - The scope is `None` (no filtering)
/// - The scope matches the file path using glob pattern matching
///
/// Returns `false` if:
/// - The scope is `Some` but doesn't match the file path
#[must_use]
pub(crate) fn matches_scope_filter(file_path: &str, scope: Option<&str>) -> bool {
    match scope {
        None => true, // No scope means match all files
        Some(scope_pattern) => path_matches_scope(file_path, scope_pattern),
    }
}

fn add_symbol_refs_to_signal(
    signal: &mut SemanticDriftSignal,
    symbol_refs: &[SymbolRef],
    file_path: &str,
) {
    for sym_ref in symbol_refs {
        if !matches_scope_filter(file_path, sym_ref.scope.as_deref()) {
            continue;
        }

        if signal
            .affected_docs
            .iter()
            .any(|doc| doc.node_id == sym_ref.node_id)
        {
            continue;
        }

        let affected = AffectedDoc::new(
            &sym_ref.doc_id,
            &sym_ref.pattern,
            &sym_ref.language,
            &sym_ref.node_id,
        )
        .with_line(sym_ref.line_number.unwrap_or(0));

        signal.add_affected_doc(affected);
    }
}

fn has_explicit_reference(affected_docs: &[AffectedDoc], file_stem: &str) -> bool {
    let function_pattern = format!("fn {file_stem}");
    let struct_pattern = format!("struct {file_stem}");
    let class_pattern = format!("class {file_stem}");

    affected_docs.iter().any(|doc| {
        doc.matching_pattern.contains(&function_pattern)
            || doc.matching_pattern.contains(&struct_pattern)
            || doc.matching_pattern.contains(&class_pattern)
    })
}

/// Phase 6: Core logic for propagating source changes to documentation.
///
/// Uses the Symbol-to-Node Inverted Index for O(1) lookup when available,
/// falling back to heuristic traversal when the index is empty or misses.
///
/// # Phase 7.6: Scope Filtering
///
/// Observations with a `scope` filter only match files within the specified
/// path pattern. This prevents false positives when the same symbol exists
/// in multiple packages.
///
/// # Returns
///
/// A vector of `SemanticDriftSignal` events for each affected observation.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn propagate_source_change(index: &LinkGraphIndex, path: &Path) -> Vec<SemanticDriftSignal> {
    info!("Propagating semantic change from code: {}", path.display());

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_stem_lower = file_stem.to_lowercase();

    // Phase 7.6: Get file path for scope filtering
    let file_path_str = path.to_string_lossy();

    let mut signal = SemanticDriftSignal::new(path.to_string_lossy(), file_stem);

    // Phase 6.4: O(1) lookup via Symbol-to-Node Inverted Index
    if index.has_symbols() {
        // Try direct symbol lookup first (fast path)
        if let Some(symbol_refs) = index.lookup_symbol(file_stem) {
            info!(
                "Phase 6.4: O(1) cache hit for symbol '{}' ({} refs)",
                file_stem,
                symbol_refs.len()
            );
            add_symbol_refs_to_signal(&mut signal, symbol_refs, &file_path_str);
        }

        // Also check for snake_case and PascalCase variants
        let snake_variant = file_stem.to_lowercase().replace('-', "_");
        if snake_variant != file_stem
            && let Some(symbol_refs) = index.lookup_symbol(&snake_variant)
        {
            add_symbol_refs_to_signal(&mut signal, symbol_refs, &file_path_str);
        }

        // PascalCase variant (e.g., "user_handler" -> "UserHandler")
        let pascal_variant = to_pascal_case(file_stem);
        if let Some(symbol_refs) = index.lookup_symbol(&pascal_variant) {
            add_symbol_refs_to_signal(&mut signal, symbol_refs, &file_path_str);
        }
    }

    // Fall back to heuristic traversal if cache misses or is empty
    if signal.affected_docs.is_empty() {
        info!("Phase 6: Cache miss, falling back to heuristic traversal");
        let trees = index.all_page_index_trees();
        for (doc_id, nodes) in trees {
            traverse_nodes_for_observations(
                nodes,
                doc_id,
                file_stem,
                &file_stem_lower,
                &mut signal,
            );
        }
    }

    if signal.affected_docs.is_empty() {
        return Vec::new();
    }

    // Determine confidence based on match quality
    let has_explicit_reference = has_explicit_reference(&signal.affected_docs, file_stem);

    signal.update_confidence(if has_explicit_reference {
        DriftConfidence::High
    } else if signal.affected_docs.len() <= 3 {
        DriftConfidence::Medium
    } else {
        DriftConfidence::Low
    });

    info!(
        "Phase 6: {} documents potentially affected by source change.",
        signal.affected_docs.len()
    );

    vec![signal]
}

/// Convert `snake_case` to `PascalCase`.
#[must_use]
pub(crate) fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Recursively traverse page index nodes to find matching observations.
fn traverse_nodes_for_observations(
    nodes: &[PageIndexNode],
    doc_id: &str,
    file_stem: &str,
    file_stem_lower: &str,
    signal: &mut SemanticDriftSignal,
) {
    for node in nodes {
        // Check observations in this node's metadata
        for obs in &node.metadata.observations {
            let pattern_lower = obs.pattern.to_lowercase();

            // Heuristic matching: pattern contains file stem or related symbols
            let matches = pattern_lower.contains(file_stem_lower)
                || obs.pattern.contains(&format!("{file_stem}_{file_stem}"))
                || obs.pattern.contains(&format!("{file_stem}::"))
                || obs.pattern.contains(&format!("{file_stem}."));

            if matches {
                let affected = AffectedDoc::new(
                    doc_id,
                    obs.pattern.clone(),
                    obs.language.clone(),
                    node.node_id.clone(),
                )
                .with_line(obs.line_number.unwrap_or(node.metadata.line_range.0));

                signal.add_affected_doc(affected);
            }
        }

        // Recurse into children
        traverse_nodes_for_observations(&node.children, doc_id, file_stem, file_stem_lower, signal);
    }
}
