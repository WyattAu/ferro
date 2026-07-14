use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A string wrapper that zeroizes its contents on drop.
/// Use for passwords, tokens, API keys, and other sensitive data.
#[derive(Clone, Debug, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct ZeroizeString(String);

impl ZeroizeString {
    /// Create a new `ZeroizeString` wrapping the given string.
    ///
    /// # Panics
    ///
    /// This function never panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use ferro_common::zeroize::ZeroizeString;
    ///
    /// let s = ZeroizeString::new("secret".to_string());
    /// assert_eq!(s.as_str(), "secret");
    /// ```
    pub fn new(s: String) -> Self {
        Self(s)
    }

    /// Return a reference to the inner string.
    ///
    /// # Panics
    ///
    /// This function never panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use ferro_common::zeroize::ZeroizeString;
    ///
    /// let s = ZeroizeString::new("secret".to_string());
    /// assert_eq!(s.as_str(), "secret");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume self and return the inner string.
    ///
    /// The original `ZeroizeString` is dropped and its memory zeroed.
    /// The returned clone is not zeroed on drop.
    ///
    /// # Panics
    ///
    /// This function never panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use ferro_common::zeroize::ZeroizeString;
    ///
    /// let s = ZeroizeString::new("secret".to_string());
    /// let inner = s.into_inner();
    /// assert_eq!(inner, "secret");
    /// ```
    pub fn into_inner(self) -> String {
        // Clone the inner value; the original will be zeroized on drop.
        self.0.clone()
    }
}

impl std::fmt::Display for ZeroizeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl std::ops::Deref for ZeroizeString {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ZeroizeString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ZeroizeString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for ZeroizeString {
    fn from(s: &str) -> Self {
        Self::new(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeroize_string_new() {
        let s = ZeroizeString::new("secret".to_string());
        assert_eq!(s.as_str(), "secret");
    }

    #[test]
    fn test_zeroize_string_into_inner() {
        let s = ZeroizeString::new("secret".to_string());
        let inner = s.into_inner();
        assert_eq!(inner, "secret");
    }

    #[test]
    fn test_zeroize_string_display() {
        let s = ZeroizeString::new("secret".to_string());
        assert_eq!(format!("{}", s), "[REDACTED]");
    }

    #[test]
    fn test_zeroize_string_deref() {
        let s = ZeroizeString::new("secret".to_string());
        let r: &str = &s;
        assert_eq!(r, "secret");
    }

    #[test]
    fn test_zeroize_string_as_ref() {
        let s = ZeroizeString::new("secret".to_string());
        let r: &str = s.as_ref();
        assert_eq!(r, "secret");
    }

    #[test]
    fn test_zeroize_string_from_string() {
        let s: ZeroizeString = "secret".to_string().into();
        assert_eq!(s.as_str(), "secret");
    }

    #[test]
    fn test_zeroize_string_from_str() {
        let s: ZeroizeString = "secret".into();
        assert_eq!(s.as_str(), "secret");
    }

    #[test]
    fn test_zeroize_string_clone() {
        let s1 = ZeroizeString::new("secret".to_string());
        let s2 = s1.clone();
        assert_eq!(s1.as_str(), s2.as_str());
    }

    #[test]
    fn test_zeroize_string_debug() {
        let s = ZeroizeString::new("secret".to_string());
        let debug = format!("{:?}", s);
        assert!(debug.contains("ZeroizeString"));
    }

    #[test]
    fn test_zeroize_string_serialize_deserialize() {
        let s = ZeroizeString::new("secret".to_string());
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: ZeroizeString = serde_json::from_str(&json).unwrap();
        assert_eq!(s.as_str(), deserialized.as_str());
    }
}
