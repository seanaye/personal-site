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
    /// shift the current hue by set degrees
    fn shift_hue(&mut self, hue: f64);
    /// set the aboslute hue shift in terms of offset
    /// from 0 degrees
    fn set_hue(&mut self, hue: f64);
}

pub struct PolineManagerImpl {
    poline: Poline,
    colors: Vec<[u8; 3]>,
    /// the absolute value of the hue shift
    abs_hue: f64,
}

impl PartialEq for PolineManagerImpl {
    fn eq(&self, other: &Self) -> bool {
        self.abs_hue == other.abs_hue
    }
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

    /// get the absolute value of the hue shift
    pub fn abs_hue(&self) -> &f64 {
        &self.abs_hue
    }
}

impl PolineManager for PolineManagerImpl {
    fn colors(&self) -> &[[u8; 3]] {
        &self.colors
    }

    fn shift_hue(&mut self, hue: f64) {
        self.poline.shift_hue(Some(hue));
        self.abs_hue += hue;
        self.colors = Self::regen_colors(&self.poline);
    }

    fn set_hue(&mut self, hue: f64) {
        let diff = hue - self.abs_hue;
        self.shift_hue(diff);
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
        let mut out = Self {
            poline,
            colors,
            abs_hue: hue_offset,
        };
        if hue_offset != 0.0 {
            out.shift_hue(hue_offset);
        }
        out
    }
}

pub struct LiquidGridImageCanvas<T> {
    poline: Memo<PolineManagerImpl>,
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
    fn grid_events(&mut self) -> (&ReadSignal<EventState>, &mut LiquidGrid);
    fn clear_events(&mut self);

    /// calculate the events
    fn compute_events(&mut self) -> Result<(), ()> {
        let (events, grid) = self.grid_events();
        let res = events.with_untracked(|val| {
            if val.cancel {
                return Err(());
            }
            for ev in &val.events {
                match ev {
                    Event::AddDrop { coord } => grid.add_drop(*coord),
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
    /// the node ref of the visible canvas
    pub visible_canvas: NodeRef<Canvas>,
    /// the node ref of the invisible canvas
    pub hidden_canvas: NodeRef<Canvas>,
    /// a signal to read incoming events from
    pub events: ReadSignal<EventState>,
    /// callback to clear the events
    pub clear_events: T,
    pub poline: Memo<PolineManagerImpl>,
}

impl<T> LiquidGridImageCanvas<T>
where
    T: Fn() + 'static,
{
    fn setup_canvas(ref_node: NodeRef<Canvas>) -> CanvasRenderingContext2d {
        let c = ref_node.get_untracked().expect("Canvas not loaded");
        let context = c
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.set_image_smoothing_enabled(false);

        context
    }

    pub fn new<S>(params: CanvasParams<T, S>) -> Self
    where
        S: Size,
    {
        let CanvasParams {
            size,
            visible_canvas,
            hidden_canvas,
            events,
            clear_events,
            poline,
        } = params;

        let grid = LiquidGridBuilder::new(size.width(), size.height()).build();

        let width = grid.grid().width();
        let height = grid.grid().height();

        // RGBA for each pixel
        let image_buffer = vec![u8::MAX; width * height * 4];

        (clear_events)();

        Self {
            poline,
            hidden_ctx: Self::setup_canvas(hidden_canvas),
            grid,
            ctx: Self::setup_canvas(visible_canvas),
            events,
            clear_events,
            image_buffer,
            f64_scaled_width: f64::from_usize(width).unwrap(),
            f64_scaled_height: f64::from_usize(height).unwrap(),
        }
    }

    pub fn fill_buffer(&mut self) {
        for (idx, value) in self.grid.grid().as_slice().iter().enumerate() {
            let color_idx = value + 128.0;
            let read_guard = self.poline.read_untracked();
            let color = unsafe {
                read_guard
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
    fn grid_events(&mut self) -> (&ReadSignal<EventState>, &mut LiquidGrid) {
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
        .log_and_consume()?;
        self.hidden_ctx
            .put_image_data(&data, 0.0, 0.0)
            .log_and_consume()?;
        self.ctx
            .draw_image_with_html_canvas_element_and_dw_and_dh(
                &self.hidden_ctx.canvas().ok_or(())?,
                0.0,
                0.0,
                self.ctx.canvas().unwrap().width() as f64,
                self.ctx.canvas().unwrap().height() as f64,
            )
            .log_and_consume()?;
        Ok(())
    }
}
