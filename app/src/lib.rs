use std::{path::PathBuf, sync::Arc};

use photo_search::SearchFilter;
use photogrid::PhotoLayoutData;
use pulldown_cmark::{Options, Parser, html};
use rand::seq::SliceRandom;
use topcoat::{
    Result,
    context::{Cx, context},
    router::{
        HeaderValue, Router, RouterBuilder, Slot, StatusCode,
        error::{NotFoundError, RouterErrorExt, redirect},
        header, layout, not_found, page, path_param, query_params,
        request::uri,
    },
    tailwind,
    view::{Child, Unescaped, View, component, error_boundary, view},
};

pub mod components;
pub mod domain;
mod hue;
pub mod photo_share;
mod style;

use components::*;

use crate::domain::ports::BucketStorage;

not_found!("/");

/// Build the site's Topcoat router. The server adds its signed image endpoint
/// and static-file directory before finalizing the builder.
pub fn router<T: BucketStorage>(
    photos: Arc<[PhotoLayoutData]>,
    shared_photos: Option<photo_share::SharedPhotosFetcher>,
    bucket: T,
) -> RouterBuilder {
    let builder = Router::builder()
        .layout(root_layout)
        .page(home_page)
        .page(file_page::<T>::default())
        .page(blog_page)
        .page(blog_post_page)
        .page(photo_page)
        .page(photo_share::shared_photo_page)
        .page(search_page)
        .page(not_found)
        .with(photos)
        .with(bucket);

    if let Some(shared_photos) = shared_photos {
        builder.with(shared_photos)
    } else {
        builder
    }
}

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let title = if uri(cx).path().starts_with("/photo/") {
        "shared photos | seanaye.ca"
    } else {
        "seanaye.ca"
    };

    Ok(view! {
        // HTML now depends on a browser preference; shared caches must not
        // serve one visitor's palette to another.
        ((header::CACHE_CONTROL, HeaderValue::from_static("private, no-cache")))
        <!DOCTYPE html>
        <html lang="en" style=(hue::document_palette_style(cx))>
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(title)</title>
                <link id="topcoat" rel="stylesheet" href=(tailwind::stylesheet!())>
                <script type="module" src="/bootstrap.js"></script>
            </head>
            <body>
                <main class="relative">
                    error_boundary(
                        fallback: |error| {
                            if error.downcast_ref::<NotFoundError>().is_none() {
                                return Err(error);
                            }

                            Ok(view! {
                                (StatusCode::NOT_FOUND)
                                <div class="relative z-10 min-h-lvh pt-28 pr-4 pl-6 sm:pt-24 sm:pr-6 sm:pl-24">
                                    <h1>"Error"</h1>
                                    <h2>"404 Not Found"</h2>
                                    <p>"Error: Not Found"</p>
                                </div>
                            })
                        },
                        (slot)
                    )
                    nav_bar()
                    debug_poline()
                </main>
            </body>
        </html>
    })
}

#[component]
async fn layout_content(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div class="relative z-10 min-h-lvh pt-28 pr-4 pl-6 sm:pt-24 sm:pr-6 sm:pl-24">
            (child)
        </div>
    })
}

#[component]
async fn bio() -> Result<impl View> {
    let shadow = "box-shadow: rgba(68, 64, 60, 0.8) 2rem 2rem;";

    Ok(view! {
        <div class="flex justify-center items-center absolute inset-0">
            <div class="max-w-3xl p-4 mx-3 bg-white" style=(shadow)>
                <div class="sm:flex">
                    <div class="flex-shrink-0 flex items-center px-4">
                        <img
                            class="h-32 w-32 border border-gray-300 text-gray-300 object-contain rounded-full mx-auto"
                            src="/avatar.jpg"
                            alt="Headshot of Sean Aye"
                        >
                    </div>
                    <div class="font-mono px-4">
                        <h1 class="text-2xl text-center sm:text-left squiggly">"sean aye"</h1>
                        <p class="my-4">"code | photography | music"</p>
                        <p class="my-4">
                            "Tech Lead at "
                            <a href="https://macro.com" class="underline">"Macro"</a>
                            " 🦀 ❄️"
                        </p>
                        <div class="flex flex-row justify-end items-center">
                            <div class="flex gap-2">
                                <a
                                    href="https://github.com/seanaye"
                                    class="text-stone-700 hover:text-stone-900"
                                >
                                    github_icon()
                                </a>
                                <a
                                    href="mailto:hello@seanaye.ca"
                                    class="text-stone-700 hover:text-stone-900"
                                >
                                    mail_icon()
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
                            ></feTurbulence>
                            <feDisplacementMap
                                id="displacement"
                                in="SourceGraphic"
                                in2="noise"
                                scale="6"
                            ></feDisplacementMap>
                        </filter>
                        <filter id="squiggly-1">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="1"
                            ></feTurbulence>
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="5"></feDisplacementMap>
                        </filter>
                        <filter id="squiggly-2">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="2"
                            ></feTurbulence>
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="6"></feDisplacementMap>
                        </filter>
                        <filter id="squiggly-3">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="3"
                            ></feTurbulence>
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="5"></feDisplacementMap>
                        </filter>
                        <filter id="squiggly-4">
                            <feTurbulence
                                id="turbulence"
                                baseFrequency="0.02"
                                numOctaves="3"
                                result="noise"
                                seed="4"
                            ></feTurbulence>
                            <feDisplacementMap in="SourceGraphic" in2="noise" scale="6"></feDisplacementMap>
                        </filter>
                    </defs>
                </svg>
            </div>
        </div>
    })
}

#[page("/")]
async fn home_page() -> Result<impl View> {
    Ok(view! {
        <header class="contents">
            liquid_canvas(
                bio()
            )
        </header>
    })
}

path_param!(*file_path);

#[page("/file/{*file_path}")]
async fn file_page<T: BucketStorage>(cx: &Cx) -> Result<()> {
    let slug = path_param::<FilePath>(cx).collect::<PathBuf>();
    tracing::warn!("{slug:?}");
    let bucket: &T = context(cx);
    let url = bucket.get_url(slug).await.ok_or_unauthorized()?;
    Err(redirect(url).into())
}

fn markdown_to_html(text: &str) -> String {
    let parser = Parser::new_ext(text, Options::all());
    let mut html = String::new();
    html::push_html(&mut html, parser);
    html
}

#[component]
async fn markdown(content: &str) -> Result<impl View> {
    Ok(view! { <div>(Unescaped::new_unchecked(markdown_to_html(content)))</div> })
}

#[derive(Debug, Clone, Copy)]
struct BlogPost {
    slug: &'static str,
    title: &'static str,
    excerpt: Option<&'static str>,
    draft: bool,
    content: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/blog_posts.rs"));

fn blog_post(slug: &str) -> Option<&'static BlogPost> {
    BLOG_POSTS.iter().find(|post| post.slug == slug)
}

fn can_show_post(post: &BlogPost, show_drafts: bool) -> bool {
    !post.draft || show_drafts
}

#[query_params]
struct BlogQuery {
    draft: Option<String>,
}

fn show_drafts(cx: &Cx) -> bool {
    query_params::<BlogQuery>(cx)
        .ok()
        .and_then(|query| query.draft.as_deref())
        == Some("true")
}

const PROSE_CLASSES: &str = "prose font-mono [--tw-prose-body:currentColor] [--tw-prose-headings:currentColor] [--tw-prose-links:currentColor] [--tw-prose-bold:currentColor] [--tw-prose-counters:currentColor] [--tw-prose-bullets:currentColor] [--tw-prose-hr:currentColor] [--tw-prose-quotes:currentColor] [--tw-prose-quote-borders:currentColor] [--tw-prose-captions:currentColor] [--tw-prose-code:currentColor] [--tw-prose-pre-bg:var(--poline-code-background-color)] [--tw-prose-pre-code:var(--poline-code-text-color)] [--tw-prose-th-borders:currentColor] [--tw-prose-td-borders:currentColor] [&_code]:[text-shadow:none] [&_pre]:[text-shadow:none] prose-a:transition-opacity hover:prose-a:opacity-75";

#[page("/blog")]
async fn blog_page(cx: &Cx) -> Result<impl View> {
    let show_drafts = show_drafts(cx);
    let posts = BLOG_POSTS
        .iter()
        .filter(|post| can_show_post(post, show_drafts))
        .collect::<Vec<_>>();

    Ok(view! {
        <section class="contents">
            liquid_canvas(
                layout_content(
                    poline_text(
                        <div class=(PROSE_CLASSES)>
                            <h1>"blog"</h1>
                            if posts.is_empty() {
                                <p>"No posts yet."</p>
                            } else {
                                for post in posts {
                                    let href = if show_drafts {
                                        format!("/blog/{}?draft=true", post.slug)
                                    } else {
                                        format!("/blog/{}", post.slug)
                                    };
                                    <article>
                                        <h2><a href=(href)>(post.title)</a></h2>
                                        match post.excerpt {
                                            Some(excerpt) => <p>(excerpt)</p>,
                                            None => "",
                                        }
                                    </article>
                                }
                            }
                        </div>
                    )
                )
            )
        </section>
    })
}

path_param!(slug);

#[page("/blog/{slug}")]
async fn blog_post_page(cx: &Cx) -> Result<impl View> {
    let show_drafts = show_drafts(cx);
    let post = blog_post(path_param::<Slug>(cx)).filter(|post| can_show_post(post, show_drafts));

    Ok(view! {
        <article class="contents">
            static_canvas(
                layout_content(
                    poline_text(
                        <div class=(PROSE_CLASSES) style="text-shadow: none;">
                            match post {
                                Some(post) => markdown(content: post.content),
                                None => <p>"Post not found."</p>,
                            }
                        </div>
                    )
                )
            )
        </article>
    })
}

#[page("/photo")]
async fn photo_page() -> Result<impl View> {
    let photo_filter = SearchFilter {
        before: Some(1727827200),
        after: None,
        rating: None,
    };

    Ok(view! {
        <div class="contents">
            layout_content(
                <div class="prose font-mono">
                    markdown(content: include_str!("content/photo.md"))
                </div>
                filtered_photo_grid(f: photo_filter, random: true)
            )
        </div>
    })
}

#[query_params]
struct SearchQuery {
    #[serde(rename = "b")]
    before: Option<u64>,
    #[serde(rename = "a")]
    after: Option<u64>,
    #[serde(rename = "r")]
    rating: Option<u8>,
}

#[page("/search")]
async fn search_page(cx: &Cx) -> Result<impl View> {
    let filter = query_params::<SearchQuery>(cx)
        .map(|query| SearchFilter {
            before: query.before,
            after: query.after,
            rating: query.rating,
        })
        .unwrap_or_default();

    Ok(view! {
        <aside class="contents">
            layout_content(
                filtered_photo_grid(f: filter, random: false)
            )
        </aside>
    })
}

#[component]
async fn filtered_photo_grid(cx: &Cx, f: SearchFilter, random: bool) -> Result<impl View> {
    let data: &Arc<[PhotoLayoutData]> = context(cx);
    let mut grid = data
        .iter()
        .filter(|photo| f.matches(photo))
        .cloned()
        .collect::<Vec<_>>();

    if random {
        grid.shuffle(&mut rand::thread_rng());
    }

    Ok(view! { photo_grid_component(data: grid) })
}

#[component]
async fn github_icon() -> Result<impl View> {
    Ok(view! {
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
            ></path>
        </svg>
    })
}

#[component]
async fn mail_icon() -> Result<impl View> {
    Ok(view! {
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
            ></path>
        </svg>
    })
}
