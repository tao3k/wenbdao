use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};

use tokio::runtime::Handle;

use crate::gateway::studio::search;
use crate::gateway::studio::symbol_index::state::{SymbolIndexCoordinator, timestamp_now};
use crate::gateway::studio::symbol_index::{SymbolIndexPhase, SymbolIndexStatus};
use crate::gateway::studio::types::UiProjectConfig;
use crate::unified_symbol::UnifiedSymbolIndex;

#[allow(clippy::too_many_lines)]
pub(crate) fn maybe_spawn_build(
    coordinator: &Arc<SymbolIndexCoordinator>,
    projects: Vec<UiProjectConfig>,
    index_cache: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    fingerprint: String,
) {
    let _spawn_guard = coordinator
        .spawn_lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let current_fingerprint = coordinator
        .active_fingerprint
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let current_status = coordinator.status();
    let current_index = index_cache
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();

    if current_fingerprint.as_deref() == Some(fingerprint.as_str()) {
        if current_index.is_some() && matches!(current_status.phase, SymbolIndexPhase::Ready) {
            return;
        }
        if matches!(current_status.phase, SymbolIndexPhase::Indexing) {
            return;
        }
    }

    *coordinator
        .active_fingerprint
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(fingerprint.clone());
    coordinator
        .shutdown_requested
        .store(false, Ordering::SeqCst);
    *coordinator
        .status
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
        phase: SymbolIndexPhase::Indexing,
        last_error: None,
        updated_at: Some(timestamp_now()),
    };

    let project_root = coordinator.project_root.clone();
    let config_root = coordinator.config_root.clone();
    let active_fingerprint = Arc::clone(&coordinator.active_fingerprint);
    let status = Arc::clone(&coordinator.status);
    let shutdown_requested = Arc::clone(&coordinator.shutdown_requested);

    if let Ok(handle) = Handle::try_current() {
        let build_task = handle.spawn(async move {
            let build = tokio::task::spawn_blocking(move || {
                search::build_symbol_index(project_root.as_path(), config_root.as_path(), &projects)
            })
            .await;

            let latest_fingerprint = active_fingerprint
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            if shutdown_requested.load(Ordering::SeqCst)
                || latest_fingerprint.as_deref() != Some(fingerprint.as_str())
            {
                return;
            }

            match build {
                Ok(index) => {
                    *index_cache
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(index));
                    *status
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
                        phase: SymbolIndexPhase::Ready,
                        last_error: None,
                        updated_at: Some(timestamp_now()),
                    };
                }
                Err(error) => {
                    *index_cache
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
                    *status
                        .write()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
                        phase: SymbolIndexPhase::Failed,
                        last_error: Some(format!("symbol index background task panicked: {error}")),
                        updated_at: Some(timestamp_now()),
                    };
                }
            }
        });
        *coordinator
            .build_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(build_task);
    } else {
        *coordinator
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
            phase: SymbolIndexPhase::Failed,
            last_error: Some("Tokio runtime unavailable for symbol index build".to_string()),
            updated_at: Some(timestamp_now()),
        };
    }
}
