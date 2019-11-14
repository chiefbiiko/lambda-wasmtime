# lambda-wasmtime

![shellcheck](https://github.com/chiefbiiko/lambda-wasmtime/workflows/shellcheck/badge.svg) ![demo](https://github.com/chiefbiiko/lambda-wasmtime/workflows/demo/badge.svg)

## wat

`lambda-wasmtime` is a custom AWS Lambda runtime built with [`wasmtime`](https://wasmtime.dev/).

## Demo

Check out the `demo` directory for an example of building a wasm lambda with Rust.
The wasm module exports a function "handler" that when invoked with two JSON strings, 
the event and context objects, processes a base64 encoded image and returns a JSON response. 

Note that in order to get madness like wasm interface types 4 things like "wasm string args" working follow [below steps](#building-a-wasm-lambda) or check this [`wasmtime guide`](https://bytecodealliance.github.io/wasmtime/wasm-rust.html#webassembly-interface-types).

...

### Getting the runtime

[public runtime arn]

[github releases]

### Building a wasm lambda

[cargo toml lib.crate-type cdylib + dependencies.wasm-bindgen]

[#[wasm_bindgen] macro 4 yo fn]

[cargo install cargo-wasi]

[cargo wasi build --release]

[zip -j $archive $wasm]

## License

[MIT](./LICENSE)