use crate::search::SearchDocument;
use crate::unified_symbol::symbol::SymbolSource;
use crate::unified_symbol::{UnifiedSymbol, UnifiedSymbolIndex};

impl UnifiedSymbolIndex {
    /// Add a project symbol.
    pub fn add_project_symbol(&mut self, name: &str, kind: &str, location: &str, crate_name: &str) {
        let symbol = UnifiedSymbol::new_project(name, kind, location, crate_name);
        self.add_symbol(symbol);
    }

    /// Add an external dependency symbol.
    pub fn add_external_symbol(
        &mut self,
        name: &str,
        kind: &str,
        location: &str,
        crate_name: &str,
    ) {
        let symbol = UnifiedSymbol::new_external(name, kind, location, crate_name);
        self.add_symbol(symbol);
    }

    /// Add a symbol from `repo_intelligence` analysis.
    pub fn add_symbol_record(&mut self, record: &crate::analyzers::SymbolRecord) {
        let kind_str = match record.kind {
            crate::analyzers::RepoSymbolKind::Function => "fn",
            crate::analyzers::RepoSymbolKind::Type => "type",
            crate::analyzers::RepoSymbolKind::Constant => "const",
            crate::analyzers::RepoSymbolKind::ModuleExport => "export",
            crate::analyzers::RepoSymbolKind::Other => "other",
        };

        let source = if record.repo_id == "stdlib" {
            SymbolSource::External("stdlib".to_string())
        } else {
            SymbolSource::Project
        };

        let symbol = UnifiedSymbol {
            name: record.name.clone(),
            kind: kind_str.to_string(),
            location: record.path.clone(),
            source,
            crate_name: record.repo_id.clone(),
        };
        self.add_symbol(symbol);
    }

    /// Record usage of an external symbol in a project file.
    pub fn record_external_usage(
        &mut self,
        crate_name: &str,
        symbol_name: &str,
        project_file: &str,
    ) {
        self.external_usage
            .entry(crate_name.to_string())
            .or_default()
            .push(project_file.to_string());

        self.project_files
            .entry(project_file.to_string())
            .or_default()
            .push(symbol_name.to_string());
    }

    pub(crate) fn add_symbol(&mut self, symbol: UnifiedSymbol) {
        // 1. In-memory fallback
        let idx = self.symbols.len();
        let key = symbol.name.to_lowercase();
        self.symbols.push(symbol);
        self.by_name.entry(key).or_default().push(idx);

        // 2. Shared search indexing
        let stored = &self.symbols[idx];
        let source_str = match &stored.source {
            SymbolSource::Project => "project",
            SymbolSource::External(_) => "external",
        };
        let _ = self.search_index.add_document(&SearchDocument {
            id: idx.to_string(),
            title: stored.name.clone(),
            kind: stored.kind.clone(),
            path: stored.location.clone(),
            scope: source_str.to_string(),
            namespace: stored.crate_name.clone(),
            terms: vec![
                stored.crate_name.clone(),
                stored.kind.clone(),
                stored.location.clone(),
                source_str.to_string(),
            ],
        });
    }

    /// Clear all symbols.
    pub fn clear(&mut self) {
        self.by_name.clear();
        self.symbols.clear();
        self.external_usage.clear();
        self.project_files.clear();
        self.search_index = crate::SearchDocumentIndex::new();
    }
}
