use wasm_bindgen::JsValue;

pub trait LogJsError<T, U> {
    fn log_and_consume(self) -> Result<T, ()>;
}

impl<T, U> LogJsError<T, U> for Result<T, U>
where
    U: std::fmt::Debug,
{
    fn log_and_consume(self) -> Result<T, ()> {
        self.map_err(|e| {
            leptos::logging::log!("{e:?}");
        })
    }
}
