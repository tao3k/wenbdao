use super::CodeObservation;
use std::collections::HashMap;
use std::hash::BuildHasher;

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

    // Check for single :OBSERVE: entry.
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
