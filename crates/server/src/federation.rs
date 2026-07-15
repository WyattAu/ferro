pub use ferro_server_activitypub::FederationState;
pub use ferro_server_activitypub::store::ActivityStore;
pub use ferro_server_activitypub::*;

use std::sync::Arc;

impl ferro_server_federation::FederationStateProvider for crate::AppState {
    fn activity_store(&self) -> &Arc<ferro_server_activitypub::store::ActivityStore> {
        &self.activity_store
    }
    fn external_url(&self) -> &str {
        &self.external_url
    }
    fn federation_secret(&self) -> &str {
        &self.federation_secret
    }
}

pub use ferro_server_federation::{
    federated_share, get_actor, inbox, list_followers, list_following, list_inbox, list_outbox, nodeinfo, webfinger,
};
