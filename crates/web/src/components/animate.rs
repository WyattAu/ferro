use leptos::prelude::*;
use leptos::html;
use wasm_bindgen::JsCast;

fn set_css(el: &web_sys::Element, prop: &str, val: &str) {
    if let Ok(html_el) = el.clone().dyn_into::<web_sys::HtmlElement>() {
        let _ = html_el.style().set_property(prop, val);
    }
}

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
        .and_then(|opt| opt)
        .map(|m| m.matches())
        .unwrap_or(false)
}

#[component]
pub fn FadeIn(
    #[prop(default = 300)]
    duration_ms: u32,
    #[prop(default = 0)]
    delay_ms: u32,
    #[prop(default = "ease-out".to_string())]
    easing: String,
    #[prop(default = true)]
    enabled: bool,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if effective_enabled {
                let transition = format!("opacity {}ms {} {}ms", duration_ms, easing, delay_ms);
                set_css(&el, "opacity", "0");
                set_css(&el, "transition", &transition);
                set_css(&el, "will-change", "opacity");
                request_animation_frame(move || {
                    set_css(&el, "opacity", "1");
                });
            } else {
                set_css(&el, "opacity", "1");
            }
        }
    });

    view! {
        <div node_ref=container_ref>
            {children()}
        </div>
    }
}

#[component]
pub fn FadeOut(
    #[prop(default = 300)]
    duration_ms: u32,
    #[prop(default = 0)]
    delay_ms: u32,
    #[prop(default = "ease-in".to_string())]
    easing: String,
    #[prop(default = true)]
    enabled: bool,
    active: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if active.get() && effective_enabled {
                let transition = format!("opacity {}ms {} {}ms", duration_ms, easing, delay_ms);
                set_css(&el, "opacity", "1");
                set_css(&el, "transition", &transition);
                set_css(&el, "will-change", "opacity");
                request_animation_frame(move || {
                    set_css(&el, "opacity", "0");
                });
            } else if !active.get() {
                set_css(&el, "opacity", "1");
            }
        }
    });

    view! {
        <div node_ref=container_ref style=move || if active.get() && effective_enabled {
            "pointer-events: none"
        } else {
            ""
        }>
            {children()}
        </div>
    }
}

#[component]
pub fn SlideIn(
    #[prop(default = SlideDirection::Bottom)]
    direction: SlideDirection,
    #[prop(default = 20)]
    distance_px: i32,
    #[prop(default = 300)]
    duration_ms: u32,
    #[prop(default = 0)]
    delay_ms: u32,
    #[prop(default = "ease-out".to_string())]
    easing: String,
    #[prop(default = true)]
    enabled: bool,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if effective_enabled {
                let (ix, iy) = match direction {
                    SlideDirection::Top => (0, -distance_px),
                    SlideDirection::Bottom => (0, distance_px),
                    SlideDirection::Left => (-distance_px, 0),
                    SlideDirection::Right => (distance_px, 0),
                };
                let initial = format!("translate({}px, {}px)", ix, iy);
                let transition = format!(
                    "transform {}ms {}, opacity {}ms {} {}ms",
                    duration_ms, easing, duration_ms, easing, delay_ms
                );
                set_css(&el, "transform", &initial);
                set_css(&el, "opacity", "0");
                set_css(&el, "transition", &transition);
                set_css(&el, "will-change", "transform, opacity");
                request_animation_frame(move || {
                    set_css(&el, "transform", "translate(0px, 0px)");
                    set_css(&el, "opacity", "1");
                });
            } else {
                set_css(&el, "transform", "translate(0px, 0px)");
                set_css(&el, "opacity", "1");
            }
        }
    });

    view! {
        <div node_ref=container_ref>
            {children()}
        </div>
    }
}

#[component]
pub fn SlideOut(
    #[prop(default = SlideDirection::Bottom)]
    direction: SlideDirection,
    #[prop(default = 20)]
    distance_px: i32,
    #[prop(default = 300)]
    duration_ms: u32,
    #[prop(default = 0)]
    delay_ms: u32,
    #[prop(default = "ease-in".to_string())]
    easing: String,
    #[prop(default = true)]
    enabled: bool,
    active: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if active.get() && effective_enabled {
                let (tx, ty) = match direction {
                    SlideDirection::Top => (0, -distance_px),
                    SlideDirection::Bottom => (0, distance_px),
                    SlideDirection::Left => (-distance_px, 0),
                    SlideDirection::Right => (distance_px, 0),
                };
                let target = format!("translate({}px, {}px)", tx, ty);
                let transition = format!(
                    "transform {}ms {}, opacity {}ms {} {}ms",
                    duration_ms, easing, duration_ms, easing, delay_ms
                );
                set_css(&el, "transform", "translate(0px, 0px)");
                set_css(&el, "opacity", "1");
                set_css(&el, "transition", &transition);
                request_animation_frame(move || {
                    set_css(&el, "transform", &target);
                    set_css(&el, "opacity", "0");
                });
            } else if !active.get() {
                set_css(&el, "transform", "translate(0px, 0px)");
                set_css(&el, "opacity", "1");
            }
        }
    });

    view! {
        <div node_ref=container_ref style=move || if active.get() && effective_enabled {
            "pointer-events: none"
        } else {
            ""
        }>
            {children()}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlideDirection {
    Top,
    Bottom,
    Left,
    Right,
}

#[component]
pub fn ScaleIn(
    #[prop(default = 0.95)]
    from_scale: f64,
    #[prop(default = 200)]
    duration_ms: u32,
    #[prop(default = 0)]
    delay_ms: u32,
    #[prop(default = "ease-out".to_string())]
    easing: String,
    #[prop(default = true)]
    enabled: bool,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if effective_enabled {
                let initial = format!("scale({})", from_scale);
                let transition = format!(
                    "transform {}ms {}, opacity {}ms {} {}ms",
                    duration_ms, easing, duration_ms, easing, delay_ms
                );
                set_css(&el, "transform", &initial);
                set_css(&el, "opacity", "0");
                set_css(&el, "transform-origin", "center");
                set_css(&el, "transition", &transition);
                set_css(&el, "will-change", "transform, opacity");
                request_animation_frame(move || {
                    set_css(&el, "transform", "scale(1)");
                    set_css(&el, "opacity", "1");
                });
            } else {
                set_css(&el, "transform", "scale(1)");
                set_css(&el, "opacity", "1");
            }
        }
    });

    view! {
        <div node_ref=container_ref>
            {children()}
        </div>
    }
}

#[component]
pub fn ScaleOut(
    #[prop(default = 0.95)]
    to_scale: f64,
    #[prop(default = 200)]
    duration_ms: u32,
    #[prop(default = 0)]
    delay_ms: u32,
    #[prop(default = "ease-in".to_string())]
    easing: String,
    #[prop(default = true)]
    enabled: bool,
    active: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if active.get() && effective_enabled {
                let target = format!("scale({})", to_scale);
                let transition = format!(
                    "transform {}ms {}, opacity {}ms {} {}ms",
                    duration_ms, easing, duration_ms, easing, delay_ms
                );
                set_css(&el, "transform", "scale(1)");
                set_css(&el, "opacity", "1");
                set_css(&el, "transform-origin", "center");
                set_css(&el, "transition", &transition);
                request_animation_frame(move || {
                    set_css(&el, "transform", &target);
                    set_css(&el, "opacity", "0");
                });
            } else if !active.get() {
                set_css(&el, "transform", "scale(1)");
                set_css(&el, "opacity", "1");
            }
        }
    });

    view! {
        <div node_ref=container_ref style=move || if active.get() && effective_enabled {
            "pointer-events: none"
        } else {
            ""
        }>
            {children()}
        </div>
    }
}

#[component]
pub fn StaggerChildren(
    #[prop(default = 50)]
    stagger_ms: u32,
    #[prop(default = 300)]
    duration_ms: u32,
    #[prop(default = true)]
    enabled: bool,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if effective_enabled {
                set_css(&el, "--stagger-duration", &format!("{}ms", duration_ms));
                set_css(&el, "--stagger-step", &format!("{}ms", stagger_ms));
            }
        }
    });

    view! {
        <div
            node_ref=container_ref
            class="stagger-children"
            style="display: contents;"
        >
            {children()}
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StaggerAnimation {
    Fade,
    SlideUp,
    Scale,
}

fn request_animation_frame(f: impl FnOnce() + 'static) {
    use std::cell::RefCell;
    use std::rc::Rc;

    let f = Rc::new(RefCell::new(Some(f)));
    let f2 = f.clone();

    let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
        if let Some(f) = f2.borrow_mut().take() {
            f();
        }
    });

    web_sys::window()
        .unwrap()
        .request_animation_frame(cb.unchecked_ref())
        .unwrap();
}
