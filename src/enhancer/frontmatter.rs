use crate::enhancer::types::NoteFrontmatter;

/// Extract YAML frontmatter from markdown content.
///
/// Looks for `---\n...\n---\n` at the start of the content.
fn extract_frontmatter_yaml(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }

    let after_first = &trimmed[3..];
    // Find the closing ---
    if let Some(end_pos) = after_first.find("\n---") {
        let yaml = &after_first[..end_pos];
        // Skip leading newline
        let yaml = yaml.strip_prefix('\n').unwrap_or(yaml);
        Some(yaml.to_string())
    } else {
        None
    }
}

/// Parse frontmatter from markdown content.
#[must_use]
pub fn parse_frontmatter(content: &str) -> NoteFrontmatter {
    let Some(yaml_str) = extract_frontmatter_yaml(content) else {
        return NoteFrontmatter::default();
    };

    // Parse top-level YAML
    let value: serde_yaml::Value = match serde_yaml::from_str(&yaml_str) {
        Ok(v) => v,
        Err(_) => return NoteFrontmatter::default(),
    };

    let Some(mapping) = value.as_mapping() else {
        return NoteFrontmatter::default();
    };

    let get_str = |key: &str| -> Option<String> {
        mapping
            .get(serde_yaml::Value::String(key.to_string()))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };

    let get_str_vec = |key: &str| -> Vec<String> {
        mapping
            .get(serde_yaml::Value::String(key.to_string()))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    // Check nested metadata block
    let metadata = mapping
        .get(serde_yaml::Value::String("metadata".to_string()))
        .and_then(|v| v.as_mapping());

    let get_metadata_vec = |key: &str| -> Vec<String> {
        metadata
            .and_then(|m| m.get(serde_yaml::Value::String(key.to_string())))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut tags = get_str_vec("tags");
    if tags.is_empty() {
        tags = get_metadata_vec("tags");
    }

    NoteFrontmatter {
        title: get_str("title"),
        description: get_str("description"),
        name: get_str("name"),
        category: get_str("category"),
        tags,
        routing_keywords: get_metadata_vec("routing_keywords"),
        intents: get_metadata_vec("intents"),
    }
}
