use leptos::ev;
use leptos::prelude::*;

use crate::components::focus_trap::FocusTrap;
use crate::t;

#[derive(Clone)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub shortcut: Option<String>,
    pub action: Callback<()>,
}

#[derive(Clone, Copy)]
pub struct CommandPaletteState {
    is_open: ReadSignal<bool>,
    set_is_open: WriteSignal<bool>,
    commands: ReadSignal<Vec<Command>>,
    set_commands: WriteSignal<Vec<Command>>,
}

impl CommandPaletteState {
    pub fn open(&self) {
        self.set_is_open.set(true);
    }

    pub fn close(&self) {
        self.set_is_open.set(false);
    }

    pub fn is_open(&self) -> bool {
        self.is_open.get()
    }

    pub fn toggle(&self) {
        let is_open = self.is_open.get();
        self.set_is_open.set(!is_open);
    }

    pub fn set_commands(&self, commands: Vec<Command>) {
        self.set_commands.set(commands);
    }
}

pub fn provide_command_palette_state() -> CommandPaletteState {
    let (is_open, set_is_open) = signal(false);
    let (commands, set_commands) = signal(Vec::<Command>::new());

    let state = CommandPaletteState {
        is_open,
        set_is_open,
        commands,
        set_commands,
    };

    provide_context(state);
    state
}

pub fn use_command_palette_state() -> CommandPaletteState {
    use_context::<CommandPaletteState>().expect("CommandPaletteState not provided")
}

#[component]
pub fn CommandPalette() -> impl IntoView {
    let state = use_command_palette_state();
    let (search, set_search) = signal(String::new());
    let (highlighted_id, set_highlighted_id) = signal(String::new());

    let filtered_commands = move || {
        let query = search.get().to_lowercase();
        let cmds = state.commands.get();
        if query.is_empty() {
            cmds
        } else {
            cmds.into_iter()
                .filter(|c| c.label.to_lowercase().contains(&query))
                .collect::<Vec<_>>()
        }
    };

    Effect::new(move |_| {
        search.get();
        let cmds = filtered_commands();
        if let Some(first) = cmds.first() {
            set_highlighted_id.set(first.id.clone());
        } else {
            set_highlighted_id.set(String::new());
        }
    });

    #[cfg(target_arch = "wasm32")]
    {
        let focus_input = move || {
            let _ = set_timeout_with_handle(
                move || {
                    if let Some(window) = web_sys::window() {
                        if let Some(doc) = window.document() {
                            let sel = "input#command-palette-search";
                            if let Ok(Some(input)) = doc.query_selector(sel) {
                                use wasm_bindgen::JsCast;
                                if let Ok(el) = input.dyn_into::<web_sys::HtmlInputElement>() {
                                    let _ = el.focus();
                                }
                            }
                        }
                    }
                },
                std::time::Duration::from_millis(50),
            );
        };
        Effect::new(move |_| {
            if state.is_open.get() {
                focus_input();
            }
        });
    }

    let handle_keydown = move |ev: ev::KeyboardEvent| match ev.key().as_str() {
        "ArrowDown" => {
            ev.prevent_default();
            let cmds = filtered_commands();
            if cmds.len() <= 1 {
                return;
            }
            let current = highlighted_id.get();
            if let Some(pos) = cmds.iter().position(|c| c.id == current) {
                let next = (pos + 1) % cmds.len();
                set_highlighted_id.set(cmds[next].id.clone());
            }
        }
        "ArrowUp" => {
            ev.prevent_default();
            let cmds = filtered_commands();
            if cmds.len() <= 1 {
                return;
            }
            let current = highlighted_id.get();
            if let Some(pos) = cmds.iter().position(|c| c.id == current) {
                let prev = if pos == 0 { cmds.len() - 1 } else { pos - 1 };
                set_highlighted_id.set(cmds[prev].id.clone());
            }
        }
        "Enter" => {
            ev.prevent_default();
            let cmds = filtered_commands();
            let current = highlighted_id.get();
            if let Some(cmd) = cmds.iter().find(|c| c.id == current) {
                let cmd = cmd.clone();
                state.close();
                cmd.action.run(());
            }
        }
        "Escape" => {
            state.close();
        }
        _ => {}
    };

    view! {
        {move || state.is_open.get().then(|| view! {
            <div
                class="fixed inset-0 bg-black bg-opacity-50 z-[60] flex items-start justify-center pt-[15vh] sm:pt-[20vh] backdrop-blur-sm"
                role="dialog"
                aria-label=t!("command_palette.aria")
                on:click=move |_| state.close()
            >
                <div
                    class="brutal-block rounded shadow-2xl w-[calc(100%-2rem)] sm:w-full sm:max-w-lg mx-auto overflow-hidden"
                    on:click=move |ev| ev.stop_propagation()
                    on:keydown=handle_keydown
                >
                <FocusTrap>
                    <div class="flex items-center border-b border-gray-200 px-4">
                        <svg class="w-5 h-5 text-accent mr-3 shrink-0" aria-hidden="true" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        <input
                            id="command-palette-search"
                            type="text"
                            class="w-full py-3 bg-transparent text-gray-900 placeholder-gray-400 focus:outline-none text-sm font-mono"
                            placeholder=t!("command_palette.placeholder")
                            prop:value=search
                            on:input=move |ev| set_search.set(event_target_value(&ev))
                            aria-label="Search commands"
                        />
                        <kbd class="hidden sm:inline-block px-2 py-0.5 text-xs text-gray-400 bg-gray-100 dark:bg-gray-700 rounded-sm brutal-border ml-2 shrink-0 font-mono">{t!("command_palette.esc")}</kbd>
                    </div>
                    <div class="max-h-64 overflow-y-auto py-1" role="listbox">
                        {move || {
                            let cmds = filtered_commands();
                            if cmds.is_empty() {
                                view! {
                                    <div class="px-4 py-8 text-center text-sm text-gray-500">{t!("empty.commands")}</div>
                                }.into_any()
                            } else {
                                view! {
                                    <div>
                                        <For
                                            each=move || filtered_commands()
                                            key=|cmd| cmd.id.clone()
                                            let:command
                                        >
                                        {
                                            let cmd_id = command.id.clone();
                                            let cmd_id_class = command.id.clone();
                                            let cmd_id_aria = command.id.clone();
                                            let cmd_label = command.label.clone();
                                            let cmd_shortcut = command.shortcut.clone();
                                            let cmd_action = command.action;
                                            let hl_id = highlighted_id;
                                            let set_hl = set_highlighted_id;
                                            let pal_state = state;
                                            view! {
                                                <button
                                                    class=move || format!(
                                                        "w-full flex items-center justify-between px-4 py-2.5 text-sm text-left transition-colors min-h-[44px] {}",
                                                        if hl_id.get() == cmd_id_class {
                                                            "bg-blue-50 dark:bg-blue-900/30 text-accent dark:text-accent border-l-4 border-l-blue-600"
                                                        } else {
                                                            "text-gray-700 hover:bg-gray-100"
                                                        }
                                                    )
                                                    role="option"
                                                    aria_selected=move || hl_id.get() == cmd_id_aria
                                                    on:click=move |_| {
                                                        pal_state.close();
                                                        cmd_action.run(());
                                                    }
                                                    on:mouseenter=move |_| set_hl.set(cmd_id.clone())
                                                >
                                                    <span>{cmd_label}</span>
                                                    {cmd_shortcut.map(|shortcut| view! {
                                                        <span class="text-xs text-gray-400 font-mono ml-4 shrink-0 surface brutal-border px-1.5 py-0.5 rounded-sm">{shortcut}</span>
                                                    })}
                                                </button>
                                            }
                                        }
                                        </For>
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                </FocusTrap>
                </div>
            </div>
        })}
    }
}
