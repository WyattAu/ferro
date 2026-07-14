use std::fmt::Write as FmtWrite;

use crate::registry::{MetricEntry, MetricsRegistry};

pub fn export_prometheus(registry: &MetricsRegistry) -> String {
    let mut output = String::new();
    let entries = registry.entries();

    for (name, help, entry) in entries.iter() {
        let _ = writeln!(output, "# HELP {} {}", name, help);

        match entry {
            MetricEntry::Counter(c) => {
                let _ = writeln!(output, "# TYPE {} counter", name);
                let _ = writeln!(output, "{}_total {}", name, c.get());
            }
            MetricEntry::Gauge(g) => {
                let _ = writeln!(output, "# TYPE {} gauge", name);
                let _ = writeln!(output, "{} {}", name, g.get());
            }
            MetricEntry::Histogram(h) => {
                let _ = writeln!(output, "# TYPE {} histogram", name);
                for (i, bucket) in h.buckets().iter().enumerate() {
                    let _ = writeln!(output, "{}_bucket{{le=\"{}\"}} {}", name, bucket, h.bucket_count(i));
                }
                let _ = writeln!(output, "{}_bucket{{le=\"+Inf\"}} {}", name, h.total_count());
                let _ = writeln!(output, "{}_sum {}", name, h.total_sum());
                let _ = writeln!(output, "{}_count {}", name, h.total_count());
            }
        }
    }

    output
}
