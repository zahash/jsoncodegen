use jsoncodegen_dispatch::dispatch;
use serde_json::Value;
use wasm_bindgen::prelude::*;

// TODO: split this wasm bundle to multiple independent generators
// that are lazily loaded when actually required
#[wasm_bindgen]
pub fn codegen(json: &str, lang: &str) -> Result<String, JsValue> {
    let json: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut out = Vec::new();

    if !dispatch(lang, json, &mut out).map_err(|e| e.to_string())? {
        return Err(format!("`{}` language not supported", lang).into());
    }

    let code = String::from_utf8(out).map_err(|e| e.to_string())?;
    Ok(code)
}
