use crate::error_template::{AppError, ErrorTemplate};
use canvas_grid::PolineManager;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::*;
mod canvas_grid;
mod components;
mod hooks;
mod log_js_trait;
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
        <Stylesheet id="leptos" href="/pkg/personal_site.css" />

        // sets the document title
        <Title text="seanaye.ca" />

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
fn Bio() -> impl IntoView {
    let description = "code | photography | music";
    let description_2 = "Senior Rust Developer at 1Password";
    let shadow = "box-shadow: rgba(68, 64, 60, 0.8) 2rem 2rem;";

    view! {
        <div class="flex justify-center items-center absolute inset-0">
            <div class="max-w-3xl mx-auto p-4 mx-6 bg-white" style=shadow>
                <div class="sm:flex">
                    <div class="flex-shrink-0 flex items-center px-4">
                        <img
                            class="h-32 w-32 border border-gray-300 text-gray-300 object-contain rounded-full mx-auto"
                            src="/avatar.jpg"
                            alt="Headshot of Sean Aye"
                        />
                    </div>
                    <div class="font-mono px-4">
                        <h1 class="text-2xl text-center sm:text-left squiggly">sean aye</h1>
                        <p class="my-4">{description}</p>
                        <p class="my-4">{description_2}</p>
                        <div class="flex flex-row justify-between items-center">
                            <Slider />
                            <div class="flex gap-2">
                                <a
                                    href="https://github.com/seanaye"
                                    class="text-stone-700 hover:text-stone-900"
                                >
                                    <GithubIcon />
                                </a>
                                <a
                                    href="mailto:hello@seanaye.ca"
                                    class="text-stone-700 hover:text-stone-900"
                                >
                                    <MailIcon />
                                </a>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="hidden">
                <svg xmlns="http://www.w3.org/2000/svg" version="1.1">
                    <defs>
                        <filter id="squiggly-0">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="0"
                            />
                            <feDisplacementMap
                                id="displacement"
                                in="SourceGraphic"
                                in2="noise"
                                scale="6"
                            />
                        </filter>
                        <filter id="squiggly-1">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="1"
                            />
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="5" />
                        </filter>

                        <filter id="squiggly-2">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="2"
                            />
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="6" />
                        </filter>
                        <filter id="squiggly-3">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="3"
                            />
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="5" />
                        </filter>

                        <filter id="squiggly-4">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="4"
                            />
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="6" />
                        </filter>
                    </defs>
                </svg>
            </div>
        </div>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <SliderProvider>
            <Canvas>
                <DebugPoline />
                <Bio />
            </Canvas>
            <Gradient>
                <div />
            </Gradient>
        </SliderProvider>
        <PhotoGridComponent />
    }
}

#[island]
fn Gradient(children: Children) -> impl IntoView {
    let SliderHue { poline, .. } = expect_slider_hue();

    let color = Signal::derive(move || {
        leptos::logging::log!("re run signal");
        let p = poline.read();
        let colors = p.colors();
        let len = colors.len();
        if len == 0 {
            return "rgba(0,0,0,1)".to_string();
        }

        let [r, g, b] = unsafe { colors.get_unchecked(len / 2) };

        format!("rgba({r},{g},{b},1)")
    });

    view! {
        <div class="w-full h-32 isolate relative">
            <div
                class="mix-blend-multiply top-0 w-full h-full absolute"
                style=move || {
                    let c = color.get();
                    format!("background: {};", c)
                }
            />
            <div
                style="
                background: linear-gradient(to top, white, transparent),
                url(/noise.svg);
                filter: contrast(290%) brightness(1000%);
                "
                class="w-full h-full noise"
            />
            {children()}
        </div>
    }
}

#[component]
fn GithubIcon() -> impl IntoView {
    view! {
        <svg
            class="w-6 h-6"
            xmlns="http://www.w3.org/2000/svg"
            width="1em"
            height="1em"
            viewBox="0 0 24 24"
        >
            <path
                fill="currentColor"
                d="M12 2A10 10 0 0 0 2 12c0 4.42 2.87 8.17 6.84 9.5c.5.08.66-.23.66-.5v-1.69c-2.77.6-3.36-1.34-3.36-1.34c-.46-1.16-1.11-1.47-1.11-1.47c-.91-.62.07-.6.07-.6c1 .07 1.53 1.03 1.53 1.03c.87 1.52 2.34 1.07 2.91.83c.09-.65.35-1.09.63-1.34c-2.22-.25-4.55-1.11-4.55-4.92c0-1.11.38-2 1.03-2.71c-.1-.25-.45-1.29.1-2.64c0 0 .84-.27 2.75 1.02c.79-.22 1.65-.33 2.5-.33s1.71.11 2.5.33c1.91-1.29 2.75-1.02 2.75-1.02c.55 1.35.2 2.39.1 2.64c.65.71 1.03 1.6 1.03 2.71c0 3.82-2.34 4.66-4.57 4.91c.36.31.69.92.69 1.85V21c0 .27.16.59.67.5C19.14 20.16 22 16.42 22 12A10 10 0 0 0 12 2"
            />
        </svg>
    }
}

#[component]
fn MailIcon() -> impl IntoView {
    view! {
        <svg
            class="w-6 h-6"
            xmlns="http://www.w3.org/2000/svg"
            width="1em"
            height="1em"
            viewBox="0 0 24 24"
        >
            <path
                fill="currentColor"
                d="m20 8l-8 5l-8-5V6l8 5l8-5m0-2H4c-1.11 0-2 .89-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2"
            />
        </svg>
    }
}
