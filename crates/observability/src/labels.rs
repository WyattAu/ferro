#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Labels(Vec<(String, String)>);

impl Labels {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with(pairs: Vec<(&str, &str)>) -> Self {
        Self(pairs.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    pub fn push(&mut self, key: &str, value: &str) {
        self.0.push((key.to_string(), value.to_string()));
    }

    pub fn as_str(&self) -> String {
        self.0
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::new()
    }
}
