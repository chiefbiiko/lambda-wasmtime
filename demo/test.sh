#!/usr/bin/env sh
event="{\"data\":\"avivWN/hj/MCC2Xo9dzf0gt6TcSEUW49NFt7KfIsLfY=\"}"
context="{}"
wasmtime --disable-cache --invoke=handler "$1" "$event" "$context" 2>&1
# wasmtime --disable-cache --invoke=handler "$1" "${event}" "${context}" 2>&1
# wasmtime --disable-cache --invoke=handler "$1" 1 2 2>&1
# wasmtime --disable-cache --invoke=handler "$1" "fraud" "money" 2>&1
# wasmtime --disable-cache --invoke=handler "$1" $'\x7b\x22\x64\x61\x74\x61\x22\x3a\x22\x61\x76\x69\x76\x57\x4e\x2f\x68\x6a\x2f\x4d\x43\x43\x32\x58\x6f\x39\x64\x7a\x66\x30\x67\x74\x36\x54\x63\x53\x45\x55\x57\x34\x39\x4e\x46\x74\x37\x4b\x66\x49\x73\x4c\x66\x59\x3d\x22\x7d' $'\x7b\x7d' 2>&1