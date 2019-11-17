# lambda-wasmtime demo

![demo](https://github.com/chiefbiiko/lambda-wasmtime/workflows/demo/badge.svg)

With `lambda-wasmtime` we have a `wasmtime`-powered custom AWS Lambda runtime 
for running WebAssembly, including futuristic stuff like 
WASI (WebAssembly System Interface) and WAIT (WebAssembly Interface Types).

To run in the AWS Lambda execution environment we need to build a WebAssembly 
module that exports a function capable of accepting and returning strings 
(`JSON`). The remainder of this post demonstrates just that.

## Building a WebAssembly Lambda with Rust

**Make** sure to have [cargo-wasi](https://github.com/bytecodealliance/cargo-wasi) installed: `cargo install cargo-wasi`

**Setup** a new project: `cargo new <project_name> --lib`

**Craft** `Cargo.toml`:

+ specify the crate type as `cdylib` to make this a `C`ish shared library

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

**Build** the `.wasm` binary: `cargo wasi build --release`

> For now, when using wasm-bindgen `--release` mode is required to build binaries with interface types

**Zipup** a lambda bundle: `zip -j <project>/lambda.zip <project>/target/wasm32-wasi/release/<project_name>.wasm`