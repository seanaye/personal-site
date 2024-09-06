use grid::{Coord, Dimension};
use leptos::{html, prelude::*};
use leptos_use::{core::Size, use_window, use_window_size_with_options, UseWindowSizeOptions};
use num_traits::FromPrimitive;
use poline_rs::Hsl;
use std::time::Duration;
use wasm_bindgen::prelude::*;

use crate::{
    canvas_grid::{
        CanvasEventManager, CanvasParams, Draw, Event, EventState, LiquidGridImageCanvas,
        PolineManager, PolineManagerImpl,
    },
    hooks::use_window_size,
};

#[island]
pub fn Slider() -> impl IntoView {
    let slider_update: SliderUpdate = expect_context();

    view! {
        <input
            type="range"
            min=0
            max=360
            class="accent-stone-700"
            value=move || slider_update.hue_value.get()
            on:input=move |ev| {
                let v: f64 = event_target_value(&ev).parse().unwrap();
                slider_update.on_update(v)
            }
        />
    }
}

#[derive(Clone, Copy)]
struct SliderUpdate {
    pub hue_value: ReadSignal<f64>,
    set_hue_value: WriteSignal<f64>,
    set_events: WriteSignal<EventState>,
}

impl SliderUpdate {
    fn on_update(&self, this: f64) {
        self.set_hue_value.update(move |last| {
            let diff = this - *last;
            self.set_events
                .update(move |canvas| canvas.add_event(Event::OffsetHue { hue: diff }));
            *last = this
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SliderHue {
    pub hue_value: ReadSignal<f64>,
    pub set_hue_value: WriteSignal<f64>,
}

pub fn use_provide_slider_hue() -> SliderHue {
    let (hue_value, set_hue_value) = signal(0.0);

    let slider_hue = SliderHue {
        hue_value,
        set_hue_value,
    };
    provide_context(slider_hue);

    slider_hue
}

pub fn expect_slider_hue() -> SliderHue {
    expect_context()
}

#[island]
pub fn Canvas(children: Children) -> impl IntoView {
    let canvas_ref: NodeRef<html::Canvas> = NodeRef::new();
    let canvas_ref_hidden: NodeRef<html::Canvas> = NodeRef::new();
    let size = use_window_size();
    let window = use_window();
    let px_ratio = window
        .as_ref()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or_default();

    let (events, set_events) = signal(EventState::default());

    let clear_events = move || set_events.update(|ev| ev.clear_events());

    let (cancel_count, set_cancel_count) = signal(0);
    let (hue_value, set_hue_value) = signal(0.0);

    provide_context(SliderUpdate {
        hue_value,
        set_hue_value,
        set_events,
    });

    Effect::new(move |val: Option<bool>| {
        size.width.read();
        size.height.read();
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
        if let Some(Ok(interval)) = &val {
            interval.clear();
        }

        set_events.set(EventState::default());

        let dw = size.width.get_untracked() / px_ratio / 4.0;
        let dots_width = usize::from_f64(dw).unwrap();
        let dh = size.height.get_untracked() / px_ratio / 4.0;
        let dots_height = usize::from_f64(dh).unwrap();

        let handle = set_interval_with_handle(
            move || {
                let f_x: f64 = rand::random();
                let f_y: f64 = rand::random();
                set_events.update(move |c| {
                    c.add_event(Event::AddDrop {
                        coord: Coord {
                            x: usize::from_f64(f_x * dw).unwrap(),
                            y: usize::from_f64(f_y * dh).unwrap(),
                        },
                    })
                });
            },
            Duration::from_millis(3500),
        );
        let hue_offset = hue_value.get_untracked();

        request_animation_frame(move || {
            fn helper<T, U>(mut g: T, on_cancel: U)
            where
                T: Draw + CanvasEventManager + 'static,
                U: Fn() + 'static,
            {
                let Ok(_) = g.compute_events() else {
                    on_cancel();
                    return;
                };
                let Ok(_) = g.draw() else {
                    log::info!("failed to draw");
                    return;
                };
                request_animation_frame(move || helper(g, on_cancel));
            }

            helper(
                LiquidGridImageCanvas::new(CanvasParams {
                    size: Dimension {
                        width: dots_width,
                        height: dots_height,
                    },
                    px_ratio,
                    scale_factor: 32,
                    visible_canvas: canvas_ref,
                    hidden_canvas: canvas_ref_hidden,
                    events,
                    clear_events,
                    hue_offset,
                }),
                on_cancel,
            )
        });

        handle
    });

    view! {
        <div
            class="relative w-screen h-screen"
            on:mousemove=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: (ev.page_x() / 8) as usize,
                        y: (ev.page_y() / 8) as usize,
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
            on:click=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: (ev.page_x() / 8) as usize,
                        y: (ev.page_y() / 8) as usize,
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
        >
            <canvas
                node_ref=canvas_ref
                width=move || size.width.get()
                height=move || size.height.get()
                class="absolute inset-0"
            />
            <div>{move || size.width.get()}</div>
            {children()}
            <canvas
                node_ref=canvas_ref_hidden
                width=move || size.width.get()
                height=move || size.height.get()
                class="hidden"
            />
        </div>
    }
}

#[island]
pub fn DebugPoline() -> impl IntoView {
    let SliderHue { hue_value, .. } = expect_slider_hue();
    let poline = Signal::derive(move || {
        let hue = hue_value.get();
        PolineManagerImpl::new(hue)
    });

    view! {
        <div class="pointer-events-none absolute left-0 top-0 h-screen flex flex-wrap flex-col">
            {move || {
                poline
                    .with(|p| {
                        p.colors()
                            .iter()
                            .map(|[r, g, b]| {
                                let style = format!("background-color: rgb({r}, {g}, {b});");
                                view! { <div style=style class="w-8 h-8"></div> }
                            })
                            .collect_view()
                    })
            }}

        </div>
    }
}
