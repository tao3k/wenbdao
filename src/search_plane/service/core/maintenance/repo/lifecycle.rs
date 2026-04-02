use crate::search_plane::service::core::maintenance::helpers::REPO_MAINTENANCE_SHUTDOWN_MESSAGE;
use crate::search_plane::service::core::types::SearchPlaneService;

impl SearchPlaneService {
    pub(crate) fn stop_repo_maintenance(&self) {
        let (worker_handle, waiters) = {
            let mut runtime = self
                .repo_maintenance
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            runtime.shutdown_requested = true;
            runtime.worker_running = false;
            runtime.active_task = None;
            runtime.queue.clear();
            runtime.in_flight.clear();
            let worker_handle = runtime.worker_handle.take();
            let waiters = std::mem::take(&mut runtime.waiters);
            (worker_handle, waiters)
        };
        if let Some(worker_handle) = worker_handle {
            worker_handle.abort();
        }
        for waiters in waiters.into_values() {
            for waiter in waiters {
                let _ = waiter.send(Err(REPO_MAINTENANCE_SHUTDOWN_MESSAGE.to_string()));
            }
        }
    }

    pub(crate) fn clear_repo_maintenance_for_repo(&self, repo_id: &str) {
        let mut runtime = self
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cleared_keys = runtime
            .in_flight
            .iter()
            .filter(|(_, candidate_repo_id, _, _)| candidate_repo_id == repo_id)
            .cloned()
            .collect::<Vec<_>>();
        runtime
            .in_flight
            .retain(|(_, candidate_repo_id, _, _)| candidate_repo_id != repo_id);
        runtime
            .queue
            .retain(|queued| queued.task.repo_id() != repo_id);
        let drained_waiter_keys = runtime
            .waiters
            .keys()
            .filter(|(_, candidate_repo_id, _, _)| candidate_repo_id == repo_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut drained_waiters = drained_waiter_keys
            .into_iter()
            .filter_map(|task_key| {
                runtime
                    .waiters
                    .remove(&task_key)
                    .map(|waiters| (task_key, waiters))
            })
            .collect::<Vec<_>>();
        drop(runtime);
        for task_key in cleared_keys {
            if let Some((_, waiters)) = drained_waiters
                .iter_mut()
                .find(|(candidate_task_key, _)| candidate_task_key == &task_key)
            {
                for waiter in waiters.drain(..) {
                    let _ = waiter.send(Err(format!(
                        "repo maintenance task for {repo_id} was cleared before completion"
                    )));
                }
            }
        }
    }
}
