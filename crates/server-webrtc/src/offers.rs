use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalingOffer {
    pub session_id: String,
    pub sdp: String,
    pub ice_candidates: Vec<String>,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
}

pub struct OfferStore {
    offers: Arc<DashMap<String, SignalingOffer>>,
    max_age: std::time::Duration,
}

impl OfferStore {
    pub fn new() -> Self {
        Self {
            offers: Arc::new(DashMap::new()),
            max_age: std::time::Duration::from_secs(300),
        }
    }

    pub fn create(&self, offer: SignalingOffer) {
        self.offers.insert(offer.session_id.clone(), offer);
    }

    pub fn get(&self, session_id: &str) -> Option<SignalingOffer> {
        self.offers.get(session_id).and_then(|o| {
            if o.created_at.elapsed() < self.max_age {
                Some(o.value().clone())
            } else {
                None
            }
        })
    }

    pub fn add_ice_candidate(&self, session_id: &str, candidate: String) -> bool {
        if let Some(mut offer) = self.offers.get_mut(session_id) {
            offer.ice_candidates.push(candidate);
            true
        } else {
            false
        }
    }

    pub fn remove(&self, session_id: &str) {
        self.offers.remove(session_id);
    }
}

impl Default for OfferStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offer_lifecycle() {
        let store = OfferStore::new();
        let offer = SignalingOffer {
            session_id: "test-session".to_string(),
            sdp: "test-sdp".to_string(),
            ice_candidates: vec![],
            created_at: std::time::Instant::now(),
            file_path: "/test.txt".to_string(),
            file_name: "test.txt".to_string(),
            file_size: 100,
        };
        store.create(offer);

        let retrieved = store.get("test-session").unwrap();
        assert_eq!(retrieved.sdp, "test-sdp");

        store.add_ice_candidate("test-session", "candidate-1".to_string());
        let updated = store.get("test-session").unwrap();
        assert_eq!(updated.ice_candidates.len(), 1);

        store.remove("test-session");
        assert!(store.get("test-session").is_none());
    }

    #[test]
    fn test_offer_expired() {
        let store = OfferStore::new();
        let offer = SignalingOffer {
            session_id: "expired".to_string(),
            sdp: "sdp".to_string(),
            ice_candidates: vec![],
            created_at: std::time::Instant::now() - std::time::Duration::from_secs(301),
            file_path: "/f.txt".to_string(),
            file_name: "f.txt".to_string(),
            file_size: 0,
        };
        store.create(offer);
        assert!(store.get("expired").is_none());
    }
}
