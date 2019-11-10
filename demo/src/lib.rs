use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn handler(event: &str, context: &str) -> String {
    
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
