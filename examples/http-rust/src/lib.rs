use std::{env, thread::sleep, time::Duration};

use bytes::Bytes;
use http::{request::Builder, Method};
use serde_json::{from_str as from_json, Value};
use wasi_experimental_http::request;

use lambda::{Context, Error, Event, Output};
wit_bindgen_rust::export!("../../lambda.wit");

struct Lambda {}

impl lambda::Lambda for Lambda {
    fn handler(event: Event, context: Option<Context>) -> Result<Output, Error> {
        let event_json = from_json::<Value>(event.as_str()).unwrap();
        println!("Event payload: {:?}", event_json);
        let context_json = from_json::<Value>(context.unwrap().as_str()).unwrap();
        println!("Execution context: {:?}", context_json);

        let future = async move {
            println!("future starting...");
            work().await;
            sleep(Duration::from_millis(1000));

            let task_root = env::vars()
                .find_map(|(key, value)| {
                    if key == "LAMBDA_TASK_ROOT" {
                        Some(value)
                    } else {
                        None
                    }
                })
                .unwrap();
            println!("Task Root {}", task_root);
            let task_dir = std::fs::read_dir("/var/task").unwrap();
            for child in task_dir {
                println!(
                    "/var/task/{}",
                    child.unwrap().file_name().into_string().unwrap()
                );
            }
            let temp_dir = std::fs::read_dir("/tmp").unwrap();
            for child in temp_dir {
                println!("/tmp/{}", child.unwrap().file_name().into_string().unwrap());
            }

            println!("making http request...");
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
            println!(
                "Content type: {:#?}",
                res.header_get("content-type".to_string()).unwrap()
            );
            let status_code = res.status_code;
            println!("Status code: {:#?}", status_code);
            Ok(str)
        };
        let res: Result<Output, Error> = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future);
        println!("String response: {:?}", res);

        res
    }
}

async fn work() {
    sleep(Duration::from_millis(100));
    println!("work thread finished");
}
