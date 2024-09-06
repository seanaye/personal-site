use grid::{Coord, Grid, Neighbours, Size};

#[derive(Debug, Clone)]
pub struct LiquidGridBuilder {
    grid: Grid<f64>,
}

impl LiquidGridBuilder {
    pub fn new(width: usize, height: usize) -> Self {
        let grid = Grid::new_with_height(width, height);
        Self { grid }
    }

    pub fn build(self) -> LiquidGrid {
        let current = self.clone();
        LiquidGrid {
            buf1: self,
            buf2: current,
            damping: 0.97,
        }
    }
}

pub struct LiquidGrid {
    buf1: LiquidGridBuilder,
    buf2: LiquidGridBuilder,
    damping: f64,
}

impl LiquidGrid {
    pub fn advance(&mut self) {
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
    pub fn add_drop(&mut self, c: Coord<usize>) {
        let Some(v) = self
            .buf2
            .grid
            .with_index_mut(c, |idx, grid| grid.get_mut(idx))
        else {
            return;
        };
        *v = 256.0;
    }

    pub fn grid(&self) -> &Grid<f64> {
        &self.buf1.grid
    }
}
