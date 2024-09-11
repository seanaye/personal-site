use leptos::{ev::resize, html::Div, prelude::*};
use leptos_use::{use_debounce_fn, use_event_listener, use_window};
use num_traits::FromPrimitive;

#[derive(Clone, Copy)]
pub struct UseWindowSizeReturn {
    pub width: ReadSignal<f64>,
    pub height: ReadSignal<f64>,
}
pub fn use_elem_size(el: NodeRef<Div>) -> UseWindowSizeReturn {
    let window = use_window();
    let (width, set_width) = signal(0.0);

    let (height, set_height) = signal(0.0);

    let update = move || {
        let d = el.get_untracked().expect("div not loaded");

        let w = f64::from_i32(d.client_width()).unwrap();
        let h = f64::from_i32(d.client_height()).unwrap();

        if width.get_untracked() != w {
            set_width(w);
        }
        if height.get_untracked() != h {
            set_height(h)
        }
    };

    Effect::new(move |_| update());

    let debounced = use_debounce_fn(update, 500.0);

    let _ = use_event_listener(window, resize, move |_| {
        debounced();
    });

    UseWindowSizeReturn { width, height }
}
