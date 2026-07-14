# ferro-server-routes Crate

This crate provides utility functions and types for route configuration in the Ferro server.

## Overview

The `ferro-server-routes` crate was created as a partial extraction from the `ferro-server` crate's `routes.rs` module. It provides:

1. **Configuration types**: `MiddlewareConfig` for configuring the middleware stack
2. **Utility functions**: Helper functions for CORS validation, API path generation, and middleware layer creation
3. **Error types**: `ApiError` for middleware error responses

## Why Partial Extraction?

The full extraction of the middleware stack from `ferro-server` was not possible due to the following reasons:

### 1. **Tight Coupling to AppState**
The middleware layers in `routes.rs` are tightly coupled to the `AppState` struct, which is defined in the `ferro-server` crate. Many middleware functions capture fields from `AppState`:

- **Auth middleware**: Uses `oidc`, `cedar`, `admin_user`, `admin_password`, `user_store`, `api_key_store`
- **Maintenance middleware**: Uses `maintenance_mode`
- **Rate limiting**: Uses `rate_limit_burst`, `rate_limit_refill`
- **Request logging**: Uses `request_count`, `request_duration_buckets`, etc.

### 2. **Orphan Rules**
Rust's orphan rules prevent implementing traits for types defined in other crates. Since `AppState` is defined in `ferro-server`, we cannot:
- Implement middleware traits for `AppState` in `ferro-server-routes`
- Create generic middleware that works with `AppState` without requiring `AppState` to be generic

### 3. **Middleware Layer Composition**
The middleware layers in `routes.rs` are composed in a specific order that depends on the application's security requirements. Extracting them would require:
- Defining a trait that `AppState` must implement
- Making all middleware generic over this trait
- This would significantly complicate the codebase

## What Was Extracted

The following utilities were extracted to `ferro-server-routes`:

### Configuration
- `MiddlewareConfig`: Configuration struct for middleware stack settings

### Utility Functions
- `validate_cors_config()`: Validates CORS configuration security
- `versioned_api_path()`: Generates versioned API paths
- `create_compression_layer()`: Creates compression middleware
- `create_concurrency_limit_layer()`: Creates concurrency limit middleware
- `create_body_limit_layer()`: Creates body size limit middleware

### Error Types
- `ApiError`: Error type for middleware responses

## Usage

```rust
use ferro_server_routes::{validate_cors_config, versioned_api_path, MiddlewareConfig};

// Validate CORS configuration
let is_secure = validate_cors_config("*", true);

// Generate versioned API path
let api_path = versioned_api_path("v1"); // Returns "/api/v1"

// Create middleware configuration
let config = MiddlewareConfig::default();
```

## Future Work

To fully extract the middleware stack, the following changes would be needed:

1. **Define a trait in `ferro-server-state`** that `AppState` implements
2. **Make middleware generic** over this trait
3. **Extract middleware layers** into `ferro-server-routes` with trait bounds

This would require significant refactoring and is not recommended for the current codebase.
