use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct VectorClock {
    pub site_id: String,
    pub counter: u64,
}

impl VectorClock {
    pub fn new(site_id: &str) -> Self {
        Self {
            site_id: site_id.to_string(),
            counter: 0,
        }
    }

    pub fn increment(&mut self) {
        self.counter += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        if other.counter > self.counter {
            self.counter = other.counter;
        }
    }

    pub fn with_counter(mut self, counter: u64) -> Self {
        self.counter = counter;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_clock_increment() {
        let mut c = VectorClock::new("site1");
        assert_eq!(c.counter, 0);
        c.increment();
        assert_eq!(c.counter, 1);
        c.increment();
        assert_eq!(c.counter, 2);
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut c1 = VectorClock::new("site1");
        let mut c2 = VectorClock::new("site2");
        c1.increment();
        c1.increment();
        c2.increment();
        c2.increment();
        c2.increment();
        c2.increment();
        c2.increment();
        c1.merge(&c2);
        assert_eq!(c1.counter, 5);
    }
}
