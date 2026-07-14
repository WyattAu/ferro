use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;

use crate::components::header::{Header, provide_header_state};
use crate::components::theme_toggle::provide_theme_state;

#[derive(Debug, Clone, PartialEq)]
enum Tool {
    Pen,
    Line,
    Rectangle,
    Circle,
    Text,
    Eraser,
}

impl Tool {
    fn label(&self) -> &'static str {
        match self {
            Tool::Pen => "Pen",
            Tool::Line => "Line",
            Tool::Rectangle => "Rect",
            Tool::Circle => "Circle",
            Tool::Text => "Text",
            Tool::Eraser => "Eraser",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Tool::Pen => "M12 20h9M16.5 3.5a2.121 2.121 0 013 3L7 19l-4 1 1-4L16.5 3.5z",
            Tool::Line => "M5 19L19 5",
            Tool::Rectangle => "M3 3h18v18H3V3z",
            Tool::Circle => "M12 12m-9 0a9 9 0 1 1 18 0a9 9 0 1 1-18 0",
            Tool::Text => "M4 7V4h16v3M9 20h6M12 4v16",
            Tool::Eraser => "M20 20H7l-4-4 8-8 9 9-4 4M18 13l-5-5",
        }
    }
}

#[derive(Debug, Clone)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone)]
struct WhiteboardElement {
    id: String,
    tool: Tool,
    points: Vec<Point>,
    color: String,
    stroke_width: f64,
    text: Option<String>,
}

#[derive(Debug, Clone)]
struct Viewport {
    x: f64,
    y: f64,
    zoom: f64,
}

const PRESET_COLORS: &[&str] = &[
    "#000000", "#ffffff", "#ff0000", "#00ff00", "#0000ff", "#ffff00", "#ff00ff", "#00ffff", "#ff8800", "#8800ff",
    "#0088ff", "#88ff00",
];

const STROKE_WIDTHS: &[f64] = &[1.0, 2.0, 3.0, 5.0, 8.0, 12.0];

#[component]
pub fn WhiteboardPage() -> impl IntoView {
    provide_theme_state();
    provide_header_state();

    let (elements, set_elements) = signal(Vec::<WhiteboardElement>::new());
    let (current_tool, set_current_tool) = signal(Tool::Pen);
    let (current_color, set_current_color) = signal("#000000".to_string());
    let (stroke_width, set_stroke_width) = signal(2.0_f64);
    let (is_drawing, set_is_drawing) = signal(false);
    let (current_element, set_current_element) = signal(None::<WhiteboardElement>);
    let (undo_stack, set_undo_stack) = signal(Vec::<Vec<WhiteboardElement>>::new());
    let (redo_stack, set_redo_stack) = signal(Vec::<Vec<WhiteboardElement>>::new());
    let (viewport, set_viewport) = signal(Viewport {
        x: 0.0,
        y: 0.0,
        zoom: 1.0,
    });
    let (is_panning, set_is_panning) = signal(false);
    let (pan_start, set_pan_start) = signal(None::<Point>);
    let (show_color_picker, set_show_color_picker) = signal(false);
    let (show_stroke_picker, set_show_stroke_picker) = signal(false);
    let (whiteboard_name, _set_whiteboard_name) = signal("Untitled Whiteboard".to_string());

    let canvas_ref: NodeRef<leptos::html::Canvas> = NodeRef::new();
    let container_ref: NodeRef<leptos::html::Div> = NodeRef::new();

    // Save to undo stack
    let save_to_undo = move || {
        let current_elements = elements.get();
        set_undo_stack.update(|stack| stack.push(current_elements));
        set_redo_stack.set(vec![]);
    };

    // Undo
    let undo = move |_: ev::MouseEvent| {
        if let Some(previous) = set_undo_stack.write().pop() {
            let current = elements.get();
            set_redo_stack.update(|stack| stack.push(current));
            set_elements.set(previous);
        }
    };

    // Redo
    let redo = move |_: ev::MouseEvent| {
        if let Some(next) = set_redo_stack.write().pop() {
            let current = elements.get();
            set_undo_stack.update(|stack| stack.push(current));
            set_elements.set(next);
        }
    };

    // Clear canvas
    let clear_canvas = move |_: ev::MouseEvent| {
        save_to_undo();
        set_elements.set(vec![]);
    };

    // Export to PNG
    let export_png = move |_: ev::MouseEvent| {
        if let Some(canvas) = canvas_ref.get() {
            let dyn_el: web_sys::HtmlElement = canvas.into();
            if let Ok(canvas_el) = dyn_el.dyn_into::<web_sys::HtmlCanvasElement>()
                && let Ok(data_url) = canvas_el.to_data_url_with_type("image/png")
            {
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();
                if let Ok(anchor) = document.create_element("a")
                    && let Ok(anchor) = anchor.dyn_into::<web_sys::HtmlAnchorElement>()
                {
                    anchor.set_href(&data_url);
                    anchor.set_download("whiteboard.png");
                    anchor.click();
                }
            }
        }
    };

    // Save whiteboard to server
    let save_to_server = move |_: ev::MouseEvent| {
        let current_elements = elements.get();
        let current_viewport = viewport.get();
        let name = whiteboard_name.get();

        spawn_local(async move {
            let elements_data: Vec<serde_json::Value> = current_elements
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "id": e.id,
                        "tool": format!("{:?}", e.tool).to_lowercase(),
                        "points": e.points.iter().map(|p| serde_json::json!({"x": p.x, "y": p.y})).collect::<Vec<_>>(),
                        "color": e.color,
                        "stroke_width": e.stroke_width,
                        "text": e.text,
                    })
                })
                .collect();

            let body = serde_json::json!({
                "name": name,
                "elements": elements_data,
                "viewport": {
                    "x": current_viewport.x,
                    "y": current_viewport.y,
                    "zoom": current_viewport.zoom,
                },
            });

            // In a real implementation, this would call the API
            // For now, just log to console
            web_sys::console::log_1(&format!("Saving whiteboard: {}", body).into());
        });
    };

    // Canvas mouse handlers
    let handle_canvas_mousedown = move |ev: ev::MouseEvent| {
        if ev.button() == 1 || (ev.button() == 0 && ev.shift_key()) {
            // Middle click or shift+click: start panning
            set_is_panning.set(true);
            set_pan_start.set(Some(Point {
                x: ev.offset_x() as f64,
                y: ev.offset_y() as f64,
            }));
            return;
        }

        if ev.button() != 0 {
            return;
        }

        let tool = current_tool.get();
        let mut element = WhiteboardElement {
            id: uuid::Uuid::new_v4().to_string(),
            tool: tool.clone(),
            points: vec![],
            color: current_color.get(),
            stroke_width: stroke_width.get(),
            text: None,
        };

        let x = (ev.offset_x() as f64 - viewport.get().x) / viewport.get().zoom;
        let y = (ev.offset_y() as f64 - viewport.get().y) / viewport.get().zoom;

        if tool == Tool::Text {
            let window = web_sys::window().unwrap();
            if let Ok(Some(text)) = window.prompt_with_message("Enter text:") {
                element.text = Some(text);
                element.points.push(Point { x, y });
                save_to_undo();
                set_elements.update(|els| els.push(element));
                return;
            }
        } else {
            element.points.push(Point { x, y });
        }

        set_is_drawing.set(true);
        set_current_element.set(Some(element));
    };

    let handle_canvas_mousemove = move |ev: ev::MouseEvent| {
        if is_panning.get() {
            if let Some(start) = pan_start.get() {
                let dx = ev.offset_x() as f64 - start.x;
                let dy = ev.offset_y() as f64 - start.y;
                set_viewport.update(|v| {
                    v.x += dx;
                    v.y += dy;
                });
                set_pan_start.set(Some(Point {
                    x: ev.offset_x() as f64,
                    y: ev.offset_y() as f64,
                }));
            }
            return;
        }

        if !is_drawing.get() {
            return;
        }

        let x = (ev.offset_x() as f64 - viewport.get().x) / viewport.get().zoom;
        let y = (ev.offset_y() as f64 - viewport.get().y) / viewport.get().zoom;

        if let Some(mut element) = current_element.get() {
            element.points.push(Point { x, y });
            set_current_element.set(Some(element));
        }
    };

    let handle_canvas_mouseup = move |_: ev::MouseEvent| {
        if is_panning.get() {
            set_is_panning.set(false);
            set_pan_start.set(None);
            return;
        }

        if is_drawing.get() {
            if let Some(element) = current_element.get()
                && !element.points.is_empty()
            {
                save_to_undo();
                set_elements.update(|els| els.push(element));
            }
            set_current_element.set(None);
            set_is_drawing.set(false);
        }
    };

    // Zoom with mouse wheel
    let handle_wheel = move |ev: ev::WheelEvent| {
        ev.prevent_default();
        let delta = ev.delta_y();
        let zoom_factor = if delta < 0.0 { 1.1 } else { 0.9 };
        let x = ev.offset_x() as f64;
        let y = ev.offset_y() as f64;

        set_viewport.update(|v| {
            let new_zoom = (v.zoom * zoom_factor).clamp(0.1, 10.0);
            v.x = x - (x - v.x) * (new_zoom / v.zoom);
            v.y = y - (y - v.y) * (new_zoom / v.zoom);
            v.zoom = new_zoom;
        });
    };

    // Zoom buttons
    let zoom_in = move |_: ev::MouseEvent| {
        set_viewport.update(|v| {
            v.zoom = (v.zoom * 1.2).min(10.0);
        });
    };

    let zoom_out = move |_: ev::MouseEvent| {
        set_viewport.update(|v| {
            v.zoom = (v.zoom / 1.2).max(0.1);
        });
    };

    let reset_view = move |_: ev::MouseEvent| {
        set_viewport.set(Viewport {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        });
    };

    // Draw on canvas
    let draw_canvas = move || {
        if let Some(canvas) = canvas_ref.get() {
            let dyn_el: web_sys::HtmlElement = canvas.into();
            if let Ok(canvas_el) = dyn_el.dyn_into::<web_sys::HtmlCanvasElement>() {
                let ctx = canvas_el
                    .get_context("2d")
                    .ok()
                    .flatten()
                    .and_then(|ctx| ctx.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());

                if let Some(ctx) = ctx {
                    let width = canvas_el.width() as f64;
                    let height = canvas_el.height() as f64;

                    // Clear canvas
                    ctx.clear_rect(0.0, 0.0, width, height);

                    // Fill background
                    ctx.set_fill_style_str("#ffffff");
                    ctx.fill_rect(0.0, 0.0, width, height);

                    let vp = viewport.get();

                    // Draw all elements
                    let all_elements = elements.get();
                    for element in &all_elements {
                        draw_element(&ctx, element, &vp);
                    }

                    // Draw current element being drawn
                    if let Some(ref element) = current_element.get() {
                        draw_element(&ctx, element, &vp);
                    }
                }
            }
        }
    };

    // Render loop
    Effect::new(move |_| {
        let _ = elements.get();
        let _ = current_element.get();
        let _ = viewport.get();
        draw_canvas();
    });

    let handle_keydown = move |ev: ev::KeyboardEvent| {
        if ev.ctrl_key() || ev.meta_key() {
            match ev.key().as_str() {
                "z" => {
                    ev.prevent_default();
                    if ev.shift_key() {
                        if let Some(next) = set_redo_stack.write().pop() {
                            let current = elements.get();
                            set_undo_stack.update(|stack| stack.push(current));
                            set_elements.set(next);
                        }
                    } else {
                        if let Some(previous) = set_undo_stack.write().pop() {
                            let current = elements.get();
                            set_redo_stack.update(|stack| stack.push(current));
                            set_elements.set(previous);
                        }
                    }
                }
                "s" => {
                    ev.prevent_default();
                    let current_elements = elements.get();
                    let current_viewport = viewport.get();
                    let name = whiteboard_name.get();
                    spawn_local(async move {
                        let elements_data: Vec<serde_json::Value> = current_elements
                            .iter()
                            .map(|e| {
                                serde_json::json!({
                                    "id": e.id,
                                    "tool": format!("{:?}", e.tool).to_lowercase(),
                                    "points": e.points.iter().map(|p| serde_json::json!({"x": p.x, "y": p.y})).collect::<Vec<_>>(),
                                    "color": e.color,
                                    "stroke_width": e.stroke_width,
                                    "text": e.text,
                                })
                            })
                            .collect();
                        let body = serde_json::json!({
                            "name": name,
                            "elements": elements_data,
                            "viewport": {
                                "x": current_viewport.x,
                                "y": current_viewport.y,
                                "zoom": current_viewport.zoom,
                            },
                        });
                        web_sys::console::log_1(&format!("Saving whiteboard: {}", body).into());
                    });
                }
                _ => {}
            }
        } else {
            match ev.key().as_str() {
                "p" => set_current_tool.set(Tool::Pen),
                "l" => set_current_tool.set(Tool::Line),
                "r" => set_current_tool.set(Tool::Rectangle),
                "c" => set_current_tool.set(Tool::Circle),
                "t" => set_current_tool.set(Tool::Text),
                "e" => set_current_tool.set(Tool::Eraser),
                _ => {}
            }
        }
    };

    view! {
        <div class="h-screen flex flex-col bg-[var(--bg-inset)] bg-[var(--bg-base)]">
            // Header
            <Header />

            // Toolbar
            <div class="flex items-center gap-2 px-4 py-2 bg-[var(--bg-surface)] border-b border-[var(--border-default)] shadow-sm">
                // Tool buttons
                <div class="flex items-center gap-1">
                    {vec![
                        Tool::Pen,
                        Tool::Line,
                        Tool::Rectangle,
                        Tool::Circle,
                        Tool::Text,
                        Tool::Eraser,
                    ].into_iter().map(|tool| {
                        let tool_for_active = tool.clone();
                        let tool_clone = tool.clone();
                        let is_active = move || current_tool.get() == tool_for_active;
                        view! {
                            <button
                                class=move || format!(
                                    "p-2 rounded-lg transition-colors {}",
                                    if is_active() {
                                        "bg-blue-100 dark:bg-blue-900 text-[var(--accent)] dark:text-[var(--accent)]"
                                    } else {
                                        "text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                                    }
                                )
                                title={tool.label()}
                                on:click=move |_| set_current_tool.set(tool_clone.clone())
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={tool.icon()} />
                                </svg>
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="w-px h-6 bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]"></div>

                // Color picker
                <div class="relative">
                    <button
                        class="p-2 rounded-lg text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                        on:click=move |_| set_show_color_picker.update(|v| *v = !*v)
                    >
                        <div class="w-5 h-5 rounded border border-[var(--border-default)]" style:background-color=move || current_color.get().clone()></div>
                    </button>
                    {move || show_color_picker.get().then(|| view! {
                        <div class="absolute top-full left-0 mt-2 p-2 bg-[var(--bg-surface)] rounded-lg shadow-lg border border-[var(--border-default)] z-50">
                            <div class="grid grid-cols-6 gap-1">
                                {PRESET_COLORS.iter().map(|color| {
                                    let c = color.to_string();
                                    view! {
                                        <button
                                            class="w-6 h-6 rounded border border-[var(--border-default)] hover:scale-110 transition-transform"
                                            style:background-color=c.clone()
                                            on:click=move |_| {
                                                set_current_color.set(c.clone());
                                                set_show_color_picker.set(false);
                                            }
                                        ></button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    })}
                </div>

                // Stroke width
                <div class="relative">
                    <button
                        class="px-2 py-1 rounded-lg text-sm font-mono text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                        on:click=move |_| set_show_stroke_picker.update(|v| *v = !*v)
                    >
                        {move || format!("{}px", stroke_width.get())}
                    </button>
                    {move || show_stroke_picker.get().then(|| view! {
                        <div class="absolute top-full left-0 mt-2 p-2 bg-[var(--bg-surface)] rounded-lg shadow-lg border border-[var(--border-default)] z-50">
                            {STROKE_WIDTHS.iter().map(|w| {
                                let sw = *w;
                                view! {
                                    <button
                                        class=move || format!(
                                            "block w-full text-left px-3 py-1 text-sm rounded hover:bg-[var(--interactive-hover)] {}",
                                            if stroke_width.get() == sw { "text-[var(--accent)] dark:text-[var(--accent)]" } else { "text-[var(--text-secondary)]" }
                                        )
                                        on:click=move |_| {
                                            set_stroke_width.set(sw);
                                            set_show_stroke_picker.set(false);
                                        }
                                    >
                                        {format!("{}px", sw)}
                                    </button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    })}
                </div>

                <div class="w-px h-6 bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]"></div>

                // Undo/Redo
                <button
                    class="p-2 rounded-lg text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] disabled:opacity-50"
                    title="Undo (Ctrl+Z)"
                    on:click=undo
                    disabled=move || undo_stack.get().is_empty()
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
                    </svg>
                </button>
                <button
                    class="p-2 rounded-lg text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)] disabled:opacity-50"
                    title="Redo (Ctrl+Shift+Z)"
                    on:click=redo
                    disabled=move || redo_stack.get().is_empty()
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 10h-10a8 8 0 00-8 8v2M21 10l-6 6m6-6l-6-6" />
                    </svg>
                </button>

                <button
                    class="p-2 rounded-lg text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                    title="Clear canvas"
                    on:click=clear_canvas
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                    </svg>
                </button>

                <div class="w-px h-6 bg-[var(--border-subtle)] dark:bg-[var(--text-tertiary)]"></div>

                // Zoom controls
                <button
                    class="p-2 rounded-lg text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                    on:click=zoom_out
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
                    </svg>
                </button>
                <button
                    class="px-2 py-1 rounded text-sm font-mono text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                    on:click=reset_view
                >
                    {move || format!("{:.0}%", viewport.get().zoom * 100.0)}
                </button>
                <button
                    class="p-2 rounded-lg text-[var(--text-secondary)] hover:bg-[var(--interactive-hover)]"
                    on:click=zoom_in
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                    </svg>
                </button>

                <div class="flex-1"></div>

                // Save/Export
                <button
                    class="px-3 py-1.5 text-sm bg-[var(--bg-inset)] bg-[var(--bg-surface-raised)] text-[var(--text-secondary)] rounded-lg hover:bg-[var(--border-subtle)] hover:bg-[var(--interactive-hover)]"
                    on:click=export_png
                >
                    "Export PNG"
                </button>
                <button
                    class="px-3 py-1.5 text-sm bg-[var(--accent)] text-[var(--text-on-accent)] rounded-lg hover:bg-[var(--accent-hover)]"
                    on:click=save_to_server
                >
                    "Save"
                </button>
            </div>

            // Canvas
            <div
                node_ref=container_ref
                class="flex-1 overflow-hidden cursor-crosshair"
                on:keydown=handle_keydown
                tabindex="0"
            >
                <canvas
                    node_ref=canvas_ref
                    width="1920"
                    height="1080"
                    class="w-full h-full"
                    on:mousedown=handle_canvas_mousedown
                    on:mousemove=handle_canvas_mousemove
                    on:mouseup=handle_canvas_mouseup
                    on:mouseleave=handle_canvas_mouseup
                    on:wheel=handle_wheel
                ></canvas>
            </div>
        </div>
    }
}

/// Draw a single whiteboard element on the canvas.
fn draw_element(ctx: &web_sys::CanvasRenderingContext2d, element: &WhiteboardElement, viewport: &Viewport) {
    if element.points.is_empty() {
        return;
    }

    ctx.set_stroke_style_str(&element.color);
    ctx.set_line_width(element.stroke_width * viewport.zoom);
    ctx.set_fill_style_str(&element.color);

    match element.tool {
        Tool::Pen => {
            ctx.begin_path();
            let first = &element.points[0];
            ctx.move_to(
                first.x * viewport.zoom + viewport.x,
                first.y * viewport.zoom + viewport.y,
            );
            for point in &element.points[1..] {
                ctx.line_to(
                    point.x * viewport.zoom + viewport.x,
                    point.y * viewport.zoom + viewport.y,
                );
            }
            ctx.stroke();
        }
        Tool::Line => {
            if element.points.len() >= 2 {
                ctx.begin_path();
                let first = &element.points[0];
                let last = &element.points[element.points.len() - 1];
                ctx.move_to(
                    first.x * viewport.zoom + viewport.x,
                    first.y * viewport.zoom + viewport.y,
                );
                ctx.line_to(last.x * viewport.zoom + viewport.x, last.y * viewport.zoom + viewport.y);
                ctx.stroke();
            }
        }
        Tool::Rectangle => {
            if element.points.len() >= 2 {
                let first = &element.points[0];
                let last = &element.points[element.points.len() - 1];
                let x = first.x.min(last.x) * viewport.zoom + viewport.x;
                let y = first.y.min(last.y) * viewport.zoom + viewport.y;
                let width = (last.x - first.x).abs() * viewport.zoom;
                let height = (last.y - first.y).abs() * viewport.zoom;
                ctx.stroke_rect(x, y, width, height);
            }
        }
        Tool::Circle => {
            if element.points.len() >= 2 {
                let first = &element.points[0];
                let last = &element.points[element.points.len() - 1];
                let cx = (first.x + last.x) / 2.0 * viewport.zoom + viewport.x;
                let cy = (first.y + last.y) / 2.0 * viewport.zoom + viewport.y;
                let rx = (last.x - first.x).abs() / 2.0 * viewport.zoom;
                let ry = (last.y - first.y).abs() / 2.0 * viewport.zoom;
                ctx.begin_path();
                ctx.ellipse(cx, cy, rx, ry, 0.0, 0.0, 2.0 * std::f64::consts::PI).ok();
                ctx.stroke();
            }
        }
        Tool::Text => {
            if let Some(ref text) = element.text {
                let point = &element.points[0];
                let x = point.x * viewport.zoom + viewport.x;
                let y = point.y * viewport.zoom + viewport.y;
                ctx.set_font(&format!("{}px sans-serif", element.stroke_width * 4.0 * viewport.zoom));
                ctx.fill_text(text, x, y).ok();
            }
        }
        Tool::Eraser => {
            ctx.set_global_composite_operation("destination-out").ok();
            ctx.begin_path();
            let first = &element.points[0];
            ctx.move_to(
                first.x * viewport.zoom + viewport.x,
                first.y * viewport.zoom + viewport.y,
            );
            for point in &element.points[1..] {
                ctx.line_to(
                    point.x * viewport.zoom + viewport.x,
                    point.y * viewport.zoom + viewport.y,
                );
            }
            ctx.stroke();
            ctx.set_global_composite_operation("source-over").ok();
        }
    }
}
