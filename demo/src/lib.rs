use anyhow::{anyhow, Result as AnyhowResult};
use base64::{decode as from_base64, encode as to_base64};
use image::{load_from_memory as load_image_from_memory, ImageOutputFormat::PNG};
use serde_json::{from_str as from_json, Value};
use wasm_bindgen::prelude::wasm_bindgen;

fn process(event: &str, mut thumbnail_buf: Vec<u8>) -> AnyhowResult<Vec<u8>> {
    load_image_from_memory(&from_base64(
        from_json::<Value>(event)?
            .get("data")
            .ok_or(anyhow!("missing property \"data\""))?
            .as_str()
            .ok_or(anyhow!("invalid string"))?,
    )?)?
    .thumbnail(128, 128)
    .write_to(&mut thumbnail_buf, PNG)?;

    Ok(thumbnail_buf)
}

#[wasm_bindgen]
pub fn handler(event: &str, _context: &str) -> String {
    match process(event, Vec::new()) {
        Ok(thumbnail_buf) => format!("{{\"data\":\"{}\"}}", to_base64(&thumbnail_buf)),
        _ => "{\"error\":\"processing failed\"}".to_string(),
    }
}
