use grid::Coord;
use leptos::{html, prelude::*};
use num_traits::FromPrimitive;
use std::{cell::Cell, rc::Rc, time::Duration};
use wasm_bindgen::prelude::*;

use crate::{
    canvas_grid::{Event, EventState, PolineManager, PolineManagerImpl},
    hooks::{use_elem_size, UseWindowSizeReturn},
};
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use crate::wgpu_renderer::WgpuLiquidRenderer;

const HUE_STORAGE_KEY: &str = "liquid-hue";

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn stored_hue_value() -> Option<f64> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(HUE_STORAGE_KEY).ok().flatten())
        .and_then(|value| value.parse().ok())
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
fn stored_hue_value() -> Option<f64> {
    None
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn store_hue_value(value: f64) {
    if let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        _ = storage.set_item(HUE_STORAGE_KEY, &value.to_string());
    }
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
fn store_hue_value(_value: f64) {}

#[island]
pub fn Slider() -> impl IntoView {
    let slider_update: SliderHue = expect_context();

    view! {
        <input
            type="range"
            min=0
            max=360
            autocomplete="off"
            class="accent-stone-700"
            prop:value=move || slider_update.hue_value.get()
            on:input=move |ev| {
                let v: f64 = event_target_value(&ev).parse().unwrap();
                store_hue_value(v);
                slider_update.set_hue_value.set(v)
            }
        />
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SliderHue {
    pub hue_value: ReadSignal<f64>,
    pub set_hue_value: WriteSignal<f64>,
    pub poline: Memo<PolineManagerImpl>,
}

pub fn use_provide_slider_hue() -> SliderHue {
    let initial_hue = stored_hue_value().unwrap_or(0.0);
    let (hue_value, set_hue_value) = signal(initial_hue);

    let colours = Memo::new_owning(move |last| {
        let value = hue_value.get();
        let Some(mut last) = last else {
            return (PolineManagerImpl::new(value), true);
        };

        let prev = *last.abs_hue();
        last.set_hue(value);
        let neq = &prev != last.abs_hue();
        (last, neq)
    });

    let slider_hue = SliderHue {
        hue_value,
        set_hue_value,
        poline: colours,
    };
    provide_context(slider_hue);

    slider_hue
}

pub fn expect_slider_hue() -> SliderHue {
    expect_context()
}

#[island]
pub fn SliderProvider(children: Children) -> impl IntoView {
    use_provide_slider_hue();
    children()
}

#[island]
pub fn Canvas(children: Children) -> impl IntoView {
    let outer_size: NodeRef<html::Div> = NodeRef::new();
    let canvas_ref: NodeRef<html::Canvas> = NodeRef::new();

    let UseWindowSizeReturn { width, height } = use_elem_size(outer_size);

    let (events, set_events) = signal(EventState::default());

    let clear_events = move || set_events.update(|ev| ev.clear_events());

    let (document_hidden, set_document_hidden) = signal(false);

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            set_document_hidden.set(document.hidden());

            let listener = gloo::events::EventListener::new(&document, "visibilitychange", move |_| {
                let hidden = web_sys::window()
                    .and_then(|window| window.document())
                    .is_some_and(|document| document.hidden());

                set_document_hidden.set(hidden);
                if hidden {
                    set_events.update(|ev| ev.clear_events());
                }
            });
            on_cleanup(move || drop(listener));
        }
    }

    let (cancel_count, set_cancel_count) = signal(0);

    let SliderHue { poline, .. } = expect_slider_hue();

    Effect::new(move |val: Option<bool>| {
        width.with(|_| {});
        height.with(|_| {});
        if val.is_some() {
            set_events.update(|e| e.cancel())
        }
        true
    });

    let on_cancel = move || {
        log::info!("aborted compute events");
        set_cancel_count.update(|c| {
            *c += 1;
        });
        set_events.update(|e| e.reset_cancel_state());
    };

    Effect::new(move |val: Option<Result<IntervalHandle, JsValue>>| {
        cancel_count.read();
        let hidden = document_hidden.get();
        if let Some(Ok(interval)) = &val {
            interval.clear();
        }

        set_events.set(EventState::default());

        let w = width.get_untracked();
        let h = height.get_untracked();
        let dots_width = usize::from_f64(w).unwrap_or(0);
        let dots_height = usize::from_f64(h).unwrap_or(0);

        if hidden || dots_width == 0 || dots_height == 0 {
            return Err(JsValue::NULL);
        }

        let last_random_drop_ms = Rc::new(Cell::new(js_sys::Date::now()));
        let handle = set_interval_with_handle(
            move || {
                #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
                {
                    let now = js_sys::Date::now();
                    if now - last_random_drop_ms.get() < 900.0 {
                        return;
                    }
                    last_random_drop_ms.set(now);
                }

                let f_x: f64 = rand::random();
                let f_y: f64 = rand::random();
                set_events.update(move |c| {
                    c.add_event(Event::AddDrop {
                        coord: Coord {
                            x: usize::from_f64(f_x * f64::from_usize(dots_width).unwrap())
                                .unwrap(),
                            y: usize::from_f64(f_y * f64::from_usize(dots_height).unwrap())
                                .unwrap(),
                        },
                    })
                });
            },
            Duration::from_millis(1000),
        );

        #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
        {
            let canvas_el = canvas_ref.get_untracked().expect("Canvas not loaded");
            let canvas_html: web_sys::HtmlCanvasElement = canvas_el.into();

            wasm_bindgen_futures::spawn_local(async move {
                let renderer = WgpuLiquidRenderer::new(
                    canvas_html,
                    dots_width as u32,
                    dots_height as u32,
                    events,
                    clear_events,
                    poline,
                )
                .await;

                let Some(renderer) = renderer else {
                    log::error!("Failed to initialize WebGPU renderer");
                    return;
                };

                fn animation_loop<T: Fn() + 'static>(
                    mut renderer: WgpuLiquidRenderer<T>,
                    on_cancel: impl Fn() + 'static,
                ) {
                    request_animation_frame(move || {
                        match renderer.draw() {
                            Ok(()) => animation_loop(renderer, on_cancel),
                            Err(()) => {
                                on_cancel();
                            }
                        }
                    });
                }

                animation_loop(renderer, on_cancel);
            });
        }

        handle
    });

    view! {
        <div
            node_ref=outer_size
            class="relative h-lvh w-lvw"
            on:pointermove=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: ev.page_x() as usize,
                        y: ev.page_y() as usize,
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
            on:click=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: ev.page_x() as usize,
                        y: ev.page_y() as usize,
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
        >
            <canvas
                node_ref=canvas_ref
                width=move || width.get()
                height=move || height.get()
                class="absolute inset-0"
            />
            {children()}
        </div>
    }
}

#[island]
pub fn DebugPoline() -> impl IntoView {
    let SliderHue { poline, .. } = expect_slider_hue();

    view! {
        <div class="pointer-events-none absolute left-0 top-0 h-lvh flex flex-wrap flex-col">
            {move || {
                poline
                    .with(|p| {
                        p.colors()
                            .iter()
                            .map(|[r, g, b]| {
                                let style = format!("background-color: rgb({r}, {g}, {b});");
                                view! { <div style=style class="w-2 h-2 md:w-4 md:h-4"></div> }
                            })
                            .collect_view()
                    })
            }}

        </div>
    }
}
