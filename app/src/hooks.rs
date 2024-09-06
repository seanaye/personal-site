use leptos::{ev::resize, prelude::*};
use leptos_use::{use_debounce_fn, use_event_listener, use_window};

#[derive(Clone, Copy)]
pub struct UseWindowSizeReturn {
    pub width: ReadSignal<f64>,
    pub height: ReadSignal<f64>,
}
pub fn use_window_size() -> UseWindowSizeReturn {
    let window = use_window();
    let (width, set_width) = signal(0.0);

    let (height, set_height) = signal(0.0);

    let update = move || {
        let window = use_window();
        let Some(w) = window.as_ref() else {
            return;
        };

        set_width(w.inner_width().unwrap().as_f64().unwrap());
        set_height(w.inner_height().unwrap().as_f64().unwrap());
    };

    Effect::new(move |_| update());

    let debounced = use_debounce_fn(update, 500.0);

    let _ = use_event_listener(window, resize, move |_| {
        debounced();
    });

    UseWindowSizeReturn { width, height }
}
