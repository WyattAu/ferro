# SCIM Provisioning

## Overview

System for Cross-domain Identity Management (SCIM) enables automatic user provisioning.

## Supported Operations

### Users
- Create user
- Read user
- Update user
- Delete user
- List users
- Search users

### Groups
- Create group
- Read group
- Update group
- Delete group
- List groups

## Configuration

```toml
[scim]
enabled = true
token = "your-scim-token"
require_auth = true
```

## Endpoints

### Users
```http
GET /scim/v2/Users
GET /scim/v2/Users/{id}
POST /scim/v2/Users
PUT /scim/v2/Users/{id}
PATCH /scim/v2/Users/{id}
DELETE /scim/v2/Users/{id}
```

### Groups
```http
GET /scim/v2/Groups
GET /scim/v2/Groups/{id}
POST /scim/v2/Groups
PUT /scim/v2/Groups/{id}
PATCH /scim/v2/Groups/{id}
DELETE /scim/v2/Groups/{id}
```

### Schemas
```http
GET /scim/v2/Schemas
GET /scim/v2/Schemas/{id}
```

### ServiceProviderConfig
```http
GET /scim/v2/ServiceProviderConfig
```

## Implementation

### SCIM Types

```rust
pub struct ScimUser {
    pub id: String,
    pub external_id: Option<String>,
    pub user_name: String,
    pub name: ScimName,
    pub emails: Vec<ScimEmail>,
    pub active: bool,
    pub groups: Vec<ScimGroupRef>,
    pub meta: ScimMeta,
}

pub struct ScimName {
    pub formatted: Option<String>,
    pub family_name: Option<String>,
    pub given_name: Option<String>,
}

pub struct ScimEmail {
    pub value: String,
    pub primary: bool,
}

pub struct ScimGroup {
    pub id: String,
    pub display_name: String,
    pub members: Vec<ScimMember>,
    pub meta: ScimMeta,
}

pub struct ScimMeta {
    pub resource_type: String,
    pub created: Option<DateTime<Utc>>,
    pub last_modified: Option<DateTime<Utc>>,
    pub location: Option<String>,
}

pub struct ScimServiceProviderConfig {
    pub patch: ScimPatchConfig,
    pub bulk: ScimBulkConfig,
    pub filter: ScimFilterConfig,
    pub change_password: ScimChangePasswordConfig,
    pub sort: ScimSortConfig,
    pub etag: ScimEtagConfig,
    pub authentication_schemes: Vec<ScimAuthenticationScheme>,
}

pub struct ScimPatchConfig {
    pub supported: bool,
}

pub struct ScimBulkConfig {
    pub supported: bool,
    pub max_operations: u32,
    pub max_payload_size: u32,
}

pub struct ScimFilterConfig {
    pub supported: bool,
    pub max_results: u32,
}

pub struct ScimChangePasswordConfig {
    pub supported: bool,
}

pub struct ScimSortConfig {
    pub supported: bool,
}

pub struct ScimEtagConfig {
    pub supported: bool,
}

pub struct ScimAuthenticationScheme {
    pub r#type: String,
    pub name: String,
    pub description: String,
    pub spec_uri: Option<String>,
    pub documentation_uri: Option<String>,
    pub primary: bool,
}
```

### SCIM Manager

```rust
pub struct ScimManager {
    db: Database,
    auth: AuthManager,
}

impl ScimManager {
    pub async fn create_user(&self, user: ScimUser) -> Result<ScimUser, ScimError> {
        todo!()
    }

    pub async fn get_user(&self, id: &str) -> Result<ScimUser, ScimError> {
        todo!()
    }

    pub async fn update_user(&self, id: &str, user: ScimUser) -> Result<ScimUser, ScimError> {
        todo!()
    }

    pub async fn delete_user(&self, id: &str) -> Result<(), ScimError> {
        todo!()
    }

    pub async fn list_users(&self, filter: Option<&str>) -> Result<Vec<ScimUser>, ScimError> {
        todo!()
    }

    pub async fn search_users(&self, query: &str) -> Result<Vec<ScimUser>, ScimError> {
        todo!()
    }

    pub async fn create_group(&self, group: ScimGroup) -> Result<ScimGroup, ScimError> {
        todo!()
    }

    pub async fn get_group(&self, id: &str) -> Result<ScimGroup, ScimError> {
        todo!()
    }

    pub async fn update_group(&self, id: &str, group: ScimGroup) -> Result<ScimGroup, ScimError> {
        todo!()
    }

    pub async fn delete_group(&self, id: &str) -> Result<(), ScimError> {
        todo!()
    }

    pub async fn list_groups(&self) -> Result<Vec<ScimGroup>, ScimError> {
        todo!()
    }

    pub fn get_service_provider_config(&self) -> ScimServiceProviderConfig {
        todo!()
    }

    pub fn get_schemas(&self) -> Vec<ScimSchema> {
        todo!()
    }
}
```

### SCIM Error Types

```rust
pub enum ScimError {
    NotFound,
    Unauthorized,
    InvalidRequest,
    TooMany,
    SchemaViolation,
    InvalidSyntax,
    InvalidValue,
    InvalidFilter,
    InvalidPath,
    InvalidFilterPath,
    InvalidFilterValue,
    InvalidFilterOperator,
    InvalidFilterAttribute,
    InvalidFilterMediaType,
    InvalidFilterUri,
    InvalidFilterValueUri,
    InvalidFilterValueString,
    InvalidFilterValueBoolean,
    InvalidFilterValueInteger,
    InvalidFilterValueDecimal,
    InvalidFilterValueDateTime,
    InvalidFilterValueBinary,
    InvalidFilterValueReference,
    InvalidFilterValueComplex,
    InvalidFilterValueComposite,
    InvalidFilterValueExpression,
    InvalidFilterValueGroup,
    InvalidFilterValuePresence,
    InvalidFilterValueCompare,
    InvalidFilterValueStringMatch,
    InvalidFilterValueStringStartsWith,
    InvalidFilterValueStringEndsWith,
    InvalidFilterValueStringContains,
    InvalidFilterValueStringExact,
    InvalidFilterValueStringRegularExpression,
    InvalidFilterValueStringIgnoreCase,
    InvalidFilterValueStringCaseExact,
    InvalidFilterValueStringCaseInsensitive,
    InvalidFilterValueStringCaseSensitive,
    InvalidFilterValueStringCaseIgnore,
    InvalidFilterValueStringCaseExactMatch,
    InvalidFilterValueStringCaseInsensitiveMatch,
    InvalidFilterValueStringCaseSensitiveMatch,
    InvalidFilterValueStringCaseIgnoreMatch,
    InvalidFilterValueStringCaseExactIgnoreCaseMatch,
    InvalidFilterValueStringCaseInsensitiveIgnoreCaseMatch,
    InvalidFilterValueStringCaseSensitiveIgnoreCaseMatch,
    InvalidFilterValueStringCaseIgnoreIgnoreCaseMatch,
    InvalidFilterValueStringCaseExactCaseSensitiveMatch,
    InvalidFilterValueStringCaseInsensitiveCaseSensitiveMatch,
    InvalidFilterValueStringCaseSensitiveCaseSensitiveMatch,
    InvalidFilterValueStringCaseIgnoreCaseSensitiveMatch,
    InvalidFilterValueStringCaseExactCaseInsensitiveMatch,
    InvalidFilterValueStringCaseInsensitiveCaseInsensitiveMatch,
    InvalidFilterValueStringCaseSensitiveCaseInsensitiveMatch,
    InvalidFilterValueStringCaseIgnoreCaseInsensitiveMatch,
    ServerError(String),
}
```
