use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct SyncPauser {
    paused: AtomicBool,
    pause_reason: RwLock<Option<String>>,
}

impl SyncPauser {
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            pause_reason: RwLock::new(None),
        }
    }

    pub fn pause(&self, reason: &str) {
        *self.pause_reason.write().unwrap() = Some(reason.to_string());
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        *self.pause_reason.write().unwrap() = None;
        self.paused.store(false, Ordering::SeqCst);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn pause_reason(&self) -> Option<String> {
        self.pause_reason.read().unwrap().clone()
    }
}

impl Default for SyncPauser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_not_paused() {
        let pauser = SyncPauser::new();
        assert!(!pauser.is_paused());
        assert!(pauser.pause_reason().is_none());
    }

    #[test]
    fn test_pause_and_resume() {
        let pauser = SyncPauser::new();
        assert!(!pauser.is_paused());

        pauser.pause("maintenance");
        assert!(pauser.is_paused());
        assert_eq!(pauser.pause_reason(), Some("maintenance".to_string()));

        pauser.resume();
        assert!(!pauser.is_paused());
        assert!(pauser.pause_reason().is_none());
    }

    #[test]
    fn test_pause_reason_tracking() {
        let pauser = SyncPauser::new();

        pauser.pause("user requested");
        assert_eq!(pauser.pause_reason(), Some("user requested".to_string()));

        pauser.pause("network issue");
        assert_eq!(pauser.pause_reason(), Some("network issue".to_string()));
    }

    #[test]
    fn test_default_trait() {
        let pauser = SyncPauser::default();
        assert!(!pauser.is_paused());
    }
}
