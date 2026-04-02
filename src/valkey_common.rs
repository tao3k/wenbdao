use std::env;

/// Return the first non-empty environment value from a precedence-ordered list.
#[must_use]
pub(crate) fn first_non_empty_env(names: &[&str]) -> Option<String> {
    first_non_empty_value(names.iter().map(|name| env::var(name).ok()))
}

#[must_use]
fn first_non_empty_value<I>(values: I) -> Option<String>
where
    I: IntoIterator<Item = Option<String>>,
{
    values.into_iter().find_map(|candidate| {
        candidate
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

#[must_use]
fn open_optional_client(valkey_url: Option<String>) -> Option<redis::Client> {
    valkey_url.and_then(|value| open_client(value.as_str()).ok())
}

/// Open a Valkey client from one already-resolved endpoint URL.
///
/// # Errors
///
/// Returns the underlying `redis` client construction error when the URL is
/// invalid.
pub(crate) fn open_client(valkey_url: &str) -> Result<redis::Client, redis::RedisError> {
    redis::Client::open(valkey_url.trim())
}

/// Normalize one optional key prefix with a required default.
#[must_use]
pub(crate) fn normalize_key_prefix(candidate: &str, default_prefix: &str) -> String {
    let normalized = candidate.trim();
    if normalized.is_empty() {
        default_prefix.to_string()
    } else {
        normalized.to_string()
    }
}

/// Open a Valkey client from a precedence-ordered environment list.
#[must_use]
pub(crate) fn resolve_optional_client_from_env(names: &[&str]) -> Option<redis::Client> {
    open_optional_client(first_non_empty_env(names))
}

#[cfg(test)]
mod tests {
    use super::{first_non_empty_value, normalize_key_prefix, open_client, open_optional_client};

    #[test]
    fn first_non_empty_value_skips_blank_candidates() {
        assert_eq!(
            first_non_empty_value([
                Some("   ".to_string()),
                None,
                Some(" redis://127.0.0.1/ ".to_string()),
            ]),
            Some("redis://127.0.0.1/".to_string())
        );
    }

    #[test]
    fn open_optional_client_returns_none_for_missing_url() {
        assert!(open_optional_client(None).is_none());
    }

    #[test]
    fn open_optional_client_opens_valid_url() {
        let client = open_optional_client(Some("redis://127.0.0.1/".to_string()));
        assert!(client.is_some());
    }

    #[test]
    fn open_client_trims_valid_url() {
        let client = open_client(" redis://127.0.0.1/ ");
        assert!(client.is_ok());
    }

    #[test]
    fn normalize_key_prefix_falls_back_for_blank_input() {
        assert_eq!(
            normalize_key_prefix("   ", "xiuxian:test"),
            "xiuxian:test".to_string()
        );
    }

    #[test]
    fn normalize_key_prefix_trims_non_blank_input() {
        assert_eq!(
            normalize_key_prefix("  xiuxian:custom  ", "xiuxian:test"),
            "xiuxian:custom".to_string()
        );
    }
}
