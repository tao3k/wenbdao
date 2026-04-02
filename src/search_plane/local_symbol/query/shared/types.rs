use crate::gateway::studio::types::AutocompleteSuggestion;
use crate::search_plane::ranking::{StreamingRerankSource, StreamingRerankTelemetry};
use xiuxian_vector::VectorStoreError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LocalSymbolSearchError {
    #[error("local symbol index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

#[derive(Debug)]
pub(crate) struct LocalSymbolSearchExecution {
    pub(crate) candidates: Vec<LocalSymbolCandidate>,
    pub(crate) telemetry: StreamingRerankTelemetry,
    pub(crate) source: StreamingRerankSource,
}

#[derive(Debug)]
pub(crate) struct LocalSymbolAutocompleteExecution {
    pub(crate) suggestions: Vec<AutocompleteSuggestion>,
    pub(crate) telemetry: StreamingRerankTelemetry,
    pub(crate) source: StreamingRerankSource,
}

#[derive(Debug)]
pub(crate) struct LocalSymbolCandidate {
    pub(crate) table_name: String,
    pub(crate) id: String,
    pub(crate) score: f64,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) line_start: usize,
}
