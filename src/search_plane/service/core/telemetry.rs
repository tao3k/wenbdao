use super::types::SearchPlaneService;
use crate::search_plane::{SearchCorpusKind, SearchQueryTelemetry};

impl SearchPlaneService {
    pub(crate) fn record_query_telemetry(
        &self,
        corpus: SearchCorpusKind,
        telemetry: SearchQueryTelemetry,
    ) {
        self.query_telemetry
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(corpus, telemetry);
    }

    pub(crate) fn query_telemetry_for(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<SearchQueryTelemetry> {
        self.query_telemetry
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&corpus)
            .cloned()
    }
}
