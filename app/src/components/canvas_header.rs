use liquid::{
    color::{canvas_background_style, rgb_css},
    palette::PolineManager,
};
use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

use crate::hue::{initial_hue, initial_palette};

fn poline_text_style() -> &'static str {
    "color: var(--poline-text-color, white); accent-color: var(--poline-text-color, white); text-shadow: 0 1px 2px var(--poline-text-shadow, rgba(0, 0, 0, 0.55));"
}

#[component]
pub async fn poline_text(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div style=(poline_text_style())>(child)</div> })
}

#[component]
pub async fn nav_bar(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <nav
            aria-label="Primary"
            class="absolute inset-x-0 top-0 z-50 flex flex-col items-start gap-2 py-4 pr-4 pl-6 font-mono text-sm lowercase tracking-[0.2em] sm:flex-row sm:items-center sm:justify-between sm:gap-8 sm:py-6 sm:pr-6 sm:pl-24"
            style=(poline_text_style())
        >
            <div class="flex gap-5 sm:gap-8">
                <a href="/" class="transition-opacity hover:opacity-75">"home"</a>
                <a href="/blog" class="transition-opacity hover:opacity-75">"blog"</a>
                <a href="/photo" class="transition-opacity hover:opacity-75">"photography"</a>
            </div>
            <hue-slider class="contents">
                <input
                    type="range"
                    min="0"
                    max="360"
                    value=(initial_hue(cx))
                    autocomplete="off"
                    aria-label="Liquid hue"
                    class="w-24 appearance-none bg-transparent accent-current [color:inherit] [&::-moz-range-thumb]:h-4 [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-current [&::-moz-range-track]:h-1 [&::-moz-range-track]:rounded-full [&::-moz-range-track]:bg-current [&::-webkit-slider-runnable-track]:h-1 [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-current [&::-webkit-slider-thumb]:-mt-1.5 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:[-webkit-appearance:none] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-current sm:w-40"
                >
            </hue-slider>
        </nav>
    })
}

#[component]
pub async fn static_canvas(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    let background = canvas_background_style(&initial_palette(cx));

    Ok(view! {
        <div class="relative min-h-lvh overflow-hidden">
            <poline-background class="absolute inset-0 w-full" style=(background)></poline-background>
            (child)
        </div>
    })
}

#[component]
pub async fn liquid_canvas(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div class="relative min-h-lvh overflow-hidden">
            <liquid-canvas class="absolute inset-0 w-full" style="background: var(--poline-background-color);">
                <canvas width="0" height="0" class="absolute inset-0 h-full w-full"></canvas>
            </liquid-canvas>
            (child)
        </div>
    })
}

#[component]
pub async fn debug_poline(cx: &Cx) -> Result<impl View> {
    let palette = initial_palette(cx);

    Ok(view! {
        <poline-palette class="pointer-events-none fixed left-0 top-0 z-40 h-lvh flex flex-wrap flex-col">
            for [r, g, b] in palette.colors().iter().copied() {
                <div
                    style=(format!("background-color: {};", rgb_css([r, g, b])))
                    class="w-2 h-2 md:w-4 md:h-4"
                ></div>
            }
        </poline-palette>
    })
}
