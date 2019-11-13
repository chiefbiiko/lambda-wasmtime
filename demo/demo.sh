#!/usr/bin/env bash

set -Eeuo pipefail

# simple demo script showcasing
#  + how to compile Rust to a wasm module that uses interface types (strings!)
#  + that such wasm can do real-world tasks like image processing

WASM=./target/wasm32-wasi/release/demo.wasm
EVENT="{\"data\":\"$(base64 ./luigi.png)\"}"
CONTEXT="{}"

# build the interface-types-enabled wasm module
cargo wasi build --release

# run an export from our wasm module - passing in strings!!!
result="$(wasmtime --disable-cache --enable-simd --invoke=handler "$WASM" "$EVENT" "$CONTEXT" 2>&1 | grep -v warning)"

# massage the image from JSON to a PNG
node -e "fs.writeFileSync('./thumbnail.png',Buffer.from(JSON.parse('$result').data,'base64'))"

printf "binary sizes of ./luigi.png and ./thumbnail.png\n"
wc -c ./luigi.png ./thumbnail.png

# if we got some arg call open
if [[ -n "$1" ]]; then
  open ./luigi.png ./thumbnail.png
fi

# zipup a demo lambda bundle
# can be deployed on aws with the runtime layer ./../lambda_wasmtime.zip
zip -j ./demo.zip "$WASM" > /dev/null