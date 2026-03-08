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

pub(super) static WIKILINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"\[\[([^\]#\|]+)(?:#([^\]#\|]+))?(?:\|[^\]]+)?\]\]"));
pub(super) static WIKILINK_REGEX_EXACT: LazyLock<Regex> =
    LazyLock::new(|| compile_regex(r"^\[\[([^\]#\|]+)(?:#([^\]#\|]+))?(?:\|[^\]]+)?\]\]$"));
