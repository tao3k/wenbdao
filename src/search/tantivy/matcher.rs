use std::cmp::Ordering;

use crate::search::fuzzy::{FuzzyMatch, FuzzyMatcher, FuzzySearchOptions};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Value};
use tantivy::{Index, TantivyDocument, TantivyError};

use super::compare::{best_match_candidate, collect_lowercase_chars};
use super::document::SearchDocumentMatchField;
use super::fields::SearchFieldSpec;

const FUZZY_CANDIDATE_WINDOW_CAP: usize = 96;
const FUZZY_CANDIDATE_WINDOW_MULTIPLIER: usize = 3;

/// One Tantivy-backed fuzzy match with matched-field metadata.
#[derive(Debug, Clone)]
pub struct TantivyDocumentMatch {
    /// Raw Tantivy document.
    pub item: TantivyDocument,
    /// Best-matching stored field when identified.
    pub matched_field: Option<SearchDocumentMatchField>,
    /// Best-matching text fragment.
    pub matched_text: String,
    /// Adjusted fuzzy score.
    pub score: f32,
    /// Edit distance for the chosen fragment.
    pub distance: usize,
}

/// Shared Tantivy-backed fuzzy matcher for text fields.
pub struct TantivyMatcher<'a> {
    index: &'a Index,
    default_fields: Vec<Field>,
    match_fields: Vec<SearchFieldSpec>,
    options: FuzzySearchOptions,
}

impl<'a> TantivyMatcher<'a> {
    /// Create a Tantivy fuzzy matcher for one primary match field.
    #[must_use]
    pub(crate) fn new(
        index: &'a Index,
        default_fields: Vec<Field>,
        match_fields: Vec<SearchFieldSpec>,
        options: FuzzySearchOptions,
    ) -> Self {
        Self {
            index,
            default_fields,
            match_fields,
            options,
        }
    }

    /// Search with fuzzy field metadata retained for rehydration-heavy callers.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot parse or execute the query.
    pub(crate) fn search_with_fields(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<TantivyDocumentMatch>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let mut query_chars = Vec::new();
        let mut candidate_chars = Vec::new();
        let mut scratch = Vec::new();
        let mut seen_ranges = Vec::new();
        let mut boundary_scratch = Vec::new();
        collect_lowercase_chars(query, &mut query_chars);

        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let mut parser = QueryParser::for_index(self.index, self.default_fields.clone());
        for spec in &self.match_fields {
            parser.set_field_fuzzy(
                spec.text_field,
                false,
                self.options.max_distance.min(2),
                self.options.transposition,
            );
        }
        let query_object = parser.parse_query(query)?;
        let candidate_limit = limit
            .max(1)
            .saturating_mul(FUZZY_CANDIDATE_WINDOW_MULTIPLIER)
            .min(FUZZY_CANDIDATE_WINDOW_CAP);
        let top_docs = searcher.search(&query_object, &TopDocs::with_limit(candidate_limit))?;

        let mut matches = Vec::new();
        for (_tantivy_score, doc_address) in top_docs {
            let document: TantivyDocument = searcher.doc(doc_address)?;
            let mut best: Option<(Option<SearchDocumentMatchField>, String, f32, usize)> = None;

            for spec in &self.match_fields {
                for stored_text in document
                    .get_all(spec.text_field)
                    .filter_map(|value| value.as_str())
                {
                    let Some((matched_text, score)) = best_match_candidate(
                        query,
                        query_chars.as_slice(),
                        stored_text,
                        self.options,
                        &mut candidate_chars,
                        &mut scratch,
                        &mut seen_ranges,
                        &mut boundary_scratch,
                    ) else {
                        continue;
                    };

                    let adjusted_score = score.score * spec.fuzzy_boost;
                    let replace = match best.as_ref() {
                        None => true,
                        Some((best_field, best_text, best_score, best_distance)) => {
                            compare_tantivy_match_parts(
                                TantivyMatchParts {
                                    field: Some(spec.label),
                                    text: matched_text.as_str(),
                                    score: adjusted_score,
                                    distance: score.distance,
                                },
                                TantivyMatchParts {
                                    field: *best_field,
                                    text: best_text.as_str(),
                                    score: *best_score,
                                    distance: *best_distance,
                                },
                            )
                            .is_lt()
                        }
                    };

                    if replace {
                        best = Some((
                            Some(spec.label),
                            matched_text,
                            adjusted_score,
                            score.distance,
                        ));
                    }
                }
            }

            let Some((matched_field, matched_text, score, distance)) = best else {
                continue;
            };
            matches.push(TantivyDocumentMatch {
                item: document,
                matched_field,
                matched_text,
                score,
                distance,
            });
        }

        matches.sort_by(compare_tantivy_matches);
        matches.truncate(limit);
        Ok(matches)
    }
}

impl FuzzyMatcher<TantivyDocument> for TantivyMatcher<'_> {
    type Error = TantivyError;

    fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<FuzzyMatch<TantivyDocument>>, Self::Error> {
        let matches = self
            .search_with_fields(query, limit)?
            .into_iter()
            .map(|hit| FuzzyMatch {
                item: hit.item,
                matched_text: hit.matched_text,
                score: hit.score,
                distance: hit.distance,
            })
            .collect::<Vec<FuzzyMatch<TantivyDocument>>>();
        Ok(matches)
    }
}

fn compare_tantivy_matches(left: &TantivyDocumentMatch, right: &TantivyDocumentMatch) -> Ordering {
    compare_tantivy_match_parts(
        TantivyMatchParts::from_match(left),
        TantivyMatchParts::from_match(right),
    )
}

#[derive(Clone, Copy)]
struct TantivyMatchParts<'a> {
    field: Option<SearchDocumentMatchField>,
    text: &'a str,
    score: f32,
    distance: usize,
}

impl<'a> TantivyMatchParts<'a> {
    fn from_match(hit: &'a TantivyDocumentMatch) -> Self {
        Self {
            field: hit.matched_field,
            text: hit.matched_text.as_str(),
            score: hit.score,
            distance: hit.distance,
        }
    }
}

fn compare_tantivy_match_parts(
    left: TantivyMatchParts<'_>,
    right: TantivyMatchParts<'_>,
) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.distance.cmp(&right.distance))
        .then_with(|| field_rank(left.field).cmp(&field_rank(right.field)))
        .then_with(|| left.text.len().cmp(&right.text.len()))
        .then_with(|| left.text.cmp(right.text))
}

fn field_rank(field: Option<SearchDocumentMatchField>) -> u8 {
    match field {
        Some(SearchDocumentMatchField::Title) => 0,
        Some(SearchDocumentMatchField::Namespace) => 1,
        Some(SearchDocumentMatchField::Path) => 2,
        Some(SearchDocumentMatchField::Terms) => 3,
        None => u8::MAX,
    }
}
