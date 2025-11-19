use jsoncodegen::{codegen, schema};
use serde_json::Value;
use std::io::Cursor;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub enum Lang {
    Java,
    Rust,
}

#[wasm_bindgen]
pub fn codegen(json: &str, lang: Lang) -> Result<String, JsValue> {
    let json: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let schema = schema::Schema::from(json);

    let mut out = Cursor::new(Vec::new());
    match lang {
        Lang::Java => codegen::java(schema, &mut out).map_err(|e| e.to_string())?,
        Lang::Rust => codegen::rust(schema, &mut out).map_err(|e| e.to_string())?,
    }
    let code = String::from_utf8(out.into_inner()).map_err(|e| e.to_string())?;

    Ok(code)
}
