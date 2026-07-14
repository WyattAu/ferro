use leptos::ev;
use leptos::prelude::*;

use super::sample_files;
use crate::components::focus_trap::FocusTrap;

#[cfg(target_arch = "wasm32")]
const SETUP_WIZARD_KEY: &str = "ferro_setup_wizard_completed";

pub fn is_setup_completed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(val)) = storage.get_item(SETUP_WIZARD_KEY) {
                    return val == "true";
                }
            }
        }
    }
    false
}

pub fn complete_setup() {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item(SETUP_WIZARD_KEY, "true");
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SetupStep {
    Welcome,
    AdminAccount,
    StorageBackend,
    AuthSetup,
    SampleFiles,
    QuickStart,
}

impl SetupStep {
    fn index(&self) -> usize {
        match self {
            SetupStep::Welcome => 0,
            SetupStep::AdminAccount => 1,
            SetupStep::StorageBackend => 2,
            SetupStep::AuthSetup => 3,
            SetupStep::SampleFiles => 4,
            SetupStep::QuickStart => 5,
        }
    }

    fn total() -> usize {
        6
    }
}

#[component]
pub fn SetupWizard() -> impl IntoView {
    let (visible, set_visible) = signal(is_setup_completed());
    let (step, set_step) = signal(SetupStep::Welcome);
    let (admin_username, set_admin_username) = signal(String::new());
    let (admin_email, set_admin_email) = signal(String::new());
    let (admin_password, set_admin_password) = signal(String::new());
    let (storage_backend, set_storage_backend) = signal("local".to_string());
    let (auth_enabled, set_auth_enabled) = signal(false);
    let (create_samples, set_create_samples) = signal(true);
    let (_current_step_val, set_current_step_val) = signal(SetupStep::Welcome);
    let (progress, set_progress) = signal(0u32);

    Effect::new(move |_| {
        set_visible.set(is_setup_completed());
    });

    let advance = move |next: SetupStep| {
        set_step.set(next);
        set_current_step_val.set(next);
        let pct = ((next.index() + 1) as f64 / SetupStep::total() as f64 * 100.0) as u32;
        set_progress.set(pct);
    };

    let finish_setup = move |_: ev::MouseEvent| {
        complete_setup();
        set_visible.set(false);
    };

    let skip = move |_: ev::MouseEvent| {
        complete_setup();
        set_visible.set(false);
    };

    let handle_escape = move |ev: ev::KeyboardEvent| {
        if ev.key() == "Escape" {
            complete_setup();
            set_visible.set(false);
        }
    };

    view! {
        {move || (!visible.get() && !is_setup_completed()).then(|| view! {
            <div class="fixed inset-0 z-[110] flex items-center justify-center p-4" on:keydown=handle_escape>
                <div class="absolute inset-0 bg-black/70 backdrop-blur-sm" on:click=move |_| { complete_setup(); set_visible.set(false); }></div>
                <FocusTrap>
                <div
                    class="relative bg-[var(--bg-surface)] bg-[var(--bg-base)] rounded-lg shadow-2xl w-full sm:w-[560px] max-h-[90vh] overflow-y-auto"
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="setup-wizard-title"
                    tabindex="-1"
                >
                    <div class="p-6 sm:p-8">
                        <div class="flex justify-between items-center mb-6">
                            <div class="flex items-center gap-2">
                                <svg class="w-8 h-8 text-orange-600" aria-hidden="true" viewBox="0 0 24 24" fill="currentColor">
                                    <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
                                </svg>
                                <span id="setup-wizard-title" class="font-bold text-lg font-mono text-[var(--text-primary)]">Ferro Setup</span>
                            </div>
                            <button
                                class="text-sm text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] transition-colors focus:outline-none focus:ring-2 focus:ring-orange-500 rounded px-2 py-1 font-mono min-w-[44px] min-h-[44px] flex items-center justify-center"
                                on:click=skip
                                aria-label="Skip setup wizard"
                            >
                                "Skip setup"
                            </button>
                        </div>

                        // Progress bar
                        <div class="w-full bg-[var(--border-default)] rounded h-2 mb-6" role="progressbar" aria-valuenow=move || progress.get() aria-valuemin="0" aria-valuemax="100" aria-label="Setup progress">
                            <div
                                class="bg-orange-600 h-2 rounded transition-all duration-300"
                                style=move || format!("width: {}%", progress.get())
                            ></div>
                        </div>

                        {move || {
                            let s = step.get();
                            match s {
                                SetupStep::Welcome => view! {
                                    <div class="text-center py-4">
                                        <svg class="w-16 h-16 text-orange-600 mx-auto mb-4" aria-hidden="true" viewBox="0 0 24 24" fill="currentColor">
                                            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
                                        </svg>
                                        <h2 class="text-2xl font-bold font-mono text-[var(--text-primary)] mb-2">
                                            "Welcome to Ferro"
                                        </h2>
                                        <p class="text-[var(--text-secondary)] font-mono text-sm mb-6 leading-relaxed">
                                            "Your personal distributed storage solution. This wizard will help you get started in just a few steps."
                                        </p>
                                        <div class="grid grid-cols-3 gap-4 mb-6 text-left">
                                            <div class="p-3 rounded bg-[var(--bg-surface-sunken)]">
                                                <div class="text-orange-600 font-bold font-mono text-sm">"Step 1"</div>
                                                <div class="text-xs text-[var(--text-tertiary)] font-mono">"Admin account"</div>
                                            </div>
                                            <div class="p-3 rounded bg-[var(--bg-surface-sunken)]">
                                                <div class="text-orange-600 font-bold font-mono text-sm">"Step 2"</div>
                                                <div class="text-xs text-[var(--text-tertiary)] font-mono">"Storage backend"</div>
                                            </div>
                                            <div class="p-3 rounded bg-[var(--bg-surface-sunken)]">
                                                <div class="text-orange-600 font-bold font-mono text-sm">"Step 3"</div>
                                                <div class="text-xs text-[var(--text-tertiary)] font-mono">"Authentication"</div>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any(),

                                SetupStep::AdminAccount => view! {
                                    <div class="py-4">
                                        <div class="text-center mb-6">
                                            <svg class="w-10 h-10 text-orange-600 mx-auto mb-2" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                                            </svg>
                                            <h3 class="text-lg font-bold font-mono text-[var(--text-primary)]">"Create Admin Account"</h3>
                                            <p class="text-sm text-[var(--text-tertiary)] font-mono mt-1">"Set up your administrator credentials"</p>
                                        </div>
                                        <div class="space-y-4">
                                            <div>
                                                <label for="setup-username" class="block text-sm font-mono text-[var(--text-secondary)] mb-1">"Username"</label>
                                                <input
                                                    id="setup-username"
                                                    type="text"
                                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded font-mono text-sm bg-[var(--bg-surface)] dark:text-[var(--text-on-accent)] focus:outline-none focus:ring-2 focus:ring-orange-500"
                                                    placeholder="admin"
                                                    prop:value=move || admin_username.get()
                                                    on:input=move |ev| set_admin_username.set(event_target_value(&ev))
                                                />
                                            </div>
                                            <div>
                                                <label for="setup-email" class="block text-sm font-mono text-[var(--text-secondary)] mb-1">"Email"</label>
                                                <input
                                                    id="setup-email"
                                                    type="email"
                                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded font-mono text-sm bg-[var(--bg-surface)] dark:text-[var(--text-on-accent)] focus:outline-none focus:ring-2 focus:ring-orange-500"
                                                    placeholder="admin@example.com"
                                                    prop:value=move || admin_email.get()
                                                    on:input=move |ev| set_admin_email.set(event_target_value(&ev))
                                                />
                                            </div>
                                            <div>
                                                <label for="setup-password" class="block text-sm font-mono text-[var(--text-secondary)] mb-1">"Password"</label>
                                                <input
                                                    id="setup-password"
                                                    type="password"
                                                    class="w-full px-3 py-2 border border-[var(--border-default)] rounded font-mono text-sm bg-[var(--bg-surface)] dark:text-[var(--text-on-accent)] focus:outline-none focus:ring-2 focus:ring-orange-500"
                                                    placeholder="Enter a strong password"
                                                    prop:value=move || admin_password.get()
                                                    on:input=move |ev| set_admin_password.set(event_target_value(&ev))
                                                />
                                            </div>
                                        </div>
                                    </div>
                                }.into_any(),

                                SetupStep::StorageBackend => view! {
                                    <div class="py-4">
                                        <div class="text-center mb-6">
                                            <svg class="w-10 h-10 text-orange-600 mx-auto mb-2" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
                                            </svg>
                                            <h3 class="text-lg font-bold font-mono text-[var(--text-primary)]">"Storage Backend"</h3>
                                            <p class="text-sm text-[var(--text-tertiary)] font-mono mt-1">"Choose where to store your files"</p>
                                        </div>
                                        <div class="space-y-3">
                                            {[
                                                ("local", "Local Filesystem", "Store files on this machine's disk"),
                                                ("s3", "AWS S3 / Compatible", "Use S3-compatible object storage"),
                                            ].into_iter().map(|(key, name, desc)| {
                                                let k = key.to_string();
                                                let n = name.to_string();
                                                let d = desc.to_string();
                                                let k_for_class = k.clone();
                                                view! {
                                                    <button
                                                        class=move || format!(
                                                            "w-full p-4 text-left rounded-lg border-2 transition-all font-mono min-h-[44px] {}",
                                                            if storage_backend.get() == k_for_class { "border-orange-500 bg-orange-50 dark:bg-orange-900/20" } else { "border-[var(--border-default)] hover:border-[var(--border-default)]" }
                                                        )
                                                        on:click=move |_| set_storage_backend.set(k.clone())
                                                    >
                                                        <div class="font-bold text-sm text-[var(--text-primary)]">{n}</div>
                                                        <div class="text-xs text-[var(--text-tertiary)] mt-1">{d}</div>
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                }.into_any(),

                                SetupStep::AuthSetup => view! {
                                    <div class="py-4">
                                        <div class="text-center mb-6">
                                            <svg class="w-10 h-10 text-orange-600 mx-auto mb-2" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
                                            </svg>
                                            <h3 class="text-lg font-bold font-mono text-[var(--text-primary)]">"Authentication"</h3>
                                            <p class="text-sm text-[var(--text-tertiary)] font-mono mt-1">"Optionally require login for file access"</p>
                                        </div>
                                        <div class="space-y-4">
                                            <label for="setup-auth-toggle" class="flex items-center gap-3 p-4 rounded-lg border border-[var(--border-default)] cursor-pointer hover:bg-[var(--bg-inset)] dark:hover:bg-[var(--interactive-hover)] transition-colors">
                                                <input
                                                    id="setup-auth-toggle"
                                                    type="checkbox"
                                                    class="w-5 h-5 rounded border-[var(--border-default)] text-orange-600 focus:ring-orange-500"
                                                    prop:checked=move || auth_enabled.get()
                                                    on:change=move |ev| set_auth_enabled.set(event_target_checked(&ev))
                                                />
                                                <div>
                                                    <div class="font-mono text-sm font-bold text-[var(--text-primary)]">"Enable authentication"</div>
                                                    <div class="font-mono text-xs text-[var(--text-tertiary)] mt-0.5">"Require users to log in before accessing files"</div>
                                                </div>
                                            </label>
                                            <div class="p-4 rounded-lg bg-[var(--bg-surface-sunken)] font-mono text-xs text-[var(--text-secondary)]" aria-live="polite">
                                                {move || if auth_enabled.get() {
                                                    "Authentication will be configured. You can add users from the admin panel after setup."
                                                } else {
                                                    "No authentication. All files will be publicly accessible on this network."
                                                }}
                                            </div>
                                        </div>
                                    </div>
                                }.into_any(),

                                SetupStep::SampleFiles => view! {
                                    <div class="py-4">
                                        <div class="text-center mb-6">
                                            <svg class="w-10 h-10 text-orange-600 mx-auto mb-2" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                                            </svg>
                                            <h3 class="text-lg font-bold font-mono text-[var(--text-primary)]">"Sample Folders"</h3>
                                            <p class="text-sm text-[var(--text-tertiary)] font-mono mt-1">"Create a starter folder structure"</p>
                                        </div>
                                        <div class="space-y-3">
                                            <label for="setup-samples-toggle" class="flex items-center gap-3 p-4 rounded-lg border border-[var(--border-default)] cursor-pointer hover:bg-[var(--bg-inset)] dark:hover:bg-[var(--interactive-hover)] transition-colors">
                                                <input
                                                    id="setup-samples-toggle"
                                                    type="checkbox"
                                                    class="w-5 h-5 rounded border-[var(--border-default)] text-orange-600 focus:ring-orange-500"
                                                    prop:checked=move || create_samples.get()
                                                    on:change=move |ev| set_create_samples.set(event_target_checked(&ev))
                                                />
                                                <div>
                                                    <div class="font-mono text-sm font-bold text-[var(--text-primary)]">"Create sample folders"</div>
                                                    <div class="font-mono text-xs text-[var(--text-tertiary)] mt-0.5">"Adds Documents, Photos, Videos, Music, Shared with README files"</div>
                                                </div>
                                            </label>
                                            <div class="p-4 rounded-lg bg-[var(--bg-surface-sunken)]">
                                                <div class="font-mono text-xs text-[var(--text-tertiary)] mb-2">"Folders that will be created:"</div>
                                                <div class="grid grid-cols-2 gap-2 font-mono text-xs">
                                                    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
                                                        <svg class="w-4 h-4 text-orange-500" aria-hidden="true" fill="currentColor" viewBox="0 0 24 24"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
                                                        "Documents"
                                                    </div>
                                                    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
                                                        <svg class="w-4 h-4 text-orange-500" aria-hidden="true" fill="currentColor" viewBox="0 0 24 24"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
                                                        "Photos"
                                                    </div>
                                                    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
                                                        <svg class="w-4 h-4 text-orange-500" aria-hidden="true" fill="currentColor" viewBox="0 0 24 24"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
                                                        "Videos"
                                                    </div>
                                                    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
                                                        <svg class="w-4 h-4 text-orange-500" aria-hidden="true" fill="currentColor" viewBox="0 0 24 24"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
                                                        "Music"
                                                    </div>
                                                    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
                                                        <svg class="w-4 h-4 text-orange-500" aria-hidden="true" fill="currentColor" viewBox="0 0 24 24"><path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/></svg>
                                                        "Shared"
                                                    </div>
                                                    <div class="flex items-center gap-2 text-[var(--text-secondary)]">
                                                        <svg class="w-4 h-4 text-[var(--text-tertiary)]" aria-hidden="true" fill="currentColor" viewBox="0 0 24 24"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6z"/></svg>
                                                        "Welcome.txt"
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any(),

                                SetupStep::QuickStart => view! {
                                    <div class="py-4">
                                        <div class="text-center mb-6">
                                            <svg class="w-10 h-10 text-orange-600 mx-auto mb-2" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
                                            </svg>
                                            <h3 class="text-lg font-bold font-mono text-[var(--text-primary)]">"Quick Start Guide"</h3>
                                            <p class="text-sm text-[var(--text-tertiary)] font-mono mt-1">"You're all set! Here's how to get started."</p>
                                        </div>
                                        <div class="space-y-4">
                                            <div class="p-4 rounded-lg bg-[var(--bg-surface-sunken)]">
                                                <div class="font-mono text-sm font-bold text-orange-600 mb-2">"Upload Files"</div>
                                                <p class="font-mono text-xs text-[var(--text-secondary)] leading-relaxed">
                                                    "Drag and drop files onto the browser window, or use the upload button in the toolbar. Files are uploaded directly to your configured storage backend."
                                                </p>
                                            </div>
                                            <div class="p-4 rounded-lg bg-[var(--bg-surface-sunken)]">
                                                <div class="font-mono text-sm font-bold text-orange-600 mb-2">"Organize with Folders"</div>
                                                <p class="font-mono text-xs text-[var(--text-secondary)] leading-relaxed">
                                                    "Create folders using the new folder button. Navigate between folders by clicking or using the breadcrumb trail at the top."
                                                </p>
                                            </div>
                                            <div class="p-4 rounded-lg bg-[var(--bg-surface-sunken)]">
                                                <div class="font-mono text-sm font-bold text-orange-600 mb-2">"Admin Dashboard"</div>
                                                <p class="font-mono text-xs text-[var(--text-secondary)] leading-relaxed">
                                                    "Access the admin panel at /ui/admin to monitor storage, manage users, and configure server settings."
                                                </p>
                                            </div>
                                            <div class="p-4 rounded-lg bg-[var(--bg-surface-sunken)]">
                                                <div class="font-mono text-sm font-bold text-orange-600 mb-2">"Keyboard Shortcuts"</div>
                                                <p class="font-mono text-xs text-[var(--text-secondary)] leading-relaxed">
                                                    "Press ? to view all available keyboard shortcuts. Navigate with arrow keys, select with Space, and delete with Del."
                                                </p>
                                            </div>
                                        </div>
                                    </div>
                                }.into_any(),
                            }
                        }}

                        // Navigation buttons
                        <div class="flex items-center justify-between gap-3 mt-6 pt-4 border-t border-[var(--border-default)]">
                            <button
                                class=move || format!(
                                    "px-4 py-2 text-sm rounded font-mono transition-colors focus:outline-none focus:ring-2 focus:ring-orange-500 min-h-[44px] {}",
                                    if step.get() == SetupStep::Welcome { "invisible" } else { "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-inset)] dark:hover:bg-[var(--interactive-hover)]" }
                                )
                                on:click=move |_| {
                                    let s = step.get();
                                    let idx = s.index();
                                    if idx > 0 {
                                        let steps = [
                                            SetupStep::Welcome,
                                            SetupStep::AdminAccount,
                                            SetupStep::StorageBackend,
                                            SetupStep::AuthSetup,
                                            SetupStep::SampleFiles,
                                            SetupStep::QuickStart,
                                        ];
                                        advance(steps[idx - 1]);
                                    }
                                }
                            >
                                "Back"
                            </button>

                            <div class="flex gap-1.5" role="group" aria-label="Setup step indicators">
                                {(0..SetupStep::total()).map(|i| view! {
                                    <div
                                        class=move || format!(
                                            "w-2 h-2 rounded-full transition-colors {}",
                                            if i == step.get().index() { "bg-orange-500" } else { "bg-[var(--text-tertiary)]" }
                                        )
                                    ></div>
                                }).collect::<Vec<_>>()}
                            </div>

                            <button
                                class=move || format!(
                                    "px-5 py-2 text-sm font-medium font-mono rounded transition-colors focus:outline-none focus:ring-2 focus:ring-orange-500 min-h-[44px] {}",
                                    if step.get() == SetupStep::QuickStart {
                                        "bg-[var(--success)] text-[var(--text-on-accent)] hover:bg-[var(--success-hover)]"
                                    } else {
                                        "bg-orange-600 text-[var(--text-on-accent)] hover:bg-orange-700"
                                    }
                                )
                                on:click=move |ev| {
                                    let s = step.get();
                                    if s == SetupStep::QuickStart {
                                        if create_samples.get() {
                                            sample_files::create_sample_folders();
                                        }
                                        finish_setup(ev);
                                    } else {
                                        let idx = s.index();
                                        let steps = [
                                            SetupStep::Welcome,
                                            SetupStep::AdminAccount,
                                            SetupStep::StorageBackend,
                                            SetupStep::AuthSetup,
                                            SetupStep::SampleFiles,
                                            SetupStep::QuickStart,
                                        ];
                                        if idx + 1 < steps.len() {
                                            advance(steps[idx + 1]);
                                        }
                                    }
                                }
                            >
                                {move || if step.get() == SetupStep::QuickStart { "Get Started" } else { "Next" }}
                            </button>
                        </div>
                    </div>
                </div>
                </FocusTrap>
            </div>
        })}
    }
}
