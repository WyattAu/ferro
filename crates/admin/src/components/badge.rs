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
            BadgeVariant::Success => "badge badge-success",
            BadgeVariant::Warning => "badge badge-warning",
            BadgeVariant::Danger => "badge badge-danger",
            BadgeVariant::Info => "badge badge-info",
            BadgeVariant::Neutral => "badge badge-neutral",
        }
    }
}

#[component]
pub fn Badge(
    text: String,
    #[prop(default = BadgeVariant::Neutral)] variant: BadgeVariant,
) -> impl IntoView {
    view! {
        <span class=variant.class()>{text}</span>
    }
}
