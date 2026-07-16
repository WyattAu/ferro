use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}

pub struct SwipeGesture {
    start_x: f64,
    start_y: f64,
    threshold: f64,
}

impl SwipeGesture {
    pub fn new(threshold: f64) -> Self {
        Self {
            start_x: 0.0,
            start_y: 0.0,
            threshold,
        }
    }

    pub fn start(&mut self, x: f64, y: f64) {
        self.start_x = x;
        self.start_y = y;
    }

    pub fn detect(&self, x: f64, y: f64) -> Option<SwipeDirection> {
        let dx = x - self.start_x;
        let dy = y - self.start_y;

        if dx.abs() < self.threshold && dy.abs() < self.threshold {
            return None;
        }

        if dx.abs() > dy.abs() {
            if dx > self.threshold {
                Some(SwipeDirection::Right)
            } else if dx < -self.threshold {
                Some(SwipeDirection::Left)
            } else {
                None
            }
        } else if dy > self.threshold {
            Some(SwipeDirection::Down)
        } else if dy < -self.threshold {
            Some(SwipeDirection::Up)
        } else {
            None
        }
    }
}

pub struct PinchGesture {
    initial_distance: f64,
    current_scale: f64,
}

impl Default for PinchGesture {
    fn default() -> Self {
        Self::new()
    }
}

impl PinchGesture {
    pub fn new() -> Self {
        Self {
            initial_distance: 0.0,
            current_scale: 1.0,
        }
    }

    pub fn start(&mut self, distance: f64) {
        self.initial_distance = distance;
        self.current_scale = 1.0;
    }

    pub fn update(&mut self, distance: f64) -> f64 {
        if self.initial_distance > 0.0 {
            self.current_scale = distance / self.initial_distance;
        }
        self.current_scale
    }
}

pub fn attach_swipe_gesture(
    element: &web_sys::Element,
    threshold: f64,
    on_swipe: impl FnMut(SwipeDirection) + 'static,
) {
    let swipe_state = Rc::new(RefCell::new((SwipeGesture::new(threshold), false)));

    let on_swipe = Rc::new(RefCell::new(on_swipe));

    let state_clone = swipe_state.clone();
    let on_pointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
        let mut state = state_clone.borrow_mut();
        state.0.start(ev.client_x() as f64, ev.client_y() as f64);
        state.1 = true;
    });

    let state_clone = swipe_state.clone();
    let on_swipe_clone = on_swipe.clone();
    let on_pointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
        let mut state = state_clone.borrow_mut();
        if state.1 {
            state.1 = false;
            let x = ev.client_x() as f64;
            let y = ev.client_y() as f64;
            if let Some(direction) = state.0.detect(x, y) {
                drop(state);
                on_swipe_clone.borrow_mut()(direction);
            }
        }
    });

    let on_pointermove = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |_ev: web_sys::PointerEvent| {});

    let _ = element.add_event_listener_with_callback("pointerdown", on_pointerdown.as_ref().unchecked_ref());
    let _ = element.add_event_listener_with_callback("pointerup", on_pointerup.as_ref().unchecked_ref());
    let _ = element.add_event_listener_with_callback("pointermove", on_pointermove.as_ref().unchecked_ref());

    on_pointerdown.forget();
    on_pointerup.forget();
    on_pointermove.forget();
}

pub fn attach_pinch_gesture(element: &web_sys::Element, on_scale: impl FnMut(f64) + 'static) {
    let pinch_state = Rc::new(RefCell::new((PinchGesture::new(), Vec::<(i32, f64, f64)>::new())));
    let on_scale = Rc::new(RefCell::new(on_scale));

    let state_clone = pinch_state.clone();
    let on_pointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
        let mut state = state_clone.borrow_mut();
        state
            .1
            .push((ev.pointer_id(), ev.client_x() as f64, ev.client_y() as f64));

        if state.1.len() == 2 {
            let dx = state.1[0].1 - state.1[1].1;
            let dy = state.1[0].2 - state.1[1].2;
            let distance = (dx * dx + dy * dy).sqrt();
            state.0.start(distance);
        }
    });

    let state_clone = pinch_state.clone();
    let on_scale_clone = on_scale.clone();
    let on_pointermove = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
        let mut state = state_clone.borrow_mut();
        if state.1.len() == 2 {
            if let Some(p) = state.1.iter_mut().find(|p| p.0 == ev.pointer_id()) {
                p.1 = ev.client_x() as f64;
                p.2 = ev.client_y() as f64;
            }
            let dx = state.1[0].1 - state.1[1].1;
            let dy = state.1[0].2 - state.1[1].2;
            let distance = (dx * dx + dy * dy).sqrt();
            let scale = state.0.update(distance);
            drop(state);
            on_scale_clone.borrow_mut()(scale);
        }
    });

    let state_clone = pinch_state.clone();
    let on_pointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |ev: web_sys::PointerEvent| {
        let mut state = state_clone.borrow_mut();
        state.1.retain(|p| p.0 != ev.pointer_id());
        if state.1.len() < 2 {
            state.0.start(0.0);
        }
    });

    let _ = element.add_event_listener_with_callback("pointerdown", on_pointerdown.as_ref().unchecked_ref());
    let _ = element.add_event_listener_with_callback("pointermove", on_pointermove.as_ref().unchecked_ref());
    let _ = element.add_event_listener_with_callback("pointerup", on_pointerup.as_ref().unchecked_ref());
    let _ = element.add_event_listener_with_callback("pointercancel", on_pointerup.as_ref().unchecked_ref());

    on_pointerdown.forget();
    on_pointermove.forget();
    on_pointerup.forget();
}

pub fn attach_tap_gesture(
    element: &web_sys::Element,
    double_tap_threshold_ms: f64,
    mut on_tap: impl FnMut(bool) + 'static,
) {
    let last_tap = Rc::new(RefCell::new(0.0_f64));
    let tap_active = Rc::new(RefCell::new(false));

    let tap_active_clone = tap_active.clone();
    let on_pointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |_ev: web_sys::PointerEvent| {
        *tap_active_clone.borrow_mut() = true;
    });

    let last_tap_clone = last_tap.clone();
    let tap_active_clone = tap_active.clone();
    let on_pointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |_ev: web_sys::PointerEvent| {
        if *tap_active_clone.borrow_mut() {
            *tap_active_clone.borrow_mut() = false;
            let now = js_sys::Date::now();
            let mut lt = last_tap_clone.borrow_mut();
            let is_double = *lt > 0.0 && (now - *lt) < double_tap_threshold_ms;
            *lt = now;
            drop(lt);
            on_tap(is_double);
        }
    });

    let _ = element.add_event_listener_with_callback("pointerdown", on_pointerdown.as_ref().unchecked_ref());
    let _ = element.add_event_listener_with_callback("pointerup", on_pointerup.as_ref().unchecked_ref());

    on_pointerdown.forget();
    on_pointerup.forget();
}
