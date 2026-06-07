use leptos::*;

#[derive(Clone)]
pub struct Column {
    pub key: String,
    pub label: String,
}

#[component]
pub fn DataTable(columns: Vec<Column>, rows: Vec<serde_json::Value>) -> impl IntoView {
    let has_data = !rows.is_empty();

    view! {
        <div class="table-wrapper">
            <table class="data-table" aria-label="Data table">
                <thead>
                    <tr>
                        {columns.iter().map(|c| view! { <th scope="col">{&c.label}</th> }).collect::<Vec<_>>()}
                    </tr>
                </thead>
                <tbody>
                    {if has_data {
                        rows.iter().map(|row| {
                            view! {
                                <tr>
                                    {columns.iter().map(|c| {
                                        let val = format_json_value(row.get(&c.key));
                                        view! { <td>{val}</td> }
                                    }).collect::<Vec<_>>()}
                                </tr>
                            }
                        }).collect::<Vec<_>>()
                    } else {
                        vec![view! {
                            <tr>
                                <td colspan={columns.len().to_string()} class="table-empty">
                                    "No data available"
                                </td>
                            </tr>
                        }]
                    }}
                </tbody>
            </table>
        </div>
    }
}

pub fn format_json_value(val: Option<&serde_json::Value>) -> String {
    match val {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) => "-".to_string(),
        Some(serde_json::Value::Array(arr)) => format!("[{} items]", arr.len()),
        Some(serde_json::Value::Object(_)) => "{...}".to_string(),
        None => "-".to_string(),
    }
}
