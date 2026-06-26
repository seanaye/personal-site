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

    // Browser-only: keep the canvas backing size in sync with its CSS size.
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;

        let cb = Closure::<dyn Fn(web_sys::Event)>::new(move |_: web_sys::Event| {
            update();
        });
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref())
            .unwrap();
        cb.forget();
    }

    UseWindowSizeReturn { width, height }
}
