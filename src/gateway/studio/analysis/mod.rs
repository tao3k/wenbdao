pub(crate) use self::service::{AnalysisError, analyze_markdown, compile_markdown_nodes};

mod markdown;
mod projection;
mod service;

#[cfg(test)]
#[path = "../../../../tests/unit/gateway/studio/analysis.rs"]
mod tests;
