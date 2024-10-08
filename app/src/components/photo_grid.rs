use grid::Size;
use leptos::prelude::*;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use std::sync::Arc;

#[component]
pub fn PhotoGridComponent(data: Arc<[PhotoLayoutData]>) -> impl IntoView {
    use crate::style::*;

    let data = ResponsivePhotoGrid::from_layout_data(data);

    let _ = "col-span-1 col-span-2 col-span-3 col-span-4 col-span-5 col-span-6 col-span-7 col-span-8 col-span-9 col-span-10 col-span-11 col-span-12";
    let _ = "row-span-1 row-span-2 row-span-3 row-span-4 row-span-5 row-span-6 row-span-7 row-span-8 row-span-9 row-span-10 row-span-11 row-span-12";
    let _ = "col-start-1 col-start-2 col-start-3 col-start-4 col-start-5 col-start-6 col-start-7 col-start-8 col-start-9 col-start-10 col-start-11 col-start-12";
    let _ = "row-start-1 row-start-2 row-start-3 row-start-4 row-start-5 row-start-6 row-start-7 row-start-8 row-start-9 row-start-10 row-start-11 row-start-12";

    let inner = data
        .grids()
        .map(|grid| {
            let class = grid.style(GridOuterClass);
            view! {
                <div class=class>
                    {grid
                        .grid
                        .into_iter()
                        .map(move |c| {
                            view! {
                                <div
                                    class=format!(
                                        "p-1 flex items-center justify-center {}",
                                        c.style(GridElemClass),
                                    )
                                    style=c.style(GridElemStyle)
                                >
                                    <img
                                        class="object-contain max-h-full max-w-full w-full"
                                        srcset=srcsets(c.content().srcs.iter())
                                    />
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
        })
        .collect_view();

    view! { <div class="">{inner}</div> }.into_any()
}

fn srcsets<'a>(s: impl Iterator<Item = &'a SrcSet>) -> String {
    s.fold(String::new(), |mut acc, cur| {
        acc.push_str(cur.url.as_str());
        acc.push(' ');
        acc.push_str(cur.dimensions.width().to_string().as_str());
        acc.push_str("w,");
        acc
    })
}
