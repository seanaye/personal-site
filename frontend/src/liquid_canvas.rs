use std::{cell::RefCell, rc::Rc};

use gloo::{
    events::EventListener,
    render::{request_animation_frame, AnimationFrame},
    timers::callback::Interval,
};
use grid::Coord;
use liquid::{
    color::{palette_for_slider, HUE_CHANGE_EVENT},
    palette::{Event as LiquidEvent, PolineManagerImpl},
};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast};
use web_sys::{HtmlCanvasElement, HtmlElement, MouseEvent};

use crate::{elements::stored_hue, wgpu_renderer::WgpuLiquidRenderer};

const POINTER_MOVE_DROP_STRIDE: u8 = 4;

#[wasm_bindgen]
pub struct LiquidCanvas {
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    element: HtmlElement,
    canvas: HtmlCanvasElement,
    renderer: Option<WgpuLiquidRenderer>,
    palette: PolineManagerImpl,
    events: Vec<LiquidEvent>,
    listeners: Vec<EventListener>,
    animation_frame: Option<AnimationFrame>,
    random_interval: Option<Interval>,
    generation: u64,
    pointer_move_count: u8,
    connected: bool,
    document_hidden: bool,
    width: u32,
    height: u32,
}

impl LiquidCanvas {
    pub(crate) fn new(element: HtmlElement) -> Self {
        let canvas = element
            .query_selector("canvas")
            .ok()
            .flatten()
            .and_then(|canvas| canvas.dyn_into::<HtmlCanvasElement>().ok())
            .unwrap_or_else(|| {
                let document = element.owner_document().expect("canvas owner document");
                let canvas = document
                    .create_element("canvas")
                    .expect("create liquid canvas")
                    .dyn_into::<HtmlCanvasElement>()
                    .expect("canvas element");
                canvas.set_class_name("absolute inset-0 h-full w-full");
                element.append_child(&canvas).expect("append liquid canvas");
                canvas
            });
        let document_hidden = element
            .owner_document()
            .is_some_and(|document| document.hidden());
        let palette = palette_for_slider(stored_hue().unwrap_or(0.0));

        Self {
            inner: Rc::new(RefCell::new(Inner {
                element,
                canvas,
                renderer: None,
                palette,
                events: Vec::new(),
                listeners: Vec::new(),
                animation_frame: None,
                random_interval: None,
                generation: 0,
                pointer_move_count: 0,
                connected: false,
                document_hidden,
                width: 0,
                height: 0,
            })),
        }
    }

    fn add_drop(inner: &Rc<RefCell<Inner>>, x: i32, y: i32) {
        let mut inner = inner.borrow_mut();
        if x < 0 || y < 0 || x as u32 >= inner.width || y as u32 >= inner.height {
            return;
        }
        inner.events.push(LiquidEvent::AddDrop {
            coord: Coord {
                x: x as usize,
                y: y as usize,
            },
        });
    }

    fn resize(inner: &Rc<RefCell<Inner>>) {
        let changed = {
            let mut state = inner.borrow_mut();
            let width = state.element.client_width().max(0) as u32;
            let height = state.element.client_height().max(0) as u32;
            if (width, height) == (state.width, state.height) {
                false
            } else {
                state.width = width;
                state.height = height;
                state.canvas.set_width(width);
                state.canvas.set_height(height);
                true
            }
        };

        if changed {
            Self::start_renderer(inner);
        }
    }

    fn start_renderer(inner: &Rc<RefCell<Inner>>) {
        let (canvas, width, height, palette, generation) = {
            let mut state = inner.borrow_mut();
            state.generation = state.generation.wrapping_add(1);
            state.renderer = None;
            if !state.connected || state.width == 0 || state.height == 0 {
                return;
            }
            (
                state.canvas.clone(),
                state.width,
                state.height,
                palette_for_slider(stored_hue().unwrap_or(0.0)),
                state.generation,
            )
        };

        let inner = inner.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let renderer = WgpuLiquidRenderer::new(canvas, width, height, &palette).await;
            let mut state = inner.borrow_mut();
            if state.connected && state.generation == generation {
                state.renderer = renderer;
            }
        });
    }

    fn schedule_frame(inner: &Rc<RefCell<Inner>>) {
        let callback_inner = inner.clone();
        let frame = request_animation_frame(move |_| {
            let draw_failed = {
                let mut state = callback_inner.borrow_mut();
                state.animation_frame = None;
                if !state.connected {
                    return;
                }
                if state.document_hidden {
                    state.events.clear();
                    false
                } else {
                    let events = std::mem::take(&mut state.events);
                    let Inner {
                        renderer, palette, ..
                    } = &mut *state;
                    match renderer.as_mut() {
                        Some(renderer) => renderer.draw(&events, palette).is_err(),
                        None => false,
                    }
                }
            };

            if draw_failed {
                Self::start_renderer(&callback_inner);
            }
            Self::schedule_frame(&callback_inner);
        });

        let mut state = inner.borrow_mut();
        if state.connected {
            state.animation_frame = Some(frame);
        }
    }

    fn install_listeners(inner: &Rc<RefCell<Inner>>) {
        let window = web_sys::window().expect("browser window");
        let document = window.document().expect("browser document");

        let pointer_inner = inner.clone();
        let pointer_move = EventListener::new(&window, "pointermove", move |event| {
            let should_add_drop = {
                let mut state = pointer_inner.borrow_mut();
                state.pointer_move_count = state.pointer_move_count.wrapping_add(1);
                if state.pointer_move_count >= POINTER_MOVE_DROP_STRIDE {
                    state.pointer_move_count = 0;
                    true
                } else {
                    false
                }
            };
            if !should_add_drop {
                return;
            }
            let Some(event) = event.dyn_ref::<MouseEvent>() else {
                return;
            };
            Self::add_drop(&pointer_inner, event.client_x(), event.client_y());
        });

        let click_inner = inner.clone();
        let click = EventListener::new(&window, "click", move |event| {
            let Some(event) = event.dyn_ref::<MouseEvent>() else {
                return;
            };
            Self::add_drop(&click_inner, event.client_x(), event.client_y());
        });

        let resize_inner = inner.clone();
        let resize = EventListener::new(&window, "resize", move |_| {
            Self::resize(&resize_inner);
        });

        let hue_inner = inner.clone();
        let hue = EventListener::new(&window, HUE_CHANGE_EVENT, move |_| {
            hue_inner.borrow_mut().palette = palette_for_slider(stored_hue().unwrap_or(0.0));
        });

        let visibility_inner = inner.clone();
        let visibility_document = document.clone();
        let visibility = EventListener::new(&document, "visibilitychange", move |_| {
            let hidden = visibility_document.hidden();
            let mut state = visibility_inner.borrow_mut();
            state.document_hidden = hidden;
            if hidden {
                state.events.clear();
            }
        });

        inner
            .borrow_mut()
            .listeners
            .extend([pointer_move, click, resize, hue, visibility]);
    }
}

#[wasm_bindgen]
impl LiquidCanvas {
    pub fn connected(&mut self) {
        {
            let mut state = self.inner.borrow_mut();
            if state.connected {
                return;
            }
            state.connected = true;
        }

        Self::install_listeners(&self.inner);
        Self::resize(&self.inner);
        Self::schedule_frame(&self.inner);

        let random_inner = self.inner.clone();
        let interval = Interval::new(1_000, move || {
            let (width, height, enabled) = {
                let state = random_inner.borrow();
                (
                    state.width,
                    state.height,
                    state.connected && !state.document_hidden,
                )
            };
            if !enabled || width == 0 || height == 0 {
                return;
            }

            let x = (rand::random::<f64>() * f64::from(width)) as usize;
            let y = (rand::random::<f64>() * f64::from(height)) as usize;
            random_inner.borrow_mut().events.push(LiquidEvent::AddDrop {
                coord: Coord { x, y },
            });
        });
        self.inner.borrow_mut().random_interval = Some(interval);
    }

    pub fn disconnected(&mut self) {
        let mut state = self.inner.borrow_mut();
        state.connected = false;
        state.generation = state.generation.wrapping_add(1);
        state.renderer = None;
        state.events.clear();
        state.listeners.clear();
        state.animation_frame = None;
        state.random_interval = None;
    }
}
