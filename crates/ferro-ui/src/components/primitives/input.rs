use leptos::prelude::*;

/// Text input component with validation.
#[component]
pub fn Input(
    #[prop(optional)] value: String,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] error: bool,
    #[prop(optional)] class: String,
    #[prop(optional)] input_type: String,
    #[prop(optional)] _on_input: Option<Callback<String>>,
) -> impl IntoView {
    let itype = if input_type.is_empty() {
        "text".to_string()
    } else {
        input_type
    };
    let cls = format!(
        "input {} {} {class}",
        if error { "input-error" } else { "" },
        if disabled { "disabled" } else { "" },
    );
    let val = value.clone();

    view! {
        <input
            type=itype
            class=cls
            placeholder=placeholder
            disabled=disabled
            prop:value=val
        />
    }
}
