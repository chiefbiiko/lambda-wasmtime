use std::sync::Arc;

use anyhow::{Context, Error, Result};
use tracing::{instrument, log};
use wasi_experimental_http_wasmtime::{HttpCtx, HttpState};
use wasmtime::{Instance, InstancePre, Linker, Store};
use wasmtime_wasi::*;

/// Builder-specific configuration.
#[derive(Clone, Debug, Default)]
pub(crate) struct ExecutionContextConfiguration {
    pub id: String,
    pub task_directory: String,
    pub temp_directory: String,
    pub source: String,
    pub allowed_hosts: Option<Vec<String>>,
    pub max_concurrent_requests: Option<u32>,
}

/// Top-level runtime context data to be passed to a component.
#[derive(Default)]
pub(crate) struct RuntimeContext<T> {
    pub wasi: Option<WasiCtx>,
    pub http: Option<HttpCtx>,
    pub data: Option<T>,
}

/// The engine struct that encapsulate wasmtime engine
#[derive(Clone)]
pub(crate) struct Engine(wasmtime::Engine);

impl Engine {
    /// Create a new engine and initialize it with the given config.
    pub fn new(mut config: wasmtime::Config) -> Result<Self, Error> {
        // In order for Wasmtime to run WebAssembly components, multi memory
        // and module linking must always be enabled.
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

/// An execution context builder.
pub(crate) struct Builder<T: Default> {
    config: ExecutionContextConfiguration,
    linker: Linker<RuntimeContext<T>>,
    store: Store<RuntimeContext<T>>,
    engine: Engine,
}

impl<T: Default + 'static> Builder<T> {
    /// Creates a new instance of the execution builder.
    pub fn new(config: ExecutionContextConfiguration) -> Result<Builder<T>> {
        Self::with_engine(config, Engine::new(Default::default())?)
    }

    /// Creates a new instance of the execution builder with the given wasmtime::Config.
    pub fn with_engine(
        config: ExecutionContextConfiguration,
        engine: Engine,
    ) -> Result<Builder<T>> {
        let data = RuntimeContext::default();
        let linker = Linker::new(&engine.inner());
        let store = Store::new(&engine.inner(), data);

        Ok(Self {
            config,
            linker,
            store,
            engine,
        })
    }

    /// Configures the WASI linker imports for the current execution context.
    pub fn link_wasi(&mut self) -> Result<&mut Self> {
        wasmtime_wasi::add_to_linker(&mut self.linker, |ctx| ctx.wasi.as_mut().unwrap())?;
        Ok(self)
    }

    /// Configures the `wasi_experimental_http` linker imports for the current execution context.
    pub fn link_wasi_http(&mut self) -> Result<&mut Self> {
        let http = HttpState::new()?;
        http.add_to_linker(&mut self.linker, |ctx| ctx.http.as_ref().unwrap())?;
        Ok(self)
    }

    /// Builds a new instance of the execution context.
    #[instrument(skip(self))]
    pub fn build(mut self) -> Result<ExecutionContext<T>> {
        let module = wasmtime::Module::from_file(&self.engine.0, &self.config.source)
            .with_context(|| {
                format!(
                    "Cannot create module for component {} from file {}",
                    self.config.id, self.config.source
                )
            })?;
        log::info!(
            "Created module for component {} from file {:?}",
            self.config.id,
            self.config.source
        );

        let component = Arc::new(self.linker.instantiate_pre(&mut self.store, &module)?);
        log::info!(
            "Created pre-instance from module for component {}.",
            self.config.id
        );

        log::info!("Execution context initialized.");

        Ok(ExecutionContext {
            config: self.config,
            engine: self.engine,
            component,
        })
    }

    /// Configures default host interface implementations.
    pub fn link_defaults(&mut self) -> Result<&mut Self> {
        self.link_wasi()?.link_wasi_http()
    }

    /// Builds a new default instance of the execution context.
    pub fn build_default(config: ExecutionContextConfiguration) -> Result<ExecutionContext<T>> {
        let mut builder = Self::new(config)?;
        builder.link_defaults()?;
        builder.build()
    }
}

/// A execution context for WebAssembly running on Lambda.
#[derive(Clone)]
pub(crate) struct ExecutionContext<T: Default> {
    /// Top-level runtime configuration.
    pub config: ExecutionContextConfiguration,
    /// Wasmtime engine.
    pub engine: Engine,
    /// Pre-initialized (and already linked) component.
    pub component: Arc<InstancePre<RuntimeContext<T>>>,
}

impl<T: Default> ExecutionContext<T> {
    /// Creates a store for a given component given its configuration and runtime data.
    pub fn prepare(&self, data: Option<T>) -> Result<(Store<RuntimeContext<T>>, Instance)> {
        log::info!("Creating store...");
        let mut ctx = RuntimeContext::default();
        let task_dir_path = std::fs::File::open(&self.config.task_directory)?;
        let temp_dir_path = std::fs::File::open(&self.config.temp_directory)?;
        let wasi_ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args()?
            .inherit_env()?
            .preopened_dir(Dir::from_std_file(task_dir_path), "/var/task")?
            .preopened_dir(Dir::from_std_file(temp_dir_path), "/tmp")?;
        ctx.wasi = Some(wasi_ctx.build());
        let http_ctx = HttpCtx {
            allowed_hosts: self.config.allowed_hosts.to_owned(),
            max_concurrent_requests: self.config.max_concurrent_requests,
        };
        ctx.http = Some(http_ctx);
        ctx.data = data;

        let mut store = Store::new(&self.engine.0, ctx);
        let instance = self.component.instantiate(&mut store)?;

        Ok((store, instance))
    }
}
