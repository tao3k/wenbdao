use std::sync::Arc;

use crate::analyzers::RepositoryAnalysisOutput;
use crate::analyzers::cache::RepositorySearchArtifacts;
use crate::analyzers::query::{
    ExampleSearchQuery, ExampleSearchResult, ImportSearchQuery, ImportSearchResult,
    ModuleSearchQuery, ModuleSearchResult, SymbolSearchQuery, SymbolSearchResult,
};
use crate::search::FuzzySearchOptions;

use super::{
    build_example_search_with_artifacts, build_import_search_with_artifacts,
    build_module_search_with_artifacts, build_symbol_search_with_artifacts,
};

pub(crate) struct RepoAnalysisFallbackContract<Q, T> {
    pub(crate) scope: &'static str,
    pub(crate) fuzzy_options: FuzzySearchOptions,
    pub(crate) build_query: Arc<dyn Fn(String, String, usize) -> Q + Send + Sync>,
    pub(crate) query_text: Arc<dyn Fn(&Q) -> String + Send + Sync>,
    pub(crate) query_limit: Arc<dyn Fn(&Q) -> usize + Send + Sync>,
    pub(crate) build_result:
        Arc<dyn Fn(&Q, &RepositoryAnalysisOutput, &RepositorySearchArtifacts) -> T + Send + Sync>,
}

pub(crate) fn canonical_import_query_text(query: &ImportSearchQuery) -> String {
    let package = query.package.as_deref().unwrap_or("*");
    let module = query.module.as_deref().unwrap_or("*");
    format!("package={package};module={module}")
}

pub(crate) fn module_fallback_contract()
-> RepoAnalysisFallbackContract<ModuleSearchQuery, ModuleSearchResult> {
    RepoAnalysisFallbackContract {
        scope: "repo.module-search",
        fuzzy_options: FuzzySearchOptions::path_search(),
        build_query: Arc::new(|repo_id, query, limit| ModuleSearchQuery {
            repo_id,
            query,
            limit,
        }),
        query_text: Arc::new(|query| query.query.clone()),
        query_limit: Arc::new(|query| query.limit),
        build_result: Arc::new(build_module_search_with_artifacts),
    }
}

pub(crate) fn symbol_fallback_contract()
-> RepoAnalysisFallbackContract<SymbolSearchQuery, SymbolSearchResult> {
    RepoAnalysisFallbackContract {
        scope: "repo.symbol-search",
        fuzzy_options: FuzzySearchOptions::symbol_search(),
        build_query: Arc::new(|repo_id, query, limit| SymbolSearchQuery {
            repo_id,
            query,
            limit,
        }),
        query_text: Arc::new(|query| query.query.clone()),
        query_limit: Arc::new(|query| query.limit),
        build_result: Arc::new(build_symbol_search_with_artifacts),
    }
}

pub(crate) fn example_fallback_contract()
-> RepoAnalysisFallbackContract<ExampleSearchQuery, ExampleSearchResult> {
    RepoAnalysisFallbackContract {
        scope: "repo.example-search",
        fuzzy_options: FuzzySearchOptions::document_search(),
        build_query: Arc::new(|repo_id, query, limit| ExampleSearchQuery {
            repo_id,
            query,
            limit,
        }),
        query_text: Arc::new(|query| query.query.clone()),
        query_limit: Arc::new(|query| query.limit),
        build_result: Arc::new(build_example_search_with_artifacts),
    }
}

pub(crate) fn import_fallback_contract(
    package: Option<String>,
    module: Option<String>,
) -> RepoAnalysisFallbackContract<ImportSearchQuery, ImportSearchResult> {
    RepoAnalysisFallbackContract {
        scope: "repo.import-search",
        fuzzy_options: FuzzySearchOptions::symbol_search(),
        build_query: Arc::new(move |repo_id, _query, limit| ImportSearchQuery {
            repo_id,
            package: package.clone(),
            module: module.clone(),
            limit,
        }),
        query_text: Arc::new(canonical_import_query_text),
        query_limit: Arc::new(|query| query.limit),
        build_result: Arc::new(build_import_search_with_artifacts),
    }
}

#[cfg(test)]
mod tests {
    use crate::analyzers::query::ImportSearchQuery;

    #[test]
    fn canonical_import_query_text_preserves_package_and_module_identity() {
        let query = ImportSearchQuery {
            repo_id: "alpha/repo".to_string(),
            package: Some("SciMLBase".to_string()),
            module: Some("BaseModelica".to_string()),
            limit: 10,
        };

        assert_eq!(
            super::canonical_import_query_text(&query),
            "package=SciMLBase;module=BaseModelica"
        );
    }

    #[test]
    fn canonical_import_query_text_uses_stable_wildcards_for_missing_filters() {
        let query = ImportSearchQuery {
            repo_id: "alpha/repo".to_string(),
            package: Some("SciMLBase".to_string()),
            module: None,
            limit: 10,
        };

        assert_eq!(
            super::canonical_import_query_text(&query),
            "package=SciMLBase;module=*"
        );
    }
}
