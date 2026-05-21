use grid::Coord;
use poline_rs::{fns::PositionFn, Hsl, Poline};

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
                    h: 235.0,
                    s: 0.95,
                    l: 0.08,
                },
                Hsl {
                    h: 188.0,
                    s: 0.9,
                    l: 0.58,
                },
                Hsl {
                    h: 48.0,
                    s: 0.95,
                    l: 0.86,
                },
                Hsl {
                    h: 315.0,
                    s: 0.85,
                    l: 0.16,
                },
            ])
            .set_x_fn(PositionFn::Sinusoidal.get_fn())
            .set_y_fn(PositionFn::Sinusoidal.get_fn())
            .set_z_fn(PositionFn::Quadratic.get_fn())
            .invert_lightness(false)
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

#[derive(Debug, Clone, Copy)]
pub enum Event {
    AddDrop { coord: Coord<usize> },
}

/// events to be sent to the liquid grid canvas
#[derive(Debug, Clone, Default)]
pub struct EventState {
    /// add a new drop of liquid at the coord
    pub events: Vec<Event>,
    /// dispose of this canvas
    pub cancel: bool,
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
