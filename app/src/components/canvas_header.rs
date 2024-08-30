use grid::{Coord, Dimension};
use leptos::*;
use leptos_dom::helpers::AnimationFrameRequestHandle;
use leptos_use::use_window;
use num_traits::FromPrimitive;
use wasm_bindgen::prelude::*;

use crate::{
    canvas_grid::{CanvasEventManager, CanvasParams, Draw, Events, LiquidGridImageCanvas},
    hooks::use_window_size,
};

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

    let (events, set_events) = create_signal::<Vec<Events>>(Vec::new());

    let clear_events = move || set_events.update(|vec| vec.clear());

    create_effect(
        move |val: Option<Result<AnimationFrameRequestHandle, JsValue>>| {
            if let Some(Ok(frame)) = val {
                frame.cancel();
            }

            let dots_width = usize::from_f64(size.width.get() / px_ratio / 4.0).unwrap();
            let dots_height = usize::from_f64(size.height.get() / px_ratio / 4.0).unwrap();

            // cancel previous grids if they exist
            if val.is_some() {
                set_events.update(|e| e.push(Events::Cancel));
            }

            request_animation_frame_with_handle(move || {
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

                helper(LiquidGridImageCanvas::new(CanvasParams {
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
                }))
            })
        },
    );

    view! {
        <div
            class="relative w-screen h-screen"
            on:mousemove=move |ev| {
                let e = Events::AddDrop {
                    coord: Coord {
                        x: (ev.offset_x() / 8) as usize,
                        y: (ev.offset_y() / 8) as usize,
                    },
                };
                set_events.update(move |v| v.push(e));
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
