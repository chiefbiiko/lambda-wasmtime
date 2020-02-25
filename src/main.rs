//! Custom AWS Lambda WebAssembly runtime based on the
//! [wasmtime](https://github.com/bytecodealliance/wasmtime) crate.
//!
//! Supports various WebAssembly proposals, such as SIMD, WASI and Interface
//! Types. The latter is actually a development requirement as to embed the
//! runtime in AWS Lambda.
//!
//! The current runtime version only supports lambdas as wasm binaries. Future
//! releases might support deploying just the wat textfiles for a WebAssembly
//! lambda. /tmp is the only preopened directory so far.

use anyhow::{bail, Context as _, Result as AnyHowResult};
use reqwest::{
    blocking::{Client, Response},
    header::HeaderMap,
};
use std::{env::var, path::Path};
use wasi_common::{
    old::snapshot_0::{WasiCtx as WasiCtxSnapshot0, WasiCtxBuilder as WasiCtxBuilderSnapshot0},
    preopen_dir, WasiCtx, WasiCtxBuilder,
};
use wasmtime::{Config, Engine, Func, Instance, Module, OptLevel, Store, Strategy}; // Config, Extern
use wasmtime_interface_types::{ModuleData, Value};
use wasmtime_wasi::{old::snapshot_0::Wasi as WasiSnapshot0, Wasi};

const EINVC: &'static str = "{\"error\":\"lambda invocation failed\"}";

macro_rules! print_runtime_info {
    () => {
        print!(
            "runtime: {} {}\nrepo: {}\nauthor: {}\n",
            option_env!("CARGO_PKG_NAME").unwrap_or_default(),
            option_env!("CARGO_PKG_VERSION").unwrap_or_default(),
            option_env!("CARGO_PKG_REPOSITORY").unwrap_or_default(),
            option_env!("CARGO_PKG_AUTHORS").unwrap_or_default()
        );
    };
}

macro_rules! create_context {
    ($headers:ident) => {
        format!(
            "{{\"function_arn\":\"{}\",\"deadline_ms\":\"{}\",\"request_id\"\
         :\"{}\",\"trace_id\":\"{}\",\"client_context\":\"{}\",\"cognito_identity\":\"{}\"}}",
            $headers["Lambda-Runtime-Invoked-Function-Arn"]
                .to_str()
                .unwrap_or_default(),
            $headers["Lambda-Runtime-Deadline-Ms"]
                .to_str()
                .unwrap_or_default(),
            $headers["Lambda-Runtime-Aws-Request-Id"]
                .to_str()
                .unwrap_or_default(),
            $headers["Lambda-Runtime-Trace-Id"]
                .to_str()
                .unwrap_or_default(),
            $headers["Lambda-Runtime-Client-Context"]
                .to_str()
                .unwrap_or_default(),
            $headers["Lambda-Runtime-Cognito-Identity"]
                .to_str()
                .unwrap_or_default()
        );
    };
}

fn get_runtime_endpoints() -> AnyHowResult<(String, String, String)> {
    let runtime_api: String = format!(
        "http://{}/2018-06-01/runtime",
        var("AWS_LAMBDA_RUNTIME_API")?
    );

    Ok((
        format!("{}/invocation/next", runtime_api),
        format!("{}/invocation/error", runtime_api),
        format!("{}/invocation/response", runtime_api),
    ))
}

// TODO: allow passing wat file?
fn get_module_info() -> AnyHowResult<(String, String)> {
    let file_handler: Vec<String> = var("_HANDLER")?.split('.').map(str::to_string).collect();
    let file: String = format!("{}.wasm", file_handler[0]);

    Ok((file, file_handler[1].to_string()))
}

fn create_engine_config() -> AnyHowResult<Config> {
    let mut config: Config = Config::new();

    config
        .cranelift_debug_verifier(false)
        .debug_info(false)
        .wasm_bulk_memory(false)
        .wasm_simd(false)
        .wasm_reference_types(false)
        .wasm_multi_value(false)
        .wasm_threads(false)
        .strategy(Strategy::Cranelift)?;

    config.cranelift_opt_level(OptLevel::Speed);

    Ok(config)
}

fn instantiate_module(
    store: &Store,
    module_registry: &LambdaModuleRegistry,
    path: &Path,
) -> AnyHowResult<(Instance, Module, ModuleData)> {
    // Read the wasm module binary either as `*.wat` or a raw binary
    let data: Vec<u8> = wat::parse_file(path)?;

    let module: Module = Module::new(store, &data)?;

    let module_data: ModuleData = ModuleData::new(&data)?;

    // Resolve import using module_registry.
    let imports = module // : Vec<Extern>
        .imports()
        .iter()
        .map(|i| {
            let export: Option<&Func> = match i.module() {
                "wasi_snapshot_preview1" => {
                    module_registry.wasi_snapshot_preview1.get_export(i.name())
                }
                "wasi_unstable" => module_registry.wasi_unstable.get_export(i.name()),
                other => bail!("import module `{}` was not found", other),
            };

            match export {
                Some(export) => Ok(export.clone().into()),
                None => bail!(
                    "import `{}` was not found in module `{}`",
                    i.name(),
                    i.module()
                ),
            }
        })
        .collect::<AnyHowResult<Vec<_>, _>>()?;

    let instance: Instance =
        Instance::new(&module, &imports).context(format!("failed to instantiate {:?}", path))?;

    Ok((instance, module, module_data))
}

fn invoke_lambda(
    instance: &Instance,
    data: &ModuleData,
    name: &str,
    event: String,
    context: String,
) -> AnyHowResult<String> {
    let values: Vec<Value> = vec![Value::String(event), Value::String(context)];

    let results: Vec<Value> = data
        .invoke_export(instance, name, &values)
        .with_context(|| format!("failed to invoke `{}`", name))?;

    let return_value: String = results
        .iter()
        .map(Value::to_string)
        .collect::<Vec<String>>()
        .join("");

    Ok(return_value)
}

struct LambdaModuleRegistry {
    wasi_snapshot_preview1: Wasi,
    wasi_unstable: WasiSnapshot0,
}

impl LambdaModuleRegistry {
    fn new(store: &Store, main_module_path: &str) -> AnyHowResult<LambdaModuleRegistry> {
        let cx1: WasiCtx = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .preopened_dir(preopen_dir("/tmp")?, "/tmp".to_string())
            .arg(main_module_path)
            .build()?;

        let cx2: WasiCtxSnapshot0 = WasiCtxBuilderSnapshot0::new()
            .inherit_stdio()
            .inherit_env()
            .preopened_dir(preopen_dir("/tmp")?, "/tmp".to_string())
            .arg(main_module_path)
            .build()?;

        Ok(LambdaModuleRegistry {
            wasi_snapshot_preview1: Wasi::new(store, cx1),
            wasi_unstable: WasiSnapshot0::new(store, cx2),
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    print_runtime_info!();

    let (api_next, api_err, api_ok): (String, String, String) = get_runtime_endpoints()?;
    let (file, lambda): (String, String) = get_module_info()?;

    let config: Config = create_engine_config()?;
    let engine: Engine = Engine::new(&config);
    let store: Store = Store::new(&engine);

    let module_registry: LambdaModuleRegistry = LambdaModuleRegistry::new(&store, &file)?;

    let (instance, _module, module_data): (Instance, Module, ModuleData) =
        instantiate_module(&store, &module_registry, Path::new(&file))?;

    let client: Client = Client::new();

    loop {
        let response: Response = client.get(&api_next).send()?;
        let headers: &HeaderMap = response.headers();
        let context: String = create_context!(headers);
        let event: String = response.text()?;

        match invoke_lambda(&instance, &module_data, &lambda, event, context) {
            Ok(result) => client.post(&api_ok).body(result).send()?,
            _ => client.post(&api_err).body(EINVC).send()?,
        };
    }
}
