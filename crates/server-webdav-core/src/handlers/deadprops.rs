//! Dead-property persistence (RFC 4918 §9.2, R7 litmus gap).
//!
//! Dead properties are stored as a JSON sidecar object next to the
//! resource: `<path>.davprops`. The sidecar is filtered out of PROPFIND
//! listings (`is_sidecar`) — it is an implementation detail of the
//! namespace, not a resource.

use common::error::Result;
use std::collections::BTreeMap;

/// Suffix of the dead-property sidecar object.
pub const SIDECAR_SUFFIX: &str = ".davprops";

/// True when a storage name is a dead-property sidecar (hide from lists).
#[must_use]
pub fn is_sidecar(path: &str) -> bool {
    path.ends_with(SIDECAR_SUFFIX)
}

pub(crate) fn sidecar_path(path: &str) -> String {
    format!("{}{}", path.trim_end_matches('/'), SIDECAR_SUFFIX)
}

/// Load the dead-property map for `path` (empty when none stored).
pub(crate) async fn load<S: crate::WebDavCoreState>(state: &S, path: &str) -> BTreeMap<String, String> {
    let raw = state.storage().get(&sidecar_path(path)).await.unwrap_or_default();
    serde_json::from_slice(&raw).unwrap_or_default()
}

/// Persist the dead-property map for `path`.
pub(crate) async fn save<S: crate::WebDavCoreState>(
    state: &S,
    path: &str,
    map: &BTreeMap<String, String>,
) -> Result<()> {
    let body: bytes::Bytes = serde_json::to_vec(map)
        .map_err(|e| common::error::FerroError::Internal(format!("deadprops serialize: {e}")))?
        .into();
    state.storage().put(&sidecar_path(path), body, "davprops").await?;
    Ok(())
}
