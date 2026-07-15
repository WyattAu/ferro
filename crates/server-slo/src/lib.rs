use std::collections::HashMap;
use std::sync::RwLock;

/// Defines an SLO with a target percentage, time window, and metric type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SloDefinition {
    pub name: String,
    pub target: f64,
    pub window: String,
    pub metric: SliMetric,
}

/// The type of SLI metric being tracked.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SliMetric {
    Availability,
    Latency,
    ErrorRate,
}

/// Collects SLI success/failure counts per SLO name.
pub struct SliCollector {
    counts: RwLock<HashMap<String, (u64, u64)>>,
}

impl SliCollector {
    pub fn new() -> Self {
        Self {
            counts: RwLock::new(HashMap::new()),
        }
    }

    pub fn record_success(&self, slo_name: &str) {
        let mut counts = self.counts.write().unwrap();
        let entry = counts.entry(slo_name.to_string()).or_insert((0, 0));
        entry.0 += 1;
    }

    pub fn record_failure(&self, slo_name: &str) {
        let mut counts = self.counts.write().unwrap();
        let entry = counts.entry(slo_name.to_string()).or_insert((0, 0));
        entry.1 += 1;
    }

    pub fn get_availability(&self, slo_name: &str) -> f64 {
        let counts = self.counts.read().unwrap();
        match counts.get(slo_name) {
            Some((success, failure)) => {
                let total = success + failure;
                if total == 0 {
                    1.0
                } else {
                    *success as f64 / total as f64
                }
            }
            None => 1.0,
        }
    }

    pub fn get_error_budget_remaining(&self, slo_name: &str, target: f64) -> f64 {
        let availability = self.get_availability(slo_name);
        let error_budget = 1.0 - target;
        if error_budget <= 0.0 {
            return 0.0;
        }
        let consumed = 1.0 - availability;
        let remaining = error_budget - consumed;
        remaining.max(0.0) / error_budget
    }

    pub fn is_breached(&self, slo_name: &str, target: f64) -> bool {
        self.get_availability(slo_name) < target
    }

    pub fn prometheus_metrics(&self, definitions: &[SloDefinition]) -> String {
        let counts = self.counts.read().unwrap();
        let mut output = String::new();

        for slo in definitions {
            let (success, failure) = counts.get(&slo.name).unwrap_or(&(0, 0));
            let total = success + failure;
            let availability = if total == 0 {
                1.0
            } else {
                *success as f64 / total as f64
            };
            let error_budget_remaining = self.get_error_budget_remaining(&slo.name, slo.target);
            let breached = self.is_breached(&slo.name, slo.target);
            let breached_val = if breached { 1 } else { 0 };

            output.push_str(&format!(
                "# HELP ferro_slo_availability{{slo=\"{}\"}} Current SLO availability\n\
                 # TYPE ferro_slo_availability gauge\n\
                 ferro_slo_availability{{slo=\"{}\"}} {}\n\
                 # HELP ferro_slo_error_budget_remaining{{slo=\"{}\"}} Remaining error budget ratio\n\
                 # TYPE ferro_slo_error_budget_remaining gauge\n\
                 ferro_slo_error_budget_remaining{{slo=\"{}\"}} {}\n\
                 # HELP ferro_slo_breached{{slo=\"{}\"}} Whether SLO is currently breached\n\
                 # TYPE ferro_slo_breached gauge\n\
                 ferro_slo_breached{{slo=\"{}\"}} {}\n",
                slo.name,
                slo.name,
                availability,
                slo.name,
                slo.name,
                error_budget_remaining,
                slo.name,
                slo.name,
                breached_val,
            ));
        }

        output
    }
}

impl Default for SliCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the default SLO definitions for the Ferro server.
pub fn default_slos() -> Vec<SloDefinition> {
    vec![
        SloDefinition {
            name: "api_availability".to_string(),
            target: 0.999,
            window: "30d".to_string(),
            metric: SliMetric::Availability,
        },
        SloDefinition {
            name: "api_latency_p99".to_string(),
            target: 0.500,
            window: "30d".to_string(),
            metric: SliMetric::Latency,
        },
        SloDefinition {
            name: "storage_availability".to_string(),
            target: 0.9999,
            window: "30d".to_string(),
            metric: SliMetric::Availability,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sli_collector_basic() {
        let collector = SliCollector::new();
        collector.record_success("test_slo");
        collector.record_success("test_slo");
        collector.record_failure("test_slo");

        assert!((collector.get_availability("test_slo") - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_error_budget_remaining() {
        let collector = SliCollector::new();
        for _ in 0..9999 {
            collector.record_success("test_slo");
        }
        collector.record_failure("test_slo");

        let remaining = collector.get_error_budget_remaining("test_slo", 0.999);
        assert!(remaining > 0.0);
        assert!(remaining < 1.0);
    }

    #[test]
    fn test_is_breached() {
        let collector = SliCollector::new();
        for _ in 0..999 {
            collector.record_success("test_slo");
        }
        collector.record_failure("test_slo");

        assert!(!collector.is_breached("test_slo", 0.999));
        assert!(collector.is_breached("test_slo", 0.9999));
    }

    #[test]
    fn test_default_slos() {
        let slos = default_slos();
        assert_eq!(slos.len(), 3);
        assert_eq!(slos[0].name, "api_availability");
        assert_eq!(slos[0].target, 0.999);
        assert_eq!(slos[1].name, "api_latency_p99");
        assert_eq!(slos[2].name, "storage_availability");
        assert_eq!(slos[2].target, 0.9999);
    }

    #[test]
    fn test_prometheus_metrics_output() {
        let collector = SliCollector::new();
        let slos = default_slos();
        collector.record_success("api_availability");
        let output = collector.prometheus_metrics(&slos);
        assert!(output.contains("ferro_slo_availability"));
        assert!(output.contains("ferro_slo_error_budget_remaining"));
        assert!(output.contains("ferro_slo_breached"));
    }
}
