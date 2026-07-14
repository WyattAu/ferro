use leptos::ev;
use leptos::prelude::*;

/// Accessible button component with proper ARIA attributes and minimum touch target.
///
/// Supports 7 variants: Primary, Secondary, Danger, Ghost, Link, Outline, Soft.
#[component]
pub fn Button(
    #[prop(default = ButtonVariant::Primary)] variant: ButtonVariant,
    #[prop(default = ButtonSize::Md)] size: ButtonSize,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] loading: bool,
    #[prop(default = None)] aria_label: Option<String>,
    #[prop(default = "button".to_string())] button_type: String,
    on_click: Option<Callback<ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    let base = "inline-flex items-center justify-center font-medium transition-colors \
        focus:outline-none focus:ring-2 focus:ring-offset-2 \
        disabled:opacity-50 disabled:cursor-not-allowed min-w-[44px] min-h-[44px]";

    let variant_class = match variant {
        ButtonVariant::Primary => {
            "bg-[var(--accent)] text-[var(--text-on-accent)] hover:bg-[var(--accent-hover)] focus:ring-[var(--border-focus)]"
        }
        ButtonVariant::Secondary => {
            "bg-[var(--bg-surface-raised)] text-[var(--text-primary)] border border-[var(--border-default)] hover:bg-[var(--interactive-hover)] hover:border-[var(--border-strong)] focus:ring-[var(--border-focus)]"
        }
        ButtonVariant::Danger => {
            "bg-[var(--danger)] text-white hover:bg-[var(--danger-hover)] focus:ring-[var(--danger)]"
        }
        ButtonVariant::Ghost => {
            "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--interactive-hover)] focus:ring-[var(--border-focus)]"
        }
        ButtonVariant::Link => {
            "text-[var(--accent)] hover:underline focus:ring-[var(--border-focus)] p-0 min-w-0 min-h-0"
        }
        ButtonVariant::Outline => {
            "border border-[var(--border-default)] text-[var(--text-primary)] bg-transparent hover:bg-[var(--interactive-hover)] hover:border-[var(--border-strong)] focus:ring-[var(--border-focus)]"
        }
        ButtonVariant::Soft => {
            "bg-[var(--accent-subtle)] text-[var(--accent)] hover:bg-[var(--accent-muted)] focus:ring-[var(--border-focus)]"
        }
    };

    let size_class = match size {
        ButtonSize::Sm => "px-3 py-1.5 text-sm rounded-md",
        ButtonSize::Md => "px-4 py-2 text-sm rounded-md",
        ButtonSize::Lg => "px-6 py-3 text-base rounded-lg",
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Link,
    Outline,
    Soft,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonSize {
    Sm,
    Md,
    Lg,
}

/// Accessible input component with label, error, and help text support.
#[component]
pub fn Input(
    #[prop(into)] id: String,
    #[prop(default = None)] label: Option<String>,
    #[prop(default = None)] aria_label: Option<String>,
    #[prop(default = "text".to_string())] input_type: String,
    #[prop(default = None)] placeholder: Option<String>,
    #[prop(default = None)] value: Option<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] required: bool,
    #[prop(default = None)] error: Option<String>,
    #[prop(default = None)] help_text: Option<String>,
    #[prop(default = None)] on_input: Option<Callback<String>>,
    #[prop(default = None)] on_blur: Option<Callback<ev::FocusEvent>>,
    #[prop(default = None)] class: Option<String>,
) -> impl IntoView {
    let input_id = id.clone();
    let error_id = format!("{}-error", id);
    let help_id = format!("{}-help", id);
    let has_error = error.is_some();

    let base_input_class = "block w-full rounded-md border px-3 py-2 text-sm \
        placeholder-[var(--text-tertiary)] \
        focus:outline-none focus:ring-2 focus:ring-offset-1 \
        disabled:bg-[var(--bg-surface-sunken)] disabled:cursor-not-allowed \
        min-h-[44px] transition-colors";

    let border_class = if has_error {
        "border-[var(--danger)] bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-[var(--danger)] focus:border-[var(--danger)]"
    } else {
        "border-[var(--border-default)] bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-[var(--border-focus)] focus:border-[var(--border-focus)]"
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
        if ids.is_empty() { None } else { Some(ids.join(" ")) }
    };

    let on_input = on_input.unwrap_or_else(|| Callback::new(|_| {}));
    let on_blur = on_blur.unwrap_or_else(|| Callback::new(|_| {}));

    view! {
        <div class="w-full">
            {label.map(|label_text| view! {
                <label for=input_id.clone() class="block text-sm font-medium text-[var(--text-primary)] mb-1.5">
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
                <p id=error_id class="mt-1.5 text-sm text-[var(--danger)]" role="alert">
                    {err}
                </p>
            })}
            {help_text.map(|help| view! {
                <p id=help_id class="mt-1.5 text-sm text-[var(--text-tertiary)]">
                    {help}
                </p>
            })}
        </div>
    }
}

/// Accessible select/dropdown component.
#[component]
pub fn Select(
    #[prop(into)] id: String,
    #[prop(default = None)] label: Option<String>,
    #[prop(default = None)] aria_label: Option<String>,
    #[prop(default = None)] value: Option<String>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] required: bool,
    #[prop(default = None)] error: Option<String>,
    #[prop(default = None)] on_change: Option<Callback<String>>,
    options: Vec<SelectOption>,
    #[prop(default = None)] class: Option<String>,
) -> impl IntoView {
    let select_id = id.clone();
    let error_id = format!("{}-error", id);
    let has_error = error.is_some();

    let base_class = "block w-full rounded-md border px-3 py-2 text-sm \
        focus:outline-none focus:ring-2 focus:ring-offset-1 \
        disabled:bg-[var(--bg-surface-sunken)] disabled:cursor-not-allowed \
        min-h-[44px] transition-colors";

    let border_class = if has_error {
        "border-[var(--danger)] bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-[var(--danger)] focus:border-[var(--danger)]"
    } else {
        "border-[var(--border-default)] bg-[var(--bg-surface)] text-[var(--text-primary)] focus:ring-[var(--border-focus)] focus:border-[var(--border-focus)]"
    };

    let select_class = match &class {
        Some(c) => format!("{} {} {}", base_class, border_class, c),
        None => format!("{} {}", base_class, border_class),
    };

    let on_change = on_change.unwrap_or_else(|| Callback::new(|_| {}));

    view! {
        <div class="w-full">
            {label.map(|label_text| view! {
                <label for=select_id.clone() class="block text-sm font-medium text-[var(--text-primary)] mb-1.5">
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
                <p id=error_id class="mt-1.5 text-sm text-[var(--danger)]" role="alert">
                    {err}
                </p>
            })}
        </div>
    }
}

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

/// Accessible checkbox with proper label association.
#[component]
pub fn Checkbox(
    #[prop(into)] id: String,
    label: String,
    checked: Signal<bool>,
    on_change: Callback<bool>,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] required: bool,
    #[prop(default = None)] error: Option<String>,
) -> impl IntoView {
    let checkbox_id = id.clone();

    view! {
        <div class="flex items-start gap-2.5">
            <div class="flex items-center h-5">
                <input
                    type="checkbox"
                    id=checkbox_id.clone()
                    class="w-4 h-4 rounded border-[var(--border-default)] text-[var(--accent)] \
                        focus:ring-[var(--border-focus)] focus:ring-offset-1 \
                        disabled:cursor-not-allowed bg-[var(--bg-surface)] min-w-[44px] min-h-[44px] \
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
            <label for=checkbox_id class="text-sm text-[var(--text-primary)] select-none cursor-pointer pt-0.5">
                {label}
            </label>
            {error.map(|err| view! {
                <p class="text-sm text-[var(--danger)]" role="alert">
                    {err}
                </p>
            })}
        </div>
    }
}

/// Card container component.
#[component]
pub fn Card(
    #[prop(default = None)] class: Option<String>,
    #[prop(default = None)] padding: Option<CardPadding>,
    children: Children,
) -> impl IntoView {
    let padding_class = padding
        .map(|p| match p {
            CardPadding::None => "",
            CardPadding::Sm => " p-4",
            CardPadding::Md => " p-6",
            CardPadding::Lg => " p-8",
        })
        .unwrap_or("");

    let base_class = "bg-[var(--bg-surface)] border border-[var(--border-default)] rounded-xl shadow-sm";
    let class_str = match class {
        Some(c) => format!("{} {}{}", base_class, c, padding_class),
        None => format!("{}{}", base_class, padding_class),
    };

    view! {
        <div class=class_str>
            {children()}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardPadding {
    None,
    Sm,
    Md,
    Lg,
}

/// Badge/tag component.
#[component]
pub fn Badge(
    #[prop(default = BadgeVariant::Default)] variant: BadgeVariant,
    #[prop(default = None)] class: Option<String>,
    children: Children,
) -> impl IntoView {
    let base = "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium";
    let variant_class = match variant {
        BadgeVariant::Default => {
            "bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] border border-[var(--border-default)]"
        }
        BadgeVariant::Accent => "bg-[var(--accent-subtle)] text-[var(--accent)]",
        BadgeVariant::Danger => "bg-[var(--danger-subtle)] text-[var(--danger)]",
        BadgeVariant::Success => "bg-[var(--success-subtle)] text-[var(--success)]",
        BadgeVariant::Warning => "bg-[var(--warning-subtle)] text-[var(--warning)]",
    };

    let class_str = match class {
        Some(c) => format!("{} {} {}", base, variant_class, c),
        None => format!("{} {}", base, variant_class),
    };

    view! {
        <span class=class_str>
            {children()}
        </span>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BadgeVariant {
    Default,
    Accent,
    Danger,
    Success,
    Warning,
}

/// Skeleton loading placeholder.
#[component]
pub fn Skeleton(
    #[prop(default = SkeletonVariant::Text)] variant: SkeletonVariant,
    #[prop(default = None)] class: Option<String>,
) -> impl IntoView {
    let base = "skeleton rounded-md";
    let (variant_class, extra_style) = match variant {
        SkeletonVariant::Text => ("h-4 w-full".to_string(), String::new()),
        SkeletonVariant::Heading => ("h-6 w-3/4".to_string(), String::new()),
        SkeletonVariant::Avatar => ("h-10 w-10 rounded-full".to_string(), String::new()),
        SkeletonVariant::Thumbnail => ("h-20 w-20 rounded-lg".to_string(), String::new()),
        SkeletonVariant::Button => ("h-10 w-24 rounded-md".to_string(), String::new()),
        SkeletonVariant::Custom(w, h) => ("rounded-md".to_string(), format!("width: {w}; height: {h};")),
    };

    let class_str = match &class {
        Some(c) => format!("{} {} {}", base, variant_class, c),
        None => format!("{} {}", base, variant_class),
    };

    view! {
        <div class=class_str style=extra_style aria-hidden="true" />
    }
}

#[derive(Clone, Debug)]
pub enum SkeletonVariant {
    Text,
    Heading,
    Avatar,
    Thumbnail,
    Button,
    Custom(String, String),
}

/// Empty state component for when lists/views have no data.
#[component]
pub fn EmptyState(
    /// Icon SVG path
    icon: String,
    /// Title text
    title: String,
    /// Description text
    #[prop(default = None)]
    description: Option<String>,
    /// Optional action button label
    #[prop(default = None)]
    action_label: Option<String>,
    /// Optional action callback
    #[prop(default = None)]
    on_action: Option<Callback<ev::MouseEvent>>,
) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-16 px-4 text-center">
            <div class="w-16 h-16 rounded-full bg-[var(--bg-surface-raised)] flex items-center justify-center mb-4">
                <svg class="w-8 h-8 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d=icon />
                </svg>
            </div>
            <h3 class="text-lg font-semibold text-[var(--text-primary)] mb-1">{title}</h3>
            {description.map(|desc| view! {
                <p class="text-sm text-[var(--text-secondary)] max-w-sm mb-4">{desc}</p>
            })}
            {move || {
                if let (Some(label), Some(cb)) = (&action_label, &on_action) {
                    let label = label.clone();
                    let cb = *cb;
                    view! {
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=Some(cb)
                        >
                            {label}
                        </Button>
                    }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </div>
    }
}
