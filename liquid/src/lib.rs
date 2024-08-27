use grid::{Coord, Grid, Size};
use streaming_iterator::StreamingIterator;

#[derive(Debug, Clone)]
pub struct LiquidGrid {
    grid: Grid<i8>,
}

impl LiquidGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let grid = Grid::new_with_height(width, height);
        Self { grid }
    }

    pub fn streaming_iter(self) -> LiquidGridIter {
        let current = self.clone();
        LiquidGridIter {
            buf1: self,
            buf2: current,
            damping_numerator: 9,
            damping_denominator: 10,
        }
    }
}

pub struct LiquidGridIter {
    buf1: LiquidGrid,
    buf2: LiquidGrid,
    damping_numerator: i8,
    damping_denominator: i8,
}

impl StreamingIterator for LiquidGridIter {
    type Item = [i8];

    fn advance(&mut self) {
        std::mem::swap(&mut self.buf1, &mut self.buf2);
        self.buf1.grid.coords_iter().for_each(|coord| {
            let mut sum: i8 = 0;
            for i in self.buf1.grid.neighbours(coord) {
                sum = sum.saturating_add(*i);
            }
            sum = sum.saturating_div(4);
            sum = sum.saturating_sub(*self.buf2.grid.get(coord).unwrap());

            // apply damping
            sum = sum.saturating_mul(self.damping_numerator);
            sum = sum.saturating_div(self.damping_denominator);

            let item = self.buf2.grid.get_mut(coord).unwrap();
            *item = sum;
        });
    }

    fn get(&self) -> Option<&Self::Item> {
        Some(self.buf1.grid.as_slice())
    }
}

impl LiquidGridIter {
    pub fn add_drop(&mut self, c: Coord) {
        let Some(v) = self.buf2.grid.get_mut(c) else {
            return;
        };
        *v = i8::MAX;
    }

    pub fn grid(&self) -> &Grid<i8> {
        &self.buf1.grid
    }
}
