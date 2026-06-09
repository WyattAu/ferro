use leptos::prelude::*;

#[component]
pub fn StatsCard(
    title: String,
    value: String,
    #[prop(default = String::new())] icon: String,
    #[prop(default = String::new())] trend: String,
) -> impl IntoView {
    let trend_class = if trend.starts_with('+') || trend.contains("up") {
        "trend-up"
    } else if trend.starts_with('-') || trend.contains("down") {
        "trend-down"
    } else {
        ""
    };
    let show_trend = !trend.is_empty();

    view! {
        <div class="stats-card">
            <div class="stats-card-header">
                <span class="stats-card-icon">{icon}</span>
                <span class="stats-card-title">{title}</span>
            </div>
            <div class="stats-card-value">{value}</div>
            {show_trend.then(|| view! { <span class={trend_class}>{trend}</span> })}
        </div>
    }
}
