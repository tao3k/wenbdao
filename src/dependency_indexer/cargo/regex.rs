use regex::Regex;
use std::sync::LazyLock;

fn compile_regex(pattern: &str) -> Regex {
    match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(_compile_err) => match Regex::new(r"$^") {
            Ok(fallback) => fallback,
            Err(fallback_err) => panic!("hardcoded fallback regex must compile: {fallback_err}"),
        },
    }
}

/// Regex for complex dependency format: name = { version = "x.y.z", features = [...] }
pub(super) static RE_DEP_COMPLEX: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r#"(\w+)\s*=\s*\{[^}]*version\s*=\s*"([^"]+)""#));

/// Regex for simple dependency format: name = "version"
pub(super) static RE_DEP_SIMPLE: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r#"^(\w+)\s*=\s*"([^"]+)""#));
