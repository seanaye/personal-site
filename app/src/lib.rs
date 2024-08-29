use crate::error_template::{AppError, ErrorTemplate};
use colorsys::Rgb;
use ev::resize;
use gloo::{
    console::log,
    utils::{format::JsValueSerdeExt, window},
};
use grid::{Coord, Size};
use html::Canvas;
use leptos::*;
use leptos_dom::helpers::AnimationFrameRequestHandle;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{
    use_debounce_fn, use_document, use_element_size, use_event_listener, use_window,
    use_window_scroll,
};
use liquid::{LiquidGrid, LiquidGridIter};
use num_traits::cast::FromPrimitive;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid};
use poline_rs::{Hsl, Poline};
use std::{borrow::Borrow, f64::consts, sync::Arc};
use streaming_iterator::StreamingIterator;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

pub mod error_template;
mod style;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css" />

        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors /> }.into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <Canvas />
        <PhotoGridComponent />
    }
}

#[island]
fn Counter() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);

    view! {
        <button on:click=move |_| {
            set_count.update(|count| *count += 1)
        }>"Click me: " {move || count()}</button>
    }
}

#[component]
fn PhotoGridComponent() -> impl IntoView {
    use style::*;

    let data = use_context::<Arc<ResponsivePhotoGrid<PhotoLayoutData>>>();

    let Some(g) = data else {
        dbg!("else");
        return view! {}.into_view();
    };

    let _ = "hidden col-span-1 row-span-1 col-span-2 row-span-2 col-span-3 row-span-3 col-span-4 row-span-4 col-start-1 row-start-1 col-start-2 row-start-2 col-start-3 row-start-3 col-start-4 row-start-4 col-start-5 row-start-5 col-start-6 row-start-6 col-start-7 row-start-7 col-start-8 row-start-8";

    g.grids()
        .map(|grid| {
            view! {
                <div class=grid
                    .style(
                        GridOuterClass,
                    )>
                    {grid
                        .grid
                        .into_iter()
                        .map(move |c| {
                            view! {
                                <div
                                    class=format!(
                                        "p-4 flex items-center justify-center {}",
                                        c.style(GridElemClass),
                                    )
                                    style=c.style(GridElemStyle)
                                >
                                    <img
                                        class="object-contain max-h-full max-w-full"
                                        src=c.content().srcs[0].url.to_string()
                                    />
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
        })
        .collect_view()
}

struct LiquidGridCanvas<T> {
    colors: Vec<String>,
    grid: LiquidGridIter,
    px_per_cell: usize,
    ctx: CanvasRenderingContext2d,
    events: ReadSignal<Vec<Events>>,
    clear_events: T,
}

struct LiquidGridImageCanvas<T> {
    colors: Vec<[u8; 3]>,
    grid: LiquidGridIter,
    ctx: CanvasRenderingContext2d,
    hidden_ctx: CanvasRenderingContext2d,
    events: ReadSignal<Vec<Events>>,
    clear_events: T,
    image_buffer: Vec<u8>,
    f64_scaled_width: f64,
    f64_scaled_height: f64,
}

/// events to be sent to the liquid grid canvas
enum Events {
    /// dispose of this canvas
    Cancel,
    /// add a new drop of liquid at the coord
    AddDrop { coord: Coord<usize> },
}

impl<T> LiquidGridCanvas<T>
where
    T: Fn(),
{
    fn new(
        grid: LiquidGridIter,
        ctx: CanvasRenderingContext2d,
        radius: usize,
        events: ReadSignal<Vec<Events>>,
        clear_events: T,
    ) -> Self {
        let p = Poline::builder().num_points(256).build().unwrap();
        let colors = p.colors_css();
        (clear_events)();
        Self {
            colors,
            grid,
            px_per_cell: radius,
            ctx,
            events,
            clear_events,
        }
    }

    fn value_to_color(&self, val: f64) -> &str {
        let idx = val + 128.0;
        self.colors.get(idx as usize).unwrap()
    }

    fn middle_color(&self) -> &str {
        self.colors.get(128).unwrap()
    }

    fn fill_rect(&mut self) {
        self.ctx.begin_path();
        self.ctx
            .set_fill_style(&JsValue::from_str(self.middle_color()));
        let width = self.grid.grid().width() + 1;
        let height = self.grid.grid().height() + 1;
        self.ctx.fill_rect(
            0.0,
            0.0,
            f64::from_usize(width * self.px_per_cell * 2).unwrap(),
            f64::from_usize(height * self.px_per_cell * 2).unwrap(),
        );
    }
}

trait CanvasManager {
    fn grid_events(&mut self) -> (&ReadSignal<Vec<Events>>, &mut LiquidGridIter);
    fn clear_events(&mut self);

    /// calculate the events
    fn compute_events(&mut self) -> Result<(), ()> {
        let (events, grid) = self.grid_events();
        let res = events.with_untracked(|val| {
            for i in val {
                match i {
                    Events::Cancel => {
                        return Err(());
                    }
                    Events::AddDrop { coord } => grid.add_drop(*coord),
                }
            }
            Ok(())
        });
        self.clear_events();
        res
    }
}

impl<T> CanvasManager for LiquidGridCanvas<T>
where
    T: Fn() + 'static,
{
    fn grid_events(&mut self) -> (&ReadSignal<Vec<Events>>, &mut LiquidGridIter) {
        (&self.events, &mut self.grid)
    }

    fn clear_events(&mut self) {
        (self.clear_events)()
    }
}

impl<T> LiquidGridImageCanvas<T>
where
    T: Fn() + 'static,
{
    pub fn new(
        grid: LiquidGridIter,
        ctx: CanvasRenderingContext2d,
        hidden_ctx: CanvasRenderingContext2d,
        scale: usize,
        events: ReadSignal<Vec<Events>>,
        clear_events: T,
    ) -> Self {
        log::info!("new");
        let p = Poline::builder().num_points(256).build().unwrap();
        let colors = p
            .colors()
            .into_iter()
            .map(|Hsl { h, s, l }| {
                let hsl = colorsys::Hsl::from((h, s * 100.0, l * 100.0));
                let rgb = colorsys::Rgb::from(hsl);
                let arr: [u8; 3] = rgb.into();
                arr
            })
            .collect();

        let width = grid.grid().width();
        let height = grid.grid().height();

        let scaled_width = width * scale;
        let scaled_height = height * scale;

        // RGBA for each pixel
        let image_buffer = vec![u8::MAX; width * height * 4];

        (clear_events)();

        Self {
            colors,
            hidden_ctx,
            grid,
            ctx,
            events,
            clear_events,
            image_buffer,
            f64_scaled_width: f64::from_usize(scaled_width).unwrap(),
            f64_scaled_height: f64::from_usize(scaled_height).unwrap(),
        }
    }

    pub fn fill_buffer(&mut self) {
        for (idx, value) in self.grid.grid().as_slice().iter().enumerate() {
            let color_idx = value + 128.0;
            let color = unsafe {
                self.colors
                    .get_unchecked(usize::from_f64(color_idx.clamp(0.0, 256.0)).unwrap())
            };
            // RGBA channel
            let start = idx * 4;
            // only write to RGB
            let end = start + 3;
            unsafe {
                let r = self.image_buffer.get_unchecked_mut(start..end);

                r.clone_from_slice(color.as_slice());
            }
        }
    }
}

impl<T> CanvasManager for LiquidGridImageCanvas<T>
where
    T: Fn() + 'static,
{
    fn grid_events(&mut self) -> (&ReadSignal<Vec<Events>>, &mut LiquidGridIter) {
        (&self.events, &mut self.grid)
    }

    fn clear_events(&mut self) {
        (self.clear_events)()
    }
}

trait Draw {
    fn draw(&mut self) -> Result<(), ()>;
}

// impl<T> Draw for LiquidGridCanvas<T>
// where
//     T: Fn() + 'static,
// {
//     fn draw(&mut self) -> Result<(), ()> {
//         self.grid.advance();
//         for coord in self.grid.grid().coords_iter() {
//             let value = self.grid.grid().get(coord).unwrap();
//             let color = self.value_to_color(*value);

//             self.ctx.set_fill_style(&JsValue::from_str(color));
//             let x = f64::from_usize((coord.x + 1) * self.px_per_cell * 2).ok_or(())?;
//             let y = f64::from_usize((coord.y + 1) * self.px_per_cell * 2).ok_or(())?;
//             self.ctx.begin_path();
//             self.ctx
//                 .arc(
//                     x,
//                     y,
//                     f64::from_usize(self.px_per_cell).ok_or(())?,
//                     0.0,
//                     2.0 * consts::PI,
//                 )
//                 .unwrap();
//             self.ctx.fill();
//         }
//         Ok(())
//     }
// }

impl<T> Draw for LiquidGridImageCanvas<T>
where
    T: Fn() + 'static,
{
    fn draw(&mut self) -> Result<(), ()> {
        self.grid.advance();
        self.fill_buffer();

        let data = ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(&self.image_buffer),
            self.grid.grid().width() as u32,
            self.grid.grid().height() as u32,
        )
        .map_err(|e| {
            log!(e);
        })?;
        self.hidden_ctx
            .put_image_data(&data, 0.0, 0.0)
            .map_err(|e| {
                log!(e);
            })?;
        self.ctx
            .draw_image_with_html_canvas_element_and_dw_and_dh(
                &self.hidden_ctx.canvas().ok_or(())?,
                0.0,
                0.0,
                self.f64_scaled_width,
                self.f64_scaled_height,
            )
            .map_err(|e| {
                log!(e);
            })?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct UseWindowSizeReturn {
    width: ReadSignal<f64>,
    height: ReadSignal<f64>,
}
fn use_window_size() -> UseWindowSizeReturn {
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

#[island]
fn Canvas() -> impl IntoView {
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
    let add_drop = move |_| {
        set_events.update(|vec| {
            vec.push(Events::AddDrop {
                coord: Coord { x: 50, y: 50 },
            })
        })
    };

    let setup_canvas = move |ref_node: NodeRef<Canvas>| -> CanvasRenderingContext2d {
        let c = ref_node.get_untracked().expect("Canvas not loaded");
        let context = c
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.scale(px_ratio, px_ratio).unwrap();
        context.set_image_smoothing_enabled(false);

        context
    };

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
                log::info!("start");
                fn helper<T>(mut g: T)
                where
                    T: Draw + CanvasManager + 'static,
                {
                    let Ok(_) = g.compute_events() else {
                        log::info!("cancelled");
                        return;
                    };
                    let Ok(_) = g.draw() else {
                        return;
                    };
                    request_animation_frame(move || helper(g));
                }

                let grid = LiquidGrid::new(dots_width, dots_height).streaming_iter();

                let canvas = LiquidGridImageCanvas::new(
                    grid,
                    setup_canvas(canvas_ref),
                    setup_canvas(canvas_ref_hidden),
                    32,
                    events,
                    clear_events,
                );
                helper(canvas)
            })
        },
    );

    view! {
        <div class="">
            <canvas
                node_ref=canvas_ref
                width=move || size.width.get()
                height=move || size.height.get()
                class="w-screen h-screen"
                on:click=add_drop
                on:mousemove=move |ev| {
                    let e = Events::AddDrop {
                        coord: Coord {
                            x: (ev.x() / 8) as usize,
                            y: (ev.y() / 8) as usize,
                        },
                    };
                    set_events.update(move |v| v.push(e));
                }
            />

            <canvas
                node_ref=canvas_ref_hidden
                width=move || size.width.get()
                height=move || size.height.get()
                class="hidden"
            />
        </div>
    }
}
