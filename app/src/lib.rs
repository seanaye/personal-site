use crate::error_template::{AppError, ErrorTemplate};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use photogrid::{PhotoLayoutData, ResponsivePhotoGrid};
use std::sync::Arc;

pub mod error_template;
mod style;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css" />

        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! { <ErrorTemplate outside_errors /> }.into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    view! { <PhotoGridComponent /> }
}

#[island]
fn Counter() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);

    view! {
        <button on:click=move |_| {
            set_count.update(|count| *count += 1)
        }>"Click me: " {move || count()}</button>
    }
}

#[component]
fn PhotoGridComponent() -> impl IntoView {
    use style::*;

    let data = use_context::<Arc<ResponsivePhotoGrid<PhotoLayoutData>>>();

    let Some(g) = data else {
        dbg!("else");
        return view! {}.into_view();
    };

    let _ = "hidden col-span-1 row-span-1 col-span-2 row-span-2 col-span-3 row-span-3 col-span-4 row-span-4 col-start-1 row-start-1 col-start-2 row-start-2 col-start-3 row-start-3 col-start-4 row-start-4 col-start-5 row-start-5 col-start-6 row-start-6 col-start-7 row-start-7 col-start-8 row-start-8";

    g.grids()
        .map(|grid| {
            view! {
                <div class=grid
                    .style(
                        GridOuterClass,
                    )>
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
        .collect_view()
}
