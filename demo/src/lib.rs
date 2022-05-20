use bytes::Bytes;
use http::{request::Builder, Method};
use serde_json::{from_str as from_json, Value};
use wasi_experimental_http::request;

use lambda::{Context, Error, Event, Output};
wit_bindgen_rust::export!("../lambda.wit");

struct Lambda {}
impl lambda::Lambda for Lambda {
    fn handler(event: Event, context: Option<Context>) -> Result<Output, Error> {
        let json = from_json::<Value>(event.as_str()).unwrap();
        println!("{:?} {:?}", json, context);
        let url = "https://postman-echo.com/post".to_string();
        let req = Builder::new()
            .method(Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .header("abc", "def");
        let b = Bytes::from(event);
        let req = req.body(Some(b)).unwrap();
        println!("{:?}", req);

        let mut res = request(req).expect("cannot make request");
        let str = std::str::from_utf8(&res.body_read_all().unwrap())
            .unwrap()
            .to_owned();
        println!("{:?}", str);
        println!("{:#?}", res.header_get("content-type".to_string()).unwrap());
        let status_code = res.status_code;
        println!("{:#?}", status_code);
        Ok(str)
    }
}
