use wasm_bindgen::JsValue;

pub trait LogJsError<T, U> {
    fn log_and_consume(self) -> Result<T, ()>;
}

impl<T> LogJsError<T, JsValue> for Result<T, JsValue> {
    fn log_and_consume(self) -> Result<T, ()> {
        self.map_err(|e| {
            gloo::console::log!(e);
        })
    }
}
