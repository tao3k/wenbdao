pub(super) fn has_valkey() -> bool {
    std::env::var("VALKEY_URL")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}
