//! Storage routing integration.
//!
//! Provides helpers for multi-backend storage routing.

use ferro_backend_router::{BackendRouter, RoutingPolicy, policy::BackendId};
use std::collections::HashMap;

pub fn create_storage_router() -> BackendRouter {
    BackendRouter::new()
}

pub fn create_router_with_default_policy(name: &str) -> BackendRouter {
    let mut router = BackendRouter::new();
    let policy = RoutingPolicy::new(name, BackendId::Local);
    router.add_policy(policy).unwrap();
    router
}

pub fn route_path(
    router: &BackendRouter,
    path: &str,
    metadata: &HashMap<String, String>,
) -> Result<ferro_backend_router::policy::RoutingDecision, ferro_backend_router::RoutingError> {
    router.route(path, metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_empty_router() {
        let router = create_storage_router();
        assert!(router.list_backends().is_empty());
    }

    #[test]
    fn test_router_with_default_policy() {
        let router = create_router_with_default_policy("default");
        let backends = router.list_backends();
        assert!(backends.contains(&BackendId::Local));
        let decision = route_path(&router, "/files/test.txt", &HashMap::new()).unwrap();
        assert_eq!(decision.policy_name, "default");
    }
}
