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

/// Canonical property names from a request body: `(raw_prefixed_name,
/// clark_form)` pairs for every child of `<D:prop>`, in document order.
/// Clark form is `{namespace-uri}local` when the prefix is declared in
/// scope — making stored dead properties independent of the client's
/// prefix choice. `None` when the body has no `<D:prop>` list.
pub(crate) fn canonical_prop_names(body: &[u8]) -> Option<Vec<(String, String)>> {
    use quick_xml::events::Event;
    use std::collections::BTreeMap;
    if body.is_empty() {
        return None;
    }
    let mut reader = quick_xml::Reader::from_reader(body);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut scopes: Vec<BTreeMap<String, String>> = vec![BTreeMap::new()];
    let mut names: Vec<(String, String)> = Vec::new();
    let mut in_prop = false;
    let canonical = |name: &str, scope: &BTreeMap<String, String>| -> String {
        match name.split_once(':') {
            Some((prefix, local)) => match scope.get(prefix) {
                Some(uri) => format!("{{{uri}}}{local}"),
                None => name.to_string(),
            },
            None => name.to_string(),
        }
    };
    loop {
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(ref e) => {
                let raw = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut scope = scopes.last().cloned().unwrap_or_default();
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&a.value).to_string();
                    if let Some(pfx) = key.strip_prefix("xmlns:") {
                        scope.insert(pfx.to_string(), val);
                    }
                }
                let local = raw.strip_prefix("D:").unwrap_or(&raw);
                if local == "prop" {
                    in_prop = true;
                } else if in_prop {
                    let scope_now = scopes.last().cloned().unwrap_or_default();
                    names.push((raw.clone(), canonical(&raw, &scope_now)));
                }
                scopes.push(scope);
            }
            Event::Empty(ref e) => {
                let raw = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let scope_now = scopes.last().cloned().unwrap_or_default();
                let mut scope = scope_now.clone();
                for a in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&a.value).to_string();
                    if let Some(pfx) = key.strip_prefix("xmlns:") {
                        scope.insert(pfx.to_string(), val);
                    }
                }
                let local = raw.strip_prefix("D:").unwrap_or(&raw);
                if in_prop && local != "prop" {
                    names.push((raw.clone(), canonical(&raw, &scope)));
                }
            }
            Event::End(ref e) => {
                let raw = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = raw.strip_prefix("D:").unwrap_or(&raw);
                if local == "prop" {
                    in_prop = false;
                }
                scopes.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if names.is_empty() { None } else { Some(names) }
}

/// `xmlns:` attributes to declare on a response root so the request's raw
/// prefixed names remain valid XML and namespace-resolvable: prefix from
/// the raw name, URI from the Clark form.
pub(crate) fn xmlns_attrs_for(requested: &[(String, String)]) -> Vec<(String, String)> {
    requested
        .iter()
        .filter_map(|(raw, canonical)| {
            let prefix = raw.split_once(':').map(|(p, _)| p.to_string())?;
            let uri = canonical
                .split_once('{')
                .and_then(|(_, rest)| rest.split_once('}'))
                .map(|(uri, _)| uri.to_string())?;
            Some((format!("xmlns:{prefix}"), uri))
        })
        .collect()
}
