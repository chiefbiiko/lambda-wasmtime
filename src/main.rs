use std::env;

use anyhow::Error;
use lambda_runtime::{service_fn, LambdaEvent};
use serde_json::Value;
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_experimental_http_wasmtime::HttpCtx;
use wasmtime::{Linker, Store};
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

    let (filename, allowed_hosts, max_concurrency) = retrieve_config();
    
    let engine = Engine::new()?;
    let mut linker: Linker<RuntimeContext> = Linker::new(&engine.inner());
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;

    let linker_ref = &linker;
    let module_ref = &wasmtime::Module::from_file(linker.engine(), filename)?;
    let allowed_hosts_ref = &allowed_hosts;
    let max_concurrency_ref = &max_concurrency;

    let handler_func = move |event: LambdaEvent<Value>| async move {
        tracing::info!("{:?}", event);
        let mut linker = linker_ref.to_owned();
        let allowed_hosts = allowed_hosts_ref.to_owned();
        let max_concurrency = max_concurrency_ref.to_owned();

        let engine = linker_ref.engine();

        let ctx = RuntimeContext {
            wasi: default_wasi()?,
            http: HttpCtx::new(Some(allowed_hosts), max_concurrency)?,
            data: Some(handler::HandlerData {}),
        };

        // Link `wasi_experimental_http`
        ctx.http.add_to_linker(&mut linker)?;

        let mut store: Store<RuntimeContext> = Store::new(engine, ctx);
    
        let (plugin, _) = Handler::instantiate(&mut store, module_ref, &mut linker, |ctx: &mut RuntimeContext| {
            ctx.data.as_mut().unwrap()
        })?;

        let resp = match plugin
            .handler(store, serde_json::to_string(&event.payload).unwrap().as_str(), None)
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

fn retrieve_config() -> (String, Vec<String>, Option<u32>) {
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

    (filename, allowed_hosts, max_concurrency)
}

/// Top-level runtime context data to be passed to a component.
pub(crate) struct RuntimeContext {
    pub wasi: WasiCtx,
    pub http: HttpCtx,
    pub data: Option<handler::HandlerData>,
}

/// The engine struct that encapsulate wasmtime engine
#[derive(Clone)]
pub struct Engine(wasmtime::Engine);

impl Engine {
    /// Create a new engine and initialize it.
    pub fn new() -> Result<Self, Error> {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
        let mut config = wasmtime::Config::default();
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
        config.wasm_multi_memory(true);
        config.wasm_module_linking(true);
        config.wasm_simd(true);
    
        Ok(Self(wasmtime::Engine::new(&config)?))
    }

    /// Get a clone of the internal `wasmtime::Engine`.
    pub fn inner(&self) -> wasmtime::Engine {
        self.0.clone()
    }
}

fn default_wasi() -> Result<WasiCtx, Error> {
    let ctx = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_args()?
        .inherit_env()?;
    Ok(ctx.build())
}
