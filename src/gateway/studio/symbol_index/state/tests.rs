use std::sync::{Arc, RwLock};

use crate::gateway::studio::symbol_index::state::{SymbolIndexCoordinator, fingerprint_projects};
use crate::gateway::studio::symbol_index::{SymbolIndexPhase, SymbolIndexStatus};
use crate::gateway::studio::types::UiProjectConfig;
use crate::unified_symbol::UnifiedSymbolIndex;

#[cfg(test)]
impl SymbolIndexCoordinator {
    pub(crate) fn set_status_for_test(
        &self,
        projects: &[UiProjectConfig],
        status: SymbolIndexStatus,
    ) {
        *self
            .active_fingerprint
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(fingerprint_projects(projects));
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = status;
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn set_ready_index_for_test(
        &self,
        projects: &[UiProjectConfig],
        index_cache: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
        index: UnifiedSymbolIndex,
    ) {
        *self
            .active_fingerprint
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(fingerprint_projects(projects));
        *index_cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Arc::new(index));
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SymbolIndexStatus {
            phase: SymbolIndexPhase::Ready,
            last_error: None,
            updated_at: Some(crate::gateway::studio::symbol_index::state::timestamp_now()),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_projects_resets_to_idle_when_projects_are_empty() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let coordinator = Arc::new(SymbolIndexCoordinator::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        ));
        let index_cache = Arc::new(RwLock::new(Some(Arc::new(UnifiedSymbolIndex::new()))));

        coordinator.sync_projects(Vec::new(), Arc::clone(&index_cache));

        assert!(
            index_cache
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_none()
        );
        assert_eq!(coordinator.status().phase, SymbolIndexPhase::Idle);
    }

    #[tokio::test]
    async fn ensure_started_marks_non_idle_for_configured_projects() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        std::fs::create_dir_all(temp.path().join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(
            temp.path().join("src").join("lib.rs"),
            "pub struct BackgroundSymbolIndex;\n",
        )
        .unwrap_or_else(|error| panic!("write source: {error}"));
        let coordinator = Arc::new(SymbolIndexCoordinator::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        ));
        let index_cache = Arc::new(RwLock::new(None));

        coordinator.ensure_started(
            vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["src".to_string()],
            }],
            Arc::clone(&index_cache),
        );

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        assert!(matches!(
            coordinator.status().phase,
            SymbolIndexPhase::Indexing | SymbolIndexPhase::Ready
        ));
    }

    #[tokio::test]
    async fn stop_resets_status_to_idle_after_starting_build() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        std::fs::create_dir_all(temp.path().join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(
            temp.path().join("src").join("lib.rs"),
            "pub struct BackgroundSymbolIndex;\n",
        )
        .unwrap_or_else(|error| panic!("write source: {error}"));
        let coordinator = Arc::new(SymbolIndexCoordinator::new(
            temp.path().to_path_buf(),
            temp.path().to_path_buf(),
        ));
        let index_cache = Arc::new(RwLock::new(None));

        coordinator.ensure_started(
            vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["src".to_string()],
            }],
            Arc::clone(&index_cache),
        );
        coordinator.stop();
        tokio::task::yield_now().await;

        assert_eq!(coordinator.status().phase, SymbolIndexPhase::Idle);
    }
}
