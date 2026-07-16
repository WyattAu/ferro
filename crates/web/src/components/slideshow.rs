use leptos::ev;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum SlideshowInterval {
    Manual,
    ThreeSeconds,
    FiveSeconds,
    TenSeconds,
    ThirtySeconds,
}

impl SlideshowInterval {
    pub fn as_millis(&self) -> u32 {
        match self {
            SlideshowInterval::Manual => 0,
            SlideshowInterval::ThreeSeconds => 3000,
            SlideshowInterval::FiveSeconds => 5000,
            SlideshowInterval::TenSeconds => 10000,
            SlideshowInterval::ThirtySeconds => 30000,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SlideshowInterval::Manual => "Manual",
            SlideshowInterval::ThreeSeconds => "3s",
            SlideshowInterval::FiveSeconds => "5s",
            SlideshowInterval::TenSeconds => "10s",
            SlideshowInterval::ThirtySeconds => "30s",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionEffect {
    Fade,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SlideshowImage {
    pub path: String,
    pub name: String,
    pub thumbnail_path: Option<String>,
}

#[component]
pub fn Slideshow(images: Vec<SlideshowImage>, initial_index: usize, on_close: Callback<()>) -> impl IntoView {
    let (current_index, set_current_index) = signal(initial_index);
    let (is_playing, set_is_playing) = signal(true);
    let (slideshow_interval, set_slideshow_interval) = signal(SlideshowInterval::FiveSeconds);
    let (transition, set_transition) = signal(TransitionEffect::Fade);
    let (show_controls, set_show_controls) = signal(true);
    let (_show_settings, _set_show_settings) = signal(false);
    let (is_fullscreen, _set_is_fullscreen) = signal(false);

    let images_len = images.len();
    let _images_for_each = images.clone();
    let images_for_thumbs = images.clone();

    let images_stored = StoredValue::new(images.clone());
    let get_current_image = move || {
        let idx = current_index.get();
        images_stored.get_value().get(idx).cloned()
    };

    let total_images = images_len;

    let next_image = move |_: ev::MouseEvent| {
        let idx = current_index.get();
        if idx + 1 < total_images {
            set_current_index.set(idx + 1);
        } else {
            set_current_index.set(0);
        }
    };

    let prev_image = move |_: ev::MouseEvent| {
        let idx = current_index.get();
        if idx > 0 {
            set_current_index.set(idx - 1);
        } else {
            set_current_index.set(total_images - 1);
        }
    };

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        match ev.key().as_str() {
            "ArrowRight" | " " => {
                ev.prevent_default();
                let idx = current_index.get();
                if idx + 1 < total_images {
                    set_current_index.set(idx + 1);
                } else {
                    set_current_index.set(0);
                }
            }
            "ArrowLeft" => {
                ev.prevent_default();
                let idx = current_index.get();
                if idx > 0 {
                    set_current_index.set(idx - 1);
                } else {
                    set_current_index.set(total_images - 1);
                }
            }
            "Escape" => {
                on_close.run(());
            }
            "f" => {
                // toggle_fullscreen is called via button click, not keyboard
            }
            _ => {}
        }
    };

    let toggle_play = move |_: ev::MouseEvent| {
        set_is_playing.update(|p| *p = !*p);
    };

    let toggle_fullscreen = move |_: ev::MouseEvent| {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("no global window");
            let document = window.document().expect("no document");
            if is_fullscreen.get() {
                let _ = document.exit_fullscreen();
                _set_is_fullscreen.set(false);
            } else {
                if let Some(body) = document.body() {
                    let _ = body.request_fullscreen();
                    _set_is_fullscreen.set(true);
                }
            }
        }
    };

    let handle_interval_change = move |ev: ev::Event| {
        if let Some(target) = ev.target()
            && let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>()
            && let Ok(val) = select.value().parse::<u32>()
        {
            let new_interval = match val {
                0 => SlideshowInterval::Manual,
                3000 => SlideshowInterval::ThreeSeconds,
                5000 => SlideshowInterval::FiveSeconds,
                10000 => SlideshowInterval::TenSeconds,
                30000 => SlideshowInterval::ThirtySeconds,
                _ => SlideshowInterval::Manual,
            };
            set_slideshow_interval.set(new_interval);
        }
    };

    let handle_transition_change = move |ev: ev::Event| {
        if let Some(target) = ev.target()
            && let Ok(select) = target.dyn_into::<web_sys::HtmlSelectElement>()
            && let Ok(val) = select.value().parse::<u32>()
        {
            let new_transition = match val {
                0 => TransitionEffect::Fade,
                1 => TransitionEffect::SlideLeft,
                2 => TransitionEffect::SlideRight,
                3 => TransitionEffect::SlideUp,
                4 => TransitionEffect::SlideDown,
                _ => TransitionEffect::Fade,
            };
            set_transition.set(new_transition);
        }
    };

    // Auto-advance timer using wasm-bindgen
    let interval_ms = move || slideshow_interval.get().as_millis();

    Effect::new(move |_| {
        let ms = interval_ms();
        if ms > 0 && is_playing.get() {
            let closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                let idx = current_index.get();
                if idx + 1 < total_images {
                    set_current_index.set(idx + 1);
                } else {
                    set_current_index.set(0);
                }
            });

            let window = web_sys::window().expect("no global window");
            let _ = window
                .set_interval_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), ms as i32);
            closure.forget();
        }
    });

    // Swipe gesture on the main image area
    let main_area_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    {
        let swipe_state = Rc::new(RefCell::new((0.0_f64, 0.0_f64, false)));

        let state_clone = swipe_state.clone();
        let on_pointerdown = Rc::new(Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
            move |ev: web_sys::PointerEvent| {
                let mut state = state_clone.borrow_mut();
                state.0 = ev.client_x() as f64;
                state.1 = ev.client_y() as f64;
                state.2 = true;
            },
        ));

        let state_clone = swipe_state.clone();
        let set_idx = set_current_index;
        let read_idx = current_index;
        let total = total_images;
        let on_pointerup = Rc::new(Closure::<dyn FnMut(web_sys::PointerEvent)>::new(
            move |ev: web_sys::PointerEvent| {
                let mut state = state_clone.borrow_mut();
                if state.2 {
                    state.2 = false;
                    let start_x = state.0;
                    let start_y = state.1;
                    drop(state);
                    let x = ev.client_x() as f64;
                    let y = ev.client_y() as f64;
                    let dx = x - start_x;
                    let dy = y - start_y;
                    if dx.abs() > 50.0 || dy.abs() > 50.0 {
                        let is_left = dx.abs() > dy.abs() && dx < -50.0;
                        let is_right = dx.abs() > dy.abs() && dx > 50.0;
                        let idx = read_idx.get();
                        if is_left {
                            if idx + 1 < total {
                                set_idx.set(idx + 1);
                            } else {
                                set_idx.set(0);
                            }
                        } else if is_right {
                            if idx > 0 {
                                set_idx.set(idx - 1);
                            } else {
                                set_idx.set(total - 1);
                            }
                        }
                    }
                }
            },
        ));

        let element_for_cleanup = main_area_ref;
        let pd_clone = on_pointerdown.clone();
        let pu_clone = on_pointerup.clone();

        Effect::new(move |_| {
            let element = match element_for_cleanup.get() {
                Some(e) => e,
                None => return None,
            };
            let _ = element.add_event_listener_with_callback("pointerdown", (*pd_clone).as_ref().unchecked_ref());
            let _ = element.add_event_listener_with_callback("pointerup", (*pu_clone).as_ref().unchecked_ref());
            None::<()>
        });
    }

    // Keyboard event listener
    Effect::new(move |_| {
        let handle = window_event_listener(ev::keydown, handle_keydown);
        move || handle.remove()
    });

    view! {
        <div
            class="fixed inset-0 z-50 bg-black flex flex-col"
            on:mousemove=move |_| set_show_controls.set(true)
            on:mouseleave=move |_| set_show_controls.set(false)
        >
            // Main image area
            <div
                node_ref=main_area_ref
                class="flex-1 relative flex items-center justify-center overflow-hidden touch-none"
            >
                {move || {
                    let image = get_current_image();
                    image.map(|img| {
                        let transition_class = match transition.get() {
                            TransitionEffect::Fade => "transition-opacity duration-500",
                            TransitionEffect::SlideLeft => "transition-transform duration-500 -translate-x-full",
                            TransitionEffect::SlideRight => "transition-transform duration-500 translate-x-full",
                            TransitionEffect::SlideUp => "transition-transform duration-500 -translate-y-full",
                            TransitionEffect::SlideDown => "transition-transform duration-500 translate-y-full",
                        };

                        view! {
                            <div class="absolute inset-0 flex items-center justify-center">
                                <img
                                    src=img.path
                                    alt=img.name
                                    class="max-w-full max-h-full object-contain"
                                    style:animation=transition_class
                                />
                            </div>
                        }
                    })
                }}

                // Navigation arrows
                {move || show_controls.get().then(|| view! {
                    <>
                        <button
                            class="absolute left-4 top-1/2 -translate-y-1/2 min-w-[44px] min-h-[44px] flex items-center justify-center bg-black/50 hover:bg-black/70 text-white rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-white/50"
                            on:click=prev_image
                            aria-label="Previous image"
                        >
                            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                            </svg>
                        </button>
                        <button
                            class="absolute right-4 top-1/2 -translate-y-1/2 min-w-[44px] min-h-[44px] flex items-center justify-center bg-black/50 hover:bg-black/70 text-white rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-white/50"
                            on:click=next_image
                            aria-label="Next image"
                        >
                            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                            </svg>
                        </button>
                    </>
                })}
            </div>

            // Thumbnail strip
            {move || show_controls.get().then(|| view! {
                <div class="bg-black/80 p-2 overflow-x-auto">
                    <div class="flex gap-2 justify-center min-w-max px-4">
                        {images_for_thumbs.iter().enumerate().map(|(index, img)| {
                            let is_current = move || current_index.get() == index;
                            let img_path = img.thumbnail_path.clone().unwrap_or_else(|| img.path.clone());
                            let img_name = img.name.clone();
                            view! {
                                <button
                                    class=move || format!(
                                        "w-16 h-16 rounded overflow-hidden border-2 transition-all focus:outline-none focus:ring-2 focus:ring-white/50 {}",
                                        if is_current() { "border-[var(--accent)] scale-110" } else { "border-transparent opacity-60 hover:opacity-100" }
                                    )
                                    on:click=move |_: ev::MouseEvent| set_current_index.set(index)
                                >
                                    <img
                                        src=img_path
                                        alt=img_name
                                        class="w-full h-full object-cover"
                                    />
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })}

            // Top controls bar
            {move || show_controls.get().then(|| view! {
                <div class="absolute top-0 left-0 right-0 bg-gradient-to-b from-black/80 to-transparent p-4">
                    <div class="flex items-center justify-between">
                        // Image info
                        <div class="text-white">
                            {move || {
                                let img_opt = get_current_image();
                                img_opt.map(|img| view! {
                                    <div class="text-sm font-medium">{img.name}</div>
                                    <div class="text-xs opacity-75">
                                        {format!("{} / {}", current_index.get() + 1, total_images)}
                                    </div>
                                })
                            }}
                        </div>

                        // Controls
                        <div class="flex items-center gap-2">
                            // Play/Pause
                            <button
                                class="min-w-[44px] min-h-[44px] flex items-center justify-center text-white hover:text-[var(--accent)] transition-colors focus:outline-none focus:ring-2 focus:ring-white/50 rounded"
                                on:click=toggle_play
                                aria-label=move || if is_playing.get() { "Pause slideshow" } else { "Play slideshow" }
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

                            // Interval selector
                            <select
                                class="bg-black/50 text-white text-sm rounded px-3 py-2 border border-white/20 focus:outline-none focus:ring-2 focus:ring-white/50 min-h-[44px]"
                                on:change=handle_interval_change
                            >
                                <option value="0">"Manual"</option>
                                <option value="3000">"3s"</option>
                                <option value="5000">"5s"</option>
                                <option value="10000">"10s"</option>
                                <option value="30000">"30s"</option>
                            </select>

                            // Transition selector
                            <select
                                class="bg-black/50 text-white text-sm rounded px-3 py-2 border border-white/20 focus:outline-none focus:ring-2 focus:ring-white/50 min-h-[44px]"
                                on:change=handle_transition_change
                            >
                                <option value="0">"Fade"</option>
                                <option value="1">"Slide Left"</option>
                                <option value="2">"Slide Right"</option>
                                <option value="3">"Slide Up"</option>
                                <option value="4">"Slide Down"</option>
                            </select>

                            // Fullscreen
                            <button
                                class="min-w-[44px] min-h-[44px] flex items-center justify-center text-white hover:text-[var(--accent)] transition-colors focus:outline-none focus:ring-2 focus:ring-white/50 rounded"
                                on:click=toggle_fullscreen
                                aria-label=move || if is_fullscreen.get() { "Exit fullscreen" } else { "Fullscreen" }
                            >
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
                                </svg>
                            </button>

                            // Close
                            <button
                                class="min-w-[44px] min-h-[44px] flex items-center justify-center text-white hover:text-[var(--danger)] transition-colors focus:outline-none focus:ring-2 focus:ring-white/50 rounded"
                                on:click=move |_: ev::MouseEvent| on_close.run(())
                                aria-label="Close slideshow"
                            >
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
pub fn SlideshowButton(
    images: Vec<SlideshowImage>,
    on_start_slideshow: Callback<Vec<SlideshowImage>>,
) -> impl IntoView {
    let images_clone = images.clone();
    let handle_click = move |_: ev::MouseEvent| {
        if !images_clone.is_empty() {
            on_start_slideshow.run(images_clone.clone());
        }
    };

    view! {
        <button
            class="min-w-[44px] min-h-[44px] flex items-center justify-center text-[var(--text-tertiary)] hover:text-[var(--accent)] hover:bg-[var(--accent-subtle)] rounded transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--border-focus)]"
            on:click=handle_click
            aria-label="Start slideshow"
            disabled=images.is_empty()
        >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
        </button>
    }
}
