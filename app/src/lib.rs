use crate::error_template::{AppError, ErrorTemplate};
use canvas_grid::{CanvasEventManager, CanvasParams, Draw, Events, LiquidGridImageCanvas};
use grid::{Coord, Dimension};
use hooks::use_window_size;
use leptos::*;
use leptos_dom::helpers::AnimationFrameRequestHandle;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::use_window;
use num_traits::cast::FromPrimitive;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

mod canvas_grid;
mod components;
mod hooks;

use components::*;

pub mod error_template;
mod style;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Html class="" />
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
    view! {
        <Canvas>
            <div class="flex justify-center items-center absolute inset-0">
                <div class="w-64 bg-white">{"Hello world"}</div>
            </div>
        </Canvas>

        <PhotoGridComponent />
    }
}
