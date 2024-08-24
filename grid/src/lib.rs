use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::Display,
    num::ParseIntError,
    ops::{Not, Range},
    str::FromStr,
};
use url::Url;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SrcSet {
    pub dimensions: Dimension,
    pub url: Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PhotoLayoutData {
    pub aspect_ratio: AspectRatio,
    pub srcs: Vec<SrcSet>,
    pub metadata: HashMap<String, String>,
}

pub trait FromAspectRatio {
    fn from_aspect_ratio(ratio: &AspectRatio) -> Self;
}

#[derive(Clone, Copy, Debug)]
pub enum Orientation {
    Portrait,
    Landscape,
}

impl FromAspectRatio for Orientation {
    fn from_aspect_ratio(ratio: &AspectRatio) -> Self {
        match ratio.width.cmp(&ratio.height) {
            Ordering::Less | Ordering::Equal => Orientation::Portrait,
            Ordering::Greater => Orientation::Landscape,
        }
    }
}

#[derive(Debug)]
pub struct Grid<T> {
    width: usize,
    contents: Vec<T>,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Debug, Clone, Copy)]
pub struct GridContent<T> {
    data: T,
    size: Dimension,
    origin: Coord,
}

impl<T> GridContent<T> {
    fn map<U>(self, cb: impl FnOnce(T) -> U) -> GridContent<U> {
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

    fn height_range(&self) -> Range<usize> {
        self.origin.y..self.origin.y + self.size.height
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

    /// extend the contents up to the row
    /// specified at the index
    fn extend_to(&mut self, idx: usize) {
        let Coord { y, .. } = self.to_dimension(idx);
        let end_coord = (y + 1) * self.width;

        let cur = self.contents.len();
        let iter = [Option::<T>::None]
            .into_iter()
            .cycle()
            .take(end_coord.saturating_sub(cur));
        self.contents.extend(iter)
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

impl<T> Grid<T> {
    /// convert to coordinates from usize
    fn to_dimension(&self, idx: usize) -> Coord {
        let y = idx / self.width;
        let x = idx % self.width;
        Coord { x, y }
    }

    fn to_index(&self, Coord { x, y }: Coord) -> usize {
        y * self.width + x
    }

    /// iterate over all coords in the dimension
    fn dimensions_iter(
        &self,
        dimension: &impl Size,
        offset: usize,
    ) -> impl Iterator<Item = usize> + '_ {
        let height = dimension.height();
        let width = dimension.width();
        (0..height)
            .flat_map(move |y| (0..width).map(move |x| Coord { x, y }))
            .map(move |coord| self.to_index(coord) + offset)
    }

    pub fn new(width: usize) -> Self {
        Self {
            width,
            contents: Vec::new(),
        }
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

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PhotoGrid<T> {
    pub grid: Vec<GridContent<T>>,
    pub width: usize,
}

impl<T> PhotoGrid<T>
where
    T: std::fmt::Debug,
{
    pub fn new_with_mapper<U, C>(photos: &[T], width: usize, cb: C) -> Self
    where
        T: Copy,
        C: for<'a> FnMut(&'a T) -> U,
        U: Size,
    {
        let mut grid = Grid::new(width);
        grid.add_all(photos.iter().map(cb));
        let mut nullable_vec: Vec<Option<T>> = photos.iter().map(|x| Some(*x)).collect();
        let content = grid
            .into_iter()
            .map(|c| {
                c.map(|idx| {
                    let item = nullable_vec.get_mut(idx).unwrap();
                    std::mem::take(item).unwrap()
                })
            })
            .collect::<Vec<_>>();
        PhotoGrid {
            grid: content,
            width,
        }
    }

    fn grow_non_intersecting(mut self) -> Self {
        let to_grow: Vec<_> = self
            .grid
            .iter()
            .enumerate()
            .filter(|(this_idx, i)| {
                let height_range = i.height_range();
                let out = self
                    .grid
                    .iter()
                    .enumerate()
                    .filter(|(_, j)| j.origin.x > i.origin.x)
                    .any(|(other_idx, j)| {
                        other_idx != *this_idx && j.height_range().does_intersect(&height_range)
                    })
                    .not();

                out
            })
            .map(|(idx, _)| idx)
            .collect();
        to_grow.into_iter().for_each(|idx| {
            let item = self.grid.get_mut(idx).unwrap();

            item.size.width = self.width - item.origin.x
        });
        self
    }
}

trait Intersect {
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

#[derive(Clone)]
pub struct ResponsivePhotoGrid<T> {
    grids: Vec<PhotoGrid<usize>>,
    data: Vec<T>,
}

impl<T> ResponsivePhotoGrid<T> {
    pub fn new<C, U>(photos: Vec<T>, sizes: impl IntoIterator<Item = usize>, mut cb: C) -> Self
    where
        C: for<'a> FnMut(&'a T) -> U,
        U: Size,
    {
        let ids: Vec<usize> = photos.iter().enumerate().map(|(idx, _)| idx).collect();
        let grids = sizes
            .into_iter()
            .map(|size| {
                PhotoGrid::new_with_mapper(ids.as_slice(), size, |id| cb(photos.get(*id).unwrap()))
            })
            .collect();
        Self {
            data: photos,
            grids,
        }
    }

    pub fn grids(&self) -> impl Iterator<Item = PhotoGrid<&T>> {
        self.grids.iter().map(|grid| {
            let width = grid.width;
            let grid = grid
                .grid
                .iter()
                .map(|content| content.map(|idx| self.data.get(idx).unwrap()))
                .collect();

            PhotoGrid { grid, width }
        })
    }

    /// on the smallest breakpoint we want to insert an outer container which always occupies the full width
    pub fn grow_to_width(mut self) -> Self {
        self.grids = self
            .grids
            .into_iter()
            .map(|g| g.grow_non_intersecting())
            .collect();
        self
    }
}

impl Default for ResponsivePhotoGrid<PhotoLayoutData> {
    fn default() -> Self {
        let s = r#"[{"aspect_ratio":{"width":3600,"height":2401},"srcs":[{"dimensions":{"width":3600,"height":2401},"url":"https://images.unsplash.com/photo-1719937206300-fc0dac6f8cac?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MXwxfGFsbHwxfHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":2095,"height":2521},"srcs":[{"dimensions":{"width":2095,"height":2521},"url":"https://images.unsplash.com/photo-1724198169550-ba2fde71cfc7?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHwyfHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":4000,"height":6000},"srcs":[{"dimensions":{"width":4000,"height":6000},"url":"https://images.unsplash.com/photo-1724384108758-dcc4f20518d7?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHwzfHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":8467,"height":11289},"srcs":[{"dimensions":{"width":8467,"height":11289},"url":"https://images.unsplash.com/photo-1724368202147-121dae0bd49d?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw0fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":11648,"height":8736},"srcs":[{"dimensions":{"width":11648,"height":8736},"url":"https://images.unsplash.com/photo-1724368202141-ef6f3522f50f?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw1fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":4000,"height":6000},"srcs":[{"dimensions":{"width":4000,"height":6000},"url":"https://images.unsplash.com/photo-1720048171527-208cb3e93192?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MXwxfGFsbHw2fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":2958,"height":3697},"srcs":[{"dimensions":{"width":2958,"height":3697},"url":"https://images.unsplash.com/photo-1724254351233-914fd32f2515?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw3fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":8736,"height":11648},"srcs":[{"dimensions":{"width":8736,"height":11648},"url":"https://images.unsplash.com/photo-1724368202143-3781f7b30d23?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw4fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":4160,"height":6240},"srcs":[{"dimensions":{"width":4160,"height":6240},"url":"https://images.unsplash.com/photo-1724348264169-6addad93be28?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw5fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":2832,"height":4240},"srcs":[{"dimensions":{"width":2832,"height":4240},"url":"https://images.unsplash.com/photo-1724340557729-e4bbb15c63c0?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHwxMHx8fHx8fDJ8fDE3MjQ0MjcyMDF8&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}}]"#;
        let d: Vec<PhotoLayoutData> = serde_json::from_str(s).unwrap();
        ResponsivePhotoGrid::new(d, [3, 4, 5, 8, 12], |x| {
            RoundedAspectRatio::<2>::from_aspect_ratio(&x.aspect_ratio)
        })
        .grow_to_width()
    }
}

pub trait Size {
    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn orientation(&self) -> Orientation {
        match self.width().cmp(&self.height()) {
            Ordering::Greater | Ordering::Equal => Orientation::Landscape,
            Ordering::Less => Orientation::Portrait,
        }
    }
}

impl Size for DynamicImage {
    fn width(&self) -> usize {
        self.width() as usize
    }

    fn height(&self) -> usize {
        self.height() as usize
    }
}

/// defines the normalized aspect ratio where the
/// short edge of an image is 1 and the long edge is an
/// integer multiple
#[derive(Clone, Copy, Debug)]
pub struct NormalizedAspectRatio {
    orientation: Orientation,
    long_edge: usize,
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

    #[test]
    fn it_should_iterate_properly() {
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
        ];

        let g = PhotoGrid::new_with_mapper(data.as_slice(), 4, |x| *x);

        assert_matches!(
            g.grid,
            [
                GridContent {
                    origin: Coord { x: 0, y: 0 },
                    size: Dimension {
                        height: 1,
                        width: 2
                    },
                    ..
                },
                GridContent {
                    origin: Coord { x: 2, y: 0 },
                    size: Dimension {
                        height: 1,
                        width: 2
                    },
                    ..
                },
                GridContent {
                    origin: Coord { x: 0, y: 1 },
                    size: Dimension {
                        height: 2,
                        width: 1
                    },
                    ..
                },
            ]
        )
    }

    #[test]
    fn another_test() {
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
                orientation: Orientation::Portrait,
                long_edge: 2,
            },
            NormalizedAspectRatio {
                orientation: Orientation::Landscape,
                long_edge: 2,
            },
        ];

        let g = PhotoGrid::new_with_mapper(data.as_slice(), 4, |x| *x);

        assert_matches!(
            g.grid,
            [
                GridContent {
                    origin: Coord { x: 0, y: 0 },
                    size: Dimension {
                        height: 2,
                        width: 1
                    },
                    ..
                },
                GridContent {
                    origin: Coord { x: 1, y: 0 },
                    size: Dimension {
                        height: 2,
                        width: 1
                    },
                    ..
                },
                GridContent {
                    origin: Coord { x: 2, y: 0 },
                    size: Dimension {
                        height: 2,
                        width: 1
                    },
                    ..
                },
                GridContent {
                    origin: Coord { x: 0, y: 2 },
                    size: Dimension {
                        height: 1,
                        width: 2
                    },
                    ..
                },
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
}
