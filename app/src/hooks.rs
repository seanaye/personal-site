use leptos::{html::Div, prelude::*};
use num_traits::FromPrimitive;

#[derive(Clone, Copy)]
pub struct UseWindowSizeReturn {
    pub width: ReadSignal<f64>,
    pub height: ReadSignal<f64>,
}

pub fn use_elem_size(el: NodeRef<Div>) -> UseWindowSizeReturn {
    let (width, set_width) = signal(0.0);
    let (height, set_height) = signal(0.0);

    let update = move || {
        let Some(d) = el.get_untracked() else {
            return;
        };

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

    // Browser-only: attach resize listener
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;

        let timeout_handle: std::cell::Cell<Option<i32>> = std::cell::Cell::new(None);
        let debounced_update = move || {
            if let Some(h) = timeout_handle.get() {
                web_sys::window().unwrap().clear_timeout_with_handle(h);
            }
            #[allow(clippy::redundant_closure)]
            let cb = Closure::once_into_js(move || update());
            let h = web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    500,
                )
                .unwrap();
            timeout_handle.set(Some(h));
        };

        let cb = Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
            debounced_update();
        });
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    UseWindowSizeReturn { width, height }
}
