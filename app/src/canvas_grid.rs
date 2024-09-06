use grid::{Coord, Size};
use leptos::html::Canvas;
use leptos::prelude::*;
use liquid::{LiquidGrid, LiquidGridBuilder};
use num_traits::FromPrimitive;
use poline_rs::{fns::PositionFn, Hsl, Poline};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, ImageData};

use crate::log_js_trait::LogJsError;

pub trait PolineManager {
    fn colors(&self) -> &[[u8; 3]];
    fn shift_hue(&mut self, hue: f64);
}

pub struct PolineManagerImpl {
    poline: Poline,
    colors: Vec<[u8; 3]>,
}

impl PolineManagerImpl {
    fn regen_colors(p: &Poline) -> Vec<[u8; 3]> {
        p.colors()
            .map(|Hsl { h, s, l }| {
                let hsl = colorsys::Hsl::from((h, s * 100.0, l * 100.0));
                let rgb = colorsys::Rgb::from(hsl);
                let arr: [u8; 3] = rgb.into();
                arr
            })
            .collect()
    }
}

impl PolineManager for PolineManagerImpl {
    fn colors(&self) -> &[[u8; 3]] {
        &self.colors
    }

    fn shift_hue(&mut self, hue: f64) {
        self.poline.shift_hue(Some(hue));
        self.colors = Self::regen_colors(&self.poline);
    }
}

impl Default for PolineManagerImpl {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl PolineManagerImpl {
    pub fn new(hue_offset: f64) -> Self {
        let poline = Poline::builder()
            .num_points(256)
            .anchor_points(vec![
                Hsl {
                    h: 263.0,
                    s: 0.8,
                    l: 0.2,
                },
                Hsl {
                    h: 154.0,
                    s: 0.4,
                    l: 0.9,
                },
            ])
            .set_x_fn(PositionFn::Sinusoidal.get_fn())
            .set_y_fn(PositionFn::Quadratic.get_fn())
            .set_z_fn(PositionFn::Sinusoidal.get_fn())
            .invert_lightness(true)
            .build()
            .unwrap();

        let colors = Self::regen_colors(&poline);
        let mut out = Self { poline, colors };
        if hue_offset != 0.0 {
            out.shift_hue(hue_offset);
        }
        out
    }
}

pub struct LiquidGridImageCanvas<T> {
    poline: PolineManagerImpl,
    grid: LiquidGrid,
    ctx: CanvasRenderingContext2d,
    hidden_ctx: CanvasRenderingContext2d,
    events: ReadSignal<EventState>,
    clear_events: T,
    image_buffer: Vec<u8>,
    f64_scaled_width: f64,
    f64_scaled_height: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    AddDrop { coord: Coord<usize> },
    OffsetHue { hue: f64 },
}

/// events to be sent to the liquid grid canvas
#[derive(Debug, Clone, Default)]
pub struct EventState {
    /// add a new drop of liquid at the coord
    events: Vec<Event>,
    /// dispose of this canvas
    cancel: bool,
}

impl EventState {
    pub fn add_event(&mut self, event: Event) {
        self.events.push(event)
    }

    pub fn cancel(&mut self) {
        self.cancel = true;
    }

    /// clears out the events but does not
    /// reset the cancel state
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn reset_cancel_state(&mut self) {
        self.cancel = bool::default()
    }
}

pub trait CanvasEventManager {
    fn grid_events(
        &mut self,
    ) -> (
        &ReadSignal<EventState>,
        &mut LiquidGrid,
        &mut impl PolineManager,
    );
    fn clear_events(&mut self);

    /// calculate the events
    fn compute_events(&mut self) -> Result<(), ()> {
        let (events, grid, poline) = self.grid_events();
        let res = events.with_untracked(|val| {
            if val.cancel {
                return Err(());
            }
            for ev in &val.events {
                match ev {
                    Event::AddDrop { coord } => grid.add_drop(*coord),
                    Event::OffsetHue { hue } => poline.shift_hue(*hue),
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
    pub events: ReadSignal<EventState>,
    /// callback to clear the events
    pub clear_events: T,
    /// the current hue_offset value
    pub hue_offset: f64,
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
            hue_offset,
        } = params;

        let grid = LiquidGridBuilder::new(size.width(), size.height()).build();

        let width = grid.grid().width();
        let height = grid.grid().height();

        let scaled_width = width * scale_factor;
        let scaled_height = height * scale_factor;

        // RGBA for each pixel
        let image_buffer = vec![u8::MAX; width * height * 4];

        (clear_events)();

        Self {
            poline: PolineManagerImpl::new(hue_offset),
            hidden_ctx: Self::setup_canvas(hidden_canvas, px_ratio),
            grid,
            ctx: Self::setup_canvas(visible_canvas, px_ratio),
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
                self.poline
                    .colors()
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
    fn grid_events(
        &mut self,
    ) -> (
        &ReadSignal<EventState>,
        &mut LiquidGrid,
        &mut impl PolineManager,
    ) {
        (&self.events, &mut self.grid, &mut self.poline)
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
        .log_and_consume()?;
        self.hidden_ctx
            .put_image_data(&data, 0.0, 0.0)
            .log_and_consume()?;
        self.ctx
            .draw_image_with_html_canvas_element_and_dw_and_dh(
                &self.hidden_ctx.canvas().ok_or(())?,
                0.0,
                0.0,
                self.f64_scaled_width,
                self.f64_scaled_height,
            )
            .log_and_consume()?;
        Ok(())
    }
}
