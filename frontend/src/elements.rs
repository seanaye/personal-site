use std::cell::Cell;

use gloo::events::EventListener;
use liquid::{
    color::{
        canvas_background_style, palette_css_properties, palette_for_slider, parse_slider_value,
        rgb_css, slider_value_from_cookie, HUE_CHANGE_EVENT, SLIDER_COOKIE_NAME,
    },
    palette::PolineManager,
};
use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen, JsCast, JsValue};
use web_sys::{HtmlDocument, HtmlElement, HtmlInputElement};

use crate::liquid_canvas::LiquidCanvas;

#[wasm_bindgen(module = "/src/elements.js")]
extern "C" {
    #[wasm_bindgen(js_name = defineElements)]
    fn define_elements(
        create_liquid: JsValue,
        create_background: JsValue,
        create_palette: JsValue,
        create_hue: JsValue,
    );
}

pub fn register() {
    let create_liquid = Closure::<dyn Fn(HtmlElement) -> LiquidCanvas>::new(LiquidCanvas::new);
    let create_background =
        Closure::<dyn Fn(HtmlElement) -> PolineBackground>::new(PolineBackground::new);
    let create_palette = Closure::<dyn Fn(HtmlElement) -> PolinePalette>::new(PolinePalette::new);
    let create_hue = Closure::<dyn Fn(HtmlElement) -> HueSlider>::new(HueSlider::new);

    define_elements(
        create_liquid.into_js_value(),
        create_background.into_js_value(),
        create_palette.into_js_value(),
        create_hue.into_js_value(),
    );
}

thread_local! {
    // Keep interactions working even when the browser blocks persistence.
    static CURRENT_SLIDER_VALUE: Cell<Option<f64>> = const { Cell::new(None) };
}

fn html_document() -> Option<HtmlDocument> {
    web_sys::window()?
        .document()?
        .dyn_into::<HtmlDocument>()
        .ok()
}

pub(crate) fn current_slider_value() -> Option<f64> {
    CURRENT_SLIDER_VALUE.get().or_else(|| {
        html_document()
            .and_then(|document| document.cookie().ok())
            .and_then(|cookies| slider_value_from_cookie(&cookies))
    })
}

fn dispatch_hue_change() {
    let Some(window) = web_sys::window() else {
        return;
    };
    if let Ok(event) = web_sys::Event::new(HUE_CHANGE_EVENT) {
        _ = window.dispatch_event(&event);
    }
}

fn store_slider_value(value: f64) {
    CURRENT_SLIDER_VALUE.set(Some(value));
    if let Some(document) = html_document() {
        let secure = web_sys::window()
            .and_then(|window| window.location().protocol().ok())
            .is_some_and(|protocol| protocol == "https:");
        let secure = if secure { "; Secure" } else { "" };
        // A non-sensitive preference, intentionally readable/writable by JS.
        _ = document.set_cookie(&format!(
            "{SLIDER_COOKIE_NAME}={value}; Path=/; Max-Age=31536000; SameSite=Lax{secure}"
        ));
    }
    dispatch_hue_change();
}

fn apply_document_palette(slider_value: f64) {
    let palette = palette_for_slider(slider_value);

    let Some(root) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
        .and_then(|root| root.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };

    let style = root.style();
    for (name, value) in palette_css_properties(&palette) {
        _ = style.set_property(name, &value);
    }
}

#[wasm_bindgen]
pub struct HueSlider {
    element: HtmlElement,
    listeners: Vec<EventListener>,
}

impl HueSlider {
    fn new(element: HtmlElement) -> Self {
        Self {
            element,
            listeners: Vec::new(),
        }
    }
}

#[wasm_bindgen]
impl HueSlider {
    pub fn connected(&mut self) {
        if !self.listeners.is_empty() {
            return;
        }

        let Some(input) = self
            .element
            .query_selector("input")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        else {
            log::error!("hue-slider requires an input element");
            return;
        };

        let slider_value = current_slider_value()
            .or_else(|| parse_slider_value(&input.value()))
            .unwrap_or(0.0);
        input.set_value(&slider_value.to_string());
        apply_document_palette(slider_value);

        let listener = EventListener::new(&input, "input", move |event| {
            let Some(input) = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(value) = parse_slider_value(&input.value()) else {
                return;
            };
            apply_document_palette(value);
            store_slider_value(value);
        });
        self.listeners.push(listener);

        // Persist the raw slider position and synchronize the other elements.
        store_slider_value(slider_value);
    }

    pub fn disconnected(&mut self) {
        self.listeners.clear();
    }
}

#[wasm_bindgen]
pub struct PolineBackground {
    element: HtmlElement,
    listeners: Vec<EventListener>,
}

impl PolineBackground {
    fn new(element: HtmlElement) -> Self {
        Self {
            element,
            listeners: Vec::new(),
        }
    }

    fn apply(&self, slider_value: f64) {
        self.element
            .set_attribute(
                "style",
                &canvas_background_style(&palette_for_slider(slider_value)),
            )
            .unwrap_or_else(|error| log::error!("failed to update liquid background: {error:?}"));
    }
}

#[wasm_bindgen]
impl PolineBackground {
    pub fn connected(&mut self) {
        if !self.listeners.is_empty() {
            return;
        }

        self.apply(current_slider_value().unwrap_or(0.0));
        let element = self.element.clone();
        let listener = EventListener::new(
            &web_sys::window().expect("browser window"),
            HUE_CHANGE_EVENT,
            move |_| {
                element
                    .set_attribute(
                        "style",
                        &canvas_background_style(&palette_for_slider(
                            current_slider_value().unwrap_or(0.0),
                        )),
                    )
                    .unwrap_or_else(|error| {
                        log::error!("failed to update liquid background: {error:?}")
                    });
            },
        );
        self.listeners.push(listener);
    }

    pub fn disconnected(&mut self) {
        self.listeners.clear();
    }
}

#[wasm_bindgen]
pub struct PolinePalette {
    element: HtmlElement,
    listeners: Vec<EventListener>,
}

impl PolinePalette {
    fn new(element: HtmlElement) -> Self {
        Self {
            element,
            listeners: Vec::new(),
        }
    }

    fn apply(&self, slider_value: f64) {
        let palette = palette_for_slider(slider_value);
        let children = self.element.children();
        for (index, [r, g, b]) in palette.colors().iter().copied().enumerate() {
            let Some(child) = children.item(index as u32) else {
                break;
            };
            child
                .set_attribute(
                    "style",
                    &format!("background-color: {};", rgb_css([r, g, b])),
                )
                .unwrap_or_else(|error| log::error!("failed to update palette: {error:?}"));
        }
    }
}

#[wasm_bindgen]
impl PolinePalette {
    pub fn connected(&mut self) {
        if !self.listeners.is_empty() {
            return;
        }

        self.apply(current_slider_value().unwrap_or(0.0));
        let element = self.element.clone();
        let listener = EventListener::new(
            &web_sys::window().expect("browser window"),
            HUE_CHANGE_EVENT,
            move |_| {
                let palette = palette_for_slider(current_slider_value().unwrap_or(0.0));
                let children = element.children();
                for (index, [r, g, b]) in palette.colors().iter().copied().enumerate() {
                    let Some(child) = children.item(index as u32) else {
                        break;
                    };
                    child
                        .set_attribute(
                            "style",
                            &format!("background-color: {};", rgb_css([r, g, b])),
                        )
                        .unwrap_or_else(|error| log::error!("failed to update palette: {error:?}"));
                }
            },
        );
        self.listeners.push(listener);
    }

    pub fn disconnected(&mut self) {
        self.listeners.clear();
    }
}
