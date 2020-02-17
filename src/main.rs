//! Custom AWS Lambda WebAssembly runtime based on the
//! [wasmtime](https://github.com/bytecodealliance/wasmtime) crate.
//!
//! Supports various WebAssembly proposals, such as SIMD, WASI and Interface
//! Types. The latter is actually a development requirement as to embed the
//! runtime in AWS Lambda.
//!
//! The current runtime version only supports lambdas as wasm binaries. Future
//! releases might support deploying just the wat textfiles for a WebAssembly
//! lambda.
//!
//! Set the `ENABLE_WASI` env var in order to have wasi_unstable enabled. This
//! includes file system access to `/tmp`, the only accessible host directory in
//! AWS Lambda.

use anyhow::{anyhow, bail, Context as _, Result as AnyHowResult};
use cranelift_codegen::settings::{builder, Builder, Configurable, Flags};
use reqwest::{
    blocking::{Client, Response},
    header::HeaderMap,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    env::var,
    ffi::OsStr,
    fs::File,
    path::{Component, Path},
    rc::Rc,
};
use wasi_common::preopen_dir;
use wasmtime::{Config, Engine, Extern, HostRef, Instance, Module, Store};
use wasmtime_interface_types::{ModuleData, Value};
use wasmtime_jit::{CompilationStrategy, Features};
use wasmtime_runtime::{Export, InstanceHandle};
use wasmtime_wasi_c::instantiate_wasi_c;

macro_rules! get_hdr_str {
    ($headers:ident, $key:tt) => {
        $headers
            .get($key)
            .ok_or(anyhow!(format!("missing header {}", $key)))?
            .to_str()?;
    };
}

fn instantiate_module(
    store: &HostRef<Store>,
    module_registry: &HashMap<String, HostRef<Instance>>,
    path: &Path,
) -> AnyHowResult<(HostRef<Instance>, HostRef<Module>, Vec<u8>)> {
    // Read the wasm module binary either as `*.wat` or a raw binary
    let data: Vec<u8> = wat::parse_file(path.to_path_buf())?;

    let module: HostRef<Module> = HostRef::new(Module::new(store, &data)?);

    // Resolve import using module_registry.
    let imports: Vec<Extern> = module
        .borrow()
        .imports()
        .iter()
        .map(|i| {
            let module_name: &str = i.module().as_str();

            if let Some(instance) = module_registry.get(module_name) {
                let field_name: &str = i.name().as_str();

                if let Some(export) = instance.borrow().find_export_by_name(field_name) {
                    Ok(export.clone())
                } else {
                    bail!(
                        "Import {} was not found in module {}",
                        field_name,
                        module_name
                    )
                }
            } else {
                bail!("Import module {} was not found", module_name)
            }
        })
        .collect::<AnyHowResult<Vec<_>, _>>()?;

    let instance: HostRef<Instance> = HostRef::new(Instance::new(store, &module, &imports)?);

    Ok((instance, module, data))
}

fn invoke_export(
    instance: &HostRef<Instance>,
    data: &ModuleData,
    name: &str,
    args: Vec<String>,
) -> AnyHowResult<String> {
    let values: Vec<Value> = args.iter().map(|v| Value::String(v.to_owned())).collect();

    let results: Vec<Value> = data
        .invoke_export(instance, name, &values)
        .with_context(|| format!("failed to invoke `{}`", name))?;

    let return_value: String = results
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join("");

    Ok(return_value)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    println!("{:?} {:?}", option_env!("CARGO_PKG_NAME"), PKG_VERSION);
    println!("{:?}", option_env!("CARGO_PKG_DESCRIPTION"));
    println!("{:?}", option_env!("CARGO_PKG_REPOSITORY"));
    println!("{:?}", option_env!("CARGO_PKG_AUTHORS"));

    let enable_wasi: bool = var("ENABLE_WASI").unwrap_or_default() != "";

    let runtime_api_host: String = var("AWS_LAMBDA_RUNTIME_API")?;
    let runtime_api: String = format!("http://{}/2018-06-01/runtime", runtime_api_host);
    let api_next: String = format!("{}/invocation/next", runtime_api);
    let api_err: String = format!("{}/invocation/error", runtime_api);
    let api_ok: String = format!("{}/invocation/response", runtime_api);

    let _file_handler: Vec<String> = var("_HANDLER")?.split('.').map(str::to_string).collect();
    // TODO: allow passing wat file
    let file: String = format!("{}.wasm", _file_handler[0]);
    let handler: String = _file_handler[1].to_owned();

    let mut flag_builder: Builder = builder();
    let mut features: Features = Default::default();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    flag_builder.enable("avoid_div_traps")?;

    // SIMD enabled by default
    flag_builder.enable("enable_simd")?;
    features.simd = true;

    // Enable optimization by default
    flag_builder.set("opt_level", "speed")?;

    let mut config: Config = Config::new();

    config
        .features(features)
        .flags(Flags::new(flag_builder))
        .debug_info(false)
        .strategy(CompilationStrategy::Cranelift);

    let engine: HostRef<Engine> = HostRef::new(Engine::new(&config));
    let store: HostRef<Store> = HostRef::new(Store::new(&engine));

    let mut module_registry: HashMap<String, HostRef<Instance>> = HashMap::new();

    if enable_wasi {
        let preopen_dirs: Vec<(String, File)> = vec![("/tmp".to_string(), preopen_dir("/tmp")?)];

        let argv: Vec<String> = vec![Path::new(&file)
            .components()
            .next_back()
            .map(Component::as_os_str)
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_owned()];

        // TODO: check what exact env vars should be passed according to aws
        let environ: Vec<(String, String)> = vec![(
            "RUNTIME_VERSION".to_string(),
            PKG_VERSION.unwrap_or_default().to_string(),
        )];

        let wasi_unstable: HostRef<Instance> = HostRef::new({
            let global_exports: Rc<RefCell<HashMap<String, Option<Export>>>> =
                store.borrow().global_exports().clone();

            let handle: InstanceHandle =
                instantiate_wasi_c("", global_exports, &preopen_dirs, &argv, &environ)?;

            Instance::from_handle(&store, handle)
        });

        module_registry.insert("wasi_unstable".to_owned(), wasi_unstable);
    }

    // Load the main wasm module.
    let (instance, _module, data): (HostRef<Instance>, HostRef<Module>, Vec<u8>) =
        instantiate_module(&store, &module_registry, Path::new(&file))?;

    let module_data: ModuleData = ModuleData::new(&data)?;

    let client: Client = Client::new();

    loop {
        let response: Response = client.get(&api_next).send()?;
        let headers: &HeaderMap = response.headers();

        let context: String = format!(
            "{{\"function_arn\":\"{}\",\"deadline_ms\":\"{:?}\",\"request_id\"\
             :\"{:?}\",\"trace_id\":\"{:?}\"}}",
            get_hdr_str!(headers, "Lambda-Runtime-Invoked-Function-Arn"),
            get_hdr_str!(headers, "Lambda-Runtime-Deadline-Ms"),
            get_hdr_str!(headers, "Lambda-Runtime-Request-Id"),
            get_hdr_str!(headers, "Lambda-Runtime-Trace-Id"),
        );

        let event: String = response.text()?;

        let args: Vec<String> = vec![event, context];

        match invoke_export(&instance, &module_data, &handler, args) {
            Ok(result) => client.post(&api_ok).body(result).send()?,
            _ => client
                .post(&api_err)
                .body("{\"error\":\"lambda invocation failed\"}")
                .send()?,
        };
    }
}
