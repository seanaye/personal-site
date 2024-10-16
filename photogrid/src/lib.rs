use std::{collections::HashMap, ops::Not, sync::Arc};

use grid::{
    AspectRatio, ClampConfig, ClampWidthTo, Dimension, FromSize, Grid, GridContent, Intersect,
    RoundedAspectRatio, Size,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SrcSet {
    pub dimensions: Dimension,
    pub url: Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PhotoLayoutData {
    pub srcs: Vec<SrcSet>,
    pub metadata: HashMap<String, String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Clone)]
pub struct ResponsivePhotoGrid<T> {
    grids: Vec<PhotoGrid<usize>>,
    data: Vec<T>,
}

impl<T> ResponsivePhotoGrid<T> {
    pub fn new<C, U>(photos: Vec<T>, sizes: impl IntoIterator<Item = usize>, mut cb: C) -> Self
    where
        C: for<'a> FnMut(&'a T, (usize, usize)) -> U,
        U: Size,
    {
        let ids: Vec<usize> = photos.iter().enumerate().map(|(idx, _)| idx).collect();
        let grids = sizes
            .into_iter()
            .enumerate()
            .map(|(idx, size)| {
                PhotoGrid::new_with_mapper(ids.as_slice(), size, |id| {
                    cb(photos.get(*id).unwrap(), (idx, size))
                })
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

    pub fn contents_at(&self, n: usize) -> impl Iterator<Item = (&T, &GridContent<usize>)> {
        self.grids.iter().filter_map(move |grid| {
            grid.grid
                .get(n)
                .map(|idx| (self.data.get(*idx.content()).unwrap(), idx))
        })
    }

    /// returns the length of the first grid.
    /// this assumes all inner grids have the same number of items
    pub fn contents_len(&self) -> usize {
        self.grids.first().map(|g| g.grid.len()).unwrap_or_default()
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

impl ResponsivePhotoGrid<PhotoLayoutData> {
    pub fn from_layout_data(data: Vec<PhotoLayoutData>) -> Self {
        ResponsivePhotoGrid::new(data, [3, 4, 6, 8, 12], |x, (idx, size)| {
            let dimensions = x
                .srcs
                .iter()
                .map(|x| x.dimensions)
                .max_by_key(|dim| dim.width)
                .expect("There must be at least 1 srcset");
            let rounded = RoundedAspectRatio::<2>::from_size(&dimensions);
            let clamp = match (idx, size) {
                (0, x) => ClampConfig {
                    min_width: Some(x),
                    max_width: Some(rounded.width()),
                },
                (_, x) => ClampConfig {
                    min_width: None,
                    max_width: Some(x),
                },
            };

            rounded.clamp_width_to(clamp)
        })
    }
}

#[cfg(test)]
mod tests {
    use cool_asserts::assert_matches;
    use grid::{Coord, NormalizedAspectRatio, Orientation};

    use super::*;

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
}
