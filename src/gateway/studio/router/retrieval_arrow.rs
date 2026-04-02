use std::sync::Arc;

#[cfg(test)]
use std::io::Cursor;

#[cfg(test)]
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use arrow::ipc::writer::StreamWriter;
#[cfg(test)]
use xiuxian_vector::lance_batch_to_engine_batch;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray, LanceUInt64Array,
};

use crate::gateway::studio::types::{RetrievalChunk, RetrievalChunkSurface};

pub(crate) fn build_retrieval_chunks_flight_batch(
    chunks: &[RetrievalChunk],
) -> Result<LanceRecordBatch, String> {
    let owner_ids = chunks
        .iter()
        .map(|chunk| chunk.owner_id.clone())
        .collect::<Vec<_>>();
    let chunk_ids = chunks
        .iter()
        .map(|chunk| chunk.chunk_id.clone())
        .collect::<Vec<_>>();
    let semantic_types = chunks
        .iter()
        .map(|chunk| chunk.semantic_type.clone())
        .collect::<Vec<_>>();
    let fingerprints = chunks
        .iter()
        .map(|chunk| chunk.fingerprint.clone())
        .collect::<Vec<_>>();
    let token_estimates = chunks
        .iter()
        .map(|chunk| {
            u64::try_from(chunk.token_estimate)
                .map_err(|_| "tokenEstimate exceeds u64 range".to_string())
        })
        .collect::<Result<Vec<u64>, _>>()?;
    let display_labels = chunks
        .iter()
        .map(|chunk| chunk.display_label.clone())
        .collect::<Vec<_>>();
    let excerpts = chunks
        .iter()
        .map(|chunk| chunk.excerpt.clone())
        .collect::<Vec<_>>();
    let line_starts = chunks
        .iter()
        .map(|chunk| {
            chunk
                .line_start
                .map(|value| {
                    u64::try_from(value).map_err(|_| "lineStart exceeds u64 range".to_string())
                })
                .transpose()
        })
        .collect::<Result<Vec<Option<u64>>, _>>()?;
    let line_ends = chunks
        .iter()
        .map(|chunk| {
            chunk
                .line_end
                .map(|value| {
                    u64::try_from(value).map_err(|_| "lineEnd exceeds u64 range".to_string())
                })
                .transpose()
        })
        .collect::<Result<Vec<Option<u64>>, _>>()?;
    let surfaces = chunks
        .iter()
        .map(|chunk| {
            chunk
                .surface
                .map(retrieval_surface_label)
                .map(ToString::to_string)
        })
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("ownerId", LanceDataType::Utf8, false),
            LanceField::new("chunkId", LanceDataType::Utf8, false),
            LanceField::new("semanticType", LanceDataType::Utf8, false),
            LanceField::new("fingerprint", LanceDataType::Utf8, false),
            LanceField::new("tokenEstimate", LanceDataType::UInt64, false),
            LanceField::new("displayLabel", LanceDataType::Utf8, true),
            LanceField::new("excerpt", LanceDataType::Utf8, true),
            LanceField::new("lineStart", LanceDataType::UInt64, true),
            LanceField::new("lineEnd", LanceDataType::UInt64, true),
            LanceField::new("surface", LanceDataType::Utf8, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(owner_ids)),
            Arc::new(LanceStringArray::from(chunk_ids)),
            Arc::new(LanceStringArray::from(semantic_types)),
            Arc::new(LanceStringArray::from(fingerprints)),
            Arc::new(LanceUInt64Array::from(token_estimates)),
            Arc::new(LanceStringArray::from(display_labels)),
            Arc::new(LanceStringArray::from(excerpts)),
            Arc::new(LanceUInt64Array::from(line_starts)),
            Arc::new(LanceUInt64Array::from(line_ends)),
            Arc::new(LanceStringArray::from(surfaces)),
        ],
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
pub(crate) fn encode_retrieval_chunks_ipc(chunks: &[RetrievalChunk]) -> Result<Vec<u8>, String> {
    let batch = build_retrieval_chunks_flight_batch(chunks)?;
    let engine_batch = lance_batch_to_engine_batch(&batch).map_err(|error| error.to_string())?;
    let schema = Schema::new(vec![
        Field::new("ownerId", DataType::Utf8, false),
        Field::new("chunkId", DataType::Utf8, false),
        Field::new("semanticType", DataType::Utf8, false),
        Field::new("fingerprint", DataType::Utf8, false),
        Field::new("tokenEstimate", DataType::UInt64, false),
        Field::new("displayLabel", DataType::Utf8, true),
        Field::new("excerpt", DataType::Utf8, true),
        Field::new("lineStart", DataType::UInt64, true),
        Field::new("lineEnd", DataType::UInt64, true),
        Field::new("surface", DataType::Utf8, true),
    ]);
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut writer =
            StreamWriter::try_new(&mut buffer, &schema).map_err(|error| error.to_string())?;
        writer
            .write(&engine_batch)
            .map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
    }
    Ok(buffer.into_inner())
}

const fn retrieval_surface_label(surface: RetrievalChunkSurface) -> &'static str {
    match surface {
        RetrievalChunkSurface::Document => "document",
        RetrievalChunkSurface::Section => "section",
        RetrievalChunkSurface::CodeBlock => "codeblock",
        RetrievalChunkSurface::Table => "table",
        RetrievalChunkSurface::Math => "math",
        RetrievalChunkSurface::Observation => "observation",
        RetrievalChunkSurface::Declaration => "declaration",
        RetrievalChunkSurface::Block => "block",
        RetrievalChunkSurface::Symbol => "symbol",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{StringArray, UInt64Array};
    use arrow::ipc::reader::StreamReader;

    #[test]
    fn retrieval_arrow_roundtrip_preserves_chunk_fields() {
        let chunks = vec![
            RetrievalChunk {
                owner_id: "section:intro".to_string(),
                chunk_id: "md:intro".to_string(),
                semantic_type: "section".to_string(),
                fingerprint: "fp:intro".to_string(),
                token_estimate: 18,
                display_label: Some("Intro".to_string()),
                excerpt: Some("Hello world".to_string()),
                line_start: Some(1),
                line_end: Some(4),
                surface: Some(RetrievalChunkSurface::Section),
            },
            RetrievalChunk {
                owner_id: "block:return:solve".to_string(),
                chunk_id: "ast:return:solve".to_string(),
                semantic_type: "return".to_string(),
                fingerprint: "fp:return".to_string(),
                token_estimate: 9,
                display_label: None,
                excerpt: None,
                line_start: Some(22),
                line_end: Some(24),
                surface: Some(RetrievalChunkSurface::Block),
            },
        ];

        let encoded = encode_retrieval_chunks_ipc(&chunks).expect("arrow encoding should succeed");
        let reader =
            StreamReader::try_new(Cursor::new(encoded), None).expect("stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("batches should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 2);

        let owner_ids = batch
            .column_by_name("ownerId")
            .expect("ownerId column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("ownerId should be utf8");
        assert_eq!(owner_ids.value(0), "section:intro");
        assert_eq!(owner_ids.value(1), "block:return:solve");

        let token_estimates = batch
            .column_by_name("tokenEstimate")
            .expect("tokenEstimate column")
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("tokenEstimate should be u64");
        assert_eq!(token_estimates.value(0), 18);
        assert_eq!(token_estimates.value(1), 9);

        let surfaces = batch
            .column_by_name("surface")
            .expect("surface column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("surface should be utf8");
        assert_eq!(surfaces.value(0), "section");
        assert_eq!(surfaces.value(1), "block");
    }
}
