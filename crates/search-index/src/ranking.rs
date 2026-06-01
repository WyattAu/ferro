use std::collections::HashMap;

pub struct Ranker {
    total_docs: usize,
    field_boosts: HashMap<String, f64>,
    fields: Vec<String>,
}

impl Ranker {
    pub fn new(total_docs: usize, field_boosts: HashMap<String, f64>, fields: Vec<String>) -> Self {
        Self {
            total_docs,
            field_boosts,
            fields,
        }
    }

    pub fn tf_idf(&self, term_freq: f64, doc_freq: usize, _term_len: usize) -> f64 {
        if self.total_docs == 0 || doc_freq == 0 {
            return 0.0;
        }
        let tf = term_freq;
        let idf = (1.0 + (self.total_docs as f64) / (doc_freq as f64)).ln();
        tf * idf
    }

    pub fn field_boost(&self, field: &str) -> f64 {
        *self.field_boosts.get(field).unwrap_or(&1.0)
    }

    #[allow(dead_code)]
    pub fn normalize_field_boosts(&self) -> HashMap<String, f64> {
        let sum: f64 = self
            .fields
            .iter()
            .map(|f| *self.field_boosts.get(f).unwrap_or(&1.0))
            .sum();
        if sum == 0.0 {
            return self.field_boosts.clone();
        }
        self.field_boosts
            .iter()
            .map(|(k, v)| (k.clone(), v / sum))
            .collect()
    }
}
