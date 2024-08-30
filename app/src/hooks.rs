use leptos::{create_signal, ev::resize, on_cleanup, ReadSignal};
use leptos_use::{use_debounce_fn, use_event_listener, use_window};

#[derive(Clone, Copy)]
pub struct UseWindowSizeReturn {
    pub width: ReadSignal<f64>,
    pub height: ReadSignal<f64>,
}
pub fn use_window_size() -> UseWindowSizeReturn {
    let window = use_window();
    let (width, set_width) = create_signal(
        window
            .as_ref()
            .and_then(|w| w.inner_width().ok())
            .and_then(|val| val.as_f64())
            .unwrap_or_default(),
    );

    let (height, set_height) = create_signal(
        window
            .as_ref()
            .and_then(|w| w.inner_height().ok())
            .and_then(|val| val.as_f64())
            .unwrap_or_default(),
    );

    let debounced = use_debounce_fn(
        move || {
            let window = use_window();
            let Some(w) = window.as_ref() else {
                return;
            };

            set_width(w.inner_width().unwrap().as_f64().unwrap());
            set_height(w.inner_height().unwrap().as_f64().unwrap());
        },
        500.0,
    );

    let cleanup = use_event_listener(window, resize, move |_| {
        debounced();
    });

    on_cleanup(cleanup);

    UseWindowSizeReturn { width, height }
}
