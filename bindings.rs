pub mod lambda {
    #[allow(unused_imports)]
    use wit_bindgen_wasmtime::{anyhow, wasmtime};
    #[repr(u8)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Error {
        ClientError,
        ServerError,
    }
    impl std::fmt::Debug for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::ClientError => f.debug_tuple("Error::ClientError").finish(),
                Error::ServerError => f.debug_tuple("Error::ServerError").finish(),
            }
        }
    }
    pub type Event<'a> = &'a str;
    pub type Output = String;
    pub type Context<'a> = &'a str;

    /// Auxiliary data associated with the wasm exports.
    ///
    /// This is required to be stored within the data of a
    /// `Store<T>` itself so lifting/lowering state can be managed
    /// when translating between the host and wasm.
    #[derive(Default)]
    pub struct LambdaData {}
    pub struct Lambda<T> {
        get_state: Box<dyn Fn(&mut T) -> &mut LambdaData + Send + Sync>,
        canonical_abi_free: wasmtime::TypedFunc<(i32, i32, i32), ()>,
        canonical_abi_realloc: wasmtime::TypedFunc<(i32, i32, i32, i32), i32>,
        handler: wasmtime::TypedFunc<(i32, i32, i32, i32, i32), (i32,)>,
        memory: wasmtime::Memory,
    }
    impl<T> Lambda<T> {
        #[allow(unused_variables)]

        /// Adds any intrinsics, if necessary for this exported wasm
        /// functionality to the `linker` provided.
        ///
        /// The `get_state` closure is required to access the
        /// auxiliary data necessary for these wasm exports from
        /// the general store's state.
        pub fn add_to_linker(
            linker: &mut wasmtime::Linker<T>,
            get_state: impl Fn(&mut T) -> &mut LambdaData + Send + Sync + Copy + 'static,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        /// Instantiates the provided `module` using the specified
        /// parameters, wrapping up the result in a structure that
        /// translates between wasm and the host.
        ///
        /// The `linker` provided will have intrinsics added to it
        /// automatically, so it's not necessary to call
        /// `add_to_linker` beforehand. This function will
        /// instantiate the `module` otherwise using `linker`, and
        /// both an instance of this structure and the underlying
        /// `wasmtime::Instance` will be returned.
        ///
        /// The `get_state` parameter is used to access the
        /// auxiliary state necessary for these wasm exports from
        /// the general store state `T`.
        pub fn instantiate(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            module: &wasmtime::Module,
            linker: &mut wasmtime::Linker<T>,
            get_state: impl Fn(&mut T) -> &mut LambdaData + Send + Sync + Copy + 'static,
        ) -> anyhow::Result<(Self, wasmtime::Instance)> {
            Self::add_to_linker(linker, get_state)?;
            let instance = linker.instantiate(&mut store, module)?;
            Ok((Self::new(store, &instance, get_state)?, instance))
        }

        /// Low-level creation wrapper for wrapping up the exports
        /// of the `instance` provided in this structure of wasm
        /// exports.
        ///
        /// This function will extract exports from the `instance`
        /// defined within `store` and wrap them all up in the
        /// returned structure which can be used to interact with
        /// the wasm module.
        pub fn new(
            mut store: impl wasmtime::AsContextMut<Data = T>,
            instance: &wasmtime::Instance,
            get_state: impl Fn(&mut T) -> &mut LambdaData + Send + Sync + Copy + 'static,
        ) -> anyhow::Result<Self> {
            let mut store = store.as_context_mut();
            let canonical_abi_free = instance
                .get_typed_func::<(i32, i32, i32), (), _>(&mut store, "canonical_abi_free")?;
            let canonical_abi_realloc = instance.get_typed_func::<(i32, i32, i32, i32), i32, _>(
                &mut store,
                "canonical_abi_realloc",
            )?;
            let handler = instance
                .get_typed_func::<(i32, i32, i32, i32, i32), (i32,), _>(&mut store, "handler")?;
            let memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| anyhow::anyhow!("`memory` export not a memory"))?;
            Ok(Lambda {
                canonical_abi_free,
                canonical_abi_realloc,
                handler,
                memory,
                get_state: Box::new(get_state),
            })
        }
        pub fn handler(
            &self,
            mut caller: impl wasmtime::AsContextMut<Data = T>,
            event: Event<'_>,
            context: Option<Context<'_>>,
        ) -> Result<Result<Output, Error>, wasmtime::Trap> {
            let func_canonical_abi_realloc = &self.canonical_abi_realloc;
            let func_canonical_abi_free = &self.canonical_abi_free;
            let memory = &self.memory;
            let vec0 = event;
            let ptr0 =
                func_canonical_abi_realloc.call(&mut caller, (0, 0, 1, vec0.len() as i32))?;
            memory
                .data_mut(&mut caller)
                .store_many(ptr0, vec0.as_bytes())?;
            let (result2_0, result2_1, result2_2) = match context {
                Some(e) => {
                    let vec1 = e;
                    let ptr1 = func_canonical_abi_realloc
                        .call(&mut caller, (0, 0, 1, vec1.len() as i32))?;
                    memory
                        .data_mut(&mut caller)
                        .store_many(ptr1, vec1.as_bytes())?;
                    (1i32, ptr1, vec1.len() as i32)
                }
                None => {
                    let e = ();
                    {
                        let () = e;
                        (0i32, 0i32, 0i32)
                    }
                }
            };
            let (result3_0,) = self.handler.call(
                &mut caller,
                (ptr0, vec0.len() as i32, result2_0, result2_1, result2_2),
            )?;
            let load4 = memory.data_mut(&mut caller).load::<u8>(result3_0 + 0)?;
            Ok(match i32::from(load4) {
                0 => Ok({
                    let load5 = memory.data_mut(&mut caller).load::<i32>(result3_0 + 4)?;
                    let load6 = memory.data_mut(&mut caller).load::<i32>(result3_0 + 8)?;
                    let ptr7 = load5;
                    let len7 = load6;

                    let data7 = copy_slice(&mut caller, memory, ptr7, len7, 1)?;
                    func_canonical_abi_free.call(&mut caller, (ptr7, len7, 1))?;
                    String::from_utf8(data7).map_err(|_| wasmtime::Trap::new("invalid utf-8"))?
                }),
                1 => Err({
                    let load8 = memory.data_mut(&mut caller).load::<u8>(result3_0 + 4)?;
                    match i32::from(load8) {
                        0 => Error::ClientError,
                        1 => Error::ServerError,
                        _ => return Err(invalid_variant("Error")),
                    }
                }),
                _ => return Err(invalid_variant("expected")),
            })
        }
    }
    use wit_bindgen_wasmtime::rt::copy_slice;
    use wit_bindgen_wasmtime::rt::invalid_variant;
    use wit_bindgen_wasmtime::rt::RawMem;
}
