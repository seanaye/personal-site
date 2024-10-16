use grid::Size;
use leptos::prelude::*;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid, SrcSet};
use std::sync::Arc;
use url::Url;

#[island]
pub fn SelectionProvider(children: Children) -> impl IntoView {
    let (selected, set_selected) = signal(Vec::new());
    provide_context(PhotoSelectionContext {
        set_selected,
        selected,
    });
    children()
}

#[derive(Clone, Copy)]
struct PhotoSelectionContext {
    set_selected: WriteSignal<Vec<Url>>,
    selected: ReadSignal<Vec<Url>>,
}

impl PhotoSelectionContext {
    fn toggle_selected(&self, url: Url) {
        self.set_selected
            .update(|val| match self.is_selected(&url) {
                Some(idx) => {
                    val.remove(idx);
                }
                None => {
                    val.push(url.clone());
                }
            });
    }

    fn is_selected(&self, url: &Url) -> Option<usize> {
        self.selected
            .read_untracked()
            .iter()
            .enumerate()
            .find_map(|(idx, x)| (x == url).then_some(idx))
    }
}

fn use_is_selected(url: Url) -> Signal<Option<usize>> {
    let PhotoSelectionContext { selected, .. } = expect_context();
    Signal::derive(move || {
        selected
            .read()
            .iter()
            .enumerate()
            .find_map(|(idx, e)| (e == &url).then_some(idx))
    })
}

#[component]
pub fn PhotoGridComponent(data: Vec<PhotoLayoutData>) -> impl IntoView {
    use crate::style::*;

    let data = ResponsivePhotoGrid::from_layout_data(data);

    let _ = "col-span-1 col-span-2 col-span-3 col-span-4 col-span-5 col-span-6 col-span-7 col-span-8 col-span-9 col-span-10 col-span-11 col-span-12";
    let _ = "row-span-1 row-span-2 row-span-3 row-span-4 row-span-5 row-span-6 row-span-7 row-span-8 row-span-9 row-span-10 row-span-11 row-span-12";
    let _ = "col-start-1 col-start-2 col-start-3 col-start-4 col-start-5 col-start-6 col-start-7 col-start-8 col-start-9 col-start-10 col-start-11 col-start-12";
    let _ = "row-start-1 row-start-2 row-start-3 row-start-4 row-start-5 row-start-6 row-start-7 row-start-8 row-start-9 row-start-10 row-start-11 row-start-12";

    data.grids()
        .map(|grid| {
            let class = grid.style(GridOuterClass);
            view! {
                <div class=class>
                    {grid
                        .grid
                        .into_iter()
                        .map(move |c| {
                            let content = c.content();
                            let class = format!(
                                "p-1 flex items-center justify-center {}",
                                c.style(GridElemClass),
                            );
                            let style = c.style(GridElemStyle);
                            let srcset = srcsets(content.srcs.iter());
                            view! {
                                <SinglePhoto class=class style=style>
                                    <img
                                        class="object-contain max-h-full max-w-full w-full"
                                        srcset=srcset
                                        loading="lazy"
                                    />
                                </SinglePhoto>
                            }
                        })
                        .collect_view()}
                </div>
            }
        })
        .collect_view()
}

#[component]
fn SinglePhoto(class: String, style: String, children: Children) -> impl IntoView {
    view! {
        <div class=class style=style>
            {children()}
        </div>
    }
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

fn original(s: &Url) -> Url {
    let mut out = s.clone();
    let segments = s.path_segments().expect("This must have a base");

    out.path_segments_mut()
        .expect("this must have a base")
        .clear()
        .push("original")
        .push(
            segments
                .last()
                .expect("There is a file here")
                .replace(".avif", ".jpg")
                .as_str(),
        );
    out
}
