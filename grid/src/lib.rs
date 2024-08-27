use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering, collections::HashSet, fmt::Display, num::ParseIntError, ops::Range, str::FromStr,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Dimension {
    pub width: usize,
    pub height: usize,
}

impl Dimension {
    fn aspect_ratio(&self) -> AspectRatio {
        let gcd = num::integer::gcd(self.width, self.height);
        let a = self.width / gcd;
        let b = self.height / gcd;
        AspectRatio {
            width: a,
            height: b,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct AspectRatio {
    width: usize,
    height: usize,
}

impl Display for AspectRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.width, self.height)
    }
}

pub enum ParseAspectRatioError {
    ParseInt(ParseIntError),
    Separator,
}
impl FromStr for AspectRatio {
    type Err = ParseAspectRatioError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(':');
        let first = split.next().ok_or(ParseAspectRatioError::Separator)?;
        let second = split.next().ok_or(ParseAspectRatioError::Separator)?;
        if split.next().is_some() {
            return Err(ParseAspectRatioError::Separator);
        }

        let a: usize = first.parse().map_err(ParseAspectRatioError::ParseInt)?;
        let b: usize = second.parse().map_err(ParseAspectRatioError::ParseInt)?;
        Ok(Self {
            width: a,
            height: b,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coord {
    pub x: usize,
    pub y: usize,
}

impl Coord {
    /// given a bottom right coord, iterate over all coords within the boundary
    fn iter_area<'a>(&'a self, bottom_right: &'a Self) -> impl Iterator<Item = Coord> + 'a {
        (self.y..=bottom_right.y)
            .flat_map(|y| (self.x..=bottom_right.x).map(move |x| Coord { x, y }))
    }
}

#[derive(Debug, Clone)]
pub struct Grid<T> {
    width: usize,
    contents: Vec<T>,
}

impl<T> Size for Grid<T> {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.contents.len() / self.width
    }
}

// #[derive(Debug, Clone, Copy)]
// enum Operation {
//     Plus,
//     Minus,
// }
// impl Operation {
//     fn do(self, a: usize, b: usize) -> usize {
//         match self {
//             Operation::Plus => a + b,
//             Operation::Minus => a - b

//         }
//     }
// }

// fn neighbours(Coord { x: this_x, y: this_y }: Coord) -> impl Iterator<Coord> {
//     [
//         ((1, Operation::Minus), (1, Operation::Minus)),
//         ((0, Operation::Minus), (1, Operation::Minus)),
//         ((1, Operation::Plus), (1, Operation::Minus)),
//         ((1, Operation::Minus), (0, Operation::Minus)),
//         ((1, Operation::Plus), (0, Operation::Minus)),
//         ((1, Operation::Minus), (1, Operation::Plus)),
//         ((0, Operation::Plus), (1, Operation::Plus)),
//         ((1, Operation::Plus), (1, Operation::Plus)),
//     ].map(|(x, y)| {
//             Coord {}
//         })
// }

impl<T> Grid<T> {
    pub fn as_slice(&self) -> &[T] {
        &self.contents
    }

    /// convert to coordinates from usize
    pub fn to_dimension(&self, idx: usize) -> Coord {
        let y = idx / self.width;
        let x = idx % self.width;
        Coord { x, y }
    }

    fn to_index(&self, Coord { x, y }: Coord) -> usize {
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

    pub fn new(width: usize) -> Self {
        Self {
            width,
            contents: Vec::new(),
        }
    }

    fn coord_in_bounds(&self, Coord { x, y }: &Coord) -> bool {
        x < &self.width && y < &self.height()
    }

    /// iterator over all adjacent members
    fn neighbours_coords(&self, coord: Coord) -> impl Iterator<Item = Coord> + '_ {
        #[derive(Debug, Clone, Copy)]
        enum Op {
            Plus,
            Minus,
        }

        impl Op {
            fn do_op(self, a: usize, b: usize) -> Option<usize> {
                match (a, self) {
                    (0, Op::Minus) => None,
                    (_, Op::Minus) => Some(a.saturating_sub(b)),
                    (_, Op::Plus) => Some(a.saturating_add(b)),
                }
            }
        }

        fn i() -> impl Iterator<Item = (usize, Op)> {
            [(1usize, Op::Minus), (0usize, Op::Plus), (1usize, Op::Plus)].into_iter()
        }

        i().flat_map(|y| i().map(move |x| (x, y)))
            .filter_map(move |((offset_x, op_x), (offset_y, op_y))| {
                dbg!({
                    let x = op_x.do_op(coord.x, offset_x)?;
                    let y = op_y.do_op(coord.y, offset_y)?;

                    Some(Coord { x, y })
                })
            })
            .filter(|x| self.coord_in_bounds(x))
            .filter(move |x| x != &coord)
    }

    /// iterator over the index values adjacent to coord
    fn neighbours_idx(&self, c: Coord) -> impl Iterator<Item = usize> + '_ {
        self.neighbours_coords(c).map(|coord| self.to_index(coord))
    }

    /// iterator over the items adjacent to coord
    pub fn neighbours(&self, c: Coord) -> impl Iterator<Item = &T> {
        self.neighbours_idx(c)
            .filter_map(|idx| self.contents.get(idx))
    }

    pub fn get_mut(&mut self, c: Coord) -> Option<&mut T> {
        let idx = self.to_index(c);
        self.contents.get_mut(idx)
    }

    pub fn get(&self, c: Coord) -> Option<&T> {
        let idx = self.to_index(c);
        self.contents.get(idx)
    }
}
impl<T> Grid<T>
where
    T: Eq,
{
    fn touching(&self, top_left: &Coord) -> Coord {
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
        let end_coord = (y + 1) * self.width;

        let cur = self.contents.len();
        let iter = (0..end_coord.saturating_sub(cur)).map(|_| T::default());
        self.contents.extend(iter)
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

#[derive(Debug, Clone, Copy)]
pub struct GridContent<T> {
    data: T,
    pub size: Dimension,
    pub origin: Coord,
}

impl<T> GridContent<T> {
    pub fn map<U>(self, cb: impl FnOnce(T) -> U) -> GridContent<U> {
        let GridContent { data, size, origin } = self;
        let data = cb(data);
        GridContent { data, size, origin }
    }

    pub fn grid_area(&self) -> (&Dimension, &Coord) {
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
    fn coords_iter(&self) -> impl Iterator<Item = Coord> + 'static {
        let height = self.height();
        let width = self.width();
        (0..height).flat_map(move |y| (0..width).map(move |x| Coord { x, y }))
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
        if max % divisor > 0 {
            long_edge += 1;
        }

        Self {
            orientation,
            long_edge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cool_asserts::assert_matches;
    use std::ops::Not;

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

    // #[tokio::test]
    // async fn grid_width_three() {
    //     let data = read_layout_data("tests/layout.json").await.unwrap();

    //     let photos = ResponsivePhotoGrid::new(data, [3], |x| {
    //         RoundedAspectRatio::<2>::from_aspect_ratio(&x.aspect_ratio)
    //     });

    //     let grid = photos.grids().next().unwrap();
    //     let first = grid.grid.first().unwrap();
    //     assert_matches!(
    //         first,
    //         GridContent {
    //             size: Dimension {
    //                 width: 3,
    //                 height: 2
    //             },
    //             origin: Coord { x: 0, y: 0 },
    //             ..
    //         }
    //     );
    // }
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
        let out: Vec<_> = g.neighbours_coords(Coord { x: 0, y: 0 }).collect();
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
        let out: Vec<_> = g.neighbours_coords(Coord { x: 10, y: 10 }).collect();
        assert_matches!(
            out,
            [
                Coord { x: 9, y: 9 },
                Coord { x: 10, y: 9 },
                Coord { x: 11, y: 9 },
                Coord { x: 9, y: 10 },
                Coord { x: 11, y: 10 },
                Coord { x: 9, y: 11 },
                Coord { x: 10, y: 11 },
                Coord { x: 11, y: 11 }
            ]
        );
    }
}
