use crate::search::tantivy::document::SearchDocumentHit;
use crate::search::tantivy::index::core::SearchDocumentIndex;
use crate::search::tantivy::index::helpers::{candidate_limit, collect_hits};
use crate::search::tantivy::tokenizer::collect_search_tokens;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, PhrasePrefixQuery, Query};
use tantivy::{TantivyError, Term};

impl SearchDocumentIndex {
    /// Run a phrase-prefix query over the shared search fields.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot execute the prefix query.
    pub fn search_prefix_hits(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchDocumentHit>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let Some(query_object) = self.build_phrase_prefix_query(query) else {
            return Ok(Vec::new());
        };

        let searcher = self.reader.searcher();
        let top_docs =
            searcher.search(&*query_object, &TopDocs::with_limit(candidate_limit(limit)))?;
        collect_hits(&self.fields, &searcher, top_docs, limit)
    }

    fn build_phrase_prefix_query(&self, query: &str) -> Option<Box<dyn Query>> {
        let tokens = collect_search_tokens(&self.index, query);
        if tokens.len() < 2 {
            return None;
        }

        let clauses = self
            .fields
            .match_field_specs()
            .into_iter()
            .map(|spec| {
                let terms = tokens
                    .iter()
                    .map(|token| Term::from_field_text(spec.text_field, token))
                    .collect::<Vec<_>>();
                (
                    Occur::Should,
                    Box::new(BoostQuery::new(
                        Box::new(PhrasePrefixQuery::new(terms)),
                        spec.query_boost,
                    )) as Box<dyn Query>,
                )
            })
            .collect::<Vec<_>>();
        Some(Box::new(BooleanQuery::new(clauses)))
    }
}
