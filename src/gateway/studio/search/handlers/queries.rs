use serde::Deserialize;

#[cfg(test)]
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    #[serde(alias = "query")]
    pub q: Option<String>,
    #[serde(default)]
    pub intent: Option<String>,
    #[cfg(test)]
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct AttachmentSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub ext: Vec<String>,
    #[serde(default)]
    pub kind: Vec<String>,
    #[serde(default)]
    pub case_sensitive: bool,
}

#[derive(Debug, Deserialize)]
pub struct AstSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ReferenceSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SymbolSearchQuery {
    pub q: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}
