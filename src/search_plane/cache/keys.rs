use crate::search_plane::SearchCorpusKind;
use crate::search_plane::cache::SearchPlaneCache;

impl SearchPlaneCache {
    pub(crate) fn autocomplete_cache_key(
        &self,
        prefix: &str,
        limit: usize,
        active_epoch: u64,
    ) -> Option<String> {
        self.client.as_ref()?;
        let token = hashed_cache_token(
            "autocomplete",
            [
                format!("epoch:{active_epoch}"),
                format!("limit:{limit}"),
                format!("prefix:{}", normalize_cache_text(prefix)),
            ],
        );
        Some(self.keyspace.autocomplete_cache_key(token.as_str()))
    }

    pub(crate) fn search_query_cache_key(
        &self,
        scope: &str,
        epochs: &[(SearchCorpusKind, u64)],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        let versions = epochs
            .iter()
            .map(|(corpus, epoch)| format!("{corpus}:{epoch}"))
            .collect::<Vec<_>>();
        self.search_query_cache_key_from_versions(
            scope,
            versions.as_slice(),
            query,
            limit,
            intent,
            repo_hint,
        )
    }

    pub(crate) fn search_query_cache_key_from_versions(
        &self,
        scope: &str,
        versions: &[String],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        self.client.as_ref()?;
        let mut components = Vec::with_capacity(4 + versions.len());
        let mut normalized_versions = versions.to_vec();
        normalized_versions.sort_unstable();
        normalized_versions.dedup();
        components.extend(normalized_versions);
        components.push(format!("limit:{limit}"));
        components.push(format!("query:{}", normalize_cache_text(query)));
        components.push(format!(
            "intent:{}",
            normalize_cache_text(intent.unwrap_or_default())
        ));
        components.push(format!(
            "repo:{}",
            normalize_cache_text(repo_hint.unwrap_or_default())
        ));
        let token = hashed_cache_token(scope, components);
        Some(self.keyspace.search_query_cache_key(scope, token.as_str()))
    }
}

fn normalize_cache_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn hashed_cache_token<I>(scope: &str, components: I) -> String
where
    I: IntoIterator<Item = String>,
{
    let mut payload = String::from(scope);
    for component in components {
        payload.push('|');
        payload.push_str(component.as_str());
    }
    blake3::hash(payload.as_bytes()).to_hex().to_string()
}
