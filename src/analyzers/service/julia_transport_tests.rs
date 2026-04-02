#![cfg(test)]

pub(crate) mod fixtures {
    use std::sync::Arc;

    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;

    use crate::analyzers::service::{
        JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_FINAL_SCORE_COLUMN, PluginArrowRequestRow,
        build_plugin_arrow_request_batch, julia_arrow_response_schema,
    };

    pub(crate) fn request_batch() -> RecordBatch {
        build_plugin_arrow_request_batch(
            &[
                PluginArrowRequestRow {
                    doc_id: "doc-1".to_string(),
                    vector_score: 0.3,
                    embedding: vec![1.0, 2.0, 3.0],
                },
                PluginArrowRequestRow {
                    doc_id: "doc-2".to_string(),
                    vector_score: 0.4,
                    embedding: vec![4.0, 5.0, 6.0],
                },
            ],
            &[9.0, 8.0, 7.0],
        )
        .expect("request batch")
    }

    pub(crate) fn response_batch() -> RecordBatch {
        let schema = julia_arrow_response_schema(false);
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.9, 0.7])),
                Arc::new(Float64Array::from(vec![0.95, 0.8])),
            ],
        )
        .expect("response batch")
    }

    pub(crate) fn response_batch_with_trace_ids() -> RecordBatch {
        let schema = julia_arrow_response_schema(true);
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.9, 0.7])),
                Arc::new(Float64Array::from(vec![0.95, 0.8])),
                Arc::new(StringArray::from(vec!["trace-123", "trace-123"])),
            ],
        )
        .expect("response batch")
    }

    pub(crate) fn invalid_response_missing_analyzer_score_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new(JULIA_ARROW_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(JULIA_ARROW_FINAL_SCORE_COLUMN, DataType::Float64, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(Float64Array::from(vec![0.95])),
            ],
        )
        .expect("invalid response batch")
    }
}
