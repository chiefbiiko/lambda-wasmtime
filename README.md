# lambda-wasmtime

[![release](https://img.shields.io/github/release/chiefbiiko/lambda-wasmtime/all.svg)](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest) [![Github All Releases](https://img.shields.io/github/downloads/chiefbiiko/lambda-wasmtime/total.svg)](https://github.com/chiefbiiko/lambda-wasmtime/releases/latest) [![GitHub license](https://img.shields.io/github/license/chiefbiiko/lambda-wasmtime.svg)](https://github.com/chiefbiiko/lambda-wasmtime/blob/master/LICENSE) [![stability-experimental](https://img.shields.io/badge/stability-experimental-orange.svg)](https://github.com/chiefbiiko/lambda-wasmtime) [![shellcheck](https://github.com/chiefbiiko/lambda-wasmtime/workflows/shellcheck/badge.svg)](./bootstrap) [![demo](https://github.com/chiefbiiko/lambda-wasmtime/workflows/demo/badge.svg)](./demo)  [![mvp](https://img.shields.io/badge/mvp-bash-lightgreen.svg)](https://shields.io/) [![bash](https://badges.frapsoft.com/bash/v1/bash.png?v=103)](./bootstrap)

**wat??** `lambda-wasmtime` is a custom AWS Lambda runtime built with [`wasmtime`](https://wasmtime.dev/). Runs WebAssembly on AWS Lambda by levaraging the [Component model proposal](https://github.com/WebAssembly/component-model).

### Prerequisite

* [Rust](https://www.rust-lang.org/tools/install)
* [Node.js](https://nodejs.org/)
* [Cargo Lambda](https://github.com/cargo-lambda/cargo-lambda)
* [Cargo WASI](https://github.com/bytecodealliance/cargo-wasi)

### Getting the Runtime

You should prepare the binary for your Lambda layer by running:

```
npm run prepare-layer
```


### Building a WebAssembly Lambda

~~Check out this [2-minute article](https://dev.to/chiefbiiko/lambda-wasmtime-running-webassembly-on-aws-lambda-51gi) for a walkthrough of building a wasm lambda. More background info can be found in the [`wasmtime` guide](https://bytecodealliance.github.io/wasmtime/wasm-rust.html#webassembly-interface-types).~~


Prepare the Wasm function using the Rust toolchain in the example that makes an outbound HTTP request: 

```
npm run prepare-function
```


** Note: you might need to setup wasm-opt yourself as described [here](https://bytecodealliance.github.io/cargo-wasi/wasm-opt.html#which-wasm-opt-executed) or disable the optimization in the [Cargo.toml](./Cargo.toml) if its binary is not available for your machine's architecture.


### Testing locally

We can use the [emulator provided by the Cargo Lambda](https://github.com/cargo-lambda/cargo-lambda#watch) to run it locally in order to shorten the development feedback cycle. In one terminal window start the server:

```
npm run lambda-watch
```

In another terminal window, you can make invoke your function like this:

```
npm run lambda-invoke
```

### Deploying to AWS

We have defined the infrastructure using the AWS CDK. Therefore, after you have built the Lambda layer and its Function, you can deploy it by running:

```
npx cdk deploy
```

## License

[MIT](./LICENSE)
