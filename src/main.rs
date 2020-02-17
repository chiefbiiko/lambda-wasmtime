// TODO: provide docs that can be build with cargo

//! CLI tool to use the functions provided by the [wasmtime](../wasmtime/index.html)
//! crate.
//!
//! Reads Wasm binary files (one Wasm module per file), translates the functions' code to Cranelift
//! IL. Can also execute the `start` function of the module by laying out the memories, globals
//! and tables, then emitting the translated code with hardcoded addresses to memory.

// #![deny(
//     missing_docs,
//     trivial_numeric_casts,
//     unused_extern_crates,
//     unstable_features
// )]
// #![warn(unused_import_braces)]
// #![cfg_attr(feature = "clippy", plugin(clippy(conf_file = "../clippy.toml")))]
// #![cfg_attr(
//     feature = "cargo-clippy",
//     allow(clippy::new_without_default, clippy::new_without_default_derive)
// )]
// #![cfg_attr(
//     feature = "cargo-clippy",
//     warn(
//         clippy::float_arithmetic,
//         clippy::mut_mut,
//         clippy::nonminimal_bool,
//         clippy::option_map_unwrap_or,
//         clippy::option_map_unwrap_or_else,
//         clippy::unicode_not_nfc,
//         clippy::use_self
//     )
// )]

use anyhow::{anyhow, bail, Context as _, Result as AnyHowResult};
use cranelift_codegen::settings::{builder, Builder as FlagBuilder, Configurable, Flags};
// use docopt::Docopt;
use reqwest::{
    blocking::{Client, Response},
    header::HeaderMap,
};
// use serde::Deserialize;
// use std::env::var;
// use std::path::{Component, Path};
use std::{
    collections::HashMap,
    env::var,
    ffi::OsStr,
    fs::File,
    path::{Component, Path},
};
// use surf;
use wasi_common::preopen_dir;
use wasmtime::{Config, Engine, Extern, HostRef, Instance, Module, Store};
// use wasmtime_runtime::InstanceHandle;
// use wasmtime_cli::pick_compilation_strategy;
// use wasmtime_environ::{cache_create_new_config, cache_init};
use wasmtime_interface_types::{ModuleData, Value};
use wasmtime_jit::{CompilationStrategy, Features};
// use wasmtime_wasi::create_wasi_instance;
// use wasmtime_wasi::old::snapshot_0::create_wasi_instance as create_wasi_instance_snapshot_0;
// #[cfg(feature = "wasi-c")]
// use wasm_webidl_bindings::ast;
// use wasmtime_interface_types::Value;
use wasmtime_wasi_c::instantiate_wasi_c;
// use wasmtime_wast::instantiate_spectest;

// const USAGE: &str = "
// Wasm runner.
// Takes a binary (wasm) or text (wat) WebAssembly module and instantiates it,
// including calling the start function if one is present. Additional functions
// given with --invoke are then called.
// Usage:
//     wasmtime [-odg] [--enable-simd] [--wasi-c] [--disable-cache | \
//      --cache-config=<cache_config_file>] [--preload=<wasm>...] [--env=<env>...] [--dir=<dir>...] \
//      [--mapdir=<mapping>...] [--lightbeam | --cranelift] <file> [<arg>...]
//     wasmtime [-odg] [--enable-simd] [--wasi-c] [--disable-cache | \
//      --cache-config=<cache_config_file>] [--env=<env>...] [--dir=<dir>...] \
//      [--mapdir=<mapping>...] --invoke=<fn> [--lightbeam | --cranelift] <file> [<arg>...]
//     wasmtime --create-cache-config [--cache-config=<cache_config_file>]
//     wasmtime --help | --version
// Options:
//     --invoke=<fn>       name of function to run
//     -o, --optimize      runs optimization passes on the translated functions
//     --disable-cache     disables cache system
//     --cache-config=<cache_config_file>
//                         use specified cache configuration;
//                         can be used with --create-cache-config to specify custom file
//     --create-cache-config
//                         creates default configuration and writes it to the disk,
//                         use with --cache-config to specify custom config file
//                         instead of default one
//     -g                  generate debug information
//     -d, --debug         enable debug output on stderr/stdout
//     --lightbeam         use Lightbeam for all compilation
//     --cranelift         use Cranelift for all compilation
//     --enable-simd       enable proposed SIMD instructions
//     --wasi-c            enable the wasi-c implementation of `wasi_unstable`
//     --preload=<wasm>    load an additional wasm module before loading the main module
//     --env=<env>         pass an environment variable (\"key=value\") to the program
//     --dir=<dir>         grant access to the given host directory
//     --mapdir=<mapping>  where <mapping> has the form <wasmdir>::<hostdir>, grant access to
//                         the given host directory with the given wasm directory name
//     -h, --help          print this help message
//     --version           print the Cranelift version
// ";

// #[derive(Deserialize, Debug, Clone)]
// struct Args {
//     arg_file: String,
//     arg_arg: Vec<String>,
//     flag_optimize: bool,
//     flag_disable_cache: bool,
//     flag_cache_config: Option<String>,
//     flag_create_cache_config: bool,
//     flag_debug: bool,
//     flag_g: bool,
//     flag_enable_simd: bool,
//     flag_lightbeam: bool,
//     flag_cranelift: bool,
//     flag_invoke: Option<String>,
//     flag_preload: Vec<String>,
//     flag_env: Vec<String>,
//     flag_dir: Vec<String>,
//     flag_mapdir: Vec<String>,
//     flag_wasi_c: bool,
// }

// fn pick_compilation_strategy(cranelift: bool, lightbeam: bool) -> CompilationStrategy {
//   // Decide how to compile.
//   match (lightbeam, cranelift) {
//       #[cfg(feature = "lightbeam")]
//       (true, false) => CompilationStrategy::Lightbeam,
//       #[cfg(not(feature = "lightbeam"))]
//       (true, false) => panic!("--lightbeam given, but Lightbeam support is not enabled"),
//       (false, true) => CompilationStrategy::Cranelift,
//       (false, false) => CompilationStrategy::Auto,
//       (true, true) => panic!("Can't enable --cranelift and --lightbeam at the same time"),
//   }
// }

// fn init_file_per_thread_logger(prefix: &'static str) {
//   file_per_thread_logger::initialize(prefix);
//
//   // Extending behavior of default spawner:
//   // https://docs.rs/rayon/1.1.0/rayon/struct.ThreadPoolBuilder.html#method.spawn_handler
//   // Source code says DefaultSpawner is implementation detail and
//   // shouldn't be used directly.
//   rayon::ThreadPoolBuilder::new()
//       .spawn_handler(move |thread| {
//           let mut b = std::thread::Builder::new();
//           if let Some(name) = thread.name() {
//               b = b.name(name.to_owned());
//           }
//           if let Some(stack_size) = thread.stack_size() {
//               b = b.stack_size(stack_size);
//           }
//           b.spawn(move || {
//               file_per_thread_logger::initialize(prefix);
//               thread.run()
//           })?;
//           Ok(())
//       })
//       .build_global()
//       .unwrap();
// }

// fn compute_preopen_dirs(flag_dir: &[String], flag_mapdir: &[String]) -> Vec<(String, File)> {
//     let mut preopen_dirs = Vec::new();
//
//     for dir in flag_dir {
//         let preopen_dir = preopen_dir(dir).unwrap_or_else(|err| {
//             println!("error while pre-opening directory {}: {}", dir, err);
//             exit(1);
//         });
//         preopen_dirs.push((dir.clone(), preopen_dir));
//     }
//
//     for mapdir in flag_mapdir {
//         let parts: Vec<&str> = mapdir.split("::").collect();
//         if parts.len() != 2 {
//             println!(
//                 "--mapdir argument must contain exactly one double colon ('::'), separating a \
//                  guest directory name and a host directory name"
//             );
//             exit(1);
//         }
//         let (key, value) = (parts[0], parts[1]);
//         let preopen_dir = preopen_dir(value).unwrap_or_else(|err| {
//             println!("error while pre-opening directory {}: {}", value, err);
//             exit(1);
//         });
//         preopen_dirs.push((key.to_string(), preopen_dir));
//     }
//
//     preopen_dirs
// }

// /// Compute the argv array values.
// fn compute_argv(argv0: &str, arg_arg: &[String]) -> Vec<String> {
//     let mut result = Vec::new();
//
//     // Add argv[0], which is the program name. Only include the base name of the
//     // main wasm module, to avoid leaking path information.
//     result.push(
//         Path::new(argv0)
//             .components()
//             .next_back()
//             .map(Component::as_os_str)
//             .and_then(OsStr::to_str)
//             .unwrap_or("")
//             .to_owned(),
//     );
//
//     // Add the remaining arguments.
//     for arg in arg_arg {
//         result.push(arg.to_owned());
//     }
//
//     result
// }

/// Compute the environ array values.
// fn compute_environ(flag_env: &[String]) -> Vec<(String, String)> {
//     let mut result = Vec::new();
//
//     // Add the environment variables, which must be of the form "key=value".
//     for env in flag_env {
//         let split = env.splitn(2, '=').collect::<Vec<_>>();
//         if split.len() != 2 {
//             println!(
//                 "environment variables must be of the form \"key=value\"; got \"{}\"",
//                 env
//             );
//         }
//         result.push((split[0].to_owned(), split[1].to_owned()));
//     }
//
//     result
// }

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
            let module_name = i.module().as_str();
            if let Some(instance) = module_registry.get(module_name) {
                let field_name = i.name().as_str();
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

// fn handle_module(
//     store: &HostRef<Store>,
//     module_registry: &HashMap<String, HostRef<Instance>>,
//     args: &Args,
//     path: &Path,
// ) -> AnyHowResult<()> {
//     let (instance, _module, data) = instantiate_module(store, module_registry, path)?;
//
//     // If a function to invoke was given, invoke it.
//     if let Some(f) = &args.flag_invoke {
//         let data = ModuleData::new(&data)?;
//         invoke_export(instance, &data, f, args)?;
//     }
//
//     Ok(())
// }

fn invoke_export(
    instance: &HostRef<Instance>,
    data: &ModuleData,
    name: &str,
    args: Vec<String>,
) -> AnyHowResult<String> {
    // let mut handle = instance.borrow().handle().clone();
    //
    // // Use the binding information in `ModuleData` to figure out what arguments
    // // need to be passed to the function that we're invoking. Currently we take
    // // the CLI parameters and attempt to parse them into function arguments for
    // // the function we'll invoke.
    // let binding = data.binding_for_export(&mut handle, name)?;
    // if binding.param_types()?.len() > 0 {
    //     eprintln!(
    //         "warning: using `--invoke` with a function that takes arguments \
    //          is experimental and may break in the future"
    //     );
    // }
    // let mut values = Vec::new();
    // let mut args = args.iter();
    // for ty in binding.param_types()? {
    //     let val = match args.next() {
    //         Some(s) => s,
    //         None => bail!("not enough arguments for `{}`", name),
    //     };
    //     values.push(match ty {
    //         // TODO: integer parsing here should handle hexadecimal notation
    //         // like `0x0...`, but the Rust standard library currently only
    //         // parses base-10 representations.
    //         ast::WebidlScalarType::Long => Value::I32(val.parse()?),
    //         ast::WebidlScalarType::LongLong => Value::I64(val.parse()?),
    //         ast::WebidlScalarType::UnsignedLong => Value::U32(val.parse()?),
    //         ast::WebidlScalarType::UnsignedLongLong => Value::U64(val.parse()?),
    //
    //         ast::WebidlScalarType::Float | ast::WebidlScalarType::UnrestrictedFloat => {
    //             Value::F32(val.parse()?)
    //         }
    //         ast::WebidlScalarType::Double | ast::WebidlScalarType::UnrestrictedDouble => {
    //             Value::F64(val.parse()?)
    //         }
    //         ast::WebidlScalarType::DomString => Value::String(val.to_string()),
    //         t => bail!("unsupported argument type {:?}", t),
    //     });
    // }

    let values: Vec<Value> = args.iter().map(|v| Value::String(v.to_owned())).collect();

    // Invoke the function and then afterwards print all the results that came
    // out, if there are any.
    let results: Vec<Value> = data
        .invoke_export(instance, name, &values)
        .with_context(|| format!("failed to invoke `{}`", name))?;
    // if results.len() > 0 {
    //     eprintln!(
    //         "warning: using `--invoke` with a function that returns values \
    //          is experimental and may break in the future"
    //     );
    // }
    // let mut return_value: String = "".to_string();

    let return_value: String = results
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<String>>()
        .join("");

    // for result in results {
    //     // println!("{}", result);
    //     return_value = format!("{}{}", return_value, result);
    // }
    //
    // Ok(())

    // Ok(results.iter().collect::<String>())

    Ok(return_value)
}

/*
CARGO_PKG_AUTHORS
CARGO_PKG_DESCRIPTION
CARGO_PKG_HOMEPAGE
CARGO_PKG_NAME
CARGO_PKG_REPOSITORY
CARGO_PKG_VERSION
*/

const PKG_NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");
const PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const PKG_DESCRIPTION: Option<&'static str> = option_env!("CARGO_PKG_DESCRIPTION");
const PKG_REPO: Option<&'static str> = option_env!("CARGO_PKG_REPOSITORY");
const PKG_AUTHOR: Option<&'static str> = option_env!("CARGO_PKG_AUTHORS");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{:?} {:?}", PKG_NAME, PKG_VERSION);
    println!("{:?}", PKG_DESCRIPTION);
    println!("{:?}", PKG_REPO);
    println!("{:?}", PKG_AUTHOR);

    let enable_wasi: bool = match var("ENABLE_WASI") {
        Ok(_val) => true,
        _ => false,
    };

    let runtime_api_host: String = var("AWS_LAMBDA_RUNTIME_API")?;
    let api: String = format!("http://{}/2018-06-01/runtime", runtime_api_host);
    let api_next: String = format!("{}/invocation/next", api);
    let api_err: String = format!("{}/invocation/error", api);
    let api_ok: String = format!("{}/invocation/response", api);

    let _file_handler: Vec<String> = var("_HANDLER")?.split('.').map(str::to_string).collect();
    let file: String = format!("{}.wasm", _file_handler[0]);
    let handler: String = _file_handler[1].to_owned();

    // let prep_env_vars = &[]; // TODO: pass env vars to module instance

    // obsolete
    // let args: Args = Docopt::new(USAGE)
    //     .and_then(|d| {
    //         d.help(true)
    //             .version(Some(String::from(version)))
    //             .deserialize()
    //     })
    //     .unwrap_or_else(|e| e.exit());

    // let log_config = if args.flag_debug {
    //     pretty_env_logger::init();
    //     None
    // } else {
    //     let prefix = "wasmtime.dbg.";
    //     wasmtime_cli::init_file_per_thread_logger(prefix);
    //     Some(prefix)
    // };

    // force debug false
    // let log_config = {
    //     let prefix = "wasmtime.dbg.";
    //     init_file_per_thread_logger(prefix);
    //     Some(prefix)
    // };

    // obsolete
    // if args.flag_create_cache_config {
    //     match cache_create_new_config(args.flag_cache_config) {
    //         Ok(path) => {
    //             println!(
    //                 "Successfully created new configuation file at {}",
    //                 path.display()
    //             );
    //             return Ok(());
    //         }
    //         Err(err) => {
    //             eprintln!("Error: {}", err);
    //             exit(1);
    //         }
    //     }
    // }

    // let errors = cache_init(
    //     !args.flag_disable_cache,
    //     args.flag_cache_config.as_ref(),
    //     log_config,
    // );

    // if !errors.is_empty() {
    //     eprintln!("Cache initialization failed. Errors:");
    //     for e in errors {
    //         eprintln!("-> {}", e);
    //     }
    //     exit(1);
    // }

    let mut flag_builder: FlagBuilder = builder();
    let mut features: Features = Default::default();

    // There are two possible traps for division, and this way
    // we get the proper one if code traps.
    // Forced
    flag_builder.enable("avoid_div_traps")?;

    // Enable/disable producing of debug info.
    // let debug_info = false; //args.flag_g;

    // // Enable verifier passes in debug mode.
    // if cfg!(debug_assertions) {
    //     flag_builder.enable("enable_verifier")?;
    // }

    // // Enable SIMD if requested
    // if args.flag_enable_simd {
    //     flag_builder.enable("enable_simd")?;
    //     features.simd = true;
    // }

    // SIMD enabled by default
    flag_builder.enable("enable_simd")?;
    features.simd = true;

    // if args.flag_optimize {
    //     flag_builder.set("opt_level", "speed")?;
    // }

    // Enable optimization by default
    flag_builder.set("opt_level", "speed")?;

    // // Decide how to compile.
    // let strategy = pick_compilation_strategy(args.flag_cranelift, args.flag_lightbeam);

    // force cranelift
    // let strategy = pick_compilation_strategy(true, false);

    let mut config: Config = Config::new();

    config
        .features(features)
        .flags(Flags::new(flag_builder))
        .debug_info(false)
        // .strategy(strategy);
        // TODO: test using lightbeam!!!
        .strategy(CompilationStrategy::Cranelift);

    let engine: HostRef<Engine> = HostRef::new(Engine::new(&config));
    let store: HostRef<Store> = HostRef::new(Store::new(&engine));

    let mut module_registry: HashMap<String, HostRef<Instance>> = HashMap::new();

    // TODO: think about tossing this...
    // // Make spectest available by default.
    // module_registry.insert(
    //     "spectest".to_owned(),
    //     HostRef::new(Instance::from_handle(&store, instantiate_spectest()?)),
    // );

    // Make wasi available by default.
    // let preopen_dirs = compute_preopen_dirs(&args.flag_dir, &args.flag_mapdir);
    // let preopen_dirs = compute_preopen_dirs(&["/tmp".to_string()], &[]);
    // let preopen_dirs: Vec<(String, File)> = vec![("/tmp".to_string(), preopen_dir("/tmp")?)];
    // 
    // // .unwrap_or_else(|err| {
    // //     println!("error while pre-opening directory {}: {}", dir, err);
    // //     exit(1);
    // // });
    // // preopen_dirs.push((dir.clone(), preopened_dir));
    // // let argv = compute_argv(&file, &[]);
    // let argv: Vec<String> = vec![Path::new(&file)
    //     .components()
    //     .next_back()
    //     .map(Component::as_os_str)
    //     .and_then(OsStr::to_str)
    //     .unwrap_or("")
    //     .to_owned()];
    // // let environ = compute_environ(&args.flag_env);
    // // TODO: check what exact env vars should be passed according to aws
    // // let environ = compute_environ(prep_env_vars);
    // let environ: Vec<(String, String)> =
    //     vec![("RUNTIME_VERSION".to_string(), format!("{:?}", PKG_VERSION))];

    // let wasi_unstable = HostRef::new(if args.flag_wasi_c {
    //     #[cfg(feature = "wasi-c")]
    //     {
    //         let global_exports = store.borrow().global_exports().clone();
    //         let handle = instantiate_wasi_c(global_exports, &preopen_dirs, &argv, &environ)?;
    //         Instance::from_handle(&store, handle)
    //     }
    //     #[cfg(not(feature = "wasi-c"))]
    //     {
    //         bail!("wasi-c feature not enabled at build time")
    //     }
    // } else {
    //     create_wasi_instance_snapshot_0(&store, &preopen_dirs, &argv, &environ)?
    // });

    if enable_wasi {
        let preopen_dirs: Vec<(String, File)> = vec![("/tmp".to_string(), preopen_dir("/tmp")?)];

        // .unwrap_or_else(|err| {
        //     println!("error while pre-opening directory {}: {}", dir, err);
        //     exit(1);
        // });
        // preopen_dirs.push((dir.clone(), preopened_dir));
        // let argv = compute_argv(&file, &[]);
        let argv: Vec<String> = vec![Path::new(&file)
            .components()
            .next_back()
            .map(Component::as_os_str)
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_owned()];
        // let environ = compute_environ(&args.flag_env);
        // TODO: check what exact env vars should be passed according to aws
        // let environ = compute_environ(prep_env_vars);
        let environ: Vec<(String, String)> =
            vec![("RUNTIME_VERSION".to_string(), PKG_VERSION.to_string())];

        let wasi_unstable: HostRef<Instance> = HostRef::new({
            let global_exports = store.borrow().global_exports().clone();
            let handle = instantiate_wasi_c("", global_exports, &preopen_dirs, &argv, &environ)?;
            Instance::from_handle(&store, handle)
        });

        module_registry.insert("wasi_unstable".to_owned(), wasi_unstable);
    }

    // let wasi_snapshot_preview1 = HostRef::new(create_wasi_instance(
    //     &store,
    //     &preopen_dirs,
    //     &argv,
    //     &environ,
    // )?);

    // module_registry.insert("wasi_snapshot_preview1".to_owned(), wasi_snapshot_preview1);

    // reenable this sometime
    // // Load the preload wasm modules.
    // for filename in &args.flag_preload {
    //     let path = Path::new(&filename);
    //     instantiate_module(&store, &module_registry, path)
    //         .with_context(|| format!("failed to process preload at `{}`", path.display()))?;
    // }

    // let path: &Path = Path::new(&file);

    // Load the main wasm module.
    let (instance, _module, data): (HostRef<Instance>, HostRef<Module>, Vec<u8>) =
        instantiate_module(&store, &module_registry, Path::new(&file))?;

    let client: Client = Client::new();

    // loop forever n poll runtime api
    loop {
        // get next event
        // let response = client.get(api_next).send().await?;
        // let mut res = surf::get(api_next).await?;
        // let event = res.body_string().await?;
        let response: Response = client.get(&api_next).send()?;
        let headers: &HeaderMap = response.headers();

        // let function_arn: &str = headers
        //     .get("Lambda-Runtime-Invoked-Function-Arn")
        //     .ok_or(anyhow!("missing header Lambda-Runtime-Invoked-Function-Arn"))?;
        let function_arn: &str = headers
            .get("Lambda-Runtime-Invoked-Function-Arn")
            .ok_or(anyhow!(
                "missing header Lambda-Runtime-Invoked-Function-Arn"
            ))?
            .to_str()?;

        let deadline_ms: &str = headers
            .get("Lambda-Runtime-Deadline-Ms")
            .ok_or(anyhow!("missing header Lambda-Runtime-Deadline-Ms"))?
            .to_str()?;

        let request_id: &str = headers
            .get("Lambda-Runtime-Request-Id")
            .ok_or(anyhow!("missing header Lambda-Runtime-Request-Id"))?
            .to_str()?;

        let trace_id: &str = headers
            .get("Lambda-Runtime-Trace-Id")
            .ok_or(anyhow!("missing header Lambda-Runtime-Trace-Id"))?
            .to_str()?;

        let context: String = format!(
            "{{\"function_arn\":\"{:?}\",\"deadline_ms\":\"{:?}\",\"request_id\":\"{:?}\",\"trace_id\":\"{:?}\"}}",
            function_arn,
            deadline_ms,
             request_id,
        trace_id
         );

        let event: String = response.text()?;

        let args: Vec<String> = vec![event, context];

        // let mut event = String::new();
        // response.read_to_string(&mut event)?;
        // let headers = response.headers();
        // craft context JSON
        // let function_arn = headers.get("Lambda-Runtime-Invoked-Function-Arn").ok_or(anyhow!("missing header Lambda-Runtime-Invoked-Function-Arn"))?;
        // let deadline_ms = headers.get("Lambda-Runtime-Deadline-Ms").ok_or(anyhow!("missing header Lambda-Runtime-Deadline-Ms"))?;
        // let request_id = headers.get("Lambda-Runtime-Request-Id").ok_or(anyhow!("missing header Lambda-Runtime-Request-Id"))?;
        // let trace_id = headers.get("Lambda-Runtime-Trace-Id").ok_or(anyhow!("missing header Lambda-Runtime-Trace-Id"))?;
        // let context = format!("{{\"function_arn\":\"{}\",\"deadline_ms\":\"{}\",\"request_id\":\"{}\",\"trace_id\":\"{}\"}}", function_arn, deadline_ms, request_id, trace_id);

        // invoke wasm n report result
        match invoke_export(&instance, &ModuleData::new(&data)?, &handler, args) {
            Ok(result) => client.post(&api_ok).body(result).send()?,
            // Ok(result) => surf::post(api_ok).body_string(result).await?,
            _ => client
                .post(&api_err)
                .body("{\"error\":\"lambda invocation failed\"}")
                .send()?,
            // _ => surf::post(api_err).body_string("{\"error\":\"bootstrap fail\"}".to_string()).await?
        };
    }

    // let data = ModuleData::new(&data)?;
    // let result = invoke_export(instance, &data, file_handler[1], args)?;

    // handle_module(&store, &module_registry, &args, path)
    //     .with_context(|| format!("failed to process main module `{}`", path.display()))?;
}

///////
//
// async fn main() -> AnyHowResult<()> {
//     // TODO: add an error trap to post 2 api/init/error
//     let version = String::from(env!("VERSION"));
//     let warning = "warning .* is experimental and may break in the future";
//     let api = format!("http://{}/2018-06-01/runtime", var("AWS_LAMBDA_RUNTIME_API")?);
//     let api_next = format!("{}/invocation/next", api);
//     let api_err = format!("{}/invocation/error", api);
//     let api_ok = format!("{}/invocation/response", api);
//
//     let file_handler = var("_HANDLER")?.split(".").collect::<Vec<&str>>();
//     let prep_env_vars; // TODO: pass env vars to module instance
//
//     let engine = Engine::default();
//     let store = HostRef::new(Store::new(&engine));
//
//     let wasm = read(file_handler[0])?;
//
//     let module = HostRef::new(Module::new(&store, &wasm)?);
//     // let instance = Instance::new(&store, &module, &[])?;
//     // store, preopened_dirs, argv, environ
//     let instance = create_wasi_instance(&store, &[], &[], &[])?;
//
//     let handler = instance.exports()[0].func().ok_or(anyhow!("failed indexing handler"))?;
//
//     // let client = Client::new();
//
//     // loop forever n poll runtime api
//     loop {
//         // get next event
//         // let response = client.get(api_next).send().await?;
//         let mut res = surf::get(api_next).await?;
//         let event = res.body_string().await?;
//         // let mut event = String::new();
//         // response.read_to_string(&mut event)?;
//         // let headers = response.headers();
//         // craft context JSON
//         // let function_arn = headers.get("Lambda-Runtime-Invoked-Function-Arn").ok_or(anyhow!("missing header Lambda-Runtime-Invoked-Function-Arn"))?;
//         // let deadline_ms = headers.get("Lambda-Runtime-Deadline-Ms").ok_or(anyhow!("missing header Lambda-Runtime-Deadline-Ms"))?;
//         // let request_id = headers.get("Lambda-Runtime-Request-Id").ok_or(anyhow!("missing header Lambda-Runtime-Request-Id"))?;
//         // let trace_id = headers.get("Lambda-Runtime-Trace-Id").ok_or(anyhow!("missing header Lambda-Runtime-Trace-Id"))?;
//         // let context = format!("{{\"function_arn\":\"{}\",\"deadline_ms\":\"{}\",\"request_id\":\"{}\",\"trace_id\":\"{}\"}}", function_arn, deadline_ms, request_id, trace_id);
//         let context = format!(
//             "{{\"function_arn\":\"{}\",\"deadline_ms\":\"{}\",\"request_id\":\"{}\",\"trace_id\":\"{}\"}}",
//             res.header("Lambda-Runtime-Invoked-Function-Arn").ok_or(anyhow!("missing header Lambda-Runtime-Invoked-Function-Arn"))?,
//             res.header("Lambda-Runtime-Deadline-Ms").ok_or(anyhow!("missing header Lambda-Runtime-Deadline-Ms"))?,
//              res.header("Lambda-Runtime-Request-Id").ok_or(anyhow!("missing header Lambda-Runtime-Request-Id"))?,
//         res.header("Lambda-Runtime-Trace-Id").ok_or(anyhow!("missing header Lambda-Runtime-Trace-Id"))?
//          );
//         // invoke wasm n report result
//         match handler.borrow().call(&[event, context]) {
//             // Ok(result) => client.post(api_ok).body(result).send(),
//             Ok(result) => surf::post(api_ok).body_string(result).await?,
//             // _ => client.post(api_err).body("{\"error\":\"bootstrap fail\"}").send(),
//             _ => surf::post(api_err).body_string("{\"error\":\"bootstrap fail\"}".to_string()).await?
//         };
//     }
// }
