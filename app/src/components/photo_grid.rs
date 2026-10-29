use grid::Size;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use topcoat::{
    Result,
    view::{View, component, view},
};
use url::Url;

use crate::style::*;

#[component]
pub async fn photo_grid_component(data: Vec<PhotoLayoutData>) -> Result<impl View> {
    let data = ResponsivePhotoGrid::from_layout_data(data);

    // Keep every responsive grid class visible to Tailwind's source scanner.
    const GRID_CLASSES: &str = "col-span-1 col-span-2 col-span-3 col-span-4 col-span-5 col-span-6 col-span-7 col-span-8 col-span-9 col-span-10 col-span-11 col-span-12 row-span-1 row-span-2 row-span-3 row-span-4 row-span-5 row-span-6 row-span-7 row-span-8 row-span-9 row-span-10 row-span-11 row-span-12 col-start-1 col-start-2 col-start-3 col-start-4 col-start-5 col-start-6 col-start-7 col-start-8 col-start-9 col-start-10 col-start-11 col-start-12 row-start-1 row-start-2 row-start-3 row-start-4 row-start-5 row-start-6 row-start-7 row-start-8 row-start-9 row-start-10 row-start-11 row-start-12";
    let _ = GRID_CLASSES;

    Ok(view! {
        for grid in data.grids() {
            let class = grid.style(GridOuterClass);
            <div class=(class)>
                for cell in grid.grid {
                    let content = cell.content();
                    let class = format!(
                        "p-1 flex items-center justify-center {}",
                        cell.style(GridElemClass),
                    );
                    let style = cell.style(GridElemStyle);
                    let srcset = srcsets(content.srcs.iter());
                    let href = content.href.clone();
                    <div class=(class) style=(style)>
                        match href {
                            Some(href) => {
                                <a
                                    class="contents"
                                    href=(href.to_string())
                                    target="_blank"
                                    rel="noopener"
                                >
                                    <img
                                        class="object-contain max-h-full max-w-full w-full"
                                        srcset=(srcset)
                                        loading="lazy"
                                    >
                                </a>
                            },
                            None => {
                                <img
                                    class="object-contain max-h-full max-w-full w-full"
                                    srcset=(srcset)
                                    loading="lazy"
                                >
                            },
                        }
                    </div>
                }
            </div>
        }
    })
}

fn srcsets<'a>(sources: impl Iterator<Item = &'a SrcSet>) -> String {
    sources.fold(String::new(), |mut acc, cur| {
        acc.push_str(cur.url.as_str());
        acc.push(' ');
        acc.push_str(cur.dimensions.width().to_string().as_str());
        acc.push_str("w,");
        acc
    })
}

#[allow(dead_code)]
fn original(s: &Url) -> Url {
    let mut out = s.clone();
    let mut segments = s.path_segments().expect("This must have a base");

    out.path_segments_mut()
        .expect("this must have a base")
        .clear()
        .push("original")
        .push(
            segments
                .next_back()
                .expect("There is a file here")
                .replace(".avif", ".jpg")
                .as_str(),
        );
    out
}
