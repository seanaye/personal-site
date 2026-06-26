use grid::Coord;
use leptos::{html, prelude::*};
use leptos_router::components::A;
use num_traits::FromPrimitive;
use std::time::Duration;
use wasm_bindgen::prelude::*;

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
use crate::wgpu_renderer::WgpuLiquidRenderer;
use crate::{
    canvas_grid::{Event, EventState, PolineManager, PolineManagerImpl},
    hooks::{use_elem_size, UseWindowSizeReturn},
};

const HUE_STORAGE_KEY: &str = "liquid-hue-v2";
const HUE_CHANGE_EVENT: &str = "liquid-hue-change";
const POINTER_MOVE_DROP_STRIDE: u8 = 4;
const DEFAULT_HUE_OFFSET_DEGREES: f64 = 171.0;

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn stored_hue_value() -> Option<f64> {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(HUE_STORAGE_KEY).ok().flatten())
        .and_then(|value| value.parse().ok())
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
fn stored_hue_value() -> Option<f64> {
    None
}

#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
fn store_hue_value(value: f64) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            _ = storage.set_item(HUE_STORAGE_KEY, &value.to_string());
        }
        if let Ok(event) = web_sys::Event::new(HUE_CHANGE_EVENT) {
            _ = window.dispatch_event(&event);
        }
    }
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
fn store_hue_value(_value: f64) {}

#[island]
pub fn Slider() -> impl IntoView {
    let slider_update = use_slider_hue();
    let input_ref: NodeRef<html::Input> = NodeRef::new();

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    request_animation_frame(move || {
        // Browsers can restore range input values across reloads independently
        // of Leptos state. Once the input is mounted, treat its DOM value as
        // authoritative and sync the shared palette state to it.
        if let Some(input) = input_ref.get_untracked() {
            let state_value = slider_update.hue_value.get_untracked();
            if stored_hue_value().is_none() {
                input.set_value(&state_value.to_string());
                return;
            }

            if let Ok(value) = input.value().parse::<f64>() {
                if value != state_value {
                    store_hue_value(value);
                    slider_update.set_hue_value.set(value);
                }
            }
        }
    });

    view! {
        <input
            node_ref=input_ref
            type="range"
            min=0
            max=360
            autocomplete="off"
            class="w-24 appearance-none bg-transparent accent-current [color:inherit] [&::-moz-range-thumb]:h-4 [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:bg-current [&::-moz-range-track]:h-1 [&::-moz-range-track]:rounded-full [&::-moz-range-track]:bg-current [&::-webkit-slider-runnable-track]:h-1 [&::-webkit-slider-runnable-track]:rounded-full [&::-webkit-slider-runnable-track]:bg-current [&::-webkit-slider-thumb]:-mt-1.5 [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:[-webkit-appearance:none] [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-current sm:w-40"
            prop:value=move || slider_update.hue_value.get()
            on:input=move |ev| {
                let v: f64 = event_target_value(&ev).parse().unwrap();
                store_hue_value(v);
                slider_update.set_hue_value.set(v)
            }
        />
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SliderHue {
    pub hue_value: ReadSignal<f64>,
    pub set_hue_value: WriteSignal<f64>,
    pub poline: Memo<PolineManagerImpl>,
}

pub fn use_provide_slider_hue() -> SliderHue {
    // Start from the SSR value, then apply the browser-persisted value after
    // hydration. If we initialize directly from localStorage, hydrated DOM like
    // DebugPoline's inline styles can keep the server-rendered palette until a
    // later signal change.
    let (hue_value, set_hue_value) = signal(0.0);

    Effect::new(move |_| {
        if let Some(value) = stored_hue_value() {
            set_hue_value.set(value);
        }
    });

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        if let Some(window) = web_sys::window() {
            let listener = gloo::events::EventListener::new(&window, HUE_CHANGE_EVENT, move |_| {
                if let Some(value) = stored_hue_value() {
                    set_hue_value.set(value);
                }
            });
            listener.forget();
        }
    }

    let colours = Memo::new_owning(move |last| {
        let value = hue_value.get() + DEFAULT_HUE_OFFSET_DEGREES;
        let Some(mut last) = last else {
            return (PolineManagerImpl::new(value), true);
        };

        let prev = *last.abs_hue();
        last.set_hue(value);
        let neq = &prev != last.abs_hue();
        (last, neq)
    });

    let slider_hue = SliderHue {
        hue_value,
        set_hue_value,
        poline: colours,
    };

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    Effect::new(move |_| {
        let text_color = colours.with(|p| readable_palette_color(p.colors()));
        let shadow_color = if relative_luminance(text_color) > 0.5 {
            "rgba(0, 0, 0, 0.55)"
        } else {
            "rgba(255, 255, 255, 0.55)"
        };

        if let Some(root) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.document_element())
            .and_then(|root| root.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let style = root.style();
            _ = style.set_property("--poline-text-color", &rgb_css(text_color));
            _ = style.set_property("--poline-text-shadow", shadow_color);
        }
    });

    provide_context(slider_hue);

    slider_hue
}

pub fn expect_slider_hue() -> SliderHue {
    expect_context()
}

pub fn use_slider_hue() -> SliderHue {
    use_context().unwrap_or_else(use_provide_slider_hue)
}

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let channel = channel as f64 / 255.0;
    if channel <= 0.04045 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance([r, g, b]: [u8; 3]) -> f64 {
    0.2126 * srgb_channel_to_linear(r)
        + 0.7152 * srgb_channel_to_linear(g)
        + 0.0722 * srgb_channel_to_linear(b)
}

fn contrast_ratio(a: [u8; 3], b: [u8; 3]) -> f64 {
    let a = relative_luminance(a);
    let b = relative_luminance(b);
    let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };

    (lighter + 0.05) / (darker + 0.05)
}

fn readable_palette_color(colors: &[[u8; 3]]) -> [u8; 3] {
    let Some(background) = colors.get(colors.len() / 2).copied() else {
        return [255, 255, 255];
    };

    colors
        .iter()
        .copied()
        .max_by(|a, b| {
            contrast_ratio(*a, background)
                .partial_cmp(&contrast_ratio(*b, background))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or([255, 255, 255])
}

fn rgb_css([r, g, b]: [u8; 3]) -> String {
    format!("rgb({r}, {g}, {b})")
}

fn poline_text_style() -> &'static str {
    "color: var(--poline-text-color, white); accent-color: var(--poline-text-color, white); text-shadow: 0 1px 2px var(--poline-text-shadow, rgba(0, 0, 0, 0.55));"
}

#[component]
pub fn PolineText(children: Children) -> impl IntoView {
    view! { <div style=poline_text_style()>{children()}</div> }
}

#[component]
pub fn NavBar() -> impl IntoView {
    view! {
        <nav
            aria-label="Primary"
            class="absolute inset-x-0 top-0 z-50 flex flex-col items-start gap-2 py-4 pr-4 pl-6 font-mono text-sm lowercase tracking-[0.2em] sm:flex-row sm:items-center sm:justify-between sm:gap-8 sm:py-6 sm:pr-6 sm:pl-24"
            style=poline_text_style()
        >
            <div class="flex gap-5 sm:gap-8">
                <A href="/" {..} class="transition-opacity hover:opacity-75">"home"</A>
                <A href="/blog" {..} class="transition-opacity hover:opacity-75">"blog"</A>
                <A href="/photo" {..} class="transition-opacity hover:opacity-75">"photography"</A>
            </div>
            <Slider />
        </nav>
    }
}

#[island]
pub fn SliderProvider(children: Children) -> impl IntoView {
    use_provide_slider_hue();
    children()
}

#[component]
pub fn Canvas(children: Children) -> impl IntoView {
    view! {
        <div class="relative min-h-lvh overflow-hidden">
            <CanvasBackground />
            {children()}
        </div>
    }
}

#[island]
fn CanvasBackground() -> impl IntoView {
    let outer_size: NodeRef<html::Div> = NodeRef::new();
    let canvas_ref: NodeRef<html::Canvas> = NodeRef::new();

    let UseWindowSizeReturn { width, height } = use_elem_size(outer_size);

    let (events, set_events) = signal(EventState::default());
    let (pointer_move_count, set_pointer_move_count) = signal(0u8);

    let clear_events = move || set_events.update(|ev| ev.clear_events());

    let (document_hidden, set_document_hidden) = signal(false);

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            set_document_hidden.set(document.hidden());

            let listener =
                gloo::events::EventListener::new(&document, "visibilitychange", move |_| {
                    let hidden = web_sys::window()
                        .and_then(|window| window.document())
                        .is_some_and(|document| document.hidden());

                    set_document_hidden.set(hidden);
                    if hidden {
                        set_events.update(|ev| ev.clear_events());
                    }
                });
            listener.forget();
        }

        if let Some(window) = web_sys::window() {
            let pointer_move_listener =
                gloo::events::EventListener::new(&window, "pointermove", move |ev| {
                    let should_add_drop = pointer_move_count
                        .with_untracked(|count| count.wrapping_add(1) >= POINTER_MOVE_DROP_STRIDE);
                    if should_add_drop {
                        set_pointer_move_count.set(0);
                    } else {
                        set_pointer_move_count.update(|count| *count = count.wrapping_add(1));
                        return;
                    }

                    let Some(ev) = ev.dyn_ref::<web_sys::MouseEvent>() else {
                        return;
                    };
                    let x = ev.client_x();
                    let y = ev.client_y();
                    if x < 0
                        || y < 0
                        || x as f64 >= width.get_untracked()
                        || y as f64 >= height.get_untracked()
                    {
                        return;
                    }

                    set_events.update(move |v| {
                        v.add_event(Event::AddDrop {
                            coord: Coord {
                                x: x as usize,
                                y: y as usize,
                            },
                        })
                    });
                });
            pointer_move_listener.forget();

            let click_listener = gloo::events::EventListener::new(&window, "click", move |ev| {
                let Some(ev) = ev.dyn_ref::<web_sys::MouseEvent>() else {
                    return;
                };
                let x = ev.client_x();
                let y = ev.client_y();
                if x < 0
                    || y < 0
                    || x as f64 >= width.get_untracked()
                    || y as f64 >= height.get_untracked()
                {
                    return;
                }

                set_events.update(move |v| {
                    v.add_event(Event::AddDrop {
                        coord: Coord {
                            x: x as usize,
                            y: y as usize,
                        },
                    })
                });
            });
            click_listener.forget();
        }
    }

    let (restart_count, set_restart_count) = signal(0u64);

    let SliderHue { poline, .. } = use_slider_hue();

    Effect::new(move |previous_size: Option<(f64, f64)>| {
        let size = (width.get(), height.get());
        let has_size = size.0 > 0.0 && size.1 > 0.0;

        if has_size {
            match previous_size {
                None => set_restart_count.update(|c| *c = c.wrapping_add(1)),
                Some((previous_width, previous_height))
                    if previous_width == 0.0 || previous_height == 0.0 =>
                {
                    set_restart_count.update(|c| *c = c.wrapping_add(1));
                }
                Some(previous_size) if previous_size != size => {
                    set_events.update(|e| e.cancel());
                }
                _ => {}
            }
        }

        size
    });

    let on_cancel = move || {
        log::info!("aborted compute events");
        set_events.update(|e| e.reset_cancel_state());
        set_restart_count.update(|c| {
            *c = c.wrapping_add(1);
        });
    };

    let (random_drop_tick, set_random_drop_tick) = signal(0u64);

    Effect::new(move |val: Option<Result<TimeoutHandle, JsValue>>| {
        random_drop_tick.track();
        let hidden = document_hidden.get();
        if let Some(Ok(timeout)) = &val {
            timeout.clear();
        }

        let w = width.get();
        let h = height.get();
        let dots_width = usize::from_f64(w).unwrap_or(0);
        let dots_height = usize::from_f64(h).unwrap_or(0);

        if hidden || dots_width == 0 || dots_height == 0 {
            return Err(JsValue::NULL);
        }

        set_timeout_with_handle(
            move || {
                let f_x: f64 = rand::random();
                let f_y: f64 = rand::random();
                set_events.update(move |c| {
                    c.add_event(Event::AddDrop {
                        coord: Coord {
                            x: usize::from_f64(f_x * f64::from_usize(dots_width).unwrap()).unwrap(),
                            y: usize::from_f64(f_y * f64::from_usize(dots_height).unwrap())
                                .unwrap(),
                        },
                    })
                });
                set_random_drop_tick.update(|tick| *tick = tick.wrapping_add(1));
            },
            Duration::from_millis(1000),
        )
    });

    Effect::new(move |_| {
        if restart_count.get() == 0 {
            return;
        }

        set_events.set(EventState::default());

        let w = width.get_untracked();
        let h = height.get_untracked();
        let dots_width = usize::from_f64(w).unwrap_or(0);
        let dots_height = usize::from_f64(h).unwrap_or(0);

        if dots_width != 0 && dots_height != 0 {
            #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
            {
                let canvas_el = canvas_ref.get_untracked().expect("Canvas not loaded");
                let canvas_html: web_sys::HtmlCanvasElement = canvas_el.into();

                wasm_bindgen_futures::spawn_local(async move {
                    let renderer = WgpuLiquidRenderer::new(
                        canvas_html,
                        dots_width as u32,
                        dots_height as u32,
                        events,
                        clear_events,
                        poline,
                    )
                    .await;

                    let Some(renderer) = renderer else {
                        log::error!("Failed to initialize WebGPU renderer");
                        return;
                    };

                    fn animation_loop<T: Fn() + 'static>(
                        mut renderer: WgpuLiquidRenderer<T>,
                        on_cancel: impl Fn() + 'static,
                    ) {
                        request_animation_frame(move || match renderer.draw() {
                            Ok(()) => animation_loop(renderer, on_cancel),
                            Err(()) => {
                                on_cancel();
                            }
                        });
                    }

                    animation_loop(renderer, on_cancel);
                });
            }
        }
    });

    view! {
        <div node_ref=outer_size class="absolute inset-0 h-lvh w-lvw">
            <canvas
                node_ref=canvas_ref
                width=move || width.get()
                height=move || height.get()
                class="absolute inset-0 h-full w-full"
            />
        </div>
    }
}

#[island]
pub fn DebugPoline() -> impl IntoView {
    let SliderHue { poline, .. } = use_slider_hue();
    let (hydrated, set_hydrated) = signal(false);

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    request_animation_frame(move || set_hydrated.set(true));

    view! {
        <div class="pointer-events-none fixed left-0 top-0 z-40 h-lvh flex flex-wrap flex-col">
            {move || {
                // Force one post-hydration rerender. The server renders this palette
                // with hue 0, but the browser may restore/persist another hue before
                // this island hydrates, leaving inline styles stale unless we patch
                // them after hydration.
                hydrated.track();
                poline
                    .with(|p| {
                        p.colors()
                            .iter()
                            .map(|[r, g, b]| {
                                let style = format!("background-color: rgb({r}, {g}, {b});");
                                view! { <div style=style class="w-2 h-2 md:w-4 md:h-4"></div> }
                            })
                            .collect_view()
                    })
            }}

        </div>
    }
}
