use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::Display,
    ops::{Add, Range, RangeInclusive},
};
#[cfg(feature = "parse")]
pub mod parse;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Dimension {
    pub width: usize,
    pub height: usize,
}

impl Size for Dimension {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[non_exhaustive]
pub struct AspectRatio {
    pub width: usize,
    pub height: usize,
}

impl Display for AspectRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coord<T> {
    pub x: T,
    pub y: T,
}

impl<T> Coord<T>
where
    T: Copy,
    RangeInclusive<T>: Iterator<Item = T>,
{
    /// given a bottom right coord, iterate over all coords within the boundary
    fn iter_area<'a>(&'a self, bottom_right: &'a Self) -> impl Iterator<Item = Coord<T>> + 'a {
        (self.y..=bottom_right.y)
            .flat_map(|y| (self.x..=bottom_right.x).map(move |x| Coord { x, y }))
    }
}

#[derive(Debug, Clone)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    contents: Vec<T>,
}

impl<T> Size for Grid<T> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Plus,
    None,
    Minus,
}

impl Add<&Op> for usize {
    type Output = Option<usize>;

    fn add(self, rhs: &Op) -> Self::Output {
        match (self, rhs) {
            (x, Op::None) => Some(x),
            (0, Op::Minus) => None,
            (x, Op::Minus) => Some(x - 1),
            (x, Op::Plus) => Some(x + 1),
        }
    }
}

impl Add<&Coord<Op>> for Coord<usize> {
    type Output = Option<Coord<usize>>;

    fn add(self, rhs: &Coord<Op>) -> Self::Output {
        Some(Coord {
            x: (self.x + &rhs.x)?,
            y: (self.y + &rhs.y)?,
        })
    }
}

/// possible neighbours in a grid
pub enum Neighbours {
    /// left right up down
    Plus,
    /// diagonal neighbours
    X,
    /// both diagonal and left right up down
    Ring,
}

impl Neighbours {
    fn offsets(self) -> &'static [Coord<Op>] {
        match self {
            Neighbours::Plus => &NEIGHBOUR_OFFSETS[0..4],
            Neighbours::X => &NEIGHBOUR_OFFSETS[4..],
            Neighbours::Ring => &NEIGHBOUR_OFFSETS[..],
        }
    }
}

const NEIGHBOUR_OFFSETS: [Coord<Op>; 8] = [
    // plus offsets
    Coord {
        x: Op::None,
        y: Op::Minus,
    },
    Coord {
        x: Op::Minus,
        y: Op::None,
    },
    Coord {
        x: Op::Plus,
        y: Op::None,
    },
    Coord {
        x: Op::None,
        y: Op::Plus,
    },
    // x offsets
    Coord {
        x: Op::Minus,
        y: Op::Minus,
    },
    Coord {
        x: Op::Plus,
        y: Op::Minus,
    },
    Coord {
        x: Op::Minus,
        y: Op::Plus,
    },
    Coord {
        x: Op::Plus,
        y: Op::Plus,
    },
];

impl<T> Grid<T> {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            height: 0,
            contents: Vec::new(),
        }
    }

    pub fn as_slice(&self) -> &[T] {
        &self.contents
    }

    /// convert to coordinates from usize
    pub fn to_dimension(&self, idx: usize) -> Coord<usize> {
        let y = idx / self.width;
        let x = idx % self.width;
        Coord { x, y }
    }

    fn to_index(&self, Coord { x, y }: Coord<usize>) -> usize {
        y * self.width + x
    }

    /// iterate over all coords in the dimension
    fn dimensions_iter<'a>(
        &'a self,
        dimension: &'a impl Size,
        offset: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        dimension
            .coords_iter()
            .map(move |coord| self.to_index(coord) + offset)
    }

    fn coord_in_bounds(&self, Coord { x, y }: &Coord<usize>) -> bool {
        x < &self.width && y < &self.height()
    }

    /// iterator over all adjacent members
    fn neighbours_coords(
        &self,
        coord: Coord<usize>,
        neighbours: Neighbours,
    ) -> impl Iterator<Item = Coord<usize>> + '_ {
        neighbours
            .offsets()
            .iter()
            .filter_map(move |offset| coord + offset)
            .filter(|x| self.coord_in_bounds(x))
    }

    /// iterator over the index values adjacent to coord
    fn neighbours_idx(
        &self,
        c: Coord<usize>,
        neighbours: Neighbours,
    ) -> impl Iterator<Item = usize> + '_ {
        self.neighbours_coords(c, neighbours)
            .map(|coord| self.to_index(coord))
    }

    /// iterator over the items adjacent to coord
    pub fn neighbours(&self, c: Coord<usize>, neighbours: Neighbours) -> impl Iterator<Item = &T> {
        self.neighbours_idx(c, neighbours)
            .filter_map(|idx| self.contents.get(idx))
    }

    pub fn with_index<'a, Cb, U>(&'a self, c: Coord<usize>, cb: Cb) -> U
    where
        Cb: FnOnce(usize, &'a Vec<T>) -> U,
    {
        cb(self.to_index(c), &self.contents)
    }

    pub fn with_index_mut<'a, Cb, U>(&'a mut self, c: Coord<usize>, cb: Cb) -> U
    where
        Cb: FnOnce(usize, &'a mut Vec<T>) -> U,
    {
        cb(self.to_index(c), &mut self.contents)
    }
}
impl<T> Grid<T>
where
    T: Eq,
{
    fn touching(&self, top_left: &Coord<usize>) -> Coord<usize> {
        let val = self.contents.get(self.to_index(*top_left)).unwrap();
        let right_extent = (top_left.x..self.width)
            .map(|x| Coord { y: top_left.y, x })
            .take_while(|coord| {
                self.contents
                    .get(self.to_index(*coord))
                    .is_some_and(|v| v == val)
            })
            .last();
        let bottom_extent = (top_left.y..)
            .map(|y| Coord { y, x: top_left.x })
            .take_while(|coord| {
                self.contents
                    .get(self.to_index(*coord))
                    .is_some_and(|v| v == val)
            })
            .last();
        match (right_extent, bottom_extent) {
            (Some(Coord { x, .. }), Some(Coord { y, .. })) => Coord { x, y },
            _ => dbg!(*top_left),
        }
    }
}

impl<T> Grid<T>
where
    T: Default,
{
    /// extend the contents up to the row
    /// specified at the index
    fn extend_to(&mut self, idx: usize) {
        let Coord { y, .. } = self.to_dimension(idx);
        let out_height = y + 1;
        let end_coord = out_height * self.width;

        let cur = self.contents.len();
        let iter = (0..end_coord.saturating_sub(cur)).map(|_| T::default());
        self.contents.extend(iter);
        self.height = out_height;
    }

    pub fn new_with_height(width: usize, height: usize) -> Self {
        let mut out = Self::new(width);
        let idx = out.to_index(Coord {
            x: 0,
            y: height - 1,
        });
        out.extend_to(idx);

        out
    }
}

impl<T> Grid<Option<T>>
where
    T: Clone,
{
    /// iterate over the unoccupied spaces
    fn available(&self) -> impl Iterator<Item = usize> + '_ {
        self.contents
            .iter()
            .enumerate()
            .filter_map(|(idx, e)| match e.is_none() {
                true => Some(idx),
                false => None,
            })
    }

    fn does_fit_at(&self, idx: usize, dimension: &impl Size) -> bool {
        if self.to_dimension(idx).x + dimension.width() > self.width {
            return false;
        }

        self.dimensions_iter(dimension, idx)
            .all(|idx| match self.contents.get(idx) {
                None => true,
                Some(None) => true,
                Some(_) => false,
            })
    }

    fn insert_at(&mut self, index: usize, id: T, dimension: &impl Size) -> Result<(), ()> {
        let last = self.dimensions_iter(dimension, index).last().ok_or(())?;
        self.extend_to(last);
        self.dimensions_iter(dimension, index)
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|idx| {
                let Some(e) = self.contents.get_mut(idx) else {
                    return;
                };
                *e = Some(id.clone())
            });
        Ok(())
    }
}

impl Grid<Option<usize>> {
    pub fn add_all<T>(&mut self, data: impl IntoIterator<Item = T>)
    where
        T: Size,
    {
        data.into_iter().enumerate().for_each(|(idx, el)| {
            let fit = self.available().find(|e| self.does_fit_at(*e, &el));

            match fit {
                Some(a) => {
                    self.insert_at(a, idx, &el)
                        .expect("failed to insert at valid location");
                }
                None => {
                    let len = self.contents.len();
                    self.extend_to(len);
                    self.insert_at(len, idx, &el)
                        .expect("failed to insert after extending");
                }
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GridContent<T> {
    data: T,
    pub size: Dimension,
    pub origin: Coord<usize>,
}

impl<T> GridContent<T> {
    pub fn map<U>(self, cb: impl FnOnce(T) -> U) -> GridContent<U> {
        let GridContent { data, size, origin } = self;
        let data = cb(data);
        GridContent { data, size, origin }
    }

    pub fn grid_area(&self) -> (&Dimension, &Coord<usize>) {
        (&self.size, &self.origin)
    }

    pub fn content(&self) -> &T {
        &self.data
    }

    pub fn height_range(&self) -> Range<usize> {
        self.origin.y..self.origin.y + self.size.height
    }
}

pub struct GridVisitor<T> {
    seen: HashSet<usize>,
    grid: Grid<T>,
    cur: usize,
}

impl<T> Iterator for GridVisitor<Option<T>>
where
    T: Eq + Copy,
{
    type Item = GridContent<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.seen.contains(&self.cur) {
            self.cur += 1;
            return self.next();
        }
        let this = self.grid.contents.get(self.cur)?;
        let coord = self.grid.to_dimension(self.cur);

        let bottom_right = self.grid.touching(&coord);

        self.seen.extend(
            coord
                .iter_area(&bottom_right)
                .map(|coord| self.grid.to_index(coord)),
        );

        let dimension = Dimension {
            width: 1 + bottom_right.x - coord.x,
            height: 1 + bottom_right.y - coord.y,
        };

        self.cur += dimension.width;

        match this {
            Some(val) => Some(GridContent {
                data: *val,
                size: dimension,
                origin: coord,
            }),
            None => self.next(),
        }
    }
}

impl<T> IntoIterator for Grid<Option<T>>
where
    T: Eq + Copy,
{
    type Item = GridContent<T>;

    type IntoIter = GridVisitor<Option<T>>;

    fn into_iter(self) -> Self::IntoIter {
        GridVisitor {
            cur: 0,
            seen: HashSet::new(),
            grid: self,
        }
    }
}

pub trait Intersect {
    fn does_intersect(&self, other: &Self) -> bool;
}

impl<T> Intersect for Range<T>
where
    T: PartialOrd,
{
    fn does_intersect(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Orientation {
    Portrait,
    Landscape,
}

pub trait FromAspectRatio {
    fn from_aspect_ratio(ratio: &AspectRatio) -> Self;
}

impl FromAspectRatio for Orientation {
    fn from_aspect_ratio(ratio: &AspectRatio) -> Self {
        match ratio.width.cmp(&ratio.height) {
            Ordering::Less | Ordering::Equal => Orientation::Portrait,
            Ordering::Greater => Orientation::Landscape,
        }
    }
}

pub trait Size {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    /// returns the orientation of the size
    fn orientation(&self) -> Orientation {
        match self.width().cmp(&self.height()) {
            Ordering::Greater | Ordering::Equal => Orientation::Landscape,
            Ordering::Less => Orientation::Portrait,
        }
    }

    /// iterator over all the coords in the size
    fn coords_iter(&self) -> impl Iterator<Item = Coord<usize>> + 'static {
        let height = self.height();
        let width = self.width();
        (0..height).flat_map(move |y| (0..width).map(move |x| Coord { x, y }))
    }

    fn aspect_ratio(&self) -> AspectRatio {
        let width = self.width();
        let height = self.height();
        let gcd = num::integer::gcd(width, height);
        let a = width / gcd;
        let b = height / gcd;
        AspectRatio {
            width: a,
            height: b,
        }
    }
}

/// defines the normalized aspect ratio where the
/// short edge of an image is 1 and the long edge is an
/// integer multiple
#[derive(Clone, Copy, Debug)]
pub struct NormalizedAspectRatio {
    pub orientation: Orientation,
    pub long_edge: usize,
}

impl Size for NormalizedAspectRatio {
    fn width(&self) -> usize {
        match self.orientation {
            Orientation::Portrait => 1,
            Orientation::Landscape => self.long_edge,
        }
    }

    fn height(&self) -> usize {
        match self.orientation {
            Orientation::Portrait => self.long_edge,
            Orientation::Landscape => 1,
        }
    }
}

impl FromAspectRatio for NormalizedAspectRatio {
    fn from_aspect_ratio(ratio: &AspectRatio) -> Self {
        let orientation = Orientation::from_aspect_ratio(ratio);

        let (min, max) = match orientation {
            Orientation::Portrait => (ratio.width, ratio.height),
            Orientation::Landscape => (ratio.height, ratio.width),
        };

        let mut long_edge = max / min;
        if max % min > 0 {
            long_edge += 1;
        }

        Self {
            orientation,
            long_edge,
        }
    }
}

#[derive(Debug)]
pub struct RoundedAspectRatio<const SIZE: usize> {
    long_edge: usize,
    orientation: Orientation,
}

impl<const SIZE: usize> Size for RoundedAspectRatio<SIZE> {
    fn width(&self) -> usize {
        match self.orientation {
            Orientation::Portrait => SIZE,
            Orientation::Landscape => self.long_edge,
        }
    }

    fn height(&self) -> usize {
        match self.orientation {
            Orientation::Portrait => self.long_edge,
            Orientation::Landscape => SIZE,
        }
    }
}

impl<const SIZE: usize> FromAspectRatio for RoundedAspectRatio<SIZE> {
    fn from_aspect_ratio(ratio: &AspectRatio) -> Self {
        let orientation = Orientation::from_aspect_ratio(ratio);

        let (min, max) = match orientation {
            Orientation::Portrait => (ratio.width, ratio.height),
            Orientation::Landscape => (ratio.height, ratio.width),
        };

        let divisor = min / SIZE;

        let mut long_edge = max / divisor;
        if max % divisor > divisor / 2 {
            long_edge += 1;
        }

        Self {
            orientation,
            long_edge,
        }
    }
}

impl<const SIZE: usize> RoundedAspectRatio<SIZE> {
    pub fn clamp_width_to(self, max_width: usize) -> Dimension {
        let height = self.height();
        let width = self.width();
        if width <= max_width {
            // we do dont need to do anything, return
            return Dimension {
                width: self.width(),
                height,
            };
        }

        let shrink_factor = max_width as f64 / width as f64;
        let shrunk_height = height as f64 * shrink_factor;
        let mut new_height = shrunk_height as usize;
        if new_height < 1 {
            new_height = 1;
        }
        Dimension {
            width: max_width,
            height: new_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cool_asserts::assert_matches;
    use std::ops::Not;

    #[test]
    fn it_should_return_3_to_2_for_z6_dimensions() {
        let a = RoundedAspectRatio::<2>::from_aspect_ratio(&crate::AspectRatio {
            width: 6048,
            height: 4024,
        });
        assert_eq!(a.width(), 3);
        assert_eq!(a.height(), 2);
    }

    #[test]
    fn it_should_round_aspect() {
        let a = RoundedAspectRatio::<2>::from_aspect_ratio(&crate::AspectRatio {
            width: 856,
            height: 1280,
        });
        assert_eq!(a.width(), 2);
        assert_eq!(a.height(), 3);
    }

    #[test]
    fn it_should_layout_a() {
        let data = vec![NormalizedAspectRatio {
            orientation: Orientation::Landscape,
            long_edge: 2,
        }];

        let mut grid = Grid::new(4);
        grid.add_all(data);

        assert_matches!(grid.contents, [Some(0), Some(0), None, None])
    }

    #[test]
    fn it_should_layout_b() {
        let data = vec![
            NormalizedAspectRatio {
                orientation: Orientation::Portrait,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Portrait,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Landscape,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Landscape,
                long_edge: 2,
            },
        ];

        let mut grid = Grid::new(4);
        grid.add_all(data);

        assert_matches!(
            grid.contents,
            [
                Some(0),
                Some(1),
                Some(2),
                Some(2),
                Some(0),
                Some(1),
                Some(3),
                Some(3)
            ]
        )
    }

    #[test]
    fn it_should_layout_c() {
        let data = vec![
            NormalizedAspectRatio {
                orientation: Orientation::Landscape,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Landscape,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Portrait,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Portrait,
                long_edge: 2,
            },
        ];

        let mut grid = Grid::new(4);
        grid.add_all(data);

        assert_matches!(
            grid.contents,
            [
                Some(0),
                Some(0),
                Some(1),
                Some(1),
                Some(2),
                Some(3),
                None,
                None,
                Some(2),
                Some(3),
                None,
                None
            ]
        )
    }

    #[test]
    fn off_by_1_aspect() {
        let data = RoundedAspectRatio::<2>::from_aspect_ratio(&crate::AspectRatio {
            width: 3600,
            height: 2401,
        });

        assert_matches!(
            data,
            RoundedAspectRatio {
                long_edge: 3,
                orientation: Orientation::Landscape
            }
        );
    }

    #[test]
    fn it_should_not_intersect() {
        let a = 5..10;
        let b = 10..12;
        assert!(a.does_intersect(&b).not())
    }

    #[test]
    fn it_should_intersect() {
        let a = 5..10;
        let b = 9..12;
        assert!(a.does_intersect(&b))
    }

    #[test]
    fn height_works() {
        let height = 100;
        let width = 100;
        let g = Grid::<usize>::new_with_height(width, height);
        assert_eq!(g.height(), height);
        assert_eq!(g.contents.len(), width * height);
    }

    #[test]
    fn neighbours_iterates_properly() {
        let g = Grid::<usize>::new_with_height(100, 100);
        let out: Vec<_> = g
            .neighbours_coords(Coord { x: 0, y: 0 }, Neighbours::Ring)
            .collect();
        assert_matches!(
            out,
            [
                Coord { x: 1, y: 0 },
                Coord { x: 0, y: 1 },
                Coord { x: 1, y: 1 }
            ]
        );
    }

    #[test]
    fn neighbours_iterates_properly_2() {
        let g = Grid::<usize>::new_with_height(100, 100);
        let out: Vec<_> = g
            .neighbours_coords(Coord { x: 10, y: 10 }, Neighbours::Ring)
            .collect();
        assert_matches!(
            out,
            [
                Coord { x: 10, y: 9 },
                Coord { x: 9, y: 10 },
                Coord { x: 11, y: 10 },
                Coord { x: 10, y: 11 },
                Coord { x: 9, y: 9 },
                Coord { x: 11, y: 9 },
                Coord { x: 9, y: 11 },
                Coord { x: 11, y: 11 }
            ]
        );
    }
}
