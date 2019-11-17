#!/usr/bin/env bash

# usage: bash $0 [demo_dir]

cat << EOL
simple demo script showcasing
 + how to compile Rust to a wasm module that uses interface types (strings!)
 + that such wasm can do real-world tasks like image processing
   + the wasm module exports a function "handler" that when invoked with two 
     JSON strings, the event and context objects, processes a base64 encoded 
     image, creates a thumbnail of it, and returns the base64 representation
     of that thumbnail as a JSON response
EOL

cd "${1:-"$(pwd)"}"

wasm=./target/wasm32-wasi/release/demo.wasm
event="{\"data\":\"$(base64 ./luigi.png)\"}"
context="{}"

# build the interface-types-enabled wasm module
cargo wasi build --release

# run an export from our wasm module - passing in strings!!!
result="$($(which wasmtime) --disable-cache --enable-simd --invoke=handler "$wasm" "$event" "$context" 2>&1 | grep -v warning)"

# massage the image from JSON to a PNG
node -e "fs.writeFileSync('./thumbnail.png',Buffer.from(JSON.parse('$result').data,'base64'))"

# inspect images
if ! viu -ntv ./luigi.png ./thumbnail.png; then
  cargo install viu
  viu -ntv ./luigi.png ./thumbnail.png
fi

# zipup a demo lambda bundle
# can be deployed on aws with the runtime layer - get its latest release from:
# https://github.com/chiefbiiko/lambda-wasmtime/releases/latest
# make sure to provision the lambda with approx. 2048 MB memory and a 5s timeout
# note: all of this is mvp and experimental at the moment
zip -j ./demo.zip "$wasm" > /dev/null