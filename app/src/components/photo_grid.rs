use grid::Size;
use leptos::prelude::*;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use std::sync::Arc;

#[component]
pub fn PhotoGridComponent() -> impl IntoView {
    use crate::style::*;

    let data = use_context::<Arc<ResponsivePhotoGrid<PhotoLayoutData>>>();

    let Some(g) = data else {
        return view! {}.into_any();
    };

    let _ = "hidden col-span-1 row-span-1 col-span-2 row-span-2 col-span-3 row-span-3 col-span-4 row-span-4 col-start-1 row-start-1 col-start-2 row-start-2 col-start-3 row-start-3 col-start-4 row-start-4 col-start-5 row-start-5 col-start-6 row-start-6 col-start-7 row-start-7 col-start-8 row-start-8";

    let inner = g
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
                                        "p-4 flex items-center justify-center {}",
                                        c.style(GridElemClass),
                                    )
                                    style=c.style(GridElemStyle)
                                >
                                    <img
                                        class="object-contain max-h-full max-w-full"
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
