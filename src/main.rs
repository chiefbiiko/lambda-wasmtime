mod engine;
use std::{env, sync::Arc};

use anyhow::{Error, Result};
use engine::{Builder, ExecutionContextConfiguration};
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use tracing::log;

wit_bindgen_wasmtime::import!("./lambda.wit");
use lambda::{Lambda, LambdaData};

type ExecutionContext = engine::ExecutionContext<LambdaData>;

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

    let builder: ExecutionContext = Builder::build_default(LambdaFunction::retrieve_config()?)?;

    let lambda = LambdaFunction::new(builder);
    let engine_ref = &lambda.engine;

    let handler_func = move |event: LambdaEvent<Value>| async move {
        log::info!("{:?}", event);
        let (mut store, instance) = engine_ref.prepare(Some(LambdaData {}))?;
        let component = Lambda::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
        let resp = match component
            .handler(
                store,
                serde_json::to_string(&event.payload).unwrap().as_str(),
                Some(serde_json::to_string(&event.context).unwrap().as_str()),
            )
            .expect("runtime failed to retrieve handler")
        {
            Ok(output) => serde_json::from_str(output.as_str()).unwrap(),
            Err(_error) => serde_json::json!("error"),
        };
        log::info!("JSON response: {:?}", resp);

        // return `Response` (it will be serialized to JSON automatically by the runtime)
        Result::<Value, Error>::Ok(resp)
    };

    let lambda = lambda_runtime::run(service_fn(handler_func));
    if let Err(err) = lambda.await {
        tracing::error!("lambda error: {:?}", err);
    }
    Ok(())
}

#[derive(Clone)]
pub(crate) struct LambdaFunction {
    /// The Lambda execution context.
    engine: Arc<ExecutionContext>,
}

impl LambdaFunction {
    pub fn retrieve_config() -> Result<ExecutionContextConfiguration> {
        let mut task_root: Option<String> = None;
        let mut handler_file: Option<String> = None;
        let mut allowed_hosts: Option<Vec<String>> = None;
        let mut max_concurrent_requests: Option<u32> = None;
        for (key, value) in env::vars() {
            if key == "_HANDLER" {
                handler_file = Some(value);
            } else if key == "ALLOWED_HOSTS" {
                allowed_hosts = Some(
                    value
                        .split(',')
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                );
            } else if key == "LAMBDA_TASK_ROOT" {
                task_root = Some(value);
            } else if key == "MAX_CONCURRENCY" {
                max_concurrent_requests = Some(value.parse().unwrap())
            }
        }
        let task_directory = task_root.unwrap();
        let source = format!("{}/{}.wasm", task_directory, handler_file.unwrap());
        let temp_directory = String::from("/tmp");

        Ok(ExecutionContextConfiguration {
            id: String::from("Lambda"),
            task_directory,
            temp_directory,
            source,
            allowed_hosts,
            max_concurrent_requests,
        })
    }

    pub fn new(execution_context: ExecutionContext) -> Self {
        Self {
            engine: Arc::new(execution_context),
        }
    }
}
