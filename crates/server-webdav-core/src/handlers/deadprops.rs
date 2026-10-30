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
            // Prefix-less names are bound by the DEFAULT namespace
            // (`xmlns="…"`) when declared — litmus writes props this way.
            None => match scope.get("") {
                Some(uri) => format!("{{{uri}}}{name}"),
                None => name.to_string(),
            },
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
                    } else if key == "xmlns" {
                        scope.insert(String::new(), val);
                    }
                }
                let local = raw.strip_prefix("D:").unwrap_or(&raw);
                if local == "prop" {
                    in_prop = true;
                } else if in_prop {
                    // The element's OWN xmlns declarations apply to its own prefix.
                    names.push((raw.clone(), canonical(&raw, &scope)));
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
                    } else if key == "xmlns" {
                        scope.insert(String::new(), val);
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

/// Namespace declarations + element naming for dead-property responses.
///
/// Each distinct URI gets a generated prefix (`ns0`, `ns1`, ...) declared
/// on the multistatus root; response elements are emitted with that
/// prefix. Correct for any mix of default/prefixed request namespaces
/// (litmus's `manyns` sets ten props across ten namespaces).
#[derive(Default)]
pub(crate) struct NsScope {
    decls: Vec<(String, String)>,
    map: std::collections::BTreeMap<String, String>,
}

impl NsScope {
    pub(crate) fn build(requested: &[(String, String)]) -> Self {
        let mut out = Self::default();
        for (_, canonical) in requested {
            let Some((_, rest)) = canonical.split_once('{') else {
                continue;
            };
            let Some((uri, _)) = rest.split_once('}') else {
                continue;
            };
            if !out.map.contains_key(uri) {
                let prefix = format!("ns{}", out.map.len());
                out.map.insert(uri.to_string(), prefix.clone());
                out.decls.push((format!("xmlns:{prefix}"), uri.to_string()));
            }
        }
        out
    }

    /// Root attribute declarations (`xmlns:ns0="uri"`).
    pub(crate) fn decls(&self) -> &[(String, String)] {
        &self.decls
    }

    /// The prefixed element name for a Clark-form property name.
    pub(crate) fn element(&self, canonical: &str) -> String {
        let Some((_, rest)) = canonical.split_once('{') else {
            return canonical.to_string();
        };
        let Some((uri, local)) = rest.split_once('}') else {
            return canonical.to_string();
        };
        match self.map.get(uri) {
            Some(prefix) => format!("{prefix}:{local}"),
            None => local.to_string(),
        }
    }
}
