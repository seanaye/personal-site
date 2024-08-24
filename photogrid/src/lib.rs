use std::{collections::HashMap, ops::Not};

use grid::{
    AspectRatio, Dimension, FromAspectRatio, Grid, GridContent, Intersect, RoundedAspectRatio, Size,
};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use url::Url;

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

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct PhotoGrid<T> {
    pub grid: Vec<GridContent<T>>,
    pub width: usize,
}

struct ImageWrapper(DynamicImage);

impl Size for ImageWrapper {
    fn width(&self) -> usize {
        self.0.width() as usize
    }

    fn height(&self) -> usize {
        self.0.height() as usize
    }
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
