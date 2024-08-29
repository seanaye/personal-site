use grid::{Coord, Grid, Neighbours, Size};
use streaming_iterator::StreamingIterator;

#[derive(Debug, Clone)]
pub struct LiquidGrid {
    grid: Grid<f64>,
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
            damping: 0.95,
        }
    }
}

pub struct LiquidGridIter {
    buf1: LiquidGrid,
    buf2: LiquidGrid,
    damping: f64,
}

impl StreamingIterator for LiquidGridIter {
    type Item = [f64];

    fn advance(&mut self) {
        std::mem::swap(&mut self.buf1, &mut self.buf2);
        self.buf1.grid.coords_iter().for_each(|coord| {
            let mut sum: f64 = 0.0;
            for i in self.buf1.grid.neighbours(coord, Neighbours::Plus) {
                sum += *i;
            }
            sum /= 2.0;

            unsafe {
                sum -= *self
                    .buf2
                    .grid
                    .with_index(coord, |idx, grid| grid.get_unchecked(idx));
                sum *= self.damping;
                let item = self
                    .buf2
                    .grid
                    .with_index_mut(coord, |idx, grid| grid.get_unchecked_mut(idx));
                *item = sum;
            }
        });
    }

    fn get(&self) -> Option<&Self::Item> {
        Some(self.buf1.grid.as_slice())
    }
}

impl LiquidGridIter {
    pub fn add_drop(&mut self, c: Coord<usize>) {
        let Some(v) = self
            .buf2
            .grid
            .with_index_mut(c, |idx, grid| grid.get_mut(idx))
        else {
            return;
        };
        *v = 128.0;
    }

    pub fn grid(&self) -> &Grid<f64> {
        &self.buf1.grid
    }
}
