use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Debug, Clone, PartialEq)]
pub enum VideoQuality {
    Auto,
    Low,
    Medium,
    High,
}

impl VideoQuality {
    pub fn label(&self) -> &'static str {
        match self {
            VideoQuality::Auto => "Auto",
            VideoQuality::Low => "360p",
            VideoQuality::Medium => "720p",
            VideoQuality::High => "1080p",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackSpeed {
    Half,
    Normal,
    OneAndHalf,
    Double,
}

impl PlaybackSpeed {
    pub fn label(&self) -> &'static str {
        match self {
            PlaybackSpeed::Half => "0.5x",
            PlaybackSpeed::Normal => "1x",
            PlaybackSpeed::OneAndHalf => "1.5x",
            PlaybackSpeed::Double => "2x",
        }
    }

    pub fn value(&self) -> f64 {
        match self {
            PlaybackSpeed::Half => 0.5,
            PlaybackSpeed::Normal => 1.0,
            PlaybackSpeed::OneAndHalf => 1.5,
            PlaybackSpeed::Double => 2.0,
        }
    }
}

fn format_time(seconds: f64) -> String {
    if seconds.is_nan() || seconds.is_infinite() {
        return "0:00".to_string();
    }
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{}:{:02}", minutes, secs)
    }
}

#[component]
pub fn VideoPlayer(src: String, #[prop(optional)] title: String) -> impl IntoView {
    let (is_playing, set_is_playing) = signal(false);
    let (is_muted, set_is_muted) = signal(false);
    let (is_fullscreen, set_is_fullscreen) = signal(false);
    let (is_buffering, set_is_buffering) = signal(true);
    let (show_controls, set_show_controls) = signal(true);
    let (show_volume_slider, set_show_volume_slider) = signal(false);
    let (show_quality_menu, set_show_quality_menu) = signal(false);
    let (show_speed_menu, set_show_speed_menu) = signal(false);
    let (current_time, set_current_time) = signal(0.0_f64);
    let (duration, set_duration) = signal(0.0_f64);
    let (volume, set_volume) = signal(1.0_f64);
    let (quality, set_quality) = signal(VideoQuality::Auto);
    let (playback_speed, set_playback_speed) = signal(PlaybackSpeed::Normal);
    let (buffered_end, set_buffered_end) = signal(0.0_f64);

    let video_ref: NodeRef<leptos::html::Video> = NodeRef::new();

    let toggle_play = move |_: ev::MouseEvent| {
        if let Some(video) = video_ref.get() {
            if video.paused() {
                let _ = video.play();
            } else {
                let _ = video.pause();
            }
        }
    };

    let toggle_mute = move |_: ev::MouseEvent| {
        if let Some(video) = video_ref.get() {
            let new_muted = !is_muted.get();
            video.set_muted(new_muted);
            set_is_muted.set(new_muted);
        }
    };

    let handle_volume = move |ev: ev::Event| {
        if let Some(target) = ev.target()
            && let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>()
            && let Ok(val) = input.value().parse::<f64>()
            && let Some(video) = video_ref.get()
        {
            video.set_volume(val);
            set_volume.set(val);
            if val == 0.0 {
                set_is_muted.set(true);
            } else if is_muted.get() {
                video.set_muted(false);
                set_is_muted.set(false);
            }
        }
    };

    let handle_seek = move |ev: ev::Event| {
        if let Some(target) = ev.target()
            && let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>()
            && let Ok(val) = input.value().parse::<f64>()
            && let Some(video) = video_ref.get()
        {
            video.set_current_time(val);
            set_current_time.set(val);
        }
    };

    let toggle_fullscreen = move |_: ev::MouseEvent| {
        if let Some(video) = video_ref.get() {
            if is_fullscreen.get() {
                let document = web_sys::window().unwrap().document().unwrap();
                document.exit_fullscreen();
                set_is_fullscreen.set(false);
            } else {
                let _ = video.request_fullscreen();
                set_is_fullscreen.set(true);
            }
        }
    };

    let set_speed = move |speed: PlaybackSpeed, _: ev::MouseEvent| {
        if let Some(video) = video_ref.get() {
            video.set_playback_rate(speed.value());
            set_playback_speed.set(speed);
            set_show_speed_menu.set(false);
        }
    };

    let set_quality_level = move |q: VideoQuality, _: ev::MouseEvent| {
        set_quality.set(q);
        set_show_quality_menu.set(false);
    };

    let handle_loaded_metadata = move |_: ev::Event| {
        if let Some(video) = video_ref.get() {
            set_duration.set(video.duration());
            set_is_buffering.set(false);
        }
    };

    let handle_time_update = move |_: ev::Event| {
        if let Some(video) = video_ref.get() {
            set_current_time.set(video.current_time());
        }
    };

    let handle_waiting = move |_: ev::Event| {
        set_is_buffering.set(true);
    };

    let handle_can_play = move |_: ev::Event| {
        set_is_buffering.set(false);
    };

    let handle_play = move |_: ev::Event| {
        set_is_playing.set(true);
    };

    let handle_pause = move |_: ev::Event| {
        set_is_playing.set(false);
    };

    let handle_progress = move |_: ev::ProgressEvent| {
        if let Some(video) = video_ref.get()
            && let Some(media) = video.dyn_ref::<web_sys::HtmlMediaElement>()
        {
            let buffered = media.buffered();
            if buffered.length() > 0
                && let Ok(end) = buffered.end(0)
            {
                set_buffered_end.set(end);
            }
        }
    };

    let current_time_str = move || format_time(current_time.get());
    let duration_str = move || format_time(duration.get());

    let progress_pct = move || {
        let d = duration.get();
        if d > 0.0 { (current_time.get() / d) * 100.0 } else { 0.0 }
    };

    let buffered_pct = move || {
        let d = duration.get();
        if d > 0.0 { (buffered_end.get() / d) * 100.0 } else { 0.0 }
    };

    view! {
        <div
            class="relative bg-black rounded-lg overflow-hidden group"
            on:mouseenter=move |_| set_show_controls.set(true)
            on:mouseleave=move |_| set_show_controls.set(false)
        >
            <video
                node_ref=video_ref
                class="w-full max-h-[70vh] object-contain"
                src=src
                on:loadedmetadata=handle_loaded_metadata
                on:timeupdate=handle_time_update
                on:waiting=handle_waiting
                on:canplay=handle_can_play
                on:play=handle_play
                on:pause=handle_pause
                on:progress=handle_progress
                on:click=toggle_play
            >
                "Your browser does not support the video element."
            </video>

            // Buffering indicator
            {move || is_buffering.get().then(|| view! {
                <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
                    <div class="animate-spin w-12 h-12 border-4 border-white border-t-transparent rounded-full"></div>
                </div>
            })}

            // Title overlay
            {move || (!title.is_empty() && show_controls.get()).then(|| view! {
                <div class="absolute top-0 left-0 right-0 bg-gradient-to-b from-black/60 to-transparent px-4 py-2 pointer-events-none">
                    <span class="text-white text-sm font-mono truncate block">{title.clone()}</span>
                </div>
            })}

            // Controls overlay
            <div class=move || {
                format!(
                    "absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent transition-opacity duration-300 px-4 pb-3 pt-8 {}",
                    if show_controls.get() { "opacity-100" } else { "opacity-0" }
                )
            }>
                // Progress bar
                <div class="relative w-full h-1.5 bg-gray-600 rounded-full mb-3 cursor-pointer group/progress">
                    // Buffered
                    <div
                        class="absolute h-full bg-gray-500 rounded-full"
                        style:width=move || format!("{}%", buffered_pct())
                    ></div>
                    // Progress
                    <div
                        class="absolute h-full bg-red-500 rounded-full"
                        style:width=move || format!("{}%", progress_pct())
                    ></div>
                    // Seek input
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

                <div class="flex items-center gap-3">
                    // Play/Pause
                    <button
                        class="text-white hover:text-red-400 transition-colors focus:outline-none"
                        on:click=toggle_play
                        aria-label=move || if is_playing.get() { "Pause" } else { "Play" }
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

                    // Time display
                    <span class="text-white text-xs font-mono">
                        {current_time_str} " / " {duration_str}
                    </span>

                    // Spacer
                    <div class="flex-1"></div>

                    // Volume
                    <div
                        class="relative"
                        on:mouseenter=move |_| set_show_volume_slider.set(true)
                        on:mouseleave=move |_| set_show_volume_slider.set(false)
                    >
                        <button
                            class="text-white hover:text-red-400 transition-colors focus:outline-none"
                            on:click=toggle_mute
                            aria-label=move || if is_muted.get() { "Unmute" } else { "Mute" }
                        >
                            {move || if is_muted.get() || volume.get() == 0.0 {
                                view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
                                    </svg>
                                }.into_any()
                            } else if volume.get() < 0.5 {
                                view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                                    </svg>
                                }.into_any()
                            } else {
                                view! {
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 12.728M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15z" />
                                    </svg>
                                }.into_any()
                            }}
                        </button>
                        {move || show_volume_slider.get().then(|| view! {
                            <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 bg-gray-800 rounded-lg p-2 shadow-lg">
                                <input
                                    type="range"
                                    min="0"
                                    max="1"
                                    step="0.05"
                                    value=move || volume.get().to_string()
                                    on:input=handle_volume
                                    class="w-20 h-1 accent-red-500"
                                />
                            </div>
                        })}
                    </div>

                    // Speed control
                    <div class="relative">
                        <button
                            class="text-white text-xs font-mono hover:text-red-400 transition-colors px-1.5 py-0.5 rounded bg-gray-700/50 focus:outline-none"
                            on:click=move |_| set_show_speed_menu.update(|v| *v = !*v)
                        >
                            {move || playback_speed.get().label()}
                        </button>
                        {move || show_speed_menu.get().then(|| view! {
                            <div class="absolute bottom-full right-0 mb-2 bg-gray-800 rounded-lg shadow-lg overflow-hidden">
                                {vec![
                                    PlaybackSpeed::Half,
                                    PlaybackSpeed::Normal,
                                    PlaybackSpeed::OneAndHalf,
                                    PlaybackSpeed::Double,
                                ].into_iter().map(|speed| {
                                    let speed_clone = speed.clone();
                                    let is_active = move || playback_speed.get() == speed_clone;
                                    view! {
                                        <button
                                            class=move || format!(
                                                "block w-full text-left px-3 py-1.5 text-xs font-mono hover:bg-gray-700 {}",
                                                if is_active() { "text-red-400" } else { "text-white" }
                                            )
                                            on:click=move |ev| set_speed(speed.clone(), ev)
                                        >
                                            {speed.label()}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        })}
                    </div>

                    // Quality selector
                    <div class="relative">
                        <button
                            class="text-white text-xs font-mono hover:text-red-400 transition-colors px-1.5 py-0.5 rounded bg-gray-700/50 focus:outline-none"
                            on:click=move |_| set_show_quality_menu.update(|v| *v = !*v)
                        >
                            {move || quality.get().label()}
                        </button>
                        {move || show_quality_menu.get().then(|| view! {
                            <div class="absolute bottom-full right-0 mb-2 bg-gray-800 rounded-lg shadow-lg overflow-hidden">
                                {vec![
                                    VideoQuality::Auto,
                                    VideoQuality::Low,
                                    VideoQuality::Medium,
                                    VideoQuality::High,
                                ].into_iter().map(|q| {
                                    let q_clone = q.clone();
                                    let is_active = move || quality.get() == q_clone;
                                    view! {
                                        <button
                                            class=move || format!(
                                                "block w-full text-left px-3 py-1.5 text-xs font-mono hover:bg-gray-700 {}",
                                                if is_active() { "text-red-400" } else { "text-white" }
                                            )
                                            on:click=move |ev| set_quality_level(q.clone(), ev)
                                        >
                                            {q.label()}
                                        </button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        })}
                    </div>

                    // Fullscreen
                    <button
                        class="text-white hover:text-red-400 transition-colors focus:outline-none"
                        on:click=toggle_fullscreen
                        aria-label=move || if is_fullscreen.get() { "Exit fullscreen" } else { "Fullscreen" }
                    >
                        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}
