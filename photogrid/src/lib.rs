use std::{collections::HashMap, ops::Not};

use grid::{
    AspectRatio, Dimension, FromAspectRatio, Grid, GridContent, Intersect, RoundedAspectRatio, Size,
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
    pub aspect_ratio: AspectRatio,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct ResponsivePhotoGrid<T> {
    grids: Vec<PhotoGrid<usize>>,
    data: Vec<T>,
}

impl<T> ResponsivePhotoGrid<T> {
    pub fn new<C, U>(photos: Vec<T>, sizes: impl IntoIterator<Item = usize>, mut cb: C) -> Self
    where
        C: for<'a> FnMut(&'a T, usize) -> U,
        U: Size,
    {
        let ids: Vec<usize> = photos.iter().enumerate().map(|(idx, _)| idx).collect();
        let grids = sizes
            .into_iter()
            .map(|size| {
                PhotoGrid::new_with_mapper(ids.as_slice(), size, |id| {
                    cb(photos.get(*id).unwrap(), size)
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
    pub fn testing() -> Self {
        let s = r#"[{"aspect_ratio":{"width":3600,"height":2401},"srcs":[{"dimensions":{"width":3600,"height":2401},"url":"https://images.unsplash.com/photo-1719937206300-fc0dac6f8cac?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MXwxfGFsbHwxfHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":2095,"height":2521},"srcs":[{"dimensions":{"width":2095,"height":2521},"url":"https://images.unsplash.com/photo-1724198169550-ba2fde71cfc7?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHwyfHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":4000,"height":6000},"srcs":[{"dimensions":{"width":4000,"height":6000},"url":"https://images.unsplash.com/photo-1724384108758-dcc4f20518d7?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHwzfHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":8467,"height":11289},"srcs":[{"dimensions":{"width":8467,"height":11289},"url":"https://images.unsplash.com/photo-1724368202147-121dae0bd49d?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw0fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":11648,"height":8736},"srcs":[{"dimensions":{"width":11648,"height":8736},"url":"https://images.unsplash.com/photo-1724368202141-ef6f3522f50f?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw1fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":4000,"height":6000},"srcs":[{"dimensions":{"width":4000,"height":6000},"url":"https://images.unsplash.com/photo-1720048171527-208cb3e93192?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MXwxfGFsbHw2fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":2958,"height":3697},"srcs":[{"dimensions":{"width":2958,"height":3697},"url":"https://images.unsplash.com/photo-1724254351233-914fd32f2515?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw3fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":8736,"height":11648},"srcs":[{"dimensions":{"width":8736,"height":11648},"url":"https://images.unsplash.com/photo-1724368202143-3781f7b30d23?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw4fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":4160,"height":6240},"srcs":[{"dimensions":{"width":4160,"height":6240},"url":"https://images.unsplash.com/photo-1724348264169-6addad93be28?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHw5fHx8fHx8Mnx8MTcyNDQyNzIwMXw&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}},{"aspect_ratio":{"width":2832,"height":4240},"srcs":[{"dimensions":{"width":2832,"height":4240},"url":"https://images.unsplash.com/photo-1724340557729-e4bbb15c63c0?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w2NDAzNjV8MHwxfGFsbHwxMHx8fHx8fDJ8fDE3MjQ0MjcyMDF8&ixlib=rb-4.0.3&q=80&w=1080"}],"metadata":{}}]"#;
        let mut d: Vec<PhotoLayoutData> = serde_json::from_str(s).unwrap();

        d.extend(d.clone());

        ResponsivePhotoGrid::new(d, [3, 4, 5, 8, 12], |x, size| {
            RoundedAspectRatio::<2>::from_aspect_ratio(&x.aspect_ratio).clamp_width_to(size)
        })
        .grow_to_width()
    }

    pub fn cached() -> Self {
        let s = r#"{"grids":[{"grid":[{"data":0,"size":{"width":3,"height":2},"origin":{"x":0,"y":0}},{"data":1,"size":{"width":3,"height":2},"origin":{"x":0,"y":2}},{"data":2,"size":{"width":3,"height":2},"origin":{"x":0,"y":4}},{"data":3,"size":{"width":3,"height":2},"origin":{"x":0,"y":6}},{"data":4,"size":{"width":3,"height":2},"origin":{"x":0,"y":8}},{"data":5,"size":{"width":3,"height":2},"origin":{"x":0,"y":10}},{"data":6,"size":{"width":3,"height":2},"origin":{"x":0,"y":12}},{"data":7,"size":{"width":3,"height":3},"origin":{"x":0,"y":14}},{"data":8,"size":{"width":3,"height":1},"origin":{"x":0,"y":17}},{"data":9,"size":{"width":3,"height":2},"origin":{"x":0,"y":18}},{"data":10,"size":{"width":3,"height":2},"origin":{"x":0,"y":20}},{"data":11,"size":{"width":3,"height":2},"origin":{"x":0,"y":22}},{"data":12,"size":{"width":3,"height":2},"origin":{"x":0,"y":24}},{"data":13,"size":{"width":3,"height":3},"origin":{"x":0,"y":26}},{"data":14,"size":{"width":3,"height":2},"origin":{"x":0,"y":29}},{"data":15,"size":{"width":3,"height":3},"origin":{"x":0,"y":31}},{"data":16,"size":{"width":3,"height":3},"origin":{"x":0,"y":34}},{"data":17,"size":{"width":3,"height":3},"origin":{"x":0,"y":37}},{"data":18,"size":{"width":3,"height":2},"origin":{"x":0,"y":40}},{"data":19,"size":{"width":3,"height":3},"origin":{"x":0,"y":42}},{"data":20,"size":{"width":3,"height":2},"origin":{"x":0,"y":45}},{"data":21,"size":{"width":3,"height":2},"origin":{"x":0,"y":47}},{"data":22,"size":{"width":3,"height":2},"origin":{"x":0,"y":49}},{"data":23,"size":{"width":3,"height":2},"origin":{"x":0,"y":51}},{"data":24,"size":{"width":3,"height":2},"origin":{"x":0,"y":53}},{"data":25,"size":{"width":3,"height":2},"origin":{"x":0,"y":55}},{"data":26,"size":{"width":3,"height":2},"origin":{"x":0,"y":57}},{"data":27,"size":{"width":3,"height":2},"origin":{"x":0,"y":59}},{"data":28,"size":{"width":3,"height":2},"origin":{"x":0,"y":61}},{"data":29,"size":{"width":3,"height":2},"origin":{"x":0,"y":63}},{"data":30,"size":{"width":3,"height":1},"origin":{"x":0,"y":65}},{"data":31,"size":{"width":3,"height":2},"origin":{"x":0,"y":66}},{"data":32,"size":{"width":3,"height":2},"origin":{"x":0,"y":68}},{"data":33,"size":{"width":3,"height":2},"origin":{"x":0,"y":70}}],"width":3},{"grid":[{"data":0,"size":{"width":4,"height":2},"origin":{"x":0,"y":0}},{"data":1,"size":{"width":4,"height":2},"origin":{"x":0,"y":2}},{"data":2,"size":{"width":4,"height":2},"origin":{"x":0,"y":4}},{"data":3,"size":{"width":4,"height":2},"origin":{"x":0,"y":6}},{"data":4,"size":{"width":4,"height":2},"origin":{"x":0,"y":8}},{"data":5,"size":{"width":4,"height":2},"origin":{"x":0,"y":10}},{"data":6,"size":{"width":4,"height":2},"origin":{"x":0,"y":12}},{"data":7,"size":{"width":2,"height":3},"origin":{"x":0,"y":14}},{"data":13,"size":{"width":2,"height":3},"origin":{"x":2,"y":14}},{"data":8,"size":{"width":4,"height":1},"origin":{"x":0,"y":17}},{"data":9,"size":{"width":4,"height":2},"origin":{"x":0,"y":18}},{"data":10,"size":{"width":4,"height":2},"origin":{"x":0,"y":20}},{"data":11,"size":{"width":4,"height":2},"origin":{"x":0,"y":22}},{"data":12,"size":{"width":4,"height":2},"origin":{"x":0,"y":24}},{"data":14,"size":{"width":4,"height":2},"origin":{"x":0,"y":26}},{"data":15,"size":{"width":2,"height":3},"origin":{"x":0,"y":28}},{"data":16,"size":{"width":2,"height":3},"origin":{"x":2,"y":28}},{"data":17,"size":{"width":2,"height":3},"origin":{"x":0,"y":31}},{"data":19,"size":{"width":2,"height":3},"origin":{"x":2,"y":31}},{"data":18,"size":{"width":4,"height":2},"origin":{"x":0,"y":34}},{"data":20,"size":{"width":4,"height":2},"origin":{"x":0,"y":36}},{"data":21,"size":{"width":4,"height":2},"origin":{"x":0,"y":38}},{"data":22,"size":{"width":4,"height":2},"origin":{"x":0,"y":40}},{"data":23,"size":{"width":4,"height":2},"origin":{"x":0,"y":42}},{"data":24,"size":{"width":4,"height":2},"origin":{"x":0,"y":44}},{"data":25,"size":{"width":4,"height":2},"origin":{"x":0,"y":46}},{"data":26,"size":{"width":4,"height":2},"origin":{"x":0,"y":48}},{"data":27,"size":{"width":4,"height":2},"origin":{"x":0,"y":50}},{"data":28,"size":{"width":4,"height":2},"origin":{"x":0,"y":52}},{"data":29,"size":{"width":4,"height":2},"origin":{"x":0,"y":54}},{"data":30,"size":{"width":4,"height":1},"origin":{"x":0,"y":56}},{"data":31,"size":{"width":4,"height":2},"origin":{"x":0,"y":57}},{"data":32,"size":{"width":4,"height":2},"origin":{"x":0,"y":59}},{"data":33,"size":{"width":4,"height":2},"origin":{"x":0,"y":61}}],"width":4},{"grid":[{"data":0,"size":{"width":3,"height":2},"origin":{"x":0,"y":0}},{"data":7,"size":{"width":2,"height":3},"origin":{"x":3,"y":0}},{"data":1,"size":{"width":3,"height":2},"origin":{"x":0,"y":2}},{"data":13,"size":{"width":2,"height":3},"origin":{"x":3,"y":3}},{"data":2,"size":{"width":3,"height":2},"origin":{"x":0,"y":4}},{"data":3,"size":{"width":3,"height":2},"origin":{"x":0,"y":6}},{"data":15,"size":{"width":2,"height":3},"origin":{"x":3,"y":6}},{"data":4,"size":{"width":3,"height":2},"origin":{"x":0,"y":8}},{"data":16,"size":{"width":2,"height":3},"origin":{"x":3,"y":9}},{"data":5,"size":{"width":3,"height":2},"origin":{"x":0,"y":10}},{"data":6,"size":{"width":5,"height":2},"origin":{"x":0,"y":12}},{"data":8,"size":{"width":5,"height":1},"origin":{"x":0,"y":14}},{"data":9,"size":{"width":3,"height":2},"origin":{"x":0,"y":15}},{"data":17,"size":{"width":2,"height":3},"origin":{"x":3,"y":15}},{"data":10,"size":{"width":3,"height":2},"origin":{"x":0,"y":17}},{"data":19,"size":{"width":2,"height":3},"origin":{"x":3,"y":18}},{"data":11,"size":{"width":3,"height":2},"origin":{"x":0,"y":19}},{"data":12,"size":{"width":5,"height":2},"origin":{"x":0,"y":21}},{"data":14,"size":{"width":5,"height":2},"origin":{"x":0,"y":23}},{"data":18,"size":{"width":5,"height":2},"origin":{"x":0,"y":25}},{"data":20,"size":{"width":5,"height":2},"origin":{"x":0,"y":27}},{"data":21,"size":{"width":5,"height":2},"origin":{"x":0,"y":29}},{"data":22,"size":{"width":5,"height":2},"origin":{"x":0,"y":31}},{"data":23,"size":{"width":5,"height":2},"origin":{"x":0,"y":33}},{"data":24,"size":{"width":5,"height":2},"origin":{"x":0,"y":35}},{"data":25,"size":{"width":5,"height":2},"origin":{"x":0,"y":37}},{"data":26,"size":{"width":5,"height":2},"origin":{"x":0,"y":39}},{"data":27,"size":{"width":5,"height":2},"origin":{"x":0,"y":41}},{"data":28,"size":{"width":5,"height":2},"origin":{"x":0,"y":43}},{"data":29,"size":{"width":5,"height":2},"origin":{"x":0,"y":45}},{"data":30,"size":{"width":5,"height":2},"origin":{"x":0,"y":47}},{"data":31,"size":{"width":5,"height":2},"origin":{"x":0,"y":49}},{"data":32,"size":{"width":5,"height":2},"origin":{"x":0,"y":51}},{"data":33,"size":{"width":5,"height":2},"origin":{"x":0,"y":53}}],"width":5},{"grid":[{"data":0,"size":{"width":3,"height":2},"origin":{"x":0,"y":0}},{"data":1,"size":{"width":3,"height":2},"origin":{"x":3,"y":0}},{"data":7,"size":{"width":2,"height":3},"origin":{"x":6,"y":0}},{"data":2,"size":{"width":3,"height":2},"origin":{"x":0,"y":2}},{"data":3,"size":{"width":3,"height":2},"origin":{"x":3,"y":2}},{"data":13,"size":{"width":2,"height":3},"origin":{"x":6,"y":3}},{"data":4,"size":{"width":3,"height":2},"origin":{"x":0,"y":4}},{"data":5,"size":{"width":3,"height":2},"origin":{"x":3,"y":4}},{"data":6,"size":{"width":3,"height":2},"origin":{"x":0,"y":6}},{"data":9,"size":{"width":5,"height":2},"origin":{"x":3,"y":6}},{"data":8,"size":{"width":8,"height":2},"origin":{"x":0,"y":8}},{"data":10,"size":{"width":3,"height":2},"origin":{"x":0,"y":10}},{"data":11,"size":{"width":3,"height":2},"origin":{"x":3,"y":10}},{"data":15,"size":{"width":2,"height":3},"origin":{"x":6,"y":10}},{"data":12,"size":{"width":3,"height":2},"origin":{"x":0,"y":12}},{"data":14,"size":{"width":3,"height":2},"origin":{"x":3,"y":12}},{"data":16,"size":{"width":2,"height":3},"origin":{"x":6,"y":13}},{"data":17,"size":{"width":2,"height":3},"origin":{"x":0,"y":14}},{"data":18,"size":{"width":3,"height":2},"origin":{"x":2,"y":14}},{"data":19,"size":{"width":2,"height":3},"origin":{"x":2,"y":16}},{"data":20,"size":{"width":4,"height":2},"origin":{"x":4,"y":16}},{"data":21,"size":{"width":4,"height":2},"origin":{"x":4,"y":18}},{"data":22,"size":{"width":3,"height":2},"origin":{"x":0,"y":19}},{"data":23,"size":{"width":5,"height":2},"origin":{"x":3,"y":20}},{"data":24,"size":{"width":3,"height":2},"origin":{"x":0,"y":21}},{"data":25,"size":{"width":5,"height":2},"origin":{"x":3,"y":22}},{"data":26,"size":{"width":3,"height":2},"origin":{"x":0,"y":23}},{"data":27,"size":{"width":5,"height":2},"origin":{"x":3,"y":24}},{"data":28,"size":{"width":3,"height":2},"origin":{"x":0,"y":25}},{"data":29,"size":{"width":5,"height":2},"origin":{"x":3,"y":26}},{"data":30,"size":{"width":5,"height":2},"origin":{"x":0,"y":28}},{"data":31,"size":{"width":3,"height":2},"origin":{"x":5,"y":28}},{"data":32,"size":{"width":3,"height":2},"origin":{"x":0,"y":30}},{"data":33,"size":{"width":5,"height":2},"origin":{"x":3,"y":30}}],"width":8},{"grid":[{"data":0,"size":{"width":3,"height":2},"origin":{"x":0,"y":0}},{"data":1,"size":{"width":3,"height":2},"origin":{"x":3,"y":0}},{"data":2,"size":{"width":3,"height":2},"origin":{"x":6,"y":0}},{"data":3,"size":{"width":3,"height":2},"origin":{"x":9,"y":0}},{"data":4,"size":{"width":3,"height":2},"origin":{"x":0,"y":2}},{"data":5,"size":{"width":3,"height":2},"origin":{"x":3,"y":2}},{"data":6,"size":{"width":3,"height":2},"origin":{"x":6,"y":2}},{"data":7,"size":{"width":3,"height":3},"origin":{"x":9,"y":2}},{"data":8,"size":{"width":8,"height":2},"origin":{"x":0,"y":4}},{"data":9,"size":{"width":4,"height":2},"origin":{"x":8,"y":5}},{"data":10,"size":{"width":3,"height":2},"origin":{"x":0,"y":6}},{"data":11,"size":{"width":3,"height":2},"origin":{"x":3,"y":6}},{"data":12,"size":{"width":3,"height":2},"origin":{"x":6,"y":7}},{"data":13,"size":{"width":3,"height":3},"origin":{"x":9,"y":7}},{"data":14,"size":{"width":3,"height":2},"origin":{"x":0,"y":8}},{"data":15,"size":{"width":2,"height":3},"origin":{"x":3,"y":8}},{"data":16,"size":{"width":2,"height":3},"origin":{"x":5,"y":9}},{"data":17,"size":{"width":2,"height":3},"origin":{"x":7,"y":9}},{"data":18,"size":{"width":3,"height":2},"origin":{"x":0,"y":10}},{"data":19,"size":{"width":3,"height":3},"origin":{"x":9,"y":10}},{"data":20,"size":{"width":3,"height":2},"origin":{"x":0,"y":12}},{"data":21,"size":{"width":3,"height":2},"origin":{"x":3,"y":12}},{"data":22,"size":{"width":3,"height":2},"origin":{"x":6,"y":12}},{"data":23,"size":{"width":3,"height":2},"origin":{"x":9,"y":13}},{"data":24,"size":{"width":3,"height":2},"origin":{"x":0,"y":14}},{"data":25,"size":{"width":3,"height":2},"origin":{"x":3,"y":14}},{"data":26,"size":{"width":3,"height":2},"origin":{"x":6,"y":14}},{"data":27,"size":{"width":3,"height":2},"origin":{"x":9,"y":15}},{"data":28,"size":{"width":3,"height":2},"origin":{"x":0,"y":16}},{"data":29,"size":{"width":3,"height":2},"origin":{"x":3,"y":16}},{"data":30,"size":{"width":6,"height":2},"origin":{"x":6,"y":17}},{"data":31,"size":{"width":3,"height":2},"origin":{"x":0,"y":18}},{"data":32,"size":{"width":3,"height":2},"origin":{"x":3,"y":18}},{"data":33,"size":{"width":6,"height":2},"origin":{"x":6,"y":19}}],"width":12}],"data":[{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4124-HDR.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4124-HDR.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4124-HDR.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4124-HDR.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4124-HDR.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4124-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":256,"height":171},"srcs":[{"dimensions":{"width":768,"height":513},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE5716.avif"},{"dimensions":{"width":2048,"height":1367},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE5716.avif"},{"dimensions":{"width":640,"height":427},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE5716.avif"},{"dimensions":{"width":1280,"height":854},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE5716.avif"},{"dimensions":{"width":1024,"height":683},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE5716.avif"},{"dimensions":{"width":1536,"height":1025},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE5716.avif"}],"metadata":{}},{"aspect_ratio":{"width":1536,"height":1025},"srcs":[{"dimensions":{"width":1536,"height":1025},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE6726.avif"},{"dimensions":{"width":640,"height":427},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE6726.avif"},{"dimensions":{"width":768,"height":513},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE6726.avif"},{"dimensions":{"width":2048,"height":1367},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE6726.avif"},{"dimensions":{"width":1280,"height":854},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE6726.avif"},{"dimensions":{"width":1024,"height":683},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE6726.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_2819.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_2819.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_2819.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_2819.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_2819.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_2819.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3968.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3968.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3968.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3968.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3968.avif"},{"dimensions":{"width":2048,"height":1362},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3968.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3943.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3943.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3943.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3943.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3943.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3943.avif"}],"metadata":{}},{"aspect_ratio":{"width":2048,"height":1363},"srcs":[{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4051-Enhanced-NR.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4051-Enhanced-NR.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4051-Enhanced-NR.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4051-Enhanced-NR.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4051-Enhanced-NR.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4051-Enhanced-NR.avif"}],"metadata":{}},{"aspect_ratio":{"width":384,"height":577},"srcs":[{"dimensions":{"width":768,"height":1154},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4090.avif"},{"dimensions":{"width":2048,"height":3078},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4090.avif"},{"dimensions":{"width":1280,"height":1924},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4090.avif"},{"dimensions":{"width":640,"height":962},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4090.avif"},{"dimensions":{"width":1024,"height":1539},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4090.avif"},{"dimensions":{"width":1536,"height":2309},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4090.avif"}],"metadata":{}},{"aspect_ratio":{"width":16,"height":5},"srcs":[{"dimensions":{"width":768,"height":240},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_0770-HDR-Pano.avif"},{"dimensions":{"width":1280,"height":400},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_0770-HDR-Pano.avif"},{"dimensions":{"width":1536,"height":480},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_0770-HDR-Pano.avif"},{"dimensions":{"width":2048,"height":640},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_0770-HDR-Pano.avif"},{"dimensions":{"width":640,"height":200},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_0770-HDR-Pano.avif"},{"dimensions":{"width":1024,"height":320},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_0770-HDR-Pano.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_2260.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_2260.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_2260.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_2260.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_2260.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_2260.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":681},"srcs":[{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4058.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4058.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4058.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4058.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4058.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4058.avif"}],"metadata":{}},{"aspect_ratio":{"width":320,"height":213},"srcs":[{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_1047.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_1047.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_1047.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_1047.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_1047.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_1047.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":681},"srcs":[{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3705-HDR.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3705-HDR.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3705-HDR.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3705-HDR.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3705-HDR.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3705-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":1539},"srcs":[{"dimensions":{"width":1024,"height":1539},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4576-HDR.avif"},{"dimensions":{"width":1536,"height":2309},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4576-HDR.avif"},{"dimensions":{"width":640,"height":962},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4576-HDR.avif"},{"dimensions":{"width":768,"height":1154},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4576-HDR.avif"},{"dimensions":{"width":2048,"height":3078},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4576-HDR.avif"},{"dimensions":{"width":1280,"height":1924},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4576-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":320,"height":213},"srcs":[{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3424.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3424.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3424.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3424.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3424.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3424.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":1539},"srcs":[{"dimensions":{"width":1024,"height":1539},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4175-HDR.avif"},{"dimensions":{"width":640,"height":962},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4175-HDR.avif"},{"dimensions":{"width":1536,"height":2308},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4175-HDR.avif"},{"dimensions":{"width":2048,"height":3078},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4175-HDR.avif"},{"dimensions":{"width":1280,"height":1924},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4175-HDR.avif"},{"dimensions":{"width":768,"height":1154},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4175-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":384,"height":577},"srcs":[{"dimensions":{"width":768,"height":1154},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4095.avif"},{"dimensions":{"width":1280,"height":1924},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4095.avif"},{"dimensions":{"width":1024,"height":1539},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4095.avif"},{"dimensions":{"width":1536,"height":2309},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4095.avif"},{"dimensions":{"width":2048,"height":3078},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4095.avif"},{"dimensions":{"width":640,"height":962},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4095.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":1151},"srcs":[{"dimensions":{"width":768,"height":1151},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE7395.avif"},{"dimensions":{"width":1280,"height":1918},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE7395.avif"},{"dimensions":{"width":1536,"height":2302},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE7395.avif"},{"dimensions":{"width":2048,"height":3069},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE7395.avif"},{"dimensions":{"width":640,"height":959},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE7395.avif"},{"dimensions":{"width":1024,"height":1534},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE7395.avif"}],"metadata":{}},{"aspect_ratio":{"width":2048,"height":1367},"srcs":[{"dimensions":{"width":2048,"height":1367},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE7507.avif"},{"dimensions":{"width":1024,"height":683},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE7507.avif"},{"dimensions":{"width":640,"height":427},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE7507.avif"},{"dimensions":{"width":1536,"height":1025},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE7507.avif"},{"dimensions":{"width":1280,"height":854},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE7507.avif"},{"dimensions":{"width":768,"height":513},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE7507.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":1539},"srcs":[{"dimensions":{"width":2048,"height":3078},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3744-HDR.avif"},{"dimensions":{"width":1280,"height":1924},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3744-HDR.avif"},{"dimensions":{"width":1536,"height":2309},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3744-HDR.avif"},{"dimensions":{"width":768,"height":1154},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3744-HDR.avif"},{"dimensions":{"width":640,"height":962},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3744-HDR.avif"},{"dimensions":{"width":1024,"height":1539},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3744-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":320,"height":213},"srcs":[{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_5207.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_5207.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_5207.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_5207.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_5207.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_5207.avif"}],"metadata":{}},{"aspect_ratio":{"width":2048,"height":1367},"srcs":[{"dimensions":{"width":2048,"height":1367},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE6320.avif"},{"dimensions":{"width":1024,"height":683},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE6320.avif"},{"dimensions":{"width":1280,"height":854},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE6320.avif"},{"dimensions":{"width":640,"height":427},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE6320.avif"},{"dimensions":{"width":1536,"height":1025},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE6320.avif"},{"dimensions":{"width":768,"height":513},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE6320.avif"}],"metadata":{}},{"aspect_ratio":{"width":2048,"height":1363},"srcs":[{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_0320.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_0320.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_0320.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_0320.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_0320.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_0320.avif"}],"metadata":{}},{"aspect_ratio":{"width":2048,"height":1363},"srcs":[{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3455.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3455.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3455.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3455.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3455.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3455.avif"}],"metadata":{}},{"aspect_ratio":{"width":640,"height":427},"srcs":[{"dimensions":{"width":640,"height":427},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE4900.avif"},{"dimensions":{"width":1280,"height":854},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE4900.avif"},{"dimensions":{"width":2048,"height":1367},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE4900.avif"},{"dimensions":{"width":768,"height":513},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE4900.avif"},{"dimensions":{"width":1536,"height":1025},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE4900.avif"},{"dimensions":{"width":1024,"height":683},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE4900.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4783-Enhanced-NR.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4783-Enhanced-NR.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4783-Enhanced-NR.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4783-Enhanced-NR.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4783-Enhanced-NR.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4783-Enhanced-NR.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_2365.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_2365.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_2365.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_2365.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_2365.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_2365.avif"}],"metadata":{}},{"aspect_ratio":{"width":320,"height":213},"srcs":[{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_4931-HDR.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_4931-HDR.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_4931-HDR.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_4931-HDR.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_4931-HDR.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_4931-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":320,"height":213},"srcs":[{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_0700-2.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_0700-2.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_0700-2.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_0700-2.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_0700-2.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_0700-2.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":683},"srcs":[{"dimensions":{"width":1024,"height":683},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/_AYE6501.avif"},{"dimensions":{"width":640,"height":427},"url":"https://cdn.seanaye.ca/resized/640x4294967295/_AYE6501.avif"},{"dimensions":{"width":2048,"height":1367},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/_AYE6501.avif"},{"dimensions":{"width":1280,"height":854},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/_AYE6501.avif"},{"dimensions":{"width":1536,"height":1025},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/_AYE6501.avif"},{"dimensions":{"width":768,"height":513},"url":"https://cdn.seanaye.ca/resized/768x4294967295/_AYE6501.avif"}],"metadata":{}},{"aspect_ratio":{"width":1024,"height":419},"srcs":[{"dimensions":{"width":2048,"height":838},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_5018-HDR-Pano.avif"},{"dimensions":{"width":640,"height":262},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_5018-HDR-Pano.avif"},{"dimensions":{"width":1024,"height":419},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_5018-HDR-Pano.avif"},{"dimensions":{"width":1280,"height":524},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_5018-HDR-Pano.avif"},{"dimensions":{"width":768,"height":314},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_5018-HDR-Pano.avif"},{"dimensions":{"width":1536,"height":629},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_5018-HDR-Pano.avif"}],"metadata":{}},{"aspect_ratio":{"width":2048,"height":1363},"srcs":[{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_3278.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_3278.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_3278.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_3278.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_3278.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_3278.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_5084-HDR.avif"},{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_5084-HDR.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_5084-HDR.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_5084-HDR.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_5084-HDR.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_5084-HDR.avif"}],"metadata":{}},{"aspect_ratio":{"width":768,"height":511},"srcs":[{"dimensions":{"width":768,"height":511},"url":"https://cdn.seanaye.ca/resized/768x4294967295/DSC_5472.avif"},{"dimensions":{"width":2048,"height":1363},"url":"https://cdn.seanaye.ca/resized/2048x4294967295/DSC_5472.avif"},{"dimensions":{"width":1536,"height":1022},"url":"https://cdn.seanaye.ca/resized/1536x4294967295/DSC_5472.avif"},{"dimensions":{"width":1280,"height":852},"url":"https://cdn.seanaye.ca/resized/1280x4294967295/DSC_5472.avif"},{"dimensions":{"width":1024,"height":681},"url":"https://cdn.seanaye.ca/resized/1024x4294967295/DSC_5472.avif"},{"dimensions":{"width":640,"height":426},"url":"https://cdn.seanaye.ca/resized/640x4294967295/DSC_5472.avif"}],"metadata":{}}]}"#;

        serde_json::from_str(s).unwrap()
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
