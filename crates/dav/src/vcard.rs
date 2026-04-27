use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VcardProperty {
    pub name: String,
    pub params: HashMap<String, String>,
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VcardValue {
    pub value: String,
    pub types: Vec<String>,
    pub pref: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VcardAddress {
    pub po_box: String,
    pub extended: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Vcard {
    pub uid: Option<String>,
    pub fn_name: String,
    pub family_name: String,
    pub given_name: String,
    pub additional_names: String,
    pub prefix: String,
    pub suffix: String,
    pub emails: Vec<VcardValue>,
    pub phones: Vec<VcardValue>,
    pub addresses: Vec<VcardAddress>,
    pub org: Option<String>,
    pub title: Option<String>,
    pub role: Option<String>,
    pub photo: Option<String>,
    pub rev: Option<String>,
    pub version: Option<String>,
    pub properties: HashMap<String, Vec<VcardProperty>>,
}

fn unfold_lines(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            if chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                chars.next();
                continue;
            }
            result.push('\n');
        } else if ch == '\n' {
            if chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
                chars.next();
                continue;
            }
            result.push('\n');
        } else {
            result.push(ch);
        }
    }
    result
}

fn parse_property_line(line: &str) -> Option<VcardProperty> {
    let (name_params, value) = line.split_once(':').map(|(n, v)| (n.trim(), v.trim()))?;
    if name_params.is_empty() {
        return None;
    }

    let mut parts = name_params.split(';');
    let name = parts.next()?.to_uppercase();

    let mut params = HashMap::new();
    for part in parts {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            params.insert(k.trim().to_uppercase(), v.trim().to_string());
        }
    }

    Some(VcardProperty {
        name,
        params,
        value: value.to_string(),
    })
}

fn parse_structured_name(value: &str) -> (String, String, String, String, String) {
    let parts: Vec<&str> = value.splitn(5, ';').collect();
    let family = parts.first().unwrap_or(&"").to_string();
    let given = parts.get(1).unwrap_or(&"").to_string();
    let additional = parts.get(2).unwrap_or(&"").to_string();
    let prefix = parts.get(3).unwrap_or(&"").to_string();
    let suffix = parts.get(4).unwrap_or(&"").to_string();
    (family, given, additional, prefix, suffix)
}

fn parse_address(value: &str, types: Vec<String>) -> VcardAddress {
    let parts: Vec<&str> = value.splitn(7, ';').collect();
    VcardAddress {
        po_box: parts.first().unwrap_or(&"").to_string(),
        extended: parts.get(1).unwrap_or(&"").to_string(),
        street: parts.get(2).unwrap_or(&"").to_string(),
        city: parts.get(3).unwrap_or(&"").to_string(),
        region: parts.get(4).unwrap_or(&"").to_string(),
        postal_code: parts.get(5).unwrap_or(&"").to_string(),
        country: parts.get(6).unwrap_or(&"").to_string(),
        types,
    }
}

fn extract_types(params: &HashMap<String, String>) -> Vec<String> {
    params
        .get("TYPE")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_uppercase())
                .collect()
        })
        .unwrap_or_default()
}

fn extract_pref(params: &HashMap<String, String>) -> Option<u32> {
    params.get("PREF").and_then(|v| v.parse().ok())
}

pub fn parse_vcard(input: &str) -> Result<Vcard, String> {
    let unfolded = unfold_lines(input);
    let lines: Vec<&str> = unfolded.lines().collect();

    let mut in_vcard = false;
    let mut vcard = Vcard::default();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("BEGIN:VCARD") {
            in_vcard = true;
            continue;
        }
        if trimmed.eq_ignore_ascii_case("END:VCARD") {
            break;
        }

        if !in_vcard {
            continue;
        }

        let Some(prop) = parse_property_line(trimmed) else {
            continue;
        };

        match prop.name.as_str() {
            "UID" => vcard.uid = Some(prop.value),
            "FN" => vcard.fn_name = prop.value,
            "N" => {
                let (family, given, additional, prefix, suffix) = parse_structured_name(&prop.value);
                vcard.family_name = family;
                vcard.given_name = given;
                vcard.additional_names = additional;
                vcard.prefix = prefix;
                vcard.suffix = suffix;
            }
            "EMAIL" => {
                vcard.emails.push(VcardValue {
                    value: prop.value,
                    types: extract_types(&prop.params),
                    pref: extract_pref(&prop.params),
                });
            }
            "TEL" => {
                vcard.phones.push(VcardValue {
                    value: prop.value,
                    types: extract_types(&prop.params),
                    pref: extract_pref(&prop.params),
                });
            }
            "ADR" => {
                vcard
                    .addresses
                    .push(parse_address(&prop.value, extract_types(&prop.params)));
            }
            "ORG" => vcard.org = Some(prop.value.replace(';', ", ")),
            "TITLE" => vcard.title = Some(prop.value),
            "ROLE" => vcard.role = Some(prop.value),
            "PHOTO" => vcard.photo = Some(prop.value),
            "REV" => vcard.rev = Some(prop.value),
            "VERSION" => vcard.version = Some(prop.value),
            _ => {
                vcard
                    .properties
                    .entry(prop.name.clone())
                    .or_default()
                    .push(prop);
            }
        }
    }

    Ok(vcard)
}

fn escape_vcard_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

pub fn serialize_vcard(vcard: &Vcard) -> String {
    let mut s = String::new();
    s.push_str("BEGIN:VCARD\r\n");

    if let Some(ref version) = vcard.version {
        s.push_str(&format!("VERSION:{}\r\n", version));
    } else {
        s.push_str("VERSION:3.0\r\n");
    }

    if let Some(ref uid) = vcard.uid {
        s.push_str(&format!("UID:{}\r\n", escape_vcard_value(uid)));
    }

    s.push_str(&format!("FN:{}\r\n", escape_vcard_value(&vcard.fn_name)));

    s.push_str(&format!(
        "N:{};{};{};{};{}\r\n",
        escape_vcard_value(&vcard.family_name),
        escape_vcard_value(&vcard.given_name),
        escape_vcard_value(&vcard.additional_names),
        escape_vcard_value(&vcard.prefix),
        escape_vcard_value(&vcard.suffix),
    ));

    for email in &vcard.emails {
        let mut params = String::new();
        if !email.types.is_empty() {
            params.push_str(&format!("TYPE={}", email.types.join(",")));
        }
        if let Some(pref) = email.pref {
            if !params.is_empty() {
                params.push(';');
            }
            params.push_str(&format!("PREF={}", pref));
        }
        if !params.is_empty() {
            s.push_str(&format!("EMAIL;{}:{}\r\n", params, escape_vcard_value(&email.value)));
        } else {
            s.push_str(&format!("EMAIL:{}\r\n", escape_vcard_value(&email.value)));
        }
    }

    for phone in &vcard.phones {
        let mut params = String::new();
        if !phone.types.is_empty() {
            params.push_str(&format!("TYPE={}", phone.types.join(",")));
        }
        if let Some(pref) = phone.pref {
            if !params.is_empty() {
                params.push(';');
            }
            params.push_str(&format!("PREF={}", pref));
        }
        if !params.is_empty() {
            s.push_str(&format!("TEL;{}:{}\r\n", params, escape_vcard_value(&phone.value)));
        } else {
            s.push_str(&format!("TEL:{}\r\n", escape_vcard_value(&phone.value)));
        }
    }

    for addr in &vcard.addresses {
        let mut params = String::new();
        if !addr.types.is_empty() {
            params.push_str(&format!("TYPE={}", addr.types.join(",")));
        }
        let addr_val = format!(
            "{};{};{};{};{};{};{}",
            addr.po_box,
            addr.extended,
            addr.street,
            addr.city,
            addr.region,
            addr.postal_code,
            addr.country
        );
        if !params.is_empty() {
            s.push_str(&format!("ADR;{}:{}\r\n", params, escape_vcard_value(&addr_val)));
        } else {
            s.push_str(&format!("ADR:{}\r\n", escape_vcard_value(&addr_val)));
        }
    }

    if let Some(ref org) = vcard.org {
        s.push_str(&format!("ORG:{}\r\n", escape_vcard_value(org)));
    }
    if let Some(ref title) = vcard.title {
        s.push_str(&format!("TITLE:{}\r\n", escape_vcard_value(title)));
    }
    if let Some(ref role) = vcard.role {
        s.push_str(&format!("ROLE:{}\r\n", escape_vcard_value(role)));
    }
    if let Some(ref photo) = vcard.photo {
        s.push_str(&format!("PHOTO:{}\r\n", escape_vcard_value(photo)));
    }
    if let Some(ref rev) = vcard.rev {
        s.push_str(&format!("REV:{}\r\n", escape_vcard_value(rev)));
    }

    let mut keys: Vec<_> = vcard.properties.keys().collect();
    keys.sort();
    for key in keys {
        if let Some(props) = vcard.properties.get(key) {
            for prop in props {
                let mut line = prop.name.clone();
                for (k, v) in &prop.params {
                    line.push_str(&format!(";{}={}", k, v));
                }
                line.push(':');
                line.push_str(&escape_vcard_value(&prop.value));
                s.push_str(&format!("{}\r\n", line));
            }
        }
    }

    s.push_str("END:VCARD\r\n");
    s
}
