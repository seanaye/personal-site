use grid::Coord;
use leptos::{html, prelude::*};
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
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        _ = storage.set_item(HUE_STORAGE_KEY, &value.to_string());
    }
}

#[cfg(not(all(feature = "hydrate", target_arch = "wasm32")))]
fn store_hue_value(_value: f64) {}

#[island]
pub fn Slider() -> impl IntoView {
    let slider_update: SliderHue = expect_context();
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
            class="accent-stone-700"
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
    provide_context(slider_hue);

    slider_hue
}

pub fn expect_slider_hue() -> SliderHue {
    expect_context()
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

#[island]
pub fn NavBar() -> impl IntoView {
    let SliderHue { poline, .. } = expect_slider_hue();

    let style = move || {
        let text_color = poline.with(|p| readable_palette_color(p.colors()));
        let shadow_color = if relative_luminance(text_color) > 0.5 {
            "rgba(0, 0, 0, 0.55)"
        } else {
            "rgba(255, 255, 255, 0.55)"
        };

        format!(
            "color: {}; text-shadow: 0 1px 2px {};",
            rgb_css(text_color),
            shadow_color
        )
    };

    view! {
        <nav
            aria-label="Primary"
            class="fixed left-0 top-0 z-50 flex gap-5 py-4 pl-24 font-mono text-sm lowercase tracking-[0.2em] sm:gap-8 sm:py-6"
            style=style
        >
            <a class="transition-opacity hover:opacity-75" href="/">"home"</a>
            <a class="transition-opacity hover:opacity-75" href="/blog">"blog"</a>
            <a class="transition-opacity hover:opacity-75" href="/#photography">"photography"</a>
        </nav>
    }
}

#[island]
pub fn SliderProvider(children: Children) -> impl IntoView {
    use_provide_slider_hue();
    children()
}

#[island]
pub fn Canvas(children: Children) -> impl IntoView {
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
    }

    let (cancel_count, set_cancel_count) = signal(0);

    let SliderHue { poline, .. } = expect_slider_hue();

    Effect::new(move |val: Option<bool>| {
        width.with(|_| {});
        height.with(|_| {});
        if val.is_some() {
            set_events.update(|e| e.cancel())
        }
        true
    });

    let on_cancel = move || {
        log::info!("aborted compute events");
        set_cancel_count.update(|c| {
            *c += 1;
        });
        set_events.update(|e| e.reset_cancel_state());
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
        cancel_count.read();
        set_events.set(EventState::default());

        let w = width.get();
        let h = height.get();
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
        <div
            node_ref=outer_size
            class="relative h-lvh w-lvw"
            on:pointermove=move |ev| {
                let should_add_drop = pointer_move_count.with_untracked(|count| {
                    count.wrapping_add(1) >= POINTER_MOVE_DROP_STRIDE
                });
                if should_add_drop {
                    set_pointer_move_count.set(0);
                } else {
                    set_pointer_move_count.update(|count| *count = count.wrapping_add(1));
                    return;
                }

                let e = Event::AddDrop {
                    coord: Coord {
                        x: ev.page_x() as usize,
                        y: ev.page_y() as usize,
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
            on:click=move |ev| {
                let e = Event::AddDrop {
                    coord: Coord {
                        x: ev.page_x() as usize,
                        y: ev.page_y() as usize,
                    },
                };
                set_events.update(move |v| v.add_event(e));
            }
        >
            <canvas
                node_ref=canvas_ref
                width=move || width.get()
                height=move || height.get()
                class="absolute inset-0"
            />
            {children()}
        </div>
    }
}

#[island]
pub fn DebugPoline() -> impl IntoView {
    let SliderHue { poline, .. } = expect_slider_hue();
    let (hydrated, set_hydrated) = signal(false);

    #[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
    request_animation_frame(move || set_hydrated.set(true));

    view! {
        <div class="pointer-events-none absolute left-0 top-0 h-lvh flex flex-wrap flex-col">
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
