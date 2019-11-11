use base64::{decode as from_base64, encode as to_base64};
use image::{load_from_memory as load_image_from_memory, DynamicImage, ImageOutputFormat::PNG};
use serde_json::{from_str as from_json, Value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn handler(event: &str, _context: &str) -> String {
    let json: Value = match from_json::<Value>(event) {
        Ok(json) => json,
        _ => return "{}".to_string(),
    };

    let data: &Value = match json.get("data") {
        Some(data) => data,
        _ => return "{}".to_string(),
    };

    let dstr: &str = match data.as_str() {
        Some(dstr) => dstr,
        _ => return "{}".to_string(),
    };

    let dbuf: Vec<u8> = match from_base64(dstr) {
        Ok(dbuf) => dbuf,
        _ => return "{}".to_string(),
    };

    let original: DynamicImage = match load_image_from_memory(&dbuf) {
        Ok(original) => original,
        _ => return "{}".to_string(),
    };

    let thumbnail: DynamicImage = original.thumbnail(256, 256);

    // let thumbnail: DynamicImage = load_image_from_memory(
    //     &from_base64(
    //         from_json::<Value>(event).unwrap()
    //             .get("data").unwrap()
    //             .as_str().unwrap(),
    //     ).unwrap(),
    // ).unwrap()
    // .thumbnail(256, 256);

    let mut thumbnail_buf: Vec<u8> = Vec::new();

    match thumbnail.write_to(&mut thumbnail_buf, PNG) {
        Ok(()) => (),
        _ => return "{}".to_string(),
    };

    // thumbnail.write_to(&mut thumbnail_buf, PNG).unwrap();

    format!("{{\"data\":\"{}\"}}", to_base64(&thumbnail_buf))
}

#[cfg(test)]
mod tests {
    #[test]
    fn handler_ok() {
        let response = super::handler(
            "{\"data\":\"qfBzYkzC5Xq3JW0wOiN5vlxu/lWEEgrdh40ZQLPvmJ0=\"}",
            "{}",
        );

        assert!(response.contains("data"));
    }
}
