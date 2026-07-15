use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrack {
    pub path: String,
    pub name: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[component]
pub fn AudioPlayer() -> impl IntoView {
    let (is_playing, set_is_playing) = signal(false);
    let (is_muted, set_is_muted) = signal(false);
    let (current_time, set_current_time) = signal(0.0_f64);
    let (duration, set_duration) = signal(0.0_f64);
    let (volume, set_volume) = signal(1.0_f64);
    let (show_queue, set_show_queue) = signal(false);
    let (queue, set_queue) = signal(Vec::<AudioTrack>::new());
    let (current_index, set_current_index) = signal(0usize);
    let (repeat_mode, set_repeat_mode) = signal(RepeatMode::Off);
    let (is_shuffled, set_is_shuffled) = signal(false);

    let audio_ref: NodeRef<leptos::html::Audio> = NodeRef::new();

    let toggle_play = move |_: ev::MouseEvent| {
        if let Some(audio) = audio_ref.get() {
            if audio.paused() {
                let _ = audio.play();
            } else {
                let _ = audio.pause();
            }
        }
    };

    let toggle_mute = move |_: ev::MouseEvent| {
        if let Some(audio) = audio_ref.get() {
            let new_muted = !is_muted.get();
            audio.set_muted(new_muted);
            set_is_muted.set(new_muted);
        }
    };

    let handle_volume = move |ev: ev::Event| {
        if let Some(target) = ev.target()
            && let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>()
            && let Ok(val) = input.value().parse::<f64>()
            && let Some(audio) = audio_ref.get()
        {
            audio.set_volume(val);
            set_volume.set(val);
            if val == 0.0 {
                set_is_muted.set(true);
            } else if is_muted.get() {
                audio.set_muted(false);
                set_is_muted.set(false);
            }
        }
    };

    let handle_seek = move |ev: ev::Event| {
        if let Some(target) = ev.target()
            && let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>()
            && let Ok(val) = input.value().parse::<f64>()
            && let Some(audio) = audio_ref.get()
        {
            audio.set_current_time(val);
            set_current_time.set(val);
        }
    };

    let handle_time_update = move |_: ev::Event| {
        if let Some(audio) = audio_ref.get() {
            set_current_time.set(audio.current_time());
        }
    };

    let handle_loaded_metadata = move |_: ev::Event| {
        if let Some(audio) = audio_ref.get() {
            set_duration.set(audio.duration());
        }
    };

    let handle_ended = move |_: ev::Event| {
        set_is_playing.set(false);
        let mode = repeat_mode.get();
        if mode == RepeatMode::One {
            if let Some(audio) = audio_ref.get() {
                audio.set_current_time(0.0);
                let _ = audio.play();
            }
        } else {
            let q = queue.get();
            let idx = current_index.get();
            if idx + 1 < q.len() {
                set_current_index.set(idx + 1);
            } else if mode == RepeatMode::All && !q.is_empty() {
                set_current_index.set(0);
            }
        }
    };

    let handle_play = move |_: ev::Event| {
        set_is_playing.set(true);
    };

    let handle_pause = move |_: ev::Event| {
        set_is_playing.set(false);
    };

    let play_track = move |index: usize, _: ev::MouseEvent| {
        set_current_index.set(index);
        if let Some(audio) = audio_ref.get() {
            let _ = audio.play();
        }
    };

    let play_next = move |_: ev::MouseEvent| {
        let q = queue.get();
        let idx = current_index.get();
        if idx + 1 < q.len() {
            set_current_index.set(idx + 1);
        } else if repeat_mode.get() == RepeatMode::All && !q.is_empty() {
            set_current_index.set(0);
        }
    };

    let play_prev = move |_: ev::MouseEvent| {
        let idx = current_index.get();
        if idx > 0 {
            set_current_index.set(idx - 1);
        } else if repeat_mode.get() == RepeatMode::All {
            let q = queue.get();
            if !q.is_empty() {
                set_current_index.set(q.len() - 1);
            }
        }
    };

    let toggle_repeat = move |_: ev::MouseEvent| {
        set_repeat_mode.update(|mode| {
            *mode = match mode {
                RepeatMode::Off => RepeatMode::All,
                RepeatMode::All => RepeatMode::One,
                RepeatMode::One => RepeatMode::Off,
            };
        });
    };

    let toggle_shuffle = move |_: ev::MouseEvent| {
        set_is_shuffled.update(|s| *s = !*s);
        if is_shuffled.get() {
            let mut q = queue.get();
            let current = q.remove(current_index.get().min(q.len()));
            // Simple shuffle without rand - just reverse as a simple alternative
            q.reverse();
            q.insert(0, current);
            set_queue.set(q);
            set_current_index.set(0);
        }
    };

    let remove_from_queue = move |index: usize, _: ev::MouseEvent| {
        set_queue.update(|q| {
            if index < q.len() {
                q.remove(index);
            }
        });
        let idx = current_index.get();
        if idx >= queue.get().len() && idx > 0 {
            set_current_index.set(idx - 1);
        }
    };

    let clear_queue = move |_: ev::MouseEvent| {
        set_queue.set(Vec::new());
        set_current_index.set(0);
        if let Some(audio) = audio_ref.get() {
            let _ = audio.pause();
            audio.set_current_time(0.0);
        }
    };

    let current_track = move || {
        let q = queue.get();
        let idx = current_index.get();
        q.get(idx).cloned()
    };

    let format_time = |seconds: f64| -> String {
        if seconds.is_nan() || seconds.is_infinite() {
            return "0:00".to_string();
        }
        let total_secs = seconds as u64;
        let minutes = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}:{:02}", minutes, secs)
    };

    let progress_pct = move || {
        let d = duration.get();
        if d > 0.0 { (current_time.get() / d) * 100.0 } else { 0.0 }
    };

    let has_tracks = move || !queue.get().is_empty();

    view! {
        <div class="fixed bottom-0 left-0 right-0 z-40">
            // Mini player (collapsed)
            <div class="bg-[var(--bg-surface)] border-t border-[var(--border-default)] shadow-lg">
                // Progress bar
                <div class="relative w-full h-1 bg-[var(--text-tertiary)]/20 cursor-pointer group">
                    <div
                        class="absolute h-full bg-[var(--accent)] transition-all duration-100"
                        style:width=move || format!("{}%", progress_pct())
                    ></div>
                    <input
                        type="range"
                        min="0"
                        max=move || duration.get().to_string()
                        step="0.1"
                        value=move || current_time.get().to_string()
                        on:input=handle_seek
                        class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                    />
                </div>

                <div class="flex items-center gap-3 px-4 py-2">
                    // Play/Pause
                    <button
                        class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-primary)] hover:text-[var(--accent)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded"
                        on:click=toggle_play
                        aria-label=move || if is_playing.get() { "Pause" } else { "Play" }
                        disabled=move || !has_tracks()
                    >
                        {move || if is_playing.get() {
                            view! {
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                            }.into_any()
                        } else {
                            view! {
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                            }.into_any()
                        }}
                    </button>

                    // Track info
                    <div class="flex-1 min-w-0">
                        {move || current_track().map(|track| view! {
                            <div class="truncate">
                                <div class="text-sm font-medium text-[var(--text-primary)] truncate">{track.name}</div>
                                <div class="text-xs text-[var(--text-tertiary)] truncate">
                                    {track.artist.unwrap_or_else(|| "Unknown Artist".to_string())}
                                </div>
                            </div>
                        })}
                        {move || current_track().is_none().then(|| view! {
                            <div class="text-sm text-[var(--text-tertiary)]">"No track selected"</div>
                        })}
                    </div>

                    // Time
                    <div class="text-xs font-mono text-[var(--text-tertiary)] hidden sm:block">
                        {move || format_time(current_time.get())} " / " {move || format_time(duration.get())}
                    </div>

                    // Controls
                    <div class="flex items-center gap-1">
                        // Prev
                        <button
                            class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded"
                            on:click=play_prev
                            disabled=move || !has_tracks()
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0019 16V8a1 1 0 00-1.6-.8l-5.333 4zM4.066 11.2a1 1 0 000 1.6l5.334 4A1 1 0 0011 16V8a1 1 0 00-1.6-.8l-5.334 4z" />
                            </svg>
                        </button>

                        // Next
                        <button
                            class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded"
                            on:click=play_next
                            disabled=move || !has_tracks()
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.933 12.8a1 1 0 000-1.6L6.6 7.2A1 1 0 005 8v8a1 1 0 001.6.8l5.333-4zM19.933 12.8a1 1 0 000-1.6l-5.333-4A1 1 0 0013 8v8a1 1 0 001.6.8l5.333-4z" />
                            </svg>
                        </button>

                        // Shuffle
                        <button
                            class=move || format!(
                                "min-w-[44px] min-h-[44px] flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded {}",
                                if is_shuffled.get() { "text-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" }
                            )
                            on:click=toggle_shuffle
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                            </svg>
                        </button>

                        // Repeat
                        <button
                            class=move || format!(
                                "min-w-[44px] min-h-[44px] flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded {}",
                                match repeat_mode.get() {
                                    RepeatMode::Off => "text-[var(--text-tertiary)] hover:text-[var(--text-primary)]",
                                    _ => "text-[var(--accent)]",
                                }
                            )
                            on:click=toggle_repeat
                        >
                            {move || match repeat_mode.get() {
                                RepeatMode::Off => view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                    </svg>
                                }.into_any(),
                                RepeatMode::All => view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                    </svg>
                                }.into_any(),
                                RepeatMode::One => view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                        <text x="12" y="14" text-anchor="middle" font-size="8" fill="currentColor">1</text>
                                    </svg>
                                }.into_any(),
                            }}
                        </button>

                        // Volume
                        <div class="relative hidden sm:block">
                            <button
                                class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded"
                                on:click=toggle_mute
                            >
                                {move || if is_muted.get() || volume.get() == 0.0 {
                                    view! {
                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
                                        </svg>
                                    }.into_any()
                                } else {
                                    view! {
                                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                                        </svg>
                                    }.into_any()
                                }}
                            </button>
                            <input
                                type="range"
                                min="0"
                                max="1"
                                step="0.05"
                                value=move || volume.get().to_string()
                                on:input=handle_volume
                                class="w-20 h-1 accent-[var(--accent)]"
                            />
                        </div>

                        // Queue
                        <button
                            class=move || format!(
                                "min-w-[44px] min-h-[44px] flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded {}",
                                if show_queue.get() { "text-[var(--accent)]" } else { "text-[var(--text-tertiary)] hover:text-[var(--text-primary)]" }
                            )
                            on:click=move |_| set_show_queue.update(|v| *v = !*v)
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                            </svg>
                        </button>
                    </div>
                </div>
            </div>

            // Queue panel
            {move || show_queue.get().then(|| view! {
                <div class="bg-[var(--bg-surface)] border-t border-[var(--border-default)] max-h-64 overflow-auto">
                    <div class="flex items-center justify-between px-4 py-2 border-b border-[var(--border-default)]">
                        <h3 class="text-sm font-medium text-[var(--text-primary)]">"Queue"</h3>
                        <div class="flex items-center gap-2">
                            <button
                                class="text-xs text-[var(--text-tertiary)] hover:text-[var(--text-primary)] transition-colors"
                                on:click=clear_queue
                            >
                                "Clear"
                            </button>
                        </div>
                    </div>
                    <div class="divide-y divide-[var(--border-subtle)]">
                        {move || queue.get().iter().enumerate().map(|(index, track)| {
                            let is_current = move || current_index.get() == index;
                            let track_name = track.name.clone();
                            let track_artist = track.artist.clone().unwrap_or_else(|| "Unknown Artist".to_string());
                            view! {
                                <div
                                    class=move || format!(
                                        "flex items-center gap-3 px-4 py-2 hover:bg-[var(--interactive-hover)] transition-colors {}",
                                        if is_current() { "bg-[var(--accent-subtle)]" } else { "" }
                                    )
                                >
                                    <button
                                        class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--accent)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded"
                                        on:click=move |ev| play_track(index, ev)
                                    >
                                        {move || if is_current() {
                                            view! {
                                                <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                                                    <path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" />
                                                </svg>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                                                </svg>
                                            }.into_any()
                                        }}
                                    </button>
                                    <div class="flex-1 min-w-0">
                                        <div class="text-sm font-medium text-[var(--text-primary)] truncate">{track_name}</div>
                                        <div class="text-xs text-[var(--text-tertiary)] truncate">{track_artist}</div>
                                    </div>
                                    <button
                                        class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--danger)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)] rounded"
                                        on:click=move |ev| remove_from_queue(index, ev)
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                        </svg>
                                    </button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })}
        </div>

        // Hidden audio element
        <audio
            node_ref=audio_ref
            on:timeupdate=handle_time_update
            on:loadedmetadata=handle_loaded_metadata
            on:ended=handle_ended
            on:play=handle_play
            on:pause=handle_pause
        ></audio>
    }
}

#[component]
pub fn AudioTrackButton(
    path: String,
    name: String,
    artist: Option<String>,
    album: Option<String>,
    on_play: Callback<AudioTrack>,
) -> impl IntoView {
    let track = AudioTrack {
        path: path.clone(),
        name: name.clone(),
        artist: artist.clone(),
        album: album.clone(),
        duration: None,
    };

    let handle_click = move |_: ev::MouseEvent| {
        on_play.run(track.clone());
    };

    view! {
        <button
            class="flex items-center gap-3 w-full p-2 text-left hover:bg-[var(--interactive-hover)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
            on:click=handle_click
        >
            <div class="min-w-[44px] min-h-[44px] flex items-center justify-center bg-[var(--accent-subtle)] rounded">
                <svg class="w-5 h-5 text-[var(--accent)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3" />
                </svg>
            </div>
            <div class="flex-1 min-w-0">
                <div class="text-sm font-medium text-[var(--text-primary)] truncate">{name}</div>
                <div class="text-xs text-[var(--text-tertiary)] truncate">
                    {artist.unwrap_or_else(|| "Unknown Artist".to_string())}
                </div>
            </div>
        </button>
    }
}
