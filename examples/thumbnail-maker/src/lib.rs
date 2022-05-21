use anyhow::{anyhow, Result as AnyhowResult};
use base64::{decode as from_base64, encode as to_base64};
use image::{load_from_memory as load_image_from_memory, ImageOutputFormat::PNG};
use serde_json::{from_str as from_json, Value};

use lambda::{Context, Error, Event, Output};
wit_bindgen_rust::export!("../../lambda.wit");

struct Lambda {}

impl lambda::Lambda for Lambda {
    fn handler(event: Event, _context: Option<Context>) -> Result<Output, Error> {
        Ok(match process(event.as_str(), Vec::new()) {
            Ok(thumbnail_buf) => format!("{{\"data\":\"{}\"}}", to_base64(&thumbnail_buf)),
            _ => "{\"error\":\"processing failed\"}".to_string(),
        })
    }
}

fn process(event: &str, mut thumbnail_buf: Vec<u8>) -> AnyhowResult<Vec<u8>> {
    load_image_from_memory(
        &from_base64(
            from_json::<Value>(event)?
                .get("data").ok_or(anyhow!("missing property \"data\""))?
                .as_str().ok_or(anyhow!("invalid string"))?,
        )?,
    )?
    .thumbnail(128, 128)
    .write_to(&mut thumbnail_buf, PNG)?;

    Ok(thumbnail_buf)
}
