#[cfg(target_arch = "wasm32")]
mod elements;
#[cfg(target_arch = "wasm32")]
mod liquid_canvas;
#[cfg(target_arch = "wasm32")]
mod wgpu_renderer;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Initialize the WASM module and upgrade the server-rendered custom elements.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    // Keep the browser console quiet; wgpu emits verbose shader diagnostics.
    _ = console_log::init_with_level(log::Level::Warn);
    console_error_panic_hook::set_once();
    elements::register();
}
