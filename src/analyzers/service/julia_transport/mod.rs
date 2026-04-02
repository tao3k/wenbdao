mod errors;
mod fetch;
mod request;
mod response;
mod schema;
#[cfg(test)]
mod tests;

#[cfg(feature = "julia")]
pub use fetch::{
    fetch_julia_flight_score_rows_for_repository, fetch_plugin_arrow_score_rows_for_repository,
};
#[cfg(feature = "julia")]
pub use request::{
    JuliaArrowRequestRow, PluginArrowRequestRow, build_julia_arrow_request_batch,
    build_plugin_arrow_request_batch,
};
#[cfg(feature = "julia")]
pub use response::{
    JuliaArrowScoreRow, PluginArrowScoreRow, decode_julia_arrow_score_rows,
    decode_plugin_arrow_score_rows,
};
pub use schema::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_EMBEDDING_COLUMN,
    JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_QUERY_EMBEDDING_COLUMN,
    JULIA_ARROW_TRACE_ID_COLUMN, JULIA_ARROW_VECTOR_SCORE_COLUMN, julia_arrow_request_schema,
    julia_arrow_response_schema,
};
