use crate::link_graph::models::LinkGraphRetrievalMode;
use crate::link_graph::runtime_config::constants::DEFAULT_LINK_GRAPH_RETRIEVAL_MODE;
use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;
use xiuxian_wendao_runtime::runtime_config::LinkGraphRetrievalBaseRuntimeConfig;

use super::julia_rerank::LinkGraphJuliaRerankRuntimeConfig;
use super::semantic_ignition::LinkGraphSemanticIgnitionRuntimeConfig;

pub struct LinkGraphRetrievalPolicyRuntimeConfig {
    pub mode: LinkGraphRetrievalMode,
    pub candidate_multiplier: usize,
    pub max_sources: usize,
    pub hybrid_min_hits: usize,
    pub hybrid_min_top_score: f64,
    pub graph_rows_per_source: usize,
    pub semantic_ignition: LinkGraphSemanticIgnitionRuntimeConfig,
    pub julia_rerank: LinkGraphJuliaRerankRuntimeConfig,
}

impl Default for LinkGraphRetrievalPolicyRuntimeConfig {
    fn default() -> Self {
        Self::from(LinkGraphRetrievalBaseRuntimeConfig::default())
    }
}

impl LinkGraphRetrievalPolicyRuntimeConfig {
    /// Resolve the current rerank provider binding through the generic plugin-runtime model.
    #[must_use]
    pub fn rerank_binding(&self) -> Option<PluginCapabilityBinding> {
        self.julia_rerank.rerank_provider_binding()
    }
}

impl From<LinkGraphRetrievalBaseRuntimeConfig> for LinkGraphRetrievalPolicyRuntimeConfig {
    fn from(base: LinkGraphRetrievalBaseRuntimeConfig) -> Self {
        Self {
            mode: LinkGraphRetrievalMode::from_alias(DEFAULT_LINK_GRAPH_RETRIEVAL_MODE)
                .unwrap_or_default(),
            candidate_multiplier: base.candidate_multiplier,
            max_sources: base.max_sources,
            hybrid_min_hits: base.hybrid_min_hits,
            hybrid_min_top_score: base.hybrid_min_top_score,
            graph_rows_per_source: base.graph_rows_per_source,
            semantic_ignition: base.semantic_ignition,
            julia_rerank: LinkGraphJuliaRerankRuntimeConfig::default(),
        }
    }
}
