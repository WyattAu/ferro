# SAML/OIDC Integration

## Overview

Ferro supports SAML 2.0 and OpenID Connect for enterprise authentication.

## SAML 2.0

### Configuration

```toml
[auth.saml]
enabled = true
idp_metadata_url = "https://idp.example.com/metadata"
idp_metadata_path = "/config/idp-metadata.xml"
sp_entity_id = "ferro.example.com"
acs_url = "https://ferro.example.com/auth/saml/acs"
slo_url = "https://ferro.example.com/auth/saml/slo"
certificate_path = "/config/sp-certificate.pem"
private_key_path = "/config/sp-private-key.pem"
```

### Endpoints

- **ACS (Assertion Consumer Service):** `/auth/saml/acs`
- **SLO (Single Logout):** `/auth/saml/slo`
- **Metadata:** `/auth/saml/metadata`

### Flow

1. User accesses Ferro
2. Redirected to IdP for authentication
3. IdP authenticates user
4. IdP sends SAML assertion to ACS
5. Ferro validates assertion
6. User session created

## OpenID Connect

### Configuration

```toml
[auth.oidc]
enabled = true
issuer = "https://auth.example.com"
client_id = "ferro"
client_secret = "secret"
redirect_uri = "https://ferro.example.com/auth/oidc/callback"
scopes = ["openid", "email", "profile"]
```

### Endpoints

- **Authorization:** `/auth/oidc/authorize`
- **Callback:** `/auth/oidc/callback`
- **Logout:** `/auth/oidc/logout`

### Flow

1. User accesses Ferro
2. Redirected to IdP for authentication
3. IdP authenticates user
4. IdP sends authorization code to callback
5. Ferro exchanges code for tokens
6. User session created

## Implementation

### SAML Types

```rust
pub struct SamlConfig {
    pub enabled: bool,
    pub idp_metadata_url: Option<String>,
    pub idp_metadata_path: Option<String>,
    pub sp_entity_id: String,
    pub acs_url: String,
    pub slo_url: String,
    pub certificate_path: Option<String>,
    pub private_key_path: Option<String>,
}

pub struct SamlAssertion {
    pub name_id: String,
    pub session_index: String,
    pub attributes: HashMap<String, String>,
    pub not_before: DateTime<Utc>,
    pub not_on_or_after: DateTime<Utc>,
}
```

### OIDC Types

```rust
pub struct OidcConfig {
    pub enabled: bool,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

pub struct OidcTokens {
    pub access_token: String,
    pub id_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture: Option<String>,
}
```
