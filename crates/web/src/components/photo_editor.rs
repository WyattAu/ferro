use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::api;

#[derive(Clone, Debug, PartialEq)]
struct EditState {
    rotation: i32,
    flip_h: bool,
    flip_v: bool,
    brightness: f64,
    contrast: f64,
    saturation: f64,
    filter: &'static str,
    crop: Option<CropRect>,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            rotation: 0,
            flip_h: false,
            flip_v: false,
            brightness: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            filter: "none",
            crop: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CropRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Debug)]
enum EditAction {
    Rotate(i32),
    FlipH,
    FlipV,
    AdjustBrightness(f64),
    AdjustContrast(f64),
    AdjustSaturation(f64),
    ApplyFilter(&'static str),
    ApplyCrop(CropRect),
    Undo,
    Redo,
}

fn apply_edit(state: &mut EditState, action: &EditAction) {
    match action {
        EditAction::Rotate(deg) => {
            state.rotation = (state.rotation + deg) % 360;
            if state.rotation < 0 {
                state.rotation += 360;
            }
        }
        EditAction::FlipH => state.flip_h = !state.flip_h,
        EditAction::FlipV => state.flip_v = !state.flip_v,
        EditAction::AdjustBrightness(v) => state.brightness = *v,
        EditAction::AdjustContrast(v) => state.contrast = *v,
        EditAction::AdjustSaturation(v) => state.saturation = *v,
        EditAction::ApplyFilter(f) => state.filter = f,
        EditAction::ApplyCrop(rect) => state.crop = Some(rect.clone()),
        EditAction::Undo | EditAction::Redo => {}
    }
}

#[component]
pub fn PhotoEditor(src: String, file_path: String, on_close: Callback<()>) -> impl IntoView {
    let (edit_state, set_edit_state) = signal(EditState::default());
    let (_undo_stack, set_undo_stack) = signal(Vec::<EditState>::new());
    let (_redo_stack, set_redo_stack) = signal(Vec::<EditState>::new());
    let (saving, set_saving) = signal(false);
    let (show_crop, set_show_crop) = signal(false);
    let (crop_start, set_crop_start) = signal(None::<(f64, f64)>);
    let (crop_end, set_crop_end) = signal(None::<(f64, f64)>);
    let (image_loaded, set_image_loaded) = signal(false);
    let (img_width, set_img_width) = signal(0.0_f64);
    let (img_height, set_img_height) = signal(0.0_f64);
    // Pinch-to-zoom state
    let (zoom_level, _set_zoom_level) = signal(1.0_f64);

    let canvas_ref: NodeRef<leptos::html::Canvas> = NodeRef::new();

    let push_undo = move || {
        let current = edit_state.get();
        set_undo_stack.update(|s| s.push(current));
        set_redo_stack.set(vec![]);
    };

    let do_edit = move |action: EditAction| {
        match &action {
            EditAction::Undo => {
                let prev = set_undo_stack.write().pop();
                if let Some(prev) = prev {
                    let current = edit_state.get();
                    set_redo_stack.update(|s| s.push(current));
                    set_edit_state.set(prev);
                }
                return;
            }
            EditAction::Redo => {
                let next = set_redo_stack.write().pop();
                if let Some(next) = next {
                    let current = edit_state.get();
                    set_undo_stack.update(|s| s.push(current));
                    set_edit_state.set(next);
                }
                return;
            }
            _ => {}
        }
        push_undo();
        set_edit_state.update(|s| apply_edit(s, &action));
    };

    let _redraw_canvas = move || {
        if let Some(canvas) = canvas_ref.get() {
            let state = edit_state.get();
            let ctx = canvas
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());

            if let Some(ctx) = ctx {
                let w = canvas.width() as f64;
                let h = canvas.height() as f64;

                ctx.clear_rect(0.0, 0.0, w, h);
                ctx.save();

                ctx.translate(w / 2.0, h / 2.0).ok();
                ctx.rotate((state.rotation as f64).to_radians()).ok();

                let sx = if state.flip_h { -1.0 } else { 1.0 };
                let sy = if state.flip_v { -1.0 } else { 1.0 };
                ctx.scale(sx, sy).ok();

                ctx.translate(-w / 2.0, -h / 2.0).ok();

                let mut filter_parts = vec![];
                if (state.brightness - 1.0).abs() > 0.01 {
                    filter_parts.push(format!("brightness({})", state.brightness));
                }
                if (state.contrast - 1.0).abs() > 0.01 {
                    filter_parts.push(format!("contrast({})", state.contrast));
                }
                if (state.saturation - 1.0).abs() > 0.01 {
                    filter_parts.push(format!("saturate({})", state.saturation));
                }
                if state.filter != "none" {
                    filter_parts.push(state.filter.to_string());
                }
                if !filter_parts.is_empty() {
                    let filter_str = filter_parts.join(" ");
                    let _ = js_sys::Reflect::set(ctx.as_ref(), &"filter".into(), &filter_str.into());
                }

                if let Some(ref crop) = state.crop {
                    let src_x = crop.x.max(0.0).min(w);
                    let src_y = crop.y.max(0.0).min(h);
                    let src_w = crop.width.max(1.0).min(w - src_x);
                    let src_h = crop.height.max(1.0).min(h - src_y);

                    let document = web_sys::window().and_then(|w| w.document());
                    if let Some(document) = document
                        && let Ok(off_el) = document.create_element("canvas")
                        && let Ok(off) = off_el.dyn_into::<web_sys::HtmlCanvasElement>()
                    {
                        off.set_width(src_w as u32);
                        off.set_height(src_h as u32);
                        if let Ok(Some(off_ctx_obj)) = off.get_context("2d")
                            && let Ok(off_ctx) = off_ctx_obj.dyn_into::<web_sys::CanvasRenderingContext2d>()
                        {
                            let _ = off_ctx
                                .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                    &canvas, src_x, src_y, src_w, src_h, 0.0, 0.0, w, h,
                                );
                            let _ = ctx.draw_image_with_html_canvas_element(&off, 0.0, 0.0);
                        }
                    }
                } else if let Some(canvas_node) = canvas_ref.get() {
                    let _ = ctx.draw_image_with_html_canvas_element(&canvas_node, 0.0, 0.0);
                }

                ctx.restore();
            }
        }
    };

    let handle_image_load = move |ev: ev::Event| {
        if let Some(img) = ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlImageElement>().ok()) {
            set_img_width.set(img.natural_width() as f64);
            set_img_height.set(img.natural_height() as f64);
            if let Some(canvas) = canvas_ref.get() {
                canvas.set_width(img.natural_width());
                canvas.set_height(img.natural_height());
                let ctx = canvas
                    .get_context("2d")
                    .ok()
                    .flatten()
                    .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());
                if let Some(ctx) = ctx {
                    let _ = ctx.draw_image_with_html_image_element(&img, 0.0, 0.0);
                }
            }
            set_image_loaded.set(true);
        }
    };

    let handle_canvas_mousedown = move |ev: ev::MouseEvent| {
        if show_crop.get()
            && let Some(canvas) = canvas_ref.get()
        {
            let rect = canvas.get_bounding_client_rect();
            let x = ev.client_x() as f64 - rect.left();
            let y = ev.client_y() as f64 - rect.top();
            set_crop_start.set(Some((x, y)));
            set_crop_end.set(Some((x, y)));
        }
    };

    let handle_canvas_mousemove = move |ev: ev::MouseEvent| {
        if show_crop.get()
            && crop_start.get().is_some()
            && let Some(canvas) = canvas_ref.get()
        {
            let rect = canvas.get_bounding_client_rect();
            let x = ev.client_x() as f64 - rect.left();
            let y = ev.client_y() as f64 - rect.top();
            set_crop_end.set(Some((x, y)));
        }
    };

    let handle_canvas_mouseup = move |_: ev::MouseEvent| {
        if show_crop.get() {
            if let (Some(start), Some(end)) = (crop_start.get(), crop_end.get()) {
                let x = start.0.min(end.0);
                let y = start.1.min(end.1);
                let w = (end.0 - start.0).abs();
                let h = (end.1 - start.1).abs();
                if w > 5.0 && h > 5.0 {
                    do_edit(EditAction::ApplyCrop(CropRect {
                        x,
                        y,
                        width: w,
                        height: h,
                    }));
                }
            }
            set_crop_start.set(None);
            set_crop_end.set(None);
            set_show_crop.set(false);
        }
    };

    let save_edited = move |_: ev::MouseEvent| {
        if let Some(canvas) = canvas_ref.get() {
            set_saving.set(true);
            let canvas = canvas.clone();
            let file_path = file_path.clone();
            spawn_local(async move {
                let blob_cb = Closure::<dyn FnMut(JsValue)>::new(move |blob_val: JsValue| {
                    if let Ok(blob) = blob_val.dyn_into::<web_sys::Blob>() {
                        let file_path = file_path.clone();
                        spawn_local(async move {
                            let array_buffer = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
                                .await
                                .expect("array_buffer");
                            let uint8 = js_sys::Uint8Array::new(&array_buffer);
                            let bytes = uint8.to_vec();
                            let _ = api::upload_file(&file_path, &bytes).await;
                        });
                    }
                });
                let blob_js = blob_cb.into_js_value();
                let func: js_sys::Function = blob_js.into();
                let _ = canvas.to_blob_with_type(&func, "image/png");
                set_saving.set(false);
            });
        }
    };

    let toggle_crop = move |_: ev::MouseEvent| {
        set_show_crop.update(|v| *v = !*v);
    };

    let (toolbar_open, set_toolbar_open) = signal(false);
    let toggle_toolbar = move |_: ev::MouseEvent| {
        set_toolbar_open.update(|v| *v = !*v);
    };

    let canvas_style = move || {
        let w = img_width.get();
        let h = img_height.get();
        if w > 0.0 && h > 0.0 {
            let max_w = 800.0_f64;
            let max_h = 600.0_f64;
            let scale = (max_w / w).min(max_h / h).min(1.0);
            format!("width: {}px; height: {}px;", w * scale, h * scale)
        } else {
            "max-width: 100%; max-height: 60vh;".to_string()
        }
    };

    let crop_overlay_style = move || {
        if let (Some(start), Some(end)) = (crop_start.get(), crop_end.get())
            && show_crop.get()
        {
            let x = start.0.min(end.0);
            let y = start.1.min(end.1);
            let w = (end.0 - start.0).abs();
            let h = (end.1 - start.1).abs();
            return format!(
                "position:absolute; left:{}px; top:{}px; width:{}px; height:{}px; border:2px dashed #3b82f6; background:rgba(59,130,246,0.1); pointer-events:none;",
                x, y, w, h
            );
        }
        "display:none;".to_string()
    };

    view! {
        <div class="fixed inset-0 bg-black bg-opacity-75 z-50 flex items-center justify-center p-4 backdrop-blur-sm">
            <div class="brutal-block rounded shadow-2xl w-full max-w-5xl max-h-[95vh] flex flex-col overflow-hidden">
                <div class="flex items-center justify-between px-4 py-3 border-b border-[var(--border-default)]">
                    <h2 class="text-section font-mono text-[var(--text-primary)]">"Photo Editor"</h2>
                    <div class="flex items-center gap-2">
                        <button
                            class="px-3 py-2 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] brutal-border rounded font-bold uppercase hover:bg-[var(--accent-hover)] transition-colors min-h-[44px]"
                            on:click=save_edited
                            disabled=move || saving.get()
                        >
                            {move || if saving.get() { "Saving..." } else { "Save" }}
                        </button>
                        <button
                            class="p-2 text-[var(--text-tertiary)] hover:text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] rounded transition-colors"
                            aria-label="Close editor"
                            on:click=move |_| on_close.run(())
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    </div>
                </div>

                <div class="flex items-center gap-1 px-3 py-2 bg-[var(--bg-inset)] border-b border-[var(--border-default)] flex-wrap">
                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Rotate 90 CW"
                        on:click=move |_| do_edit(EditAction::Rotate(90))
                    >
                        "↻ 90°"
                    </button>
                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Rotate 90 CCW"
                        on:click=move |_| do_edit(EditAction::Rotate(-90))
                    >
                        "↺ 90°"
                    </button>
                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Rotate 180"
                        on:click=move |_| do_edit(EditAction::Rotate(180))
                    >
                        "↕ 180°"
                    </button>

                    <div class="w-px h-5 bg-[var(--border-subtle)] mx-1"></div>

                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Flip Horizontal"
                        on:click=move |_| do_edit(EditAction::FlipH)
                    >
                        "Flip H"
                    </button>
                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Flip Vertical"
                        on:click=move |_| do_edit(EditAction::FlipV)
                    >
                        "Flip V"
                    </button>

                    <div class="w-px h-5 bg-[var(--border-subtle)] mx-1"></div>

                    <button
                        class=move || format!("px-3 py-2 text-xs font-mono rounded transition-colors min-h-[44px] {}",
                            if show_crop.get() { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "hover:bg-[var(--interactive-hover)]" }
                        )
                        title="Crop"
                        on:click=toggle_crop
                    >
                        "Crop"
                    </button>

                    <div class="w-px h-5 bg-[var(--border-subtle)] mx-1"></div>

                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Undo"
                        on:click=move |_| do_edit(EditAction::Undo)
                    >
                        "Undo"
                    </button>
                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Redo"
                        on:click=move |_| do_edit(EditAction::Redo)
                    >
                        "Redo"
                    </button>

                    <div class="flex-1"></div>

                    <button
                        class="px-3 py-2 text-xs font-mono rounded hover:bg-[var(--interactive-hover)] transition-colors min-h-[44px]"
                        title="Adjustments"
                        on:click=toggle_toolbar
                    >
                        {move || if toolbar_open.get() { "▲ Adjust" } else { "▼ Adjust" }}
                    </button>
                </div>

                {move || toolbar_open.get().then(|| view! {
                    <div class="px-4 py-3 bg-[var(--bg-surface)] border-b border-[var(--border-default)] grid grid-cols-3 gap-4">
                        <div class="flex flex-col gap-1">
                            <label class="text-xs font-mono text-[var(--text-tertiary)]">"Brightness"</label>
                            <input
                                type="range"
                                min="0"
                                max="2"
                                step="0.05"
                                prop:value=move || edit_state.get().brightness.to_string()
                                on:input=move |ev: ev::Event| {
                                    let v = event_target_value(&ev).parse::<f64>().unwrap_or(1.0);
                                    do_edit(EditAction::AdjustBrightness(v));
                                }
                                class="w-full"
                            />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label class="text-xs font-mono text-[var(--text-tertiary)]">"Contrast"</label>
                            <input
                                type="range"
                                min="0"
                                max="2"
                                step="0.05"
                                prop:value=move || edit_state.get().contrast.to_string()
                                on:input=move |ev: ev::Event| {
                                    let v = event_target_value(&ev).parse::<f64>().unwrap_or(1.0);
                                    do_edit(EditAction::AdjustContrast(v));
                                }
                                class="w-full"
                            />
                        </div>
                        <div class="flex flex-col gap-1">
                            <label class="text-xs font-mono text-[var(--text-tertiary)]">"Saturation"</label>
                            <input
                                type="range"
                                min="0"
                                max="2"
                                step="0.05"
                                prop:value=move || edit_state.get().saturation.to_string()
                                on:input=move |ev: ev::Event| {
                                    let v = event_target_value(&ev).parse::<f64>().unwrap_or(1.0);
                                    do_edit(EditAction::AdjustSaturation(v));
                                }
                                class="w-full"
                            />
                        </div>
                    </div>
                })}

                <div class="flex items-center gap-2 px-4 py-2 bg-[var(--bg-surface)] border-b border-[var(--border-default)]">
                    <span class="text-xs font-mono text-[var(--text-tertiary)]">"Filters:"</span>
                    {["none", "grayscale(100%)", "sepia(100%)", "invert(100%)"]
                        .iter()
                        .map(|f| {
                            let filter = *f;
                            let label = match filter {
                                "none" => "None",
                                "grayscale(100%)" => "Grayscale",
                                "sepia(100%)" => "Sepia",
                                "invert(100%)" => "Invert",
                                _ => "",
                            };
                            let is_active = move || edit_state.get().filter == filter;
                            view! {
                                <button
                                    class=move || format!("px-3 py-2 text-xs font-mono rounded transition-colors min-h-[44px] {}",
                                        if is_active() { "bg-[var(--accent)] text-[var(--text-on-accent)]" } else { "hover:bg-[var(--interactive-hover)]" }
                                    )
                                    on:click=move |_| do_edit(EditAction::ApplyFilter(filter))
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()
                    }
                </div>

                <div class="flex-1 overflow-auto p-4 flex items-center justify-center bg-[var(--bg-base)] relative">
                    <div class="relative inline-block">
                        <img
                            src=src.clone()
                            alt="Source"
                            class="hidden"
                            on:load=handle_image_load
                        />
                        <canvas
                            node_ref=canvas_ref
                            style=move || format!("{} transform: scale({});", canvas_style(), zoom_level.get())
                            class="border border-[var(--border-default)] rounded touch-none"
                            on:mousedown=handle_canvas_mousedown
                            on:mousemove=handle_canvas_mousemove
                            on:mouseup=handle_canvas_mouseup
                        ></canvas>
                        <div style=crop_overlay_style></div>
                    </div>
                    {move || if !image_loaded.get() {
                        view! {
                            <div class="absolute inset-0 flex items-center justify-center">
                                <div class="animate-spin w-8 h-8 border-2 border-[var(--accent)] border-t-transparent rounded-full"></div>
                                <span class="ml-3 text-[var(--text-tertiary)]">"Loading image..."</span>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span class="hidden"></span> }.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
