use std::collections::HashMap;

/// A single property within an iCalendar component.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IcalProperty {
    /// Property name (e.g. "DTSTART", "SUMMARY").
    pub name: String,
    /// Parameter key-value pairs (e.g. {"VALUE": "DATE"}).
    pub params: HashMap<String, String>,
    /// Property value.
    pub value: String,
}

/// A parsed iCalendar component (e.g. VCALENDAR, VEVENT, VTODO).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IcalComponent {
    /// Component name (e.g. "VCALENDAR", "VEVENT").
    pub name: String,
    /// Properties grouped by name, preserving multiple occurrences.
    pub properties: HashMap<String, Vec<IcalProperty>>,
    /// Nested child components.
    pub children: Vec<IcalComponent>,
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

fn parse_property(line: &str) -> Option<IcalProperty> {
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

    Some(IcalProperty {
        name,
        params,
        value: value.to_string(),
    })
}

fn parse_component_lines(lines: &[&str], offset: &mut usize) -> Option<IcalComponent> {
    if *offset >= lines.len() {
        return None;
    }

    let header = lines[*offset].trim_start_matches("BEGIN:").trim();
    if header.is_empty() {
        return None;
    }
    *offset += 1;

    let mut component = IcalComponent {
        name: header.to_uppercase(),
        properties: HashMap::new(),
        children: Vec::new(),
    };

    while *offset < lines.len() {
        let line = lines[*offset].trim();
        if line.is_empty() {
            *offset += 1;
            continue;
        }

        if let Some(end_tag) = line.strip_prefix("END:") {
            let end = end_tag.trim().to_uppercase();
            if end == component.name {
                *offset += 1;
                break;
            }
        }

        if let Some(begin_tag) = line.strip_prefix("BEGIN:") {
            let begin = begin_tag.trim().to_uppercase();
            if begin != component.name
                && let Some(child) = parse_component_lines(lines, offset)
            {
                component.children.push(child);
                continue;
            }
        }

        if let Some(prop) = parse_property(line) {
            component
                .properties
                .entry(prop.name.clone())
                .or_default()
                .push(prop);
        }

        *offset += 1;
    }

    Some(component)
}

/// Parse an iCalendar (RFC 5545) string into structured components.
pub fn parse_ical(input: &str) -> Result<Vec<IcalComponent>, String> {
    let unfolded = unfold_lines(input);
    let lines: Vec<&str> = unfolded.lines().collect();
    let mut components = Vec::new();
    let mut offset = 0;

    while offset < lines.len() {
        let line = lines[offset].trim();
        if line.starts_with("BEGIN:") {
            if let Some(comp) = parse_component_lines(&lines, &mut offset) {
                components.push(comp);
            }
        } else {
            offset += 1;
        }
    }

    Ok(components)
}

fn escape_ical_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn format_property(prop: &IcalProperty) -> String {
    let mut s = prop.name.clone();
    for (k, v) in &prop.params {
        s.push_str(&format!(";{}={}", k, v));
    }
    s.push(':');
    s.push_str(&escape_ical_value(&prop.value));
    s
}

fn serialize_component(component: &IcalComponent, indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut s = format!("{}BEGIN:{}\n", pad, component.name);

    let mut props_ordered: Vec<_> = component.properties.iter().collect();
    props_ordered.sort_by_key(|(k, _)| k.as_str());

    for (_, props) in &props_ordered {
        for prop in *props {
            s.push_str(&format!("{}{}\n", pad, format_property(prop)));
        }
    }

    for child in &component.children {
        s.push_str(&serialize_component(child, indent));
    }

    s.push_str(&format!("{}END:{}\n", pad, component.name));
    s
}

/// Serialize a list of iCalendar components back to an iCalendar string.
pub fn serialize_ical(components: &[IcalComponent]) -> String {
    let mut s = String::new();
    for comp in components {
        s.push_str(&serialize_component(comp, 0));
    }
    s
}

/// Get the first property with the given name from a component.
pub fn get_first_prop<'a>(component: &'a IcalComponent, name: &str) -> Option<&'a IcalProperty> {
    component.properties.get(name).and_then(|v| v.first())
}

/// Get all properties with the given name from a component.
pub fn get_all_props<'a>(component: &'a IcalComponent, name: &str) -> Vec<&'a IcalProperty> {
    component
        .properties
        .get(name)
        .map(|v| v.iter().collect())
        .unwrap_or_default()
}
