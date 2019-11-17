# lambda-wasmtime

[![release](https://img.shields.io/github/release/chiefbiiko/lambda-wasmtime/all.svg)](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest) [![Github All Releases](https://img.shields.io/github/downloads/chiefbiiko/lambda-wasmtime/total.svg)](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest) [![GitHub license](https://img.shields.io/github/license/chiefbiiko/lambda-wasmtime.svg)](https://github.com/chiefbiiko/lambda-wasmtime/blob/master/LICENSE) [![stability-experimental](https://img.shields.io/badge/stability-experimental-orange.svg)](https://github.com/chiefbiiko/lambda-wasmtime) [![shellcheck](https://github.com/chiefbiiko/lambda-wasmtime/workflows/shellcheck/badge.svg)](./bootstrap) [![demo](https://github.com/chiefbiiko/lambda-wasmtime/workflows/demo/badge.svg)](./demo)

**wat??** `lambda-wasmtime` is a custom AWS Lambda runtime built with [`wasmtime`](https://wasmtime.dev/). Runs WebAssembly on AWS Lambda.

### Getting the Runtime

Currently only available from [Github Releases](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest). Make sure to check for new releases and update your runtime layer from time to time.

### Building a WebAssembly Lambda

Check out this [2-minute article](...) for a walkthrough of building a wasm lambda. More background info can be found in the [`wasmtime guide`](https://bytecodealliance.github.io/wasmtime/wasm-rust.html#webassembly-interface-types).

## License

[MIT](./LICENSE)