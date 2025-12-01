use std::io;

use jsoncodegen_java::codegen as java;
use jsoncodegen_rust::codegen as rust;

pub fn dispatch(
    lang: &str,
    json: serde_json::Value,
    out: &mut dyn io::Write,
) -> Result<bool, io::Error> {
    match lang {
        "java" => java(json, out)?,
        "rust" => rust(json, out)?,
        _ => return Ok(false),
    };

    Ok(true)
}
