use leptos::prelude::*;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid};
use std::sync::Arc;

#[component]
pub fn PhotoGridComponent() -> impl IntoView {
    use crate::style::*;

    let data = use_context::<Arc<ResponsivePhotoGrid<PhotoLayoutData>>>();

    let Some(g) = data else {
        dbg!("else");
        return view! {}.into_any();
    };
    dbg!("render");

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
                                        src=c.content().srcs[0].url.to_string()
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
