use leptos::*;

#[derive(Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    Success,
    Warning,
    Danger,
    Info,
    Neutral,
}

impl BadgeVariant {
    fn class(&self) -> &'static str {
        match self {
            BadgeVariant::Success => "badge badge-success font-display",
            BadgeVariant::Warning => "badge badge-warning font-display",
            BadgeVariant::Danger => "badge badge-danger font-display",
            BadgeVariant::Info => "badge badge-info font-display",
            BadgeVariant::Neutral => "badge badge-neutral font-display",
        }
    }
}

#[component]
pub fn Badge(
    text: String,
    #[prop(default = BadgeVariant::Neutral)] variant: BadgeVariant,
) -> impl IntoView {
    let aria_label = format!("Status: {}", text);
    view! {
        <span class=variant.class() aria-label=aria_label>{text}</span>
    }
}
