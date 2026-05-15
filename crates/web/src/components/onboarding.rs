use leptos::*;

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
            OnboardingStep::Welcome => "Welcome to Ferro",
            OnboardingStep::Upload => "Upload Files",
            OnboardingStep::Organize => "Organize",
            OnboardingStep::Share => "Share",
            OnboardingStep::Settings => "Settings",
            OnboardingStep::Shortcuts => "Keyboard Shortcuts",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => {
                "Ferro is your personal storage orchestrator. Upload, organize, share, and manage your files with ease. Let us show you around."
            }
            OnboardingStep::Upload => {
                "Click the Upload button or drag and drop files directly into the browser to upload them. You can upload multiple files at once."
            }
            OnboardingStep::Organize => {
                "Use the New Folder button to create directories. Navigate using breadcrumbs or the parent directory button to keep your files organized."
            }
            OnboardingStep::Share => {
                "Share any file with others using the share button. Create time-limited links with optional password protection."
            }
            OnboardingStep::Settings => {
                "Access Settings from the gear icon in the header. Customize your theme, default view, sort order, and other preferences."
            }
            OnboardingStep::Shortcuts => {
                "Press Ctrl+K to open the command palette for quick access to actions. Use Ctrl+F to search, Ctrl+A to select all, and more."
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
    let (step, set_step) = create_signal(OnboardingStep::Welcome);
    let (visible, set_visible) = create_signal(false);

    create_effect(move |_| {
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
                <svg class="w-12 h-12 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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
                <div class="relative brutal-block rounded shadow-2xl w-[calc(100%-2rem)] sm:w-[480px] max-h-[90vh] overflow-y-auto transition-all duration-200 scale-100 opacity-100">
                    <div class="p-6 sm:p-8">
                        <div class="flex justify-end mb-2">
                            <button
                                class="text-sm text-gray-400 hover:text-gray-600 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 rounded px-2 py-1 font-mono text-label"
                                on:click=handle_skip
                            >
                                "Skip tour"
                            </button>
                        </div>

                        <div class="text-center mb-6">
                            {step_icon(current_step())}
                            <h2 class="text-xl font-bold font-mono text-gray-900 mt-4 tracking-tight">
                                {current_step().title()}
                            </h2>
                            <div class="w-full bg-gray-200 rounded-sm h-3 mt-4">
                                <div
                                    class="bg-blue-600 h-3 rounded-sm transition-all duration-300"
                                    style=move || format!("width: {}%", progress())
                                ></div>
                            </div>
                            <span class="text-xs text-gray-400 mt-1 block font-mono">
                                {move || format!("Step {} of {}", current_step().index() + 1, OnboardingStep::total())}
                            </span>
                        </div>

                        <p class="text-sm text-gray-600 text-center leading-relaxed mb-8 font-mono">
                            {current_step().description()}
                        </p>

                        <div class="flex items-center justify-between gap-3">
                            <button
                                class=move || format!(
                                    "px-4 py-2 text-sm rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}",
                                    if is_first() { "invisible" } else { "text-gray-600 hover:text-gray-800 hover:bg-gray-100" }
                                )
                                on:click=handle_back
                                disabled=is_first()
                            >
                                "Back"
                            </button>

                            <div class="flex gap-1.5">
                                {(0..OnboardingStep::total()).map(|i| view! {
                                    <div
                                        class=move || format!(
                                            "w-2 h-2 rounded-full transition-colors {}",
                                            if i == current_step().index() { "bg-blue-500" } else { "bg-gray-300 dark:bg-gray-600" }
                                        )
                                    ></div>
                                }).collect::<Vec<_>>()
                                }
                            </div>

                            <button
                                class=move || format!(
                                    "px-5 py-2 text-sm font-medium rounded transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 brutal-border font-bold uppercase {}",
                                    if is_last() { "bg-green-600 text-white hover:bg-green-700" } else { "bg-blue-600 text-white hover:bg-blue-700" }
                                )
                                on:click=handle_next
                            >
                                {move || if is_last() { "Get Started" } else { "Next" }}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        })}
    }
}
