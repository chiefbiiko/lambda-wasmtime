use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Request {
    input: i32,
}

#[derive(Debug, Serialize)]
struct Response {
    output: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // The runtime logging can be enabled here by initializing `tracing` with `tracing-subscriber`
    // While `tracing` is used internally, `log` can be used as well if preferred.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        .init();
    let lambda = lambda_runtime::run(service_fn(my_handler));
    if let Err(err) = lambda.await {
        tracing::error!("lambda error: {:?}", err);
    }
    Ok(())
}

pub(crate) async fn my_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    println!("{:?}", event);
    // extract some useful info from the request
    let input = event.payload.input;

    // prepare the response
    let resp = Response {
        output: input,
    };
    println!("{:?}", resp);

    // return `Response` (it will be serialized to JSON automatically by the runtime)
    Ok(resp)
}
