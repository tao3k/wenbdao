use std::collections::BTreeMap;

use crate::gateway::studio::types::ReferenceSearchHit;
use crate::search_plane::reference_occurrence::ReferenceOccurrenceSearchError;
use xiuxian_vector::SearchEngineContext;

use super::candidates::ReferenceOccurrenceCandidate;

pub(super) async fn decode_reference_hits(
    engine: &SearchEngineContext,
    table_name: &str,
    candidates: Vec<ReferenceOccurrenceCandidate>,
) -> Result<Vec<ReferenceSearchHit>, ReferenceOccurrenceSearchError> {
    let payloads = load_hit_payloads_by_id(engine, table_name, candidates.as_slice()).await?;
    candidates
        .into_iter()
        .map(|candidate| {
            let hit_json = payloads.get(candidate.id.as_str()).ok_or_else(|| {
                ReferenceOccurrenceSearchError::Decode(format!(
                    "reference occurrence hydration missing payload for id `{}`",
                    candidate.id
                ))
            })?;
            let mut hit: ReferenceSearchHit = serde_json::from_str(hit_json.as_str())
                .map_err(|error| ReferenceOccurrenceSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

async fn load_hit_payloads_by_id(
    engine: &SearchEngineContext,
    table_name: &str,
    candidates: &[ReferenceOccurrenceCandidate],
) -> Result<BTreeMap<String, String>, ReferenceOccurrenceSearchError> {
    if candidates.is_empty() {
        return Ok(BTreeMap::new());
    }

    let sql = format!(
        "SELECT {id_column}, {hit_json_column} FROM {table_name} WHERE {id_column} IN ({ids})",
        id_column = crate::search_plane::reference_occurrence::schema::id_column(),
        hit_json_column = crate::search_plane::reference_occurrence::schema::hit_json_column(),
        ids = candidates
            .iter()
            .map(|candidate| sql_string_literal(candidate.id.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let batches = engine.sql_batches(sql.as_str()).await?;
    let mut payloads = BTreeMap::new();

    for batch in batches {
        let id = string_column(
            &batch,
            crate::search_plane::reference_occurrence::schema::id_column(),
        )?;
        let hit_json = string_column(
            &batch,
            crate::search_plane::reference_occurrence::schema::hit_json_column(),
        )?;
        for row in 0..batch.num_rows() {
            payloads.insert(id.value(row).to_string(), hit_json.value(row).to_string());
        }
    }

    Ok(payloads)
}

fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn string_column<'a>(
    batch: &'a xiuxian_vector::EngineRecordBatch,
    name: &str,
) -> Result<EngineStringColumn<'a>, ReferenceOccurrenceSearchError> {
    let column = batch.column_by_name(name).ok_or_else(|| {
        ReferenceOccurrenceSearchError::Decode(format!("missing engine string column `{name}`"))
    })?;
    if let Some(array) = column.as_any().downcast_ref::<arrow::array::StringArray>() {
        return Ok(EngineStringColumn::Utf8(array));
    }
    if let Some(array) = column
        .as_any()
        .downcast_ref::<arrow::array::StringViewArray>()
    {
        return Ok(EngineStringColumn::Utf8View(array));
    }
    Err(ReferenceOccurrenceSearchError::Decode(format!(
        "engine column `{name}` is not utf8-like"
    )))
}

#[derive(Clone, Copy)]
enum EngineStringColumn<'a> {
    Utf8(&'a arrow::array::StringArray),
    Utf8View(&'a arrow::array::StringViewArray),
}

impl<'a> EngineStringColumn<'a> {
    fn value(self, row: usize) -> &'a str {
        match self {
            Self::Utf8(column) => column.value(row),
            Self::Utf8View(column) => column.value(row),
        }
    }
}
