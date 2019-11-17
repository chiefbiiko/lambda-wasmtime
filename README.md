# lambda-wasmtime

![shellcheck](https://github.com/chiefbiiko/lambda-wasmtime/workflows/shellcheck/badge.svg) ![demo](https://github.com/chiefbiiko/lambda-wasmtime/workflows/demo/badge.svg)

**wat??** `lambda-wasmtime` is a custom AWS Lambda runtime built with [`wasmtime`](https://wasmtime.dev/). Runs WebAssembly on AWS Lambda.

### Getting the Runtime

Currently only available from [Github Releases](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest). Make sure to check for new releases and update your runtime layer from time to time.

### Building a WebAssembly Lambda

Check out this [x-minute article](...) for a detailed walkthrough of building a wasm lambda. A little background info can be found in the [`wasmtime guide`](https://bytecodealliance.github.io/wasmtime/wasm-rust.html#webassembly-interface-types).

[cargo toml lib.crate-type cdylib + dependencies.wasm-bindgen]

[#[wasm_bindgen] macro 4 yo fn]

[cargo install cargo-wasi]

[cargo wasi build --release]

[zip -j $archive $wasm]

## License

[MIT](./LICENSE)