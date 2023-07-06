# tomb-wasm

This is a [WebAssembly](https://webassembly.org/) implementation a client that can read and write metadata and content created by tomb.

## Setup
```
cargo install wasm-pack
```

## Build
```
# For stable release
wasm-pack build
# For nightly release
rustup run nightly wasm-pack build
```

## Test
```
# For stable release
wasm-pack test --headless --firefox
# For nightly release
rustup run nightly wasm-pack test --headless --firefox
```

## Run
```
# import the module in your javascript e.g.
import { Tomb } from '/path/to/tomb-wasm/pkg/tomb_wasm';

...

let tomb = Tomb.new();
tomb.init();
```
