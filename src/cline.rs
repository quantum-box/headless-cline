use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hellp_world() ->  Result<String, JsValue> {
    Ok("hello world".into())
}