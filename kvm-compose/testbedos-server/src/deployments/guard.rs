use crate::AppState;

/// This is a guard to assign to deployments, to prevent running any sort of read or write to the
/// deployment while this lock is in place. This lock is essentially going to be used when there
/// is a long-running websocket process provisioning the guests and network. In other words, any
/// sort of action on a deployment that is 'destructive'.
///
/// The idea here is to maintain this in memory so that this state is lost if the server crashes or
/// is turned off before any locks can be removed gracefully. Also, if the owner of the lock is
/// uninitialised, then as we implement `Drop` for the guard, it will be removed.
pub struct DeploymentGuard {
    state: AppState,
    deployment_name: String,
}

impl Drop for DeploymentGuard {
    fn drop(&mut self) {
        let mut active = self.state.active_deployments
            .lock()
            // for now, we allow getting the lock even if another thread has panicked when holding
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        active.remove(&self.deployment_name);
        // mutex is dropped automatically and the deployment name is removed from the hashset
        // so everything is cleaned up, and we can start this again for the same deployment in a
        // clean state
    }
}

impl AppState {
    pub fn try_lock_deployment(&self, id: String) -> Option<DeploymentGuard> {
        let mut active = self.active_deployments
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if active.insert(id.clone()) {
            Some(DeploymentGuard {
                state: self.clone(),
                deployment_name: id,
            })
        } else {
            None
        }
    }

    pub fn is_deployment_locked(&self, id: &str) -> bool {
        self.active_deployments
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(id)
    }
}
