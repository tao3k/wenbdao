//! Agentic suggested-link schema and runtime planning/execution contracts.

mod config;
mod execution;
mod plan;
mod schema;
mod suggested_link;

pub use config::{LinkGraphAgenticExecutionConfig, LinkGraphAgenticExpansionConfig};
pub use execution::{
    LinkGraphAgenticExecutionResult, LinkGraphAgenticWorkerExecution, LinkGraphAgenticWorkerPhase,
};
pub use plan::{
    LinkGraphAgenticCandidatePair, LinkGraphAgenticExpansionPlan, LinkGraphAgenticWorkerPlan,
};
pub use schema::{
    LINK_GRAPH_SUGGESTED_LINK_DECISION_SCHEMA_VERSION, LINK_GRAPH_SUGGESTED_LINK_SCHEMA_VERSION,
    LinkGraphSuggestedLinkState,
};
pub use suggested_link::{
    LinkGraphSuggestedLink, LinkGraphSuggestedLinkDecision, LinkGraphSuggestedLinkDecisionRequest,
    LinkGraphSuggestedLinkDecisionResult, LinkGraphSuggestedLinkRequest,
};
