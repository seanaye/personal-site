use std::time::Duration;

use grid::{Coord, Dimension};
use leptos::*;
use leptos_dom::helpers::{AnimationFrameRequestHandle, IntervalHandle};
use leptos_use::use_window;
use num_traits::FromPrimitive;
use poline_rs::Hsl;
use wasm_bindgen::prelude::*;

use crate::{
    canvas_grid::{
        default_poline, CanvasEventManager, CanvasParams, Draw, Event, EventState,
        LiquidGridImageCanvas,
    },
    hooks::use_window_size,
};

#[island]
pub fn Slider() -> impl IntoView {
    log::info!("before");
    // let slider_update: Option<SliderUpdate> = use_context();
    // log::info!("{:?}", &slider_update);

    // let Some(slider_update) = slider_update else {
    //     return view! {}.into_view();
    // };

    // todo: remove portal on leptos 0.7
    // https://github.com/leptos-rs/leptos/issues/2440
    view! { <input type="range" min=0 max=360 /> }
}

#[derive(Clone, Copy, Debug)]
struct SliderUpdate {
    pub value: ReadSignal<f64>,
    set_value: WriteSignal<f64>,
    set_events: WriteSignal<EventState>,
}

impl SliderUpdate {
    fn on_update(&self, this: f64) {
        self.set_value.update(move |last| {
            let diff = this - *last;
            self.set_events
                .update(move |canvas| canvas.add_event(Event::OffsetHue { hue: diff }));
            *last = this
        });
    }
}

#[island]
pub fn Canvas(children: Children) -> impl IntoView {
    let canvas_ref: NodeRef<html::Canvas> = create_node_ref();
    let canvas_ref_hidden: NodeRef<html::Canvas> = create_node_ref();
    let size = use_window_size();
    let window = use_window();
    let px_ratio = window
        .as_ref()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or_default();

    let (events, set_events) = create_signal(EventState::default());

    let clear_events = move || set_events.update(|ev| ev.clear_events());

    let (value, set_value) = create_signal(0.0);

    // provide_context(SliderUpdate {
    //     value,
    //     set_value,
    //     set_events,
    // });

    create_effect(
        move |val: Option<(
            Result<AnimationFrameRequestHandle, JsValue>,
            Result<IntervalHandle, JsValue>,
        )>| {
            if let Some((Ok(frame), _)) = &val {
                frame.cancel();
            }
            if let Some((_, Ok(interval))) = &val {
                interval.clear();
            }

            let dw = size.width.get() / px_ratio / 4.0;
            let dots_width = usize::from_f64(dw).unwrap();
            let dh = size.height.get() / px_ratio / 4.0;
            let dots_height = usize::from_f64(dh).unwrap();

            // cancel previous grids if they exist
            if val.is_some() {
                set_events.update(|e| e.cancel());
            }

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

            let handle_2 = request_animation_frame_with_handle(move || {
                fn helper<T>(mut g: T)
                where
                    T: Draw + CanvasEventManager + 'static,
                {
                    let Ok(_) = g.compute_events() else {
                        return;
                    };
                    let Ok(_) = g.draw() else {
                        return;
                    };
                    request_animation_frame(move || helper(g));
                }

                // helper(LiquidGridImageCanvas::new(CanvasParams {
                //     size: Dimension {
                //         width: dots_width,
                //         height: dots_height,
                //     },
                //     px_ratio,
                //     scale_factor: 32,
                //     visible_canvas: canvas_ref,
                //     hidden_canvas: canvas_ref_hidden,
                //     events,
                //     clear_events,
                // }))
            });

            (handle_2, handle)
        },
    );

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
        >
            <canvas
                node_ref=canvas_ref
                width=move || size.width.get()
                height=move || size.height.get()
                class="absolute inset-0"
            />
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

#[component]
pub fn DebugPoline() -> impl IntoView {
    let poline = default_poline();

    let inner = poline
        .colors()
        .map(|Hsl { h, s, l }| {
            let style = format!(
                "background-color: hsl({h}, {}%, {}%);",
                s * 100.0,
                l * 100.0
            );
            view! { <div style=style class="w-8 h-8"></div> }
        })
        .collect_view();

    view! { <div class="flex flex-wrap">{inner}</div> }
}
