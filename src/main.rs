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

use anyhow::{bail, Context as _, Result as AnyHowResult};
// use cranelift_codegen::settings::{builder, Builder, Configurable, Flags};
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
// use wasi_common::preopen_dir;
// use wasmtime::{Config, Engine, Extern, HostRef, Instance, Module, Store};
// use wasmtime_interface_types::{ModuleData, Value};
// use wasmtime_jit::{CompilationStrategy, Features};
// use wasmtime_runtime::{Export, InstanceHandle};
// use wasmtime_wasi::create_wasi_instance;
// use wasmtime_wasi_c::instantiate_wasi_c;

use wasi_common::preopen_dir;
use wasmtime::{Config, Engine, Extern, Instance, Module, OptLevel, Store, Strategy}; // Config, Extern
use wasmtime_interface_types::{ModuleData, Value};
use wasmtime_wasi::{old::snapshot_0::Wasi as WasiSnapshot0, Wasi};

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

// macro_rules! lambda_wasi_args {
//     ($response: ident) => {
//         vec![
//             Value::String($response.text()?),
//             Value::String(create_context!($response.headers())),
//         ];
//     };
// }

fn instantiate_module(
    store: &Store,
    module_registry: &ModuleRegistry,
    path: &Path,
) -> AnyHowResult<(Instance, Module, Vec<u8>)> {
    // Read the wasm module binary either as `*.wat` or a raw binary
    let data: Vec<u8> = wat::parse_file(path)?;

    let module: Module = Module::new(store, &data)?;

    // Resolve import using module_registry.
    let imports: Vec<Extern> = module
        .imports()
        .iter()
        .map(|i| {
            // TODO: annotations
            let export = match i.module() {
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

    Ok((instance, module, data))
}

fn invoke_export(
    instance: &Instance,
    data: &ModuleData,
    name: &str,
    args: Vec<Value>,
) -> AnyHowResult<String> {
    // let values: Vec<Value> = args.iter().map(|v| Value::String(v.to_owned())).collect();

    let results: Vec<Value> = data
        .invoke_export(instance, name, &args)
        .with_context(|| format!("failed to invoke `{}`", name))?;

    let return_value: String = results
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join("");

    Ok(return_value)
}

// TODO: make this compile
fn create_static_config() -> AnyHowResult<Config> {
    let mut config: Config = Config::new();

    config
        .cranelift_debug_verifier(false)
        .debug_info(false)
        .wasm_bulk_memory(true)
        .wasm_simd(true)
        .wasm_reference_types(true)
        .wasm_multi_value(true)
        .wasm_threads(true)
        .strategy(Strategy::Cranelift)?;

    config.cranelift_opt_level(OptLevel::Speed);

    Ok(config)
}

struct ModuleRegistry {
    wasi_snapshot_preview1: Wasi,
    wasi_unstable: WasiSnapshot0,
}

impl ModuleRegistry {
    fn new(
        store: &Store,
        preopen_dirs: &[(String, File)],
        argv: &[String],
        env_vars: &[(String, String)],
    ) -> AnyHowResult<ModuleRegistry> {
        let mut cx1 = wasi_common::WasiCtxBuilder::new()
            .inherit_stdio()
            .args(argv)
            .envs(env_vars);

        for (name, file) in preopen_dirs {
            cx1 = cx1.preopened_dir(file.try_clone()?, name);
        }

        let cx1 = cx1.build()?;

        let mut cx2 = wasi_common::old::snapshot_0::WasiCtxBuilder::new()
            .inherit_stdio()
            .args(argv)
            .envs(env_vars);

        for (name, file) in preopen_dirs {
            cx2 = cx2.preopened_dir(file.try_clone()?, name);
        }

        let cx2 = cx2.build()?;

        Ok(ModuleRegistry {
            wasi_snapshot_preview1: Wasi::new(store, cx1),
            wasi_unstable: WasiSnapshot0::new(store, cx2),
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

    print!(
        "{} {}\n{}\n{}\n{}\n",
        option_env!("CARGO_PKG_NAME").unwrap_or_default(),
        PKG_VERSION.unwrap_or_default(),
        option_env!("CARGO_PKG_DESCRIPTION").unwrap_or_default(),
        option_env!("CARGO_PKG_REPOSITORY").unwrap_or_default(),
        option_env!("CARGO_PKG_AUTHORS").unwrap_or_default()
    );

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

    // let mut flag_builder: Builder = builder();
    // let mut features: Features = Default::default();

    // // There are two possible traps for division, and this way
    // // we get the proper one if code traps.
    // flag_builder.enable("avoid_div_traps")?;
    //
    // // SIMD enabled by default
    // flag_builder.enable("enable_simd")?;
    // features.simd = true;
    //
    // // Enable optimization by default
    // flag_builder.set("opt_level", "speed")?;
    //
    // let mut config: Config = Config::new();
    //
    // config
    //     .features(features)
    //     .flags(Flags::new(flag_builder))
    //     .debug_info(false)
    //     .strategy(CompilationStrategy::Cranelift);
    let preopen_dirs: Vec<(String, File)> = vec![("/tmp".to_string(), preopen_dir("/tmp")?)];

    let argv: Vec<String> = vec![Path::new(&file)
        .components()
        .next_back()
        .map(Component::as_os_str)
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_owned()];

    // TODO: check what exact env vars should be passed according to aws
    let env_vars: Vec<(String, String)> = vec![(
        "RUNTIME_VERSION".to_string(),
        PKG_VERSION.unwrap_or_default().to_string(),
    )];

    let config: Config = create_static_config()?;
    let engine: Engine = Engine::new(&config);
    let store: Store = Store::new(&engine);
    let module_registry: ModuleRegistry =
        ModuleRegistry::new(&store, &preopen_dirs, &argv, &env_vars)?;

    // let mut module_registry: HashMap<String, HostRef<Instance>> = HashMap::new();

    // if enable_wasi {
    //     let preopen_dirs: Vec<(String, File)> = vec![("/tmp".to_string(), preopen_dir("/tmp")?)];
    //
    //     let argv: Vec<String> = vec![Path::new(&file)
    //         .components()
    //         .next_back()
    //         .map(Component::as_os_str)
    //         .and_then(OsStr::to_str)
    //         .unwrap_or("")
    //         .to_owned()];
    //
    //     // TODO: check what exact env vars should be passed according to aws
    //     let environ: Vec<(String, String)> = vec![(
    //         "RUNTIME_VERSION".to_string(),
    //         PKG_VERSION.unwrap_or_default().to_string(),
    //     )];
    //
    //     let wasi_unstable: HostRef<Instance> = HostRef::new({
    //         let global_exports: Rc<RefCell<HashMap<String, Option<Export>>>> =
    //             store.borrow().global_exports().clone();
    //
    //         let handle: InstanceHandle =
    //             instantiate_wasi_c("", global_exports, &preopen_dirs, &argv, &environ)?;
    //
    //         Instance::from_handle(&store, handle)
    //     });
    //
    //     let wasi_snapshot_preview1 = HostRef::new(create_wasi_instance(
    //         &store,
    //         &preopen_dirs,
    //         &argv,
    //         &environ,
    //     )?);
    //
    //     module_registry.insert("wasi_unstable".to_owned(), wasi_unstable);
    //     module_registry.insert("wasi_snapshot_preview1".to_owned(), wasi_snapshot_preview1);
    // }

    let main_module_path: &Path = Path::new(&file);

    // Load the main wasm module.
    let (instance, _module, data): (Instance, Module, Vec<u8>) =
        instantiate_module(&store, &module_registry, main_module_path)?;

    let main_module_data: ModuleData = ModuleData::new(&data)?;

    let client: Client = Client::new();

    loop {
        let response: Response = client.get(&api_next).send()?;
        let headers: &HeaderMap = response.headers();
        let context: String = create_context!(headers);
        let event: String = response.text()?;
        let args: Vec<Value> = vec![Value::String(event), Value::String(context)];

        match invoke_export(&instance, &main_module_data, &handler, args) {
            Ok(result) => client.post(&api_ok).body(result).send()?,
            _ => client
                .post(&api_err)
                .body("{\"error\":\"lambda invocation failed\"}")
                .send()?,
        };
    }
}
