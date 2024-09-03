use crate::error_template::{AppError, ErrorTemplate};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::*;
mod canvas_grid;
mod components;
mod hooks;

use components::*;

pub mod error_template;
mod style;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options islands=true />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Html />
        <Stylesheet id="leptos" href="/pkg/start-axum-workspace.css" />

        // sets the document title
        <Title text="Welcome to Leptos" />

        // content for this welcome page
        <Router>
            <main>
                <Routes fallback=|| {
                    let mut outside_errors = Errors::default();
                    outside_errors.insert_with_default_key(AppError::NotFound);
                    view! { <ErrorTemplate outside_errors /> }.into_view()
                }>
                    <Route path=StaticSegment("") view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let description = "description";
    view! {
        <Canvas>
            <div class="flex justify-center items-center absolute inset-0">
                <div class="max-w-3xl mx-auto p-4 mx-6 bg-gray-200 rounded-lg">
                    <div class="sm:flex">
                        // <div class="flex-shrink-0 flex items-center px-4">
                        // <img
                        // class="h-32 w-32 border border-gray-300 text-gray-300 object-contain rounded-full mx-auto"
                        // src="https://seanaye.ca/avatar.jpg?__frsh_c=c080ff30930a3a1ad7a60e278c943eac618b5b4a"
                        // alt="Headshot of Sean Aye"
                        // />
                        // </div>
                        <div class="font-mono">
                            <h1 class="text-xl font-bold text-center sm:text-left">Sean Aye</h1>
                            <p class="my-4">{description}</p>
                            <Slider />
                        </div>
                    </div>
                </div>
            </div>
        </Canvas>
        <PhotoGridComponent />
    }
}
