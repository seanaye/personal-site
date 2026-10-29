use std::cell::Cell;

use gloo::events::EventListener;
use liquid::{
    color::{
        canvas_background_style, hue_from_cookie, palette_css_properties, palette_for_slider,
        parse_slider_hue, rgb_css, HUE_CHANGE_EVENT, HUE_COOKIE_NAME, HUE_STORAGE_KEY,
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
    static CURRENT_HUE: Cell<Option<f64>> = const { Cell::new(None) };
}

fn html_document() -> Option<HtmlDocument> {
    web_sys::window()?
        .document()?
        .dyn_into::<HtmlDocument>()
        .ok()
}

pub(crate) fn stored_hue() -> Option<f64> {
    CURRENT_HUE.get().or_else(|| {
        // Prefer the cookie used by SSR to avoid a stale local-storage value
        // changing the palette immediately after navigation.
        html_document()
            .and_then(|document| document.cookie().ok())
            .and_then(|cookies| hue_from_cookie(&cookies))
            .or_else(|| {
                web_sys::window()
                    .and_then(|window| window.local_storage().ok().flatten())
                    .and_then(|storage| storage.get_item(HUE_STORAGE_KEY).ok().flatten())
                    .and_then(|value| parse_slider_hue(&value))
            })
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

fn store_hue(value: f64) {
    CURRENT_HUE.set(Some(value));
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        _ = storage.set_item(HUE_STORAGE_KEY, &value.to_string());
    }
    if let Some(document) = html_document() {
        let secure = web_sys::window()
            .and_then(|window| window.location().protocol().ok())
            .is_some_and(|protocol| protocol == "https:");
        let secure = if secure { "; Secure" } else { "" };
        // A non-sensitive preference, intentionally readable/writable by JS.
        _ = document.set_cookie(&format!(
            "{HUE_COOKIE_NAME}={value}; Path=/; Max-Age=31536000; SameSite=Lax{secure}"
        ));
    }
    dispatch_hue_change();
}

fn apply_document_palette(hue: f64) {
    let palette = palette_for_slider(hue);

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

        let hue = stored_hue()
            .or_else(|| parse_slider_hue(&input.value()))
            .unwrap_or(0.0);
        input.set_value(&hue.to_string());
        apply_document_palette(hue);

        let listener = EventListener::new(&input, "input", move |event| {
            let Some(input) = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
            else {
                return;
            };
            let Some(hue) = parse_slider_hue(&input.value()) else {
                return;
            };
            apply_document_palette(hue);
            store_hue(hue);
        });
        self.listeners.push(listener);

        // Migrate existing local-storage preferences and synchronize the
        // other elements. Future page requests can now render this hue.
        store_hue(hue);
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

    fn apply(&self, hue: f64) {
        self.element
            .set_attribute("style", &canvas_background_style(&palette_for_slider(hue)))
            .unwrap_or_else(|error| log::error!("failed to update liquid background: {error:?}"));
    }
}

#[wasm_bindgen]
impl PolineBackground {
    pub fn connected(&mut self) {
        if !self.listeners.is_empty() {
            return;
        }

        self.apply(stored_hue().unwrap_or(0.0));
        let element = self.element.clone();
        let listener = EventListener::new(
            &web_sys::window().expect("browser window"),
            HUE_CHANGE_EVENT,
            move |_| {
                element
                    .set_attribute(
                        "style",
                        &canvas_background_style(&palette_for_slider(stored_hue().unwrap_or(0.0))),
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

    fn apply(&self, hue: f64) {
        let palette = palette_for_slider(hue);
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

        self.apply(stored_hue().unwrap_or(0.0));
        let element = self.element.clone();
        let listener = EventListener::new(
            &web_sys::window().expect("browser window"),
            HUE_CHANGE_EVENT,
            move |_| {
                let palette = palette_for_slider(stored_hue().unwrap_or(0.0));
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
