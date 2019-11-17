# hello lambda-wasmtime

With [`lambda-wasmtime`](https://github.com/chiefbiiko/lambda-wasmtime) we have 
a [`wasmtime`](https://wasmtime.dev/)-powered custom AWS Lambda runtime 
for running WebAssembly, including futuristic stuff like 
[WASI (WebAssembly System Interface)](https://wasi.dev/) and [WAIT (WebAssembly Interface Types)](https://github.com/WebAssembly/interface-types/blob/master/proposals/interface-types/Explainer.md).

To run in the AWS Lambda execution environment we need to build a WebAssembly 
module that exports a function capable of accepting and returning strings 
(`JSON`). The remainder of this post demonstrates just that.

## Building a WebAssembly Lambda with Rust

**Make** sure to have [cargo-wasi](https://github.com/bytecodealliance/cargo-wasi) installed: `cargo install cargo-wasi`

**Setup** a new project: `cargo new <project_name> --lib`

**Craft** `Cargo.toml`:

+ specify the crate type as `cdylib` to make this a `C`-ish shared library

+ include a recent `wasm-bindgen` (tested with `0.2.54`)

``` toml
[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "x.x.x"
```

**Define** a handler in `src/lib.rs` and `#[wasm_bindgen]` it:

``` rust
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn handler(event: &str, context: &str) -> String {/*.*/}
```

Note that you can name your handler whatever you want. The `lambda-wasmtime` runtime determines the actual handler name from the environment variable `_HANDLER` which is user-defined in AWS Lambda.

**Build** the `.wasm` binary: `cargo wasi build --release`

> For now, when using wasm-bindgen `--release` mode is required to build binaries with interface types (~strings)

**Zipup** a lambda bundle: `zip -j <project>/lambda.zip <project>/target/wasm32-wasi/release/<project_name>.wasm`

**Deploy** the lambda bundle on AWS with the `lambda-wasmtime` runtime layer - get its latest release from [here](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest)

If your handler performs non-trivial computations you probably need to provision the lambda with extra memory. Also note that currently all of this is in MVP state and experimental.