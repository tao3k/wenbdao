use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::Serialize;

use crate::analyzers::registry::PluginRegistry;
use crate::gateway::studio::repo_index::RepoIndexCoordinator;
use crate::gateway::studio::symbol_index::{SymbolIndexCoordinator, timestamp_now};
use crate::gateway::studio::types::UiConfig;
use crate::link_graph::LinkGraphIndex;
use crate::search_plane::SearchPlaneService;
use crate::unified_symbol::UnifiedSymbolIndex;

use crate::gateway::studio::types::VfsScanResult;
use xiuxian_zhenfa::ZhenfaSignal;

#[derive(Clone)]
pub(crate) struct DeferredBootstrapBackgroundIndexingActivation {
    pub(crate) activated_at: String,
    pub(crate) source: String,
}

/// Shared bootstrap-indexing telemetry derived from the Studio runtime state.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudioBootstrapBackgroundIndexingTelemetry {
    studio_bootstrap_background_indexing_enabled: bool,
    studio_bootstrap_background_indexing_mode: &'static str,
    studio_bootstrap_background_indexing_deferred_activation_observed: bool,
    studio_bootstrap_background_indexing_deferred_activation_at: Option<String>,
    studio_bootstrap_background_indexing_deferred_activation_source: Option<String>,
}

impl StudioBootstrapBackgroundIndexingTelemetry {
    /// Returns whether bootstrap-time background indexing is enabled.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.studio_bootstrap_background_indexing_enabled
    }

    /// Returns the stable bootstrap-time background-indexing mode label.
    #[must_use]
    pub fn mode(&self) -> &'static str {
        self.studio_bootstrap_background_indexing_mode
    }

    /// Returns whether deferred bootstrap indexing has been lazily activated since boot.
    #[must_use]
    pub fn deferred_activation_observed(&self) -> bool {
        self.studio_bootstrap_background_indexing_deferred_activation_observed
    }

    /// Returns the first deferred bootstrap-indexing activation timestamp, if any.
    #[must_use]
    pub fn deferred_activation_at(&self) -> Option<&str> {
        self.studio_bootstrap_background_indexing_deferred_activation_at
            .as_deref()
    }

    /// Returns the source that first activated deferred bootstrap indexing, if any.
    #[must_use]
    pub fn deferred_activation_source(&self) -> Option<&str> {
        self.studio_bootstrap_background_indexing_deferred_activation_source
            .as_deref()
    }
}

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: PathBuf,
    pub(crate) config_root: PathBuf,
    pub(crate) bootstrap_background_indexing: bool,
    pub(crate) bootstrap_background_indexing_deferred_activation:
        Arc<RwLock<Option<DeferredBootstrapBackgroundIndexingActivation>>>,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: Arc<RwLock<Option<Arc<LinkGraphIndex>>>>,
    pub(crate) symbol_index: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    pub(crate) symbol_index_coordinator: Arc<SymbolIndexCoordinator>,
    pub(crate) search_plane: SearchPlaneService,
    pub(crate) vfs_scan: Arc<RwLock<Option<VfsScanResult>>>,
    pub(crate) repo_index: Arc<RepoIndexCoordinator>,
    /// Registry of repository intelligence plugins.
    pub(crate) plugin_registry: Arc<PluginRegistry>,
}

impl StudioState {
    /// Returns one clone of the shared search-plane service owned by the Studio runtime.
    #[must_use]
    pub fn search_plane_service(&self) -> SearchPlaneService {
        self.search_plane.clone()
    }

    /// Returns whether bootstrap-time background indexing is enabled for this state instance.
    #[must_use]
    pub fn bootstrap_background_indexing_enabled(&self) -> bool {
        self.bootstrap_background_indexing
    }

    /// Returns the stable mode label for bootstrap-time background indexing.
    #[must_use]
    pub fn bootstrap_background_indexing_mode(&self) -> &'static str {
        if self.bootstrap_background_indexing_enabled() {
            "enabled"
        } else {
            "deferred"
        }
    }

    /// Returns the current bootstrap-indexing telemetry snapshot.
    #[must_use]
    pub fn bootstrap_background_indexing_telemetry(
        &self,
    ) -> StudioBootstrapBackgroundIndexingTelemetry {
        let deferred_activation_at = self.bootstrap_background_indexing_deferred_activation_at();
        let deferred_activation_source =
            self.bootstrap_background_indexing_deferred_activation_source();
        StudioBootstrapBackgroundIndexingTelemetry {
            studio_bootstrap_background_indexing_enabled: self
                .bootstrap_background_indexing_enabled(),
            studio_bootstrap_background_indexing_mode: self.bootstrap_background_indexing_mode(),
            studio_bootstrap_background_indexing_deferred_activation_observed:
                deferred_activation_at.is_some(),
            studio_bootstrap_background_indexing_deferred_activation_at: deferred_activation_at,
            studio_bootstrap_background_indexing_deferred_activation_source:
                deferred_activation_source,
        }
    }

    /// Returns the first deferred bootstrap-indexing activation timestamp, if any.
    #[must_use]
    pub fn bootstrap_background_indexing_deferred_activation_at(&self) -> Option<String> {
        self.bootstrap_background_indexing_deferred_activation
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|activation| activation.activated_at.clone())
    }

    /// Returns the source that first activated deferred bootstrap indexing, if any.
    #[must_use]
    pub fn bootstrap_background_indexing_deferred_activation_source(&self) -> Option<String> {
        self.bootstrap_background_indexing_deferred_activation
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|activation| activation.source.clone())
    }

    pub(crate) fn record_deferred_bootstrap_background_indexing_activation(
        &self,
        source: &'static str,
    ) {
        if self.bootstrap_background_indexing_enabled() {
            return;
        }

        let mut guard = self
            .bootstrap_background_indexing_deferred_activation
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.is_some() {
            return;
        }

        *guard = Some(DeferredBootstrapBackgroundIndexingActivation {
            activated_at: timestamp_now(),
            source: source.to_string(),
        });
    }
}

/// Shared state used by the top-level gateway process.
#[derive(Clone)]
pub struct GatewayState {
    /// Optional graph index for CLI-powered stats endpoint.
    pub index: Option<Arc<LinkGraphIndex>>,
    /// Signal sender for notification worker.
    pub signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
    /// Studio-specific state for VFS/graph/search APIs.
    pub studio: Arc<StudioState>,
}
