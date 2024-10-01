use grid::{Coord, Dimension};
use leptos::{html, prelude::*};
use num_traits::FromPrimitive;
use std::time::Duration;
use wasm_bindgen::prelude::*;

use crate::{
    canvas_grid::{
        CanvasEventManager, CanvasParams, Draw, Event, EventState, LiquidGridImageCanvas,
        PolineManager, PolineManagerImpl,
    },
    hooks::{use_elem_size, UseWindowSizeReturn},
};

#[island]
pub fn Slider() -> impl IntoView {
    let slider_update: SliderHue = expect_context();

    view! {
        <input
            type="range"
            min=0
            max=360
            class="accent-stone-700"
            value=move || slider_update.hue_value.get()
            on:input=move |ev| {
                let v: f64 = event_target_value(&ev).parse().unwrap();
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
    let (hue_value, set_hue_value) = signal(0.0);

    let colours = Memo::new_owning(move |last| {
        let value = hue_value.get();
        let Some(mut last) = last else {
            return (PolineManagerImpl::default(), true);
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
    let canvas_ref_hidden: NodeRef<html::Canvas> = NodeRef::new();

    let UseWindowSizeReturn { width, height } = use_elem_size(outer_size);

    let (events, set_events) = signal(EventState::default());

    let clear_events = move || set_events.update(|ev| ev.clear_events());

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

    let reduce_scale = Signal::derive(move || match width.get() > 1000.0 {
        true => 2,
        false => 1,
    });

    let dots_width_sig =
        Signal::derive(move || usize::from_f64(width.get()).unwrap() / reduce_scale.get());

    let dots_height_sig =
        Signal::derive(move || usize::from_f64(height.get()).unwrap() / reduce_scale.get());

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

        let dots_width = dots_width_sig.get_untracked();
        let dots_height = dots_height_sig.get_untracked();

        let handle = set_interval_with_handle(
            move || {
                let f_x: f64 = rand::random();
                let f_y: f64 = rand::random();
                set_events.update(move |c| {
                    c.add_event(Event::AddDrop {
                        coord: Coord {
                            x: usize::from_f64(f_x * f64::from_usize(dots_width).unwrap()).unwrap(),
                            y: usize::from_f64(f_y * f64::from_usize(dots_height).unwrap())
                                .unwrap(),
                        },
                    })
                });
            },
            Duration::from_millis(2000),
        );

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
                    visible_canvas: canvas_ref,
                    hidden_canvas: canvas_ref_hidden,
                    events,
                    clear_events,
                    poline,
                }),
                on_cancel,
            )
        });

        handle
    });

    view! {
        <div
            node_ref=outer_size
            class="relative h-dvh w-dvw"
            on:pointermove=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: ev.page_x() as usize / reduce_scale.get_untracked(),
                        y: ev.page_y() as usize / reduce_scale.get_untracked(),
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
            on:click=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: ev.page_x() as usize / reduce_scale.get_untracked(),
                        y: ev.page_y() as usize / reduce_scale.get_untracked(),
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
            <canvas
                node_ref=canvas_ref_hidden
                width=move || dots_width_sig.get()
                height=move || dots_height_sig.get()
                class="hidden"
            />
        </div>
    }
}

#[island]
pub fn DebugPoline() -> impl IntoView {
    let SliderHue { poline, .. } = expect_slider_hue();

    view! {
        <div class="pointer-events-none absolute left-0 top-0 h-dvh flex flex-wrap flex-col">
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
