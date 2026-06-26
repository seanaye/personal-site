// this import is required to make islands work
#[allow(clippy::single_component_path_imports, unused_imports)]
use app;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
    // Keep browser console quiet; wgpu emits verbose shader diagnostics at Debug/Info.
    _ = console_log::init_with_level(log::Level::Warn);
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_islands();
}
