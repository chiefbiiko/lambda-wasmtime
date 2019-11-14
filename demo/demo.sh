#!/usr/bin/env bash

set -o pipefail #-Ee

# simple demo script showcasing
#  + how to compile Rust to a wasm module that uses interface types (strings!)
#  + that such wasm can do real-world tasks like image processing

# usage: bash $0 [demo_dir] [open] 

demo_dir="${1:-"$(pwd)"}"

cd "$demo_dir"

wasm=./target/wasm32-wasi/release/demo.wasm
event="{\"data\":\"$(base64 ./luigi.png)\"}"
context="{}"

# build the interface-types-enabled wasm module
cargo wasi build --release

# run an export from our wasm module - passing in strings!!!
result="$($(which wasmtime) --disable-cache --enable-simd --invoke=handler "$wasm" "$event" "$context" 2>&1 | grep -v warning)"

# massage the image from JSON to a PNG
node -e "fs.writeFileSync('./thumbnail.png',Buffer.from(JSON.parse('$result').data,'base64'))"

printf "binary sizes of luigi.png and thumbnail.png\n"
wc -c ./luigi.png ./thumbnail.png

# if flag set call open
if [[ -n "$2" ]]; then
  open ./luigi.png ./thumbnail.png
fi

# zipup a demo lambda bundle
# can be deployed on aws with the runtime layer ./../lambda_wasmtime.zip
zip -j ./demo.zip "$wasm" > /dev/null