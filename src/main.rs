use std::env;

use anyhow::Error;
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_experimental_http_wasmtime::HttpCtx;
use wasmtime::{Engine, Linker, Store};
use wasmtime_wasi::*;

wit_bindgen_wasmtime::import!("./handler.wit");
use handler::*;

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

    let handler_func = move |event: LambdaEvent<Value>| async move {
        tracing::info!("{:?}", event);
        let mut task_root = "".to_string();
        let mut handler_file = "".to_string();
        let mut allowed_hosts: Vec<String> = Vec::new();
        let mut max_concurrency: Option<u32> = None;
        for (key, value) in env::vars() {
            if key == "_HANDLER" {
                handler_file = value
            } else if key == "ALLOWED_HOSTS" {
                allowed_hosts = value
                    .split(',')
                    .into_iter()
                    .map(|x| x.to_string())
                    .collect();
            } else if key == "LAMBDA_TASK_ROOT" {
                task_root = value;
            } else if key == "MAX_CONCURRENCY" {
                max_concurrency = Some(value.parse().unwrap())
            }
        }
        let filename = format!("{}/{}.wasm", task_root, handler_file);
        let (plugin, store) = create_instance(filename, Some(allowed_hosts), max_concurrency)?;

        // extract some useful info from the request
        let input = event.payload;

        let resp = match plugin
            .handler(store, serde_json::to_string(&input).unwrap().as_str(), None)
            .expect("runtime failed to retrieve handler")
        {
            Ok(output) => serde_json::from_str::<Value>(output.as_str()).unwrap(),
            Err(_error) => serde_json::json!("error"),
        };
        println!("{:?}", resp);

        // return `Response` (it will be serialized to JSON automatically by the runtime)
        Result::<Value, Error>::Ok(resp)
    };

    let lambda = lambda_runtime::run(service_fn(handler_func));
    if let Err(err) = lambda.await {
        tracing::error!("lambda error: {:?}", err);
    }
    Ok(())
}

fn create_instance(
    filename: String,
    allowed_hosts: Option<Vec<String>>,
    max_concurrent_requests: Option<u32>,
) -> Result<(Handler<Context>, Store<Context>), Error> {
    tracing::info!("create_instance 1");
    let mut wasmtime_config = wasmtime::Config::default();
    wasmtime_config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    wasmtime_config.wasm_multi_memory(true);
    wasmtime_config.wasm_module_linking(true);
    wasmtime_config.wasm_simd(true);
    let engine = Engine::new(&wasmtime_config)?;
    let mut linker = Linker::new(&engine);

    let ctx = Context {
        wasi: default_wasi()?,
        http: HttpCtx::new(allowed_hosts, max_concurrent_requests)?,
        runtime_data: Some(handler::HandlerData {}),
    };

    wasmtime_wasi::add_to_linker(&mut linker, |cx: &mut Context| &mut cx.wasi)?;

    // Link `wasi_experimental_http`
    ctx.http.add_to_linker(&mut linker)?;

    tracing::info!("create_instance 2");

    let mut store = Store::new(&engine, ctx);

    let module = wasmtime::Module::from_file(store.engine(), filename)?;

    let (plugin, _instance) = Handler::instantiate(&mut store, &module, &mut linker, |ctx| {
        ctx.runtime_data.as_mut().unwrap()
    })?;

    tracing::info!("create_instance 3");

    Ok((plugin, store))
}

struct Context {
    pub wasi: WasiCtx,
    pub http: HttpCtx,
    pub runtime_data: Option<handler::HandlerData>,
}

fn default_wasi() -> Result<WasiCtx, Error> {
    let ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .inherit_env()?;
    Ok(ctx.build())
}
