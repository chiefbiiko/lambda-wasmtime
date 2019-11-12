use base64::{decode as from_base64, encode as to_base64};
use image::{load_from_memory as load_image_from_memory, DynamicImage, ImageOutputFormat::PNG};
use serde_json::{from_str as from_json, Value};

#[no_mangle]
pub extern fn handler(event: &str, _context: &str) -> String {
    let thumbnail: DynamicImage = load_image_from_memory(
        &from_base64(
            from_json::<Value>(event).unwrap()
                .get("data").unwrap()
                .as_str().unwrap(),
        ).unwrap(),
    ).unwrap()
    .thumbnail(256, 256);

    let mut thumbnail_buf: Vec<u8> = Vec::new();

    thumbnail.write_to(&mut thumbnail_buf, PNG).unwrap();

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
