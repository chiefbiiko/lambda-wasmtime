use std::{env, ffi::CStr, os::raw::c_char};

use anyhow::Error;
use lambda_runtime::{service_fn, LambdaEvent};
use serde::{Deserialize, Serialize};
use wasi_cap_std_sync::WasiCtxBuilder;
use wasi_experimental_http_wasmtime::HttpCtx;
use wasmtime::{AsContextMut, Engine, Instance, Linker, Store};
use wasmtime_wasi::*;

const MEMORY: &str = "memory";
const HANDLER_METHOD: &str = "_start";
const ALLOC_FN: &str = "alloc";
const DEALLOC_FN: &str = "dealloc";

#[derive(Debug, Deserialize)]
struct Request {
    input: String,
}

#[derive(Debug, Serialize)]
struct Response {
    output: String,
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

    let handler_func = move |event: LambdaEvent<Request>| async move {
        let mut vars: Vec<(String, String)> = Vec::new();
        let mut task_root = "".to_string();
        let mut handler = "".to_string();
        let mut allowed_hosts: Vec<String> = Vec::new();
        let mut max_concurrency: Option<u32> = None;
        for (key, value) in env::vars() {
            tracing::info!("{:?}, {:?}", key, value);
            vars.push((key.clone(), value.clone()));
            if key == "_HANDLER" {
                handler = value
            } else if key == "ALLOWED_HOSTS" {
                allowed_hosts = value.split(',').into_iter().map(|x| x.to_string()).collect();
            } else if key == "LAMBDA_TASK_ROOT" {
                task_root = value;
            } else if key == "MAX_CONCURRENCY" {
                max_concurrency = Some(value.parse().unwrap())
            }
        }
        let filename = format!("{}/{}.wasm", task_root, handler);
        let (instance, mut store) = create_instance(
            filename,
            vars,
            Vec::from(["".to_string()]),
            Some(allowed_hosts),
            max_concurrency,
        )?;
        let func = instance
            .get_typed_func::<(i32, i32), i32, _>(&mut store, HANDLER_METHOD)
            .unwrap_or_else(|_err| panic!("cannot find function {}", HANDLER_METHOD));

        tracing::info!("{:?}", event);
        // extract some useful info from the request
        let input = event.payload.input;
        // write the input array to the module's linear memory
        let input_bytes = input.as_bytes();
        let ptr = copy_memory(input_bytes, &instance, &mut store)?;
        let args = (ptr, input_bytes.len() as i32);

        let res_ptr = func.call(&mut store, args).expect("failed to invoke command");
        println!("{:?}", res_ptr);

        // let output: String = read_string(res_ptr);

        // call the module's dealloc function for the result string
        let dealloc = instance
            .get_typed_func::<i32, _, _>(&mut store, DEALLOC_FN)
            .expect("expected function not found");

        dealloc.call(&mut store, res_ptr)?;
            
        // prepare the response
        let resp = Response {
            output: input,
        };
        println!("{:?}", resp);
    
        // return `Response` (it will be serialized to JSON automatically by the runtime)
        Result::<Response, Error>::Ok(resp)
    };

    let lambda = lambda_runtime::run(service_fn(handler_func));
    if let Err(err) = lambda.await {
        tracing::error!("lambda error: {:?}", err);
    }
    Ok(())
}

fn create_instance(
    filename: String,
    vars: Vec<(String, String)>,
    args: Vec<String>,
    allowed_hosts: Option<Vec<String>>,
    max_concurrent_requests: Option<u32>,
) -> Result<(Instance, Store<WasiCtx>), Error> {
    tracing::info!("create_instance 1");
    let mut wasmtime_config = wasmtime::Config::default();
    wasmtime_config.wasm_multi_memory(true);
    wasmtime_config.wasm_module_linking(true);
    wasmtime_config.wasm_simd(true);
    let engine = Engine::new(&wasmtime_config)?;
    let mut linker = Linker::new(&engine);

    let ctx = WasiCtxBuilder::new()
        .inherit_stdin()
        .inherit_stdout()
        .inherit_stderr()
        .envs(&vars)?
        .args(&args)?
        .build();

    tracing::info!("create_instance 2");

    let mut store = Store::new(&engine, ctx);
    wasmtime_wasi::add_to_linker(&mut linker, |cx| cx)?;

    // Link `wasi_experimental_http`
    let http = HttpCtx::new(allowed_hosts, max_concurrent_requests)?;
    http.add_to_linker(&mut linker)?;

    let module = wasmtime::Module::from_file(store.engine(), filename)?;
    let instance = linker.instantiate(&mut store, &module)?;

    tracing::info!("create_instance 3");

    Ok((instance, store))
}

/// Copy a byte array into an instance's linear memory
/// and return the offset relative to the module's memory.
fn copy_memory(bytes: &[u8], instance: &Instance, mut store: impl AsContextMut) -> Result<i32, anyhow::Error> {
    // Get the "memory" export of the module.
    // If the module does not export it, just panic,
    // since we are not going to be able to copy array data.
    let memory = instance
        .get_memory(&mut store, MEMORY)
        .expect("expected memory not found");

    // The module is not using any bindgen libraries, so it should export
    // its own alloc function.
    //
    // Get the guest's exported alloc function, and call it with the
    // length of the byte array we are trying to copy.
    // The result is an offset relative to the module's linear memory, which is
    // used to copy the bytes into the module's memory.
    // Then, return the offset.
    let alloc = instance
        .get_typed_func::<i32, i32, _>(&mut store, ALLOC_FN)
        .expect("expected alloc function not found");
    let guest_ptr_offset = alloc.call(&mut store, bytes.len() as i32)?;
    unsafe {
        let raw = memory.data_ptr(&mut store).offset(guest_ptr_offset as isize);
        raw.copy_from(bytes.as_ptr(), bytes.len());
    }
    Ok(guest_ptr_offset)
}

/// Read a Rust `String` from a module's memory, given an offset and length.
pub fn read_string(data_ptr: i32) -> String {
    let slice = unsafe { CStr::from_ptr(data_ptr as *const c_char) };
    let str = slice.to_str().unwrap().to_string();
    println!("string returned: {}", slice.to_str().unwrap());
    str
}
