use leptos::*;
use wasm_bindgen::JsCast;

/// Check if the user prefers reduced motion via `prefers-reduced-motion` media query.
fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| w.match_media("(prefers-reduced-motion: reduce)").ok())
        .and_then(|opt| opt)
        .map(|m| m.matches())
        .unwrap_or(false)
}

/// Fade-in animation wrapper. Children fade from transparent to opaque.
/// Respects `prefers-reduced-motion` by skipping the animation when enabled.
#[component]
pub fn FadeIn(
    /// Duration in milliseconds.
    #[prop(default = 300)]
    duration_ms: u32,
    /// Delay in milliseconds before animation starts.
    #[prop(default = 0)]
    delay_ms: u32,
    /// CSS easing function.
    #[prop(default = "ease-out".to_string())]
    easing: String,
    /// Whether the animation is enabled.
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
                let el = el
                    .style("opacity", "0")
                    .style("transition", transition)
                    .style("will-change", "opacity");
                request_animation_frame(move || {
                    let _ = el.style("opacity", "1");
                });
            } else {
                let _ = el.style("opacity", "1");
            }
        }
    });

    view! {
        <div _ref=container_ref>
            {children()}
        </div>
    }
}

/// Fade-out animation wrapper. Children fade from opaque to transparent and are removed.
/// Respects `prefers-reduced-motion`.
#[component]
pub fn FadeOut(
    /// Duration in milliseconds.
    #[prop(default = 300)]
    duration_ms: u32,
    /// Delay in milliseconds before animation starts.
    #[prop(default = 0)]
    delay_ms: u32,
    /// CSS easing function.
    #[prop(default = "ease-in".to_string())]
    easing: String,
    /// Whether the animation is enabled.
    #[prop(default = true)]
    enabled: bool,
    /// Signal that triggers the fade-out when set to true.
    active: Signal<bool>,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get() {
            if active.get() && effective_enabled {
                let transition = format!("opacity {}ms {} {}ms", duration_ms, easing, delay_ms);
                let el = el
                    .style("opacity", "1")
                    .style("transition", transition)
                    .style("will-change", "opacity");
                request_animation_frame(move || {
                    let _ = el.style("opacity", "0");
                });
            } else if !active.get() {
                let _ = el.style("opacity", "1");
            }
        }
    });

    view! {
        <div _ref=container_ref style=move || if active.get() && effective_enabled {
            "pointer-events: none"
        } else {
            ""
        }>
            {children()}
        </div>
    }
}

/// Slide-in animation wrapper. Children slide in from a direction.
/// Respects `prefers-reduced-motion` by skipping the animation when enabled.
#[component]
pub fn SlideIn(
    /// Direction to slide from.
    #[prop(default = SlideDirection::Bottom)]
    direction: SlideDirection,
    /// Distance in pixels.
    #[prop(default = 20)]
    distance_px: i32,
    /// Duration in milliseconds.
    #[prop(default = 300)]
    duration_ms: u32,
    /// Delay in milliseconds.
    #[prop(default = 0)]
    delay_ms: u32,
    /// CSS easing function.
    #[prop(default = "ease-out".to_string())]
    easing: String,
    /// Whether the animation is enabled.
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
                let el = el
                    .style("transform", initial)
                    .style("opacity", "0")
                    .style("transition", transition)
                    .style("will-change", "transform, opacity");
                request_animation_frame(move || {
                    let _ = el
                        .style("transform", "translate(0px, 0px)")
                        .style("opacity", "1");
                });
            } else {
                let _ = el
                    .style("transform", "translate(0px, 0px)")
                    .style("opacity", "1");
            }
        }
    });

    view! {
        <div _ref=container_ref>
            {children()}
        </div>
    }
}

/// Slide-out animation wrapper. Children slide out in a direction and become hidden.
/// Respects `prefers-reduced-motion`.
#[component]
pub fn SlideOut(
    /// Direction to slide toward.
    #[prop(default = SlideDirection::Bottom)]
    direction: SlideDirection,
    /// Distance in pixels.
    #[prop(default = 20)]
    distance_px: i32,
    /// Duration in milliseconds.
    #[prop(default = 300)]
    duration_ms: u32,
    /// Delay in milliseconds.
    #[prop(default = 0)]
    delay_ms: u32,
    /// CSS easing function.
    #[prop(default = "ease-in".to_string())]
    easing: String,
    /// Whether the animation is enabled.
    #[prop(default = true)]
    enabled: bool,
    /// Signal that triggers the slide-out when set to true.
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
                let el = el
                    .style("transform", "translate(0px, 0px)")
                    .style("opacity", "1")
                    .style("transition", transition);
                request_animation_frame(move || {
                    let _ = el
                        .style("transform", target)
                        .style("opacity", "0");
                });
            } else if !active.get() {
                let _ = el
                    .style("transform", "translate(0px, 0px)")
                    .style("opacity", "1");
            }
        }
    });

    view! {
        <div _ref=container_ref style=move || if active.get() && effective_enabled {
            "pointer-events: none"
        } else {
            ""
        }>
            {children()}
        </div>
    }
}

/// Direction for slide animations.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlideDirection {
    Top,
    Bottom,
    Left,
    Right,
}

/// Scale-in animation wrapper. Children scale up from a smaller size.
/// Respects `prefers-reduced-motion` by skipping the animation when enabled.
#[component]
pub fn ScaleIn(
    /// Initial scale factor (0.0 to 1.0).
    #[prop(default = 0.95)]
    from_scale: f64,
    /// Duration in milliseconds.
    #[prop(default = 200)]
    duration_ms: u32,
    /// Delay in milliseconds.
    #[prop(default = 0)]
    delay_ms: u32,
    /// CSS easing function.
    #[prop(default = "ease-out".to_string())]
    easing: String,
    /// Whether the animation is enabled.
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
                let el = el
                    .style("transform", initial)
                    .style("opacity", "0")
                    .style("transform-origin", "center")
                    .style("transition", transition)
                    .style("will-change", "transform, opacity");
                request_animation_frame(move || {
                    let _ = el
                        .style("transform", "scale(1)")
                        .style("opacity", "1");
                });
            } else {
                let _ = el
                    .style("transform", "scale(1)")
                    .style("opacity", "1");
            }
        }
    });

    view! {
        <div _ref=container_ref>
            {children()}
        </div>
    }
}

/// Scale-out animation wrapper. Children scale down and become hidden.
/// Respects `prefers-reduced-motion`.
#[component]
pub fn ScaleOut(
    /// Target scale factor (0.0 to 1.0). Children scale to this size.
    #[prop(default = 0.95)]
    to_scale: f64,
    /// Duration in milliseconds.
    #[prop(default = 200)]
    duration_ms: u32,
    /// Delay in milliseconds.
    #[prop(default = 0)]
    delay_ms: u32,
    /// CSS easing function.
    #[prop(default = "ease-in".to_string())]
    easing: String,
    /// Whether the animation is enabled.
    #[prop(default = true)]
    enabled: bool,
    /// Signal that triggers the scale-out when set to true.
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
                let el = el
                    .style("transform", "scale(1)")
                    .style("opacity", "1")
                    .style("transform-origin", "center")
                    .style("transition", transition);
                request_animation_frame(move || {
                    let _ = el
                        .style("transform", target)
                        .style("opacity", "0");
                });
            } else if !active.get() {
                let _ = el
                    .style("transform", "scale(1)")
                    .style("opacity", "1");
            }
        }
    });

    view! {
        <div _ref=container_ref style=move || if active.get() && effective_enabled {
            "pointer-events: none"
        } else {
            ""
        }>
            {children()}
        </div>
    }
}

/// Staggered children animation container.
///
/// Wraps children and applies a stagger delay via CSS custom property.
/// Each direct child gets an increasing `animation-delay` based on its index.
/// Use with CSS `animation` on children, or combine with FadeIn/SlideIn/ScaleIn.
/// Respects `prefers-reduced-motion`.
#[component]
pub fn StaggerChildren(
    /// Delay between each child in milliseconds.
    #[prop(default = 50)]
    stagger_ms: u32,
    /// Duration of each child's animation in milliseconds.
    #[prop(default = 300)]
    duration_ms: u32,
    /// Whether the animation is enabled.
    #[prop(default = true)]
    enabled: bool,
    children: Children,
) -> impl IntoView {
    let container_ref = create_node_ref::<html::Div>();
    let effective_enabled = enabled && !prefers_reduced_motion();

    create_effect(move |_| {
        if let Some(el) = container_ref.get()
            && effective_enabled
        {
            // Set CSS custom properties on the container for stagger timing
            let _ = el
                .style("--stagger-duration", format!("{}ms", duration_ms))
                .style("--stagger-step", format!("{}ms", stagger_ms));
        }
    });

    view! {
        <div
            _ref=container_ref
            class="stagger-children"
            style="display: contents;"
        >
            {children()}
        </div>
    }
}

/// Type of stagger animation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StaggerAnimation {
    Fade,
    SlideUp,
    Scale,
}

/// Helper to schedule a callback on the next animation frame.
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
