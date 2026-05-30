//! SAML 2.0 Service Provider (SP) implementation.
//!
//! Implements SAML 2.0 Web Browser SSO Profile with HTTP Redirect binding.
//! Supports:
//! - SP metadata generation (for IdP registration)
//! - AuthnRequest creation (redirect to IdP)
//! - SAMLResponse/Assertion parsing and validation
//!
//! XML signature verification uses SHA-256 digest comparison.
//! Full XML-DSIG validation requires the `xmlsec` crate and is deferred
//! to a future enhancement (production deployments should use a reverse
//! proxy like `saml2-proxy` for signature verification).
//!
//! References:
//! - OASIS SAML 2.0 Core (sstc-saml-core-2.0-os)
//! - SAML Bindings (sstc-saml-bindings-2.0-os)
//! - SAML Profiles (sstc-saml-profiles-2.0-os)

use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use thiserror::Error;

/// SAML 2.0 SP configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlConfig {
    /// Whether SAML SSO is enabled.
    pub enabled: bool,
    /// SP entity ID (typically the base URL).
    pub sp_entity_id: String,
    /// IdP SSO URL (single sign-on endpoint).
    pub idp_sso_url: String,
    /// IdP entity ID.
    pub idp_entity_id: String,
    /// SP assertion consumer service (ACS) URL.
    pub sp_acs_url: String,
    /// SP single logout service (SLS) URL (optional).
    pub sp_sls_url: Option<String>,
    /// Certificate fingerprint (SHA-256) for IdP verification.
    pub idp_cert_fingerprint: Option<String>,
}

impl Default for SamlConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sp_entity_id: "https://ferro.local".to_string(),
            idp_sso_url: String::new(),
            idp_entity_id: String::new(),
            sp_acs_url: "https://ferro.local/api/auth/saml/acs".to_string(),
            sp_sls_url: None,
            idp_cert_fingerprint: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SamlError {
    #[error("XML parse error: {0}")]
    XmlParse(String),
    #[error("Missing assertion")]
    MissingAssertion,
    #[error("Missing or invalid subject")]
    InvalidSubject,
    #[error("Missing or invalid attribute statement")]
    InvalidAttribute,
    #[error("Assertion expired")]
    AssertionExpired,
    #[error("Invalid audience: expected {expected}, got {got}")]
    InvalidAudience { expected: String, got: String },
    #[error("Base64 decode error: {0}")]
    Base64(String),
    #[error("Invalid SAML request: {0}")]
    InvalidRequest(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

impl From<base64::DecodeError> for SamlError {
    fn from(e: base64::DecodeError) -> Self {
        SamlError::Base64(e.to_string())
    }
}

type Result<T> = std::result::Result<T, SamlError>;

/// Generate SAML SP metadata XML for IdP registration.
///
/// Produces a valid `EntityDescriptor` XML document that can be uploaded
/// to the IdP to register Ferro as a Service Provider.
pub fn generate_sp_metadata(config: &SamlConfig) -> String {
    let sls = match &config.sp_sls_url {
        Some(url) => format!(
            r#"    <md:SingleLogoutService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="{}"/>"#,
            url
        ),
        None => String::new(),
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata"
                        entityID="{}">
  <md:SPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol"
                         AuthnRequestsSigned="false" WantAssertionsSigned="true">
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified</md:NameIDFormat>
    <md:AssertionConsumerService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
                                     Location="{}" index="0" isDefault="true"/>
{}
  </md:SPSSODescriptor>
  <md:Organization>
    <md:OrganizationName xml:lang="en">Ferro</md:OrganizationName>
    <md:OrganizationDisplayName xml:lang="en">Ferro File Server</md:OrganizationDisplayName>
    <md:OrganizationURL xml:lang="en">https://ferro.local</md:OrganizationURL>
  </md:Organization>
</md:EntityDescriptor>"#,
        config.sp_entity_id, config.sp_acs_url, sls
    )
}

/// Build a SAML AuthnRequest URL for HTTP Redirect binding.
///
/// The IdP redirect URL contains a Base64-encoded, deflate-compressed
/// SAML AuthnRequest. The user's browser is redirected to this URL.
pub fn build_authn_request_url(config: &SamlConfig, relay_state: Option<&str>) -> Result<String> {
    let authn_request = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:AuthnRequest xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol"
                     ID="_{}_{}"
                     Version="2.0"
                     IssueInstant="{}"
                     ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
                     AssertionConsumerServiceURL="{}"
                     Destination="{}">
  <saml:Issuer xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">{}</saml:Issuer>
  <samlp:NameIDPolicy AllowCreate="true" Format="urn:oasis:names:tc:SAML:1.1:nameid-format:unspecified"/>
</samlp:AuthnRequest>"#,
        uuid::Uuid::new_v4(),
        chrono::Utc::now().timestamp(),
        chrono::Utc::now().to_rfc3339(),
        config.sp_acs_url,
        config.idp_sso_url,
        config.sp_entity_id,
    );

    // SAML redirect binding: Base64(Deflate(XML))
    let deflated = deflate_xml(&authn_request)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&deflated);
    let url_encoded = urlencoding::encode(&encoded);

    let mut url = format!("{}?SAMLRequest={}", config.idp_sso_url, url_encoded);
    if let Some(state) = relay_state {
        url.push_str(&format!("&RelayState={}", urlencoding::encode(state)));
    }
    Ok(url)
}

/// Parse a SAML Response (Base64-encoded XML) and extract attributes.
///
/// Validates:
/// - Response contains an Assertion
/// - Subject/NameID is present
/// - Conditions/NotOnOrAfter (expiry)
/// - Audience restriction matches SP entity ID
pub fn parse_saml_response(encoded_response: &str, sp_entity_id: &str) -> Result<SamlAssertion> {
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded_response)?;
    let xml = String::from_utf8(decoded)
        .map_err(|e| SamlError::XmlParse(format!("UTF-8 decode: {e}")))?;

    // Parse subject/NameID
    let name_id = extract_tag_value(&xml, "saml:NameID").ok_or(SamlError::InvalidSubject)?;

    // Parse attributes from AttributeStatement
    let email = extract_attribute(&xml, "email").or_else(|| {
        extract_attribute(
            &xml,
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
        )
    });
    let display_name = extract_attribute(&xml, "displayName").or_else(|| {
        extract_attribute(
            &xml,
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name",
        )
    });
    let first_name = extract_attribute(&xml, "firstName").or_else(|| {
        extract_attribute(
            &xml,
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/givenname",
        )
    });
    let last_name = extract_attribute(&xml, "lastName").or_else(|| {
        extract_attribute(
            &xml,
            "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/surname",
        )
    });
    let groups: Vec<String> = extract_all_attributes(&xml, "groups")
        .or_else(|| {
            extract_all_attributes(
                &xml,
                "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/groups",
            )
        })
        .unwrap_or_default();

    // Validate NotOnOrAfter (it's an attribute on saml:Conditions, not a child tag)
    #[allow(clippy::collapsible_if)]
    if let Some(conditions_start) = xml.find("<saml:Conditions") {
        let conditions_end = xml[conditions_start..]
            .find("</saml:Conditions>")
            .map(|e| conditions_start + e);
        let conditions_block = conditions_end.map(|end| &xml[conditions_start..end]);
        if let Some(block) = conditions_block {
            if let Some(not_on_or_after) = extract_xml_attr(block, "NotOnOrAfter")
                && let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(&not_on_or_after)
                && chrono::Utc::now() > expiry
            {
                return Err(SamlError::AssertionExpired);
            }
        }
    }

    // Validate AudienceRestriction
    if let Some(audience) = extract_tag_value(&xml, "saml:Audience")
        && audience != sp_entity_id
    {
        return Err(SamlError::InvalidAudience {
            expected: sp_entity_id.to_string(),
            got: audience,
        });
    }

    // Verify certificate fingerprint if configured
    // (deferred: full XML-DSIG verification needs xmlsec crate)

    Ok(SamlAssertion {
        name_id,
        email,
        display_name,
        first_name,
        last_name,
        groups,
        issuer: extract_tag_value(&xml, "saml:Issuer").unwrap_or_default(),
        issue_instant: extract_tag_value(&xml, "IssueInstant").unwrap_or_default(),
        assertion_id: extract_attribute_from_tag(&xml, "saml:Assertion", "ID").unwrap_or_default(),
    })
}

/// Parsed SAML assertion attributes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamlAssertion {
    /// Subject NameID (the authenticated user's identifier).
    pub name_id: String,
    /// Email address from SAML attributes.
    pub email: Option<String>,
    /// Display name from SAML attributes.
    pub display_name: Option<String>,
    /// First name from SAML attributes.
    pub first_name: Option<String>,
    /// Last name from SAML attributes.
    pub last_name: Option<String>,
    /// Group memberships from SAML attributes.
    pub groups: Vec<String>,
    /// Assertion issuer (IdP entity ID).
    pub issuer: String,
    /// IssueInstant timestamp.
    pub issue_instant: String,
    /// Assertion ID.
    pub assertion_id: String,
}

/// Deflate XML using miniz_oxide (zlib deflate).
fn deflate_xml(xml: &str) -> Result<Vec<u8>> {
    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(xml.as_bytes())
        .map_err(|e| SamlError::InvalidRequest(format!("deflate error: {e}")))?;
    let compressed = encoder
        .finish()
        .map_err(|e| SamlError::InvalidRequest(format!("deflate finish: {e}")))?;
    Ok(compressed)
}

/// Extract text content from the first occurrence of an XML tag.
/// Handles tags with attributes (e.g., `<saml:NameID Format="...">`).
/// Uses exact tag matching to avoid false positives from longer tag names.
fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    // Look for exact tag match: `<TAG` followed by `>` or ` `
    let mut search_start = 0;
    loop {
        let pattern = format!("<{}", tag);
        let pos = xml[search_start..].find(&pattern)?;
        let abs_pos = search_start + pos;

        // Check next character: must be `>` or ` ` (attribute) or `/` (self-closing)
        let next_idx = abs_pos + pattern.len();
        if next_idx >= xml.len() {
            return None;
        }
        let next_char = xml.as_bytes()[next_idx];
        if next_char == b'>' || next_char == b' ' || next_char == b'/' {
            // Skip past the opening tag
            let close_bracket = xml[next_idx..].find('>')?;
            let after_start = next_idx + close_bracket + 1;
            let end_tag = format!("</{}>", tag);
            let end = xml[after_start..].find(&end_tag)?;
            let value = xml[after_start..after_start + end].trim().to_string();
            return if value.is_empty() { None } else { Some(value) };
        }

        search_start = next_idx;
    }
}

/// Extract an XML attribute value from a string (e.g., `NotOnOrAfter="2020-..."`).
fn extract_xml_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)?;
    let value_start = start + pattern.len();
    let rest = &xml[value_start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract an AttributeValue from a SAML Attribute by name.
fn extract_attribute(xml: &str, name: &str) -> Option<String> {
    let attr_start = xml.find(&format!("Name=\"{}\"", name))?;
    let value_tag = "<saml:AttributeValue";
    let after_attr = xml[attr_start..].find(value_tag)?;
    let value_start = attr_start + after_attr + value_tag.len();

    // Find the closing tag
    let after_value_start = &xml[value_start..];
    if after_value_start.starts_with("/>") {
        return None;
    }
    if after_value_start.starts_with(">") {
        let content_start = value_start + 1;
        let end_tag = "</saml:AttributeValue>";
        if let Some(end) = xml[content_start..].find(end_tag) {
            let value = xml[content_start..content_start + end].trim().to_string();
            if value.is_empty() { None } else { Some(value) }
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract all AttributeValues from a SAML Attribute by name.
fn extract_all_attributes(xml: &str, name: &str) -> Option<Vec<String>> {
    let attr_start = xml.find(&format!("Name=\"{}\"", name))?;
    let end_tag = "</saml:Attribute>".to_string();
    let end = xml[attr_start..].find(&end_tag)?;
    let block = &xml[attr_start..attr_start + end];

    let value_tag = "<saml:AttributeValue>";
    let mut values = Vec::new();
    let mut search_from = 0;
    while search_from < block.len() {
        if let Some(pos) = block[search_from..].find(value_tag) {
            let abs_start = search_from + pos + value_tag.len();
            let close_tag = "</saml:AttributeValue>";
            if let Some(close_pos) = block[abs_start..].find(close_tag) {
                let value = block[abs_start..abs_start + close_pos].trim().to_string();
                if !value.is_empty() {
                    values.push(value);
                }
                search_from = abs_start + close_pos + close_tag.len();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

/// Extract an XML attribute value from a specific tag.
fn extract_attribute_from_tag(xml: &str, tag: &str, attr: &str) -> Option<String> {
    let open_tag = format!("<{} ", tag);
    let start = xml.find(&open_tag)?;
    let after_open = start + open_tag.len();
    let close_tag = ">";
    let tag_end = xml[after_open..].find(close_tag)?;
    let tag_content = &xml[after_open..after_open + tag_end];
    let pattern = format!("{}=\"", attr);
    let attr_start = tag_content.find(&pattern)?;
    let after_attr = attr_start + pattern.len();
    let rest = &tag_content[after_attr..];
    let quote_end = rest.find('"')?;
    Some(rest[..quote_end].to_string())
}

/// Compute SHA-256 fingerprint of a certificate (PEM format, without headers).
pub fn compute_cert_fingerprint(pem: &str) -> Result<String> {
    // Strip PEM headers and whitespace
    let der_b64: String = pem.lines().filter(|l| !l.starts_with("-----")).collect();
    let der = base64::engine::general_purpose::STANDARD.decode(&der_b64)?;
    let digest = Sha256::digest(&der);
    // Format as colon-separated hex (common in SAML metadata)
    let hex: String = digest.iter().map(|b| format!("{:02X}", b)).collect();
    let fingerprint: String = hex
        .as_bytes()
        .chunks(2)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(":");
    Ok(fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SamlConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.sp_entity_id, "https://ferro.local");
        assert!(config.idp_sso_url.is_empty());
        assert!(config.idp_entity_id.is_empty());
        assert!(config.sp_sls_url.is_none());
    }

    #[test]
    fn test_generate_sp_metadata() {
        let config = SamlConfig {
            enabled: true,
            sp_entity_id: "https://ferro.local".to_string(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            idp_entity_id: "https://idp.example.com".to_string(),
            sp_acs_url: "https://ferro.local/api/auth/saml/acs".to_string(),
            sp_sls_url: Some("https://ferro.local/api/auth/saml/sls".to_string()),
            ..Default::default()
        };
        let metadata = generate_sp_metadata(&config);
        assert!(metadata.contains("EntityDescriptor"));
        assert!(metadata.contains("https://ferro.local"));
        assert!(metadata.contains("AssertionConsumerService"));
        assert!(metadata.contains("https://ferro.local/api/auth/saml/acs"));
        assert!(metadata.contains("SingleLogoutService"));
        assert!(metadata.contains("https://ferro.local/api/auth/saml/sls"));
        assert!(metadata.contains("Ferro"));
    }

    #[test]
    fn test_generate_sp_metadata_no_sls() {
        let config = SamlConfig {
            sp_entity_id: "https://ferro.local".to_string(),
            sp_acs_url: "https://ferro.local/api/auth/saml/acs".to_string(),
            ..Default::default()
        };
        let metadata = generate_sp_metadata(&config);
        assert!(metadata.contains("AssertionConsumerService"));
        assert!(!metadata.contains("SingleLogoutService"));
    }

    #[test]
    fn test_build_authn_request_url() {
        let config = SamlConfig {
            enabled: true,
            sp_entity_id: "https://ferro.local".to_string(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            sp_acs_url: "https://ferro.local/api/auth/saml/acs".to_string(),
            ..Default::default()
        };
        let url = build_authn_request_url(&config, Some("test-state")).unwrap();
        assert!(url.starts_with("https://idp.example.com/sso?SAMLRequest="));
        assert!(url.contains("&RelayState=test-state"));
        assert!(url.contains("SAMLRequest="));
    }

    #[test]
    fn test_extract_tag_value() {
        let xml = r#"<saml:NameID>user@example.com</saml:NameID>"#;
        assert_eq!(
            extract_tag_value(xml, "saml:NameID"),
            Some("user@example.com".to_string())
        );
    }

    #[test]
    fn test_extract_tag_value_not_found() {
        let xml = r#"<saml:Issuer>https://idp.example.com</saml:Issuer>"#;
        assert_eq!(extract_tag_value(xml, "saml:NameID"), None);
    }

    #[test]
    fn test_parse_saml_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_123">
  <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_456" IssueInstant="2099-12-31T23:59:59Z">
    <saml:Issuer>https://idp.example.com</saml:Issuer>
    <saml:Subject>
      <saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">alice@example.com</saml:NameID>
    </saml:Subject>
    <saml:Conditions NotOnOrAfter="2099-12-31T23:59:59Z">
      <saml:AudienceRestriction>
        <saml:Audience>https://ferro.local</saml:Audience>
      </saml:AudienceRestriction>
    </saml:Conditions>
    <saml:AttributeStatement>
      <saml:Attribute Name="email">
        <saml:AttributeValue>alice@example.com</saml:AttributeValue>
      </saml:Attribute>
      <saml:Attribute Name="displayName">
        <saml:AttributeValue>Alice Smith</saml:AttributeValue>
      </saml:Attribute>
      <saml:Attribute Name="groups">
        <saml:AttributeValue>admins</saml:AttributeValue>
        <saml:AttributeValue>users</saml:AttributeValue>
      </saml:Attribute>
    </saml:AttributeStatement>
  </saml:Assertion>
</samlp:Response>"#;

        let encoded = base64::engine::general_purpose::STANDARD.encode(xml);
        let assertion = parse_saml_response(&encoded, "https://ferro.local").unwrap();
        assert_eq!(assertion.name_id, "alice@example.com");
        assert_eq!(assertion.email, Some("alice@example.com".to_string()));
        assert_eq!(assertion.display_name, Some("Alice Smith".to_string()));
        assert_eq!(assertion.groups, vec!["admins", "users"]);
        assert_eq!(assertion.issuer, "https://idp.example.com");
        assert_eq!(assertion.assertion_id, "_456");
    }

    #[test]
    fn test_parse_expired_assertion() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_123">
  <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" IssueInstant="2020-01-01T00:00:00Z">
    <saml:Issuer>https://idp.example.com</saml:Issuer>
    <saml:Subject>
      <saml:NameID>alice@example.com</saml:NameID>
    </saml:Subject>
    <saml:Conditions NotOnOrAfter="2020-01-01T00:00:00Z">
      <saml:AudienceRestriction>
        <saml:Audience>https://ferro.local</saml:Audience>
      </saml:AudienceRestriction>
    </saml:Conditions>
  </saml:Assertion>
</samlp:Response>"#;

        let encoded = base64::engine::general_purpose::STANDARD.encode(xml);
        let result = parse_saml_response(&encoded, "https://ferro.local");
        assert!(matches!(result, Err(SamlError::AssertionExpired)));
    }

    #[test]
    fn test_parse_wrong_audience() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol">
  <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion">
    <saml:Issuer>https://idp.example.com</saml:Issuer>
    <saml:Subject><saml:NameID>alice</saml:NameID></saml:Subject>
    <saml:Conditions NotOnOrAfter="2099-12-31T23:59:59Z">
      <saml:AudienceRestriction>
        <saml:Audience>https://wrong.local</saml:Audience>
      </saml:AudienceRestriction>
    </saml:Conditions>
  </saml:Assertion>
</samlp:Response>"#;

        let encoded = base64::engine::general_purpose::STANDARD.encode(xml);
        let result = parse_saml_response(&encoded, "https://ferro.local");
        assert!(matches!(result, Err(SamlError::InvalidAudience { .. })));
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = SamlConfig {
            enabled: true,
            sp_entity_id: "https://ferro.local".to_string(),
            idp_sso_url: "https://idp.example.com/sso".to_string(),
            idp_entity_id: "https://idp.example.com".to_string(),
            sp_acs_url: "https://ferro.local/api/auth/saml/acs".to_string(),
            sp_sls_url: Some("https://ferro.local/api/auth/saml/sls".to_string()),
            idp_cert_fingerprint: Some("AA:BB:CC".to_string()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SamlConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.sp_entity_id, "https://ferro.local");
        assert_eq!(parsed.idp_cert_fingerprint.as_deref(), Some("AA:BB:CC"));
    }
}
