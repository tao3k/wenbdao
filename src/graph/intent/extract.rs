use super::models::QueryIntent;
use super::vocabulary::{ACTION_VERBS, DOMAIN_TARGETS, STOP_WORDS};
use std::collections::HashSet;

/// Extract a structured `QueryIntent` from a raw natural-language query.
///
/// The algorithm is zero-allocation-heavy and rule-based — no ML model needed.
/// It runs in microseconds, making it suitable for hot-path routing.
#[must_use]
pub fn extract_intent(query: &str) -> QueryIntent {
    let normalized = query.trim().to_lowercase();
    if normalized.is_empty() {
        return QueryIntent {
            normalized_query: normalized,
            ..Default::default()
        };
    }

    let stop_set: HashSet<&str> = STOP_WORDS.iter().copied().collect();

    // Tokenize: split on whitespace and common delimiters
    let tokens: Vec<&str> = normalized
        .split(|c: char| {
            c.is_whitespace() || c == '.' || c == '_' || c == '-' || c == '/' || c == ','
        })
        .filter(|token| !token.is_empty())
        .collect();

    // Extract keywords (non-stop-word tokens with len >= 2)
    let keywords: Vec<String> = tokens
        .iter()
        .filter(|token| token.len() >= 2 && !stop_set.contains(**token))
        .map(std::string::ToString::to_string)
        .collect();

    // Identify action — first matching action verb (order-sensitive: first token wins)
    let mut action: Option<String> = None;
    for token in &tokens {
        if let Some((_, canonical)) = ACTION_VERBS.iter().find(|(verb, _)| verb == token) {
            action = Some(canonical.to_string());
            break;
        }
    }

    // Identify target — first matching domain noun that is NOT the action token
    let mut target: Option<String> = None;
    for token in &tokens {
        // Skip the action token itself to avoid "commit" mapping to both action and target
        if action.as_deref() == Some(*token) {
            continue;
        }
        if let Some((_, canonical)) = DOMAIN_TARGETS.iter().find(|(noun, _)| noun == token) {
            target = Some(canonical.to_string());
            break;
        }
    }

    // If no explicit target found, try to infer from action context
    if target.is_none()
        && let Some(ref action_name) = action
    {
        match action_name.as_str() {
            "commit" | "push" | "pull" | "merge" | "rebase" | "branch" | "checkout" | "diff"
            | "status" | "log" | "stash" => {
                target = Some("git".to_string());
            }
            "crawl" | "research" => {
                target = Some("web".to_string());
            }
            "embed" | "index" => {
                target = Some("database".to_string());
            }
            _ => {}
        }
    }

    // Context: remaining non-stop, non-action, non-target keywords
    let action_tokens: HashSet<&str> = ACTION_VERBS
        .iter()
        .filter_map(|(verb, canonical)| {
            if action.as_deref() == Some(*canonical) {
                Some(*verb)
            } else {
                None
            }
        })
        .collect();

    let target_tokens: HashSet<&str> = DOMAIN_TARGETS
        .iter()
        .filter_map(|(noun, canonical)| {
            if target.as_deref() == Some(*canonical) {
                Some(*noun)
            } else {
                None
            }
        })
        .collect();

    let context: Vec<String> = keywords
        .iter()
        .filter(|kw| !action_tokens.contains(kw.as_str()) && !target_tokens.contains(kw.as_str()))
        .cloned()
        .collect();

    QueryIntent {
        action,
        target,
        context,
        keywords,
        normalized_query: normalized,
    }
}
