use crate::error_template::{AppError, ErrorTemplate};
use gloo::{console::log, utils::format::JsValueSerdeExt};
use grid::{Coord, Size};
use leptos::{request_animation_frame, *};
use leptos_meta::*;
use leptos_router::*;
use liquid::{LiquidGrid, LiquidGridIter};
use num_traits::cast::FromPrimitive;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid};
use poline_rs::Poline;
use std::{f64::consts, sync::Arc};
use streaming_iterator::StreamingIterator;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

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

struct LiquidGridCanvas {
    colors: Vec<String>,
    grid: LiquidGridIter,
    pxPerCell: usize,
    ctx: CanvasRenderingContext2d,
}

impl LiquidGridCanvas {
    fn new(grid: LiquidGridIter, ctx: CanvasRenderingContext2d) -> Self {
        let p = Poline::builder().num_points(256).build().unwrap();
        let colors = p.colors_css();
        Self {
            colors,
            grid,
            pxPerCell: 3,
            ctx,
        }
    }

    fn value_to_color(&self, val: i8) -> &str {
        let idx = (val as u8).wrapping_add(128);
        self.colors.get(idx as usize).unwrap()
    }
}

trait Draw {
    fn draw(&mut self) -> Result<(), ()>;
}

impl Draw for LiquidGridCanvas {
    fn draw(&mut self) -> Result<(), ()> {
        self.grid.advance();
        for coord in self.grid.grid().coords_iter() {
            let value = self.grid.grid().get(coord).unwrap();
            let color = self.value_to_color(*value);

            self.ctx.set_fill_style(&JsValue::from_str(color));
            let x = f64::from_usize((coord.x + 1) * self.pxPerCell).ok_or(())?;
            let y = f64::from_usize((coord.y + 1) * self.pxPerCell).ok_or(())?;
            self.ctx.begin_path();
            self.ctx
                .arc(
                    x,
                    y,
                    f64::from_usize(self.pxPerCell).ok_or(())?,
                    0.0,
                    2.0 * consts::PI,
                )
                .unwrap();
            self.ctx.fill();
        }
        Ok(())
    }
}

#[island]
fn Canvas() -> impl IntoView {
    let canvas_ref: NodeRef<html::Canvas> = create_node_ref();

    create_effect(move |_| {
        let c = canvas_ref.get().expect("Canvas not loaded");

        let context = c
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.scale(2.0, 2.0).unwrap();

        let mut grid = LiquidGrid::new(100, 100).streaming_iter();
        // for i in 0..100 {
        // grid.add_drop(Coord { x: i, y: i });
        // }
        grid.add_drop(Coord { x: 50, y: 50 });

        let canvas = LiquidGridCanvas::new(grid, context);

        request_animation_frame(move || {
            fn helper(mut g: LiquidGridCanvas) {
                g.draw().unwrap();
                request_animation_frame(move || helper(g));
            }
            helper(canvas)
        })
    });

    view! {
        <div class="p-4">
            <canvas node_ref=canvas_ref width=1000 height=1000 />
        </div>
    }
}
