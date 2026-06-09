use leptos::ev;
use leptos::prelude::*;

/// Accessible button component with proper ARIA attributes and minimum touch target.
///
/// Follows the leptix pattern: wraps a native `<button>` with consistent
/// styling, focus management, and accessibility attributes.
/// See: https://docs.rs/leptix for the upstream component library patterns.
#[component]
pub fn Button(
    /// Button variant.
    #[prop(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    /// Button size.
    #[prop(default = ButtonSize::Md)]
    size: ButtonSize,
    /// Whether the button is disabled.
    #[prop(default = false)]
    disabled: bool,
    /// Whether the button is loading (shows spinner, disables interaction).
    #[prop(default = false)]
    loading: bool,
    /// Accessible label when button only contains an icon.
    #[prop(default = None)]
    aria_label: Option<String>,
    /// Button type attribute.
    #[prop(default = "button".to_string())]
    button_type: String,
    /// Called on click.
    on_click: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let base = "inline-flex items-center justify-center font-medium transition-colors \
        focus:outline-none focus:ring-2 focus:ring-offset-2 dark:focus:ring-offset-gray-800 \
        disabled:opacity-50 disabled:cursor-not-allowed min-w-[44px] min-h-[44px]";

    let variant_class = match variant {
        ButtonVariant::Primary => "bg-blue-600 text-white hover:bg-blue-700 focus:ring-blue-500",
        ButtonVariant::Secondary => {
            "bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 hover:bg-gray-200 dark:hover:bg-gray-600 focus:ring-gray-500"
        }
        ButtonVariant::Danger => "bg-red-600 text-white hover:bg-red-700 focus:ring-red-500",
        ButtonVariant::Ghost => {
            "text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-gray-500"
        }
        ButtonVariant::Link => {
            "text-blue-600 hover:underline focus:ring-blue-500 p-0 min-w-0 min-h-0"
        }
    };

    let size_class = match size {
        ButtonSize::Sm => "px-3 py-1.5 text-sm rounded",
        ButtonSize::Md => "px-4 py-2 text-sm rounded-md",
        ButtonSize::Lg => "px-6 py-3 text-base rounded-md",
    };

    let class = format!("{} {} {}", base, variant_class, size_class);

    let on_click = on_click.unwrap_or_else(|| Callback::new(|_| {}));

    view! {
        <button
            type=button_type
            class=class
            disabled=disabled || loading
            aria-disabled=disabled || loading
            aria-busy=loading
            aria-label=aria_label
            on:click=move |ev| on_click.run(ev)
        >
            {move || {
                if loading {
                    view! {
                        <svg class="animate-spin -ml-1 mr-2 h-4 w-4" aria-hidden="true" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                        </svg>
                        <span class="sr-only">"Loading..."</span>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
            {children()}
        </button>
    }
}

/// Button variant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Link,
}

/// Button size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

/// Accessible input component with label association.
///
/// Wraps a native `<input>` with a visually hidden or visible `<label>`,
/// proper `id`/`for` association, and error/description support.
/// Pattern inspired by leptix's `<TextField>` and `<NumberField>` primitives.
#[component]
pub fn Input(
    /// Unique ID for the input (used to associate label).
    #[prop(into)]
    id: String,
    /// Visible label text. If None, `aria_label` must be provided.
    #[prop(default = None)]
    label: Option<String>,
    /// Accessible label when no visible label is used.
    #[prop(default = None)]
    aria_label: Option<String>,
    /// Input type.
    #[prop(default = "text".to_string())]
    input_type: String,
    /// Placeholder text.
    #[prop(default = None)]
    placeholder: Option<String>,
    /// Current value (controlled).
    #[prop(default = None)]
    value: Option<String>,
    /// Whether the input is disabled.
    #[prop(default = false)]
    disabled: bool,
    /// Whether the input is required.
    #[prop(default = false)]
    required: bool,
    /// Error message displayed below the input.
    #[prop(default = None)]
    error: Option<String>,
    /// Help text displayed below the input.
    #[prop(default = None)]
    help_text: Option<String>,
    /// Called when the input value changes.
    #[prop(default = None)]
    on_input: Option<Callback<String>>,
    /// Called on blur.
    #[prop(default = None)]
    on_blur: Option<Callback<ev::FocusEvent>>,
    /// Additional CSS classes for the input element.
    #[prop(default = None)]
    class: Option<String>,
) -> impl IntoView {
    let input_id = id.clone();
    let error_id = format!("{}-error", id);
    let help_id = format!("{}-help", id);

    let has_error = error.is_some();
    let base_input_class = "block w-full rounded-md border px-3 py-2 text-sm \
        placeholder-gray-400 dark:placeholder-gray-500 \
        focus:outline-none focus:ring-2 focus:ring-offset-1 dark:focus:ring-offset-gray-800 \
        disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed \
        min-h-[44px]";

    let border_class = if has_error {
        "border-red-300 dark:border-red-600 focus:ring-red-500 focus:border-red-500"
    } else {
        "border-gray-300 dark:border-gray-600 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800"
    };

    let input_class = match &class {
        Some(c) => format!("{} {} {}", base_input_class, border_class, c),
        None => format!("{} {}", base_input_class, border_class),
    };

    let described_by = {
        let mut ids = Vec::new();
        if error.is_some() {
            ids.push(error_id.clone());
        }
        if help_text.is_some() {
            ids.push(help_id.clone());
        }
        if ids.is_empty() {
            None
        } else {
            Some(ids.join(" "))
        }
    };

    let on_input = on_input.unwrap_or_else(|| Callback::new(|_| {}));
    let on_blur = on_blur.unwrap_or_else(|| Callback::new(|_| {}));

    view! {
        <div class="w-full">
            {label.map(|label_text| view! {
                <label for=input_id.clone() class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    {label_text}
                </label>
            })}
            <input
                type=input_type
                id=input_id.clone()
                class=input_class
                placeholder=placeholder
                prop:value=value
                disabled=disabled
                required=required
                aria-label=aria_label
                aria-invalid=has_error
                aria-describedby=described_by
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    on_input.run(val);
                }
                on:blur=move |ev| on_blur.run(ev)
            />
            {error.map(|err| view! {
                <p id=error_id class="mt-1 text-sm text-red-600 dark:text-red-400" role="alert">
                    {err}
                </p>
            })}
            {help_text.map(|help| view! {
                <p id=help_id class="mt-1 text-sm text-gray-500 dark:text-gray-400">
                    {help}
                </p>
            })}
        </div>
    }
}

/// Accessible select/dropdown component.
///
/// Wraps a native `<select>` with label association and error support.
#[component]
pub fn Select(
    /// Unique ID for the select.
    #[prop(into)]
    id: String,
    /// Visible label text.
    #[prop(default = None)]
    label: Option<String>,
    /// Accessible label when no visible label is used.
    #[prop(default = None)]
    aria_label: Option<String>,
    /// Currently selected value.
    #[prop(default = None)]
    value: Option<String>,
    /// Whether the select is disabled.
    #[prop(default = false)]
    disabled: bool,
    /// Whether the select is required.
    #[prop(default = false)]
    required: bool,
    /// Error message.
    #[prop(default = None)]
    error: Option<String>,
    /// Called when selection changes.
    #[prop(default = None)]
    on_change: Option<Callback<String>>,
    /// Select options as (value, label, disabled) tuples.
    options: Vec<SelectOption>,
    /// Additional CSS classes.
    #[prop(default = None)]
    class: Option<String>,
) -> impl IntoView {
    let select_id = id.clone();
    let error_id = format!("{}-error", id);

    let has_error = error.is_some();
    let base_class = "block w-full rounded-md border px-3 py-2 text-sm \
        focus:outline-none focus:ring-2 focus:ring-offset-1 dark:focus:ring-offset-gray-800 \
        disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:cursor-not-allowed \
        min-h-[44px]";

    let border_class = if has_error {
        "border-red-300 dark:border-red-600 focus:ring-red-500 focus:border-red-500"
    } else {
        "border-gray-300 dark:border-gray-600 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-800"
    };

    let select_class = match &class {
        Some(c) => format!("{} {} {}", base_class, border_class, c),
        None => format!("{} {}", base_class, border_class),
    };

    let on_change = on_change.unwrap_or_else(|| Callback::new(|_| {}));

    view! {
        <div class="w-full">
            {label.map(|label_text| view! {
                <label for=select_id.clone() class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    {label_text}
                </label>
            })}
            <select
                id=select_id.clone()
                class=select_class
                prop:value=value
                disabled=disabled
                required=required
                aria-label=aria_label
                aria-invalid=has_error
                aria-describedby=if has_error { Some(error_id.clone()) } else { None }
                on:change=move |ev| {
                    let val = event_target_value(&ev);
                    on_change.run(val);
                }
            >
                {options.into_iter().map(|opt| view! {
                    <option
                        value=opt.value
                        disabled=opt.disabled
                        selected=opt.selected
                    >
                        {opt.label}
                    </option>
                }).collect_view()}
            </select>
            {error.map(|err| view! {
                <p id=error_id class="mt-1 text-sm text-red-600 dark:text-red-400" role="alert">
                    {err}
                </p>
            })}
        </div>
    }
}

/// Option for the Select component.
#[derive(Clone, Debug)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
    #[allow(dead_code)]
    pub disabled: bool,
    pub selected: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
            selected: false,
        }
    }

    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }

    #[allow(dead_code)]
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// Accessible checkbox component with proper label association.
///
/// Renders a styled checkbox with a visible or associated label.
#[component]
pub fn Checkbox(
    /// Unique ID for the checkbox.
    #[prop(into)]
    id: String,
    /// Label text displayed next to the checkbox.
    label: String,
    /// Whether the checkbox is checked.
    checked: Signal<bool>,
    /// Toggle the checked state.
    on_change: Callback<bool>,
    /// Whether the checkbox is disabled.
    #[prop(default = false)]
    disabled: bool,
    /// Whether the checkbox is required.
    #[prop(default = false)]
    required: bool,
    /// Error message.
    #[prop(default = None)]
    error: Option<String>,
) -> impl IntoView {
    let checkbox_id = id.clone();

    view! {
        <div class="flex items-start gap-2">
            <div class="flex items-center h-5">
                <input
                    type="checkbox"
                    id=checkbox_id.clone()
                    class="w-4 h-4 rounded border-gray-300 dark:border-gray-600 text-blue-600 \
                        focus:ring-blue-500 focus:ring-offset-1 dark:focus:ring-offset-gray-800 \
                        disabled:cursor-not-allowed dark:bg-gray-800 min-w-[44px] min-h-[44px] \
                        flex items-center justify-center cursor-pointer"
                    prop:checked=checked
                    disabled=disabled
                    required=required
                    aria-invalid=error.is_some()
                    on:change=move |ev| {
                        let checked = event_target_checked(&ev);
                        on_change.run(checked);
                    }
                />
            </div>
            <label for=checkbox_id class="text-sm text-gray-700 dark:text-gray-300 select-none cursor-pointer pt-0.5">
                {label}
            </label>
            {error.map(|err| view! {
                <p class="text-sm text-red-600 dark:text-red-400" role="alert">
                    {err}
                </p>
            })}
        </div>
    }
}
