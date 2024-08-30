use grid::{Coord, Size};
use leptos::{html::Canvas, NodeRef, ReadSignal, SignalWithUntracked};
use liquid::{LiquidGrid, LiquidGridIter};
use num_traits::FromPrimitive;
use poline_rs::{Hsl, Poline};
use streaming_iterator::StreamingIterator;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, ImageData};

pub struct LiquidGridImageCanvas<T> {
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
pub enum Events {
    /// dispose of this canvas
    Cancel,
    /// add a new drop of liquid at the coord
    AddDrop { coord: Coord<usize> },
}

pub trait CanvasEventManager {
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

pub struct CanvasParams<T, S> {
    /// the size of dots to make the grid
    pub size: S,
    /// the window device pixel ratio
    pub px_ratio: f64,
    /// how much scaled up the visible grid should be from the hidden one
    pub scale_factor: usize,
    /// the node ref of the visible canvas
    pub visible_canvas: NodeRef<Canvas>,
    /// the node ref of the invisible canvas
    pub hidden_canvas: NodeRef<Canvas>,
    /// a signal to read incoming events from
    pub events: ReadSignal<Vec<Events>>,
    /// callback to clear the events
    pub clear_events: T,
}

impl<T> LiquidGridImageCanvas<T>
where
    T: Fn() + 'static,
{
    fn setup_canvas(ref_node: NodeRef<Canvas>, px_ratio: f64) -> CanvasRenderingContext2d {
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
    }

    pub fn new<S>(params: CanvasParams<T, S>) -> Self
    where
        S: Size,
    {
        let CanvasParams {
            size,
            px_ratio,
            scale_factor,
            visible_canvas,
            hidden_canvas,
            events,
            clear_events,
        } = params;

        let grid = LiquidGrid::new(size.width(), size.height()).streaming_iter();

        LiquidGridImageCanvas::new_from_grid_iter(
            grid,
            Self::setup_canvas(visible_canvas, px_ratio),
            Self::setup_canvas(hidden_canvas, px_ratio),
            scale_factor,
            events,
            clear_events,
        )
    }

    fn new_from_grid_iter(
        grid: LiquidGridIter,
        ctx: CanvasRenderingContext2d,
        hidden_ctx: CanvasRenderingContext2d,
        scale: usize,
        events: ReadSignal<Vec<Events>>,
        clear_events: T,
    ) -> Self {
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

impl<T> CanvasEventManager for LiquidGridImageCanvas<T>
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

pub trait Draw {
    fn draw(&mut self) -> Result<(), ()>;
}

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
            gloo::console::log!(e);
        })?;
        self.hidden_ctx
            .put_image_data(&data, 0.0, 0.0)
            .map_err(|e| {
                gloo::console::log!(e);
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
                gloo::console::log!(e);
            })?;
        Ok(())
    }
}
