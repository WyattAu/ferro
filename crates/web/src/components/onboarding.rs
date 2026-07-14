use leptos::ev;
use leptos::prelude::*;

use crate::components::focus_trap::FocusTrap;
use crate::t;

#[allow(dead_code)] // Used by WASM runtime
const ONBOARDING_KEY: &str = "ferro_onboarding_completed";

#[derive(Clone, Copy, PartialEq, Eq)]
enum OnboardingStep {
    Welcome,
    Upload,
    Organize,
    Share,
    Settings,
    Shortcuts,
}

impl OnboardingStep {
    fn index(&self) -> usize {
        match self {
            OnboardingStep::Welcome => 0,
            OnboardingStep::Upload => 1,
            OnboardingStep::Organize => 2,
            OnboardingStep::Share => 3,
            OnboardingStep::Settings => 4,
            OnboardingStep::Shortcuts => 5,
        }
    }

    fn total() -> usize {
        6
    }

    fn title(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => t!("onboarding.step_1_title"),
            OnboardingStep::Upload => t!("onboarding.step_2_title"),
            OnboardingStep::Organize => t!("onboarding.step_3_title"),
            OnboardingStep::Share => t!("onboarding.step_4_title"),
            OnboardingStep::Settings => t!("onboarding.step_5_title"),
            OnboardingStep::Shortcuts => t!("onboarding.step_6_title"),
        }
    }

    fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => {
                t!("onboarding.step_1_desc")
            }
            OnboardingStep::Upload => {
                t!("onboarding.step_2_desc")
            }
            OnboardingStep::Organize => {
                t!("onboarding.step_3_desc")
            }
            OnboardingStep::Share => {
                t!("onboarding.step_4_desc")
            }
            OnboardingStep::Settings => {
                t!("onboarding.step_5_desc")
            }
            OnboardingStep::Shortcuts => {
                t!("onboarding.step_6_desc")
            }
        }
    }
}

pub fn is_onboarding_completed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(val)) = storage.get_item(ONBOARDING_KEY) {
                    return val == "true";
                }
            }
        }
    }
    true
}

pub fn reset_onboarding() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(ONBOARDING_KEY, "false");
            }
        }
    }
}

pub fn complete_onboarding() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(ONBOARDING_KEY, "true");
            }
        }
    }
}

#[component]
pub fn OnboardingOverlay() -> impl IntoView {
    let (step, set_step) = signal(OnboardingStep::Welcome);
    let (visible, set_visible) = signal(false);

    Effect::new(move |_| {
        set_visible.set(!is_onboarding_completed());
    });

    let current_step = move || step.get();
    let progress = move || {
        let s = current_step();
        ((s.index() + 1) as f64 / OnboardingStep::total() as f64 * 100.0) as u32
    };

    let handle_next = move |_: ev::MouseEvent| {
        let s = current_step();
        if s == OnboardingStep::Shortcuts {
            complete_onboarding();
            set_visible.set(false);
        } else {
            let next_idx = s.index() + 1;
            let steps = [
                OnboardingStep::Welcome,
                OnboardingStep::Upload,
                OnboardingStep::Organize,
                OnboardingStep::Share,
                OnboardingStep::Settings,
                OnboardingStep::Shortcuts,
            ];
            if next_idx < steps.len() {
                set_step.set(steps[next_idx]);
            }
        }
    };

    let handle_back = move |_: ev::MouseEvent| {
        let s = current_step();
        if s.index() > 0 {
            let prev_idx = s.index() - 1;
            let steps = [
                OnboardingStep::Welcome,
                OnboardingStep::Upload,
                OnboardingStep::Organize,
                OnboardingStep::Share,
                OnboardingStep::Settings,
                OnboardingStep::Shortcuts,
            ];
            set_step.set(steps[prev_idx]);
        }
    };

    let handle_skip = move |_: ev::MouseEvent| {
        complete_onboarding();
        set_visible.set(false);
    };

    let is_first = move || current_step() == OnboardingStep::Welcome;
    let is_last = move || current_step() == OnboardingStep::Shortcuts;

    let step_icon = move |s: OnboardingStep| {
        match s {
            OnboardingStep::Welcome => view! {
                <svg class="w-12 h-12 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6" />
                </svg>
            }.into_any(),
            OnboardingStep::Upload => view! {
                <svg class="w-12 h-12 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                </svg>
            }.into_any(),
            OnboardingStep::Organize => view! {
                <svg class="w-12 h-12 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
            }.into_any(),
            OnboardingStep::Share => view! {
                <svg class="w-12 h-12 text-indigo-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8.684 13.342C8.886 12.938 9 12.482 9 12c0-.482-.114-.938-.316-1.342m0 2.684a3 3 0 110-2.684m0 2.684l6.632 3.316m-6.632-6l6.632-3.316m0 0a3 3 0 105.367-2.684 3 3 0 00-5.367 2.684zm0 9.316a3 3 0 105.368 2.684 3 3 0 00-5.368-2.684z" />
                </svg>
            }.into_any(),
            OnboardingStep::Settings => view! {
                    <svg class="w-12 h-12 text-[var(--text-tertiary)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
            }.into_any(),
            OnboardingStep::Shortcuts => view! {
                <svg class="w-12 h-12 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                </svg>
            }.into_any(),
        }
    };

    view! {
        {move || visible.get().then(|| view! {
            <div class="fixed inset-0 z-[100] flex items-center justify-center p-4">
                <div class="absolute inset-0 bg-black/60 backdrop-blur-sm" on:click=handle_skip></div>
                <FocusTrap>
                <div class="relative brutal-block rounded shadow-2xl w-[calc(100%-2rem)] sm:w-[480px] max-h-[90vh] overflow-y-auto transition-all duration-200 scale-100 opacity-100" role="dialog" aria-modal="true" aria-label=t!("onboarding.aria") tabindex="-1">
                    <div class="p-6 sm:p-8">
                        <div class="flex justify-end mb-2">
                            <button
                                class="text-sm text-[var(--text-tertiary)] hover:text-gray-600 transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded px-2 py-1 font-mono text-label"
                                on:click=handle_skip
                            >
                                {t!("onboarding.skip_tour")}
                            </button>
                        </div>

                        <div class="text-center mb-6">
                            {step_icon(current_step())}
                            <h2 class="text-xl font-bold font-mono text-gray-900 mt-4 tracking-tight">
                                {current_step().title()}
                            </h2>
                            <div class="w-full bg-[var(--border-default)] rounded-sm h-3 mt-4">
                                <div
                                    class="bg-[var(--accent)] h-3 rounded-sm transition-all duration-300"
                                    style=move || format!("width: {}%", progress())
                                ></div>
                            </div>
                            <span class="text-xs text-[var(--text-tertiary)] mt-1 block font-mono">
                                {move || format!("Step {} of {}", current_step().index() + 1, OnboardingStep::total())}
                            </span>
                        </div>

                        <p class="text-sm text-[var(--text-secondary)] text-center leading-relaxed mb-8 font-mono">
                            {current_step().description()}
                        </p>

                        <div class="flex items-center justify-between gap-3">
                            <button
                                class=move || format!(
                                    "px-4 py-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}",
                                    if is_first() { "invisible" } else { "text-[var(--text-secondary)] hover:text-gray-800 hover:bg-gray-100" }
                                )
                                on:click=handle_back
                                disabled=is_first()
                            >
                                {t!("common.back")}
                            </button>

                            <div class="flex gap-1.5">
                                {(0..OnboardingStep::total()).map(|i| view! {
                                    <div
                                        class=move || format!(
                                            "w-2 h-2 rounded-full transition-colors {}",
                                            if i == current_step().index() { "bg-blue-500" } else { "bg-[var(--text-tertiary)]" }
                                        )
                                    ></div>
                                }).collect::<Vec<_>>()
                                }
                            </div>

                            <button
                                class=move || format!(
                                    "px-5 py-2 text-sm font-medium rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] focus:ring-offset-2 dark:focus:ring-offset-gray-800 brutal-border font-bold uppercase {}",
                                    if is_last() { "bg-green-600 text-white hover:bg-green-700" } else { "bg-[var(--accent)] text-[var(--text-on-accent)] hover:bg-blue-700" }
                                )
                                on:click=handle_next
                            >
                                {move || if is_last() { t!("onboarding.get_started") } else { t!("common.next") }}
                            </button>
                        </div>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}
