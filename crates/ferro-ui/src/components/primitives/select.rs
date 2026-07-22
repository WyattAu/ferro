use leptos::prelude::*;

/// Select dropdown component.
#[component]
pub fn Select(
    #[prop(into)] value: String,
    #[prop(into, optional)] placeholder: String,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: String,
    #[prop(optional)] options: Vec<SelectOption>,
) -> impl IntoView {
    let cls = format!("input {class}");
    let val = value.clone();

    view! {
        <select class=cls disabled=disabled prop:value=val>
            {if !placeholder.is_empty() {
                view! { <option value="" disabled>{placeholder}</option> }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
            {options.into_iter().map(|opt| {
                let sel = opt.value == value;
                view! { <option value=opt.value selected=sel>{opt.label}</option> }
            }).collect_view()}
        </select>
    }
}

#[derive(Clone, Debug)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}
